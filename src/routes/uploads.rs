use std::io::Cursor;

use aws_sdk_s3::primitives::ByteStream;
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, ImageReader, Limits};
use serde::Serialize;

use crate::state::AppState;

const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const MAX_SIDE: u32 = 1200; // resize so the longest side is at most this
const JPEG_QUALITY: u8 = 85;

// --- Error handling ---
enum UploadError {
    NotConfigured,
    BadRequest(String),
    Storage(String),
}

impl IntoResponse for UploadError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            UploadError::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "uploads are not configured (set the S3_* env vars)".to_string(),
            ),
            UploadError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            UploadError::Storage(e) => {
                eprintln!("storage error: {e}");
                (StatusCode::BAD_GATEWAY, "failed to store image".to_string())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

#[derive(Serialize)]
struct UploadResponse {
    url: String,
}

// Decode → resize → re-encode as JPEG. This is CPU-heavy, so it runs
// on a blocking thread (see spawn_blocking below) instead of the async
// executor, which would otherwise stall every other request.
fn process_image(bytes: &[u8]) -> Result<Vec<u8>, UploadError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| UploadError::BadRequest("could not read file".into()))?;

    // A tiny file can claim to be 50000x50000 pixels and eat all RAM
    // when decoded ("decompression bomb"). Refuse anything that big.
    let mut limits = Limits::default();
    limits.max_image_width = Some(8000);
    limits.max_image_height = Some(8000);
    reader.limits(limits);

    // Decoding is also our validation: if it's not a real JPEG/PNG/WebP,
    // this fails — we never trust the filename or Content-Type.
    let img = reader
        .decode()
        .map_err(|_| UploadError::BadRequest("file is not a supported image (jpeg, png, webp)".into()))?;

    let img = if img.width() > MAX_SIDE || img.height() > MAX_SIDE {
        // Keeps aspect ratio: fits the image inside MAX_SIDE x MAX_SIDE.
        img.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3)
    } else {
        img
    };

    // JPEG has no transparency, so flatten to plain RGB first.
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|e| UploadError::Storage(format!("jpeg encode failed: {e}")))?;
    Ok(out)
}

// POST /api/admin/uploads  (multipart/form-data, field name "file")
// Returns { "url": "..." } — put that url into a product's image_url.
async fn upload_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), UploadError> {
    let storage = state.storage.as_ref().ok_or(UploadError::NotConfigured)?;

    // A multipart body is a list of fields; find the one called "file".
    let mut file_bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| UploadError::BadRequest(e.body_text()))?
    {
        if field.name() == Some("file") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| UploadError::BadRequest(e.body_text()))?;
            file_bytes = Some(bytes);
            break;
        }
    }
    let bytes = file_bytes.ok_or(UploadError::BadRequest("missing 'file' field".into()))?;

    let jpeg = tokio::task::spawn_blocking(move || process_image(&bytes))
        .await
        .map_err(|e| UploadError::Storage(format!("image task panicked: {e}")))??;

    // Random name: no collisions, and users can't overwrite each other's
    // files or guess paths. Never use the client's filename as the key.
    let key = format!("products/{}.jpg", uuid::Uuid::new_v4());

    storage
        .client
        .put_object()
        .bucket(&storage.bucket)
        .key(&key)
        .body(ByteStream::from(jpeg))
        .content_type("image/jpeg")
        // The key is unique per upload, so browsers/CDNs can cache forever.
        .cache_control("public, max-age=31536000, immutable")
        .send()
        .await
        .map_err(|e| UploadError::Storage(format!("{e:?}")))?;

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            url: format!("{}/{key}", storage.public_url),
        }),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/api/admin/uploads",
        // axum rejects bodies over 2 MB by default; raise it for this route only.
        post(upload_image).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
    )
}

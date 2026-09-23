use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region, RequestChecksumCalculation};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    // None when the S3_* env vars aren't set — the server still starts,
    // only the upload route is unavailable.
    pub storage: Option<Storage>,
}

#[derive(Clone)]
pub struct Storage {
    pub client: aws_sdk_s3::Client, // cheap to clone, like PgPool
    pub bucket: String,
    // Where uploaded files are publicly reachable, e.g. your R2 public
    // bucket URL (https://pub-xxxx.r2.dev) or a custom domain.
    pub public_url: String,
}

impl Storage {
    // Works for both Cloudflare R2 and AWS S3:
    //   R2:  S3_ENDPOINT=https://<account_id>.r2.cloudflarestorage.com  S3_REGION=auto
    //   S3:  leave S3_ENDPOINT unset,  S3_REGION=eu-west-1 (etc.)
    pub fn from_env() -> Option<Storage> {
        let bucket = std::env::var("S3_BUCKET").ok()?;
        let access_key = std::env::var("S3_ACCESS_KEY_ID").ok()?;
        let secret_key = std::env::var("S3_SECRET_ACCESS_KEY").ok()?;
        let public_url = std::env::var("S3_PUBLIC_URL").ok()?;
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "auto".to_string());

        let mut config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(Credentials::new(access_key, secret_key, None, None, "env"))
            // Only add checksums when the API requires them; some
            // S3-compatible stores (older R2) reject the newer defaults.
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired);

        if let Ok(endpoint) = std::env::var("S3_ENDPOINT") {
            config = config.endpoint_url(endpoint).force_path_style(true);
        }

        Some(Storage {
            client: aws_sdk_s3::Client::from_conf(config.build()),
            bucket,
            public_url: public_url.trim_end_matches('/').to_string(),
        })
    }
}

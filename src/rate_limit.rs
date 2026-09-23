use std::{net::IpAddr, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    Json,
};
use governor::middleware::NoOpMiddleware;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::KeyExtractor, GovernorError, GovernorLayer,
};

// Which IP a request counts against.
//
// Locally, that's simply who connected (the peer address). On Render,
// every request arrives from Render's proxy, so the peer address is the
// same for everyone — rate limiting on it would throttle all customers
// together. There, set TRUST_PROXY=true and we use X-Forwarded-For.
//
// We take the LAST entry of X-Forwarded-For: that's the one added by the
// proxy we trust. Earlier entries come from the client and can be faked
// to dodge the limit.
#[derive(Clone)]
pub struct ClientIp {
    trust_proxy: bool,
}

impl KeyExtractor for ClientIp {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<IpAddr, GovernorError> {
        if self.trust_proxy {
            let forwarded = req
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit(',').next())
                .and_then(|ip| ip.trim().parse().ok());
            if let Some(ip) = forwarded {
                return Ok(ip);
            }
        }
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip())
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

// The frontend shows `error` to the user as-is, so make it readable.
fn too_many_requests(err: GovernorError) -> Response<Body> {
    match err {
        GovernorError::TooManyRequests { wait_time, headers } => {
            let mut res = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": format!("Too many attempts. Please wait {wait_time} seconds and try again.")
                })),
            )
                .into_response();
            if let Some(headers) = headers {
                res.headers_mut().extend(headers); // includes Retry-After
            }
            res
        }
        other => {
            eprintln!("rate limiter error: {other}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// Per IP: a burst of 5 requests, then 1 more every 12 seconds (≈5/minute).
// Plenty for a real customer; tiresome for a script.
// Each call makes an independent limiter, so orders and feedback each
// get their own budget.
pub fn public_write_limit() -> GovernorLayer<ClientIp, NoOpMiddleware, Body> {
    let trust_proxy = std::env::var("TRUST_PROXY").is_ok_and(|v| v == "true");

    let config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(12)
            .burst_size(5)
            .key_extractor(ClientIp { trust_proxy })
            .finish()
            .expect("invalid rate limit config"),
    );

    // The limiter remembers every IP it has seen; forget idle ones every
    // minute so memory doesn't grow forever.
    let limiter = config.limiter().clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            limiter.retain_recent();
        }
    });

    GovernorLayer::new(config).error_handler(too_many_requests)
}

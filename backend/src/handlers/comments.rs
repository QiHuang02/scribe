use crate::server::app::AppState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::{Router, routing::get};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Tracks comment submissions from users or IP addresses to prevent abuse.
///
/// Requests over the threshold in the given window will immediately receive a
/// `429 Too Many Requests` response.
async fn rate_limit(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // Identify the client either by a custom `X-User-Id` header or fall back to IP.
    let key = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|id| format!("user:{id}"))
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|ip| format!("ip:{ip}"))
        })
        .unwrap_or_else(|| "ip:unknown".to_string());

    // Window and threshold for rate limiting.
    const WINDOW: Duration = Duration::from_secs(60);
    const THRESHOLD: usize = 5;

    // Global in-memory store of submission timestamps per key.
    static STORE: OnceLock<Arc<Mutex<HashMap<String, Vec<Instant>>>>> = OnceLock::new();
    let store = STORE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));

    let now = Instant::now();
    {
        let mut map = store.lock().await;
        let entry = map.entry(key).or_default();
        entry.push(now);
        let cutoff = now - WINDOW;
        entry.retain(|t| *t > cutoff);
        if entry.len() > THRESHOLD {
            let res = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::from("Too many comments"))
                .unwrap();
            return Ok(res);
        }
    }

    Ok(next.run(req).await)
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/comments", get(not_implemented).post(not_implemented))
        .layer(middleware::from_fn(rate_limit))
}

async fn not_implemented() -> &'static str {
    "Comments feature not implemented"
}

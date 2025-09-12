use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::{Body, to_bytes};
use axum::http::{self, Method, Request, Response};
use bytes::Bytes;
use moka2::future::Cache;
use tower::{Layer, Service};

// Routes that should never be cached (e.g. authentication endpoints).
const CACHE_BYPASS_PATHS: &[&str] = &["/api/auth/"];
/// Maximum response body size that will be cached (1 MiB).
const MAX_CACHED_RESPONSE_SIZE: usize = 1 * 1024 * 1024;

#[derive(Clone)]
pub struct CachedResponse {
    pub body: Bytes,
    pub content_type: Option<String>,
}

#[derive(Clone)]
pub struct ResponseCacheLayer {
    cache: Arc<Cache<String, CachedResponse>>,
}

impl ResponseCacheLayer {
    pub fn new(cache: Arc<Cache<String, CachedResponse>>) -> Self {
        Self { cache }
    }
}

impl<S> Layer<S> for ResponseCacheLayer {
    type Service = ResponseCacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ResponseCacheService {
            inner,
            cache: self.cache.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ResponseCacheService<S> {
    inner: S,
    cache: Arc<Cache<String, CachedResponse>>,
}

impl<S> Service<Request<Body>> for ResponseCacheService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Only cache unauthenticated GET requests
        if req.method() != Method::GET {
            let fut = self.inner.call(req);
            return Box::pin(async move { fut.await });
        }

        let path = req.uri().path().to_string();
        let has_auth = req.headers().contains_key(http::header::AUTHORIZATION)
            || req.headers().contains_key(http::header::COOKIE);

        // Bypass cache if credentials are present or the path is sensitive to
        // avoid leaking user-specific responses.
        if has_auth || CACHE_BYPASS_PATHS.iter().any(|p| path.starts_with(p)) {
            let fut = self.inner.call(req);
            return Box::pin(async move { fut.await });
        }

        let query = req.uri().query().unwrap_or("").to_string();
        let cache_key = {
            let mut pairs: Vec<&str> = query.split('&').filter(|s| !s.is_empty()).collect();
            pairs.sort_unstable();
            format!("{}?{}", path, pairs.join("&"))
        };

        let cache = self.cache.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if let Some(cached) = cache.get(&cache_key).await {
                let CachedResponse { body, content_type } = cached;
                let mut builder = Response::builder();
                if let Some(ct) = content_type {
                    builder = builder.header(axum::http::header::CONTENT_TYPE, ct);
                }
                let resp = builder.body(Body::from(body)).unwrap();
                return Ok(resp);
            }

            let resp = inner.call(req).await?;
            let (parts, body) = resp.into_parts();
            let bytes = match to_bytes(body, MAX_CACHED_RESPONSE_SIZE).await {
                Ok(b) => b,
                Err(_) => {
                    // If the body is too large or an error occurs, skip caching and
                    // return the original response headers with an empty body.
                    return Ok(Response::from_parts(parts, Body::empty()));
                }
            };

            if parts.status.is_success() && bytes.len() <= MAX_CACHED_RESPONSE_SIZE {
                let content_type = parts
                    .headers
                    .get(axum::http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                cache
                    .insert(
                        cache_key,
                        CachedResponse {
                            body: bytes.clone(),
                            content_type,
                        },
                    )
                    .await;
            }

            Ok(Response::from_parts(parts, Body::from(bytes)))
        })
    }
}

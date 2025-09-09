use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::{Body, to_bytes};
use axum::http::{self, Method, Request, Response};
use moka2::future::Cache;
use tower::{Layer, Service};

// Routes that should never be cached (e.g. authentication endpoints).
const CACHE_BYPASS_PATHS: &[&str] = &["/api/auth/"];

#[derive(Clone)]
pub struct ResponseCacheLayer {
    cache: Arc<Cache<String, String>>,
}

impl ResponseCacheLayer {
    pub fn new(cache: Arc<Cache<String, String>>) -> Self {
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
    cache: Arc<Cache<String, String>>,
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
                let resp = Response::builder()
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(cached))
                    .unwrap();
                return Ok(resp);
            }

            let resp = inner.call(req).await?;
            let (parts, body) = resp.into_parts();
            let bytes = to_bytes(body, usize::MAX).await.unwrap();
            let body_str = String::from_utf8(bytes.to_vec()).unwrap();

            if parts.status.is_success() {
                cache.insert(cache_key, body_str.clone()).await;
            }

            Ok(Response::from_parts(parts, Body::from(body_str)))
        })
    }
}

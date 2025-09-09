use crate::handlers::error::AppError;
use crate::server::app::AppState;
use axum::Router;
use axum::extract::State;
use axum::http::header;
use axum::response::Response;
use axum::routing::get;
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/sitemap.xml", get(get_sitemap))
}

async fn get_sitemap(State(state): State<Arc<AppState>>) -> Result<Response, AppError> {
    let store = state.store.read().await;
    let base_url = state.config.base_url.trim_end_matches('/');
    let articles = store.query(|a| !a.metadata.draft);

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">");

    for article in articles {
        xml.push_str(&format!(
            "<url><loc>{}/articles/{}</loc><lastmod>{}</lastmod></url>",
            base_url,
            article.slug,
            article.updated_at.to_rfc3339()
        ));
    }

    xml.push_str("</urlset>");

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/xml")
        .body(axum::body::Body::from(xml))
        .unwrap();

    Ok(response)
}

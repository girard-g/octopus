use axum::body::Body;
use axum::http::{Request, Response};
use axum::Router;
use axum_extra::extract::cookie::Key;
use http_body_util::BodyExt;
use octopus::app::{build_router, AppState};
use sqlx::PgPool;
use tower::ServiceExt;

pub fn test_app(pool: PgPool) -> Router {
    let key = Key::from(&[0u8; 64]);
    build_router(AppState { pool, key })
}

pub async fn send(app: &Router, req: Request<Body>) -> (axum::http::StatusCode, serde_json::Value) {
    let resp: Response<Body> = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json)
}

pub fn json_req(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

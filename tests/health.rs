mod helpers;
use axum::http::StatusCode;
use helpers::{send, test_app};

#[sqlx::test]
async fn health_returns_ok(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let req = axum::http::Request::builder()
        .uri("/api/health")
        .body(axum::body::Body::empty())
        .unwrap();
    let (status, json) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
}

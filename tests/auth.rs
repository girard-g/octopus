mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, send, test_app};
use serde_json::json;

#[sqlx::test]
async fn login_rejects_wrong_password(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let (status, _) = send(&app, json_req("POST", "/api/login", json!({"password": "nope"}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn login_accepts_right_password_and_sets_cookie(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let resp = tower::ServiceExt::oneshot(
        app,
        json_req("POST", "/api/login", json!({"password": "secret"})),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(resp.headers().get("set-cookie").is_some());
}

#[sqlx::test]
async fn protected_route_401_without_cookie(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let req = axum::http::Request::builder()
        .uri("/api/contacts")
        .body(axum::body::Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

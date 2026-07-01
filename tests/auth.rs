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
async fn login_locks_out_after_repeated_failures(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    // 5 wrong attempts are each rejected 401...
    for _ in 0..5 {
        let (status, _) = send(&app, json_req("POST", "/api/login", json!({"password": "nope"}))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // ...the 6th attempt is locked out — even with the CORRECT password.
    let (status, _) = send(&app, json_req("POST", "/api/login", json!({"password": "secret"}))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
async fn login_success_resets_failure_count(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    // 4 failures (below the threshold of 5)...
    for _ in 0..4 {
        send(&app, json_req("POST", "/api/login", json!({"password": "nope"}))).await;
    }
    // ...a success resets the counter...
    let (status, _) = send(&app, json_req("POST", "/api/login", json!({"password": "secret"}))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    // ...so another 4 failures still don't lock out (would have at 8 without reset).
    for _ in 0..4 {
        let (status, _) = send(&app, json_req("POST", "/api/login", json!({"password": "nope"}))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
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

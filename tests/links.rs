mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn link_crud(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    // create
    let (status, l) = send(
        &app,
        json_req("POST", "/api/links", json!({
            "url": "https://rust-lang.org/learn",
            "title": "Rust",
            "category": "Rust",
            "tags": ["reference", "free", "reference"]
        })).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(l["title"], "Rust");
    assert_eq!(l["category"], "Rust");
    // dedupe preserves first occurrence
    assert_eq!(l["tags"], json!(["reference", "free"]));
    let id = l["id"].as_str().unwrap().to_string();

    // title defaults to host when blank
    let (_, l2) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "https://example.com/x"})).with_cookie(&cookie),
    ).await;
    assert_eq!(l2["title"], "example.com");

    // filter by category
    let (status, list) = send(
        &app,
        json_req("GET", "/api/links?category=Rust", json!(null)).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // filter by tag
    let (_, list) = send(
        &app,
        json_req("GET", "/api/links?tag=free", json!(null)).with_cookie(&cookie),
    ).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // update
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/links/{id}"), json!({
            "url": "https://rust-lang.org", "title": "Rust Lang", "tags": []
        })).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["title"], "Rust Lang");
    assert_eq!(upd["tags"], json!([]));

    // delete
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/links/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 404 after delete
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/links/{id}"), json!({"url": "https://x.com"})).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn link_rejects_empty_url(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "   "})).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn link_rejects_non_http_url(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "ftp://nope.com"})).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

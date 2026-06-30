mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn standalone_task_create_and_list(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({"title":"Call accountant","due_on":"2026-07-01"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(t["status"], "todo");
    assert_eq!(t["project_id"], serde_json::Value::Null);

    let (status, list) = send(&app, json_req("GET", "/api/tasks", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn task_update_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, t) = send(&app, json_req("POST", "/api/tasks", json!({"title":"X"})).with_cookie(&cookie)).await;
    let id = t["id"].as_str().unwrap().to_string();

    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/tasks/{id}"), json!({"title":"X","status":"done"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["status"], "done");
}

#[sqlx::test]
async fn task_rejects_bad_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/tasks", json!({"title":"X","status":"frozen"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn task_update_rejects_bad_project_id(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, t) = send(&app, json_req("POST", "/api/tasks", json!({"title":"X"})).with_cookie(&cookie)).await;
    let id = t["id"].as_str().unwrap().to_string();
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/tasks/{id}"), json!({"title":"X","project_id": bogus})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

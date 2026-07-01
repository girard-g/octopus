mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

async fn make_contact(app: &axum::Router, cookie: &str) -> String {
    let (_, c) = send(app, json_req("POST", "/api/contacts", json!({"kind":"company","name":"Acme"})).with_cookie(cookie)).await;
    c["id"].as_str().unwrap().to_string()
}

#[sqlx::test]
async fn project_create_defaults_to_active(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;

    let (status, p) = send(
        &app,
        json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"Website"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(p["status"], "active");
}

#[sqlx::test]
async fn project_list_filters_by_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"A","status":"active"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"B","status":"archived"})).with_cookie(&cookie)).await;

    let (status, list) = send(&app, json_req("GET", "/api/projects?status=active", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "A");
    assert_eq!(list[0]["task_count"], 0);
}

#[sqlx::test]
async fn project_update_can_set_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"A","status":"active"})).with_cookie(&cookie)).await;
    let id = p["id"].as_str().unwrap().to_string();

    let (status, upd) = send(&app, json_req("PUT", &format!("/api/projects/{id}"), json!({"contact_id": contact_id, "title":"A", "status":"archived"})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["status"], "archived");
}

#[sqlx::test]
async fn project_rejects_bad_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"X","status":"wat"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn project_update_preserves_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"A","status":"active"})).with_cookie(&cookie)).await;
    let id = p["id"].as_str().unwrap().to_string();
    // PUT without status must NOT reset status to lead
    let (status, upd) = send(&app, json_req("PUT", &format!("/api/projects/{id}"), json!({"contact_id": contact_id, "title":"A renamed"})).with_cookie(&cookie)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(upd["status"], "active");
    assert_eq!(upd["title"], "A renamed");
}

#[sqlx::test]
async fn project_update_rejects_bad_contact_id(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"X"})).with_cookie(&cookie)).await;
    let id = p["id"].as_str().unwrap().to_string();

    // PUT with a contact_id that does not exist -> 400, not 500
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/projects/{id}"), json!({"contact_id": bogus, "title":"X"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

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

#[sqlx::test]
async fn task_roundtrips_new_fields(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({
            "title": "Ship it",
            "priority": "high",
            "size": "m",
            "description": "the big one",
            "checklist": [{"title":"a","done":false},{"title":"b","done":true}],
            "position": 2
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(t["priority"], "high");
    assert_eq!(t["size"], "m");
    assert_eq!(t["description"], "the big one");
    assert_eq!(t["position"], 2);
    assert_eq!(t["checklist"].as_array().unwrap().len(), 2);
    assert_eq!(t["checklist"][1]["done"], true);
}

// Guards the drag-wipe trap: a full-object PUT that changes status must NOT
// drop priority/checklist. (The board's drag handler sends the full object.)
#[sqlx::test]
async fn task_move_preserves_fields(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({
            "title": "X", "priority": "low",
            "checklist": [{"title":"step","done":false}]
        })).with_cookie(&cookie),
    )
    .await;
    let id = t["id"].as_str().unwrap().to_string();

    // Full-object PUT with status flipped to doing (as the drag handler sends it).
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/tasks/{id}"), json!({
            "title": "X", "status": "doing", "priority": "low",
            "checklist": [{"title":"step","done":false}], "position": 0
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["status"], "doing");
    assert_eq!(upd["priority"], "low");
    assert_eq!(upd["checklist"].as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn task_rejects_bad_priority(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/tasks", json!({"title":"X","priority":"urgent"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn task_roundtrips_version_and_type(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({
            "title": "Ship v1", "version": "v1.0", "type": "feature"
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(t["version"], "v1.0");
    assert_eq!(t["type"], "feature");

    // Full-object PUT (drag path) must preserve both.
    let id = t["id"].as_str().unwrap().to_string();
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/tasks/{id}"), json!({
            "title": "Ship v1", "status": "doing",
            "version": "v1.0", "type": "feature", "position": 0
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["version"], "v1.0");
    assert_eq!(upd["type"], "feature");
}

#[sqlx::test]
async fn task_rejects_bad_type(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/tasks", json!({"title":"X","type":"wibble"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

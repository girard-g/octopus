mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn folder_crud_and_reparent(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, root) = send(&app, json_req("POST", "/api/folders", json!({"name":"clients"})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    let root_id = root["id"].as_str().unwrap().to_string();

    let (_, child) = send(&app, json_req("POST", "/api/folders", json!({"name":"acme","parent_id":root_id})).with_cookie(&cookie)).await;
    let child_id = child["id"].as_str().unwrap().to_string();
    assert_eq!(child["parent_id"], root_id);

    // rename
    let (status, ren) = send(&app, json_req("PUT", &format!("/api/folders/{child_id}"), json!({"name":"acme corp"})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ren["name"], "acme corp");

    // list has both
    let (_, list) = send(&app, json_req("GET", "/api/folders", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(list.as_array().unwrap().len(), 2);

    // self-parent rejected
    let (status, _) = send(&app, json_req("PUT", &format!("/api/folders/{child_id}"), json!({"name":"acme corp","parent_id":child_id})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn folder_create_with_nonexistent_parent_is_bad_request(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, _) = send(&app, json_req("POST", "/api/folders", json!({"name":"orphan","parent_id":"00000000-0000-0000-0000-000000000000"})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn folder_delete_cascades_and_unfiles_notes(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, root) = send(&app, json_req("POST", "/api/folders", json!({"name":"root"})).with_cookie(&cookie)).await;
    let root_id = root["id"].as_str().unwrap().to_string();
    let (_, child) = send(&app, json_req("POST", "/api/folders", json!({"name":"child","parent_id":root_id})).with_cookie(&cookie)).await;
    let child_id = child["id"].as_str().unwrap().to_string();

    let (_, note) = send(&app, json_req("POST", "/api/notes", json!({"body":"in child","folder_id":child_id})).with_cookie(&cookie)).await;
    let note_id = note["id"].as_str().unwrap().to_string();

    // delete root -> child cascades, note falls to Unfiled
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/folders/{root_id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, folders) = send(&app, json_req("GET", "/api/folders", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(folders.as_array().unwrap().len(), 0);

    // note survives with folder_id null
    let (_, all) = send(&app, json_req("GET", "/api/notes", json!(null)).with_cookie(&cookie)).await;
    let n = all.as_array().unwrap().iter().find(|n| n["id"] == note_id).unwrap();
    assert!(n["folder_id"].is_null());
}

#[sqlx::test]
async fn folder_reparent_rejects_indirect_cycle(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, a) = send(&app, json_req("POST", "/api/folders", json!({"name":"A"})).with_cookie(&cookie)).await;
    let a_id = a["id"].as_str().unwrap().to_string();
    let (_, b) = send(&app, json_req("POST", "/api/folders", json!({"name":"B","parent_id":a_id})).with_cookie(&cookie)).await;
    let b_id = b["id"].as_str().unwrap().to_string();

    // B is already a child of A; moving A under B would create a cycle.
    let (status, _) = send(&app, json_req("PUT", &format!("/api/folders/{a_id}"), json!({"name":"A","parent_id":b_id})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn folder_reparent_rejects_direct_self_parent(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, a) = send(&app, json_req("POST", "/api/folders", json!({"name":"A"})).with_cookie(&cookie)).await;
    let a_id = a["id"].as_str().unwrap().to_string();

    let (status, _) = send(&app, json_req("PUT", &format!("/api/folders/{a_id}"), json!({"name":"A","parent_id":a_id})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn folder_reparent_allows_legitimate_move(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, a) = send(&app, json_req("POST", "/api/folders", json!({"name":"A"})).with_cookie(&cookie)).await;
    let a_id = a["id"].as_str().unwrap().to_string();
    let (_, b) = send(&app, json_req("POST", "/api/folders", json!({"name":"B"})).with_cookie(&cookie)).await;
    let b_id = b["id"].as_str().unwrap().to_string();

    // A and B are siblings (both roots); moving B under A is a legitimate re-parent.
    let (status, moved) = send(&app, json_req("PUT", &format!("/api/folders/{b_id}"), json!({"name":"B","parent_id":a_id})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved["parent_id"], a_id);
}

mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

async fn make_contact(app: &axum::Router, cookie: &str) -> String {
    let (_, c) = send(app, json_req("POST", "/api/contacts", json!({"kind":"person","name":"Alice"})).with_cookie(cookie)).await;
    c["id"].as_str().unwrap().to_string()
}

#[sqlx::test]
async fn note_crud(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let cid = make_contact(&app, &cookie).await;

    // create
    let (status, n) = send(
        &app,
        json_req("POST", "/api/notes", json!({"body":"Hello","contact_id":cid})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(n["body"], "Hello");
    let id = n["id"].as_str().unwrap().to_string();

    // list by contact
    let (status, list) = send(
        &app,
        json_req("GET", &format!("/api/notes?contact_id={cid}"), json!(null)).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // update
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/notes/{id}"), json!({"body":"Updated","contact_id":cid})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["body"], "Updated");

    // delete
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/notes/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 404 after delete
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/notes/{id}"), json!({"body":"X","contact_id":cid})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn note_allows_no_parent(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, n) = send(
        &app,
        json_req("POST", "/api/notes", json!({"body":"standalone"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(n["body"], "standalone");
}

#[sqlx::test]
async fn note_allows_title_only(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, n) = send(
        &app,
        json_req("POST", "/api/notes", json!({"title":"Just a title","body":""})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(n["title"], "Just a title");
}

#[sqlx::test]
async fn note_rejects_empty_title_and_body(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/notes", json!({"body":"   "})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn note_rejects_two_parents(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let cid = make_contact(&app, &cookie).await;
    // need a real project_id too — create a project
    let (_, p) = send(
        &app,
        json_req("POST", "/api/projects", json!({"contact_id":cid,"title":"P"})).with_cookie(&cookie),
    )
    .await;
    let pid = p["id"].as_str().unwrap().to_string();
    let (status, _) = send(
        &app,
        json_req("POST", "/api/notes", json!({"body":"Hello","contact_id":cid,"project_id":pid})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn note_create_bad_contact_id_is_400(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(
        &app,
        json_req("POST", "/api/notes", json!({"body":"Hello","contact_id":bogus})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn note_update_bad_project_id_is_400(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let cid = make_contact(&app, &cookie).await;
    let (_, n) = send(
        &app,
        json_req("POST", "/api/notes", json!({"body":"Hello","contact_id":cid})).with_cookie(&cookie),
    )
    .await;
    let id = n["id"].as_str().unwrap().to_string();
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/notes/{id}"), json!({"body":"X","project_id":bogus})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn note_parent_filter(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let cid1 = make_contact(&app, &cookie).await;
    let cid2 = make_contact(&app, &cookie).await;

    send(&app, json_req("POST", "/api/notes", json!({"body":"For c1","contact_id":cid1})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/notes", json!({"body":"For c2","contact_id":cid2})).with_cookie(&cookie)).await;

    let (_, list) = send(
        &app,
        json_req("GET", &format!("/api/notes?contact_id={cid1}"), json!(null)).with_cookie(&cookie),
    )
    .await;
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["body"], "For c1");
}

#[sqlx::test]
async fn note_carries_title_and_pin(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, n) = send(
        &app,
        json_req("POST", "/api/notes", json!({"title":"T","body":"B","pinned":true})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(n["title"], "T");
    assert_eq!(n["pinned"], true);
    assert!(n["updated_at"].is_string());
}

#[sqlx::test]
async fn note_search_matches_title_and_body(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    send(&app, json_req("POST", "/api/notes", json!({"title":"Kickoff","body":"agenda"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/notes", json!({"body":"unrelated"})).with_cookie(&cookie)).await;

    let (_, hits) = send(&app, json_req("GET", "/api/notes?q=kick", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(hits.as_array().unwrap().len(), 1);

    let (_, hits2) = send(&app, json_req("GET", "/api/notes?q=agenda", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(hits2.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn note_folder_filter_pins_first(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, f) = send(&app, json_req("POST", "/api/folders", json!({"name":"F"})).with_cookie(&cookie)).await;
    let fid = f["id"].as_str().unwrap().to_string();

    send(&app, json_req("POST", "/api/notes", json!({"body":"plain","folder_id":fid})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/notes", json!({"body":"pinned","folder_id":fid,"pinned":true})).with_cookie(&cookie)).await;

    let (_, list) = send(&app, json_req("GET", &format!("/api/notes?folder_id={fid}"), json!(null)).with_cookie(&cookie)).await;
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["body"], "pinned"); // pinned sorts first
}

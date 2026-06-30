mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn contacts_crud_flow(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    // create
    let (status, created) = send(
        &app,
        json_req("POST", "/api/contacts", json!({"kind":"company","name":"Acme"}))
            .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "Acme");
    let id = created["id"].as_str().unwrap().to_string();

    // list
    let (status, list) = send(&app, json_req("GET", "/api/contacts", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // get
    let (status, got) = send(&app, json_req("GET", &format!("/api/contacts/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["id"], id);

    // update
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/contacts/{id}"), json!({"kind":"company","name":"Acme Inc"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["name"], "Acme Inc");

    // delete
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/contacts/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // get after delete -> 404
    let (status, _) = send(&app, json_req("GET", &format!("/api/contacts/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn contact_create_rejects_bad_company_id(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(&app, json_req("POST", "/api/contacts", json!({"kind":"person","name":"P","company_id": bogus})).with_cookie(&cookie)).await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn contact_update_rejects_bad_company_id(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, c) = send(&app, json_req("POST", "/api/contacts", json!({"kind":"company","name":"Acme"})).with_cookie(&cookie)).await;
    let id = c["id"].as_str().unwrap().to_string();
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(&app, json_req("PUT", &format!("/api/contacts/{id}"), json!({"kind":"person","name":"P","company_id": bogus})).with_cookie(&cookie)).await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn contact_rejects_bad_kind(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/contacts", json!({"kind":"alien","name":"X"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

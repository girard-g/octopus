mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn event_crud(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    // create
    let (status, e) = send(
        &app,
        json_req(
            "POST",
            "/api/events",
            json!({"title":"Team sync","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(e["title"], "Team sync");
    assert_eq!(e["all_day"], false);
    let id = e["id"].as_str().unwrap().to_string();

    // get
    let (status, e2) = send(&app, json_req("GET", &format!("/api/events/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(e2["title"], "Team sync");

    // list
    let (status, list) = send(&app, json_req("GET", "/api/events", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // update
    let (status, upd) = send(
        &app,
        json_req(
            "PUT",
            &format!("/api/events/{id}"),
            json!({"title":"Updated","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T12:00:00Z","all_day":false}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["title"], "Updated");

    // delete
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/events/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 404 after delete
    let (status, _) = send(&app, json_req("GET", &format!("/api/events/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn event_rejects_empty_title(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/events", json!({"title":"","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn event_rejects_ends_before_starts(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/events", json!({"title":"X","starts_at":"2026-07-01T11:00:00Z","ends_at":"2026-07-01T10:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn event_create_bad_project_id_is_400(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(
        &app,
        json_req(
            "POST",
            "/api/events",
            json!({"title":"X","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","project_id":bogus}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn event_update_bad_contact_id_is_400(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, e) = send(
        &app,
        json_req("POST", "/api/events", json!({"title":"X","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    let id = e["id"].as_str().unwrap().to_string();
    let bogus = "00000000-0000-0000-0000-000000000000";
    let (status, _) = send(
        &app,
        json_req(
            "PUT",
            &format!("/api/events/{id}"),
            json!({"title":"X","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","contact_id":bogus}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn event_update_rejects_empty_title(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, e) = send(
        &app,
        json_req("POST", "/api/events", json!({"title":"X","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    let id = e["id"].as_str().unwrap().to_string();
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/events/{id}"), json!({"title":"","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn event_update_rejects_ends_before_starts(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, e) = send(
        &app,
        json_req("POST", "/api/events", json!({"title":"X","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    let id = e["id"].as_str().unwrap().to_string();
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/events/{id}"), json!({"title":"X","starts_at":"2026-07-01T11:00:00Z","ends_at":"2026-07-01T10:00:00Z"}))
            .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn event_from_only_filter_returns_ok(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, list) = send(
        &app,
        json_req("GET", "/api/events?from=2026-07-01T00:00:00Z", json!(null)).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(list.is_array());
}

#[sqlx::test]
async fn event_range_filter(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    // inside range
    send(&app, json_req("POST", "/api/events", json!({"title":"In","starts_at":"2026-07-05T10:00:00Z","ends_at":"2026-07-05T11:00:00Z"})).with_cookie(&cookie)).await;
    // outside range
    send(&app, json_req("POST", "/api/events", json!({"title":"Out","starts_at":"2026-08-01T10:00:00Z","ends_at":"2026-08-01T11:00:00Z"})).with_cookie(&cookie)).await;

    let (status, list) = send(
        &app,
        json_req("GET", "/api/events?from=2026-07-01T00:00:00Z&to=2026-07-31T00:00:00Z", json!(null)).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "In");
}

#[sqlx::test]
async fn created_event_exposes_null_series_id(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, e) = send(
        &app,
        json_req(
            "POST",
            "/api/events",
            json!({"title":"Solo","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    // Field must be PRESENT in the response and null for a standalone event.
    assert!(e.as_object().unwrap().contains_key("series_id"));
    assert!(e["series_id"].is_null());
}

#[sqlx::test]
async fn create_series_inserts_rows_with_shared_series_id(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let body = json!({
        "occurrences": [
            {"title":"Standup","starts_at":"2026-07-06T10:00:00Z","ends_at":"2026-07-06T10:15:00Z"},
            {"title":"Standup","starts_at":"2026-07-13T10:00:00Z","ends_at":"2026-07-13T10:15:00Z"},
            {"title":"Standup","starts_at":"2026-07-20T10:00:00Z","ends_at":"2026-07-20T10:15:00Z"}
        ]
    });
    let (status, rows) = send(&app, json_req("POST", "/api/events/series", body).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    let sid = arr[0]["series_id"].as_str().unwrap().to_string();
    assert!(!sid.is_empty());
    assert!(arr.iter().all(|r| r["series_id"] == json!(sid)));

    // all three visible via list
    let (_, list) = send(&app, json_req("GET", "/api/events", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(list.as_array().unwrap().len(), 3);
}

#[sqlx::test]
async fn create_series_rejects_empty(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/events/series", json!({"occurrences": []})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn create_series_rejects_oversize(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let mut occ = Vec::new();
    for _ in 0..367 {
        occ.push(json!({"title":"x","starts_at":"2026-07-06T10:00:00Z","ends_at":"2026-07-06T10:15:00Z"}));
    }
    let (status, _) = send(
        &app,
        json_req("POST", "/api/events/series", json!({"occurrences": occ})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

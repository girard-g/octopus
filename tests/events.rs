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

async fn make_weekly_series(app: &axum::Router, cookie: &str) -> Vec<String> {
    let body = json!({
        "occurrences": [
            {"title":"W","starts_at":"2026-07-06T10:00:00Z","ends_at":"2026-07-06T10:15:00Z"},
            {"title":"W","starts_at":"2026-07-13T10:00:00Z","ends_at":"2026-07-13T10:15:00Z"},
            {"title":"W","starts_at":"2026-07-20T10:00:00Z","ends_at":"2026-07-20T10:15:00Z"},
            {"title":"W","starts_at":"2026-07-27T10:00:00Z","ends_at":"2026-07-27T10:15:00Z"}
        ]
    });
    let (_, rows) = send(app, json_req("POST", "/api/events/series", body).with_cookie(cookie)).await;
    rows.as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_string()).collect()
}

async fn event_count(app: &axum::Router, cookie: &str) -> usize {
    let (_, list) = send(app, json_req("GET", "/api/events", json!(null)).with_cookie(cookie)).await;
    list.as_array().unwrap().len()
}

#[sqlx::test]
async fn delete_scope_one_removes_single_occurrence(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let ids = make_weekly_series(&app, &cookie).await;
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/events/{}?scope=one", ids[1]), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(event_count(&app, &cookie).await, 3);
}

#[sqlx::test]
async fn delete_scope_following_removes_from_target_onward(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let ids = make_weekly_series(&app, &cookie).await; // ids[2] = Jul 20
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/events/{}?scope=following", ids[2]), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(event_count(&app, &cookie).await, 2); // Jul 6 & 13 remain
}

#[sqlx::test]
async fn delete_scope_series_removes_all(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let ids = make_weekly_series(&app, &cookie).await;
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/events/{}?scope=series", ids[0]), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(event_count(&app, &cookie).await, 0);
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

#[sqlx::test]
async fn update_series_following_shifts_and_splits(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let ids = make_weekly_series(&app, &cookie).await; // Jul 6,13,20,27 @10:00

    // original series_id (from the first occurrence)
    let (_, first) = send(&app, json_req("GET", &format!("/api/events/{}", ids[0]), json!(null)).with_cookie(&cookie)).await;
    let orig_sid = first["series_id"].as_str().unwrap().to_string();

    // "this and following" on Jul 20: +1h, rename
    let (status, updated) = send(
        &app,
        json_req(
            "PATCH",
            &format!("/api/events/{}/series?scope=following", ids[2]),
            json!({"title":"Renamed","notes":null,"project_id":null,"contact_id":null,"all_day":false,"shift_seconds":3600}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = updated.as_array().unwrap();
    assert_eq!(arr.len(), 2); // Jul 20 & 27
    for r in arr {
        assert_eq!(r["title"], "Renamed");
        assert_ne!(r["series_id"].as_str().unwrap(), orig_sid); // split → new series_id
        assert!(r["starts_at"].as_str().unwrap().contains("11:00:00")); // 10:00 + 1h
    }

    // earlier occurrence untouched
    let (_, jul6) = send(&app, json_req("GET", &format!("/api/events/{}", ids[0]), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(jul6["title"], "W");
    assert_eq!(jul6["series_id"].as_str().unwrap(), orig_sid);
    assert!(jul6["starts_at"].as_str().unwrap().contains("10:00:00"));
}

#[sqlx::test]
async fn update_series_all_shifts_every_row(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let ids = make_weekly_series(&app, &cookie).await;
    let (status, updated) = send(
        &app,
        json_req(
            "PATCH",
            &format!("/api/events/{}/series?scope=series", ids[0]),
            json!({"title":"All","notes":null,"project_id":null,"contact_id":null,"all_day":false,"shift_seconds":-3600}),
        )
        .with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = updated.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    for r in arr {
        assert_eq!(r["title"], "All");
        assert!(r["starts_at"].as_str().unwrap().contains("09:00:00")); // 10:00 - 1h
    }
}

#[sqlx::test]
async fn following_split_survives_later_entire_series_edit(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let ids = make_weekly_series(&app, &cookie).await; // Jul 6,13,20,27 @10:00

    let (_, first) = send(&app, json_req("GET", &format!("/api/events/{}", ids[0]), json!(null)).with_cookie(&cookie)).await;
    let orig_sid = first["series_id"].as_str().unwrap().to_string();

    // "this and following" from Jul 20: +1h, splits the tail to a new series_id
    let (_, tail) = send(&app, json_req("PATCH", &format!("/api/events/{}/series?scope=following", ids[2]),
        json!({"title":"Tail","notes":null,"project_id":null,"contact_id":null,"all_day":false,"shift_seconds":3600})).with_cookie(&cookie)).await;
    let new_sid = tail.as_array().unwrap()[0]["series_id"].as_str().unwrap().to_string();
    assert_ne!(new_sid, orig_sid);

    // "entire series" on the ORIGINAL series (via ids[0]): -1h, rename — must touch ONLY the pre-split rows
    let (_, updated) = send(&app, json_req("PATCH", &format!("/api/events/{}/series?scope=series", ids[0]),
        json!({"title":"Head","notes":null,"project_id":null,"contact_id":null,"all_day":false,"shift_seconds":-3600})).with_cookie(&cookie)).await;
    let arr = updated.as_array().unwrap();
    assert_eq!(arr.len(), 2); // only Jul 6 & 13 remain on orig_sid
    for r in arr {
        assert_eq!(r["title"], "Head");
        assert!(r["starts_at"].as_str().unwrap().contains("09:00:00"));
    }

    // the split tail (Jul 20 & 27) is untouched by the second edit
    let (_, jul20) = send(&app, json_req("GET", &format!("/api/events/{}", ids[2]), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(jul20["title"], "Tail");
    assert_eq!(jul20["series_id"].as_str().unwrap(), new_sid);
    assert!(jul20["starts_at"].as_str().unwrap().contains("11:00:00"));
}

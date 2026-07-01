# Recurring Events Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user create daily/weekly/monthly recurring calendar events, and edit or delete a single occurrence, a range ("this and following"), or the whole series.

**Architecture:** Every occurrence is a concrete `event` row; rows of one series share a `series_id` (uuid, nullable). Occurrences are generated on the **client** (DST-correct, reuses the frontend's existing local↔UTC conversion) and inserted via one batch endpoint. Per-occurrence edits are plain row edits; range edits are a single scoped SQL `UPDATE` that sets content absolutely and shifts times by a fixed interval — no stored recurrence rule, no read-time expansion.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8 / Postgres), Svelte 5 (runes), Vitest.

## Global Constraints

- **Recurrence patterns:** Daily, Weekly (one weekday), Monthly (one day-of-month) only. No intervals, no multi-weekday, no RRULE.
- **End date required** for any recurring event. No infinite series.
- **Cap: 366 occurrences** per series (server rejects more with 400; client blocks before sending).
- **Monthly on the 31st skips** months without that day (no clamp).
- **`series_id` null = standalone event.** All existing rows stay null and behave exactly as today.
- Reuse existing validation (`title` non-empty, `ends_at >= starts_at`) — do not invent new messages.
- Follow existing code style: no new dependencies, sqlx `query_as::<_, Event>` with `returning *` / `select *`, tests via `#[sqlx::test]` + `tests/helpers.rs`.

**Prerequisite:** Backend integration tests need Postgres reachable via `DATABASE_URL` (the existing suite already relies on this). `#[sqlx::test]` auto-applies everything in `migrations/`. Frontend tests run from `frontend/` via `npm test`.

---

### Task 1: Add `series_id` column and model field

**Files:**
- Create: `migrations/0004_recurring.sql`
- Modify: `src/models.rs` (the `Event` struct, ~line 74)
- Test: `tests/events.rs` (add one test)

**Interfaces:**
- Produces: `event.series_id uuid` column (nullable) with index `event_series_idx`; `Event.series_id: Option<Uuid>` field, serialized as `series_id` in JSON.

- [ ] **Step 1: Write the failing test**

Add to `tests/events.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test events created_event_exposes_null_series_id`
Expected: FAIL — `series_id` key absent from the response object (the assertion `contains_key` fails).

- [ ] **Step 3: Create the migration**

`migrations/0004_recurring.sql`:

```sql
alter table event add column series_id uuid;
create index event_series_idx on event(series_id);
```

- [ ] **Step 4: Add the field to the model**

In `src/models.rs`, add `series_id` to the `Event` struct (keep it last so column names still map by name):

```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub project_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub series_id: Option<Uuid>,
}
```

(No change to `EventInput` — the server assigns `series_id`, clients never send it.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test events`
Expected: PASS — the new test plus all existing event tests (existing `create`/`update` use `returning *`, so the new nullable column maps to `None` automatically).

- [ ] **Step 6: Commit**

```bash
git add migrations/0004_recurring.sql src/models.rs tests/events.rs
git commit -m "feat(events): add nullable series_id column + Event field"
```

---

### Task 2: Batch series-create endpoint

**Files:**
- Modify: `src/models.rs` (add `SeriesInput`)
- Modify: `src/routes/events.rs` (add `create_series`)
- Modify: `src/app.rs` (register route, ~line 48)
- Test: `tests/events.rs`

**Interfaces:**
- Consumes: `Event.series_id` (Task 1).
- Produces: `POST /api/events/series` — body `{ "occurrences": [EventInput, …] }` → `201` + JSON array of inserted `Event` rows (all sharing one fresh `series_id`), in insertion order. `400` on empty list, `> 366` rows, empty title, `ends_at < starts_at`, or bad `project_id`/`contact_id`.

- [ ] **Step 1: Write the failing tests**

Add to `tests/events.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test events create_series`
Expected: FAIL — route `/api/events/series` not found (404), so the status asserts fail.

- [ ] **Step 3: Add the input struct**

In `src/models.rs`, after `EventInput`:

```rust
#[derive(Debug, Deserialize)]
pub struct SeriesInput {
    pub occurrences: Vec<EventInput>,
}
```

- [ ] **Step 4: Add the handler**

In `src/routes/events.rs`, update the `models` import and add the handler. Change the existing import line to:

```rust
use crate::models::{Event, EventInput, SeriesInput};
```

Append:

```rust
pub async fn create_series(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<SeriesInput>,
) -> Result<(StatusCode, Json<Vec<Event>>), AppError> {
    if input.occurrences.is_empty() {
        return Err(AppError::BadRequest("occurrences must not be empty".into()));
    }
    if input.occurrences.len() > 366 {
        return Err(AppError::BadRequest("too many occurrences (max 366)".into()));
    }
    for occ in &input.occurrences {
        if occ.title.trim().is_empty() {
            return Err(AppError::BadRequest("title is required".into()));
        }
        if occ.ends_at < occ.starts_at {
            return Err(AppError::BadRequest("ends_at must be >= starts_at".into()));
        }
    }

    let series_id = Uuid::new_v4();
    let mut tx = s.pool.begin().await?;
    let mut rows = Vec::with_capacity(input.occurrences.len());
    for occ in &input.occurrences {
        let all_day = occ.all_day.unwrap_or(false);
        let row = sqlx::query_as::<_, Event>(
            "insert into event (title, starts_at, ends_at, all_day, project_id, contact_id, notes, series_id) \
             values ($1,$2,$3,$4,$5,$6,$7,$8) returning *",
        )
        .bind(&occ.title)
        .bind(occ.starts_at)
        .bind(occ.ends_at)
        .bind(all_day)
        .bind(occ.project_id)
        .bind(occ.contact_id)
        .bind(&occ.notes)
        .bind(series_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
                AppError::BadRequest("project_id or contact_id does not exist".into())
            }
            other => AppError::Db(other),
        })?;
        rows.push(row);
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(rows)))
}
```

- [ ] **Step 5: Register the route**

In `src/app.rs`, add this line immediately after the `/api/events` route (line ~48). A static segment (`series`) takes priority over the `{id}` capture in axum, so ordering is safe, but keep it adjacent for readability:

```rust
        .route("/api/events/series", post(events::create_series))
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test events create_series`
Expected: PASS (all three).

- [ ] **Step 7: Commit**

```bash
git add src/models.rs src/routes/events.rs src/app.rs tests/events.rs
git commit -m "feat(events): batch series-create endpoint"
```

---

### Task 3: Scoped delete

**Files:**
- Modify: `src/routes/events.rs` (extend `delete`)
- Test: `tests/events.rs`

**Interfaces:**
- Consumes: `POST /api/events/series` (Task 2) to set up series in tests; `Event.series_id`.
- Produces: `DELETE /api/events/{id}?scope=one|following|series` → `204`. `one` (and no/unknown scope) deletes just that row. `following` deletes rows with the same `series_id` and `starts_at >= target.starts_at`. `series` deletes all rows with the same `series_id`. `404` if the target id doesn't exist; `400` if `following`/`series` used on a row whose `series_id` is null.

- [ ] **Step 1: Write the failing tests**

Add to `tests/events.rs`. This helper inline creates a 4-Monday weekly series and returns the ids:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test events delete_scope`
Expected: FAIL — `scope` is ignored today, so `following`/`series` delete only one row and the counts are wrong (3 instead of 2, 3 instead of 0).

- [ ] **Step 3: Extend the delete handler**

In `src/routes/events.rs`, add a query struct and replace the `delete` function:

```rust
#[derive(Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

pub async fn delete(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<ScopeQuery>,
) -> Result<StatusCode, AppError> {
    let res = match q.scope.as_deref() {
        Some("following") | Some("series") => {
            let target = sqlx::query_as::<_, Event>("select * from event where id = $1")
                .bind(id)
                .fetch_optional(&s.pool)
                .await?
                .ok_or(AppError::NotFound)?;
            let sid = target
                .series_id
                .ok_or_else(|| AppError::BadRequest("event is not part of a series".into()))?;
            if q.scope.as_deref() == Some("following") {
                sqlx::query("delete from event where series_id = $1 and starts_at >= $2")
                    .bind(sid)
                    .bind(target.starts_at)
                    .execute(&s.pool)
                    .await?
            } else {
                sqlx::query("delete from event where series_id = $1")
                    .bind(sid)
                    .execute(&s.pool)
                    .await?
            }
        }
        _ => {
            sqlx::query("delete from event where id = $1")
                .bind(id)
                .execute(&s.pool)
                .await?
        }
    };
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
```

(`Query`, `Deserialize`, `Event` are already imported at the top of the file.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test events`
Expected: PASS — new scope tests plus the existing `event_crud` delete (no scope → single-row path, still `204`).

- [ ] **Step 5: Commit**

```bash
git add src/routes/events.rs tests/events.rs
git commit -m "feat(events): scoped delete (one/following/series)"
```

---

### Task 4: Scoped series edit (content + time shift)

**Files:**
- Modify: `src/models.rs` (add `SeriesUpdateInput`)
- Modify: `src/routes/events.rs` (add `update_series`)
- Modify: `src/app.rs` (register route)
- Test: `tests/events.rs`

**Interfaces:**
- Consumes: `POST /api/events/series`, `Event.series_id`.
- Produces: `PATCH /api/events/{id}/series?scope=following|series` — body `{ "title": String, "notes": Option<String>, "project_id": Option<Uuid>, "contact_id": Option<Uuid>, "all_day": Option<bool>, "shift_seconds": i64 }` → `200` + JSON array of the updated `Event` rows. Sets content fields absolutely; shifts `starts_at`/`ends_at` by `shift_seconds`. `following` targets rows with the target's `series_id` where `starts_at >= target.starts_at` and stamps them with a **new** `series_id`; `series` targets all rows with that `series_id` and keeps it. `404` unknown id; `400` empty title, non-series row, or scope other than following/series.

- [ ] **Step 1: Write the failing tests**

Add to `tests/events.rs` (reuses `make_weekly_series` from Task 3):

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test events update_series`
Expected: FAIL — route `/api/events/{id}/series` not found (404).

- [ ] **Step 3: Add the input struct**

In `src/models.rs`, after `SeriesInput`:

```rust
#[derive(Debug, Deserialize)]
pub struct SeriesUpdateInput {
    pub title: String,
    pub notes: Option<String>,
    pub project_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub all_day: Option<bool>,
    pub shift_seconds: i64,
}
```

- [ ] **Step 4: Add the handler**

In `src/routes/events.rs`, extend the import to include `SeriesUpdateInput`:

```rust
use crate::models::{Event, EventInput, SeriesInput, SeriesUpdateInput};
```

Append the handler (reuses `ScopeQuery` from Task 3):

```rust
pub async fn update_series(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<ScopeQuery>,
    Json(input): Json<SeriesUpdateInput>,
) -> Result<Json<Vec<Event>>, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let target = sqlx::query_as::<_, Event>("select * from event where id = $1")
        .bind(id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let sid = target
        .series_id
        .ok_or_else(|| AppError::BadRequest("event is not part of a series".into()))?;
    let all_day = input.all_day.unwrap_or(false);

    let map_fk = |e: sqlx::Error| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id or contact_id does not exist".into())
        }
        other => AppError::Db(other),
    };

    let rows = match q.scope.as_deref() {
        Some("series") => sqlx::query_as::<_, Event>(
            "update event set title=$2, notes=$3, project_id=$4, contact_id=$5, all_day=$6, \
             starts_at = starts_at + ($7::bigint * interval '1 second'), \
             ends_at   = ends_at   + ($7::bigint * interval '1 second') \
             where series_id=$1 returning *",
        )
        .bind(sid)
        .bind(&input.title)
        .bind(&input.notes)
        .bind(input.project_id)
        .bind(input.contact_id)
        .bind(all_day)
        .bind(input.shift_seconds)
        .fetch_all(&s.pool)
        .await
        .map_err(map_fk)?,

        Some("following") => {
            let new_sid = Uuid::new_v4();
            sqlx::query_as::<_, Event>(
                "update event set title=$2, notes=$3, project_id=$4, contact_id=$5, all_day=$6, \
                 starts_at = starts_at + ($7::bigint * interval '1 second'), \
                 ends_at   = ends_at   + ($7::bigint * interval '1 second'), \
                 series_id = $8 \
                 where series_id=$1 and starts_at >= $9 returning *",
            )
            .bind(sid)
            .bind(&input.title)
            .bind(&input.notes)
            .bind(input.project_id)
            .bind(input.contact_id)
            .bind(all_day)
            .bind(input.shift_seconds)
            .bind(new_sid)
            .bind(target.starts_at)
            .fetch_all(&s.pool)
            .await
            .map_err(map_fk)?
        }

        _ => return Err(AppError::BadRequest("scope must be 'following' or 'series'".into())),
    };
    Ok(Json(rows))
}
```

- [ ] **Step 5: Register the route**

In `src/app.rs`, add after the `/api/events/{id}` route block:

```rust
        .route("/api/events/{id}/series", patch(events::update_series))
```

(`patch` is already imported — it's used by `projects::move_`.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test events update_series`
Expected: PASS (both). Then run the whole suite: `cargo test` → all green.

- [ ] **Step 7: Commit**

```bash
git add src/models.rs src/routes/events.rs src/app.rs tests/events.rs
git commit -m "feat(events): scoped series edit (content + time shift, following splits series_id)"
```

---

### Task 5: `generateOccurrences` (client generation)

**Files:**
- Modify: `frontend/src/lib/calendar.js` (add export)
- Test: `frontend/src/lib/calendar.test.js`

**Interfaces:**
- Produces: `generateOccurrences({ start, end, freq, until }) → [{ starts_at, ends_at }, …]` where `start`/`end` are local `Date` objects for the first occurrence, `freq` is `'daily' | 'weekly' | 'monthly'`, `until` is a local `Date` (inclusive through end-of-day). Returns UTC RFC3339 strings. Preserves each occurrence's local wall-clock time; monthly infers the day-of-month from `start` and **skips** months lacking that day.

- [ ] **Step 1: Write the failing tests**

Add to `frontend/src/lib/calendar.test.js` (extend the import on line 2 to include `generateOccurrences`):

```js
import { monthMatrix, monthRange, eventsByDay, fmtTime, generateOccurrences } from './calendar.js'
```

Then append:

```js
describe('generateOccurrences', () => {
  it('daily: one per day, until inclusive', () => {
    const occ = generateOccurrences({
      start: new Date(2026, 6, 6, 9, 0),
      end: new Date(2026, 6, 6, 10, 0),
      freq: 'daily',
      until: new Date(2026, 6, 8),
    })
    expect(occ).toHaveLength(3) // Jul 6, 7, 8
  })

  it('until is inclusive at a single day', () => {
    const occ = generateOccurrences({
      start: new Date(2026, 6, 6, 9, 0),
      end: new Date(2026, 6, 6, 10, 0),
      freq: 'daily',
      until: new Date(2026, 6, 6),
    })
    expect(occ).toHaveLength(1)
  })

  it('weekly: same weekday each week', () => {
    const occ = generateOccurrences({
      start: new Date(2026, 6, 6, 12, 0), // Mon Jul 6
      end: new Date(2026, 6, 6, 13, 0),
      freq: 'weekly',
      until: new Date(2026, 6, 27), // Mon Jul 27
    })
    expect(occ).toHaveLength(4)
    for (const o of occ) {
      expect(new Date(o.starts_at).getDay()).toBe(1) // Monday
    }
  })

  it('monthly: same day-of-month', () => {
    const occ = generateOccurrences({
      start: new Date(2026, 0, 15, 9, 0), // Jan 15
      end: new Date(2026, 0, 15, 10, 0),
      freq: 'monthly',
      until: new Date(2026, 2, 31), // Mar 31
    })
    expect(occ).toHaveLength(3) // Jan, Feb, Mar
    for (const o of occ) {
      expect(new Date(o.starts_at).getDate()).toBe(15)
    }
  })

  it('monthly on the 31st skips months without that day', () => {
    const occ = generateOccurrences({
      start: new Date(2026, 0, 31, 9, 0), // Jan 31
      end: new Date(2026, 0, 31, 10, 0),
      freq: 'monthly',
      until: new Date(2026, 3, 30), // Apr 30
    })
    // Jan 31 yes; Feb (no 31) skip; Mar 31 yes; Apr (no 31) skip
    expect(occ.map((o) => new Date(o.starts_at).getMonth())).toEqual([0, 2])
  })

  it('preserves the event duration on every occurrence', () => {
    const occ = generateOccurrences({
      start: new Date(2026, 6, 6, 9, 0),
      end: new Date(2026, 6, 6, 10, 30), // 90 min
      freq: 'daily',
      until: new Date(2026, 6, 8),
    })
    for (const o of occ) {
      const mins = (new Date(o.ends_at) - new Date(o.starts_at)) / 60000
      expect(mins).toBe(90)
    }
  })

  it('weekly keeps the same local hour across a DST boundary (portable)', () => {
    // Spans the US spring-forward (Mar 8 2026). setDate preserves wall-clock,
    // so every occurrence stays at the same local hour regardless of machine tz.
    const occ = generateOccurrences({
      start: new Date(2026, 2, 2, 12, 0), // Mar 2
      end: new Date(2026, 2, 2, 13, 0),
      freq: 'weekly',
      until: new Date(2026, 3, 6), // Apr 6
    })
    for (const o of occ) {
      expect(new Date(o.starts_at).getHours()).toBe(12)
      expect(new Date(o.ends_at).getHours()).toBe(13)
    }
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run (from `frontend/`): `npm test -- calendar`
Expected: FAIL — `generateOccurrences is not a function` (not yet exported).

- [ ] **Step 3: Implement the function**

Append to `frontend/src/lib/calendar.js`:

```js
/**
 * generateOccurrences({start, end, freq, until}) → [{starts_at, ends_at}, …]
 * start,end: local Date of the FIRST occurrence. until: local Date, inclusive
 * (through that day's 23:59). freq: 'daily' | 'weekly' | 'monthly'.
 * Returns UTC RFC3339 strings, preserving each occurrence's local wall-clock
 * time (DST-safe via local Date arithmetic). Monthly infers the day-of-month
 * from `start` and SKIPS months without that day (e.g. the 31st in February).
 */
export function generateOccurrences({ start, end, freq, until }) {
  const durationMs = end.getTime() - start.getTime()
  const cutoff = new Date(until.getFullYear(), until.getMonth(), until.getDate(), 23, 59, 59, 999)
  const out = []
  const push = (s) => out.push({
    starts_at: s.toISOString(),
    ends_at: new Date(s.getTime() + durationMs).toISOString(),
  })

  if (freq === 'daily' || freq === 'weekly') {
    const step = freq === 'daily' ? 1 : 7
    const cur = new Date(start)
    while (cur <= cutoff) {
      push(new Date(cur))
      cur.setDate(cur.getDate() + step) // preserves local wall-clock across DST
    }
  } else if (freq === 'monthly') {
    const day = start.getDate()
    const hh = start.getHours()
    const mm = start.getMinutes()
    let y = start.getFullYear()
    let m = start.getMonth()
    while (new Date(y, m, 1) <= cutoff) {
      const cand = new Date(y, m, day, hh, mm, 0, 0)
      // cand.getMonth() !== m means the day rolled over → month lacks that day → skip
      if (cand.getMonth() === m && cand <= cutoff) push(cand)
      m += 1
      if (m > 11) { m = 0; y += 1 }
    }
  }
  return out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run (from `frontend/`): `npm test -- calendar`
Expected: PASS — all `generateOccurrences` tests plus the existing calendar tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/calendar.js frontend/src/lib/calendar.test.js
git commit -m "feat(calendar): generateOccurrences (daily/weekly/monthly, DST-safe)"
```

---

### Task 6: Recurring-create UI

**Files:**
- Modify: `frontend/src/routes/Calendar.svelte`

**Interfaces:**
- Consumes: `generateOccurrences` (Task 5), `POST /api/events/series` (Task 2).
- Produces: the "new event" modal gains **Repeat** (None/Daily/Weekly/Monthly) and a required **Ends** date; saving a repeating event posts the generated occurrences to `/api/events/series`.

> No Svelte component-test harness exists in this repo (Vitest only covers `lib/`), so this task verifies via the dev server. The generation logic it depends on is already unit-tested in Task 5.

- [ ] **Step 1: Import the generator**

In `frontend/src/routes/Calendar.svelte`, add `generateOccurrences` to the calendar import:

```js
import { monthMatrix, monthRange, eventsByDay, fmtTime, toISODate, generateOccurrences } from '../lib/calendar.js'
```

- [ ] **Step 2: Add repeat fields to the new-event skeleton**

In `newEvSkeleton(iso)`, add two fields:

```js
  function newEvSkeleton(iso) {
    return {
      title: '',
      all_day: false,
      starts_at_local: `${iso}T09:00`,
      ends_at_local: `${iso}T10:00`,
      starts_date: iso,
      ends_date: iso,
      project_id: '',
      contact_id: '',
      notes: '',
      repeat: 'none',
      until: '',
    }
  }
```

- [ ] **Step 3: Handle the recurring branch in `saveModal`**

Replace the entire `try { … } catch (err) { modalError = err.message }` block inside `saveModal` (the last statement of the function, right after `modalError = ''`) with the following. Do not touch the earlier title / `ends >= start` guards or the `modalError = ''` line above it:

```js
    try {
      if (modal.mode === 'new') {
        if (ev.repeat && ev.repeat !== 'none') {
          if (!ev.until) { modalError = 'End date is required for repeats'; return }
          const start = new Date(body.starts_at)
          const end = new Date(body.ends_at)
          const until = new Date(`${ev.until}T00:00:00`)
          if (until < start) { modalError = 'End date must be on or after the start'; return }
          const gen = generateOccurrences({ start, end, freq: ev.repeat, until })
          if (gen.length === 0) { modalError = 'No occurrences in that range'; return }
          if (gen.length > 366) { modalError = 'Too many occurrences (max 366)'; return }
          const { title, all_day, project_id, contact_id, notes } = body
          const occurrences = gen.map((o) => ({
            title, all_day, project_id, contact_id, notes,
            starts_at: o.starts_at, ends_at: o.ends_at,
          }))
          await api.post('/api/events/series', { occurrences })
        } else {
          await api.post('/api/events', body)
        }
      } else {
        await api.put(`/api/events/${ev.id}`, body)
      }
      modal = null
      const { from, to } = monthRange(year, monthIndex)
      events = await api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`)
    } catch (err) { modalError = err.message }
```

(The edit branch stays `api.put` for now; Task 7 replaces it.)

- [ ] **Step 4: Add the Repeat/Ends controls to the form**

In the modal `<form>`, insert this block immediately **after** the date/time grid (the `{#if modal.ev.all_day} … {:else} … {/if}` block) and **before** the Project `<div>`:

```svelte
      {#if modal.mode === 'new'}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <p class="label mb-1.5">Repeat</p>
            <select bind:value={modal.ev.repeat} class={FIELD}>
              <option value="none">Does not repeat</option>
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
              <option value="monthly">Monthly</option>
            </select>
          </div>
          {#if modal.ev.repeat !== 'none'}
            <div>
              <p class="label mb-1.5">Ends</p>
              <input type="date" bind:value={modal.ev.until} required class={FIELD} />
            </div>
          {/if}
        </div>
      {/if}
```

- [ ] **Step 5: Verify in the browser**

Run (from `frontend/`): `npm run dev`, then in the app:
1. Click a day → "new event". Set a title, a start time, **Repeat = Weekly**, **Ends** ~3 weeks out. Create.
2. Expected: a chip appears on that weekday in each week through the end date (4 chips across 4 weeks).
3. Set **Repeat = None** and create a normal event → still a single chip (regression).
4. Run `npm test` → all lib tests still pass.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/routes/Calendar.svelte
git commit -m "feat(calendar): recurring-event create UI (repeat + ends → series)"
```

---

### Task 7: Edit/delete scope UI

**Files:**
- Modify: `frontend/src/routes/Calendar.svelte`

**Interfaces:**
- Consumes: `DELETE /api/events/{id}?scope=…` (Task 3), `PATCH /api/events/{id}/series?scope=…` (Task 4). Relies on `Event.series_id` now present in list responses.
- Produces: editing/deleting an event that belongs to a series shows an "Apply to" scope selector (This event only / This and following / Entire series) and routes to the matching endpoint; time changes across a range are sent as a `shift_seconds` delta.

- [ ] **Step 1: Carry `series_id`, original start, and scope into the edit modal**

In `openEdit`, add three fields to the `ev` object built for `mode: 'edit'`:

```js
    modal = {
      mode: 'edit',
      ev: {
        id: ev.id,
        title: ev.title,
        all_day: ev.all_day,
        starts_at_local: toLocal(ev.starts_at),
        ends_at_local: toLocal(ev.ends_at),
        starts_date: toDate(ev.starts_at),
        ends_date: toDate(ev.ends_at),
        project_id: ev.project_id ?? '',
        contact_id: ev.contact_id ?? '',
        notes: ev.notes ?? '',
        series_id: ev.series_id ?? null,
        orig_starts_at: ev.starts_at,
        scope: 'one',
      },
    }
```

- [ ] **Step 2: Route the edit save by scope**

In `saveModal`, replace the edit branch `await api.put(\`/api/events/${ev.id}\`, body)` with:

```js
      } else {
        if (ev.series_id && ev.scope !== 'one') {
          const shift_seconds = Math.round(
            (new Date(body.starts_at).getTime() - new Date(ev.orig_starts_at).getTime()) / 1000
          )
          await api.patch(`/api/events/${ev.id}/series?scope=${ev.scope}`, {
            title: body.title,
            notes: body.notes,
            project_id: body.project_id,
            contact_id: body.contact_id,
            all_day: body.all_day,
            shift_seconds,
          })
        } else {
          await api.put(`/api/events/${ev.id}`, body)
        }
      }
```

- [ ] **Step 3: Route the delete by scope**

Replace `deleteEvent` with:

```js
  async function deleteEvent() {
    const ev = modal.ev
    const scope = ev.series_id ? ev.scope : 'one'
    try {
      await api.del(`/api/events/${ev.id}?scope=${scope}`)
      modal = null
      const { from, to } = monthRange(year, monthIndex)
      events = await api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`)
    } catch (err) { modalError = err.message }
  }
```

- [ ] **Step 4: Add the scope selector to the form**

Insert immediately **before** the button row (`<div class="flex gap-2 pt-1">`):

```svelte
      {#if modal.mode === 'edit' && modal.ev.series_id}
        <div>
          <p class="label mb-1.5">Apply to</p>
          <select bind:value={modal.ev.scope} class={FIELD}>
            <option value="one">This event only</option>
            <option value="following">This and following</option>
            <option value="series">Entire series</option>
          </select>
        </div>
      {/if}
```

- [ ] **Step 5: Verify in the browser**

Run (from `frontend/`): `npm run dev`, then:
1. Create a weekly series (Task 6). Click a middle occurrence.
2. **Delete → "This and following"**: that occurrence and later ones disappear; earlier remain.
3. Re-create a series. Edit a middle occurrence, change the start time by +1h, **Apply to = This and following**, save. Expected: that occurrence and later ones shift +1h; earlier keep the original time.
4. Edit a single occurrence with **Apply to = This event only**, change the title → only that chip changes.
5. Edit a **non-recurring** event → no "Apply to" selector appears; save/delete behave as before (regression).

- [ ] **Step 6: Commit**

```bash
git add frontend/src/routes/Calendar.svelte
git commit -m "feat(calendar): edit/delete scope (this/following/series)"
```

---

## Final verification

- [ ] Backend: `cargo test` → all green (existing + new event tests).
- [ ] Frontend: from `frontend/`, `npm test` → all green; `npm run build` → succeeds.
- [ ] Manual smoke: create weekly + monthly series; edit "this and following" time shift; delete "entire series".

## Self-review notes (traceability to spec)

- Spec §1 data model → Task 1.
- Spec §2 client generation (DST, monthly-skip) → Task 5.
- Spec §3 batch create + scoped delete → Tasks 2, 3.
- Spec §4 scoped edit (content + shift, following splits series_id) → Task 4 (backend) + Task 7 (UI).
- Spec §5 form (Repeat + required Ends) → Task 6.
- Spec §6 tests → Rust tests in Tasks 1–4; JS tests in Task 5; manual verify for Svelte UI (no component harness in repo).

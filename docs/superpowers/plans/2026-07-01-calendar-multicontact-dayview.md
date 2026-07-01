# Calendar: Multi-Contact + Day View + Time-Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attach multiple contacts to a calendar event, fix the "event time always 00:00" bug, and add a single-day agenda view.

**Architecture:** A new `event_contact` junction replaces the single `event.contact_id` (reads aggregate `contact_ids` via `array_agg`; writes replace links in a transaction). The buggy combined `datetime-local` field is split into a date + start/end time (time can no longer silently reset to 00:00). A `view: 'month' | 'day'` toggle adds a chronological day agenda that reuses the existing range fetch.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8 / Postgres), Svelte 5 (runes), Vitest.

## Global Constraints

- **Multi-contact = junction table** `event_contact(event_id, contact_id)`; migrate the existing single `contact_id` into it, then **drop** `event.contact_id`.
- API shape: `Event`/`EventInput`/`SeriesUpdateInput` drop `contact_id`, add **`contact_ids: Vec<Uuid>`** (serde `default` → omitted = `[]`).
- Every write path (create, update, series create, scoped series edit) maintains the junction inside a transaction. Reads aggregate `contact_ids`.
- Bad `contact_id`/`project_id` (FK violation) → **400**, message `"project_id or contact_id does not exist"` (existing text).
- **Timed events are single-day**: one `Date` + `Start` time + `End` time. All-day keeps its start-date/end-date range. Multi-day *timed* events are out of scope.
- Recurrence, series `series_id` split semantics, the 366 cap, and DST behavior are unchanged by this work — do not regress them.
- Reuse existing validation messages (`"title is required"`, `"ends_at must be >= starts_at"` / `"End must be >= start"`). No new dependencies.
- Follow existing patterns: sqlx `query_as::<_, Event>`, `#[sqlx::test]` + `tests/helpers.rs`; Svelte form uses the shared `FIELD` class and `.label`.

**Prerequisites:** Backend tests need Postgres via `DATABASE_URL`; `#[sqlx::test]` auto-applies `migrations/`. Frontend tests/build run from `frontend/` (`npm test`, `npm run build`). No Svelte component-test harness exists — frontend UI tasks verify via `npm run build` + the lib suite; pure helpers get Vitest tests.

---

### Task 1: Time-fix — split the datetime-local into Date + Start + End

**Files:**
- Modify: `frontend/src/lib/calendar.js` (add `localDateTimeToUtc`)
- Modify: `frontend/src/routes/Calendar.svelte` (`newEvSkeleton`, `openEdit`, `buildBody`, the timed form inputs, the range-edit lock)
- Test: `frontend/src/lib/calendar.test.js`

**Interfaces:**
- Produces: `localDateTimeToUtc(dateStr, timeStr) → RFC3339 UTC string` — composes a local `YYYY-MM-DD` + `HH:MM` into a UTC instant. The timed create/edit form now stores `date`, `start_time`, `end_time` on `modal.ev` instead of `starts_at_local`/`ends_at_local`.

- [ ] **Step 1: Write the failing test**

Add to `frontend/src/lib/calendar.test.js` (extend the import on line 2 to include `localDateTimeToUtc`):

```js
import { monthMatrix, monthRange, eventsByDay, fmtTime, generateOccurrences, localDateTimeToUtc } from './calendar.js'
```

Append:

```js
describe('localDateTimeToUtc', () => {
  it('composes a local date + time into a UTC instant that round-trips to the same local time', () => {
    const iso = localDateTimeToUtc('2026-07-01', '14:30')
    // The stored instant, read back in local time, must still be 14:30 — NOT 00:00.
    expect(fmtTime(iso)).toBe('14:30')
  })
  it('a non-midnight time never collapses to 00:00', () => {
    expect(fmtTime(localDateTimeToUtc('2026-07-01', '09:00'))).toBe('09:00')
    expect(fmtTime(localDateTimeToUtc('2026-12-31', '23:15'))).toBe('23:15')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run (from `frontend/`): `npm test -- calendar`
Expected: FAIL — `localDateTimeToUtc is not a function`.

- [ ] **Step 3: Implement the helper**

Append to `frontend/src/lib/calendar.js`:

```js
/**
 * localDateTimeToUtc(dateStr, timeStr) → RFC3339 UTC string.
 * dateStr 'YYYY-MM-DD', timeStr 'HH:MM' — interpreted in LOCAL time (a datetime
 * string without a timezone is local per the JS spec), so the wall-clock time is
 * preserved. Splitting date and time into separate fields is what prevents the
 * combined datetime-local widget from silently zeroing the time to 00:00.
 */
export function localDateTimeToUtc(dateStr, timeStr) {
  return new Date(`${dateStr}T${timeStr}`).toISOString()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run (from `frontend/`): `npm test -- calendar`
Expected: PASS (all calendar tests).

- [ ] **Step 5: Rework the skeleton, edit-population, and body builder**

In `frontend/src/routes/Calendar.svelte`:

Add `localDateTimeToUtc` to the calendar import:

```js
import { monthMatrix, monthRange, eventsByDay, fmtTime, toISODate, generateOccurrences, localDateTimeToUtc } from '../lib/calendar.js'
```

Replace `newEvSkeleton` (timed fields become `date`/`start_time`/`end_time`; all-day keeps `starts_date`/`ends_date`):

```js
  function newEvSkeleton(iso) {
    return {
      title: '',
      all_day: false,
      date: iso,
      start_time: '09:00',
      end_time: '10:00',
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

In `openEdit`, replace the `starts_at_local`/`ends_at_local` derivation with `date`/`start_time`/`end_time` (local basis). Replace the whole `modal = { mode: 'edit', ev: {...} }` assignment with:

```js
    const pad = (n) => String(n).padStart(2, '0')
    const localDate = (iso) => { const d = new Date(iso); return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` }
    const localTime = (iso) => { const d = new Date(iso); return `${pad(d.getHours())}:${pad(d.getMinutes())}` }
    const toDate = (iso) => iso.slice(0, 10)
    modal = {
      mode: 'edit',
      ev: {
        id: ev.id,
        title: ev.title,
        all_day: ev.all_day,
        date: localDate(ev.starts_at),
        start_time: localTime(ev.starts_at),
        end_time: localTime(ev.ends_at),
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

(Delete the old `toLocal` helper — it is replaced by `localDate`/`localTime`.)

Replace `buildBody`'s timed branch to compose from date + times:

```js
  function buildBody(ev) {
    let starts_at, ends_at
    if (ev.all_day) {
      starts_at = `${ev.starts_date}T00:00:00Z`
      ends_at = `${ev.ends_date}T23:59:59Z`
    } else {
      starts_at = localDateTimeToUtc(ev.date, ev.start_time)
      ends_at = localDateTimeToUtc(ev.date, ev.end_time)
    }
    return {
      title: ev.title.trim(),
      starts_at,
      ends_at,
      all_day: ev.all_day,
      project_id: ev.project_id || null,
      contact_id: ev.contact_id || null,
      notes: ev.notes || null,
    }
  }
```

- [ ] **Step 6: Replace the timed form inputs (and carry the range-edit lock)**

In `Calendar.svelte`, replace the `{:else}` timed grid inside the date/time block (currently the two `datetime-local` inputs) with a Date + Start + End row. A range edit (series member, scope ≠ `one`) shifts by the **start** delta only, so lock Date + End; keep Start editable:

```svelte
      {:else}
        <div class="grid grid-cols-3 gap-2">
          <div class:opacity-50={modal.ev.series_id && modal.ev.scope !== 'one'}>
            <p class="label mb-1.5">Date</p>
            <input type="date" bind:value={modal.ev.date} required class={FIELD} disabled={modal.ev.series_id && modal.ev.scope !== 'one'} />
          </div>
          <div>
            <p class="label mb-1.5">Start</p>
            <input type="time" bind:value={modal.ev.start_time} required class={FIELD} />
          </div>
          <div class:opacity-50={modal.ev.series_id && modal.ev.scope !== 'one'}>
            <p class="label mb-1.5">End</p>
            <input type="time" bind:value={modal.ev.end_time} required class={FIELD} disabled={modal.ev.series_id && modal.ev.scope !== 'one'} />
          </div>
        </div>
      {/if}
```

Update the "Apply to" hint text (it currently says "End time and all-day are locked") to also mention the date — find the hint `<p>` inside the `{#if modal.mode === 'edit' && modal.ev.series_id}` block and set its text to:

```
Range edits change title/notes and shift the start time. Date, end time and all-day are locked — use "This event only" to change them.
```

- [ ] **Step 7: Verify build + tests + browser**

Run (from `frontend/`):
- `npm test` → all lib tests pass (34 + the 2 new).
- `npm run build` → clean, zero warnings.
Then `npm run dev` and confirm: create a timed event, set Start = 14:30, save → the chip shows **14:30** (not 00:00); reopen it → Start still 14:30. Change the date → the time stays put.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/lib/calendar.js frontend/src/lib/calendar.test.js frontend/src/routes/Calendar.svelte
git commit -m "fix(calendar): split datetime-local into date + start/end time (fixes 00:00 bug)"
```

---

### Task 2: Backend multi-contact (junction table)

**Files:**
- Create: `migrations/0005_event_contacts.sql`
- Modify: `src/models.rs` (`Event`, `EventInput`, `SeriesUpdateInput`)
- Modify: `src/routes/events.rs` (all handlers + a helper)
- Test: `tests/events.rs`

**Interfaces:**
- Consumes: existing `Event`/`EventInput`/`SeriesInput`/`SeriesUpdateInput`, `ScopeQuery`.
- Produces: `event_contact(event_id, contact_id)` table; `Event.contact_ids: Vec<Uuid>`, `EventInput.contact_ids: Vec<Uuid>`, `SeriesUpdateInput.contact_ids: Vec<Uuid>`. All event write endpoints accept + persist `contact_ids`; reads return them aggregated. Internal helper `set_event_contacts(conn: &mut PgConnection, event_id: Uuid, contact_ids: &[Uuid])`.

- [ ] **Step 1: Write the failing tests**

Add to `tests/events.rs` (helpers to make a contact and read an event's `contact_ids`):

```rust
async fn make_contact(app: &axum::Router, cookie: &str, name: &str) -> String {
    let (status, c) = send(app, json_req("POST", "/api/contacts", json!({"kind":"person","name":name})).with_cookie(cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    c["id"].as_str().unwrap().to_string()
}

#[sqlx::test]
async fn create_event_with_multiple_contacts(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let a = make_contact(&app, &cookie, "Alice").await;
    let b = make_contact(&app, &cookie, "Bob").await;
    let (status, e) = send(&app, json_req("POST", "/api/events",
        json!({"title":"Sync","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","contact_ids":[a,b]})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    let ids: Vec<String> = e["contact_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&a) && ids.contains(&b));

    // list aggregates them back
    let (_, list) = send(&app, json_req("GET", "/api/events", json!(null)).with_cookie(&cookie)).await;
    let got = &list.as_array().unwrap()[0]["contact_ids"];
    assert_eq!(got.as_array().unwrap().len(), 2);
}

#[sqlx::test]
async fn event_with_no_contacts_returns_empty_array(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, e) = send(&app, json_req("POST", "/api/events",
        json!({"title":"Solo","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z"})).with_cookie(&cookie)).await;
    assert_eq!(e["contact_ids"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn update_replaces_contact_set(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let a = make_contact(&app, &cookie, "Alice").await;
    let b = make_contact(&app, &cookie, "Bob").await;
    let c = make_contact(&app, &cookie, "Carol").await;
    let (_, e) = send(&app, json_req("POST", "/api/events",
        json!({"title":"E","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","contact_ids":[a]})).with_cookie(&cookie)).await;
    let id = e["id"].as_str().unwrap().to_string();
    let (status, upd) = send(&app, json_req("PUT", &format!("/api/events/{id}"),
        json!({"title":"E","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","contact_ids":[b,c]})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<String> = upd["contact_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&b) && ids.contains(&c) && !ids.contains(&a));
}

#[sqlx::test]
async fn series_writes_contacts_on_every_occurrence(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let a = make_contact(&app, &cookie, "Alice").await;
    let body = json!({"occurrences":[
        {"title":"W","starts_at":"2026-07-06T10:00:00Z","ends_at":"2026-07-06T10:15:00Z","contact_ids":[a]},
        {"title":"W","starts_at":"2026-07-13T10:00:00Z","ends_at":"2026-07-13T10:15:00Z","contact_ids":[a]}
    ]});
    let (status, rows) = send(&app, json_req("POST", "/api/events/series", body).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::CREATED);
    for r in rows.as_array().unwrap() {
        assert_eq!(r["contact_ids"].as_array().unwrap().len(), 1);
    }
}

#[sqlx::test]
async fn scoped_series_edit_replaces_contacts_on_affected(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let a = make_contact(&app, &cookie, "Alice").await;
    let b = make_contact(&app, &cookie, "Bob").await;
    // 2-occurrence weekly series, both with contact a
    let body = json!({"occurrences":[
        {"title":"W","starts_at":"2026-07-06T10:00:00Z","ends_at":"2026-07-06T10:15:00Z","contact_ids":[a]},
        {"title":"W","starts_at":"2026-07-13T10:00:00Z","ends_at":"2026-07-13T10:15:00Z","contact_ids":[a]}
    ]});
    let (_, rows) = send(&app, json_req("POST", "/api/events/series", body).with_cookie(&cookie)).await;
    let ids: Vec<String> = rows.as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_string()).collect();
    // entire-series edit sets contacts to [b]
    let (status, updated) = send(&app, json_req("PATCH", &format!("/api/events/{}/series?scope=series", ids[0]),
        json!({"title":"W","notes":null,"project_id":null,"contact_ids":[b],"all_day":false,"shift_seconds":0})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    for r in updated.as_array().unwrap() {
        let cs: Vec<String> = r["contact_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        assert_eq!(cs, vec![b.clone()]);
    }
}

#[sqlx::test]
async fn create_event_bad_contact_id_is_400(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(&app, json_req("POST", "/api/events",
        json!({"title":"E","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","contact_ids":["00000000-0000-0000-0000-000000000000"]})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
```

Also **update the one existing test that references the removed field.**
`event_update_bad_contact_id_is_400` (currently sends `"contact_id": bogus` in a
`PUT` body and expects 400) must move the bad id into the new field, since an
unknown `contact_id` key is now silently ignored by serde (the junction insert is
what yields the FK→400). Change its body to:

```rust
            json!({"title":"X","starts_at":"2026-07-01T10:00:00Z","ends_at":"2026-07-01T11:00:00Z","contact_ids":[bogus]}),
```

(The recurring scoped-edit tests that pass `"contact_id":null` still pass — the
extra key is ignored and `contact_ids` defaults to `[]` — so leave them as-is.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test events create_event_with_multiple_contacts`
Expected: FAIL — `contact_ids` not accepted/returned (compile error on the new field, or `contact_ids` absent from responses).

- [ ] **Step 3: Migration**

`migrations/0005_event_contacts.sql`:

```sql
create table event_contact (
    event_id   uuid not null references event(id)   on delete cascade,
    contact_id uuid not null references contact(id) on delete cascade,
    primary key (event_id, contact_id)
);
create index event_contact_contact_idx on event_contact(contact_id);

-- carry the existing single link forward, then retire the column
insert into event_contact (event_id, contact_id)
    select id, contact_id from event where contact_id is not null;
alter table event drop column contact_id;
```

- [ ] **Step 4: Model changes**

In `src/models.rs`: in `Event`, remove `pub contact_id: Option<Uuid>,` and add `contact_ids` last with `#[sqlx(default)]`:

```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub project_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub series_id: Option<Uuid>,
    #[sqlx(default)]
    pub contact_ids: Vec<Uuid>,
}
```

In `EventInput`, remove `pub contact_id: Option<Uuid>,` and add:

```rust
    #[serde(default)]
    pub contact_ids: Vec<Uuid>,
```

In `SeriesUpdateInput`, remove `pub contact_id: Option<Uuid>,` and add:

```rust
    #[serde(default)]
    pub contact_ids: Vec<Uuid>,
```

- [ ] **Step 5: Rewrite `src/routes/events.rs`**

Replace the entire file with the version below (junction-aware; single-event writes now run in a transaction; reads aggregate `contact_ids`):

```rust
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Event, EventInput, SeriesInput, SeriesUpdateInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

// A FK violation on project_id (event insert) or contact_id (junction insert) → 400.
fn fk_400(e: sqlx::Error) -> AppError {
    match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id or contact_id does not exist".into())
        }
        other => AppError::Db(other),
    }
}

// Replace an event's contact links with `contact_ids` (delete-then-insert).
async fn set_event_contacts(
    conn: &mut PgConnection,
    event_id: Uuid,
    contact_ids: &[Uuid],
) -> Result<(), AppError> {
    sqlx::query("delete from event_contact where event_id = $1")
        .bind(event_id)
        .execute(&mut *conn)
        .await?;
    for cid in contact_ids {
        sqlx::query("insert into event_contact (event_id, contact_id) values ($1, $2)")
            .bind(event_id)
            .bind(cid)
            .execute(&mut *conn)
            .await
            .map_err(fk_400)?;
    }
    Ok(())
}

// Read query fragment: aggregate each event's contact_ids into a uuid[].
const AGG: &str = "select e.*, coalesce(array_agg(ec.contact_id) filter (where ec.contact_id is not null), '{}') as contact_ids \
     from event e left join event_contact ec on ec.event_id = e.id";

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Event>>, AppError> {
    let rows = match (q.from, q.to) {
        (Some(from), Some(to)) => sqlx::query_as::<_, Event>(&format!(
            "{AGG} where e.ends_at >= $1 and e.starts_at < $2 group by e.id order by e.starts_at"
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&s.pool)
        .await?,
        _ => sqlx::query_as::<_, Event>(&format!("{AGG} group by e.id order by e.starts_at"))
            .fetch_all(&s.pool)
            .await?,
    };
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<EventInput>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if input.ends_at < input.starts_at {
        return Err(AppError::BadRequest("ends_at must be >= starts_at".into()));
    }
    let all_day = input.all_day.unwrap_or(false);
    let mut tx = s.pool.begin().await?;
    let mut row = sqlx::query_as::<_, Event>(
        "insert into event (title, starts_at, ends_at, all_day, project_id, notes) \
         values ($1,$2,$3,$4,$5,$6) returning *",
    )
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(all_day)
    .bind(input.project_id)
    .bind(&input.notes)
    .fetch_one(&mut *tx)
    .await
    .map_err(fk_400)?;
    set_event_contacts(&mut tx, row.id, &input.contact_ids).await?;
    tx.commit().await?;
    row.contact_ids = input.contact_ids;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, AppError> {
    let row = sqlx::query_as::<_, Event>(&format!("{AGG} where e.id = $1 group by e.id"))
        .bind(id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<EventInput>,
) -> Result<Json<Event>, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if input.ends_at < input.starts_at {
        return Err(AppError::BadRequest("ends_at must be >= starts_at".into()));
    }
    let all_day = input.all_day.unwrap_or(false);
    let mut tx = s.pool.begin().await?;
    let row = sqlx::query_as::<_, Event>(
        "update event set title=$2, starts_at=$3, ends_at=$4, all_day=$5, project_id=$6, notes=$7 \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(all_day)
    .bind(input.project_id)
    .bind(&input.notes)
    .fetch_optional(&mut *tx)
    .await
    .map_err(fk_400)?;
    let mut row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };
    set_event_contacts(&mut tx, id, &input.contact_ids).await?;
    tx.commit().await?;
    row.contact_ids = input.contact_ids;
    Ok(Json(row))
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
        _ => sqlx::query("delete from event where id = $1")
            .bind(id)
            .execute(&s.pool)
            .await?,
    };
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

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
        let mut row = sqlx::query_as::<_, Event>(
            "insert into event (title, starts_at, ends_at, all_day, project_id, notes, series_id) \
             values ($1,$2,$3,$4,$5,$6,$7) returning *",
        )
        .bind(&occ.title)
        .bind(occ.starts_at)
        .bind(occ.ends_at)
        .bind(all_day)
        .bind(occ.project_id)
        .bind(&occ.notes)
        .bind(series_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(fk_400)?;
        set_event_contacts(&mut tx, row.id, &occ.contact_ids).await?;
        row.contact_ids = occ.contact_ids.clone();
        rows.push(row);
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(rows)))
}

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
    let mut tx = s.pool.begin().await?;
    let target = sqlx::query_as::<_, Event>("select * from event where id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    let sid = target
        .series_id
        .ok_or_else(|| AppError::BadRequest("event is not part of a series".into()))?;
    let all_day = input.all_day.unwrap_or(false);

    let mut rows = match q.scope.as_deref() {
        Some("series") => sqlx::query_as::<_, Event>(
            "update event set title=$2, notes=$3, project_id=$4, all_day=$5, \
             starts_at = starts_at + ($6::bigint * interval '1 second'), \
             ends_at   = ends_at   + ($6::bigint * interval '1 second') \
             where series_id=$1 returning *",
        )
        .bind(sid)
        .bind(&input.title)
        .bind(&input.notes)
        .bind(input.project_id)
        .bind(all_day)
        .bind(input.shift_seconds)
        .fetch_all(&mut *tx)
        .await
        .map_err(fk_400)?,

        Some("following") => {
            let new_sid = Uuid::new_v4();
            sqlx::query_as::<_, Event>(
                "update event set title=$2, notes=$3, project_id=$4, all_day=$5, \
                 starts_at = starts_at + ($6::bigint * interval '1 second'), \
                 ends_at   = ends_at   + ($6::bigint * interval '1 second'), \
                 series_id = $7 \
                 where series_id=$1 and starts_at >= $8 returning *",
            )
            .bind(sid)
            .bind(&input.title)
            .bind(&input.notes)
            .bind(input.project_id)
            .bind(all_day)
            .bind(input.shift_seconds)
            .bind(new_sid)
            .bind(target.starts_at)
            .fetch_all(&mut *tx)
            .await
            .map_err(fk_400)?
        }

        _ => return Err(AppError::BadRequest("scope must be 'following' or 'series'".into())),
    };

    for row in rows.iter_mut() {
        set_event_contacts(&mut tx, row.id, &input.contact_ids).await?;
        row.contact_ids = input.contact_ids.clone();
    }
    tx.commit().await?;
    Ok(Json(rows))
}
```

Note the `&mut tx` calls to `set_event_contacts` rely on `Transaction`'s `DerefMut` to `PgConnection`; if the compiler rejects `&mut tx`, use `&mut *tx`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test events`
Expected: PASS — the new multi-contact tests plus all existing event tests (existing tests send no `contact_ids` → `[]`; the recurring split/shift tests are unaffected because contacts default empty).

Then the whole suite: `cargo test` → all green.

- [ ] **Step 7: Commit**

```bash
git add migrations/0005_event_contacts.sql src/models.rs src/routes/events.rs tests/events.rs
git commit -m "feat(events): multiple contacts per event via event_contact junction"
```

---

### Task 3: Multi-select contacts UI

**Files:**
- Modify: `frontend/src/routes/Calendar.svelte`

**Interfaces:**
- Consumes: `Event.contact_ids` and the `contact_ids`-accepting endpoints (Task 2); `localDateTimeToUtc` / date-time form (Task 1).
- Produces: the event modal edits a `contact_ids` array (checkbox chip list); month chips + edit modal show `@name`s; create / update / series / scoped-edit send `contact_ids`.

> Verify via `npm run build` + lib suite + browser (no component harness).

- [ ] **Step 1: Carry `contact_ids` on the modal instead of `contact_id`**

In `Calendar.svelte`:

`newEvSkeleton` — replace `contact_id: ''` with `contact_ids: []`.

`openEdit` — replace `contact_id: ev.contact_id ?? ''` with `contact_ids: ev.contact_ids ?? []`.

`buildBody` — replace the `contact_id: ev.contact_id || null` line with `contact_ids: ev.contact_ids ?? []`.

- [ ] **Step 2: Send `contact_ids` in the scoped series edit**

In `saveModal`, the range-edit `api.patch(...)` body currently sends `contact_id: body.contact_id`. Replace that field with:

```js
            contact_ids: ev.contact_ids ?? [],
```

(The create / `PUT` / `/series` paths already send the whole `body`, which now carries `contact_ids` from `buildBody`.)

- [ ] **Step 3: Add a `@name` helper and a contacts checkbox list**

In the `<script>`, add a helper that maps ids → names (uses the already-fetched `contacts`):

```js
  const contactName = (id) => contacts.find((c) => c.id === id)?.name ?? '?'
  function toggleContact(id) {
    const set = modal.ev.contact_ids
    modal.ev.contact_ids = set.includes(id) ? set.filter((x) => x !== id) : [...set, id]
  }
```

Replace the existing single Contact `<select>` block in the form with a checkbox chip list:

```svelte
      <div>
        <p class="label mb-1.5">People <span class="text-faint">(optional)</span></p>
        <div class="flex flex-wrap gap-1.5">
          {#each contacts as c}
            {@const on = modal.ev.contact_ids.includes(c.id)}
            <button
              type="button"
              onclick={() => toggleContact(c.id)}
              class="rounded-sm border px-2 py-1 font-mono text-[12px] transition {on ? 'border-accent bg-accent-dim/25 text-accent' : 'border-border bg-surface-2 text-muted hover:border-accent-dim'}"
            >{on ? '✓ ' : ''}{c.name}</button>
          {/each}
          {#if contacts.length === 0}<span class="font-mono text-[12px] text-faint">// no contacts</span>{/if}
        </div>
      </div>
```

- [ ] **Step 4: Show attendees on the month chips**

In the month-grid event chip, extend the `title` tooltip to include names. Replace the chip `title="..."` attribute with:

```svelte
                title="{ev.title}{ev.all_day ? '' : ' ' + fmtTime(ev.starts_at)}{ev.contact_ids?.length ? ' — ' + ev.contact_ids.map(contactName).join(', ') : ''}"
```

- [ ] **Step 5: Verify build + tests + browser**

Run (from `frontend/`): `npm run build` (clean) and `npm test` (green). Then `npm run dev`: create an event, tick two people → chips highlight; save; reopen → both still ticked; hover the month chip → tooltip lists both names. Edit a series occurrence with "Entire series" + change people → all occurrences update.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/routes/Calendar.svelte
git commit -m "feat(calendar): attach multiple people to an event (checkbox chips)"
```

---

### Task 4: Day-by-day agenda view

**Files:**
- Modify: `frontend/src/lib/calendar.js` (add `dayRange`, `dayAgenda`)
- Modify: `frontend/src/routes/Calendar.svelte` (view state, toggle, day fetch, agenda render)
- Test: `frontend/src/lib/calendar.test.js`

**Interfaces:**
- Consumes: existing `/api/events?from=&to=`; `contactName` (Task 3); `fmtTime`.
- Produces: `dayRange(iso) → { from, to }` (local-day UTC bounds); `dayAgenda(events) → { allDay: Event[], timed: Event[] }`; a `Month | Day` toggle + day agenda in `Calendar.svelte`.

- [ ] **Step 1: Write the failing tests**

Add `dayRange`, `dayAgenda` to the `calendar.test.js` import, then append:

```js
describe('dayRange', () => {
  it('spans one local day (from local midnight to next local midnight)', () => {
    const { from, to } = dayRange('2026-07-01')
    expect(from).toBe(new Date(2026, 6, 1, 0, 0, 0).toISOString())
    expect(to).toBe(new Date(2026, 6, 2, 0, 0, 0).toISOString())
  })
})

describe('dayAgenda', () => {
  const events = [
    { id: '1', title: 'Timed late', all_day: false, starts_at: new Date(2026, 6, 1, 15, 0).toISOString() },
    { id: '2', title: 'All day', all_day: true, starts_at: new Date(2026, 6, 1, 0, 0).toISOString() },
    { id: '3', title: 'Timed early', all_day: false, starts_at: new Date(2026, 6, 1, 9, 0).toISOString() },
  ]
  it('separates all-day and sorts timed ascending by start', () => {
    const { allDay, timed } = dayAgenda(events)
    expect(allDay.map((e) => e.id)).toEqual(['2'])
    expect(timed.map((e) => e.id)).toEqual(['3', '1'])
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run (from `frontend/`): `npm test -- calendar`
Expected: FAIL — `dayRange`/`dayAgenda` not functions.

- [ ] **Step 3: Implement the helpers**

Append to `frontend/src/lib/calendar.js`:

```js
/**
 * dayRange(iso) → { from, to } RFC3339 UTC instants bounding the local day
 * `iso` ('YYYY-MM-DD'): local midnight → next local midnight. For the day fetch.
 */
export function dayRange(iso) {
  const start = new Date(`${iso}T00:00:00`)
  const end = new Date(start)
  end.setDate(start.getDate() + 1)
  return { from: start.toISOString(), to: end.toISOString() }
}

/**
 * dayAgenda(events) → { allDay, timed }. All-day events separated out; timed
 * events sorted ascending by start. Used by the day view.
 */
export function dayAgenda(events) {
  const allDay = events.filter((e) => e.all_day)
  const timed = events
    .filter((e) => !e.all_day)
    .sort((a, b) => new Date(a.starts_at) - new Date(b.starts_at))
  return { allDay, timed }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run (from `frontend/`): `npm test -- calendar`
Expected: PASS.

- [ ] **Step 5: Add view state, toggle, and day-scoped fetch**

In `Calendar.svelte`:

Import the helpers:

```js
import { monthMatrix, monthRange, eventsByDay, fmtTime, toISODate, generateOccurrences, localDateTimeToUtc, dayRange, dayAgenda } from '../lib/calendar.js'
```

Add state near the other `$state` declarations:

```js
  let view = $state('month') // 'month' | 'day'
  let selectedDate = $state(toISODate(new Date()))
```

Replace the data-loading `$effect` so it fetches the month range or the single day depending on `view`:

```js
  $effect(() => {
    const range = view === 'day' ? dayRange(selectedDate) : monthRange(year, monthIndex)
    error = ''
    Promise.all([
      api.get(`/api/events?from=${encodeURIComponent(range.from)}&to=${encodeURIComponent(range.to)}`),
      api.get('/api/projects'),
      api.get('/api/contacts'),
    ]).then(([evs, pjs, cts]) => {
      events = evs
      projects = pjs
      contacts = cts
    }).catch((e) => { error = e.message })
  })
```

Add day-nav + agenda derivation helpers in `<script>`:

```js
  const agenda = $derived(dayAgenda(events))
  function shiftDay(delta) {
    const d = new Date(`${selectedDate}T00:00:00`)
    d.setDate(d.getDate() + delta)
    selectedDate = toISODate(d)
  }
```

- [ ] **Step 6: Add the Month|Day toggle and the day agenda to the template**

In the header bar, add a toggle (place it right after the `> {monthLabel}` / nav cluster, before the `new event` button). Show the month nav only in month view and the day nav only in day view:

```svelte
  <div class="flex items-center gap-1">
    <button onclick={() => (view = 'month')}
      class="h-8 rounded-sm border px-2.5 font-mono text-[12px] transition {view === 'month' ? 'border-accent bg-accent-dim/20 text-accent' : 'border-border text-muted hover:text-ink'}">month</button>
    <button onclick={() => (view = 'day')}
      class="h-8 rounded-sm border px-2.5 font-mono text-[12px] transition {view === 'day' ? 'border-accent bg-accent-dim/20 text-accent' : 'border-border text-muted hover:text-ink'}">day</button>
  </div>
```

Wrap the existing month header nav cluster (`prev/label/today/next`) in `{#if view === 'month'} … {/if}`, and add a day nav shown in day view:

```svelte
  {#if view === 'day'}
    <div class="flex items-center gap-2">
      <button onclick={() => shiftDay(-1)} class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink" aria-label="Previous day">[ &lt; ]</button>
      <span class="font-mono text-[13px] text-accent glow-text tabular-nums">&gt; {selectedDate}</span>
      <button onclick={() => (selectedDate = toISODate(new Date()))} class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink">[ today ]</button>
      <button onclick={() => shiftDay(1)} class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink" aria-label="Next day">[ &gt; ]</button>
    </div>
  {/if}
```

Wrap the existing month-grid `<div class="rise rounded-sm border border-border bg-surface">…</div>` in `{#if view === 'month'} … {/if}`, and add the day agenda after it:

```svelte
{#if view === 'day'}
  <div class="rise rounded-sm border border-border bg-surface" style="animation-delay:40ms">
    {#if agenda.allDay.length === 0 && agenda.timed.length === 0}
      <p class="px-4 py-8 text-center font-mono text-[13px] text-faint">// no events</p>
    {:else}
      {#each agenda.allDay as ev}
        <button onclick={(e) => openEdit(ev, e)} class="flex w-full items-center gap-3 border-b border-border/50 px-4 py-2.5 text-left transition hover:bg-surface-2">
          <span class="w-14 font-mono text-[11px] text-faint">all-day</span>
          <span class="font-mono text-[13px] text-ink">{ev.title}</span>
          {#if ev.contact_ids?.length}<span class="font-mono text-[11px] text-accent">{ev.contact_ids.map(contactName).map((n) => '@' + n).join(' ')}</span>{/if}
        </button>
      {/each}
      {#each agenda.timed as ev}
        <button onclick={(e) => openEdit(ev, e)} class="flex w-full items-center gap-3 border-b border-border/50 px-4 py-2.5 text-left transition last:border-0 hover:bg-surface-2">
          <span class="w-14 font-mono text-[12px] tabular-nums text-accent">{fmtTime(ev.starts_at)}</span>
          <span class="font-mono text-[13px] text-ink">{ev.title}</span>
          {#if ev.contact_ids?.length}<span class="font-mono text-[11px] text-accent">{ev.contact_ids.map(contactName).map((n) => '@' + n).join(' ')}</span>{/if}
        </button>
      {/each}
    {/if}
  </div>
{/if}
```

Make the header `+ new event` button open the modal on the right day in day view — change its handler to:

```svelte
    onclick={() => openNew(view === 'day' ? selectedDate : todayIso)}
```

- [ ] **Step 7: Verify build + tests + browser**

Run (from `frontend/`): `npm test` (green) and `npm run build` (clean). Then `npm run dev`: toggle **day**; the agenda lists that day's events (all-day first, then by time, with `@names`); `[ < ] [ > ]` change day and the list refetches; `+ new event` opens on the selected day; clicking a row edits it; toggle back to **month** — grid unchanged.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/lib/calendar.js frontend/src/lib/calendar.test.js frontend/src/routes/Calendar.svelte
git commit -m "feat(calendar): day agenda view with Month|Day toggle"
```

---

## Final verification

- [ ] Backend: `cargo test` → all green.
- [ ] Frontend: from `frontend/`, `npm test` → green; `npm run build` → clean.
- [ ] Manual smoke: create a timed event (time sticks, not 00:00); attach 2 people (persist + show); create a weekly series with people; day view lists a day's agenda with `@names`.

## Self-review notes (traceability to spec)

- Spec §1 junction data model + migrate/drop → Task 2 (migration + models).
- Spec §2 write paths maintain junction (create/series/update/scoped) → Task 2 (handlers + `set_event_contacts`).
- Spec §1 reads aggregate `contact_ids` → Task 2 (`AGG` query in list/get).
- Spec §3 time-fix (date + start/end time) → Task 1.
- Spec §4 day agenda view (toggle, nav, sort) → Task 4 (`dayRange`/`dayAgenda` + template).
- Spec §5 multi-select contacts UI + `@name` display → Task 3.
- Spec §6 tests → Rust in Task 2; JS (`localDateTimeToUtc`, `dayAgenda`) in Tasks 1 & 4; manual for Svelte UI.
- Spec "existing 184 midnight events" is explicitly out of scope — no task, correct.

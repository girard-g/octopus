# Favorite Links (pinned on dashboard) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `favorite` flag to links, toggled with a star on the Links page and surfaced as a "pinned" panel on the dashboard.

**Architecture:** One additive column (`link.favorite`) with a partial index. Toggling reuses the existing `PUT /api/links/{id}` — the client resends the full link object with `favorite` flipped; no new endpoint. The dashboard aggregate gains a `favorite_links` field. Frontend adds a star button per link row and a conditional pinned panel on the dashboard.

**Tech Stack:** Rust / Axum 0.8 / SQLx (Postgres), Svelte 5 SPA (svelte-spa-router), Vitest, `#[sqlx::test]` integration tests.

## Global Constraints

- Single-user app: every handler takes `_: AuthUser` as its first extractor.
- Backend errors go through `AppError` (`BadRequest(String)` → 400, `NotFound` → 404).
- No new Rust or npm dependencies.
- SQL is lowercase (matches existing migrations/handlers).
- No new endpoint — favorite is toggled via the existing `PUT /api/links/{id}`.
- `favorite` defaults to `false`; column order in `Link` is after `tags`, before `created_at` (must match `select *`).
- Favicons render client-side via `faviconUrl` from `frontend/src/lib/links.js` with a `▸` glyph fallback; never fetched/stored server-side.
- Terminal aesthetic: reuse existing Tailwind classes/tokens exactly as surrounding code does.
- Dashboard pinned panel is omitted entirely when there are no favorites.

---

### Task 1: Backend — favorite column, model + handler wiring, dashboard aggregate, tests

**Files:**
- Create: `migrations/0011_link_favorite.sql`
- Modify: `src/models.rs` (`Link`, `LinkInput`)
- Modify: `src/routes/links.rs` (`normalize`, `create`, `update`)
- Modify: `src/routes/dashboard.rs` (`Dashboard` struct + query)
- Test: `tests/links.rs` (favorite round-trip), `tests/dashboard.rs` (favorite_links filter)

**Interfaces:**
- Consumes: existing `Link`/`LinkInput`, `normalize`, the `PUT /api/links/{id}` route, `Dashboard` aggregate.
- Produces (relied on by Task 2):
  - `Link` JSON now includes `favorite: bool`.
  - `POST`/`PUT /api/links` accept optional `favorite` (bool; absent → false).
  - `GET /api/dashboard` response includes `favorite_links: Link[]` (only favorited links, ordered by title).

- [ ] **Step 1: Write the migration**

Create `migrations/0011_link_favorite.sql`:

```sql
alter table link add column favorite boolean not null default false;
create index link_favorite_idx on link(favorite) where favorite;
```

- [ ] **Step 2: Add the model fields**

In `src/models.rs`, the `Link` struct — add `favorite` after `tags`, before `created_at`:

```rust
pub struct Link {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub created_at: DateTime<Utc>,
}
```

And `LinkInput` — add `favorite`:

```rust
pub struct LinkInput {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub favorite: Option<bool>,
}
```

- [ ] **Step 3: Write the failing backend tests**

Append to `tests/links.rs`:

```rust
#[sqlx::test]
async fn link_favorite_defaults_false_and_toggles(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    // create without favorite → false
    let (_, l) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "https://a.com"})).with_cookie(&cookie),
    ).await;
    assert_eq!(l["favorite"], false);
    let id = l["id"].as_str().unwrap().to_string();
    let tags: Vec<String> = vec![];

    // PUT favorite=true (full-object replace, like the frontend toggle)
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/links/{id}"), json!({
            "url": "https://a.com", "title": "a.com", "tags": tags, "favorite": true
        })).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["favorite"], true);

    // PUT favorite=false clears it
    let (_, upd2) = send(
        &app,
        json_req("PUT", &format!("/api/links/{id}"), json!({
            "url": "https://a.com", "title": "a.com", "favorite": false
        })).with_cookie(&cookie),
    ).await;
    assert_eq!(upd2["favorite"], false);
}

#[sqlx::test]
async fn link_create_with_favorite_true(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, l) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "https://b.com", "favorite": true})).with_cookie(&cookie),
    ).await;
    assert_eq!(l["favorite"], true);
}
```

Append to `tests/dashboard.rs`:

```rust
#[sqlx::test]
async fn dashboard_favorite_links_only(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    send(&app, json_req("POST", "/api/links", json!({"url": "https://fav.com", "title": "Fav", "favorite": true})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/links", json!({"url": "https://plain.com", "title": "Plain"})).with_cookie(&cookie)).await;

    let (status, d) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let favs = d["favorite_links"].as_array().unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0]["url"], "https://fav.com");
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test --test links --test dashboard`
Expected: FAIL — `favorite` field missing on `Link`/`LinkInput` (won't compile) and `favorite_links` absent from the dashboard response.

- [ ] **Step 5: Wire `favorite` through the links handlers**

In `src/routes/links.rs`, change `normalize` to return the flag. Update its signature and add the computation + return value:

```rust
/// Validate + normalize input into the columns to store.
/// Returns (url, title, description, category, tags, favorite).
fn normalize(input: &LinkInput) -> Result<(String, String, Option<String>, Option<String>, Vec<String>, bool), AppError> {
    let url = input.url.trim().to_string();
    if url.is_empty() {
        return Err(AppError::BadRequest("url is required".into()));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::BadRequest("url must start with http:// or https://".into()));
    }
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| host_of(&url).to_string());
    let clean = |o: &Option<String>| {
        o.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
    };
    let description = clean(&input.description);
    let category = clean(&input.category);
    let mut seen = HashSet::new();
    let tags: Vec<String> = input
        .tags
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .collect();
    let favorite = input.favorite.unwrap_or(false);
    Ok((url, title, description, category, tags, favorite))
}
```

Update `create` — destructure the new field, add the column + bind:

```rust
pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<LinkInput>,
) -> Result<(StatusCode, Json<Link>), AppError> {
    let (url, title, description, category, tags, favorite) = normalize(&input)?;
    let row = sqlx::query_as::<_, Link>(
        "insert into link (url, title, description, category, tags, favorite) \
         values ($1,$2,$3,$4,$5,$6) returning *",
    )
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&category)
    .bind(&tags)
    .bind(favorite)
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}
```

Update `update` — destructure, add `favorite=$7` + bind:

```rust
pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<LinkInput>,
) -> Result<Json<Link>, AppError> {
    let (url, title, description, category, tags, favorite) = normalize(&input)?;
    let row = sqlx::query_as::<_, Link>(
        "update link set url=$2, title=$3, description=$4, category=$5, tags=$6, favorite=$7 \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&category)
    .bind(&tags)
    .bind(favorite)
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}
```

- [ ] **Step 6: Add `favorite_links` to the dashboard aggregate**

In `src/routes/dashboard.rs`, import `Link` (extend the models use line):

```rust
use crate::models::{Event, Link, Project, Task};
```

Add the field to the `Dashboard` struct (after `upcoming_events`):

```rust
#[derive(Serialize)]
pub struct Dashboard {
    pub active_projects: Vec<Project>,
    pub due_tasks: Vec<Task>,
    pub counts: Counts,
    pub upcoming_events: Vec<Event>,
    pub favorite_links: Vec<Link>,
}
```

In `get`, add the query before the final `Ok(Json(...))` (after `upcoming_events`):

```rust
    let favorite_links = sqlx::query_as::<_, Link>(
        "select * from link where favorite order by title",
    )
    .fetch_all(&s.pool)
    .await?;
```

And include it in the returned struct:

```rust
    Ok(Json(Dashboard {
        active_projects,
        due_tasks,
        counts: Counts { projects, active, open_tasks },
        upcoming_events,
        favorite_links,
    }))
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test --test links --test dashboard`
Expected: PASS (new tests + existing ones — existing link/dashboard tests keep passing; the extra `favorite` field in responses doesn't break their assertions).

- [ ] **Step 8: Full check**

Run: `cargo check && cargo test`
Expected: clean build, all tests pass.

- [ ] **Step 9: Commit**

```bash
git add migrations/0011_link_favorite.sql src/models.rs src/routes/links.rs src/routes/dashboard.rs tests/links.rs tests/dashboard.rs
git commit -m "feat(links): favorite flag + dashboard favorite_links aggregate"
```

---

### Task 2: Frontend — star toggle on Links page + pinned panel on dashboard

**Files:**
- Modify: `frontend/src/routes/Links.svelte` (star button + `toggleFavorite`)
- Modify: `frontend/src/routes/Dashboard.svelte` (pinned panel + `unpin` + consume `favorite_links`)

**Interfaces:**
- Consumes: `Link` JSON with `favorite: bool` (Task 1); `PUT /api/links/{id}` accepting `favorite`; `GET /api/dashboard` returning `favorite_links` (Task 1); `faviconUrl` from `../lib/links.js`.

- [ ] **Step 1: Add the favorite toggle to Links.svelte**

In `frontend/src/routes/Links.svelte`, add a `toggleFavorite` function right after the existing `deleteLink` function (around line 60):

```js
  async function toggleFavorite(l) {
    try {
      await api.put('/api/links/' + l.id, { ...l, favorite: !l.favorite })
      await load()
    } catch (e) { error = e.message }
  }
```

In the row controls cluster (the `<div class="flex shrink-0 items-center gap-1">` block, currently holding edit + ×), add a star button as the FIRST child, before the edit button:

```svelte
                <button
                  onclick={() => toggleFavorite(l)}
                  aria-label={l.favorite ? 'Unfavorite link' : 'Favorite link'}
                  class="grid h-10 w-10 place-items-center font-mono text-[15px] leading-none transition md:h-auto md:w-auto md:px-1 {l.favorite ? 'text-accent glow-text' : 'text-faint hover:text-accent'}"
                >{l.favorite ? '★' : '☆'}</button>
```

- [ ] **Step 2: Verify the Links page builds**

Run: `cd frontend && npm run build`
Expected: build succeeds, Links.svelte compiles.

- [ ] **Step 3: Add the pinned panel to Dashboard.svelte**

In `frontend/src/routes/Dashboard.svelte`:

Extend the imports (line 3 area) to add `faviconUrl`:

```js
  import { faviconUrl } from '../lib/links.js'
```

Add state (near the other `$state` declarations, e.g. after `upcomingEvents`):

```js
  let favoriteLinks = $state([])
```

In `load()`, after `upcomingEvents = dash.upcoming_events ?? []`, add:

```js
      favoriteLinks = dash.favorite_links ?? []
```

Add an `unpin` function (after `toggleDone`):

```js
  async function unpin(l) {
    try {
      await api.put('/api/links/' + l.id, { ...l, favorite: false })
      await load()
    } catch (e) { error = e.message }
  }
```

In the template, insert the pinned panel BETWEEN the stat-tiles grid (the `<div class="rise mb-5 grid grid-cols-3 gap-3">…</div>` block) and the two-column `<div class="rise grid grid-cols-1 gap-4 md:grid-cols-2" …>` block:

```svelte
{#if favoriteLinks.length}
  <section class="rise mb-5 rounded-sm border border-border bg-surface" style="animation-delay:40ms">
    <div class="flex items-center justify-between border-b border-border px-4 py-2.5">
      <h2 class="font-mono text-[12px] font-medium text-muted"><span class="text-accent glow-text">&gt;</span> pinned</h2>
      <span class="font-mono text-[12px] tabular-nums text-faint">[{favoriteLinks.length}]</span>
    </div>
    <ul>
      {#each favoriteLinks as l (l.id)}
        <li class="flex items-center gap-3 border-b border-border px-4 py-2.5 last:border-0">
          <img
            src={faviconUrl(l.url)}
            alt=""
            width="16"
            height="16"
            class="h-4 w-4 shrink-0 rounded-[2px]"
            onerror={(e) => { e.currentTarget.replaceWith(document.createTextNode('▸')) }}
          />
          <a
            href={l.url}
            target="_blank"
            rel="noopener noreferrer"
            class="truncate font-mono text-[13px] text-ink hover:text-accent"
          >{l.title}</a>
          <button
            onclick={() => unpin(l)}
            aria-label="Unpin {l.title}"
            class="ml-auto shrink-0 font-mono text-[14px] leading-none text-accent glow-text transition hover:text-st-lost"
          >★</button>
        </li>
      {/each}
    </ul>
  </section>
{/if}
```

- [ ] **Step 4: Verify the full frontend suite + build**

Run: `cd frontend && npx vitest run && npm run build`
Expected: all Vitest tests pass; Vite build succeeds (both files compile).

- [ ] **Step 5: Manual smoke check (optional — skip if no browser harness)**

If the app can be run: star a link on `/links` (star fills, `★`); go to the dashboard — it appears under "pinned" with its favicon; click the pinned star to unpin; it disappears from the panel and the panel hides when the last favorite is removed. If no harness is available, skip and note it in the report; the build is the required gate.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/routes/Links.svelte frontend/src/routes/Dashboard.svelte
git commit -m "feat(links): star toggle on Links page + pinned panel on dashboard"
```

---

## Self-Review Notes

- **Spec coverage:** `favorite` column + partial index (Task 1 Step 1) · `Link`/`LinkInput` fields (Step 2) · normalize/create/update wiring, no new endpoint (Step 5) · dashboard `favorite_links` ordered by title (Step 6) · star toggle reusing PUT (Task 2 Step 1) · pinned panel with unpin + favicon fallback, hidden when empty (Task 2 Step 3) · backend tests both layers (Step 3). All covered.
- **Out of scope confirmed absent:** no PATCH endpoint, no favorites sort/filter on Links page, no cap, no manual ordering.
- **Type consistency:** `favorite: bool` on `Link` (Task 1) ↔ `l.favorite` reads (Task 2). `favorite_links` field name identical in dashboard struct (Task 1) and `dash.favorite_links` consumer (Task 2). `normalize` 6-tuple return matched in both `create` and `update` destructures. Toggle sends `{ ...l, favorite: … }`; the server's `LinkInput` ignores the extra `id`/`created_at` keys (serde ignores unknown fields by default) and reads `favorite`.
```

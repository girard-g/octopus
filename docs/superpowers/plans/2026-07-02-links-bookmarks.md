# Links (bookmarks) Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a standalone `/links` bookmark page with per-link category + free-form tags, plus favicons.

**Architecture:** New `link` table (Postgres, `tags text[]`), an Axum CRUD route copying the `notes.rs` shape, and a Svelte page (`Links.svelte`) that groups links by category with search + tag-chip filtering. Favicons render client-side from DuckDuckGo's icon service — no backend/network work server-side. Pure view logic lives in a testable `links.js` helper module.

**Tech Stack:** Rust / Axum 0.8 / SQLx (Postgres), Svelte SPA (svelte-spa-router), Vitest, `#[sqlx::test]` integration tests.

## Global Constraints

- Single-user app: every handler takes `_: AuthUser` as its first extractor.
- Backend errors go through `AppError` (`BadRequest(String)` → 400, `NotFound` → 404).
- No new Rust or npm dependencies (host parsing is hand-rolled; favicons are a plain `<img>`).
- Terminal aesthetic: reuse existing Tailwind classes/tokens (`accent`, `border`, `surface`, `st-lost`, `font-mono`, etc.) exactly as the other routes do.
- Frontend never fetches favicons server-side or stores them.

---

### Task 1: Backend — migration, model, CRUD route, wiring, tests

**Files:**
- Create: `migrations/0010_links.sql`
- Modify: `src/models.rs` (append `Link` + `LinkInput`)
- Create: `src/routes/links.rs`
- Modify: `src/routes/mod.rs` (add `pub mod links;`)
- Modify: `src/app.rs` (register two routes)
- Test: `tests/links.rs`

**Interfaces:**
- Consumes: `crate::app::AppState`, `crate::auth::AuthUser`, `crate::error::AppError`.
- Produces (relied on by the frontend tasks):
  - `GET /api/links` — optional `?category=<str>` and `?tag=<str>` filters → `200` `[Link]`
  - `POST /api/links` → `201` `Link`
  - `PUT /api/links/{id}` → `200` `Link` / `404`
  - `DELETE /api/links/{id}` → `204` / `404`
  - `Link` JSON shape: `{ id, url, title, description|null, category|null, tags: string[], created_at }`

- [ ] **Step 1: Write the migration**

Create `migrations/0010_links.sql`:

```sql
create table link (
    id          uuid primary key default gen_random_uuid(),
    url         text not null,
    title       text not null,
    description text,
    category    text,
    tags        text[] not null default '{}',
    created_at  timestamptz not null default now()
);
create index link_category_idx on link(category);
create index link_tags_idx on link using gin(tags);
```

- [ ] **Step 2: Add the models**

Append to `src/models.rs`:

```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Link {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LinkInput {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}
```

- [ ] **Step 3: Write the failing integration test**

Create `tests/links.rs`:

```rust
mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn link_crud(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    // create
    let (status, l) = send(
        &app,
        json_req("POST", "/api/links", json!({
            "url": "https://rust-lang.org/learn",
            "title": "Rust",
            "category": "Rust",
            "tags": ["reference", "free", "reference"]
        })).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(l["title"], "Rust");
    assert_eq!(l["category"], "Rust");
    // dedupe preserves first occurrence
    assert_eq!(l["tags"], json!(["reference", "free"]));
    let id = l["id"].as_str().unwrap().to_string();

    // title defaults to host when blank
    let (_, l2) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "https://example.com/x"})).with_cookie(&cookie),
    ).await;
    assert_eq!(l2["title"], "example.com");

    // filter by category
    let (status, list) = send(
        &app,
        json_req("GET", "/api/links?category=Rust", json!(null)).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // filter by tag
    let (_, list) = send(
        &app,
        json_req("GET", "/api/links?tag=free", json!(null)).with_cookie(&cookie),
    ).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // update
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/links/{id}"), json!({
            "url": "https://rust-lang.org", "title": "Rust Lang", "tags": []
        })).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["title"], "Rust Lang");
    assert_eq!(upd["tags"], json!([]));

    // delete
    let (status, _) = send(&app, json_req("DELETE", &format!("/api/links/{id}"), json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 404 after delete
    let (status, _) = send(
        &app,
        json_req("PUT", &format!("/api/links/{id}"), json!({"url": "https://x.com"})).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn link_rejects_empty_url(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "   "})).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn link_rejects_non_http_url(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/links", json!({"url": "ftp://nope.com"})).with_cookie(&cookie),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test --test links`
Expected: FAIL — `tests/links.rs` won't compile / route module `links` does not exist.

- [ ] **Step 5: Write the route handler**

Create `src/routes/links.rs`:

```rust
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::collections::HashSet;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Link, LinkInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
    pub tag: Option<String>,
}

/// Host portion of an already-validated http(s) URL, for the default title.
fn host_of(url: &str) -> &str {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/').next().unwrap_or(rest)
}

/// Validate + normalize input into the columns to store.
/// Returns (url, title, description, category, tags).
fn normalize(input: &LinkInput) -> Result<(String, String, Option<String>, Option<String>, Vec<String>), AppError> {
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
    Ok((url, title, description, category, tags))
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Link>>, AppError> {
    let rows = sqlx::query_as::<_, Link>(
        "select * from link \
         where ($1::text is null or category = $1) \
           and ($2::text is null or tags @> array[$2]) \
         order by category nulls last, created_at desc",
    )
    .bind(q.category.as_deref())
    .bind(q.tag.as_deref())
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<LinkInput>,
) -> Result<(StatusCode, Json<Link>), AppError> {
    let (url, title, description, category, tags) = normalize(&input)?;
    let row = sqlx::query_as::<_, Link>(
        "insert into link (url, title, description, category, tags) \
         values ($1,$2,$3,$4,$5) returning *",
    )
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&category)
    .bind(&tags)
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<LinkInput>,
) -> Result<Json<Link>, AppError> {
    let (url, title, description, category, tags) = normalize(&input)?;
    let row = sqlx::query_as::<_, Link>(
        "update link set url=$2, title=$3, description=$4, category=$5, tags=$6 \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&category)
    .bind(&tags)
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn delete(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let res = sqlx::query("delete from link where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 6: Register the module and routes**

In `src/routes/mod.rs`, add the module (keep alphabetical with the others):

```rust
pub mod links;
```

In `src/app.rs`, add two routes next to the notes routes (immediately after the `/api/notes/{id}` line):

```rust
        .route("/api/links", get(links::list).post(links::create))
        .route("/api/links/{id}", axum::routing::put(links::update).delete(links::delete))
```

(`get`, `post` are already imported in `app.rs`; `links` resolves via `crate::routes::links` — match how `notes` is referenced there.)

- [ ] **Step 7: Run the test to verify it passes**

Run: `cargo test --test links`
Expected: PASS (3 tests).

- [ ] **Step 8: Full check**

Run: `cargo check && cargo test`
Expected: clean build, all tests pass.

- [ ] **Step 9: Commit**

```bash
git add migrations/0010_links.sql src/models.rs src/routes/links.rs src/routes/mod.rs src/app.rs tests/links.rs
git commit -m "feat(links): link CRUD API with category + tags"
```

---

### Task 2: Frontend — pure view helpers + unit tests

**Files:**
- Create: `frontend/src/lib/links.js`
- Test: `frontend/src/lib/links.test.js`

**Interfaces:**
- Produces (relied on by Task 3):
  - `linkHost(url) -> string` — hostname, or `''` if unparseable
  - `faviconUrl(url) -> string` — DuckDuckGo icon URL, or `''`
  - `parseTags(str) -> string[]` — comma-split, trimmed, non-empty, de-duped (first wins)
  - `allTags(links) -> string[]` — sorted unique tags across all links
  - `filterLinks(links, query, activeTags) -> Link[]` — search over title/url/description AND every active tag present
  - `groupByCategory(links) -> {category, links}[]` — categories sorted alpha, uncategorized (`category === ''`) group last

- [ ] **Step 1: Write the failing tests**

Create `frontend/src/lib/links.test.js`:

```js
import { describe, it, expect } from 'vitest'
import { linkHost, faviconUrl, parseTags, allTags, filterLinks, groupByCategory } from './links.js'

describe('links helpers', () => {
  it('extracts host, empty on garbage', () => {
    expect(linkHost('https://rust-lang.org/learn')).toBe('rust-lang.org')
    expect(linkHost('not a url')).toBe('')
  })

  it('builds duckduckgo favicon url', () => {
    expect(faviconUrl('https://rust-lang.org/x')).toBe('https://icons.duckduckgo.com/ip3/rust-lang.org.ico')
    expect(faviconUrl('nope')).toBe('')
  })

  it('parses tags: split, trim, drop empty, dedupe first-wins', () => {
    expect(parseTags(' rust,  free , rust ,')).toEqual(['rust', 'free'])
    expect(parseTags('')).toEqual([])
  })

  it('collects sorted unique tags', () => {
    expect(allTags([{ tags: ['b', 'a'] }, { tags: ['a', 'c'] }, { tags: null }])).toEqual(['a', 'b', 'c'])
  })

  it('filters by query and requires all active tags', () => {
    const links = [
      { title: 'Rust', url: 'https://rust-lang.org', description: null, tags: ['lang', 'free'] },
      { title: 'Svelte', url: 'https://svelte.dev', description: 'ui', tags: ['ui'] },
    ]
    expect(filterLinks(links, 'rust', []).map((l) => l.title)).toEqual(['Rust'])
    expect(filterLinks(links, '', ['ui']).map((l) => l.title)).toEqual(['Svelte'])
    expect(filterLinks(links, '', ['lang', 'free']).map((l) => l.title)).toEqual(['Rust'])
    expect(filterLinks(links, '', ['lang', 'missing'])).toEqual([])
  })

  it('groups by category, uncategorized last', () => {
    const groups = groupByCategory([
      { id: 1, category: 'Rust' },
      { id: 2, category: null },
      { id: 3, category: 'Design' },
    ])
    expect(groups.map((g) => g.category)).toEqual(['Design', 'Rust', ''])
    expect(groups[2].links.map((l) => l.id)).toEqual([2])
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd frontend && npx vitest run src/lib/links.test.js`
Expected: FAIL — `./links.js` does not exist.

- [ ] **Step 3: Write the helpers**

Create `frontend/src/lib/links.js`:

```js
export function linkHost(url) {
  try { return new URL(url).hostname } catch { return '' }
}

export function faviconUrl(url) {
  const host = linkHost(url)
  return host ? `https://icons.duckduckgo.com/ip3/${host}.ico` : ''
}

export function parseTags(str) {
  const seen = new Set()
  return String(str)
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t !== '' && !seen.has(t) && seen.add(t))
}

export function allTags(links) {
  const s = new Set()
  for (const l of links) for (const t of l.tags || []) s.add(t)
  return [...s].sort()
}

export function filterLinks(links, query, activeTags) {
  const q = query.trim().toLowerCase()
  return links.filter((l) => {
    const matchesQuery =
      !q ||
      (l.title || '').toLowerCase().includes(q) ||
      (l.url || '').toLowerCase().includes(q) ||
      (l.description || '').toLowerCase().includes(q)
    const tags = l.tags || []
    const matchesTags = activeTags.every((t) => tags.includes(t))
    return matchesQuery && matchesTags
  })
}

export function groupByCategory(links) {
  const groups = new Map()
  for (const l of links) {
    const key = l.category || ''
    if (!groups.has(key)) groups.set(key, [])
    groups.get(key).push(l)
  }
  const named = [...groups.keys()].filter((k) => k).sort()
  const result = named.map((k) => ({ category: k, links: groups.get(k) }))
  if (groups.has('')) result.push({ category: '', links: groups.get('') })
  return result
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd frontend && npx vitest run src/lib/links.test.js`
Expected: PASS (6 assertions across the suite).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/links.js frontend/src/lib/links.test.js
git commit -m "feat(links): pure view helpers (host, favicon, tags, filter, group)"
```

---

### Task 3: Frontend — Links page, nav entry, route registration

**Files:**
- Create: `frontend/src/routes/Links.svelte`
- Modify: `frontend/src/lib/nav.js` (append nav item)
- Modify: `frontend/src/App.svelte` (import + register route + title)

**Interfaces:**
- Consumes: `../lib/api.js` (`api`), `../lib/components/Modal.svelte`, and all six helpers from `../lib/links.js` (Task 2).
- Consumes: `GET/POST/PUT/DELETE /api/links` (Task 1).

- [ ] **Step 1: Add the nav item**

In `frontend/src/lib/nav.js`, append to `NAV_ITEMS` (after the Notes entry):

```js
  { href: '/links', n: '06', label: 'Links', short: 'link' },
```

- [ ] **Step 2: Register the route in App.svelte**

In `frontend/src/App.svelte`, add the import alongside the other route imports:

```js
  import Links from './routes/Links.svelte'
```

Add to the `routes` object (after `'/notes': Notes,`):

```js
    '/links': Links,
```

Add to the `TITLES` object (extend the `/notes` line):

```js
    '/calendar': 'Calendar', '/notes': 'Notes', '/links': 'Links',
```

- [ ] **Step 3: Write the Links page**

Create `frontend/src/routes/Links.svelte`:

```svelte
<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'
  import { faviconUrl, linkHost, parseTags, allTags, filterLinks, groupByCategory } from '../lib/links.js'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let links   = $state([])
  let error   = $state('')
  let query   = $state('')
  let active  = $state([]) // active tag filter (AND)

  let showForm = $state(false)
  let editId   = $state(null)
  let fUrl     = $state('')
  let fTitle   = $state('')
  let fDesc    = $state('')
  let fCategory = $state('')
  let fTags    = $state('')
  let fError   = $state('')
  let fBusy    = $state(false)

  const tags = $derived(allTags(links))
  const groups = $derived(groupByCategory(filterLinks(links, query, active)))
  const shownCount = $derived(groups.reduce((n, g) => n + g.links.length, 0))

  async function load() {
    error = ''
    try { links = await api.get('/api/links') }
    catch (e) { error = e.message }
  }

  $effect(() => { load() })

  function toggleTag(t) {
    active = active.includes(t) ? active.filter((x) => x !== t) : [...active, t]
  }

  function openNew() {
    editId = null
    fUrl = ''; fTitle = ''; fDesc = ''; fCategory = ''; fTags = ''
    fError = ''
    showForm = true
  }

  function openEdit(l) {
    editId = l.id
    fUrl = l.url
    fTitle = l.title
    fDesc = l.description ?? ''
    fCategory = l.category ?? ''
    fTags = (l.tags || []).join(', ')
    fError = ''
    showForm = true
  }

  async function deleteLink(id, ev) {
    ev.stopPropagation()
    try { await api.del('/api/links/' + id); await load() }
    catch (e) { error = e.message }
  }

  async function save(e) {
    e.preventDefault()
    fError = ''
    const url = fUrl.trim()
    if (!url) { fError = 'url is required'; return }
    if (!(url.startsWith('http://') || url.startsWith('https://'))) {
      fError = 'url must start with http:// or https://'; return
    }
    fBusy = true
    try {
      const payload = {
        url,
        title: fTitle.trim() || null,
        description: fDesc.trim() || null,
        category: fCategory.trim() || null,
        tags: parseTags(fTags),
      }
      if (editId) await api.put('/api/links/' + editId, payload)
      else await api.post('/api/links', payload)
      showForm = false
      await load()
    } catch (err) { fError = err.message }
    finally { fBusy = false }
  }
</script>

<!-- Header -->
<div class="rise mb-6">
  <div class="flex flex-wrap items-center gap-4">
    <div>
      <h2 class="font-mono text-[15px] font-bold text-ink">
        <span class="text-accent glow-text">&gt;</span> links
      </h2>
      <p class="mt-0.5 font-mono text-[12px] text-faint">// bookmarks, by category and tag</p>
    </div>
    <button
      onclick={openNew}
      class="ml-auto h-8 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
    >+ new link</button>
  </div>

  {#if error}
    <p class="mt-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Search + tag chips -->
<div class="rise mb-5 flex flex-col gap-3" style="animation-delay:40ms">
  <input bind:value={query} placeholder="search title / url / description…" class={FIELD} />
  {#if tags.length}
    <div class="flex flex-wrap items-center gap-2">
      {#each tags as t (t)}
        <button
          onclick={() => toggleTag(t)}
          class="rounded-sm border px-2.5 py-1 font-mono text-[12px] transition {active.includes(t)
            ? 'border-accent text-accent shadow-[0_0_8px_rgba(62,245,196,0.18)]'
            : 'border-border text-faint hover:border-border-2 hover:text-muted'}"
        >#{t}</button>
      {/each}
      <span class="ml-auto font-mono text-[12px] text-faint tabular-nums">[{shownCount}]</span>
    </div>
  {/if}
</div>

<!-- Grouped list -->
<div class="rise flex flex-col gap-6" style="animation-delay:80ms">
  {#if shownCount === 0}
    <p class="font-mono text-[12px] text-faint">no links</p>
  {:else}
    {#each groups as g (g.category)}
      <div>
        <p class="mb-2 font-mono text-[12px] font-bold text-muted">
          {g.category || 'uncategorized'} <span class="text-faint tabular-nums">[{g.links.length}]</span>
        </p>
        <div class="flex flex-col gap-2">
          {#each g.links as l (l.id)}
            <div class="group flex items-start gap-3 rounded-sm border border-border bg-surface p-3 transition-all hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]">
              <!-- Favicon: falls back to a block-cursor glyph on load error. -->
              <img
                src={faviconUrl(l.url)}
                alt=""
                width="16"
                height="16"
                class="mt-0.5 h-4 w-4 shrink-0 rounded-[2px]"
                onerror={(e) => { e.currentTarget.replaceWith(document.createTextNode('▸')) }}
              />
              <div class="min-w-0 flex-1">
                <a
                  href={l.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="block truncate font-mono text-[13px] text-ink hover:text-accent"
                >{l.title}</a>
                <p class="truncate font-mono text-[11px] text-faint">{linkHost(l.url)}</p>
                {#if l.description}
                  <p class="mt-1 font-mono text-[12px] text-muted">{l.description}</p>
                {/if}
                {#if l.tags?.length}
                  <div class="mt-1.5 flex flex-wrap gap-1.5">
                    {#each l.tags as t (t)}
                      <span class="rounded-sm border border-border-2 px-1.5 py-0.5 font-mono text-[10px] text-faint">#{t}</span>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="flex shrink-0 items-center gap-1">
                <button
                  onclick={() => openEdit(l)}
                  aria-label="Edit link"
                  class="grid h-10 w-10 place-items-center font-mono text-[13px] text-faint transition hover:text-accent md:h-auto md:w-auto md:px-1"
                >edit</button>
                <button
                  onclick={(e) => deleteLink(l.id, e)}
                  aria-label="Delete link"
                  class="grid h-10 w-10 place-items-center font-mono text-[16px] leading-none text-faint transition hover:text-st-lost"
                >×</button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</div>

<!-- New / edit modal -->
{#if showForm}
  <Modal title={editId ? 'Edit link' : 'New link'} onclose={() => (showForm = false)}>
    <form onsubmit={save} class="flex flex-col gap-3">
      {#if fError}
        <p class="rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {fError}</p>
      {/if}
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">url</p>
        <input bind:value={fUrl} placeholder="https://…" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">title <span class="text-faint">(defaults to host)</span></p>
        <input bind:value={fTitle} placeholder="optional" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">description</p>
        <input bind:value={fDesc} placeholder="optional" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">category</p>
        <input bind:value={fCategory} placeholder="optional (e.g. Rust)" class={FIELD} />
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">tags <span class="text-faint">(comma-separated)</span></p>
        <input bind:value={fTags} placeholder="reference, free" class={FIELD} />
      </div>
      <button
        type="submit"
        disabled={fBusy}
        class="h-9 rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50"
      >{editId ? 'save changes' : 'save link'}</button>
    </form>
  </Modal>
{/if}
```

- [ ] **Step 4: Run the frontend test suite + build**

Run: `cd frontend && npx vitest run && npm run build`
Expected: all Vitest tests pass; Vite build succeeds (Links.svelte compiles).

- [ ] **Step 5: Manual smoke check**

Run the app (existing dev flow). Verify: `/links` appears in sidebar (06) and bottom tabs (link); add a link with a category + two tags; it shows under its category with a favicon; search and tag chips filter; edit updates; delete removes; blank title falls back to the host.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/routes/Links.svelte frontend/src/lib/nav.js frontend/src/App.svelte
git commit -m "feat(links): bookmarks page — grouped by category, tag filter, favicons"
```

---

## Self-Review Notes

- **Spec coverage:** standalone global (Task 1 no FK) · category + tags text[] (Task 1 migration/model) · list filters (Task 1 `list`) · url/title/tag validation + title-defaults-to-host (Task 1 `normalize`, tested) · nav 06 (Task 3) · grouped-by-category + search + tag chips (Tasks 2/3) · DuckDuckGo favicon + `▸` fallback (Tasks 2/3) · shared Modal (Task 3) · tests both layers. All covered.
- **Out of scope confirmed absent:** no project/contact columns, no server-side scraping, no import/export/nested folders/reorder.
- **Type consistency:** `Link` JSON fields (Task 1) match frontend access (`l.url/title/description/category/tags/id`) in Tasks 2/3. Helper names (`linkHost`, `faviconUrl`, `parseTags`, `allTags`, `filterLinks`, `groupByCategory`) identical across Task 2 defs and Task 3 imports.
```

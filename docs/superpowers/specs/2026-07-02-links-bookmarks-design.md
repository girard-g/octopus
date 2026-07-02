# Links (bookmarks) page — design

**Date:** 2026-07-02
**Status:** Approved (design), ready for implementation planning

## Summary

A standalone, global bookmark library as the 6th nav section (`/links`). Store
internet links with a title, optional description, one **category** (folder-like
bucket) and free-form **tags**. Browse grouped by category, filter by search /
tag, add-edit-delete via the shared modal. Mirrors the existing Notes CRUD
pattern end to end.

Single-user app (`AuthUser`), Axum 0.8 + SQLx (Postgres) backend, Svelte SPA
(`svelte-spa-router`) frontend, terminal aesthetic.

## Decisions

- **Standalone global** — links are not attached to projects or contacts.
- **Category + tags** — one nullable `category` text column plus a `tags text[]`
  column (GIN-indexed). No join tables; Postgres does the tag filtering.
- **Favicons on** — rendered client-side from DuckDuckGo's icon service keyed on
  the link's host. No server-side fetching or storage.

## Data model

Migration `migrations/0010_links.sql`:

```sql
create table link (
    id          uuid primary key default gen_random_uuid(),
    url         text not null,
    title       text not null,
    description text,
    category    text,                          -- one folder-like bucket, nullable
    tags        text[] not null default '{}',  -- free-form labels
    created_at  timestamptz not null default now()
);
create index link_category_idx on link(category);
create index link_tags_idx on link using gin(tags);
```

## Backend — `src/routes/links.rs`

Copy the shape of `src/routes/notes.rs`.

Models in `src/models.rs`:

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
    pub title: Option<String>,       // defaults to URL host when blank
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,   // defaults to []
}
```

Handlers (registered in `src/routes/mod.rs` under `/api/links`):

- `list` — optional `?category=` and `?tag=` query filters. Order by
  `category nulls last, created_at desc`.
- `create`
- `update`
- `delete`

**Validation at the trust boundary** (in `validate_input`, mirroring notes):

- `url` — trimmed, non-empty, must start with `http://` or `https://`.
- `title` — if blank/absent, default to the host parsed from `url`; else
  trimmed non-empty.
- `category` — trim; empty string normalized to `NULL`.
- `tags` — trim each, drop empties, dedupe; default `[]`.

## Frontend — `frontend/src/routes/Links.svelte`

- **Nav** (`frontend/src/lib/nav.js`): append
  `{ href: '/links', n: '06', label: 'Links', short: 'link' }`. Route registered
  in `App.svelte` alongside the others.
- **Layout:**
  - Top bar: search box (filters title/url/description client-side) + tag filter
    chips (click to toggle; active tags AND-filter the list).
  - Links **grouped by category**, uncategorized group last. Each row:
    favicon · title (anchor, `target="_blank" rel="noopener"`) · domain · tag
    chips · edit/delete controls (40px tap target on mobile, matching kanban).
  - Add / edit through the shared `frontend/src/lib/components/Modal.svelte`:
    fields url, title, description, category, tags (comma-separated input:
    split on `,`, trim each, drop empties, dedupe).
- **Favicon:** `https://icons.duckduckgo.com/ip3/<host>.ico` where `<host>` is
  derived from the link URL. `<img>` with `onerror` fallback to a `▸` glyph
  (the app's block-cursor motif). No caching/storage.
- **API** (`frontend/src/lib/api.js`): reuse the generic `api` helper; no new
  transport code needed.

## Error handling

- Backend returns `AppError` (400 on validation failure, 404 on unknown id) —
  same as notes.
- Frontend surfaces API errors inline in the modal (existing pattern).
- Favicon load failure degrades to the glyph; never blocks the row.

## Testing

Follow existing conventions:

- **Backend:** integration tests in `tests/` mirroring the notes tests —
  create → list (incl. `?category=` / `?tag=` filters) → update → delete, plus
  validation rejections (empty/non-http url) and title-defaults-to-host.
- **Frontend:** a small unit test for the pure helpers (host extraction, tag
  parsing, tag AND-filter) alongside the other `*.test.js` files.

## Explicitly out of scope (YAGNI)

- Project / contact attachment.
- Server-side URL scraping (title/description autofill, favicon fetching/storage).
- Import / export, nested folders, drag-to-reorder.

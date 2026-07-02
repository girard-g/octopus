# Favorite links → pinned on dashboard — design

**Date:** 2026-07-02
**Status:** Approved (design), ready for implementation planning
**Extends:** `2026-07-02-links-bookmarks-design.md` (the links feature)

## Summary

Add a `favorite` flag to links. On the Links page each row gets a star toggle.
Favorited links are surfaced as a compact "pinned" panel on the dashboard, with
an unpin control. No new endpoints — toggling reuses the existing
`PUT /api/links/{id}`.

Single-user app (`AuthUser`), Axum 0.8 + SQLx (Postgres) backend, Svelte 5 SPA
frontend, terminal aesthetic.

## Decisions

- **Star toggle only** on the Links page — no favorites-first sort, no Favorites
  filter. Favorites otherwise sit in their normal category group.
- **Toggle reuses `PUT /api/links/{id}`** — the client holds the full link
  object, so it re-sends it with `favorite` flipped. No dedicated favorite/PATCH
  endpoint.
- **Dashboard "pinned" panel** surfaces favorites, with an unpin control that
  flips `favorite` off via the same PUT. Panel hidden when there are no
  favorites.
- **No cap** on the number of pinned links.

## Data model

Migration `migrations/0011_link_favorite.sql`:

```sql
alter table link add column favorite boolean not null default false;
create index link_favorite_idx on link(favorite) where favorite;
```

Partial index (`where favorite`) — only the dashboard's favorited-rows query
benefits, and the set is small.

## Backend

`src/models.rs`:

- `Link` gains `pub favorite: bool` (after `tags`, before `created_at` — column
  order matches `select *`).
- `LinkInput` gains `pub favorite: Option<bool>`.

`src/routes/links.rs`:

- `normalize` returns `favorite: bool` alongside the existing fields, defaulting
  `input.favorite.unwrap_or(false)`.
- `create` and `update` bind `favorite` into their INSERT / UPDATE statements
  (add the column + one bind each). No new handler, no new route.

`src/routes/dashboard.rs`:

- `Dashboard` struct gains `pub favorite_links: Vec<Link>` (import `Link`).
- Populate with `select * from link where favorite order by title`.

## Frontend

`frontend/src/routes/Links.svelte`:

- Each link row gets a star button in the existing controls cluster: `★` (filled,
  `text-accent`) when `l.favorite`, `☆` (outline, `text-faint`) otherwise.
  `aria-label` reflects state ("Unfavorite" / "Favorite").
- Click handler: `api.put('/api/links/' + l.id, { ...l, favorite: !l.favorite })`
  then `await load()`. Errors surface in the existing `error` banner.

`frontend/src/routes/Dashboard.svelte`:

- Consume `favorite_links` from the `/api/dashboard` payload.
- Render a compact **"pinned"** panel (terminal styling matching the existing
  dashboard panels). Each favorite: favicon (`faviconUrl` from `../lib/links.js`,
  `▸` glyph fallback on error) · title as an anchor
  (`target="_blank" rel="noopener noreferrer"`) · a filled-star **unpin** button.
- Unpin handler: `api.put('/api/links/' + l.id, { ...l, favorite: false })` then
  reload the dashboard. `aria-label="Unpin"`.
- Panel is omitted entirely when `favorite_links` is empty.

## Error handling

- Backend: `AppError` as elsewhere (400 validation, 404 unknown id).
- Frontend: PUT failures surface in each page's existing error banner; a failed
  favicon load degrades to the `▸` glyph and never blocks the row.

## Testing

- **Backend** `tests/links.rs`: `favorite` defaults `false` on create; a PUT with
  `favorite: true` sets it and the value round-trips on GET; a subsequent PUT with
  `favorite: false` clears it.
- **Backend** `tests/dashboard.rs`: seed favorited + non-favorited links; assert
  `favorite_links` contains only the favorited ones.
- **Frontend:** the toggle is a one-line `api.put` with no extractable pure logic,
  so no new unit test — covered by the suite building (`npm run build`) and the
  existing Vitest run.

## Out of scope (YAGNI)

- Dedicated favorite / PATCH endpoint.
- Favorites-first sort or a Favorites filter on the Links page.
- A cap on the number of pinned links.
- Reordering / manual pin ordering (dashboard pinned list is alphabetical).

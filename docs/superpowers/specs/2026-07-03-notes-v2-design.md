# Notes v2 — folders, markdown, standalone notes — design

**Date:** 2026-07-03
**Status:** Approved (design), ready for implementation planning
**Replaces:** the flat notes feature from `2026-06-30-octopus-freelance-hub-design.md`

## Summary

Rebuild the notes feature into a standalone knowledge space. Notes are no longer
forced to hang off a contact or project — instead every note lives in a **nested
folder** (or "Unfiled"), has an optional **title**, a markdown **body**, and an
*optional* link to a contact or project. The Notes page becomes a two-pane
console UI: a folder tree with notes inline on the left, a markdown editor on the
right (raw|preview split on desktop, edit/preview toggle on mobile). Adds
full-text search, autosave, note/folder move, and pin.

Single-user app (`AuthUser`), Axum 0.8 + SQLx (Postgres) backend, Svelte 5 SPA
frontend, terminal aesthetic (JetBrains Mono, mint accent `#3ef5c4`, grid +
scanlines).

## Decisions

- **Folder is the home; contact/project is an optional link.** A note belongs to
  one folder (or none → "Unfiled") and may *additionally* reference one contact
  or one project, so it still appears on that hub's timeline.
- **Nested folders** via `parent_id` self-reference. Folders are few, so the
  client fetches all of them and builds the tree in JS — no recursive SQL.
- **Title is optional** (nullable). The Notes page always sets one; the hub
  quick-notes (ContactDetail / ProjectBoard) create body-only notes with a null
  title, which displays as a derived first-line title (falling back to
  "Untitled"). This is what keeps the existing hubs working untouched.
- **Content invariant:** a note requires **at least one of `title` / `body`
  non-empty**. This serves both the title-first Notes page and the body-only hub
  quick-notes.
- **Two-pane layout** (validated via mockup): merged folder+notes sidebar, large
  editor. Not three-pane.
- **Markdown via `marked`** (headings, lists, code fences, tables, links).
  Rendered with `{@html marked(body)}`.
- **Autosave** replaces the save button (debounce ~600ms + on blur).
- **Move is menu-based**, not drag: a folder picker on a note, a "move to"
  picker for re-parenting folders.
- **Pin** sorts a note to the top of its folder.

## Data model

### New migration `0012_notes_v2.sql`

```sql
-- folders (nested)
create table folder (
    id         uuid primary key default gen_random_uuid(),
    name       text not null,
    parent_id  uuid references folder(id) on delete cascade,
    position   int  not null default 0,
    created_at timestamptz not null default now()
);
create index folder_parent_idx on folder(parent_id);

-- extend note
alter table note add column title      text;
alter table note add column folder_id  uuid references folder(id) on delete set null;
alter table note add column pinned      boolean not null default false;
alter table note add column updated_at  timestamptz not null default now();
create index note_folder_idx on note(folder_id);

-- drop the "exactly one parent" rule; links are now optional + independent
alter table note drop constraint note_one_parent;

-- backfill
update note set updated_at = created_at;
-- existing contact_id / project_id values are left untouched.
```

Notes:
- `folder_id ON DELETE SET NULL` → deleting a folder drops its notes to Unfiled
  rather than deleting them.
- `parent_id ON DELETE CASCADE` → deleting a folder deletes its subfolders; every
  note in that whole subtree falls to Unfiled (see Folder delete below).
- The old `note_one_parent` check is removed. "At most one of contact/project"
  becomes a *soft* validation rule (below), not a DB constraint.

## Backend

### `models.rs`
- `Note`: add `title: Option<String>`, `folder_id: Option<Uuid>`,
  `pinned: bool`, `updated_at: DateTime<Utc>`.
- `NoteInput`: add `title: Option<String>`, `folder_id: Option<Uuid>`,
  `pinned: Option<bool>`.
- New `Folder { id, name, parent_id: Option<Uuid>, position, created_at }` and
  `FolderInput { name, parent_id: Option<Uuid>, position: Option<i32> }`.

### `notes.rs`
- **`validate_input`** rewritten: require at least one of `title`/`body`
  non-empty; if both `contact_id` and `project_id` are set → 400
  ("link to a contact or a project, not both"). No parent is required.
- **`list`**: extend `ListQuery` with `folder_id: Option<Uuid>` and
  `q: Option<String>`.
  - `q` present → `where (title ilike '%'||$1||'%' or body ilike '%'||$1||'%')`,
    global across all notes, `order by updated_at desc`.
  - `folder_id` present → filter to that folder, `order by pinned desc,
    updated_at desc`.
  - existing `contact_id` / `project_id` filters unchanged.
- **`create`**: insert `title, body, folder_id, contact_id, project_id, pinned`;
  return row. `updated_at` defaults to now().
- **`update`**: set `title, body, folder_id, contact_id, project_id, pinned,
  updated_at=now()`.
- `delete` unchanged.

### `routes/folders.rs` (new, mirrors `links.rs` shape)
- `GET /api/folders` → all folders (client builds the tree).
- `POST /api/folders` → create (`name` required, optional `parent_id`).
- `PUT /api/folders/{id}` → rename and/or re-parent (`parent_id`). Reject setting
  `parent_id` to self (400). *(Deeper cycle prevention is deferred — see
  Deferred; a self-parent guard covers the common slip.)*
- `DELETE /api/folders/{id}` → cascade subfolders, notes fall to Unfiled.
- Register the router in `app.rs` / `routes/mod.rs` alongside notes.

## Frontend

### `Notes.svelte` — rebuilt, two-pane

**Left sidebar**
- Search input at top. When the query is non-empty, the tree is **replaced** by a
  flat result list — each hit shows the note title + a folder-path breadcrumb;
  clicking opens it. Clearing the query restores the tree.
- Folder tree: expand/collapse rows, note titles inline under their folder,
  pinned notes first (★), then by `updated_at`. "Unfiled" pseudo-folder at the
  bottom with its count.
- Actions: `+ folder` (name prompt, optional parent = current folder),
  `+ note` (creates an in-memory draft in the selected folder — see Autosave).

**Right editor**
- Title input on top.
- Body: **desktop** = raw textarea | live `marked` preview, side by side;
  **mobile** = single pane with `[ edit ] [ preview ]` toggle (reuse the existing
  responsive breakpoint used elsewhere in the app).
- Footer row: folder picker (move), optional `@contact / #project` link picker,
  pin toggle (★), delete (×), and a subtle autosave status ("saved" / "saving…").

**Autosave lifecycle**
- `+ note` opens a blank **in-memory draft** — no row exists yet.
- On the first debounced change where title+body is non-empty, POST the note;
  store the returned `id`. Subsequent edits PATCH (`PUT /api/notes/{id}`),
  debounced ~600ms and flushed on blur / selection change.
- If the user leaves an empty draft (never non-empty), nothing was ever written —
  no empty rows.

**Folder delete**
- Confirm dialog stating the blast radius: "Delete '<name>'? This removes N
  subfolder(s) and moves M note(s) to Unfiled." Counts computed client-side from
  the already-loaded folder/note lists.

### Untouched
- `ContactDetail.svelte` and `ProjectBoard.svelte` quick-notes keep POSTing
  `{ body, contact_id|project_id }` (title null). Their `buildTimeline` /
  `lastTouch` logic reads `body` + `created_at`, both still present. No changes
  required there.

## Dependencies
- Add `marked` to `frontend/package.json`.

## Testing
- Backend: `validate_input` unit test — rejects empty title+body, rejects
  contact+project both set, accepts title-only, accepts body-only.
- Backend: folder delete moves notes to Unfiled (folder_id null) and removes
  subfolders.
- Frontend: existing `Calendar.mobile.test.js` style — a small test that the
  markdown preview renders and that an abandoned empty draft issues no POST.
- Manual: hub quick-notes still create + appear on timelines after the migration.

## Deferred (conscious, not oversights)
- **HTML sanitization.** `{@html marked(body)}` will execute embedded HTML —
  e.g. markdown pasted from an external source containing `<img onerror=…>` runs
  in the authenticated session. Acceptable for v1: single self-hosted user
  authoring their own notes behind `APP_PASSWORD`. If wanted later, DOMPurify is
  a ~2-line wrap, not a rewrite.
- **Deep folder-cycle prevention.** Only self-parenting is blocked. Building a
  longer cycle via repeated re-parents is possible; a full ancestor-walk check is
  deferred until it's an actual problem.
- **Drag-and-drop** reordering of notes/folders. Move is menu-based for v1.
- **Tags, attachments, sharing, note reordering within a folder.**

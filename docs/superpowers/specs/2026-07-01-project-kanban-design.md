# Remove pipeline, add per-project kanban + project list

**Date:** 2026-07-01
**Status:** approved (design)

## Summary

Remove the lead **Pipeline** (a cross-project board whose columns are project
statuses `lead→lost`) and replace its role as the entry point to projects with a
**Projects list** page. Each project's task kanban (`ProjectBoard.svelte`,
columns `todo/doing/done`) already exists and is unchanged; the list is how you
reach and switch between them. Project `status` is simplified from five sales
stages to **`active` / `archived`**.

## Motivation

The pipeline modeled a sales funnel the user doesn't want. The useful artifact —
a per-project task board — already exists but is only reachable by drilling into
the pipeline. Deleting the pipeline and adding a plain list keeps the task boards
and drops the funnel.

## Scope

### Delete (pipeline)

Frontend:
- `frontend/src/routes/Pipeline.svelte`
- `frontend/src/lib/pipeline.js`, `frontend/src/lib/pipeline.test.js`
- In `App.svelte`: the `Pipeline` import, the `/pipeline` route entry, and its
  page-title entry.
- In `Sidebar.svelte`: the Pipeline nav item + its stale comment.

Backend:
- `projects::move_` handler and the `PATCH /api/projects/{id}/move` route
  (`src/app.rs`).
- `ProjectMove` struct (`src/models.rs`).
- `board_order` column (pipeline-only ordering) — dropped in the migration and
  removed from the `Project` struct.

**Do not touch** the `--color-st-lead/proposal/active/done/lost` CSS tokens in
`app.css`; they are used repo-wide as a generic palette (e.g. red errors via
`st-lost`), not only for project status.

### Add: Projects list

- **New `frontend/src/routes/Projects.svelte`** at route `/projects`:
  - Grid of project cards: title, contact name, **task count**.
  - **Active / Archived** filter toggle (default: Active).
  - **New project** button → modal (title + contact select; status defaults to
    `active`). Replaces the pipeline's "New lead" modal.
  - Card click → `/projects/:id` (existing board).
- **`Sidebar.svelte`:** replace the Pipeline nav item with **Projects**
  (`/projects`).
- **`App.svelte`:** add `/projects` → `Projects.svelte` route + page title.

### ProjectBoard.svelte (small fixes)

- Back button label `< pipeline` → `< projects`; its target and the post-delete
  redirect `push('/pipeline')` → `push('/projects')`.
- Add an **Archive / Restore** button that flips `status` between `active` and
  `archived` via the existing `PUT /api/projects/{id}`.

### Status field: `active` / `archived`

- **New migration** (`migrations/0006_project_status_active_archived.sql`):
  - Drop the old `CHECK` on `project.status`; migrate rows
    `lead/proposal/active → active`, `done/lost → archived`; set default
    `'active'`; add new `CHECK (status IN ('active','archived'))`.
  - `ALTER TABLE project DROP COLUMN board_order`.
  - (Order matters: remap values, then re-add the constraint.)
- **`src/models.rs`:** `PROJECT_STATUSES = ["active","archived"]`; remove
  `board_order` from `Project`; remove `ProjectMove`.
- **`src/routes/projects.rs`:**
  - `create`: default status `"active"` (was `"lead"`).
  - `list`: `ORDER BY created_at` (was `board_order`); honor `?status=` filter
    (already present); include a **task count per project** in the response.
  - Remove `move_`.

### Dashboard (status consumer — must update)

- **`src/routes/dashboard.rs`:**
  - Replace the `leads` count (`status='lead'`) with a **total projects** count
    (`select count(*) from project`); rename the field `leads → projects`.
  - Active-projects query: `ORDER BY created_at` (was `board_order`).
- **`frontend/src/routes/Dashboard.svelte`:** relabel the first stat card to
  **Projects** bound to `counts.projects`. Cards: Projects / Active / Open tasks.

### ContactDetail.svelte (status consumer)

- Add `active` / `archived` entries to the project status color map (lines 7-8).
  Old keys become unused; no functional break.

## Data flow

1. Sidebar → **Projects** → `GET /api/projects?status=active` → card grid
   (title, contact, task_count).
2. Click card → `/projects/:id` → existing board (`GET /api/tasks?project_id=`),
   drag task → `PUT /api/tasks/{id}`.
3. Archive on board → `PUT /api/projects/{id}` `{status:'archived'}` →
   disappears from the Active filter, appears under Archived.
4. Back → Projects list → pick another project (the "switch").

## Error handling

Unchanged from existing patterns: API errors surface via the standard
`[ ERR ]` banner already used across routes. New project modal validates
non-empty title + a selected contact before POST (mirrors the old "New lead"
modal and `Contacts.svelte`).

## Testing

- Delete `pipeline.test.js`. `tasks.test.js`, `calendar.test.js` unchanged.
- **`tests/projects.rs`:** rename/adjust `project_create_defaults_to_lead` →
  asserts default `active`; delete the `/move` endpoint test; change the two
  `status:"lead"` creates to `active`/`archived`. Add: `list?status=archived`
  returns only archived projects; list response includes `task_count`.
- **`tests/dashboard.rs`:** replace the `status:"lead"` + `counts.leads==1`
  assertions with `counts.projects` (total) assertions.
- Migration sanity: after migrate, a pre-existing `lead` row reads back as
  `active`, a `done` row as `archived`.

## Out of scope / YAGNI

- No in-board project-switcher dropdown (user chose list-only switching).
- No task ordering/position field (tasks stay ordered by `due_on`, `created_at`).
- Tasks with `project_id = NULL` remain unsurfaced — same as today under the
  pipeline; not a regression.

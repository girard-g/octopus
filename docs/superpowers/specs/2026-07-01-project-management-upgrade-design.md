# Project management upgrade — design

**Date:** 2026-07-01
**Status:** approved, ready for implementation plan

## Goal

Upgrade the project feature from a thin per-project kanban into a proper
project-management tool for a solo freelancer. Three gaps drive the work:

1. **Planning/time** — surface task due dates (overdue-flagged, sorted). No
   project deadlines or milestones.
2. **Richer tasks** — priority, subtasks/checklist, description, manual
   ordering, and a t-shirt size estimate.
3. **Overview/health** — a cross-project daily task view on the Dashboard, and
   per-project progress stats on the Projects list.

Explicitly out of scope: project-level deadlines/milestones, a separate subtask
table, client-facing output, invoicing changes.

## Current state (baseline)

- `task`: `id, project_id, title, status (todo/doing/done), due_on, created_at`.
- `project`: `id, contact_id (opt), title, status (active/archived),
  description, invoice_url, created_at`; list query already returns
  `task_count` via a `#[sqlx(default)]` field.
- `ProjectBoard.svelte`: drag-drop kanban (`svelte-dnd-action`). Tasks have
  **no edit/detail UI** — only drag-to-move and delete. The drag `finalize`
  handler sends a *minimal* `PUT` payload (`title/status/project_id/due_on`).
- `Dashboard` route already returns `due_tasks` (`status <> 'done'`, ordered by
  `due_on nulls last`), but the frontend does not split overdue vs upcoming or
  show which project a task belongs to.
- `PUT /api/tasks/:id` is a **full replace** (`update task set project_id=$2,
  title=$3, status=$4, due_on=$5`).

## Data model changes

### Migration `0008` — task fields

Add to `task`:

| field | type | notes |
|---|---|---|
| `priority` | `text` null, check `in ('low','medium','high')` | **nullable, no default** — a flag shows only on tasks explicitly marked |
| `size` | `text` null, check `in ('xs','s','m','l','xl')` | t-shirt estimate |
| `description` | `text` null | free-text body |
| `checklist` | `jsonb not null default '[]'` | array of `{ "title": string, "done": bool }` |
| `position` | `int not null default 0` | manual order within a `(project_id, status)` column |

No separate `subtask` table — checklist items live in the `jsonb` column.

## Backend changes

### Models (`src/models.rs`)

- `Task` gains: `priority: Option<String>`, `size: Option<String>`,
  `description: Option<String>`, `checklist: Vec<ChecklistItem>` (mapped with
  `#[sqlx(json)]` so `FromRow` compiles), `position: i32`.
- Add `project_title: Option<String>` to `Task` as `#[sqlx(default)]`,
  populated **only** by the dashboard query (mirrors the `task_count` pattern).
- New `ChecklistItem { title: String, done: bool }` (Serialize + Deserialize).
- `TaskInput` gains the same editable fields: `priority`, `size`,
  `description`, `checklist` (`#[serde(default)]`), `position` (`#[serde(default)]`).
- New consts: `PRIORITY_LEVELS = ["low","medium","high"]`,
  `TASK_SIZES = ["xs","s","m","l","xl"]`.
- `Project` gains `#[sqlx(default)]` health fields: `done_count: i64`,
  `open_count: i64`, `overdue_count: i64`, `next_due: Option<NaiveDate>`,
  populated by the list query.

### Tasks routes (`src/routes/tasks.rs`)

- `create`: accept and insert the new fields (checklist defaults to `[]`,
  position defaults to `0`). Validate `priority`/`size` against the consts when
  present (reuse the `check_*` helper pattern).
- `update` (full replace): extend the `set` clause to write **all** editable
  fields including `priority`, `size`, `description`, `checklist`, `position`.
  Because it stays a full replace, fields remain clearable (description → null).
- `list`: `order by position, created_at` (per column ordering is applied
  client-side within each status group, but stable ordering starts here).

### Dashboard route (`src/routes/dashboard.rs`)

- Change the `due_tasks` query to join `project` and populate `project_title`:
  `select t.*, p.title as project_title from task t left join project p on
  p.id = t.project_id where t.status <> 'done' order by due_on nulls last,
  created_at limit 20`. (Overdue vs upcoming split is done client-side by
  comparing `due_on` to today.)

### Projects route (`src/routes/projects.rs`)

- Extend the list aggregation to compute health per project:
  `done_count` (`count filter status='done'`), `open_count`
  (`count filter status<>'done'`), `overdue_count`
  (`count filter status<>'done' and due_on < current_date`), and `next_due`
  (`min(due_on) filter status<>'done'`). Keep existing `task_count`.

## Frontend changes

### Task detail modal (`ProjectBoard.svelte`)

- Click a task card → open a modal bound to the in-memory task (no new
  single-task GET; the board already loads all tasks via
  `/api/tasks?project_id=`).
- Modal edits: title, description, priority (none/low/med/high), size
  (none/xs…xl), due date (`<input type="date">`), and checklist (add item,
  toggle done, remove item; order = list order).
- Save = one full-object `PUT`. Toggling a checklist item is the same
  full-object `PUT` with the mutated `checklist`.
- Card display gains: a priority dot/flag (only when set), a size chip (only
  when set), and `done/total` checklist progress (only when non-empty), in
  addition to the existing title + due date.

### Drag handler — the wipe trap (must fix)

The `finalize` handler currently sends a minimal payload. With the new fields
this would wipe them on every drag. **Fix: send the full task object** —
spread `...t` and override only `status` and `position`. Applies to the drag
`finalize` PUTs and any other task PUT caller. This keeps the full-replace
semantics (fields stay clearable) while surviving drags.

`position` on finalize: reindex the affected column(s) and PUT each task whose
`status` or `position` changed (the handler already loops over moves). No
dedicated reorder endpoint; no fractional positions.

### Projects list (`Projects.svelte`)

- Each project row shows: `% done` (done/total), open count, overdue count
  (highlighted when > 0), and next due date.

### Dashboard (`Dashboard.svelte`)

- Upgrade the existing due-tasks list into the daily view: **overdue first
  (red), then upcoming**, each row tagged with its `project_title`.

## Testing

- **Regression test (the trap):** create a task with `priority` + `checklist`
  set, then issue the drag-style full-object `PUT` (status change), and assert
  `priority` and `checklist` survive. This one test guards the wipe trap.
- Backend: task create/update accepts and round-trips the new fields; invalid
  `priority`/`size` rejected; project list returns health counts; dashboard
  due_tasks carry `project_title`.
- Frontend: checklist add/toggle/remove updates the payload; card shows
  progress; dashboard splits overdue vs upcoming.

## Implementation gotchas

- `checklist` needs `#[sqlx(json)]` (or `sqlx::types::Json<Vec<ChecklistItem>>`)
  on the `Task` struct or `FromRow` won't compile.
- Add check-constraints for `priority`/`size` mirroring the existing
  `status` constraint style.
- Click-vs-drag coexistence on the dndzone card: `svelte-dnd-action` fires a
  normal click when there's no drag — verify the card open still feels right.
- `time` crate stays pinned `=0.3.41` (existing constraint); no new deps.

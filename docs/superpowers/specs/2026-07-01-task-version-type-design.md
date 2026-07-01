# Task version + type — design

**Date:** 2026-07-01
**Status:** Approved

## Goal

Give tasks two new optional attributes:

- **version** — free-text milestone/release grouping (e.g. `v1.0`, `MVP`). Lets you
  eyeball and filter which release a task belongs to.
- **type** — fixed enum label describing the kind of work: `feature`, `bug`,
  `enhancement`, `chore`, `docs`.

Both mirror the existing `priority` / `size` task fields exactly (nullable column,
enum where constrained, colored badge on the board card, editor in the task detail
modal). No new tables, no new views, no grouping layout.

## Data model

New migration `migrations/0009_task_version_type.sql`:

```sql
alter table task add column version text;
alter table task add column type    text check (type in ('feature','bug','enhancement','chore','docs'));
```

- `version`: free text, nullable. No enum constraint (typos are acceptable for a solo hub;
  promote to a managed set later only if they become a real problem).
- `type`: nullable, check-constrained to the five values.

## Backend (`src/`)

- `models.rs`:
  - Add to `Task`: `pub version: Option<String>` and `pub type_: Option<String>`
    with `#[serde(rename = "type")]` (since `type` is a Rust keyword).
  - Add the same two fields to `TaskInput`.
  - Add `pub const TASK_TYPES: [&str; 5] = ["feature","bug","enhancement","chore","docs"];`.
- Task route (create + full-object PUT update): extend the INSERT/UPDATE SQL and the
  bound parameters to carry `version` and `type`, following the existing pattern that
  already writes `priority`/`size`/`description`/`checklist`/`position`.

## Frontend

- **Task detail modal**: a text input for **Version** and a `<select>` dropdown for
  **Type** (options = the five enum values plus a blank/none), placed alongside the
  existing priority/size controls. Values round-trip through the full-object PUT.
- **Board card**: render a colored **type** badge (reuse the existing badge styling used
  for priority/size), and — when `version` is set — a small **version** tag on the card.

## Out of scope (YAGNI)

- Grouping the board by version (badge-only for now).
- Managed per-project version sets / dropdown (free text is enough).
- Free-form multi-tag labels (type is a single fixed enum).
- Filtering UI (the badges already make versions/types visually scannable).

## Testing

- Migration applies cleanly; existing tasks get NULL version/type.
- Create a task with version + type → persists and round-trips through GET.
- Edit version/type via the modal (full-object PUT) → other fields unchanged.
- Invalid `type` value rejected by the DB check constraint.

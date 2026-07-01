# Recurring Events — Design

**Date:** 2026-07-01
**Status:** Approved, pre-implementation
**Scope:** Add recurring events to the calendar (e.g. "every Monday 12:00–13:00").

## Problem

The calendar stores single-instance events only (`event` table: `starts_at` /
`ends_at`). There is no way to create a repeating event. Users need daily /
weekly / monthly recurrence, and the ability to edit or delete a single
occurrence, a range, or the whole series.

## Decisions (locked)

| Question | Decision |
|----------|----------|
| Recurrence patterns | Simple presets: **Daily**, **Weekly** (on one weekday), **Monthly** (on one day-of-month). One frequency + a time window. |
| Per-occurrence edits | **Yes** — support "this occurrence", "this and following", "entire series". |
| Storage model | **Materialize** every occurrence as a real `event` row, tied by `series_id`. |
| End of recurrence | **Required end date.** No infinite series (rows are concrete). |

## Core model: materialized occurrences

Each occurrence is a real `event` row. A `series_id` (uuid) ties the rows of a
series together. Because occurrences are concrete rows:

- Per-occurrence edits/skips are free — edit or delete a row.
- The existing frontend range-fetch + group-by-day (`monthRange`,
  `eventsByDay`) is **unchanged** — occurrences are just more events.
- No rule table, no read-time expansion, no exception/override tables.

`series_id` null = standalone event. All existing rows remain valid (null).

## 1. Data model

`migrations/0004_recurring.sql`:

```sql
alter table event add column series_id uuid;
create index event_series_idx on event(series_id);
```

Expose `series_id` on the `Event` struct / API response (`src/models.rs`).

## 2. Occurrence generation — client-side

Occurrences are generated on the **client**, not the server.

Rationale: the frontend already does all local↔UTC conversion (`toISOString()`
on a local `Date`). "Every Monday 12:00 **local**" requires calendar-day
arithmetic in the user's timezone so the wall-clock time survives DST
transitions — JS `Date` (`setDate`, local constructors) does this natively.
Server-side generation would need a stored timezone plus `chrono-tz`. Client
generation reuses existing conversion, adds no Rust dependency, and is
DST-correct.

New pure function in `frontend/src/lib/calendar.js`:

```
generateOccurrences({ start, end, freq, until }) → [{ starts_at, ends_at }, …]
```

- `start`, `end` — the first occurrence's local start/end `Date`.
- `freq` — `'daily' | 'weekly' | 'monthly'`.
- `until` — inclusive local end date for the series.
- **Weekly** infers the weekday from `start`; **monthly** infers day-of-month
  from `start`; **daily** = every day.
- Returns UTC RFC3339 strings (`starts_at`, `ends_at`), preserving the local
  wall-clock time and the event duration on each occurrence.
- **Monthly on the 31st skips** months without that day (iCal `BYMONTHDAY`
  behavior — no clamp to the 28th/30th).

Unit-tested in the existing `frontend/src/lib/calendar.test.js`.

## 3. Server additions

Two small additions to `src/routes/events.rs`; single create/update/delete stay
as they are (the `series_id`-null path).

**Batch series create** — `POST /api/events/series`
- Body: `{ occurrences: [EventInput, …] }`.
- Validates each row (reuse existing `title` non-empty and `ends_at >=
  starts_at` checks).
- Inserts all rows in one transaction under a single freshly generated
  `series_id`.
- Returns the inserted rows.
- Rejects an empty list or a list longer than **366** rows (runaway guard) with
  `400`.

**Scoped delete** — `DELETE /api/events/:id?scope=one|following|series`
- Loads the target row first (needs its `series_id` and `starts_at`).
- `one` — delete just this row (default; also the standalone path).
- `following` — `delete from event where series_id = $1 and starts_at >= $2`.
- `series` — `delete from event where series_id = $1`.

## 4. Edit / delete scope (UI)

When the user edits or deletes an event that has a non-null `series_id`, the UI
prompts for scope.

| Action | This occurrence only | This and following | Entire series |
|--------|----------------------|--------------------|---------------|
| **Delete** | `DELETE ?scope=one` | `DELETE ?scope=following` | `DELETE ?scope=series` |
| **Edit** | existing single `PUT` (row diverges, keeps id) | `DELETE ?scope=following`, then regenerate the tail with new settings via `POST /series` (fresh `series_id`) | `DELETE ?scope=series`, then regenerate whole range via `POST /series` |

Series edits = **delete affected rows + regenerate**. This reuses the
generation path and avoids per-row wall-clock arithmetic on the server. The
regenerated tail gets a **new `series_id`** (Google-Calendar-style split), so a
later "entire series" edit on the original series does not clobber it. Events
are not referenced by any other entity, so regenerated ids dangle nothing —
and re-patterning a range is exactly the intended semantics.

## 5. Form (Calendar.svelte)

Add to the event create/edit form:

- **Repeat:** None / Daily / Weekly / Monthly.
- **Ends:** a date input, **required** when Repeat ≠ None (Save disabled until
  set).
- Weekday (weekly) and day-of-month (monthly) are inferred from the start date —
  no separate control.

## 6. Testing

**Rust** (`tests/`):
- Series batch-insert creates N rows sharing one `series_id`.
- Scoped delete: `one` / `following` / `series` each remove exactly the right
  rows.
- Batch validation rejects an empty list, an oversize list (> 366), and a row
  that fails the title / time checks.

**JS** (`calendar.test.js`):
- `generateOccurrences` for daily / weekly / monthly.
- `until` is inclusive.
- A spring-forward week stays at 12:00 **local** (DST correctness).
- Monthly-on-31st skips short months.

## Deliberate simplifications (ponytail)

- **No rule table.** Occurrences are concrete rows. Consequence: an occurrence
  can't render "repeats weekly on Mon", and changing the frequency/weekday of an
  existing series = delete + recreate, not an in-place rule edit. Acceptable for
  a solo ops hub.
- **Series edits regenerate rows** (new ids) — the re-pattern semantics we want;
  nothing references events, so no dangling refs.
- **Monthly-on-31st skips** short months rather than clamping.
- **Cap 366 rows/series** as a runaway guard.

## Out of scope (YAGNI)

- Multiple weekdays per week (Mon+Wed+Fri), "every N weeks" intervals.
- Full iCal RRULE (last-Friday-of-month, etc.).
- Infinite / no-end series.
- Recurring all-day events beyond what the preset covers (all_day flows through
  unchanged per row).

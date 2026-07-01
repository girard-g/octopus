# Calendar: Multi-Contact + Day View + Time Fix — Design

**Date:** 2026-07-01
**Status:** Approved, pre-implementation
**Scope:** Three cohesive calendar changes — (1) attach multiple contacts to an
event, (2) fix the "event time is always 00:00" bug, (3) add a day-by-day view.

## Problem

1. An `event` links to at most one contact (`event.contact_id`, nullable). Users
   need several people on one event (attendees).
2. Every event the user creates ends up at **00:00**. Root cause (verified by
   reproduction, not inspection): the single `datetime-local` field silently
   drops its time to `00:00` when the date is changed via the native widget, and
   nothing blocks saving a midnight event. The create/generate code itself is
   correct — probe events created with the untouched default saved at 09:00.
3. The calendar only has a month grid. Users want a single-day agenda.

## Decisions (locked)

| Question | Decision |
|----------|----------|
| Multi-contact storage | **Junction table** `event_contact`; migrate the existing single `contact_id` into it, then drop the column. |
| Time-fix approach | **Split** the `datetime-local` into a date field + start/end time fields. |
| Day-view style | **Agenda list** (chronological), with a Month\|Day toggle and prev/next-day nav. |

## 1. Data model — multi-contact

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

- `on delete cascade` on both sides: deleting an event or a contact cleans up its
  links automatically.
- `event_contact_contact_idx` supports the reverse lookup ("events for this
  contact") a contact page could add later.

### API shape changes (`src/models.rs`)

- `Event`: **remove** `contact_id`; **add** `contact_ids: Vec<Uuid>`.
- `EventInput`: **remove** `contact_id`; **add** `contact_ids: Vec<Uuid>`
  (serde `default` → an omitted field is `[]`).
- `SeriesUpdateInput`: **remove** `contact_id`; **add** `contact_ids: Vec<Uuid>`.
- `SeriesInput.occurrences` are `EventInput`, so each occurrence carries its own
  `contact_ids` (the client sends the same set on every occurrence).

### Reads return aggregated contact_ids

`list` and `get` select events joined to the junction, aggregating the ids in
one query:

```sql
select e.*,
       coalesce(array_agg(ec.contact_id) filter (where ec.contact_id is not null), '{}') as contact_ids
from event e
left join event_contact ec on ec.event_id = e.id
where <existing predicate>
group by e.id
order by e.starts_at
```

`Event.contact_ids` is annotated `#[sqlx(default)]`: the aggregated read maps it
from the `contact_ids` `uuid[]` column, while a plain `insert … returning *` (no
such column) falls back to `[]`. Write handlers then overwrite that `[]` with the
`contact_ids` they just persisted (they already hold the input set — no re-query).
Because `array_agg` is now in play, the read `where`/`order` clauses stay as they
are today, just wrapped by the join + `group by e.id`.

## 2. Backend write paths — every path maintains the junction

All write paths run in a transaction so the event row and its junction rows
commit together.

- **create** (`POST /api/events`): insert the event; then insert one
  `event_contact` row per `contact_ids` entry; return the event with its
  `contact_ids`.
- **series** (`POST /api/events/series`): inside the existing transaction, after
  inserting each occurrence insert that occurrence's junction rows (same contact
  set for all).
- **update** (`PUT /api/events/{id}`): update the event, then **replace** its
  links — `delete from event_contact where event_id = $1`, insert the new set.
- **scoped series edit** (`PATCH /api/events/{id}/series?scope=…`): after the
  content+shift `UPDATE … RETURNING id`, collect the affected event ids and
  replace their junction rows — `delete from event_contact where event_id =
  any($ids)`, then insert the new set for each id.
- A non-existent `contact_id` (FK violation) → `400`, matching the existing
  `create`/`create_series` FK→400 mapping.

Helper: a small internal `set_event_contacts(tx, event_id, &[Uuid])` (delete +
insert) used by create/update; and a batch variant over many event ids for
series/scoped-edit, to keep the four handlers DRY.

## 3. Time-fix (the 00:00 bug) — frontend only

Replace the single `datetime-local` control with **one date + two time fields**:

- Timed event form: `Date` (`<input type="date">`), `Start` and `End`
  (`<input type="time">`). Defaults `09:00` / `10:00`.
- `buildBody` (timed): `starts_at = new Date(\`${date}T${startTime}\`).toISOString()`,
  `ends_at = new Date(\`${date}T${endTime}\`).toISOString()` — same day.
- `openEdit`: derive `date` / `startTime` / `endTime` from the stored UTC instant
  in **local** time (same local-basis conversion used today).
- Validation: `End` must be `>= Start` (same-day). Reuse the existing
  "End must be >= start" message.
- All-day events keep the start-date / end-date range (unchanged).
- **Multi-day timed events are out of scope** (a timed event is one calendar
  day). All-day covers multi-day spans.

Because time now lives in a dedicated `type="time"` field, changing the date can
no longer wipe it — the 00:00 trap is closed.

## 4. Day-by-day view — frontend only

`Calendar.svelte` gains `view: 'month' | 'day'` and `selectedDate` (local ISO,
defaults today) state.

- **Header**: a `Month | Day` toggle. Month view is unchanged. Day view shows
  `[ < ]  <selectedDate>  [ > ]  today`.
- **Data**: day view fetches the single day's range through the existing
  `/api/events?from=&to=` (local-midnight of `selectedDate` → local-midnight of
  the next day).
- **Agenda render**: all-day events pinned at the top, then timed events sorted
  ascending by `starts_at`; each row shows `HH:MM  title  @person @person`.
  Empty day → a `// no events` line.
- **Interactions**: clicking a row opens the edit modal; `+ new event` opens the
  new-event modal prefilled with `selectedDate`. Optionally, clicking a month
  cell's day number switches to day view for that date (nice-to-have; the toggle
  is the primary path).

A pure helper `dayAgenda(events)` in `calendar.js` returns
`{ allDay: Event[], timed: Event[] }` sorted as above — unit-tested.

## 5. Multi-select contacts UI

The single contact `<select>` becomes a **checkbox chip list** of contacts
(fits the terminal aesthetic; the user has few contacts). `modal.ev.contact_ids`
is an array toggled by the checkboxes. Selected people render as `@name` chips:
in the month-cell chip `title` tooltip and inline in the day agenda. Project
selection is unchanged (still single).

## 6. Testing

**Rust** (`tests/`):
- Migration carries the pre-existing `contact_id` into `event_contact` (seed a
  contact-linked event, migrate, assert the junction row exists and the column
  is gone).
- `create` with `contact_ids: [a, b]` returns `contact_ids` containing both and
  writes two junction rows.
- `list`/`get` aggregate `contact_ids` (event with 2 contacts → array of 2;
  event with none → `[]`).
- `update` replaces the set (was `[a]`, update to `[b, c]` → junction now `[b,c]`).
- `series` create writes the contact set on every occurrence.
- scoped series edit replaces the set on exactly the affected occurrences.
- bad `contact_id` → `400`.

**JS** (`calendar.test.js`):
- `buildBody` composes correct UTC `starts_at`/`ends_at` from a date + a
  **non-midnight** time (regression guard: a 14:30 start must not become 00:00).
- `dayAgenda` puts all-day first, then timed sorted by start.

## Deliberate simplifications (ponytail)

- **Timed events are single-day.** Multi-day spans use all-day. Kills the need
  for a second date field on timed events and keeps the time-fix minimal.
- **Contacts are a flat multi-select** — no roles/RSVP/organizer distinction.
- Day view **reuses the existing range fetch**; no new endpoint.
- `set_event_contacts` is a small delete-then-insert; no diffing of the existing
  set (the contact count per event is tiny).

## Out of scope (YAGNI)

- Bulk-fixing the existing 184 midnight "Conception" events (the user fixes the
  series in two clicks after this ships, or a one-off SQL shift — not built).
- Contact roles / attendee status / invitations.
- Hour-grid (timeline) day view.
- A week view.
- Reverse "events on this contact" panel on the contact page (the index is laid
  down now so it's cheap to add later).

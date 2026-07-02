# Contacts redesign — relationship hub + card grid

**Date:** 2026-07-02
**Scope:** Frontend only. Redesign the Contacts feature to be more modern and more
powerful while keeping the existing terminal/console aesthetic. No backend or
schema changes.

## Goal

Turn Contacts from a flat CRUD table into a **relationship hub**: "see everything
about a contact at a glance." The power comes from *assembling* data that already
exists (notes + events + projects) into one view — not from new fields.

## Non-goals

- No new contact fields, no migration (`kind`/`name`/`email`/`phone`/`company_id`
  stay as-is).
- No new backend endpoints or server-side filters. All aggregation is client-side,
  matching the codebase's existing `ponytail:` client-filter pattern
  (`ContactDetail.svelte:30`).
- The create/edit `Modal` and its form fields are unchanged.

## Design language (unchanged, reused)

JetBrains Mono, near-black canvas (`--color-bg`), one neon-mint accent
(`--color-accent`), CRT scanlines + boot sweep, `> label` prompt headers,
`[ KIND ]` bracket badges, `glow-soft`/`glow-text` utilities, `.rise` entrance,
`.label` section labels. Kind colors: person → `st-lead` (blue), company →
`accent` (mint). Everything new must read as part of the existing shell.

## Data flow

Two components change: `Contacts.svelte` (list) and `ContactDetail.svelte` (hub).

**Endpoints (all pre-existing):**

- `GET /api/contacts` — all contacts. Used by list, and by detail for roster +
  company-name resolution.
- `GET /api/projects` — all projects; filtered client-side by `contact_id`
  (already done today). Also drives per-card project counts on the list.
- `GET /api/notes?contact_id=:id` — contact notes (already done today).
- `GET /api/events` — **new fetch on detail only**; filter client-side to events
  whose `contact_ids` array includes this contact's id.

**Derived, zero extra requests:**

- Unified timeline = notes (`created_at`) + this contact's events (dated), merged
  into one list, sorted descending. Future events float to the top under an
  `UPCOMING` group.
- Stats (active projects, total projects, note count, last-touch) are `$derived`
  from the arrays above.
- List cards' `N active · M total` counts come from grouping `/api/projects` by
  `contact_id` client-side.

## Component 1 — Contacts list (card grid)

Replaces the flat table with a responsive card grid (3-up → 2-up → 1-up).

**Command bar (top):**
- Live **search** input — filters cards on name / email / company name,
  client-side, `$derived`. Console styling (`> search` prompt, mint focus ring).
- **Kind toggle** — segmented `all / people / company`, filters by `kind`.

**Card contents:**
- Kind badge `[ PERSON ]` (blue) / `[ COMPANY ]` (mint).
- Name (primary, `text-ink`).
- Company line `⌂ Acme Corp` for people (`⌂ —` when none); for companies this line
  is omitted.
- Email + phone row.
- Stat line: `◈ N active · M total` projects.
- Footer quick actions: `[✉]` `mailto:`, `[☏]` `tel:`, `[⧉]` copy email to
  clipboard. Disabled/hidden when the field is absent.
- Whole-card click (and a `›` affordance) → `/contacts/:id`.
- Edit / delete move into a hover `⋯` menu to keep cards clean. Delete keeps the
  existing cascade-warning `confirm()`.

**States:** hover = mint border + `glow-soft`. Empty = existing `no contacts yet`
console line. Errors reuse the `[ ERR ]` banner.

## Component 2 — ContactDetail (relationship hub)

**Header:**
- Breadcrumb `< contacts / {name}` + `[ KIND ]` badge.
- Sub-line: `⌂ {company}` (links to company detail for a person) · email · phone.
- Quick actions: `[✉ email]` `mailto:`, `[☏ call]` `tel:`, `[⧉]` copy, `[ ⋯ ]`
  menu (edit → opens Modal / delete → cascade confirm, existing behavior).

**Stat strip** (`◈ stats`, derived): `N active · M projects · K notes · last touch
{humanized}`. Last-touch = most recent of any note or event date, rendered as
`3d ago` / `today` / `—` when empty.

**Projects panel** (`> projects`): same data (client-filtered by `contact_id`),
restyled; keeps `[ ACTIVE ]`/`[ ARCHIVED ]` status badges; each row → `/projects/:id`.

**Roster panel** (`> roster`, **companies only**): people whose `company_id` is
this contact, from `/api/contacts`. Each links to that person's detail. For a
`person`, this slot is not rendered (their company appears in the header instead),
so there's no dead space.

**Timeline panel** (`> timeline`, the centerpiece):
- Inline **note composer** floated at the top; `⌘⏎` / `Ctrl⏎` submits (button also
  present). Reuses the existing `POST /api/notes { body, contact_id }`.
- Merged, descending feed of notes + events.
- Future events grouped under an `⧗ UPCOMING` divider at the top.
- Row markers: `●` note, `◆` event. Each row shows text/title + date.
- Notes have `×` delete (existing `DELETE /api/notes/:id`). Events show `↗`
  deep-link to the calendar day (route to the existing Calendar view).
- Empty timeline: `· start of history ·` console line.

**Log shortcuts** (`log ▸ [note] [event] [project]`):
- `note` → focuses the composer.
- `event` / `project` → navigate to the existing Calendar / Projects pages.
  **No contact prefill** — navigate-only, marked with a `ponytail:` comment.
  Prefill wiring is out of scope.

## Testing

- Follow existing test style (`frontend/src/lib/*.test.js`, Vitest). Unit-test the
  pure helpers: timeline merge/sort (notes + events, upcoming-first), last-touch
  humanizer, and per-contact project counting. No component/e2e additions unless
  the existing suite already covers these routes.

## Out of scope / deferred

- Backend `?contact_id` / `?company_id` filters (client-side is intentional).
- Contact prefill when creating a project/event from the hub.
- New contact fields (role, address, tags, website).
- Pagination/virtualization on the card grid.

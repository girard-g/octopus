# Octopus — Solo Freelance Ops Hub

**Date:** 2026-06-30
**Status:** Approved design, ready for implementation planning
**Author:** Guillaume (freelance software developer)

## Purpose

One self-hosted tool to govern a solo freelance software business: calendar,
contacts (CRM), projects/tasks, notes, and a lead pipeline — in a single place.
Today these are scattered or nonexistent. Invoicing/accounting stays in **Indy**
(out of scope); Octopus links out to it.

Single user (the owner). No multi-tenancy, no signup, no team features.

## Non-goals (v1)

- Billable time tracking / timers — Indy owns billable hours.
- Invoicing, accounting, expenses, taxes — Indy.
- Multi-user, sharing, permissions.
- Email/calendar provider sync (Google/CalDAV) — Octopus owns its own data.
- Mobile-native apps — responsive web is enough.

## Architecture

- **Single Rust + Axum service.** Serves the JSON API *and* the built Svelte
  static assets from one process. One Docker container.
- **Postgres**, Coolify-managed (one-click, automated backups, survives
  redeploys). Access via `sqlx`.
- **Frontend:** Svelte SPA, talks only to the JSON API.
- **Auth:** single user. Password from env var; server issues a signed session
  cookie. No registration flow.
- **Deploy:** Docker image via Coolify on the owner's Hetzner server. Postgres
  connection string + session secret + user password injected as env.

**Design principle — API-first:** every feature goes through the JSON API. The
web SPA is just the first client. A keyboard-driven TUI client can be added
later against the same API with no backend changes. (TUI itself is deferred —
not built in any v1 phase.)

**Build-don't-buy boundary:** do not hand-roll a calendar grid. Use a Svelte
calendar component (e.g. `@event-calendar`). Markdown rendering uses an existing
library, not a custom parser.

## Data model (the spine)

The central move: **the lead pipeline is just a project's status.** A project's
lifecycle *is* the pipeline — no separate deal/opportunity entity. One table
drives both the lead board and the active-project list.

### `contact`
- `id`
- `kind`: `person` | `company`
- `name`
- `email` (nullable)
- `phone` (nullable)
- `company_id` (nullable FK → `contact` where kind=company; lets a person link
  to their company)
- `created_at`

### `project`
- `id`
- `contact_id` (FK → contact; the client)
- `title`
- `status`: `lead` | `proposal` | `active` | `done` | `lost`  ← the pipeline
- `description` (nullable)
- `invoice_url` (nullable text; link out to the matching Indy invoice)
- `board_order` (int; manual ordering within a status column)
- `created_at`

### `task`
- `id`
- `project_id` (nullable FK → project; null = standalone task)
- `title`
- `status`: `todo` | `doing` | `done`
- `due_on` (date, nullable)
- `created_at`

### `event`
- `id`
- `title`
- `starts_at`, `ends_at` (timestamptz)
- `all_day` (bool)
- `project_id` (nullable FK → project)
- `contact_id` (nullable FK → contact)
- `notes` (nullable text)

### `note`
- `id`
- `body` (markdown text)
- `contact_id` (nullable FK → contact)
- `project_id` (nullable FK → project)
- `created_at`
- (Exactly one of `contact_id` / `project_id` set; enforced in app logic.)

## Views

- **Dashboard (home):** today's events, tasks due soon, active projects, recent
  contacts. The "governing" landing screen.
- **Contacts:** list + detail. Detail shows the contact's projects, notes, and
  upcoming events.
- **Pipeline:** kanban board of projects grouped by `status`; drag a card
  between columns (`lead → proposal → active → done | lost`).
- **Calendar:** month / week / agenda views; click an event to edit and link it
  to a project/contact.
- **Notes:** rendered inline within contact and project detail (no standalone
  notes screen in v1).

## Phasing

Each phase is independently deployable and useful.

### Phase 1 — Spine
- Axum service skeleton, Postgres + `sqlx` migrations, cookie auth, Dockerfile,
  Coolify deploy.
- Contacts CRUD.
- Projects CRUD + status kanban board (this *is* the pipeline), incl.
  `invoice_url`.
- Tasks (under a project or standalone).
- Dashboard v1 (events panel stubbed until Phase 2).
- Svelte SPA shell + the above views.

### Phase 2 — Calendar
- `event` entity + endpoints.
- Calendar views (month/week/agenda) via a Svelte calendar library.
- Link events to projects/contacts; events surface on the dashboard.

### Phase 3 — Notes / docs
- `note` entity + endpoints; markdown notes inside contact/project detail.
- File attachments on contacts/projects (storage approach decided at plan time).

### Phase 4 — Extras / polish (deferred backlog)
- TUI client against the JSON API.
- Follow-up nudges (a `follow_up_on` date on contacts/leads surfaced on the
  dashboard).
- Quick-capture inbox (capture a task/note unfiled, sort later).
- Recurring events.

## Error handling

- API returns structured JSON errors with appropriate HTTP status codes.
- Validation at the API boundary (required fields, valid enum values, FK
  existence) before any DB write.
- DB constraints (FKs, NOT NULL, enum checks) as the backstop.
- Unauthenticated API requests → 401; the SPA redirects to login.

## Testing

- Backend: `sqlx`-backed integration tests per endpoint against a test Postgres
  (CRUD happy paths + validation/auth failures). Per global rule, `cargo check`
  and `cargo test` must pass before any task is considered done.
- Frontend: light component tests for the interactive pieces (kanban
  drag/drop, calendar event create/edit); not exhaustive.

## Open questions (resolve at plan time, not blocking)

- File-attachment storage in Phase 3: Postgres bytea vs. a mounted volume vs.
  object storage.
- Exact Svelte calendar library choice (Phase 2).
- Session-cookie crate / auth middleware choice (Phase 1).

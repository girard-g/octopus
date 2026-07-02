# Mobile-responsive octopus UI — design

**Date:** 2026-07-02
**Status:** Approved for planning

## Goal

Make the octopus web UI genuinely usable on a smartphone — responsive and
touch-first, not merely a shrunken desktop layout. Support full parity with
desktop, including dragging task cards between kanban columns by touch.

## Guiding principles

- **One structural breakpoint** at Tailwind `md` (768px). Below `md` we reflow
  to a mobile shell; at/above `md` the desktop UI stays **byte-identical** so we
  do not regress the existing desktop polish. In practice: base (unprefixed)
  classes describe the mobile layout, `md:` prefixes restore the current desktop
  layout. Where that inverts awkwardly, use `max-md:` for mobile-only rules.
- **Fluid within mobile**, not fixed: full-width panels, `min-w-0` guards on
  flex/grid children, no hardcoded pixel widths that can overflow the viewport,
  native momentum scroll.
- **Touch-first**: interactive targets ≥40px in the smaller dimension; no
  reliance on `:hover` to reveal actions; respect `env(safe-area-inset-bottom)`
  so the fixed bottom bar clears the home indicator/notch.
- **No new dependencies, no separate mobile components/routes.** Same `.svelte`
  files, responsive utility classes. Reuse existing markup for expanded views.
- Base font stays 13px; only paddings shrink on mobile (`px-8 py-7` → `px-4 py-4`).

## Baseline target

- Primary: 375px (iPhone) and 360px (common Android) widths.
- Graceful down to 320px (no horizontal overflow, content still reachable).
- Desktop (`md+`) unchanged.

## Components

### 1. Shell + navigation — `App.svelte`, `lib/components/Sidebar.svelte`

- **Sidebar**: `hidden md:flex` — hidden on mobile, unchanged on desktop.
- **Bottom tab bar** (new, `md:hidden`): `fixed bottom-0 inset-x-0`, 5 tabs for
  the routes `/`, `/contacts`, `/projects`, `/calendar`, `/notes`. Each tab is a
  short mono label (`dash`, `cont`, `proj`, `cal`, `note`) — no icon library.
  Active route is accent-mint with an underline/glow; inactive is `text-faint`.
  Bar has `border-t border-border bg-surface` and
  `pb-[env(safe-area-inset-bottom)]`. Tab height ≥48px.
  - Active-route detection reuses the same logic the sidebar uses (compare
    `router.location`); the tab bar can be a sibling of the sidebar driven by the
    same nav item list to avoid duplicating the route table.
- **Top bar** (in `App.svelte` header): keep the `> {title}` prompt. The
  `search… ⌘K` cue becomes `hidden md:flex` (meaningless without a keyboard). Add
  the `octopus` wordmark + a logout control (⏻) on the right, `md:hidden`
  (on desktop these live in the sidebar).
- **Main content**: add `pb-20 md:pb-0` so the fixed bottom bar never covers
  content; reduce horizontal padding on mobile (`px-4 md:px-8`, `py-4 md:py-7`).

### 2. Dashboard — `routes/Dashboard.svelte`

- Stat tiles: keep `grid-cols-3` (compact numbers meant to read as a row).
- Two main panels (`active_projects` | `tasks_due`): `grid-cols-2` →
  `grid-cols-1 lg:grid-cols-2` so they stack on phone and tablet, side-by-side on
  wide screens.
- `upcoming` section: already full-width, no change.

### 3. Projects / Contacts — `routes/Projects.svelte`, `routes/Contacts.svelte`

- Card grids currently `grid-cols-2 md:grid-cols-3` → `grid-cols-1
  sm:grid-cols-2 md:grid-cols-3`: single column on phone (full-width cards,
  bigger tap area), two on small tablets, three on desktop.
- Any inline add/filter forms: ensure inputs are full-width and ≥40px tall
  (the shared `FIELD` class already uses `w-full`).

### 4. ProjectBoard (kanban) — `routes/ProjectBoard.svelte`

Desktop unchanged. Mobile changes:

- **Header actions** (archive/edit/delete + back): ensure the row wraps
  (`flex-wrap`) and buttons stay ≥40px tall.
- **Board + notes rail**: the outer `flex … gap-4` wrapper becomes
  `flex-col md:flex-row` so the notes rail stacks **below** the board on mobile
  instead of competing for horizontal space.
- **Columns**: keep the `overflow-x-auto` horizontal scroller. On mobile, column
  width becomes `w-[85vw]` (so the next column peeks) with `snap-x snap-mandatory`
  on the scroller and `snap-center` on each column; desktop keeps
  `w-[280px] min-w-[240px] flex-1`. Peeking is what makes cross-column drag
  possible on a narrow screen.
- **Touch drag**: `svelte-dnd-action` has built-in touch support and auto-scrolls
  the scroll container when a dragged card nears its edge — this is how a card
  moves to the next (peeking) column. No config change expected beyond what is
  already passed to `dndzone`. **Risk:** touch drag can conflict with vertical
  list scroll / horizontal board scroll; if it does, add the library's
  drag-threshold/handle affordance. Verify empirically (see Testing).
- **Card delete `×`**: currently `opacity-0 … group-hover:opacity-100`. Since
  touch has no hover, make it always visible on mobile (`opacity-100 md:opacity-0
  md:group-hover:opacity-100`).

### 5. Calendar — `routes/Calendar.svelte`

Desktop unchanged. Mobile changes:

- Keep the month `grid-cols-7` (7 cells ≈ 46px at 328px content width).
- Day cells: hide event **text** on mobile; render up to 4 colored **event dots**
  (one per event, capped at 4 with the rest implied, using the existing
  status/type colors). Desktop keeps the text rows.
- **Tap a day** → that day's events render in a list **below the grid**, reusing
  the existing event-row markup (time + title + link). This replaces the
  in-cell text list on mobile.
- Event create/edit form grids (`grid-cols-2`, `grid-cols-3`) →
  `grid-cols-1 sm:grid-cols-2/3` so fields stack on phone.

### 6. Modal — `lib/components/Modal.svelte`

- Constrain to viewport: `w-[calc(100vw-2rem)] max-w-[400px]` and ensure the body
  scrolls (`max-h-[85vh] overflow-y-auto`) when content is tall. Keep it centered
  (not converting to a full bottom-sheet).

### 7. Global CSS — `app.css`

- No structural change required; mobile behavior is driven by utility classes in
  markup. Add a `-webkit-tap-highlight-color: transparent` reset if tap flashes
  are distracting. The CRT sweep/scanline overlays are unaffected and remain on
  mobile.

## Data flow

No data-flow or API changes. This is purely a presentation/layout change.
Calendar's "tap a day → show events below" adds a single piece of local
component state (`selectedDay`) in `Calendar.svelte`; everything else is CSS.

## Error handling

Unchanged — existing error banners (`[ ERR ] …`) already use `w-full`/wrap and
render fine at mobile widths; just confirm during the screenshot pass.

## Testing

- **Unit (existing vitest):** unchanged; layout work does not touch the tested
  logic modules (`tasks`, `calendar`, `guard`, `api`).
- **Manual verification (acceptance check):** using the CDP + headless-chromium
  screenshot harness, capture every route at **375px and 360px** widths, logged
  in:
  - Login, Dashboard, Projects, Contacts, ContactDetail, ProjectBoard, Calendar,
    Notes.
  - Assert per route: no horizontal overflow; bottom tab bar visible, reachable,
    and not covering content; active tab highlighted.
  - ProjectBoard: emulate a touch drag of a card to the peeking column and
    confirm it moves (this is the highest-risk item — verify explicitly).
  - Calendar: dots render; tapping a day shows its events below.
- No e2e framework exists in the repo; we are **not** adding one. The screenshot
  pass is the acceptance gate.

## Build order

One spec, implemented in four independently-verifiable steps:

1. **Shell + nav** — sidebar hide, bottom tab bar, mobile top bar, content
   padding. (Foundation; unblocks visual testing of every other route.)
2. **Dashboard + card grids** (Projects, Contacts) — straightforward reflows.
3. **Calendar** — dots + tap-to-expand + form stacking.
4. **ProjectBoard / kanban** — peek columns, snap scroll, notes-rail stacking,
   always-visible delete, touch-drag verification. (Highest risk, done last.)

## Out of scope (YAGNI)

- No CSS-framework swap, no icon library.
- No PWA/offline, no service worker, no install prompt.
- No custom touch gestures beyond native scroll + `svelte-dnd-action`.
- No separate mobile routes or duplicated components.
- No changes to the desktop layout or to any backend/API.

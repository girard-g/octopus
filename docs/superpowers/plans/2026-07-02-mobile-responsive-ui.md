# Mobile-Responsive octopus UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the octopus web UI touch-first responsive on phones (≤767px) while leaving the ≥768px desktop layout byte-identical.

**Architecture:** Single structural breakpoint at Tailwind `md` (768px). Base (unprefixed) utility classes describe the mobile layout; `md:` prefixes restore the current desktop layout. A fixed bottom tab bar replaces the sidebar below `md`; grids/flex containers reflow; the kanban gains peek-columns + snap scroll; the calendar month grid shows event dots and drills into the existing day view. No new dependencies, no separate mobile components/routes.

**Tech Stack:** Svelte 5 (runes), Tailwind CSS v4 (`@theme` tokens in `app.css`), `svelte-spa-router`, `svelte-dnd-action` (already installed, has built-in touch support), Vite, Vitest (jsdom).

## Global Constraints

- **Breakpoint:** exactly one structural divide at Tailwind `md` = 768px. Mobile = `<768px`, desktop = `≥768px`.
- **Desktop unchanged:** every rendered pixel at ≥768px must match `master` before this plan. Achieve this by only ever *adding* base/`max-md:` classes or converting a bare class into `base + md:<original>`. Never change an existing `md:`/`lg:` value.
- **No new npm dependencies.** No icon library, no CSS framework change.
- **No API/backend changes.** Presentation layer only. (Calendar adds one piece of client-only reactive state.)
- **Touch targets ≥40px** in their smaller dimension for anything tappable added or altered on mobile.
- **Safe area:** the fixed bottom bar must include `pb-[env(safe-area-inset-bottom)]`.
- **Verification is rendered-screenshot based**, not jsdom. jsdom does not evaluate media queries or layout, so responsive behavior is verified by capturing the real app in headless Chromium at 375px and 360px widths (harness in Task 1). Existing `npm test` (logic units) and `npm run build` must stay green after every task.
- **Copy/case rule:** nav labels in source stay natural-case for e2e (`Dashboard`, `Contacts`, …); CSS lowercases for display. Do not lowercase the source strings.

---

## File Structure

- `frontend/src/lib/nav.js` — **new.** Single source of truth for nav items, the active-route predicate, and `logout()`. Consumed by `Sidebar.svelte`, the new `BottomTabs.svelte`, and `App.svelte`.
- `frontend/src/lib/nav.test.js` — **new.** Unit tests for `NAV_ITEMS` shape and `isActive()`.
- `frontend/src/lib/components/BottomTabs.svelte` — **new.** Fixed mobile bottom tab bar (`md:hidden`).
- `frontend/src/lib/components/Sidebar.svelte` — modify: import from `nav.js`, hide on mobile.
- `frontend/src/App.svelte` — modify: render `BottomTabs`, mobile top-bar bits, content padding.
- `frontend/src/lib/components/Modal.svelte` — modify: clamp width to viewport, scroll tall bodies.
- `frontend/src/routes/Dashboard.svelte` — modify: panel grid reflow.
- `frontend/src/routes/Projects.svelte` — modify: card grid reflow.
- `frontend/src/routes/Calendar.svelte` — modify: month dots, drill-to-day on mobile, form grid stacking, cell height.
- `frontend/src/routes/ProjectBoard.svelte` — modify: peek columns, snap scroll, notes-rail stacking, always-visible delete, mobile board height.
- Verified but likely no code change (confirm by screenshot, tweak padding only if overflow): `Contacts.svelte`, `Notes.svelte`, `ContactDetail.svelte`, `Login.svelte`.

---

## Task 1: Verification harness + baseline capture

Stands up the running app and a headless-Chromium screenshot helper used by every later task. Nothing here is committed to the repo (dev-only scratch).

**Files:**
- Create (scratch, NOT committed): `<SCRATCH>/shot.mjs` where `<SCRATCH>` = the session scratchpad dir.

**Interfaces:**
- Produces: a shell recipe `capture <route> <width> <outfile>` (documented below) that later tasks call to screenshot an authenticated route at a given viewport width.

- [ ] **Step 1: Start Postgres-backed backend**

The backend needs env vars from `.env` (already present: `DATABASE_URL`, `SESSION_SECRET`, `APP_PASSWORD=change-me`, `PORT=8090`). A debug binary exists at `target/debug/octopus`; if not, `cargo build` first.

Run (from repo root):
```bash
pkill -x octopus 2>/dev/null; sleep 1
set -a && . ./.env && set +a
(./target/debug/octopus > /tmp/octopus.log 2>&1 &)
sleep 3
curl -s -o /dev/null -w "login:%{http_code}\n" -X POST http://localhost:8090/api/login \
  -H 'Content-Type: application/json' -d '{"password":"change-me"}'
```
Expected: `login:204`. If Postgres is down: `pg_isready -h localhost -p 5432` and start it.

- [ ] **Step 2: Start the Vite dev server**

Run (from `frontend/`):
```bash
(npm run dev > /tmp/vite.log 2>&1 &)
sleep 4
grep -oE 'localhost:[0-9]+' /tmp/vite.log | head -1
```
Expected: a `localhost:<PORT>` line (commonly 5173, or 5174 if 5173 is taken). Vite proxies `/api` → `http://localhost:8090` (see `frontend/vite.config.js`). **Use this printed port as `<VITE>` below.**

- [ ] **Step 3: Start headless Chromium with a debug port**

IMPORTANT: never `pkill -f` a pattern that also appears in your own command line (it kills your shell). Use `pkill -x chromium`.
```bash
pkill -x chromium 2>/dev/null; sleep 1
(chromium --headless=new --disable-gpu --no-sandbox --remote-debugging-port=9222 \
  --user-data-dir=/tmp/chrprof about:blank > /tmp/chr.log 2>&1 &)
sleep 5
curl -s http://127.0.0.1:9222/json/version | head -c 60; echo
```
Expected: a JSON blob starting with `{ "Browser": "Chrome/...`.

- [ ] **Step 4: Write the CDP screenshot helper**

Create `<SCRATCH>/shot.mjs` (the auth cookie is HttpOnly+Secure, so it must be injected via CDP `Network.setCookie`; localhost is a secure context so `secure:true` cookies work over http). Args: `<route> <width> <outfile>`; env `AUTH_COOKIE`, `VITE`.
```js
const [,, route='/', width='375', out='/tmp/shot.png'] = process.argv
const cookie = process.env.AUTH_COOKIE
const vite = process.env.VITE || 'localhost:5173'
const sleep = ms => new Promise(r => setTimeout(r, ms))
const targets = await (await fetch('http://localhost:9222/json')).json()
const page = targets.find(t => t.type === 'page')
const ws = new WebSocket(page.webSocketDebuggerUrl)
let id = 0; const pend = new Map()
const send = (m, p={}) => { const i = ++id; ws.send(JSON.stringify({id:i,method:m,params:p})); return new Promise(r => pend.set(i, r)) }
ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id && pend.has(m.id)) { pend.get(m.id)(m.result); pend.delete(m.id) } }
await new Promise(r => ws.onopen = r)
await send('Network.enable'); await send('Page.enable')
await send('Emulation.setDeviceMetricsOverride', {width:Number(width), height:800, deviceScaleFactor:2, mobile:true})
await send('Network.setCookie', {name:'auth', value:cookie, domain:'localhost', path:'/', httpOnly:true, sameSite:'Lax', secure:true})
await send('Page.navigate', {url:'about:blank'}); await sleep(200)
await send('Page.navigate', {url:`http://${vite}/#${route}`}); await sleep(2500)
const {data} = await send('Page.captureScreenshot', {format:'png', captureBeyondViewport:true})
;(await import('fs')).writeFileSync(out, Buffer.from(data, 'base64'))
console.log('wrote', out); ws.close()
```

- [ ] **Step 5: Define the `capture` recipe and take BEFORE screenshots**

The reusable recipe (later tasks call this exact form; `<VITE>` from Step 2):
```bash
AUTH=$(curl -s -D - -o /dev/null -X POST http://localhost:8090/api/login \
  -H 'Content-Type: application/json' -d '{"password":"change-me"}' \
  | grep -i set-cookie | sed -E 's/.*auth=([^;]+);.*/\1/')
# capture <route> <width> <outfile>:
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/' 375 /tmp/before-dash-375.png
```
Capture the current (pre-change) state of every route at 375px for reference:
`/`, `/projects`, `/contacts`, `/calendar`, `/notes`, `/login`, and one project board (`/projects/<id>` — get an id: `curl -s localhost:8090/api/projects -b <(printf 'auth=%s' "$AUTH") | head`).

- [ ] **Step 6: Confirm baseline is broken (sanity)**

Open `/tmp/before-dash-375.png` (Read tool). Expected: the 220px sidebar consumes most of the 375px width and content is crushed — confirming the problem. No commit (harness is scratch-only).

---

## Task 2: Shell + navigation (bottom tab bar, mobile top bar, Modal)

Replaces the sidebar with a fixed bottom tab bar below `md`, moves wordmark+logout into a mobile top bar, adds content bottom-padding, and clamps the Modal to the viewport. Extracts nav data into a shared module first.

**Files:**
- Create: `frontend/src/lib/nav.js`
- Test: `frontend/src/lib/nav.test.js`
- Create: `frontend/src/lib/components/BottomTabs.svelte`
- Modify: `frontend/src/lib/components/Sidebar.svelte`
- Modify: `frontend/src/App.svelte`
- Modify: `frontend/src/lib/components/Modal.svelte`

**Interfaces:**
- Produces:
  - `NAV_ITEMS: { href: string, n: string, label: string, short: string }[]` (5 entries)
  - `isActive(location: string, href: string): boolean`
  - `logout(): Promise<void>`
- Consumes: `svelte-spa-router` (`link`, `push`, `router`), `./api.js` (`api`), `./session.svelte.js` (`markLoggedOut`).

- [ ] **Step 1: Write the failing test for `nav.js`**

Create `frontend/src/lib/nav.test.js`:
```js
import { describe, it, expect } from 'vitest'
import { NAV_ITEMS, isActive } from './nav.js'

describe('NAV_ITEMS', () => {
  it('has the five routes with href + short label', () => {
    expect(NAV_ITEMS.map((i) => i.href)).toEqual(['/', '/contacts', '/projects', '/calendar', '/notes'])
    for (const i of NAV_ITEMS) {
      expect(typeof i.short).toBe('string')
      expect(i.short.length).toBeGreaterThan(0)
      expect(i.label[0]).toBe(i.label[0].toUpperCase()) // natural-case for e2e
    }
  })
})

describe('isActive', () => {
  it('matches dashboard only exactly', () => {
    expect(isActive('/', '/')).toBe(true)
    expect(isActive('/projects', '/')).toBe(false)
  })
  it('matches a section and its detail sub-routes', () => {
    expect(isActive('/projects', '/projects')).toBe(true)
    expect(isActive('/projects/abc123', '/projects')).toBe(true)
    expect(isActive('/contacts/xy', '/contacts')).toBe(true)
  })
  it('does not cross-match sections', () => {
    expect(isActive('/contacts', '/projects')).toBe(false)
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run (from `frontend/`): `npx vitest run src/lib/nav.test.js`
Expected: FAIL — `Failed to resolve import "./nav.js"`.

- [ ] **Step 3: Create `nav.js`**

Create `frontend/src/lib/nav.js`:
```js
import { push } from 'svelte-spa-router'
import { api } from './api.js'
import { markLoggedOut } from './session.svelte.js'

// Single source of truth for navigation. `label` stays natural-case (e2e matches
// <a> textContent); CSS lowercases for the terminal look. `short` is the compact
// label used on the mobile bottom tab bar. `n` is the desktop sidebar numbering.
export const NAV_ITEMS = [
  { href: '/', n: '01', label: 'Dashboard', short: 'dash' },
  { href: '/contacts', n: '02', label: 'Contacts', short: 'cont' },
  { href: '/projects', n: '03', label: 'Projects', short: 'proj' },
  { href: '/calendar', n: '04', label: 'Calendar', short: 'cal' },
  { href: '/notes', n: '05', label: 'Notes', short: 'note' },
]

// Active when the location equals the href, or (for non-root sections) is a
// sub-route of it — so a project board highlights the Projects tab.
export function isActive(location, href) {
  if (href === '/') return location === '/'
  return location === href || location.startsWith(href + '/')
}

export async function logout() {
  try { await api.post('/api/logout') } catch { /* ignore */ }
  markLoggedOut()
  push('/login')
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/lib/nav.test.js`
Expected: PASS (3 files? no — 1 file, 3 test blocks all green).

- [ ] **Step 5: Refactor `Sidebar.svelte` to use `nav.js` (no visual change)**

In `frontend/src/lib/components/Sidebar.svelte`:
- Replace the `<script>` import block + inline `items`/`logout` with:
```js
<script>
  import { link, router } from 'svelte-spa-router'
  import { NAV_ITEMS, logout } from '../nav.js'
  const items = NAV_ITEMS
</script>
```
(Remove the now-unused `push`, `api`, `markLoggedOut` imports and the local `logout`/`items` definitions. Keep the exact-match `active` logic — `router.location === it.href` — unchanged so desktop highlighting is byte-identical.)
- Change the root `<aside>` class from:
  `sticky top-0 flex h-screen w-[220px] shrink-0 flex-col border-r border-border bg-surface`
  to:
  `sticky top-0 hidden h-screen w-[220px] shrink-0 flex-col border-r border-border bg-surface md:flex`
  (i.e. `flex` → `hidden … md:flex`).

- [ ] **Step 6: Create `BottomTabs.svelte`**

Create `frontend/src/lib/components/BottomTabs.svelte`:
```svelte
<script>
  import { link, router } from 'svelte-spa-router'
  import { NAV_ITEMS, isActive } from '../nav.js'
</script>

<!-- Fixed thumb-reachable tab bar; mobile only. Clears the home indicator via
     safe-area inset. Sits above content (z-20) which pads its bottom (pb-20). -->
<nav
  class="fixed inset-x-0 bottom-0 z-20 grid grid-cols-5 border-t border-border bg-surface/95 pb-[env(safe-area-inset-bottom)] backdrop-blur md:hidden"
  aria-label="Primary"
>
  {#each NAV_ITEMS as it}
    {@const active = isActive(router.location, it.href)}
    <a
      href={it.href}
      use:link
      class="relative flex h-14 flex-col items-center justify-center gap-1 font-mono text-[11px] lowercase transition-colors"
      class:text-accent={active}
      class:glow-text={active}
      class:text-faint={!active}
      aria-current={active ? 'page' : undefined}
    >
      {#if active}<span class="absolute inset-x-3 top-0 h-0.5 rounded-full bg-accent glow-soft"></span>{/if}
      <span>{it.short}</span>
      <span class="sr-only">{it.label}</span>
    </a>
  {/each}
</nav>
```

- [ ] **Step 7: Wire `BottomTabs` + mobile top bar + padding into `App.svelte`**

In `frontend/src/App.svelte`:
- Add imports:
```js
  import BottomTabs from './lib/components/BottomTabs.svelte'
  import { logout } from './lib/nav.js'
```
- In the authed branch (`{:else if showChrome(...)}`), inside `<main class="min-w-0 flex-1">`, add the mobile wordmark+logout to the header. Change the header (currently line ~66) to append, right after the `<h1>…</h1>` block and BEFORE the `search…` cue div, a mobile-only cluster; and make the search cue desktop-only. Concretely:
  - The `search…` cue `<div class="ml-auto flex items-center gap-2 …">` → prepend `hidden md:flex` and drop the now-redundant `ml-auto` from it (the mobile cluster takes `ml-auto` instead). Result class starts: `ml-auto hidden items-center gap-2 rounded-sm … md:flex`.
  - Immediately BEFORE that cue div, insert:
```svelte
            <div class="ml-auto flex items-center gap-3 md:hidden">
              <span class="select-none font-mono text-[13px] font-bold tracking-tight text-ink">octopus<span class="cursor text-accent glow-text">▋</span></span>
              <button
                onclick={logout}
                aria-label="Logout"
                title="Logout"
                class="grid h-9 w-9 place-items-center rounded-sm border border-border text-[14px] text-faint transition-colors hover:border-st-lost/50 hover:text-st-lost"
              >⏻</button>
            </div>
```
- Content wrapper (currently `<div class="px-8 py-7">`) → `<div class="px-4 py-4 pb-24 md:px-8 md:py-7 md:pb-7">` (mobile: tighter padding + bottom room for the tab bar).
- Just before the closing `</div>` of `<div class="flex min-h-screen">` (after `</main>`), render the tab bar so it appears on all authed chrome pages:
```svelte
    </main>
    <BottomTabs />
  </div>
```
  (`BottomTabs` is `position: fixed`, so its DOM placement inside the flex row is irrelevant to layout.)

- [ ] **Step 8: Clamp the Modal to the viewport (`Modal.svelte`)**

In `frontend/src/lib/components/Modal.svelte`, the panel div (line ~20) currently has `w-full max-w-[400px]`. Change to `w-[calc(100vw-2rem)] max-w-[400px] max-h-[85dvh] overflow-y-auto`. This keeps it centered, prevents horizontal overflow on narrow screens, and lets tall forms (e.g. the event editor) scroll.

- [ ] **Step 9: Verify — tests, build, screenshots**

```bash
cd frontend && npx vitest run && npm run build
```
Expected: all tests PASS (including the 3 new nav blocks), build succeeds.

Then screenshot (harness from Task 1 must be running; restart vite if you stopped it):
```bash
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/' 375 /tmp/t2-dash-375.png
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/' 768 /tmp/t2-dash-768.png
```
Read both. Assert on the 375px shot: no sidebar; a bottom tab bar with `dash cont proj cal note`, `dash` highlighted mint; top bar shows `octopus▋` + ⏻; content is full-width and not hidden behind the bar. Assert on the 768px shot: sidebar visible, NO bottom bar, top bar shows the `search… ⌘K` cue (i.e. desktop unchanged).

- [ ] **Step 10: Commit**

```bash
git add frontend/src/lib/nav.js frontend/src/lib/nav.test.js \
  frontend/src/lib/components/BottomTabs.svelte frontend/src/lib/components/Sidebar.svelte \
  frontend/src/App.svelte frontend/src/lib/components/Modal.svelte
git commit -m "feat(web): mobile shell — bottom tab bar, mobile top bar, viewport-safe modal"
```

---

## Task 3: Dashboard + card-grid reflows

Stacks multi-column grids on phones. Small, purely additive class changes.

**Files:**
- Modify: `frontend/src/routes/Dashboard.svelte`
- Modify: `frontend/src/routes/Projects.svelte`

**Interfaces:** none (self-contained layout classes).

- [ ] **Step 1: Reflow the Dashboard two-panel row**

In `frontend/src/routes/Dashboard.svelte`, the panels row (line ~92):
`<div class="rise grid grid-cols-2 gap-4" style="animation-delay:60ms">`
→ change `grid-cols-2` to `grid-cols-1 lg:grid-cols-2`:
`<div class="rise grid grid-cols-1 gap-4 lg:grid-cols-2" style="animation-delay:60ms">`
(The 3 stat tiles at line ~72 stay `grid-cols-3` — compact numbers read fine as a row at 360px. The `upcoming` section is already full-width.)

- [ ] **Step 2: Reflow the Projects card grid**

In `frontend/src/routes/Projects.svelte` (line ~65):
`<div class="rise grid grid-cols-2 gap-3 md:grid-cols-3" style="animation-delay:40ms">`
→ prepend a single-column base and a small-tablet step:
`<div class="rise grid grid-cols-1 gap-3 sm:grid-cols-2 md:grid-cols-3" style="animation-delay:40ms">`

- [ ] **Step 3: Verify — build + screenshots**

```bash
cd frontend && npm run build
```
Expected: build succeeds.
```bash
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/' 360 /tmp/t3-dash-360.png
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/projects' 360 /tmp/t3-proj-360.png
```
Read both. Assert: dashboard `active_projects` and `tasks_due` panels are stacked (full-width, one above the other); project cards are one-per-row full-width; no horizontal scrollbar.

- [ ] **Step 4: Confirm Contacts / Notes / ContactDetail don't overflow**

These use full-width list rows / flex-wrap headers already (no multi-col grids). Screenshot to confirm — only edit if a shot shows horizontal overflow:
```bash
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/contacts' 360 /tmp/t3-cont-360.png
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/notes' 360 /tmp/t3-notes-360.png
```
Read both. Expected: content fits within 360px, no horizontal overflow. If a row overflows, add `min-w-0` to the offending flex child and `break-words` to long text, then re-shoot. (Do not restructure otherwise.)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/routes/Dashboard.svelte frontend/src/routes/Projects.svelte
git commit -m "feat(web): stack dashboard panels and project cards on mobile"
```

---

## Task 4: Calendar — month dots + drill-to-day + form stacking

On mobile the month grid shows event dots instead of text, tapping a day drills into the existing day view (reused, not rebuilt), and the event-editor date/time grids stack.

**Files:**
- Modify: `frontend/src/routes/Calendar.svelte`
- Test: `frontend/src/routes/Calendar.mobile.test.js` (new)

**Interfaces:**
- Produces: `openDayCell(iso: string): void` — on mobile drills into day view (`view='day'; selectedDate=iso`); on desktop opens the new-event modal (`openNew(iso)`). Decision reads a reactive `isMobile` boolean.
- Consumes: existing `openNew`, `view`, `selectedDate` from the same component.

- [ ] **Step 1: Write the failing test for the day-cell branch**

The branch decision is the one bit of real logic. Extract it as a pure helper so it is testable without a DOM. Create `frontend/src/routes/Calendar.mobile.test.js`:
```js
import { describe, it, expect } from 'vitest'
import { dayCellAction } from '../lib/calendar.js'

describe('dayCellAction', () => {
  it('drills into day view on mobile', () => {
    expect(dayCellAction(true)).toBe('day')
  })
  it('creates a new event on desktop', () => {
    expect(dayCellAction(false)).toBe('new')
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run (from `frontend/`): `npx vitest run src/routes/Calendar.mobile.test.js`
Expected: FAIL — `dayCellAction` is not exported from `../lib/calendar.js`.

- [ ] **Step 3: Add the pure helper to `calendar.js`**

Append to `frontend/src/lib/calendar.js`:
```js
// On mobile a day cell drills into the day agenda; on desktop it starts a new event.
export function dayCellAction(isMobile) {
  return isMobile ? 'day' : 'new'
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/routes/Calendar.mobile.test.js`
Expected: PASS.

- [ ] **Step 5: Add reactive `isMobile` + `openDayCell` to `Calendar.svelte`**

In the `<script>` of `frontend/src/routes/Calendar.svelte`:
- Add `dayCellAction` to the existing import from `../lib/calendar.js`.
- Add a reactive viewport flag and the handler:
```js
  // Reactive <768px flag driven by matchMedia (browser only; SSR-safe guard).
  let isMobile = $state(false)
  $effect(() => {
    if (typeof window === 'undefined') return
    const mq = window.matchMedia('(max-width: 767px)')
    isMobile = mq.matches
    const on = (e) => { isMobile = e.matches }
    mq.addEventListener('change', on)
    return () => mq.removeEventListener('change', on)
  })

  function openDayCell(iso) {
    if (dayCellAction(isMobile) === 'day') {
      selectedDate = iso
      view = 'day'
    } else {
      openNew(iso)
    }
  }
```

- [ ] **Step 6: Point the day cell at `openDayCell` and add mobile dots**

In the month grid (line ~275), change the cell's `onclick={() => openNew(cell.iso)}` to `onclick={() => openDayCell(cell.iso)}`. Update its `aria-label` from `"New event on {cell.iso}"` to `"{cell.iso}"` (it no longer always creates).

Shrink cell height on mobile — change `min-h-[90px]` (line ~277) to `min-h-[54px] md:min-h-[90px]`.

Replace the event-chips block (lines ~285–299) so text chips are desktop-only and dots are mobile-only:
```svelte
            {#if byDay.has(cell.iso)}
              {@const dayEvents = byDay.get(cell.iso)}
              <!-- Mobile: up to 4 event dots -->
              <div class="flex flex-wrap gap-0.5 md:hidden" aria-hidden="true">
                {#each dayEvents.slice(0, 4) as _ev}
                  <span class="h-1.5 w-1.5 rounded-full bg-accent glow-soft"></span>
                {/each}
              </div>
              <!-- Desktop: text chips (unchanged) -->
              <div class="hidden md:block">
                {#each dayEvents.slice(0, 3) as ev}
                  <button
                    onclick={(e) => openEdit(ev, e)}
                    class="mb-0.5 w-full truncate rounded-sm bg-accent-dim/20 px-1.5 py-0.5 text-left font-mono text-[11px] text-accent transition hover:bg-accent-dim/40"
                    title="{ev.title}{ev.all_day ? '' : ' ' + fmtTime(ev.starts_at)}{ev.contact_ids?.length ? ' — ' + ev.contact_ids.map(contactName).join(', ') : ''}"
                  >
                    {#if !ev.all_day}<span class="text-faint">{fmtTime(ev.starts_at)} </span>{/if}{ev.title}
                  </button>
                {/each}
                {#if dayEvents.length > 3}
                  <div class="mt-0.5 font-mono text-[10px] text-faint">+{dayEvents.length - 3} more</div>
                {/if}
              </div>
            {/if}
```

- [ ] **Step 7: Stack the event-editor date/time grids on mobile**

In the modal form, three grids need a single-column base:
- All-day dates (line ~346): `class="grid grid-cols-2 gap-2"` → `class="grid grid-cols-1 gap-2 sm:grid-cols-2"`
- Timed date/start/end (line ~357): `class="grid grid-cols-3 gap-2"` → `class="grid grid-cols-1 gap-2 sm:grid-cols-3"`
- Repeat/ends (line ~373): `class="grid grid-cols-2 gap-2"` → `class="grid grid-cols-1 gap-2 sm:grid-cols-2"`

- [ ] **Step 8: Verify — tests, build, screenshots**

```bash
cd frontend && npx vitest run && npm run build
```
Expected: all tests PASS (including `Calendar.mobile.test.js`), build succeeds.
```bash
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/calendar' 375 /tmp/t4-cal-375.png
```
Read it. Assert: month grid fits 375px with no horizontal overflow; days with events show small mint dots (no text chips); day-of-week row intact. Then verify drill-down by capturing the day view (the harness can't tap, so assert the day toggle works):
```bash
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/calendar' 768 /tmp/t4-cal-768.png
```
Read it. Assert desktop still shows text chips (unchanged). For the tap-drill, manually confirm in a real browser at mobile width if available; otherwise trust the unit-tested `dayCellAction` + the wired handler.

- [ ] **Step 9: Commit**

```bash
git add frontend/src/routes/Calendar.svelte frontend/src/lib/calendar.js frontend/src/routes/Calendar.mobile.test.js
git commit -m "feat(web): mobile calendar — event dots + tap-to-drill day view + stacked form"
```

---

## Task 5: ProjectBoard / kanban — peek columns, snap, notes stacking, touch drag

The hardest task. Columns become peek-width with snap scroll (so a card can be dragged toward the peeking next column), the notes rail stacks below the board on mobile, the board height switches to a mobile-friendly `dvh`, and the hover-only delete becomes tap-visible.

**Files:**
- Modify: `frontend/src/routes/ProjectBoard.svelte`

**Interfaces:** none new (layout + one always-visible affordance).

- [ ] **Step 1: Make the board section grow naturally on mobile**

The root section (line ~194) pins a desktop height:
`<section class="rise flex h-[calc(100vh-108px)] flex-col">`
On mobile the shell header (52) + mobile padding (32) + bottom tab bar (~64) differ, and mobile browser chrome moves the viewport — so use natural height on mobile and let the page scroll:
→ `<section class="rise flex h-auto flex-col md:h-[calc(100vh-108px)]">`
(Also update the `ponytail:` comment above it to note the mobile branch uses `h-auto` + per-scroller heights.)

- [ ] **Step 2: Stack the board and notes rail on mobile**

The board+notes wrapper (line ~245):
`<div class="flex min-h-0 flex-1 gap-4">`
→ `<div class="flex min-h-0 flex-1 flex-col gap-4 md:flex-row">`

- [ ] **Step 3: Give the horizontal scroller a mobile height + snap**

The inner scroller (line ~246):
`<div class="flex min-h-0 flex-1 gap-3 overflow-x-auto pb-1">`
→ add a bounded mobile height and horizontal snap:
`<div class="flex h-[65dvh] min-h-0 gap-3 overflow-x-auto pb-1 snap-x snap-mandatory md:h-auto md:flex-1 md:snap-none">`

- [ ] **Step 4: Make columns peek-width and snap on mobile**

Each column (line ~248):
`<div class="flex min-h-0 w-[280px] min-w-[240px] flex-1 flex-col overflow-hidden rounded-md border border-border/60 bg-bg-2/40">`
→ mobile: 85vw wide, snap-center; desktop unchanged:
`<div class="flex min-h-0 w-[85vw] shrink-0 snap-center flex-col overflow-hidden rounded-md border border-border/60 bg-bg-2/40 md:w-[280px] md:min-w-[240px] md:shrink md:flex-1 md:snap-align-none">`
(If `md:snap-align-none` is not a Tailwind utility in this version, use `md:[scroll-snap-align:none]`.)

- [ ] **Step 5: Make the card delete affordance tap-visible on mobile**

The per-card delete `×` (line ~286) is hover-revealed (`opacity-0 … group-hover:opacity-100`), which is unreachable by touch. Prepend `opacity-100 md:opacity-0` so it is always visible on mobile and hover-revealed on desktop:
current: `class="shrink-0 font-mono text-[16px] leading-none text-faint opacity-0 transition hover:text-st-lost focus:opacity-100 group-hover:opacity-100 group-focus-within:opacity-100"`
→ `class="shrink-0 font-mono text-[16px] leading-none text-faint opacity-100 transition hover:text-st-lost focus:opacity-100 md:opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"`

- [ ] **Step 6: Stack the notes rail full-width on mobile**

The notes `<aside>` (line ~317):
`<aside class="flex w-[300px] shrink-0 flex-col overflow-hidden rounded-md border border-border/60 bg-bg-2/40">`
→ `<aside class="flex max-h-[50dvh] w-full shrink-0 flex-col overflow-hidden rounded-md border border-border/60 bg-bg-2/40 md:max-h-none md:w-[300px]">`

- [ ] **Step 7: Verify — build + screenshots + touch-drag check**

```bash
cd frontend && npx vitest run && npm run build
```
Expected: tests PASS, build succeeds.

Get a project id and screenshot the board at 375px:
```bash
PID=$(curl -s http://localhost:8090/api/projects -b <(printf 'auth=%s' "$AUTH") | grep -oE '"id":[[:space:]]*"[a-f0-9-]+"' | head -1 | grep -oE '[a-f0-9-]{36}')
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs "/projects/$PID" 375 /tmp/t5-board-375.png
AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs "/projects/$PID" 768 /tmp/t5-board-768.png
```
Read both. Assert on 375px: one column ~85% wide with the next peeking at the right edge; header buttons wrapped and tappable; card `×` visible without hover; no page-level horizontal overflow (only the board scroller scrolls). Assert on 768px: columns flex to fill, no peek, delete `×` hidden until hover (desktop unchanged).

**Touch-drag (the flagged risk):** the harness cannot emulate a drag gesture. Verify manually — open the board in a real browser at mobile width (e.g. Chrome DevTools device toolbar, or a phone on the LAN via `npm run dev -- --host`), then drag a card toward the peeking column and confirm: (a) the drag starts without hijacking vertical list scroll, and (b) the scroller auto-scrolls so the card lands in the next column. If drag fights scrolling, add a drag handle or the library's touch delay: pass `{ ...existing, dragDisabled: false, dropFromOthersDisabled: false }` is already default — instead set a small press delay via `svelte-dnd-action`'s `use:dndzone` option `{ delayTouchStart: true }`-equivalent (check the installed version's README for the exact option name, e.g. `dragStartDelay`/`morphDisabled`). Document what you changed in the commit.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/routes/ProjectBoard.svelte
git commit -m "feat(web): mobile kanban — peek columns, snap scroll, stacked notes, tap-visible delete"
```

---

## Task 6: Full-route acceptance sweep

Confirms every route at both target widths and that desktop is untouched. No code unless a regression surfaces.

**Files:** none (verification), unless a fix is needed.

- [ ] **Step 1: Capture every route at 375px and 360px**

With the harness running and `$AUTH`/`<VITE>`/`$PID` set:
```bash
for R in '/' '/projects' '/contacts' '/calendar' '/notes' "/projects/$PID"; do
  N=$(echo "$R" | tr '/' '_')
  for W in 375 360; do
    AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs "$R" $W /tmp/final${N}_$W.png
  done
done
# login is unauthenticated:
AUTH_COOKIE="" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs '/login' 375 /tmp/final_login_375.png
```

- [ ] **Step 2: Review each shot against the acceptance checklist**

Read each `/tmp/final*_*.png`. For every route assert:
- No horizontal overflow (content ends within the viewport width).
- Bottom tab bar present, `dash cont proj cal note`, the current section highlighted, not covering content.
- Tap targets look ≥40px; text is legible (no crushed columns).
List any failures; fix inline in the owning route file following the same base/`md:` pattern, rebuild, re-shoot the affected route.

- [ ] **Step 3: Confirm desktop is byte-identical**

```bash
for R in '/' '/projects' '/contacts' '/calendar' '/notes' "/projects/$PID"; do
  N=$(echo "$R" | tr '/' '_')
  AUTH_COOKIE="$AUTH" VITE="localhost:<VITE>" node <SCRATCH>/shot.mjs "$R" 1280 /tmp/final${N}_1280.png
done
```
Read a couple (dashboard, board). Assert they match the pre-change desktop look: sidebar present, no bottom bar, original grids/columns. (Compare against the Task 1 BEFORE shots if you captured desktop ones.)

- [ ] **Step 4: Final green check + commit any fixes**

```bash
cd frontend && npx vitest run && npm run build
```
Expected: all PASS, build clean. If Step 2 required fixes:
```bash
git add -A frontend/src
git commit -m "fix(web): mobile responsive acceptance-sweep tweaks"
```
If no fixes were needed, no commit — the feature is complete across Tasks 2–5.

---

## Self-Review notes (for the planner; delete on execution)

- **Spec coverage:** shell+nav (T2), dashboard/card grids (T3), calendar dots+drill+form (T4), kanban peek/snap/notes/delete/touch (T5), Modal (T2 Step 8), safe-area (T2 Step 6), testing harness + acceptance (T1/T6). Contacts/Notes/ContactDetail explicitly checked (T3 Step 4). All spec sections mapped.
- **Deviation from spec (approved rationale):** spec §5 said "events render in a list below the grid"; plan instead drills into the *existing* day view (`view='day'`) — same event-row markup, less new code (DRY). Outcome identical for the user.
- **Type/name consistency:** `NAV_ITEMS`, `isActive`, `logout`, `dayCellAction`, `openDayCell` used consistently across tasks and tests.
- **Risk called out honestly:** touch-drag (T5 Step 7) is verify-manually with a concrete fallback, not assumed working.

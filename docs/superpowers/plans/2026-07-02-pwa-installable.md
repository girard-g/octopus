# Installable PWA Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make octopus installable to the phone home screen — standalone launch, terminal-themed OS chrome, app-shell precache — without offline data.

**Architecture:** Add `vite-plugin-pwa` (`generateSW`) to emit a manifest + Workbox service worker (precaching Vite's hashed assets) into the existing `../static` build output. Add a `> ▋` app icon set (one source SVG rasterized to PNGs via headless chromium) and PWA `<meta>`/`<link>` tags to `index.html`. Verify the Rust static handler serves the new files with correct content-types (not swallowed by the SPA `index.html` fallback).

**Tech Stack:** Svelte 5 + Vite, Tailwind v4, `vite-plugin-pwa` (Workbox) [new dev dep], Rust/Axum + tower-http `ServeDir` (static serving), headless chromium (icon rasterization + verification).

## Global Constraints

- **One new dependency only:** `vite-plugin-pwa` (devDependencies). No other new deps (icons via already-present chromium; no icon-generator lib).
- **No offline data:** the service worker precaches the static app shell only. It must **never** cache or intercept `/api/*` (`navigateFallbackDenylist: [/^\/api\//]`, no runtime caching).
- **No API/backend feature changes.** The only permissible backend change is static content-type/fallback correctness, and only if Task 3 finds a gap.
- **Brand values (verbatim):** accent mint `#3ef5c4`; canvas near-black `#06070a`. `theme_color` and `background_color` are both `#06070a` (the canvas, not the mint).
- **Manifest:** `name: "Octopus"`, `short_name: "Octopus"`, `display: "standalone"`, `start_url: "/"`, `scope: "/"`, `description: "Self-hosted hub for your freelance business — projects, tasks, contacts, calendar, notes."`, `registerType: "autoUpdate"`.
- **Build output dir** is `../static` (from `frontend/vite.config.js`), served by the Rust backend `ServeDir::new("static")` at `src/app.rs:61-64`.
- **Verify PWA against the Rust-served build** (`http://localhost:8090`), not the vite dev server — only the backend exercises the real static-serving integration.

---

## File Structure

- `frontend/scripts/gen-icons.mjs` — **new.** Node script: defines the two source SVGs (standard + maskable), rasterizes them to PNGs via headless chromium, writes all icon assets into `frontend/public/`. Committed for reproducibility.
- `frontend/public/` — **new assets** (Vite copies `public/` to the build root → `static/`): `icon-192.png`, `icon-512.png`, `icon-512-maskable.png`, `apple-touch-icon.png`, `favicon.svg`.
- `frontend/vite.config.js` — modify: add and configure `VitePWA(...)`.
- `frontend/index.html` — modify: add PWA `<meta>`/`<link>` tags.
- `frontend/package.json` — modify: `vite-plugin-pwa` in `devDependencies` (via `npm install -D`).
- `src/app.rs` — **verify only**; modify solely if Task 3 finds a content-type/fallback gap.

---

## Task 1: App icons

Generate the `> ▋` icon set from one source SVG via headless chromium. No runtime code; deliverable is the committed image assets + generator.

**Files:**
- Create: `frontend/scripts/gen-icons.mjs`
- Create (generated): `frontend/public/icon-192.png`, `frontend/public/icon-512.png`, `frontend/public/icon-512-maskable.png`, `frontend/public/apple-touch-icon.png`, `frontend/public/favicon.svg`

**Interfaces:**
- Produces: the five asset files at the exact paths above (Task 2's manifest + `index.html` reference them by root path, e.g. `/icon-192.png`).

- [ ] **Step 1: Write the icon generator**

Create `frontend/scripts/gen-icons.mjs`. The glyph is drawn as SVG primitives (a chevron polyline + a cursor rect) — font-independent, crisp at every size. The standard icon fills ~55% of the canvas; the maskable variant is smaller (~65% scaled down and centered) to sit inside Android's ~80% safe zone. Background is full-bleed `#06070a` on every raster (so the dark icon is visible on any launcher and iOS gets no transparency).

```js
// Generates the octopus PWA icon set: a mint "> ▋" prompt on near-black.
// Rasterizes SVG → PNG via headless chromium (no image-lib dependency).
import { writeFileSync, mkdirSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const MINT = '#3ef5c4', BG = '#06070a'
const PUBLIC = new URL('../public/', import.meta.url).pathname

// scale: fraction of the 512 viewBox the glyph group occupies (1 = the coords below).
// A smaller scale (maskable) shrinks the glyph toward the center for the safe zone.
function svg(scale = 1) {
  // Base glyph coords are tuned for scale=1 (standard icon).
  const glyph = `
    <polyline points="176,196 256,256 176,316" fill="none" stroke="${MINT}"
      stroke-width="34" stroke-linecap="round" stroke-linejoin="round"/>
    <rect x="292" y="214" width="46" height="84" rx="6" fill="${MINT}"/>`
  // Scale the glyph group about the canvas center (256,256).
  const tx = 256 * (1 - scale), ty = 256 * (1 - scale)
  return `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
    <rect width="512" height="512" fill="${BG}"/>
    <g filter="url(#g)" transform="translate(${tx} ${ty}) scale(${scale})">${glyph}</g>
    <defs><filter id="g" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="6" result="b"/><feMerge>
      <feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>
  </svg>`
}

const STANDARD = svg(1)          // > ▋ filling ~55%
const MASKABLE = svg(0.72)       // shrunk into the maskable safe zone

// Rasterize an SVG string to a PNG of the given pixel size via headless chromium.
function raster(svgStr, size, outName) {
  const html = `<!doctype html><html><head><meta charset="utf-8">
    <style>html,body{margin:0;padding:0;background:${BG}}
    svg{display:block;width:${size}px;height:${size}px}</style></head>
    <body>${svgStr}</body></html>`
  const htmlPath = join(tmpdir(), `octo-icon-${size}-${outName}.html`)
  const pngPath = join(PUBLIC, outName)
  writeFileSync(htmlPath, html)
  execFileSync('chromium', [
    '--headless', '--disable-gpu', '--no-sandbox', '--hide-scrollbars',
    '--force-device-scale-factor=1', `--window-size=${size},${size}`,
    `--screenshot=${pngPath}`, '--default-background-color=00000000',
    `file://${htmlPath}`,
  ], { stdio: 'inherit' })
  console.log('wrote', pngPath)
}

mkdirSync(PUBLIC, { recursive: true })
raster(STANDARD, 192, 'icon-192.png')
raster(STANDARD, 512, 'icon-512.png')
raster(MASKABLE, 512, 'icon-512-maskable.png')
raster(STANDARD, 180, 'apple-touch-icon.png')
writeFileSync(join(PUBLIC, 'favicon.svg'), STANDARD)
console.log('wrote', join(PUBLIC, 'favicon.svg'))
```

- [ ] **Step 2: Run the generator**

Run (from `frontend/`): `node scripts/gen-icons.mjs`
Expected: five "wrote …" lines; no errors. (If `chromium` isn't on PATH, use the full path — `which chromium` → `/usr/bin/chromium` on this machine.)

- [ ] **Step 3: Verify the assets exist at the right sizes**

Run (from `frontend/`):
```bash
for f in icon-192 icon-512 icon-512-maskable apple-touch-icon; do
  python3 -c "from PIL import Image; im=Image.open('public/$f.png'); print('$f', im.size, im.mode)" 2>/dev/null \
   || node -e "const b=require('fs').readFileSync('public/$f.png');console.log('$f',b.readUInt32BE(16),'x',b.readUInt32BE(20))"
done
ls -la public/favicon.svg
```
Expected: `icon-192` = 192×192, `icon-512` = 512×512, `icon-512-maskable` = 512×512, `apple-touch-icon` = 180×180; `favicon.svg` present. (The node fallback reads PNG width/height from the IHDR chunk.)

- [ ] **Step 4: Visually confirm the icons**

Open each PNG with the Read tool (`frontend/public/icon-512.png`, `frontend/public/icon-512-maskable.png`, `frontend/public/apple-touch-icon.png`). Confirm: mint `>` chevron + block cursor on near-black; the **maskable** version has visibly more padding (glyph well inside the edges); no clipping; soft mint glow. If the glyph is off-center or clipped, adjust the `scale`/coords in `gen-icons.mjs` and re-run Step 2.

- [ ] **Step 5: Commit**

```bash
cd /home/guillaume/Work/tries/2026-06-30-octopus
git add frontend/scripts/gen-icons.mjs frontend/public/icon-192.png frontend/public/icon-512.png frontend/public/icon-512-maskable.png frontend/public/apple-touch-icon.png frontend/public/favicon.svg
git commit -m "feat(pwa): app icon set (> block-cursor motif) + generator"
```

---

## Task 2: PWA wiring (plugin, manifest, meta tags)

Add and configure `vite-plugin-pwa`, and add the PWA meta/link tags to `index.html`. Build and verify the manifest + service worker via CDP.

**Files:**
- Modify: `frontend/package.json` (via `npm install -D vite-plugin-pwa`)
- Modify: `frontend/vite.config.js`
- Modify: `frontend/index.html`

**Interfaces:**
- Consumes: the icon files from Task 1 at `/icon-192.png`, `/icon-512.png`, `/icon-512-maskable.png`, `/apple-touch-icon.png`, `/favicon.svg`.
- Produces (at build): `static/manifest.webmanifest`, `static/sw.js`, `static/workbox-*.js`; SW registration injected into the app.

- [ ] **Step 1: Install the plugin**

Run (from `frontend/`): `npm install -D vite-plugin-pwa`
Expected: added to `devDependencies`; `npm ls vite-plugin-pwa` shows a version.

- [ ] **Step 2: Configure VitePWA in `vite.config.js`**

The current file (verify before editing):
```js
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  build: { outDir: '../static', emptyOutDir: true },
  server: {
    port: 5173,
    proxy: { '/api': 'http://localhost:8090' },
  },
  test: { environment: 'jsdom' },
})
```
Replace it with (adds the import and the `VitePWA({...})` plugin):
```js
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    svelte(),
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      // Icons live in public/ (Task 1); include them in the precache too.
      includeAssets: ['favicon.svg', 'apple-touch-icon.png'],
      manifest: {
        name: 'Octopus',
        short_name: 'Octopus',
        description: 'Self-hosted hub for your freelance business — projects, tasks, contacts, calendar, notes.',
        display: 'standalone',
        start_url: '/',
        scope: '/',
        theme_color: '#06070a',
        background_color: '#06070a',
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
          { src: '/icon-512-maskable.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,svg,png,woff2}'],
        navigateFallback: '/index.html',
        navigateFallbackDenylist: [/^\/api\//],  // never serve the shell for API requests
        // no runtimeCaching: API is never cached
      },
    }),
  ],
  build: { outDir: '../static', emptyOutDir: true },
  server: {
    port: 5173,
    proxy: { '/api': 'http://localhost:8090' },
  },
  test: { environment: 'jsdom' },
})
```

- [ ] **Step 3: Add PWA meta/link tags to `index.html`**

Current `<head>`:
```html
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Octopus</title>
  </head>
```
Replace with (the plugin auto-injects `<link rel="manifest">` and the SW registration, so do NOT add those by hand):
```html
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="theme-color" content="#06070a" />
    <meta name="description" content="Self-hosted hub for your freelance business — projects, tasks, contacts, calendar, notes." />
    <meta name="apple-mobile-web-app-capable" content="yes" />
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
    <meta name="apple-mobile-web-app-title" content="Octopus" />
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
    <title>Octopus</title>
  </head>
```

- [ ] **Step 4: Build and confirm PWA artifacts are emitted**

Run (from `frontend/`): `npm run build`
Then:
```bash
ls -la ../static/manifest.webmanifest ../static/sw.js
ls ../static/ | grep -E 'workbox|icon-|apple-touch|favicon'
grep -o 'rel="manifest"' ../static/index.html
```
Expected: `manifest.webmanifest` and `sw.js` exist; a `workbox-*.js` exists; icons + favicon are in `../static/`; `index.html` contains `rel="manifest"`. Also confirm `npx vitest run` still passes (no logic changed).

- [ ] **Step 5: Verify manifest validity + SW registration via CDP**

Ensure the verification harness is up (start if needed — same pattern used throughout this repo):
```bash
cd /home/guillaume/Work/tries/2026-06-30-octopus
pkill -x octopus 2>/dev/null; sleep 1
set -a && . ./.env && set +a
(./target/debug/octopus > /tmp/octopus.log 2>&1 &)   # serves ../static at :8090
sleep 3
pkill -x chromium 2>/dev/null; sleep 1
(chromium --headless=new --disable-gpu --no-sandbox --remote-debugging-port=9222 --user-data-dir=/tmp/chrprof about:blank > /tmp/chr.log 2>&1 &)
sleep 5
curl -s http://127.0.0.1:9222/json/version | head -c 40; echo
```
Write `/tmp/pwa-check.mjs`:
```js
// Loads the Rust-served build (:8090), reads the parsed manifest + installability
// errors via CDP, and confirms a service worker registers.
const sleep = ms => new Promise(r => setTimeout(r, ms))
const targets = await (await fetch('http://localhost:9222/json')).json()
const ws = new WebSocket(targets.find(t => t.type === 'page').webSocketDebuggerUrl)
let id = 0; const pend = new Map()
const send = (m, p={}) => { const i = ++id; ws.send(JSON.stringify({id:i,method:m,params:p})); return new Promise(r => pend.set(i, r)) }
ws.onmessage = e => { const m = JSON.parse(e.data); if (m.id && pend.has(m.id)) { pend.get(m.id)(m.result); pend.delete(m.id) } }
await new Promise(r => ws.onopen = r)
await send('Page.enable'); await send('Runtime.enable')
await send('Page.navigate', { url: 'http://localhost:8090/' })
await sleep(3500)   // let the SW register
const man = await send('Page.getAppManifest')
console.log('MANIFEST url:', man.url)
console.log('MANIFEST errors:', JSON.stringify(man.errors || []))
const sw = await send('Runtime.evaluate', {
  expression: `navigator.serviceWorker.getRegistrations().then(r => r.length + ' registration(s); scope=' + (r[0]&&r[0].scope))`,
  awaitPromise: true,
})
console.log('SW:', sw.result.value)
ws.close()
```
Run: `node /tmp/pwa-check.mjs`
Expected: `MANIFEST url` ends with `/manifest.webmanifest`; `MANIFEST errors: []` (empty — no installability errors); `SW: 1 registration(s); scope=http://localhost:8090/`. If `errors` is non-empty, read them (common causes: bad icon path/size, missing maskable) and fix the manifest/icons, rebuild, re-run.

- [ ] **Step 6: Commit**

```bash
cd /home/guillaume/Work/tries/2026-06-30-octopus
git add frontend/package.json frontend/package-lock.json frontend/vite.config.js frontend/index.html
git commit -m "feat(pwa): vite-plugin-pwa manifest + app-shell service worker + install meta"
```

---

## Task 3: Backend static-serving verification

Confirm the Rust backend serves the new PWA files with correct content-types and does not swallow them into the SPA `index.html` fallback. Fix only if a gap is found.

**Files:**
- Verify: `src/app.rs:61-64` (`ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"))`)
- Modify **only if a content-type gap is found**: `src/app.rs`

**Interfaces:**
- Consumes: the built `static/manifest.webmanifest`, `static/sw.js`, `static/icon-192.png` from Task 2.

- [ ] **Step 1: Check content-types of the PWA files served by the Rust backend**

With the backend from Task 2 still running at :8090 (restart per Task 2 Step 5 if not), run from repo root:
```bash
for p in manifest.webmanifest sw.js icon-192.png favicon.svg; do
  echo -n "/$p -> "; curl -s -o /dev/null -w "%{http_code} %{content_type}\n" "http://localhost:8090/$p"
done
```
Expected:
- `/manifest.webmanifest -> 200 application/manifest+json` (tower-http's `mime_guess` maps `.webmanifest`; **anything other than `text/html` and a 200** is acceptable — `text/html` means the SPA fallback swallowed it, which is the failure case)
- `/sw.js -> 200 text/javascript` (or `application/javascript`)
- `/icon-192.png -> 200 image/png`
- `/favicon.svg -> 200 image/svg+xml`

- [ ] **Step 2: Decision — is a fix needed?**

- **If all four return 200 with a non-`text/html` type:** no backend change is required. The static handler already serves real files before the fallback. Record the four `code content_type` lines in the task report and skip to Step 4 (no commit — the deliverable is the verified-correct serving, captured in the report).
- **If `/manifest.webmanifest` (or any) returns `text/html` or a 404:** the SPA fallback is intercepting it (or the file is missing) — apply the fix in Step 3.

- [ ] **Step 3: (Only if Step 2 found a gap) Force the manifest content-type with an explicit route**

Add a dedicated route ahead of the static fallback so the manifest is served with the correct type. In `src/app.rs`, add the import and a route before `.fallback_service(static_files)`:
```rust
use axum::response::IntoResponse;
use axum::http::header;

async fn manifest() -> impl IntoResponse {
    // Served explicitly to guarantee the PWA content-type regardless of mime_guess.
    let body = std::fs::read_to_string("static/manifest.webmanifest").unwrap_or_default();
    ([(header::CONTENT_TYPE, "application/manifest+json")], body)
}
```
and register it on the router (adjust to the existing router-builder style in `app.rs`):
```rust
    // ... existing route registrations ...
    .route("/manifest.webmanifest", axum::routing::get(manifest))
    .fallback_service(static_files)
```
Then rebuild the backend (`cargo build`), restart it, and re-run Step 1 — expect `/manifest.webmanifest -> 200 application/manifest+json`. (`sw.js`/`png`/`svg` are handled correctly by `ServeDir`; only add routes for any that actually failed.)

- [ ] **Step 4: Confirm the backend build + tests (only if Step 3 changed Rust)**

If Step 3 modified `src/app.rs`:
```bash
cd /home/guillaume/Work/tries/2026-06-30-octopus
cargo build 2>&1 | tail -3      # expect: Finished / no errors
cargo test 2>&1 | tail -5       # expect: all existing tests pass
```
If Step 3 was skipped, there is nothing to build here.

- [ ] **Step 5: Commit (only if Step 3 changed Rust)**

```bash
cd /home/guillaume/Work/tries/2026-06-30-octopus
git add src/app.rs
git commit -m "fix(pwa): serve manifest.webmanifest with application/manifest+json"
```
If no Rust change was needed, there is no commit for this task — the verification evidence in the task report is the deliverable.

---

## Self-Review notes (planner; delete on execution)

- **Spec coverage:** icons (T1), plugin+manifest+SW+meta (T2), backend serving verification (T3), verification via CDP `getAppManifest` + content-type curl against the Rust build (T2.5 + T3). `navigateFallbackDenylist` for `/api` and no runtime caching (T2.2). theme/background `#06070a` (T2.2). autoUpdate (T2.2). All spec sections mapped.
- **Deliberate simplification vs spec:** spec listed `favicon.ico` in addition to `favicon.svg`; the plan ships **`favicon.svg` only** (all current browsers support SVG favicons; a true `.ico` needs an ICO encoder we don't have — avoiding a dependency). Flag for the user if a legacy-browser `.ico` is actually wanted.
- **Placeholder scan:** none — every step has concrete code/commands. T3 is legitimately conditional (verify-then-fix), with the full contingency code provided, not deferred.
- **Type/name consistency:** icon filenames (`icon-192.png`, `icon-512.png`, `icon-512-maskable.png`, `apple-touch-icon.png`, `favicon.svg`) are identical across T1 (produced), T2 manifest/meta (consumed), and T3 checks.

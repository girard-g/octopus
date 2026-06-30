# Octopus "Terminal" — Design System (cyber / hacker-tool, dark)

Supersedes the "Ink" system. The aesthetic is a **refined terminal / cyber**
look — a power-user's CRT, not a cheesy Matrix gimmick. Mono everything, near-
black canvas, one neon accent used with restraint + glow, subtle CRT/grid
texture, command-palette feel. Distinctive and opinionated. Fits a developer
who lives in the terminal.

**Commit boldly.** The previous attempts were "meh / safe." This must have a
memorable signature: the bracketed mono labels, the prompt glyphs, the blinking
cursor, the faint scanline/grid, the accent glow. Refined, not corny.

**Anti-goals:** no generic Linear-clone grays, no Inter/sans UI, no pastel, no
rainbow neon overload, no literal "Matrix rain." Restraint + one strong accent.

## Type — MONO IS THE IDENTITY
- **Primary (almost everything):** JetBrains Mono, self-hosted (`@fontsource/jetbrains-mono`, weights 400/500/700). `font-variant-numeric: tabular-nums`.
- Long-form body (notes only, later): a readable sans is OK; everywhere else is mono.
- Base 13px. Headings: mono 700, can be UPPERCASE with letter-spacing. Section labels: `> label` prompt style, mono, faint→accent.

## Color tokens (Tailwind 4 `@theme` in app.css)
```
--color-bg:        #06070a   /* near-black canvas */
--color-bg-2:      #0a0c11
--color-surface:   #0d1016   /* panels/cards */
--color-surface-2: #12161d   /* hover/raised/inputs */
--color-border:    #1b2husband... use #1b2129  /* hairline */
--color-border-2:  #283041
--color-ink:       #d7e0d8   /* slightly green-tinted off-white */
--color-muted:     #7e8a83
--color-faint:     #4d564f
--color-accent:    #3ef5c4   /* neon mint — the ONE accent (bioluminescent ink) */
--color-accent-dim:#1c8f76
--color-on-accent: #04130e
/* status (neon-tuned, still usable): */
--color-st-lead:     #6fb2ff   /* cool blue */
--color-st-proposal: #f5c14e   /* amber */
--color-st-active:   #3ef5c4   /* accent mint */
--color-st-done:     #5af58a   /* green */
--color-st-lost:     #ff5c6a   /* red */
```
(Fix the `--color-border` value to `#1b2129`.)

## Signature elements (this is what makes it stand out — do all of them)
1. **Scanlines + grid:** a fixed full-screen overlay with very subtle horizontal scanlines (repeating-linear-gradient, ~3-4% opacity) AND a faint dotted/line grid on the canvas. `pointer-events:none`. Tasteful, low-contrast.
2. **Accent glow:** accent text/borders/active states get a soft neon glow (`text-shadow: 0 0 8px rgba(62,245,196,.5)` / `box-shadow: 0 0 0 1px var(--accent), 0 0 12px rgba(62,245,196,.18)`). Used ONLY on accent things, never body text.
3. **Prompt glyphs:** section headers prefixed with `>`; the wordmark is `octopus` with a blinking block cursor `▋` after it (CSS blink animation). Nav items numbered: `01`, `02`… in faint mono before the label.
4. **Bracketed status:** status badges render `[ LEAD ]` `[ ACTIVE ]` etc. in mono, status color, with brackets.
5. **Boot reveal:** on first authed mount, a fast staggered "type-in"/scan reveal of the shell + content (subtle, ~300–500ms total, respects prefers-reduced-motion).
6. **Command-bar cue:** a top-bar element styled like a command input with a `⌘K` / `/` hint chip (visual cue; a working palette is optional/stretch).
7. **Square-ish cards:** small radius (2–4px), hairline borders; corner tick accents (e.g. tiny `┐`/`└` or a 6px accent corner) on key panels. Hover = border brightens to accent-dim + faint glow.

## Layout
- **Sidebar** ~220px, bg surface, right hairline. Wordmark `octopus▋` (accent glow + blink). Nav: `01 dashboard / 02 contacts / 03 pipeline / 04 calendar / 05 notes` — number faint, label mono; active = accent text + glow + a `>` prompt marker + surface-2 bg. Footer: `user@octopus` + `logout`. Keep nav as real `<a use:link>` in a `<nav>` with the visible labels Dashboard/Contacts/Pipeline/Calendar/Notes (case can be styled).
- **Main:** top bar = page title as `> dashboard` style + the command-bar cue + right-aligned primary action. Content dense.
- Custom thin accent-tinted scrollbars.

## Components
- **Button primary:** accent bg, on-accent text, 2–4px radius, mono 500, subtle glow, hover brighten. **ghost:** transparent, border, ink, hover surface-2 + border-2.
- **Input/select/textarea:** bg surface-2, 1px border, 2–4px radius, mono, focus = accent border + accent glow ring. A leading `>` or `$` prompt inside command-like inputs.
- **Stat tile:** huge mono number (accent for headline), `> label` faint caption, corner tick.
- **Pipeline (sales) column:** header `[ STATUS ]` badge + mono count; cards square-ish, status-colored left bar + faint glow on hover, mono client name, `> invoice` link if present. Cards clickable → open the project's task board.
- **Per-project task board (NEW):** route `/projects/:id`. Header: project title, `[ status ]`, client, edit + back. Three columns `[ TODO ] [ DOING ] [ DONE ]`, drag tasks between (svelte-dnd-action) → on drop PUT the task full-object with new status. Add-task input per board (command-styled). Task card: title, due date (mono), delete.
- **Modal:** scrim `rgba(3,4,7,.7)` + blur; panel surface, 1px border (accent-dim), small radius, glow shadow, a `> title` header.
- **Table (contacts):** dense mono rows, hairline separators, header `> col` faint, row hover surface-2.

## Motion
- Boot reveal (above). Blinking cursor on wordmark. Hovers 120ms. Accent glow pulses subtly on active nav (optional, very subtle). `prefers-reduced-motion`: disable blink/boot, keep static.

## Accessibility
- Keep contrast AA (ink/muted on near-black pass). Status conveyed by label text, not color alone. `:focus-visible` accent glow ring. Scanline/grid are decorative `pointer-events:none` and must not reduce text contrast below AA.

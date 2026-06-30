# Octopus "Ink" — Design System (Linear-style dense pro, dark)

The aesthetic for the octopus web UI. Every screen follows this. Dark, refined,
technical, dense — a power-user tool, not a friendly SaaS. Octopus-ink identity:
deep blue-black canvas, hairline structure, one luminous teal accent.

**Anti-goals (the previous UI failed here):** no generic Inter/Roboto, no
purple gradients, no pastel rounded SaaS look, no big empty gray boxes, no
flat undifferentiated typography. Density + precision + restraint.

## Type
- **UI / body:** IBM Plex Sans (self-hosted via `@fontsource/ibm-plex-sans`, weights 400/500/600).
- **Data / numbers / dates / status / counts / IDs:** IBM Plex Mono (`@fontsource/ibm-plex-mono`, 400/500), `font-variant-numeric: tabular-nums`.
- Base size **13px** (dense). Headings tight: `letter-spacing: -0.01em`, weight 600. Section labels: mono, 11px, uppercase, `letter-spacing: 0.08em`, faint.

## Color tokens (define in Tailwind 4 `@theme` in app.css)
```
--color-bg:           #0a0c10   /* app canvas */
--color-surface:      #12151b   /* cards, panels, sidebar */
--color-surface-2:    #171b22   /* hover, raised, inputs */
--color-border:       #232831   /* hairline borders (1px everywhere) */
--color-border-strong:#2e333d
--color-ink:          #e8eaef   /* primary text */
--color-muted:        #8b929e   /* secondary text */
--color-faint:        #5c636e   /* labels, meta */
--color-accent:       #45d6c4   /* teal — the ONE accent */
--color-accent-dim:   #2b8a80
--color-on-accent:    #04110f   /* text on accent fills */
/* pipeline status (desaturated for dark): */
--color-st-lead:      #8a93a3
--color-st-proposal:  #d9a441
--color-st-active:    #45d6c4
--color-st-done:      #4cc38a
--color-st-lost:      #e06a6a
```
Status badge = color@14% bg + color text + 1px color@32% border, mono uppercase 11px, rounded 4px.

## Layout
- **Left sidebar**, fixed ~216px, bg surface, right hairline border. Top: wordmark `🐙 octopus` (mono, accent glow). Nav items (Dashboard, Contacts, Pipeline, Calendar, Notes) each with a 16px inline-SVG icon; active = accent 2px left-border + surface-2 bg + ink text; inactive = muted, hover surface-2. Bottom: user row + logout (faint, hover ink).
- **Main:** slim top bar (page title, weight 600, + a right-aligned primary action button), then content with generous but dense padding. Content max-width ~1100px.
- Custom thin faint scrollbars. Subtle radial glow behind the sidebar wordmark.

## Components
- **Panel/Card:** bg surface, 1px border, radius 8px, shadow `0 1px 2px rgba(0,0,0,.4)`. Dense padding (12–16px).
- **Stat tile:** mono number 28px (ink; accent for the headline metric), tiny mono uppercase faint label.
- **Button — primary:** accent bg, on-accent text, radius 6px, h~32px, weight 500, hover brighten (`filter: brightness(1.08)`). **secondary/ghost:** transparent, 1px border, ink text, hover surface-2.
- **Input/Select/Textarea:** bg surface-2, 1px border, radius 6px, h~32px, ink text, placeholder faint; focus = accent border + faint accent ring (`box-shadow: 0 0 0 3px rgba(69,214,196,.12)`).
- **Kanban column:** transparent with a header row (status badge + mono count); cards = surface, hairline border, radius 6px, hover = border-strong + slight lift; a 2px status-colored left edge on each card.
- **Modal:** backdrop `rgba(4,6,10,.6)` + `backdrop-filter: blur(4px)`; panel surface, 1px border, radius 10px, big soft shadow.
- **Table:** dense rows (h~38px), hairline row separators, header mono uppercase faint, row hover surface-2.

## Motion (subtle, fast)
- Page content mounts with a small staggered fade + translate-y (8px→0), 220ms, ~40ms stagger between sections.
- Hovers/focus 120ms ease. No bouncy/elastic, no scattered micro-noise. One composed entrance > many fidgets.

## Accessibility
- `:focus-visible` accent ring on all interactive elements. Status never conveyed by color alone (label text present). Contrast: ink/muted on bg pass AA.

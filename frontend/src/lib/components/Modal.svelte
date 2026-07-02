<script>
  let { title, onclose, children } = $props()
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onclose() }} />

<!-- Backdrop closes on click; Escape + the ✕ button give keyboard access, so the
     static backdrop is presentational. svelte-ignore keeps build output pristine. -->
<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-20 overflow-y-auto bg-[rgba(3,4,7,0.7)] backdrop-blur-[5px]"
  role="presentation"
  onclick={onclose}
>
  <!-- min-h-full + flex centers the panel when it fits, and lets the backdrop
       scroll (top stays visible) when the panel is taller than the viewport. -->
  <div class="flex min-h-full items-center justify-center p-4">
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="relative my-auto w-[calc(100vw-2rem)] max-w-[400px] max-h-[85dvh] overflow-y-auto rounded-sm border border-accent-dim bg-surface shadow-[0_0_0_1px_rgba(62,245,196,0.08),0_24px_70px_-16px_rgba(0,0,0,0.85),0_0_26px_rgba(62,245,196,0.1)]"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <!-- corner ticks -->
    <span class="pointer-events-none absolute -left-px -top-px h-2.5 w-2.5 border-l border-t border-accent"></span>
    <span class="pointer-events-none absolute -bottom-px -right-px h-2.5 w-2.5 border-b border-r border-accent"></span>

    <div class="flex items-center justify-between border-b border-border px-4 py-2.5">
      <h2 class="font-mono text-[13px] font-bold text-ink"><span class="text-accent glow-text">&gt;</span> {title}</h2>
      <button
        onclick={onclose}
        aria-label="Close"
        class="grid h-6 w-6 place-items-center rounded-sm font-mono text-faint transition-colors duration-100 hover:bg-surface-2 hover:text-st-lost"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 6 6 18"/><path d="m6 6 12 12"/>
        </svg>
      </button>
    </div>
    <div class="p-4">
      {@render children()}
    </div>
  </div>
  </div>
</div>

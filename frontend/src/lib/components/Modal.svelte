<script>
  let { title, onclose, children } = $props()
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onclose() }} />

<!-- Backdrop closes on click; Escape + the ✕ button give keyboard access, so the
     static backdrop is presentational. svelte-ignore keeps build output pristine. -->
<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-20 flex items-center justify-center bg-[rgba(4,6,10,0.6)] p-4 backdrop-blur-[4px]"
  role="presentation"
  onclick={onclose}
>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="w-full max-w-[380px] rounded-[10px] border border-border bg-surface p-5 shadow-[0_24px_60px_-12px_rgba(0,0,0,0.7)]"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-[15px] font-semibold tracking-[-0.01em] text-ink">{title}</h2>
      <button
        onclick={onclose}
        aria-label="Close"
        class="grid h-7 w-7 place-items-center rounded-md text-faint transition-colors duration-100 hover:bg-surface-2 hover:text-ink"
      >
        <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 6 6 18"/><path d="m6 6 12 12"/>
        </svg>
      </button>
    </div>
    {@render children()}
  </div>
</div>

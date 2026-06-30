<script>
  let { title, onclose, children } = $props()
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onclose() }} />

<!-- Backdrop closes on click; Escape + the ✕ button give keyboard access, so the
     static backdrop is presentational. svelte-ignore keeps build output pristine. -->
<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="fixed inset-0 z-10 flex items-center justify-center bg-black/30" role="presentation" onclick={onclose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="w-96 rounded-lg bg-white p-5 shadow-xl" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
    <div class="mb-3 flex items-center justify-between">
      <h2 class="font-semibold text-slate-800">{title}</h2>
      <button onclick={onclose} class="text-slate-400 hover:text-slate-700">✕</button>
    </div>
    {@render children()}
  </div>
</div>

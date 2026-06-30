<script>
  import { router } from 'svelte-spa-router'
  // One component, two routes — derive the title from the active path.
  const title = $derived(router.location === '/notes' ? 'Notes' : 'Calendar')
</script>

<header class="rise mb-7 flex items-end justify-between">
  <h1 class="text-[22px] font-semibold tracking-[-0.01em] text-ink">{title}</h1>
</header>

<div class="rise grid min-h-[420px] place-items-center rounded-lg border border-dashed border-border bg-surface" style="animation-delay:40ms">
  <div class="flex flex-col items-center gap-3 text-center">
    <div class="grid h-12 w-12 place-items-center rounded-lg border border-border-strong bg-surface-2 text-accent">
      <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="9"/><path d="M12 8v4l2.5 2.5"/>
      </svg>
    </div>
    <div class="label">Coming soon</div>
    <p class="max-w-xs text-[13px] text-muted">{title} isn't wired up yet. It'll land in a later pass of the build.</p>
  </div>
</div>

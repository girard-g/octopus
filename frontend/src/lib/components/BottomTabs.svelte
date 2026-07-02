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

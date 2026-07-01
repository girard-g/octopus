<script>
  import { link, push, router } from 'svelte-spa-router'
  import { api } from '../api.js'
  import { markLoggedOut } from '../session.svelte.js'

  // Numbered terminal nav. Source labels stay natural-case (e2e matches
  // <a> textContent on 'Contacts'/'Projects'); CSS lowercases for the look.
  const items = [
    { href: '/', n: '01', label: 'Dashboard' },
    { href: '/contacts', n: '02', label: 'Contacts' },
    { href: '/projects', n: '03', label: 'Projects' },
    { href: '/calendar', n: '04', label: 'Calendar' },
    { href: '/notes', n: '05', label: 'Notes' },
  ]

  async function logout() {
    try { await api.post('/api/logout') } catch { /* ignore */ }
    markLoggedOut()
    push('/login')
  }
</script>

<aside class="sticky top-0 flex h-screen w-[220px] shrink-0 flex-col border-r border-border bg-surface">
  <!-- Wordmark: octopus + blinking accent block cursor, faint glow halo. -->
  <div class="relative px-4 pb-5 pt-5">
    <div class="pointer-events-none absolute -left-8 -top-8 h-24 w-24 rounded-full bg-accent/15 blur-2xl"></div>
    <span class="relative select-none font-mono text-[16px] font-bold tracking-tight text-ink">octopus<span class="cursor text-accent glow-text">▋</span></span>
  </div>

  <nav class="flex flex-col gap-px px-2.5">
    {#each items as it}
      {@const active = router.location === it.href}
      <a
        href={it.href}
        use:link
        class="group relative flex items-center gap-2.5 rounded-sm px-2.5 py-2 font-mono text-[13px] transition-colors duration-100"
        class:bg-surface-2={active}
        class:text-accent={active}
        class:glow-text={active}
        class:text-muted={!active}
        class:hover:bg-surface-2={!active}
        class:hover:text-ink={!active}
      >
        {#if active}<span class="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-accent glow-soft"></span>{/if}
        <span class="w-4 shrink-0 text-right text-[11px] tabular-nums {active ? 'text-accent/70' : 'text-faint'}">{it.n}</span>
        <span class="shrink-0 {active ? 'text-accent' : 'text-faint group-hover:text-muted'}">{active ? '>' : ' '}</span>
        <span class="font-medium lowercase">{it.label}</span>
      </a>
    {/each}
  </nav>

  <div class="mt-auto border-t border-border px-3 py-3">
    <div class="flex items-center gap-2">
      <span class="grid h-6 w-6 shrink-0 place-items-center rounded-sm border border-border-strong bg-surface-2 font-mono text-[11px] font-bold text-accent">P</span>
      <span class="min-w-0 flex-1 truncate font-mono text-[12px] text-muted"><span class="text-ink">user</span>@octopus</span>
      <button
        onclick={logout}
        title="Logout"
        class="flex shrink-0 items-center rounded-sm border border-border px-2 py-1 font-mono text-[11px] lowercase text-faint transition-colors duration-100 hover:border-st-lost/50 hover:text-st-lost"
      >Logout</button>
    </div>
  </div>
</aside>

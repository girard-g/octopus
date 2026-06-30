<script>
  import { link, push, router } from 'svelte-spa-router'
  import { api } from '../api.js'
  import { markLoggedOut } from '../session.svelte.js'

  // Feather-style 16px line icons (inner markup; wrapped in a shared <svg> below).
  const items = [
    { href: '/', label: 'Dashboard', icon: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/>' },
    { href: '/contacts', label: 'Contacts', icon: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>' },
    { href: '/pipeline', label: 'Pipeline', icon: '<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/><path d="M15 3v18"/>' },
    { href: '/calendar', label: 'Calendar', icon: '<rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4"/><path d="M8 2v4"/><path d="M3 10h18"/>' },
    { href: '/notes', label: 'Notes', icon: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8"/><path d="M16 17H8"/>' },
  ]

  async function logout() {
    try { await api.post('/api/logout') } catch { /* ignore */ }
    markLoggedOut()
    push('/login')
  }
</script>

<aside class="sticky top-0 flex h-screen w-[216px] shrink-0 flex-col border-r border-border bg-surface">
  <!-- Wordmark with a subtle radial teal glow behind it. -->
  <div class="relative px-4 pb-4 pt-5">
    <div class="pointer-events-none absolute -left-6 -top-6 h-28 w-28 rounded-full bg-accent/20 blur-2xl"></div>
    <span class="relative font-mono text-[15px] font-medium tracking-tight text-ink">
      <span class="text-accent [text-shadow:0_0_12px_rgba(69,214,196,0.55)]">🐙</span> octopus
    </span>
  </div>

  <nav class="flex flex-col gap-0.5 px-2.5 py-2">
    {#each items as it}
      {@const active = router.location === it.href}
      <a
        href={it.href}
        use:link
        class="group relative flex items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] transition-colors duration-100"
        class:bg-surface-2={active}
        class:text-ink={active}
        class:text-muted={!active}
        class:hover:bg-surface-2={!active}
        class:hover:text-ink={!active}
      >
        {#if active}
          <span class="absolute inset-y-1 left-0 w-0.5 rounded-full bg-accent"></span>
        {/if}
        <svg
          viewBox="0 0 24 24" width="16" height="16" fill="none"
          stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"
          class={active ? 'text-accent' : 'text-faint group-hover:text-muted'}
        >{@html it.icon}</svg>
        <span class="font-medium">{it.label}</span>
      </a>
    {/each}
  </nav>

  <div class="mt-auto border-t border-border p-2.5">
    <div class="flex items-center gap-2.5 rounded-md px-2.5 py-2">
      <div class="grid h-7 w-7 shrink-0 place-items-center rounded-full border border-border-strong bg-surface-2 font-mono text-[12px] font-medium text-accent">P</div>
      <div class="min-w-0 leading-tight">
        <div class="truncate text-[12px] font-medium text-ink">parker</div>
        <div class="label !text-[9px]">owner</div>
      </div>
      <button
        onclick={logout}
        title="Logout"
        class="ml-auto flex items-center gap-1.5 rounded-md px-2 py-1.5 text-[12px] text-faint transition-colors duration-100 hover:bg-surface-2 hover:text-ink"
      >
        <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="M16 17l5-5-5-5"/><path d="M21 12H9"/>
        </svg>
        Logout
      </button>
    </div>
  </div>
</aside>

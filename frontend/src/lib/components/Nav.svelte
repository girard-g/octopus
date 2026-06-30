<script>
  import { link, push, router } from 'svelte-spa-router'
  import { api } from '../api.js'
  import { markLoggedOut } from '../session.svelte.js'

  const items = [
    { href: '/', label: 'Dashboard' },
    { href: '/contacts', label: 'Contacts' },
    { href: '/pipeline', label: 'Pipeline' },
  ]

  async function logout() {
    try { await api.post('/api/logout') } catch { /* ignore */ }
    markLoggedOut()
    push('/login')
  }
</script>

<nav class="flex items-center gap-1 border-b bg-white px-4 py-2">
  <span class="mr-4 font-bold text-blue-600">🐙 Octopus</span>
  {#each items as it}
    <a
      href={it.href}
      use:link
      class="rounded px-3 py-1 text-sm hover:bg-slate-100"
      class:bg-slate-200={router.location === it.href}
    >{it.label}</a>
  {/each}
  <button onclick={logout} class="ml-auto rounded px-3 py-1 text-sm text-slate-600 hover:bg-slate-100">
    Logout
  </button>
</nav>

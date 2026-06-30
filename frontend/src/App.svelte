<script>
  import Router, { push, router } from 'svelte-spa-router'
  import Sidebar from './lib/components/Sidebar.svelte'
  import Login from './routes/Login.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import Contacts from './routes/Contacts.svelte'
  import Pipeline from './routes/Pipeline.svelte'
  import Placeholder from './routes/Placeholder.svelte'
  import { getAuthed, markLoggedIn } from './lib/session.svelte.js'
  import { shouldRedirectToLogin, showChrome } from './lib/guard.js'
  import { api } from './lib/api.js'

  const routes = {
    '/': Dashboard,
    '/contacts': Contacts,
    '/pipeline': Pipeline,
    '/calendar': Placeholder,
    '/notes': Placeholder,
    '/login': Login,
  }

  // On first load, probe the session: a successful /api/dashboard means the
  // cookie is still valid; a 401 leaves us logged out and the gate redirects.
  let ready = $state(false)
  $effect(() => {
    api.get('/api/dashboard')
      .then(() => markLoggedIn())
      .catch(() => {})
      .finally(() => { ready = true })
  })

  // Auth gate: pure decision in guard.js (unit-tested), applied here.
  $effect(() => {
    if (shouldRedirectToLogin(ready, getAuthed(), router.location)) push('/login')
  })
</script>

{#if !ready}
  <div class="grid min-h-screen place-items-center font-mono text-[13px] text-faint">Loading…</div>
{:else if showChrome(getAuthed(), router.location)}
  <div class="flex min-h-screen">
    <Sidebar />
    <main class="min-w-0 flex-1 px-8 py-7">
      <Router {routes} />
    </main>
  </div>
{:else}
  <Router {routes} />
{/if}

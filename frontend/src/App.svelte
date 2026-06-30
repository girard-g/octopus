<script>
  import Router, { push, router } from 'svelte-spa-router'
  import Nav from './lib/components/Nav.svelte'
  import Login from './routes/Login.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import Contacts from './routes/Contacts.svelte'
  import Pipeline from './routes/Pipeline.svelte'
  import { getAuthed, markLoggedIn } from './lib/session.svelte.js'
  import { shouldRedirectToLogin, showChrome } from './lib/guard.js'
  import { api } from './lib/api.js'

  const routes = {
    '/': Dashboard,
    '/contacts': Contacts,
    '/pipeline': Pipeline,
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
  <div class="p-6 text-slate-500">Loading…</div>
{:else}
  {#if showChrome(getAuthed(), router.location)}
    <Nav />
  {/if}
  <main class="mx-auto max-w-5xl p-4">
    <Router {routes} />
  </main>
{/if}

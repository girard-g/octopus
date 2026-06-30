<script>
  import Router, { push, router } from 'svelte-spa-router'
  import Sidebar from './lib/components/Sidebar.svelte'
  import Login from './routes/Login.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import Contacts from './routes/Contacts.svelte'
  import Pipeline from './routes/Pipeline.svelte'
  import Placeholder from './routes/Placeholder.svelte'
  import ProjectBoard from './routes/ProjectBoard.svelte'
  import { getAuthed, markLoggedIn } from './lib/session.svelte.js'
  import { shouldRedirectToLogin, showChrome } from './lib/guard.js'
  import { api } from './lib/api.js'

  const routes = {
    '/': Dashboard,
    '/contacts': Contacts,
    '/pipeline': Pipeline,
    '/calendar': Placeholder,
    '/notes': Placeholder,
    '/projects/:id': ProjectBoard,
    '/login': Login,
  }

  // Natural-case page titles (sr-only twin keeps innerText matchable for e2e
  // even though the visible title is lowercased for the terminal look).
  const TITLES = {
    '/': 'Dashboard', '/contacts': 'Contacts', '/pipeline': 'Pipeline',
    '/calendar': 'Calendar', '/notes': 'Notes',
  }
  const title = $derived(
    router.location.startsWith('/projects/') ? 'project' : (TITLES[router.location] ?? 'Dashboard')
  )

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
  <div class="grid min-h-screen place-items-center font-mono text-[13px] text-faint"><span class="text-accent glow-text">&gt;</span>&nbsp;booting<span class="cursor">▋</span></div>
{:else if showChrome(getAuthed(), router.location)}
  <div class="flex min-h-screen">
    <Sidebar />
    <main class="min-w-0 flex-1">
      <div class="boot">
        <!-- Top bar: prompt-style page title + command-bar cue. -->
        <header class="sticky top-0 z-10 flex h-13 items-center gap-4 border-b border-border bg-bg/85 px-8 py-3 backdrop-blur">
          <h1 class="font-mono text-[14px] font-bold tracking-tight">
            <span class="text-accent glow-text">&gt;</span>
            <span class="lowercase text-ink">{title}</span>
            <span class="sr-only">{title}</span>
          </h1>
          <div
            class="ml-auto flex items-center gap-2 rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 font-mono text-[12px] text-faint"
            aria-hidden="true"
          >
            <span class="text-accent-dim">/</span>
            <span>search…</span>
            <kbd class="ml-2 rounded-sm border border-border-strong bg-bg px-1.5 py-0.5 text-[11px] text-muted">⌘K</kbd>
          </div>
        </header>
        <div class="px-8 py-7">
          <Router {routes} />
        </div>
      </div>
    </main>
  </div>
{:else}
  <Router {routes} />
{/if}

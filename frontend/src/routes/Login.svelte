<script>
  import { push } from 'svelte-spa-router'
  import { api, ApiError } from '../lib/api.js'
  import { markLoggedIn } from '../lib/session.svelte.js'

  let password = $state('')
  let error = $state('')
  let busy = $state(false)

  async function submit(e) {
    e.preventDefault()
    error = ''
    busy = true
    try {
      await api.post('/api/login', { password })
      markLoggedIn()
      push('/')
    } catch (err) {
      error = err instanceof ApiError && err.status === 401
        ? 'Wrong password.'
        : 'Login failed.'
    } finally {
      busy = false
    }
  }
</script>

<div class="relative grid min-h-screen place-items-center overflow-hidden px-4">
  <!-- Ambient mint glow + faint grid already on body. -->
  <div class="pointer-events-none absolute left-1/2 top-[38%] h-[460px] w-[460px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent/8 blur-[130px]"></div>

  <form onsubmit={submit} class="rise relative w-full max-w-[360px] rounded-sm border border-border-strong bg-surface shadow-[0_24px_70px_-16px_rgba(0,0,0,0.85)]">
    <!-- corner ticks -->
    <span class="pointer-events-none absolute -left-px -top-px h-2.5 w-2.5 border-l border-t border-accent"></span>
    <span class="pointer-events-none absolute -right-px -top-px h-2.5 w-2.5 border-r border-t border-accent"></span>
    <span class="pointer-events-none absolute -bottom-px -left-px h-2.5 w-2.5 border-b border-l border-accent"></span>
    <span class="pointer-events-none absolute -bottom-px -right-px h-2.5 w-2.5 border-b border-r border-accent"></span>

    <!-- title bar -->
    <div class="flex items-center gap-2 border-b border-border px-4 py-2.5">
      <span class="h-2 w-2 rounded-full bg-st-lost/70"></span>
      <span class="h-2 w-2 rounded-full bg-st-proposal/70"></span>
      <span class="h-2 w-2 rounded-full bg-st-done/70"></span>
      <span class="ml-2 font-mono text-[12px] font-bold tracking-tight text-ink">octopus<span class="cursor text-accent glow-text">▋</span></span>
    </div>

    <div class="p-5">
      <p class="font-mono text-[12px] text-muted"><span class="text-accent glow-text">&gt;</span> authenticate --session</p>
      <p class="mt-1 font-mono text-[12px] text-faint">enter access key to continue</p>

      <p class="label mb-1.5 mt-5">Password</p>
      <div class="flex items-center gap-2 rounded-sm border border-border bg-surface-2 px-2.5 transition-colors duration-100 focus-within:border-accent focus-within:shadow-[0_0_0_3px_rgba(62,245,196,0.14)]">
        <span class="select-none font-mono text-[13px] text-accent-dim">$</span>
        <input
          type="password"
          bind:value={password}
          placeholder="••••••••"
          autocomplete="current-password"
          class="h-9 w-full bg-transparent font-mono text-[13px] text-ink placeholder:text-faint focus:outline-none"
        />
      </div>

      {#if error}<p class="mt-3 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

      <button
        type="submit"
        disabled={busy}
        class="mt-5 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50"
      >
        {busy ? 'Signing in…' : 'Sign in'}
      </button>
    </div>
  </form>
</div>

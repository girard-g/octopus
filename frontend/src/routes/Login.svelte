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
  <!-- Ambient ink glow on the dark canvas. -->
  <div class="pointer-events-none absolute left-1/2 top-[34%] h-[420px] w-[420px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent/10 blur-[120px]"></div>

  <form onsubmit={submit} class="rise relative w-full max-w-[340px] rounded-[10px] border border-border bg-surface p-6 shadow-[0_24px_60px_-12px_rgba(0,0,0,0.7)]">
    <div class="mb-5 flex items-center gap-2">
      <span class="font-mono text-[17px] font-medium tracking-tight text-ink">
        <span class="text-accent [text-shadow:0_0_12px_rgba(69,214,196,0.55)]">🐙</span> octopus
      </span>
    </div>
    <p class="label mb-1.5">Password</p>
    <input
      type="password"
      bind:value={password}
      placeholder="Enter password"
      autocomplete="current-password"
      class="h-9 w-full rounded-md border border-border bg-surface-2 px-3 text-[13px] text-ink placeholder:text-faint transition-colors duration-100 focus:border-accent focus:shadow-[0_0_0_3px_rgba(69,214,196,0.12)] focus:outline-none"
    />
    {#if error}<p class="mt-3 font-mono text-[12px] text-st-lost">{error}</p>{/if}
    <button
      type="submit"
      disabled={busy}
      class="mt-4 h-9 w-full rounded-md bg-accent text-[13px] font-medium text-on-accent transition hover:brightness-110 disabled:opacity-50"
    >
      {busy ? 'Signing in…' : 'Sign in'}
    </button>
  </form>
</div>

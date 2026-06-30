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

<div class="flex min-h-screen items-center justify-center bg-slate-100">
  <form onsubmit={submit} class="w-80 rounded-lg bg-white p-6 shadow">
    <h1 class="mb-4 text-xl font-bold text-slate-800">Octopus</h1>
    <input
      type="password"
      bind:value={password}
      placeholder="Password"
      autocomplete="current-password"
      class="mb-3 w-full rounded border border-slate-300 px-3 py-2"
    />
    {#if error}<p class="mb-3 text-sm text-red-600">{error}</p>{/if}
    <button
      type="submit"
      disabled={busy}
      class="w-full rounded bg-blue-600 py-2 font-medium text-white hover:bg-blue-700 disabled:opacity-50"
    >
      {busy ? 'Signing in…' : 'Sign in'}
    </button>
  </form>
</div>

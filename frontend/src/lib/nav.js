import { push } from 'svelte-spa-router'
import { api } from './api.js'
import { markLoggedOut } from './session.svelte.js'

// Single source of truth for navigation. `label` stays natural-case (e2e matches
// <a> textContent); CSS lowercases for the terminal look. `short` is the compact
// label used on the mobile bottom tab bar. `n` is the desktop sidebar numbering.
export const NAV_ITEMS = [
  { href: '/', n: '01', label: 'Dashboard', short: 'dash' },
  { href: '/contacts', n: '02', label: 'Contacts', short: 'cont' },
  { href: '/projects', n: '03', label: 'Projects', short: 'proj' },
  { href: '/calendar', n: '04', label: 'Calendar', short: 'cal' },
  { href: '/notes', n: '05', label: 'Notes', short: 'note' },
]

// Active when the location equals the href, or (for non-root sections) is a
// sub-route of it — so a project board highlights the Projects tab.
export function isActive(location, href) {
  if (href === '/') return location === '/'
  return location === href || location.startsWith(href + '/')
}

export async function logout() {
  try { await api.post('/api/logout') } catch { /* ignore */ }
  markLoggedOut()
  push('/login')
}

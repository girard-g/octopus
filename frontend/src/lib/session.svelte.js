// Tiny auth flag shared across the app. Runes work in .svelte.js modules.
let authed = $state(false)

export function getAuthed() { return authed }
export function markLoggedIn() { authed = true }
export function markLoggedOut() { authed = false }

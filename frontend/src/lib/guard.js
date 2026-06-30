// Pure decision for the auth gate — no runes, no DOM, trivially unit-testable.
// Redirect to /login only once the session probe finished, we're not authed,
// and we're not already on the login route.
export function shouldRedirectToLogin(ready, authed, location) {
  return ready && !authed && location !== '/login'
}

// Show the app chrome (nav) only when authed and off the login route.
export function showChrome(authed, location) {
  return authed && location !== '/login'
}

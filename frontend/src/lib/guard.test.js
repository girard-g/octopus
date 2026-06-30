import { describe, it, expect } from 'vitest'
import { shouldRedirectToLogin, showChrome } from './guard.js'

describe('auth gate', () => {
  it('does not redirect before the session probe completes', () => {
    expect(shouldRedirectToLogin(false, false, '/')).toBe(false)
  })
  it('redirects an unauthed user off a protected route', () => {
    expect(shouldRedirectToLogin(true, false, '/')).toBe(true)
  })
  it('does not redirect when already on /login', () => {
    expect(shouldRedirectToLogin(true, false, '/login')).toBe(false)
  })
  it('does not redirect an authed user', () => {
    expect(shouldRedirectToLogin(true, true, '/')).toBe(false)
  })
  it('shows chrome only when authed and off login', () => {
    expect(showChrome(true, '/')).toBe(true)
    expect(showChrome(true, '/login')).toBe(false)
    expect(showChrome(false, '/')).toBe(false)
  })
})

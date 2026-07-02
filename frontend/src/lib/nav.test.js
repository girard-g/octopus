import { describe, it, expect } from 'vitest'
import { NAV_ITEMS, isActive } from './nav.js'

describe('NAV_ITEMS', () => {
  it('has the six routes with href + short label', () => {
    expect(NAV_ITEMS.map((i) => i.href)).toEqual(['/', '/contacts', '/projects', '/calendar', '/notes', '/links'])
    for (const i of NAV_ITEMS) {
      expect(typeof i.short).toBe('string')
      expect(i.short.length).toBeGreaterThan(0)
      expect(i.label[0]).toBe(i.label[0].toUpperCase()) // natural-case for e2e
    }
  })
})

describe('isActive', () => {
  it('matches dashboard only exactly', () => {
    expect(isActive('/', '/')).toBe(true)
    expect(isActive('/projects', '/')).toBe(false)
  })
  it('matches a section and its detail sub-routes', () => {
    expect(isActive('/projects', '/projects')).toBe(true)
    expect(isActive('/projects/abc123', '/projects')).toBe(true)
    expect(isActive('/contacts/xy', '/contacts')).toBe(true)
  })
  it('does not cross-match sections', () => {
    expect(isActive('/contacts', '/projects')).toBe(false)
  })
})

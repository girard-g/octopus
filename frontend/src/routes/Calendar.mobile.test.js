import { describe, it, expect } from 'vitest'
import { dayCellAction } from '../lib/calendar.js'

describe('dayCellAction', () => {
  it('drills into day view on mobile', () => {
    expect(dayCellAction(true)).toBe('day')
  })
  it('creates a new event on desktop', () => {
    expect(dayCellAction(false)).toBe('new')
  })
})

import { describe, it, expect } from 'vitest'
import { renderMarkdown } from './markdown.js'

describe('renderMarkdown', () => {
  it('renders headings, lists, and code fences', () => {
    const html = renderMarkdown('# Title\n\n- a\n- b\n\n```\ncode\n```')
    expect(html).toContain('<h1>')
    expect(html).toContain('<li>a</li>')
    expect(html).toContain('<code>')
  })
  it('is safe on empty/null input', () => {
    expect(renderMarkdown('')).toBe('')
    expect(renderMarkdown(null)).toBe('')
  })
})

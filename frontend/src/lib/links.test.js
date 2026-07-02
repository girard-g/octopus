import { describe, it, expect } from 'vitest'
import { linkHost, faviconUrl, parseTags, allTags, filterLinks, groupByCategory } from './links.js'

describe('links helpers', () => {
  it('extracts host, empty on garbage', () => {
    expect(linkHost('https://rust-lang.org/learn')).toBe('rust-lang.org')
    expect(linkHost('not a url')).toBe('')
  })

  it('builds duckduckgo favicon url', () => {
    expect(faviconUrl('https://rust-lang.org/x')).toBe('https://icons.duckduckgo.com/ip3/rust-lang.org.ico')
    expect(faviconUrl('nope')).toBe('')
  })

  it('parses tags: split, trim, drop empty, dedupe first-wins', () => {
    expect(parseTags(' rust,  free , rust ,')).toEqual(['rust', 'free'])
    expect(parseTags('')).toEqual([])
  })

  it('collects sorted unique tags', () => {
    expect(allTags([{ tags: ['b', 'a'] }, { tags: ['a', 'c'] }, { tags: null }])).toEqual(['a', 'b', 'c'])
  })

  it('filters by query and requires all active tags', () => {
    const links = [
      { title: 'Rust', url: 'https://rust-lang.org', description: null, tags: ['lang', 'free'] },
      { title: 'Svelte', url: 'https://svelte.dev', description: 'ui', tags: ['ui'] },
    ]
    expect(filterLinks(links, 'rust', []).map((l) => l.title)).toEqual(['Rust'])
    expect(filterLinks(links, '', ['ui']).map((l) => l.title)).toEqual(['Svelte'])
    expect(filterLinks(links, '', ['lang', 'free']).map((l) => l.title)).toEqual(['Rust'])
    expect(filterLinks(links, '', ['lang', 'missing'])).toEqual([])
  })

  it('groups by category, uncategorized last', () => {
    const groups = groupByCategory([
      { id: 1, category: 'Rust' },
      { id: 2, category: null },
      { id: 3, category: 'Design' },
    ])
    expect(groups.map((g) => g.category)).toEqual(['Design', 'Rust', ''])
    expect(groups[2].links.map((l) => l.id)).toEqual([2])
  })
})

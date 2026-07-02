export function linkHost(url) {
  try { return new URL(url).hostname } catch { return '' }
}

export function faviconUrl(url) {
  const host = linkHost(url)
  return host ? `https://icons.duckduckgo.com/ip3/${host}.ico` : ''
}

export function parseTags(str) {
  const seen = new Set()
  return String(str)
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t !== '' && !seen.has(t) && seen.add(t))
}

export function allTags(links) {
  const s = new Set()
  for (const l of links) for (const t of l.tags || []) s.add(t)
  return [...s].sort()
}

export function filterLinks(links, query, activeTags) {
  const q = query.trim().toLowerCase()
  return links.filter((l) => {
    const matchesQuery =
      !q ||
      (l.title || '').toLowerCase().includes(q) ||
      (l.url || '').toLowerCase().includes(q) ||
      (l.description || '').toLowerCase().includes(q)
    const tags = l.tags || []
    const matchesTags = activeTags.every((t) => tags.includes(t))
    return matchesQuery && matchesTags
  })
}

export function groupByCategory(links) {
  const groups = new Map()
  for (const l of links) {
    const key = l.category || ''
    if (!groups.has(key)) groups.set(key, [])
    groups.get(key).push(l)
  }
  const named = [...groups.keys()].filter((k) => k).sort()
  const result = named.map((k) => ({ category: k, links: groups.get(k) }))
  if (groups.has('')) result.push({ category: '', links: groups.get('') })
  return result
}

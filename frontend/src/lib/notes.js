export function noteTitle(note) {
  const t = (note.title || '').trim()
  if (t) return t
  const line = (note.body || '').split('\n').map((l) => l.trim()).find((l) => l !== '')
  return line || 'untitled'
}

export function buildFolderTree(folders) {
  const byId = new Map(folders.map((f) => [f.id, { ...f, children: [] }]))
  const roots = []
  for (const node of byId.values()) {
    if (node.parent_id && byId.has(node.parent_id)) byId.get(node.parent_id).children.push(node)
    else roots.push(node)
  }
  const cmp = (a, b) => (a.position - b.position) || a.name.localeCompare(b.name)
  const sortRec = (nodes) => { nodes.sort(cmp); for (const n of nodes) sortRec(n.children) }
  sortRec(roots)
  return roots
}

export function sortNotes(notes) {
  return [...notes].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1
    return String(b.updated_at).localeCompare(String(a.updated_at))
  })
}

export function searchNotes(notes, q) {
  const term = (q || '').trim().toLowerCase()
  if (!term) return []
  return notes.filter(
    (n) => (n.title || '').toLowerCase().includes(term) || (n.body || '').toLowerCase().includes(term),
  )
}

export function folderPath(folders, id) {
  if (!id) return ''
  const byId = new Map(folders.map((f) => [f.id, f]))
  const parts = []
  let cur = byId.get(id)
  const guard = new Set()
  while (cur && !guard.has(cur.id)) {
    guard.add(cur.id)
    parts.unshift(cur.name)
    cur = cur.parent_id ? byId.get(cur.parent_id) : null
  }
  return parts.join(' / ')
}

// inclusive set of a folder's own id + all descendant folder ids (walk children via parent_id)
export function folderSubtreeIds(folders, id) {
  const childrenOf = new Map()
  for (const f of folders) {
    if (!childrenOf.has(f.parent_id)) childrenOf.set(f.parent_id, [])
    childrenOf.get(f.parent_id).push(f.id)
  }
  const set = new Set([id])
  const stack = [id]
  while (stack.length) {
    const cur = stack.pop()
    for (const c of childrenOf.get(cur) || []) { if (!set.has(c)) { set.add(c); stack.push(c) } }
  }
  return set
}

export function folderBlastRadius(folders, notes, id) {
  const subtree = folderSubtreeIds(folders, id)
  return {
    subfolders: subtree.size - 1,
    notes: notes.filter((n) => subtree.has(n.folder_id)).length,
  }
}

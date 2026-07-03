import { describe, it, expect } from 'vitest'
import { noteTitle, buildFolderTree, sortNotes, searchNotes, folderPath, folderBlastRadius } from './notes.js'

describe('noteTitle', () => {
  it('prefers explicit title', () => {
    expect(noteTitle({ title: 'Hi', body: 'x' })).toBe('Hi')
  })
  it('derives from first non-empty body line', () => {
    expect(noteTitle({ title: null, body: '\n\n# Kickoff\nmore' })).toBe('# Kickoff')
  })
  it('falls back to untitled', () => {
    expect(noteTitle({ title: '  ', body: '  ' })).toBe('untitled')
  })
})

describe('buildFolderTree', () => {
  it('nests children under parents, sorted', () => {
    const tree = buildFolderTree([
      { id: 'a', name: 'clients', parent_id: null, position: 0 },
      { id: 'b', name: 'acme', parent_id: 'a', position: 1 },
      { id: 'c', name: 'admin', parent_id: null, position: 0 },
    ])
    expect(tree.map((n) => n.name)).toEqual(['admin', 'clients']) // position tie -> name
    const clients = tree.find((n) => n.id === 'a')
    expect(clients.children.map((n) => n.name)).toEqual(['acme'])
  })
})

describe('sortNotes', () => {
  it('pins first then newest', () => {
    const out = sortNotes([
      { id: '1', pinned: false, updated_at: '2026-01-01' },
      { id: '2', pinned: true, updated_at: '2026-01-01' },
      { id: '3', pinned: false, updated_at: '2026-02-01' },
    ])
    expect(out.map((n) => n.id)).toEqual(['2', '3', '1'])
  })
})

describe('searchNotes', () => {
  it('matches title or body, case-insensitive', () => {
    const notes = [{ id: '1', title: 'Kickoff', body: '' }, { id: '2', title: null, body: 'agenda' }]
    expect(searchNotes(notes, 'kick').map((n) => n.id)).toEqual(['1'])
    expect(searchNotes(notes, 'AGENDA').map((n) => n.id)).toEqual(['2'])
    expect(searchNotes(notes, '   ')).toEqual([])
  })
})

describe('folderPath', () => {
  it('builds a breadcrumb', () => {
    const folders = [
      { id: 'a', name: 'clients', parent_id: null },
      { id: 'b', name: 'acme', parent_id: 'a' },
    ]
    expect(folderPath(folders, 'b')).toBe('clients / acme')
    expect(folderPath(folders, null)).toBe('')
  })
})

describe('folderBlastRadius', () => {
  it('counts descendant folders and subtree notes', () => {
    const folders = [
      { id: 'a', name: 'root', parent_id: null },
      { id: 'b', name: 'child', parent_id: 'a' },
      { id: 'c', name: 'grand', parent_id: 'b' },
    ]
    const notes = [
      { id: 'n1', folder_id: 'a' },
      { id: 'n2', folder_id: 'c' },
      { id: 'n3', folder_id: null },
    ]
    expect(folderBlastRadius(folders, notes, 'a')).toEqual({ subfolders: 2, notes: 2 })
  })
})

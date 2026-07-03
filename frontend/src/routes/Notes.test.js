import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, fireEvent, waitFor, cleanup } from '@testing-library/svelte'
import Notes from './Notes.svelte'

const noteB = { id: 'b', title: 'Note B', body: 'existing body', folder_id: null, contact_id: null, project_id: null, pinned: false }

vi.mock('../lib/api.js', () => ({
  api: {
    get: vi.fn((path) => Promise.resolve(path === '/api/notes' ? [noteB] : [])),
    post: vi.fn(),
    put: vi.fn(() => Promise.resolve({ ...noteB })),
    patch: vi.fn(),
    del: vi.fn(() => Promise.resolve(null)),
  },
}))

const { api } = await import('../lib/api.js')

beforeEach(() => { vi.clearAllMocks() })
afterEach(() => cleanup())

describe('Notes autosave', () => {
  it('never POSTs an abandoned empty draft', async () => {
    const { getByText } = render(Notes)
    await waitFor(() => expect(api.get).toHaveBeenCalled())

    await fireEvent.click(getByText('+ note'))
    await new Promise((r) => setTimeout(r, 700)) // past the 600ms debounce

    expect(api.post).not.toHaveBeenCalled()
  })

  it('does not let a create-in-flight race overwrite another note (switch-during-create)', async () => {
    let resolvePost
    api.post.mockImplementation(() => new Promise((r) => { resolvePost = r }))

    const { getByText, getByPlaceholderText } = render(Notes)
    await waitFor(() => expect(api.get).toHaveBeenCalled())

    // Start a brand-new note A and give it content so autosave schedules a create.
    await fireEvent.click(getByText('+ note'))
    const title = getByPlaceholderText('title…')
    await fireEvent.input(title, { target: { value: 'Note A' } })
    await fireEvent.blur(title) // triggers persist() immediately -> POST in flight, unresolved

    await waitFor(() => expect(api.post).toHaveBeenCalledTimes(1))

    // While the create is still in flight, switch to note B.
    await fireEvent.click(getByText('Note B'))
    await waitFor(() => expect(getByPlaceholderText('title…').value).toBe('Note B'))

    // Now the original POST for note A resolves — this is where the bug used to
    // mutate whichever object `draft` pointed at *now* (note B), stamping B with A's id.
    resolvePost({ id: 'a-server-id', title: 'Note A', body: '', folder_id: null, contact_id: null, project_id: null, pinned: false })
    await new Promise((r) => setTimeout(r, 0))

    // Edit B and let it autosave: if B.id got corrupted to A's id, this PUTs the wrong URL.
    const titleAfterSwitch = getByPlaceholderText('title…')
    await fireEvent.input(titleAfterSwitch, { target: { value: 'Note B edited' } })
    await fireEvent.blur(titleAfterSwitch)
    await waitFor(() => expect(api.put).toHaveBeenCalled())

    expect(api.put).toHaveBeenCalledWith('/api/notes/b', expect.anything())
    for (const call of api.put.mock.calls) {
      expect(call[0]).not.toBe('/api/notes/a-server-id')
    }
  })

  it('does not drop text typed during an in-flight create (fix 4)', async () => {
    let resolvePost
    api.post.mockImplementation(() => new Promise((r) => { resolvePost = r }))

    const { getByText, getByPlaceholderText } = render(Notes)
    await waitFor(() => expect(api.get).toHaveBeenCalled())

    await fireEvent.click(getByText('+ note'))
    const title = getByPlaceholderText('title…')
    await fireEvent.input(title, { target: { value: 'Draft' } })
    await fireEvent.blur(title) // persist now → POST in flight, unresolved
    await waitFor(() => expect(api.post).toHaveBeenCalledTimes(1))

    // Type more while the create round-trips: the scheduled save must reschedule, not vanish.
    await fireEvent.input(title, { target: { value: 'Draft plus more' } })
    await new Promise((r) => setTimeout(r, 700)) // let the scheduled save FIRE while the create is still in flight → must reschedule, not drop

    // Resolve the create; the rescheduled save must then PUT the *latest* text.
    resolvePost({ id: 'new-id', title: 'Draft', body: '', folder_id: null, contact_id: null, project_id: null, pinned: false })

    await waitFor(
      () => expect(api.put).toHaveBeenCalledWith('/api/notes/new-id', expect.objectContaining({ title: 'Draft plus more' })),
      { timeout: 3000 },
    )
  })

  it('discards a brand-new note deleted mid-create (fix 5)', async () => {
    let resolvePost
    api.post.mockImplementation(() => new Promise((r) => { resolvePost = r }))

    const { getByText, queryByText, getByPlaceholderText } = render(Notes)
    await waitFor(() => expect(api.get).toHaveBeenCalled())

    await fireEvent.click(getByText('+ note'))
    const title = getByPlaceholderText('title…')
    await fireEvent.input(title, { target: { value: 'Ghost' } })
    await fireEvent.blur(title) // POST in flight
    await waitFor(() => expect(api.post).toHaveBeenCalledTimes(1))

    // Delete the note while its create is still in flight.
    await fireEvent.click(getByText('delete'))

    // Create resolves — it must be undone (api.del), not merged back into the list.
    resolvePost({ id: 'ghost-id', title: 'Ghost', body: '', folder_id: null, contact_id: null, project_id: null, pinned: false })

    await waitFor(() => expect(api.del).toHaveBeenCalledWith('/api/notes/ghost-id'), { timeout: 3000 })
    expect(queryByText('Ghost')).toBeNull() // never resurrected into the sidebar
  })
})

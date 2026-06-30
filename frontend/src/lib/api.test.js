import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api, ApiError } from './api.js'

function mockFetch(status, payload) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    status,
    ok: status >= 200 && status < 300,
    text: async () => (payload === undefined ? '' : JSON.stringify(payload)),
  })
}

describe('api wrapper', () => {
  beforeEach(() => { vi.restoreAllMocks() })

  it('returns parsed JSON on 200', async () => {
    mockFetch(200, { name: 'Acme' })
    expect(await api.get('/api/contacts/1')).toEqual({ name: 'Acme' })
  })

  it('returns null on 204', async () => {
    mockFetch(204)
    expect(await api.post('/api/login', { password: 'x' })).toBeNull()
  })

  it('throws ApiError with server message on 400', async () => {
    mockFetch(400, { error: 'name is required' })
    await expect(api.post('/api/contacts', {})).rejects.toMatchObject({
      status: 400, message: 'name is required',
    })
  })

  it('throws ApiError(401) on unauthorized', async () => {
    mockFetch(401, { error: 'unauthorized' })
    await expect(api.get('/api/dashboard')).rejects.toBeInstanceOf(ApiError)
  })

  it('sends credentials and JSON content-type on POST', async () => {
    mockFetch(201, { id: '1' })
    await api.post('/api/contacts', { name: 'A' })
    const [, opts] = globalThis.fetch.mock.calls[0]
    expect(opts.credentials).toBe('include')
    expect(opts.headers['content-type']).toBe('application/json')
  })
})

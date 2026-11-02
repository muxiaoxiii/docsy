import { describe, expect, it, vi } from 'vitest'
import { createPdfPreviewCache } from './pdfPreviewCache.js'

function setup(options = {}) {
  const statFile = vi.fn().mockResolvedValue({ size: 100, mtime: new Date(1) })
  const renderPage = vi.fn(async (args) => ({ ...args, imageDataUrl: `image-${args.page}-${args.dpi}` }))
  return { statFile, renderPage, ...createPdfPreviewCache({ statFile, renderPage, ...options }) }
}
const args = { inputPath: '/doc.pdf', page: 32, dpi: 115 }
describe('versioned bounded PDF page cache', () => {
  it('reuses a page across views and merges simultaneous rendering requests', async () => {
    const cache = setup()
    const [a, b] = await Promise.all([cache.load(args), cache.load(args)])
    expect(a).toBe(b)
    expect(await cache.load(args)).toBe(a)
    expect(cache.renderPage).toHaveBeenCalledTimes(1)
    await cache.load({ ...args, dpi: 72 })
    expect(cache.renderPage).toHaveBeenCalledTimes(2)
  })
  it('invalidates after external save or explicit refresh, even when the size is unchanged', async () => {
    const cache = setup()
    await cache.load(args)
    cache.statFile.mockResolvedValue({ size: 100, mtime: new Date(2) })
    await cache.load(args)
    await cache.load({ ...args, forceRefresh: true })
    expect(cache.renderPage).toHaveBeenCalledTimes(3)
    expect(cache.renderPage.mock.calls[2][0].forceRefresh).toBe(true)
  })
  it('does not return a rendering if the source changes during rendering', async () => {
    const cache = setup()
    cache.statFile
      .mockResolvedValueOnce({ size: 100, mtime: new Date(1) })
      .mockResolvedValue({ size: 101, mtime: new Date(2) })
    await cache.load(args)
    expect(cache.renderPage).toHaveBeenCalledTimes(2)
  })
  it('preserves explicit refresh when retrying a changed file', async () => {
    const cache = setup()
    cache.statFile.mockResolvedValueOnce({ size: 100, mtime: new Date(1) }).mockResolvedValue({ size: 101, mtime: new Date(2) })
    await cache.load({ ...args, forceRefresh: true })
    expect(cache.renderPage).toHaveBeenCalledTimes(2)
    expect(cache.renderPage.mock.calls[1][0].forceRefresh).toBe(true)
  })
  it('evicts least recently used pages and oversized images', async () => {
    const cache = setup({ maxEntries: 2 })
    for (const page of [1, 2, 1, 3, 2]) await cache.load({ ...args, page })
    expect(cache.renderPage).toHaveBeenCalledTimes(4)
    const tiny = setup({ maxBytes: 1 })
    await tiny.load(args)
    await tiny.load(args)
    expect(tiny.renderPage).toHaveBeenCalledTimes(2)
  })
  it('renders without caching when metadata is inaccessible and never caches failures', async () => {
    const cache = setup()
    cache.statFile.mockRejectedValue(new Error('no stat'))
    await cache.load(args)
    await cache.load(args)
    expect(cache.renderPage).toHaveBeenCalledTimes(2)
    cache.renderPage.mockRejectedValueOnce(new Error('render failed'))
    await expect(cache.load(args)).rejects.toThrow('render failed')
    await cache.load(args)
    expect(cache.renderPage).toHaveBeenCalledTimes(4)
  })
})

it('keeps an older in-flight request from replacing an explicitly refreshed page', async () => {
  const cache = setup()
  let finishOld
  cache.renderPage.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finishOld = resolve
      }),
  )
  const old = cache.load(args)
  await vi.waitFor(() => expect(finishOld).toBeTypeOf('function'))
  const fresh = await cache.load({ ...args, forceRefresh: true })
  finishOld({ page: args.page, imageDataUrl: 'old-image' })
  await old
  expect(await cache.load(args)).toBe(fresh)
  expect(cache.renderPage).toHaveBeenCalledTimes(2)
})

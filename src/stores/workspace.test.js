import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useWorkspaceStore } from './workspace.js'

describe('workspace session persistence', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('草稿写入失败时不会清除其他会话数据', () => {
    const values = new Map([['docsy.workspace.media-transfer', JSON.stringify({ paths: ['keep.png'] })]])
    let failOnce = true
    const storage = {
      getItem: (key) => values.get(key) || null,
      setItem: (key, value) => {
        if (failOnce) {
          failOnce = false
          throw new Error('quota exceeded')
        }
        values.set(key, value)
      },
      removeItem: (key) => values.delete(key),
      clear: vi.fn(() => values.clear()),
    }
    vi.stubGlobal('window', { sessionStorage: storage })
    vi.spyOn(console, 'warn').mockImplementation(() => {})

    useWorkspaceStore().setFrameSelectionDraft({ items: [{ path: 'frame.png' }] })

    expect(storage.clear).not.toHaveBeenCalled()
    expect(values.has('docsy.workspace.media-transfer')).toBe(true)
    expect(values.has('docsy.workspace.frame-selection-draft')).toBe(true)
  })
})

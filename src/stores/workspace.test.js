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
  it('待导入文档保留完整路径，旧确认不能清除新任务', () => {
    const values = new Map()
    vi.stubGlobal('window', { sessionStorage: { getItem: key => values.get(key) || null, setItem: (key, value) => values.set(key, value), removeItem: key => values.delete(key) } })
    const store = useWorkspaceStore()
    const first = store.sendFiles(['/输出/裁剪 1.pdf'], 'pdf-merge', '裁剪')
    const next = store.sendFiles(['/输出/裁剪 2.pdf'], 'evidence-merge', '裁剪')
    store.clearFileTransfer(first.id)
    expect(store.fileTransfer.id).toBe(next.id)
    setActivePinia(createPinia())
    expect(useWorkspaceStore().fileTransfer.paths).toEqual(['/输出/裁剪 2.pdf'])
  })
})

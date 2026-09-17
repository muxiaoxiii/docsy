import { afterEach, describe, expect, it, vi } from 'vitest'

const hooks = vi.hoisted(() => ({ mounted: null, handler: null, element: null }))
vi.mock('vue', () => ({
  getCurrentInstance: () => ({ proxy: { get $el() { return hooks.element } } }),
  onMounted: callback => { hooks.mounted = callback },
  onActivated: vi.fn(), onDeactivated: vi.fn(), onBeforeUnmount: vi.fn(),
}))
vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => ({
  onDragDropEvent: async callback => { hooks.handler = callback; return vi.fn() },
}) }))
import { useWindowFileDrop } from './useWindowFileDrop.js'

describe('窗口拖拽隔离', () => {
  afterEach(() => vi.unstubAllGlobals())
  it('隐藏标签页不接收拖入文件，显示后才接收', async () => {
    class FakeElement { getClientRects() { return this.visible ? [{}] : [] } }
    vi.stubGlobal('Element', FakeElement)
    hooks.element = new FakeElement()
    const onDrop = vi.fn()
    useWindowFileDrop({ onDrop })
    await hooks.mounted()
    await hooks.handler({ payload: { type: 'drop', paths: ['a.pdf'] } })
    expect(onDrop).not.toHaveBeenCalled()
    hooks.element.visible = true
    await hooks.handler({ payload: { type: 'drop', paths: ['a.pdf'] } })
    expect(onDrop).toHaveBeenCalledExactlyOnceWith(['a.pdf'])
  })
})

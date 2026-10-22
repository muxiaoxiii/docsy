import { afterEach, describe, expect, it, vi } from 'vitest'
vi.mock('vue', async (original) => ({ ...(await original()), useSSRContext: () => ({ modules: new Set() }) }))
import FileQueuePanel from './FileQueuePanel.vue'
afterEach(() => vi.unstubAllGlobals())
describe('queue sorting and explicit page drops', () => {
  it('uses the dedicated page target without also reordering the file', () => {
    const emit = vi.fn(),
      handle = { setPointerCapture: vi.fn(), releasePointerCapture: vi.fn() }
    const state = FileQueuePanel.setup(
      { items: [{ path: 'a' }, { path: 'b' }], groupDropAttribute: 'data-image-page-drop', busy: false },
      { expose: vi.fn(), emit },
    )
    vi.stubGlobal('document', {
      elementFromPoint: () => ({
        closest: (selector) => (selector.includes('data-image-page-drop') ? { getAttribute: () => 'page-b' } : null),
      }),
    })
    const event = { button: 0, pointerId: 1, clientX: 40, clientY: 80, currentTarget: handle, preventDefault: vi.fn() }
    state.start(0, event)
    state.handleMove(event)
    state.handleFinish(event)
    expect(emit).toHaveBeenCalledWith('group-drop', { from: 0, target: 'page-b' })
    expect(emit.mock.calls.some(([name]) => name === 'reorder')).toBe(false)
    expect(handle.releasePointerCapture).toHaveBeenCalledWith(1)
  })
})

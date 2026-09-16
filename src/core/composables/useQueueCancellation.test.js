import { afterEach, describe, expect, it, vi } from 'vitest'

const hooks = vi.hoisted(() => ({ mounted: [], unmounted: [] }))
vi.mock('vue', async importOriginal => ({
  ...(await importOriginal()),
  onMounted: callback => hooks.mounted.push(callback),
  onBeforeUnmount: callback => hooks.unmounted.push(callback),
}))
import { useQueueCancellation } from './useQueueCancellation.js'

describe('批量任务取消', () => {
  afterEach(() => {
    hooks.unmounted.forEach(callback => callback())
    hooks.mounted = []
    hooks.unmounted = []
    vi.unstubAllGlobals()
  })

  it('当前不可中断任务返回成功时，也不再启动下一项；下次执行可恢复', async () => {
    vi.stubGlobal('window', new globalThis.EventTarget())
    const queue = useQueueCancellation()
    hooks.mounted.forEach(callback => callback())
    const completed = []
    queue.reset()
    for (const item of ['one', 'two']) {
      if (queue.cancelled.value) break
      await Promise.resolve().then(() => window.dispatchEvent(new globalThis.Event('docsy-cancel-requested')))
      completed.push(item)
    }
    expect(completed).toEqual(['one'])
    queue.reset()
    expect(queue.cancelled.value).toBe(false)
  })

  it('卸载时停止队列并移除监听，不影响下一次挂载', () => {
    vi.stubGlobal('window', new globalThis.EventTarget())
    const remove = vi.spyOn(window, 'removeEventListener')
    const queue = useQueueCancellation()
    hooks.mounted.forEach(callback => callback())
    hooks.unmounted.forEach(callback => callback())
    expect(queue.cancelled.value).toBe(true)
    expect(remove).toHaveBeenCalledWith('docsy-cancel-requested', expect.any(Function))
  })
})

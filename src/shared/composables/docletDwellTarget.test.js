import { afterEach, describe, expect, it, vi } from 'vitest'
import { createDwellTarget } from './docletDwellTarget.js'
function setup() {
  vi.useFakeTimers()
  const emit = vi.fn()
  const attention = createDwellTarget({ emit, schedule: setTimeout, unschedule: clearTimeout })
  return { attention, emit }
}
afterEach(() => vi.useRealTimers())
describe('Doclet 停留注意力', () => {
  it('快速扫过多个位置全部忽略，停留够长才接受最终位置', () => {
    const h = setup()
    for (let i = 0; i < 10; i++) {
      h.attention.move(i * 100, 0)
      vi.advanceTimersByTime(20)
    }
    expect(h.emit).not.toHaveBeenCalled()
    vi.advanceTimersByTime(299)
    expect(h.emit).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(h.emit).toHaveBeenCalledWith({ x: 900, y: 0 })
  })
  it('小范围晃动算作停留，并只发一次平均位置', () => {
    const h = setup()
    h.attention.move(100, 100)
    vi.advanceTimersByTime(100)
    h.attention.move(110, 110)
    vi.advanceTimersByTime(220)
    expect(h.emit).toHaveBeenCalledWith({ x: 105, y: 105 })
    h.attention.move(105, 103)
    vi.advanceTimersByTime(1000)
    expect(h.emit).toHaveBeenCalledTimes(1)
  })
  it('离开或销毁时取消待处理位置', () => {
    const h = setup()
    h.attention.move(10, 10)
    h.attention.reset()
    vi.advanceTimersByTime(1000)
    expect(h.emit).not.toHaveBeenCalled()
  })
})

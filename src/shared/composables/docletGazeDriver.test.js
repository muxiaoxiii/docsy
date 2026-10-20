import { describe, expect, it } from 'vitest'
import { createGazeDriver } from './docletGazeDriver.js'
function harness() {
  let id = 0
  let now = 0
  const callbacks = new Map()
  const drawn = []
  const driver = createGazeDriver({
    draw: frame => drawn.push({ frame, time: now }),
    request: callback => { callbacks.set(++id, callback); return id },
    cancel: key => callbacks.delete(key),
  })
  const advance = duration => {
    const end = now + duration
    while (now < end) {
      now += 1000 / 120
      const batch = [...callbacks.values()]
      callbacks.clear()
      for (const callback of batch) callback(now)
    }
  }
  driver.start()
  return { driver, drawn, advance, callbacks }
}
describe('Doclet 慢速跟随刷新预算', () => {
  it('高刷新屏也限制绘制频率和转头速度，收敛后完全停止', () => {
    const h = harness()
    h.driver.aim(12)
    h.advance(1000)
    expect(h.drawn.length).toBeLessThanOrEqual(32)
    expect(h.drawn.at(-1).frame).toBeLessThanOrEqual(42)
    h.advance(5000)
    expect(h.callbacks.size).toBe(0)
    expect(h.drawn.at(-1).frame).toBe(60)
    const count = h.drawn.length
    h.advance(1000)
    expect(h.drawn).toHaveLength(count)
  })
  it('只跟随最新方向，不排队追赶旧位置；停止时释放回调', () => {
    const h = harness()
    h.driver.aim(12)
    h.driver.aim(36)
    h.advance(50)
    expect(h.drawn.at(-1).frame).toBeGreaterThan(200)
    h.driver.stop()
    expect(h.callbacks.size).toBe(0)
  })
})

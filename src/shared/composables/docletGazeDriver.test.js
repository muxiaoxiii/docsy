import { describe, expect, it } from 'vitest'
import { createGazeDriver } from './docletGazeDriver.js'
import { gazePosition } from './useDocletGaze.js'
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
  it('从右侧到上侧经过右上姿态，不经过视频中的正面片段', () => {
    const h = harness()
    h.driver.aim(gazePosition(1, 0))
    h.advance(6000)
    expect(h.drawn.at(-1).frame).toBe(85)
    const first = h.drawn.length
    h.driver.aim(gazePosition(0, -1))
    h.advance(6000)
    const transition = h.drawn.slice(first).map(item => item.frame)
    expect(transition.length).toBeGreaterThan(5)
    expect(transition.at(-1)).toBe(55)
    expect(transition.every(frame => frame >= 55 && frame <= 85)).toBe(true)
    expect(transition.some(frame => frame >= 60 && frame <= 70)).toBe(true)
    for (let index = 1; index < transition.length; index++) {
      expect(transition[index]).toBeLessThanOrEqual(transition[index - 1])
    }
  })
  it('只跟随最新方向，不排队追赶旧位置；停止时释放回调', () => {
    const h = harness()
    h.driver.aim(12)
    h.driver.aim(36)
    h.advance(400)
    expect(h.drawn.at(-1).frame).toBeGreaterThan(200)
    h.driver.stop()
    expect(h.callbacks.size).toBe(0)
  })
})

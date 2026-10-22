import { describe, expect, it } from 'vitest'
import { createGazeDriver } from './docletGazeDriver.js'
function harness() {
  let id = 0,
    now = 0
  const callbacks = new Map(),
    drawn = []
  const driver = createGazeDriver({
    draw: (frame, from, blend) => drawn.push({ frame, from, blend, time: now }),
    request: (callback) => {
      callbacks.set(++id, callback)
      return id
    },
    cancel: (key) => callbacks.delete(key),
  })
  function advance(duration) {
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
describe('Doclet 停留目标过渡', () => {
  it('只在起点与终点间短暂过渡，高刷新屏不增加绘制频率，停稳后停止', () => {
    const h = harness()
    h.driver.aim({ x: 1, y: 0 })
    h.advance(500)
    expect(h.drawn.length).toBeLessThanOrEqual(7)
    expect(h.drawn.at(-1)).toMatchObject({ frame: 85, from: 0, blend: 1 })
    expect(h.drawn.at(-1).time).toBeLessThan(180)
    expect(h.callbacks.size).toBe(0)
    const count = h.drawn.length
    h.advance(1000)
    expect(h.drawn.length).toBe(count)
  })
  it('右侧到上侧不播放环顾中间帧，也不经过正面', () => {
    const h = harness()
    h.driver.aim({ x: 1, y: 0 })
    h.advance(500)
    const start = h.drawn.length
    h.driver.aim({ x: 0, y: -1 })
    h.advance(500)
    const transition = h.drawn.slice(start)
    expect(transition.length).toBeGreaterThan(1)
    expect(transition.every((item) => item.frame === 55 && item.from === 85)).toBe(true)
    expect(transition.at(-1).blend).toBe(1)
  })
  it('最新目标覆盖旧目标，不播放待处理的方向队列；卸载取消绘制', () => {
    const h = harness()
    h.driver.aim({ x: 1, y: 0 })
    h.driver.aim({ x: -1, y: 0 })
    h.advance(500)
    expect(h.drawn.at(-1).frame).toBe(175)
    expect(h.drawn.slice(1).every((item) => item.frame === 175)).toBe(true)
    h.driver.aim({ x: 0, y: 0 })
    h.driver.stop()
    expect(h.callbacks.size).toBe(0)
  })
})

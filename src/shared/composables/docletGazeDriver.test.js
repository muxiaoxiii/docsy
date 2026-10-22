import { describe, expect, it } from 'vitest'
import { createGazeDriver } from './docletGazeDriver.js'
import { GAZE_LAYOUT as layout } from './docletGazePoses.js'
function harness() {
  let id = 0,
    now = 0
  const callbacks = new Map(),
    drawn = []
  const driver = createGazeDriver({
    draw: (frame) => drawn.push({ frame, time: now }),
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
describe('Doclet 正面转向片段', () => {
  it('仅播放该方向的真实转向片段，停稳后不刷新，高刷新屏不重复画同一姿态', () => {
    const h = harness()
    h.driver.aim('right')
    h.advance(1500)
    expect(h.drawn.slice(1).map((item) => item.frame)).toEqual(layout.routes.right.out)
    expect(h.drawn.at(-1).frame).toBe(layout.routes.right.out.at(-1))
    expect(h.callbacks.size).toBe(0)
    expect(h.drawn.length).toBeLessThan(20)
    const count = h.drawn.length
    h.advance(1000)
    expect(h.drawn.length).toBe(count)
  })
  it('右到上只播放右侧回正与正面向上，不穿过下方或左方', () => {
    const h = harness()
    h.driver.aim('right')
    h.advance(1000)
    const start = h.drawn.length
    h.driver.aim('up')
    h.advance(2000)
    const transition = h.drawn.slice(start)
    const center = transition.findIndex((item) => item.frame === layout.center)
    expect(center).toBeGreaterThan(0)
    expect(transition.slice(0, center).every((item) => layout.routes.right.back.includes(item.frame))).toBe(true)
    expect(transition.slice(center + 1).map((item) => item.frame)).toEqual(layout.routes.up.out)
    expect(transition[center].time - transition[0].time).toBeLessThanOrEqual(125)
    expect(transition.at(-1).time - transition[center].time).toBeLessThanOrEqual(250)
  })
  it('回正期间只保留最新目标，跳过曾经指向的方向', () => {
    const h = harness()
    h.driver.aim('right')
    h.advance(1000)
    const start = h.drawn.length
    h.driver.aim('left')
    h.advance(30)
    h.driver.aim('down')
    h.driver.aim('up')
    h.advance(2000)
    const frames = h.drawn.slice(start).map((item) => item.frame)
    expect(frames.at(-1)).toBe(layout.routes.up.out.at(-1))
    expect(frames.some((frame) => layout.routes.left.out.includes(frame))).toBe(false)
    expect(frames.some((frame) => layout.routes.down.out.includes(frame))).toBe(false)
    h.driver.aim(null)
    h.advance(1000)
    expect(h.drawn.at(-1).frame).toBe(layout.center)
    expect(h.callbacks.size).toBe(0)
    h.driver.aim('left')
    h.driver.stop()
    expect(h.callbacks.size).toBe(0)
  })
  it('转向途中改变目标也会先收回当前方向，不继续播放旧转向', () => {
    const h = harness()
    h.driver.aim('right')
    h.advance(120)
    const start = h.drawn.length
    h.driver.aim('left')
    h.advance(2000)
    const frames = h.drawn.slice(start).map((item) => item.frame)
    expect(frames.at(-1)).toBe(layout.routes.left.out.at(-1))
    expect(frames.includes(layout.routes.right.out.at(-1))).toBe(false)
    expect(frames.includes(layout.center)).toBe(true)
  })
})

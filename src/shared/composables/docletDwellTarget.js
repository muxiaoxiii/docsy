// Approximate attention: a point must stay inside one small region for 320 ms.
// Fast crossings are discarded, with one pending timer and no position queue.
export function createDwellTarget({ emit, schedule, unschedule, delay = 320, radius = 40 }) {
  let region
  let timer = null
  let reported = false
  const clear = () => {
    if (timer !== null) unschedule(timer)
    timer = null
  }
  return {
    move(x, y) {
      if (!region || Math.hypot(x - region.x, y - region.y) > radius) {
        clear()
        region = { x, y, sumX: x, sumY: y, count: 1 }
        reported = false
      } else {
        region.sumX += x
        region.sumY += y
        region.count++
      }
      if (reported || timer !== null) return
      timer = schedule(() => {
        timer = null
        reported = true
        emit({ x: region.sumX / region.count, y: region.sumY / region.count })
      }, delay)
    },
    reset() {
      clear()
      region = null
      reported = false
    },
  }
}

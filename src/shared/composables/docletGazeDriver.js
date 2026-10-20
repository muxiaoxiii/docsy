// No reactive state or DOM writes in the pointer/render loop.
export function gazeDistance(from, to) {
  return ((to - from + 72) % 48) - 24
}
export function createGazeDriver({ draw, request, cancel }) {
  let position = 0
  let target = 0
  let pending = null
  let last = null
  let painted = -1
  const interval = 1000 / 30
  function paint() {
    const frame = Math.round(position * 5) % 240
    if (frame !== painted) { draw(frame); painted = frame }
  }
  function tick(time) {
    pending = null
    const elapsed = last === null ? interval : time - last
    if (elapsed + .01 >= interval) {
      last = time
      const delta = gazeDistance(position, target)
      const settled = Math.abs(delta) < .02
      const step = delta * (1 - Math.exp(-Math.min(80, elapsed) / 180))
      const limit = Math.min(80, elapsed) * .008
      position = settled ? target : (position + Math.max(-limit, Math.min(limit, step)) + 48) % 48
      paint()
      if (settled) { last = null; return }
    }
    pending = request(tick)
  }
  return {
    aim(next) {
      target = next
      if (pending === null && Math.abs(gazeDistance(position, target)) > .02) pending = request(tick)
    },
    start() { paint() },
    stop() {
      if (pending !== null) cancel(pending)
      pending = null
      last = null
    },
  }
}

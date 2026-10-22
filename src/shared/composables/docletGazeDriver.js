import { GAZE_LAYOUT } from './docletGazePoses.js'

// Each route is genuine center → direction → center footage. A new intent
// replaces the old one; unrelated intermediate directions are never played.
export function createGazeDriver({ draw, request, cancel }) {
  let direction = null
  let target = null
  let phase = 'center'
  let started = null
  let offset = 0
  let progress = 0
  let pending = null
  let painted = -1
  const paint = (frame) => {
    if (frame !== painted) {
      draw(frame)
      painted = frame
    }
  }
  function begin(next, fraction = 0) {
    phase = next
    offset = fraction
    progress = fraction
    started = null
  }
  function tick(time) {
    pending = null
    if (phase === 'center') {
      if (!target) return
      direction = target
      begin('out')
    }
    started ??= time
    const route = GAZE_LAYOUT.routes[direction]
    const frames = route[phase]
    const duration =
      phase === 'out'
        ? Math.max(320, ((frames.length - 1) * 1000) / 24)
        : Math.max(200, ((frames.length - 1) * 1000) / 30)
    progress = Math.min(1, offset + (time - started) / duration)
    paint(frames[Math.min(frames.length - 1, Math.floor(progress * (frames.length - 1)))])
    if (progress === 1) {
      if (phase === 'out') {
        if (target === direction) {
          phase = 'hold'
          return
        }
        begin('back')
      } else {
        phase = 'center'
        direction = null
        paint(GAZE_LAYOUT.center)
        if (!target) return
      }
    }
    pending = request(tick)
  }
  return {
    aim(next) {
      if (next === target) return
      target = next
      if (phase === 'hold' && next !== direction) begin('back')
      // Match the partial outward pose to the corresponding return progress.
      if (phase === 'out' && next !== direction) begin('back', 1 - progress)
      if (pending === null && (phase !== 'center' || target)) pending = request(tick)
    },
    start() {
      paint(GAZE_LAYOUT.center)
    },
    stop() {
      if (pending !== null) cancel(pending)
      pending = null
      started = null
    },
  }
}

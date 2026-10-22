import { GAZE_LAYOUT } from './docletGazePoses.js'

// Radial departures and short diagonal snippets share a center hub. A new
// intent replaces the old one; unrelated directions are never played.
export function createGazeDriver({ draw, request, cancel }) {
  let direction = null
  let target = null
  let phase = 'center'
  let started = null
  let offset = 0
  let progress = 0
  let pending = null
  let painted = -1
  let seam = null
  const paint = (frame, time = 0, settled = false) => {
    const crossesSource = painted >= 0 && frame >= 1000 !== painted >= 1000
    const a = Math.min(frame, painted) - 1000
    const b = Math.max(frame, painted) - 1000
    const crossesBlinkGap = frame >= 1000 && painted >= 1000 && ((a < 92 && b > 100) || (a < 186 && b > 194))
    if (settled) seam = null
    else if (!seam && (crossesSource || crossesBlinkGap)) seam = { from: painted, started: time }
    const mix = seam ? Math.min(1, (time - seam.started) / 45) : 1
    if (frame !== painted || seam || settled) {
      draw(frame, seam?.from ?? frame, mix)
      painted = frame
    }
    if (mix === 1) seam = null
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
    // A turn is a short reaction, not a slow replay of the source video.
    // Sample by elapsed time so slower displays skip frames instead of queuing.
    const duration = phase === 'out' ? 220 : 100
    progress = Math.min(1, offset + (time - started) / duration)
    paint(frames[Math.min(frames.length - 1, Math.floor(progress * (frames.length - 1)))], time, progress === 1)
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
        // The real return clip already ends in a frontal pose. Do not flash
        // a second, slightly different center image before the next departure.
        if (!target) {
          paint(GAZE_LAYOUT.center, time, true)
          return
        }
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
      paint(GAZE_LAYOUT.center, 0, true)
    },
    stop() {
      if (pending !== null) cancel(pending)
      pending = null
      started = null
      seam = null
    },
  }
}

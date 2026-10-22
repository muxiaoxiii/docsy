import { nearestGazePose } from './docletGazePoses.js'

// A circular source clip cannot supply arbitrary radial turns. Transition only
// between stable destination poses; never manufacture a path through that clip.
export function createGazeDriver({ draw, request, cancel }) {
  let current = nearestGazePose(0, 0)
  let destination = current
  let pending = null
  let started = null
  let lastPaint = null
  const duration = 140
  const interval = 1000 / 30

  function tick(time) {
    pending = null
    started ??= time
    if (lastPaint === null || time - lastPaint + 0.01 >= interval || time - started >= duration) {
      lastPaint = time
      const progress = Math.min(1, (time - started) / duration)
      const blend = progress * progress * (3 - 2 * progress)
      draw(destination, current, blend)
      if (progress === 1) {
        current = destination
        started = null
        lastPaint = null
        return
      }
    }
    pending = request(tick)
  }

  return {
    aim(target) {
      const next = nearestGazePose(target.x, target.y, destination)
      if (next === destination) return
      // New stable intent replaces the old turn; no direction queue. If a
      // transition is interrupted, retain its destination as the next origin.
      current = destination
      destination = next
      started = null
      lastPaint = null
      if (pending === null) pending = request(tick)
    },
    start() {
      draw(current, current, 1)
    },
    stop() {
      if (pending !== null) cancel(pending)
      pending = null
      started = null
      lastPaint = null
    },
  }
}

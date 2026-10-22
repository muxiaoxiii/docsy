import layout from '../../assets/doclet-video/gaze-layout.json'

export const GAZE_POSES = layout.poses
export const GAZE_LAYOUT = layout
export const GAZE_CELL = new Map(GAZE_POSES.map((pose, index) => [pose.frame, index]))

export function gazeTarget(x, y) {
  const length = Math.hypot(x, y)
  if (length < 0.18) return { x: 0, y: 0 }
  const scale = Math.max(1, length)
  return { x: x / scale, y: y / scale }
}

// Pick the nearest actual pose in screen space. Never walk video timestamps.
export function nearestGazePose(x, y, previous) {
  let best = GAZE_POSES[0]
  let distance = Infinity
  for (const pose of GAZE_POSES) {
    const next = (pose.x - x) ** 2 + (pose.y - y) ** 2
    if (next < distance) {
      best = pose
      distance = next
    }
  }
  const old = GAZE_POSES[GAZE_CELL.get(previous)]
  if (old && (old.x - x) ** 2 + (old.y - y) ** 2 <= distance + 0.0004) return old.frame
  return best.frame
}

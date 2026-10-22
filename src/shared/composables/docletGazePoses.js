import layout from '../../assets/doclet-video/gaze-layout.json'

export const GAZE_LAYOUT = layout
export const GAZE_CELL = new Map(layout.frames.map((frame, index) => [frame, index]))

// Only map directions actually present in the radial-turn source. The later
// head tilts are not reliable diagonal gaze poses, so use the nearest cardinal.
export function gazeTarget(x, y) {
  if (Math.hypot(x, y) < 0.28) return null
  if (Math.abs(x) > Math.abs(y)) return x > 0 ? 'right' : 'left'
  return y > 0 ? 'down' : 'up'
}

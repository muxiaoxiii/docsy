import layout from '../../assets/doclet-video/gaze-layout.json'

export const GAZE_LAYOUT = layout
export const GAZE_CELL = new Map(layout.frames.map((frame, index) => [frame, index]))

// New radial clips supply cardinal departures and returns. The original
// orbit contributes only short adjacent diagonal segments, never a full loop.
const directions = ['right', 'down-right', 'down', 'down-left', 'left', 'up-left', 'up', 'up-right']
export function gazeTarget(x, y) {
  if (Math.hypot(x, y) < 0.28) return null
  const sector = (Math.round(Math.atan2(y, x) / (Math.PI / 4)) + 8) % 8
  return directions[sector]
}

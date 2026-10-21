// Coordinates are source-video frame numbers, not compass bearings.
// 40 is a frontal pose; 55 is the start of the raised, upper-right pose.
// Keep the right -> upper-right -> up arc inside frames 55..85.
export const GAZE_COMPASS = [11, 13, 17, 22, 26, 31, 36, 41, 59]

// Do not park on a blink or a partially closed eye from the look-around video.
// Wink remains a separate complete action (open -> wink -> open).
export const GAZE_BLINK_RANGES = [[20, 34], [92, 100], [116, 124], [186, 194], [211, 219], [231, 239]]
export function gazeSourceFrame(position) {
  const frame = ((Math.round(position * 5) % 240) + 240) % 240
  const blink = GAZE_BLINK_RANGES.find(([start, end]) => frame >= start && frame <= end)
  if (!blink) return frame
  const [start, end] = blink
  return frame - start <= end - frame ? start - 1 : (end + 1) % 240
}

const clamp = (value, min, max) => Math.max(min, Math.min(max, value))
/** Normalized crop box [left, top, right, bottom], with a minimum source-pixel span. */
export function dragCrop(box, handle, dx, dy, minX = 0.01, minY = 0.01) {
  let [left, top, right, bottom] = box
  if (handle === 'move') {
    const x = clamp(dx, -left, 1 - right), y = clamp(dy, -top, 1 - bottom)
    return [left + x, top + y, right + x, bottom + y]
  }
  if (handle.includes('w')) left = clamp(left + dx, 0, right - minX)
  if (handle.includes('e')) right = clamp(right + dx, left + minX, 1)
  if (handle.includes('n')) top = clamp(top + dy, 0, bottom - minY)
  if (handle.includes('s')) bottom = clamp(bottom + dy, top + minY, 1)
  return [left, top, right, bottom]
}

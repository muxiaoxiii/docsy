/**
 * Chinese number conversion (0-9999).
 * Used by page numbers, evidence numbering, and template fields.
 */
const CN_DIGITS = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九']
const CN_UNITS = ['', '十', '百', '千']

export function toChineseNumber(value) {
  const num = Number(value)
  if (!Number.isInteger(num) || num < 0) return String(value)
  if (num > 9999) return String(value)
  if (num === 0) return '零'

  const chars = String(num).split('').map(Number)
  let result = ''
  let pendingZero = false

  chars.forEach((digit, index) => {
    const unitIndex = chars.length - index - 1
    if (digit === 0) {
      pendingZero = result.length > 0 && chars.slice(index + 1).some((next) => next !== 0)
      return
    }
    if (pendingZero) result += '零'
    pendingZero = false
    // 一十 → 十 (e.g., 10 = 十, not 一十)
    if (!(digit === 1 && unitIndex === 1 && result === '')) result += CN_DIGITS[digit]
    result += CN_UNITS[unitIndex]
  })
  return result || '零'
}

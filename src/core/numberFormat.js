const CN_DIGITS = ['', '一', '二', '三', '四', '五', '六', '七', '八', '九']

export function toChineseNumber(value) {
  const num = Number(value)
  if (!Number.isInteger(num) || num <= 0) return String(value)
  if (num < 10) return CN_DIGITS[num]
  if (num === 10) return '十'
  if (num < 20) return `十${CN_DIGITS[num % 10]}`
  if (num < 100) {
    const tens = Math.floor(num / 10)
    const ones = num % 10
    return `${CN_DIGITS[tens]}十${CN_DIGITS[ones]}`
  }
  return String(value)
}

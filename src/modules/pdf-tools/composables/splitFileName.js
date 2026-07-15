export function todayCompact(date = new Date()) {
  const yyyy = date.getFullYear()
  const mm = String(date.getMonth() + 1).padStart(2, '0')
  const dd = String(date.getDate()).padStart(2, '0')
  return `${yyyy}${mm}${dd}`
}

export function formatSplitFileName({
  base,
  index = 0,
  prefix = '',
  suffix = '',
  dateValue = '',
  separator = '-',
  customSeparator = '',
}) {
  const fallback = String(base || `文件${index + 1}`).trim() || `文件${index + 1}`
  const parts = [
    expandSplitNameTokens(prefix, index, dateValue),
    expandSplitNameTokens(fallback, index, dateValue),
    expandSplitNameTokens(suffix, index, dateValue),
  ]
    .map((part) => String(part || '').trim())
    .filter(Boolean)
  return parts.join(resolveSplitNameSeparator(separator, customSeparator)) || fallback
}

export function resolveSplitNameSeparator(separator, customSeparator = '') {
  return separator === 'custom' ? customSeparator : separator
}

export function expandSplitNameTokens(value, index = 0, dateValue = '') {
  return String(value || '').replace(/\[([^\]]+)\]/g, (match, token) => {
    if (/^#+$/.test(token)) return formatSequenceToken(token, index)
    if (token === '序号') return String(index + 1)
    if (token === '日期' || /[YyMmDd]/.test(token)) {
      return formatDateToken(token === '日期' ? 'YYYYMMDD' : token, dateValue)
    }
    return match
  })
}

export function formatSequenceToken(token, index = 0) {
  const value = String(Math.max(1, Number(index || 0) + 1))
  return value.padStart(token.length, '0')
}

export function formatDateToken(pattern, value) {
  const digits = String(value || '').replace(/\D/g, '')
  const fallback = todayCompact()
  const normalized = digits.length >= 8 ? digits.slice(0, 8) : fallback
  const yyyy = normalized.slice(0, 4)
  const yy = yyyy.slice(2)
  const month = Number(normalized.slice(4, 6)) || 1
  const day = Number(normalized.slice(6, 8)) || 1
  const mm = String(month).padStart(2, '0')
  const dd = String(day).padStart(2, '0')

  return String(pattern || 'YYYYMMDD').replace(/YYYY|yyyy|YY|yy|MM|mm|M|m|DD|dd|D|d/g, (token) => {
    switch (token) {
      case 'YYYY':
      case 'yyyy':
        return yyyy
      case 'YY':
      case 'yy':
        return yy
      case 'MM':
      case 'mm':
        return mm
      case 'M':
      case 'm':
        return String(month)
      case 'DD':
      case 'dd':
        return dd
      case 'D':
      case 'd':
        return String(day)
      default:
        return token
    }
  })
}

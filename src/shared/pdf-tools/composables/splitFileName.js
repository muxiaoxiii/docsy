import { toChineseNumber } from '../../../core/numberFormat.js'

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

export function expandSplitNameTokens(value, index = 0, dateValue = '', options = {}) {
  const start = Number(options.sequenceStart ?? 1)
  const step = Number(options.sequenceStep ?? 1)

  return String(value || '').replace(/\[([^\]]+)\]/g, (match, token) => {
    // 解析 [#, 起点, 步长] 格式
    const parts = token.split(/[,，]/).map((s) => s.trim())
    const mainToken = parts[0]
    if (/^#+$/.test(mainToken)) {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      const seq = tokenStart + index * tokenStep
      return formatSequenceToken(mainToken, 0, seq)
    }
    if (mainToken === '序号') {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      return String(tokenStart + index * tokenStep)
    }
    if (mainToken === '中文序号') {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      return toChineseNumber(tokenStart + index * tokenStep)
    }
    if (mainToken === '壹贰叁') {
      const tokenStart = parts.length > 1 ? Number(parts[1]) : start
      const tokenStep = parts.length > 2 ? Number(parts[2]) : step
      return toChineseFormalNumber(tokenStart + index * tokenStep)
    }
    if (mainToken === '日期' || /[YyMmDd]/.test(mainToken)) {
      return formatDateToken(mainToken === '日期' ? 'YYYYMMDD' : mainToken, dateValue)
    }
    return match
  })
}

export function formatSequenceToken(token, index = 0, overrideValue = null) {
  const value = overrideValue != null ? String(overrideValue) : String(Math.max(1, Number(index || 0) + 1))
  return value.padStart(token.length, '0')
}

function toChineseFormalNumber(n) {
  const digits = ['零', '壹', '贰', '叁', '肆', '伍', '陆', '柒', '捌', '玖']
  if (n <= 0) return String(n)
  if (n <= 9) return digits[n]
  if (n === 10) return '拾'
  if (n < 20) return `拾${digits[n % 10]}`
  if (n < 100) return `${digits[Math.floor(n / 10)]}拾${n % 10 === 0 ? '' : digits[n % 10]}`
  return String(n)
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

export { toChineseNumber }

/**
 * Render a filename from a token array (FilenameTokenInput format).
 * @param {Array} tokens - [{ type: 'field'|'preset'|'literal', value: string }]
 * @param {Object} fieldValues - { fieldName: value }
 * @param {number} index - row index (for sequence tokens)
 * @returns {string}
 */
export function renderFilenameFromTokens(tokens, fieldValues = {}, index = 0, templateName = '') {
  if (!tokens?.length) return ''
  const parts = tokens
    .map((token) => {
      if (token.type === 'field') {
        const v = fieldValues[token.value]
        if (v == null || v === '' || v === false) return token.value
        if (Array.isArray(v))
          return v
            .map((i) => (typeof i === 'object' ? i.text : i))
            .filter(Boolean)
            .join('、')
        return String(v)
      }
      if (token.type === 'preset') {
        if (token.value === '模板名') return templateName || '模板'
        if (token.value === '日期') return todayCompact()
        if (token.value === '日期-') {
          const d = new Date()
          return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
        }
        if (token.value === '日期短') {
          const d = new Date()
          return `${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
        }
        if (token.value === '序号') return String(index + 1)
        if (token.value === '序号01') return String(index + 1).padStart(2, '0')
        if (token.value === '序号001') return String(index + 1).padStart(3, '0')
        if (token.value === '中文序号') return toChineseNumber(index + 1)
        return `[${token.value}]`
      }
      return String(token.value || '')
    })
    .filter(Boolean)
  return sanitizeFilename(parts.join(''))
}

/** Replace filesystem-unsafe characters */
export function sanitizeFilename(name) {
  return String(name || '')
    .replace(/[/\\:*?"<>|]/g, '_')
    .trim()
}

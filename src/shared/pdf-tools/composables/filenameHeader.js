import { fileName } from '../../../core/filePath.js'

export function createFilenameHeaderRule() {
  return { separator: '-', start: 1, end: 1, removeOrder: true, replaceFrom: '', replaceTo: '', overrides: {} }
}

export function filenameHeaderKey(file) {
  return String(file.id || file.path || file.name || '')
}

export function filenameHeaderSource(file) {
  return file.sourcePath ? fileName(file.sourcePath) : file.name || fileName(file.path)
}

export function filenameHeaderParts(file, rule = {}) {
  let name = String(filenameHeaderSource(file) || '')
    .replace(/\.[^.]+$/, '')
    .trim()
  if (rule.removeOrder !== false) name = name.replace(/^\d{1,4}\s*[.．、]\s*/, '')
  return String(rule.separator ?? '-').length ? name.split(rule.separator ?? '-').map((part) => part.trim()) : [name]
}

export function extractFilenameHeader(file, config = {}) {
  const rule = { ...createFilenameHeaderRule(), ...config }
  const key = filenameHeaderKey(file)
  // Read the property even before it exists, so a newly added correction
  // invalidates Vue's computed preview instead of leaving the old result.
  const override = rule.overrides?.[key]
  if (Object.hasOwn(rule.overrides || {}, key)) {
    const text = String(override ?? '').trim()
    return { text, error: text ? '' : '单独修正的页眉不能为空' }
  }
  if (!rule.separator) return { text: '', error: '请填写分隔符' }
  const parts = filenameHeaderParts(file, rule)
  const start = Number(rule.start),
    end = rule.end === 0 ? parts.length : Number(rule.end)
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || end < start || end > parts.length)
    return { text: '', error: `文件名只有 ${parts.length} 段，请调整截取范围或单独修正` }
  let text = parts.slice(start - 1, end).join(rule.separator)
  if (rule.replaceFrom) text = text.replaceAll(rule.replaceFrom, rule.replaceTo || '')
  text = text.trim()
  return { text, error: text ? '' : '提取结果为空，请调整规则或单独修正' }
}

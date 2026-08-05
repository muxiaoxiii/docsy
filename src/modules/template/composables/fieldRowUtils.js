/**
 * Pure utility functions for template field row manipulation.
 * All functions in this file are side-effect-free and accept their
 * dependencies (like `rows`) as explicit parameters.
 */
import {
  looksLikePrefixMark,
  looksLikeSuffixMark,
  prefixStructureInfo,
  suffixTargetRule,
} from '../rules/publicRules.js'

// ── Type / usage helpers ─────────────────────────────────────────────────────

// Merged field-type groups shown in the type picker. Groups with subTypes
// expose a second "方式" selector; the stored type stays the concrete one
// (checkbox/radio_group/…), so the backend/scan/render logic is untouched.
export const FIELD_TYPE_GROUPS = [
  { value: 'text', label: '文本', description: '普通可替换文字，如法院、案号、律所名称。' },
  { value: 'date', label: '日期', description: '日期字段，填写时用日期选择器；可设输出格式或留空手写。' },
  { value: 'select', label: '下拉选择', description: '从预设选项中选择或手动输入，如案由、诉讼阶段。' },
  { value: 'party_list', label: '列表', description: '适合当事人、律师等多项内容；多个名称按顺序用顿号连接。' },
  { value: 'reference', label: '引用', description: '复用前面字段的值；来源由填写时选择或在设置里指定。' },
  {
    value: 'check',
    label: '勾选',
    description: '方框勾选，可设为单个、互斥组或多选组。',
    subTypes: [
      { value: 'checkbox', label: '单个勾选', description: '一个独立方框，只控制是否勾选。' },
      { value: 'radio_group', label: '互斥勾选组', description: '多个方框只能选一个，如一般授权/特别授权。' },
      { value: 'checkbox_group', label: '多选勾选组', description: '多个方框可同时选中，如多个保全事项。' },
    ],
  },
  {
    value: 'link',
    label: '连接文字',
    description: '字段为空时随字段一起删除的连接文字（前缀/后缀）。',
    subTypes: [
      { value: 'prefix', label: '前缀', description: '如"原告""，第三人""（案号："。' },
      { value: 'suffix', label: '后缀', description: '如"律师""）"。' },
    ],
  },
  {
    value: 'action',
    label: '文本处理',
    description: '对这段标黄文字的特殊处理。',
    subTypes: [
      { value: 'delete_text', label: '删除文本', description: '保存模板时从 Word 原文中删除这段文字。' },
      { value: 'ignore', label: '保留原文', description: '不作为字段或规则保存；保存模板时只清除黄色高亮。' },
    ],
  },
]

export const typeHelpItems = FIELD_TYPE_GROUPS

// Map a concrete stored type to its merged group (or itself).
export function typeGroupOf(type) {
  if (['checkbox', 'radio_group', 'checkbox_group'].includes(type)) return 'check'
  if (type === 'prefix' || type === 'suffix') return 'link'
  if (type === 'delete_text' || type === 'ignore') return 'action'
  return type
}

// The 方式 sub-options for a stored type's group (null when no sub types).
export function typeGroupSubOptions(type) {
  const group = typeGroupOf(type)
  const entry = FIELD_TYPE_GROUPS.find((g) => g.value === group)
  return entry?.subTypes || null
}

// Concrete type for a (group, sub) selection; plain groups use the group value.
export function typeActualOf(group, sub) {
  const entry = FIELD_TYPE_GROUPS.find((g) => g.value === group)
  if (entry?.subTypes?.length) {
    const chosen = entry.subTypes.find((s) => s.value === sub)
    return chosen ? chosen.value : entry.subTypes[0].value
  }
  return group
}

export const previewLegendItems = [
  { className: 'preview-text', label: '文本', type: 'text' },
  { className: 'preview-text', label: '下拉选择', type: 'select' },
  { className: 'preview-date', label: '日期', type: 'date' },
  { className: 'preview-party', label: '列表', type: 'party_list' },
  { className: 'preview-reference', label: '引用', type: 'reference' },
  { className: 'preview-checkbox', label: '单个勾选', type: 'checkbox' },
  { className: 'preview-radio', label: '互斥勾选组', type: 'radio_group' },
  { className: 'preview-checkbox-group', label: '多选勾选组', type: 'checkbox_group' },
  { className: 'preview-prefix', label: '前缀', type: 'prefix' },
  { className: 'preview-suffix', label: '后缀', type: 'suffix' },
  { className: 'preview-ignore', label: '保留原文', type: 'ignore' },
]

export const checkedSymbolOptions = ['☑', '☒', '✓', '√', '✔', '●', '(√)']
export const uncheckedSymbolOptions = ['☐', '□', '○', '( )']

export function rowUsage(row) {
  if (['prefix', 'suffix', 'ignore', 'delete_text'].includes(row?.type)) return row.type
  return 'field'
}

export function isMarkerType(type) {
  return ['checkbox', 'radio_group', 'checkbox_group'].includes(type)
}

export function typeLabel(type) {
  return typeHelpItems.find((item) => item.value === type)?.label || type
}

export function isPartyFieldRow(row) {
  return row?.enabled && rowUsage(row) === 'field' && row.type === 'party_list' && row.name?.trim()
}

export function isSelectableFieldRow(row) {
  return !row.displayOnly
}

export function groupedRowKey(row) {
  return `${row?.type || ''}:${String(row?.name || '').trim()}`
}

export function isGeneratedFieldName(name) {
  return /^field_\d+$/.test(String(name || '')) || /^字段\d+$/.test(String(name || ''))
}

export function isLikelyPrefixMark(text) {
  return looksLikePrefixMark(text)
}

export function isLikelySuffixMark(text) {
  return looksLikeSuffixMark(text)
}

export function normalizeComparableText(text) {
  return String(text || '')
    .replace(/\s+/g, '')
    .trim()
}

export function charLength(text) {
  return [...String(text || '')].length
}

export function sliceChars(text, start, end) {
  return [...String(text || '')].slice(start, end).join('')
}

export function defaultCheckedText(text) {
  const trimmed = String(text || '').trim()
  if (trimmed.includes('(') || trimmed.includes('（')) return '(√)'
  return '☑'
}

export function defaultUncheckedText(text) {
  const trimmed = String(text || '').trim()
  if (trimmed.includes('(') || trimmed.includes('（')) return '( )'
  if (trimmed === '□') return '□'
  return '☐'
}

export function cleanTemplateName(name) {
  return String(name || '')
    .replace(/[-_ ]?(标黄|模板|可替换|待填|字段版)$/i, '')
    .trim()
}

export function splitPartyLabelText(text) {
  return splitPartyLabelSegments(text).map((item) => item.text)
}

export function splitPartyLabelSegments(text) {
  const chars = [...String(text || '')]
  const segments = []
  let start = 0
  const flush = (end) => {
    let trimmedStart = start
    let trimmedEnd = end
    while (trimmedStart < trimmedEnd && /\s/.test(chars[trimmedStart])) trimmedStart += 1
    while (trimmedEnd > trimmedStart && /\s/.test(chars[trimmedEnd - 1])) trimmedEnd -= 1
    if (trimmedEnd > trimmedStart) {
      segments.push({
        text: chars.slice(trimmedStart, trimmedEnd).join(''),
        start: trimmedStart,
        end: trimmedEnd,
      })
    }
  }
  for (let index = 0; index < chars.length; index += 1) {
    if (/[、，,；;\n]/.test(chars[index])) {
      flush(index)
      start = index + 1
    }
  }
  flush(chars.length)
  return segments
}

export function stableFieldId(name, type) {
  const slug = String(name || '')
    .trim()
    .replace(/[^a-zA-Z0-9_]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .toLowerCase()
  return `fld_${type}_${slug || hashText(name)}`
}

export function hashText(text) {
  let hash = 2166136261
  for (const char of String(text || 'field')) {
    hash ^= char.charCodeAt(0)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(16)
}

export function ensureExtension(path, extension) {
  return String(path || '')
    .toLowerCase()
    .endsWith(`.${extension}`)
    ? path
    : `${path}.${extension}`
}

export function displayValue(value) {
  if (value == null) return ''
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (Array.isArray(value)) return value.map(displayValue).join('、')
  return value.name || value.label || JSON.stringify(value)
}

export function shortDateTime(value) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

export function historyTime(value) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value)
  const pad = (n) => String(n).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

export function todayText() {
  const now = new Date()
  const year = now.getFullYear()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function isEmptyValue(value) {
  if (Array.isArray(value)) {
    return !value.length || !partyItemsToValues(value).length
  }
  return value === '' || value == null
}

export function isConnectorRow(row) {
  if (rowUsage(row) !== 'prefix') return false
  const info = prefixStructureInfo(row.text)
  return Boolean(info.connectorText && !info.roleText)
}

export function displayNameForFieldRow(row) {
  if (!row) return ''
  const label = String(row.label || '').trim()
  const rawText = String(row.text || '').trim()
  if (label && label !== rawText && !isGeneratedFieldName(label)) return label
  return String(row.name || '').trim()
}

export function isGeneratedConnectorRow(row) {
  return rowUsage(row) === 'prefix' && String(row.rowId || '').includes(':auto-connector')
}

export function isPureConnectorText(text) {
  const value = String(text || '').trim()
  return Boolean(value) && /^(?:以及|或者|[，,、;；和与及\s])+$/.test(value)
}

// ── Mark / ref helpers ───────────────────────────────────────────────────────

export function markRefsForTextRange(row, start, end) {
  const segments = row.markSegments?.length ? row.markSegments : [{ markId: row.markId, text: row.text }]
  const refs = []
  let cursor = 0
  for (const segment of segments) {
    const length = charLength(segment.text)
    const segmentStart = cursor
    const segmentEnd = cursor + length
    const overlapStart = Math.max(start, segmentStart)
    const overlapEnd = Math.min(end, segmentEnd)
    if (overlapEnd > overlapStart && segment.markId) {
      refs.push({
        markId: segment.markId,
        start: overlapStart - segmentStart,
        end: overlapEnd - segmentStart,
      })
    }
    cursor = segmentEnd
  }
  if (!refs.length && row.markId) refs.push({ markId: row.markId, start, end })
  return refs
}

export function refsForRowTextRange(row, start, end) {
  const refs = row.markRefs?.length
    ? row.markRefs
    : [{ markId: row.markId, start: row.charStart || 0, end: row.charEnd }]
  const result = []
  let cursor = 0
  for (const ref of refs) {
    const refStart = ref.start ?? 0
    const refEnd = ref.end ?? refStart
    const length = Math.max(0, refEnd - refStart)
    const segmentStart = cursor
    const segmentEnd = cursor + length
    const overlapStart = Math.max(start, segmentStart)
    const overlapEnd = Math.min(end, segmentEnd)
    if (overlapEnd > overlapStart && ref.markId) {
      result.push({
        markId: ref.markId,
        start: refStart + overlapStart - segmentStart,
        end: refStart + overlapEnd - segmentStart,
      })
    }
    cursor = segmentEnd
  }
  if (!result.length && row.markId) result.push({ markId: row.markId, start, end })
  return result
}

export function markSegmentsFromRefs(refs, text) {
  const segments = []
  let cursor = 0
  for (const ref of refs) {
    const length = Math.max(0, (ref.end ?? ref.start ?? 0) - (ref.start ?? 0))
    segments.push({
      markId: ref.markId,
      text: sliceChars(text, cursor, cursor + length),
    })
    cursor += length
  }
  return segments
}

// ── Range row helpers ────────────────────────────────────────────────────────

export function makeRangeRow(row, start, end, attrs, suffix) {
  return {
    ...row,
    ...attrs,
    rowId: `${row.rowId}:auto-${suffix}`,
    text: sliceChars(row.text, start, end),
    markRefs: markRefsForTextRange(row, start, end),
    charStart: start,
    charEnd: end,
    required: false,
    optionalWhenEmpty: false,
    referenceHintSeen: false,
    partyItems: attrs.type === 'party_list' ? splitPartyLabelText(sliceChars(row.text, start, end)) : [],
  }
}

export function dedupeRangeRows(rows) {
  const seen = new Set()
  return rows.filter((row) => {
    const key = `${row.charStart}:${row.charEnd}:${row.type}:${row.name}`
    if (seen.has(key)) return false
    seen.add(key)
    return row.charEnd > row.charStart
  })
}

// ── Neighbor / structure helpers (accept `rows` parameter) ──────────────────

export function findNeighborFieldRow(rows, startIndex, direction) {
  for (let index = startIndex + direction; index >= 0 && index < rows.length; index += direction) {
    const row = rows[index]
    if (row.enabled && rowUsage(row) === 'field') return row
    if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') continue
    break
  }
  return null
}

export function structureTargetRow(structureRow, rows) {
  const usage = rowUsage(structureRow)
  if (usage !== 'prefix' && usage !== 'suffix') return null
  const boundTarget = rows.find(
    (row) => row.enabled && rowUsage(row) === 'field' && row.rowId === structureRow.structureTargetRowId,
  )
  if (boundTarget) return boundTarget
  const sameNameFields = rows.filter(
    (row) => row.enabled && rowUsage(row) === 'field' && row.name.trim() === structureRow.name.trim(),
  )
  if (sameNameFields.length === 1) return sameNameFields[0]
  const positionalTarget = positionalStructureTargetRow(structureRow, rows)
  if (positionalTarget) return positionalTarget
  return null
}

export function positionalStructureTargetRow(structureRow, rows) {
  const usage = rowUsage(structureRow)
  if (usage !== 'prefix' && usage !== 'suffix') return null
  const direction = usage === 'prefix' ? 1 : -1
  return findNeighborFieldRow(rows, rows.indexOf(structureRow), direction)
}

export function bindStructureRowToTarget(row, target) {
  if (!row || !target) return
  row.structureTargetRowId = target.rowId || ''
  row.name = target.name || row.name || ''
}

export function sameFieldRows(row, rows) {
  return rows.filter(
    (item) =>
      item.enabled &&
      rowUsage(item) === 'field' &&
      item.name.trim() &&
      item.name.trim() === row.name.trim() &&
      item.type === row.type,
  )
}

export function isGroupedField(row, rows) {
  return rowUsage(row) === 'field' && sameFieldRows(row, rows).length > 1
}

export function groupedFieldSummary(row) {
  if (isMarkerType(row.type)) return '同一勾选组'
  // Same-name multi-position fields are fillAllPositions: later positions
  // reference the first one, so label them accordingly.
  const sourceName =
    row?.referenceSourceField || row?.referenceSourceSemanticKey || ''
  if (sourceName) return `引用：${sourceName}`
  return '同一字段，已改成引用'
}

export function markerGroupMembers(row, rows) {
  return sameFieldRows(row, rows)
    .filter((item) => isMarkerType(item.type))
    .map((item) => item.rowId)
}

export function groupColorIndex(row, rows) {
  if (!row) return 0
  const groups = []
  const seen = new Set()
  for (const item of rows) {
    if (!isGroupedField(item, rows)) continue
    const key = groupedRowKey(item)
    if (seen.has(key)) continue
    seen.add(key)
    groups.push(key)
  }
  const key = row.partyGroupKey || groupedRowKey(row)
  return Math.max(0, groups.indexOf(key)) % 6
}

export function structureTargetDisplayName(row, rows) {
  const target = structureTargetRow(row, rows)
  if (!target) return row.name || '未指定'
  return displayNameForFieldRow(target) || row.name || '未指定'
}

export function prefixRelationSummary(row, rows) {
  if (isConnectorRow(row)) return `连接符归属：${structureTargetDisplayName(row, rows)}`
  return `前缀归属：${structureTargetDisplayName(row, rows)}`
}

export function relationSummary(row, rows) {
  const usage = rowUsage(row)
  if (usage === 'prefix') return prefixRelationSummary(row, rows)
  if (usage === 'suffix') return `后缀归属：${structureTargetDisplayName(row, rows)}`
  if (usage === 'delete_text') return '保存模板时删除'
  if (usage === 'ignore') return '保留原文，仅清除高亮'
  if (isMarkerType(row.type)) return `${typeLabel(row.type)}：${row.optionLabel || row.text}`
  if (isGroupedField(row, rows)) return groupedFieldSummary(row)
  if (row.optionalWhenEmpty) return `空值处理：${row.optionalScope === 'field' ? '全部同名' : '仅此位置'}`
  return '普通字段'
}

export function displayMarkText(row) {
  if (row.virtualPartyGroup) return row.name || row.label || '当事人列表'
  if (row.partyGroupChild) return row.text
  if (isConnectorRow(row)) return `连接符：${row.text}`
  if (row.type === 'party_list' && row.partyItems?.length > 1) return row.name || row.label || row.text
  return row.text
}

export function fieldCellSecondaryLabel(row, rows) {
  if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') return structureTargetDisplayName(row, rows)
  return row.label || row.name || relationSummary(row, rows)
}

export function fieldRowClassName({ row }, rows) {
  if (row.virtualPartyGroup) {
    return `party-group-row grouped-field-row grouped-field-row-${groupColorIndex(row, rows)}`
  }
  if (row.displayOnly) {
    return `party-child-row grouped-field-row grouped-field-row-${groupColorIndex(row, rows)}`
  }
  if (!row.enabled) return 'disabled-field-row'
  if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') return 'structure-field-row'
  if (rowUsage(row) === 'delete_text') return 'delete-field-row'
  if (rowUsage(row) === 'ignore') return 'ignored-field-row'
  if (isGroupedField(row, rows)) return `grouped-field-row grouped-field-row-${groupColorIndex(row, rows)}`
  return ''
}

// ── Table builder ────────────────────────────────────────────────────────────

export function buildFieldTableRows(rows) {
  const result = []
  for (const row of rows) {
    row.partyGroupChild = false
    row.partyItemIndex = null
    row.partyItemCount = null
    row.partyGroupKey = ''
  }
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index]
    const group = consecutivePartyRows(rows, index)
    if (group.length > 1) {
      const partyGroupKey = groupedRowKey(row)
      result.push({
        rowId: `party-group:${row.name}:${index}`,
        displayOnly: true,
        virtualPartyGroup: true,
        text: row.name,
        type: row.type,
        name: row.name,
        label: row.label || row.name,
        partyItemCount: group.length,
        partyGroupKey,
      })
      for (const [itemIndex, child] of group.entries()) {
        child.partyGroupChild = true
        child.partyItemIndex = itemIndex
        child.partyItemCount = group.length
        child.partyGroupKey = partyGroupKey
        result.push(child)
      }
      index += group.length - 1
      continue
    }
    result.push(row)
  }
  return result
}

function consecutivePartyRows(rows, startIndex) {
  const first = rows[startIndex]
  if (!isPartyFieldRow(first)) return []
  const group = [first]
  for (let cursor = startIndex + 1; cursor < rows.length; cursor += 1) {
    const row = rows[cursor]
    if (!isPartyFieldRow(row) || row.name.trim() !== first.name.trim()) break
    group.push(row)
  }
  return group
}

// ── Preview utilities ────────────────────────────────────────────────────────

export function previewRangesByRun(rows) {
  const map = new Map()
  const occurrenceByRow = new Map()
  for (const row of rows) {
    if (!row.enabled) continue
    const refs = row.markRefs?.length
      ? row.markRefs
      : [{ markId: row.markId, start: row.charStart, end: row.charEnd }]
    for (const ref of refs) {
      if (!ref?.markId) continue
      if (!map.has(ref.markId)) map.set(ref.markId, [])
      const occurrenceKey = row.rowId
      const occurrence = occurrenceByRow.get(occurrenceKey) || 0
      occurrenceByRow.set(occurrenceKey, occurrence + 1)
      map.get(ref.markId).push({ row, start: ref.start ?? 0, end: ref.end ?? undefined, occurrence })
    }
  }
  for (const ranges of map.values()) {
    ranges.sort((a, b) => (a.start ?? 0) - (b.start ?? 0))
  }
  return map
}

export function previewRunSegment(run, start, end, row, index) {
  return {
    id: `run-${run.id}-${start}-${end}-${index}`,
    text: sliceChars(run.text, start, end),
    row,
    runId: run.id,
    start,
    end,
    bold: run.bold,
    italic: run.italic,
    underline: run.underline,
  }
}

export function previewPlainSegment(text, index) {
  return { id: `plain-${index}`, text, row: null, runId: '', start: 0, end: charLength(text) }
}

export function previewSourceLabel(row) {
  if (rowUsage(row) === 'ignore') return '【保留原文】'
  if (rowUsage(row) === 'delete_text') return `【删除：${row.text}】`
  if (rowUsage(row) === 'prefix')
    return `【${isConnectorRow(row) ? '连接符' : '前缀'}：${row.name || '未指定'}】`
  if (rowUsage(row) === 'suffix') return `【后缀：${row.name || '未指定'}】`
  if (row.type === 'reference') return `【引用：${row.name || '引用'}】`
  // Display name (label) wins so renaming the display name updates the
  // preview; scan rows without a label fall back to the field name.
  return `【${row.label || row.name || row.text}】`
}

export function previewReplacementText(row, sampleValues = {}) {
  const usage = rowUsage(row)
  if (usage === 'ignore') return row.text
  if (usage === 'delete_text') return ''
  const hasSample = hasPreviewSampleValue(sampleValues, row.name)
  if (usage === 'prefix' || usage === 'suffix')
    return hasSample && isEmptyPreviewValue(sampleValues[row.name]) ? '' : row.text
  if (row.type === 'reference') return previewReferenceValue(row, sampleValues) || row.text
  const value = sampleValues[row.name]
  if (row.type === 'date' && !isEmptyPreviewValue(value)) return normalizePreviewDate(value)
  if (!isEmptyPreviewValue(value)) return Array.isArray(value) ? value.join('、') : String(value)
  return row.text
}

function previewReferenceValue(row, sampleValues = {}) {
  const source = normalizedReferenceSource(row)
  if (source.mode === 'auto') return sampleValues[row.name] || ''
  const value = source.mode === 'semantic' ? sampleValues[source.sourceSemanticKey] : sampleValues[source.sourceField]
  if (Array.isArray(value)) {
    return source.sourceIndex == null ? value.join('、') : String(value[source.sourceIndex] || '')
  }
  return source.sourceIndex == null && !isEmptyPreviewValue(value) ? String(value) : ''
}

export function hasPreviewSampleValue(sampleValues, name) {
  return Object.prototype.hasOwnProperty.call(sampleValues || {}, name)
}

export function isEmptyPreviewValue(value) {
  return value == null || value === '' || (Array.isArray(value) && value.length === 0)
}

function normalizePreviewDate(value) {
  const raw = String(value || '').trim()
  if (!raw) return ''
  if (/今天|今日/.test(raw)) return todayText()
  const compact = raw.replace(/\s+/g, '')
  const ymd = compact.match(/^(\d{4})(\d{2})(\d{2})$/)
  if (ymd) return `${Number(ymd[1])}年${Number(ymd[2])}月${Number(ymd[3])}日`
  const md = compact.match(/^(\d{2})(\d{2})$/)
  if (md) return `${new Date().getFullYear()}年${Number(md[1])}月${Number(md[2])}日`
  const cn = compact.match(/^(\d{4})年(\d{1,2})月(\d{1,2})日?$/)
  if (cn) return `${Number(cn[1])}年${Number(cn[2])}月${Number(cn[3])}日`
  const dashed = compact.match(/^(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})$/)
  if (dashed) return `${Number(dashed[1])}年${Number(dashed[2])}月${Number(dashed[3])}日`
  return raw
}

// ── Date formatting (fill values → rendered date forms) ──────────────────────

const CN_DIGITS = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九']
const EN_MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']
const EN_MONTHS_SHORT = ['Jan.', 'Feb.', 'Mar.', 'Apr.', 'May', 'Jun.', 'Jul.', 'Aug.', 'Sep.', 'Oct.', 'Nov.', 'Dec.']

// Parse an entered date value into {y, m, d}; a zero part means "leave blank".
// Accepts 20260805, 2026-08-05, 2026年8月5日, "留空", etc.
export function parseDateParts(value) {
  const s = String(value ?? '').trim()
  if (!s) return null
  if (s === '留空') return { y: 0, m: 0, d: 0 }
  let y = 0
  let m = 0
  let d = 0
  const compact = s.replace(/\s+/g, '')
  if (/^\d{8}$/.test(compact)) {
    y = Number(compact.slice(0, 4))
    m = Number(compact.slice(4, 6))
    d = Number(compact.slice(6, 8))
  } else {
    const parts = compact.split(/[-/年月日.]+/).filter(Boolean)
    if (parts.length >= 3) {
      y = Number(parts[0])
      m = Number(parts[1])
      d = Number(parts[2])
    } else if (parts.length === 1 && /^\d+$/.test(compact)) {
      // bare digits without separators are ambiguous; treat as y-m-d only when 8 wide
      return null
    }
  }
  return { y: y || 0, m: m || 0, d: d || 0 }
}

function cnNumber(n) {
  if (!n) return ''
  return String(n).split('').map((c) => CN_DIGITS[Number(c)] ?? c).join('')
}

function ordinal(n) {
  const rem10 = n % 10
  const rem100 = n % 100
  if (rem100 >= 11 && rem100 <= 13) return `${n}th`
  if (rem10 === 1) return `${n}st`
  if (rem10 === 2) return `${n}nd`
  if (rem10 === 3) return `${n}rd`
  return `${n}th`
}

// Render a date value in the requested format. Zero parts become blanks
// (no limit: 0000 year / 00 month / 00 day are all "leave blank").
export function formatDateValue(value, format) {
  const parts = parseDateParts(value)
  if (!parts) return String(value ?? '')
  const { y, m, d } = parts
  const fmt = format || 'iso'
  const yStr = y ? String(y) : '    '
  const mStr = m ? String(m) : '  '
  const dStr = d ? String(d) : '  '
  if (fmt === 'cn' || fmt === 'blank') {
    // blank is the same form with all parts empty (供打印后手写)
    return `${yStr}年${mStr}月${dStr}日`
  }
  if (fmt === 'cn_full') {
    return `${cnNumber(y)}年${m ? cnNumber(m) : '  '}月${d ? cnNumber(d) : '  '}日`
  }
  const monthLong = m >= 1 && m <= 12 ? EN_MONTHS[m - 1] : ' '
  const monthShort = m >= 1 && m <= 12 ? EN_MONTHS_SHORT[m - 1] : ' '
  if (fmt === 'en_long') return `${monthLong} ${dStr}, ${yStr}`.replace(/\s+/g, ' ').trim()
  if (fmt === 'en_short') return `${monthShort} ${dStr}, ${yStr}`.replace(/\s+/g, ' ').trim()
  if (fmt === 'en_dmy') return `${dStr} ${monthLong} ${yStr}`.replace(/\s+/g, ' ').trim()
  if (fmt === 'en_ordinal') {
    const dOrd = d ? ordinal(d) : ''
    return `${yStr} ${monthLong} ${dOrd}`.replace(/\s+/g, ' ').trim()
  }
  // iso default: 2026-08-05 (zero parts stay blank segments)
  return `${yStr}-${mStr}-${dStr}`.trim()
}

export function previewTokenClass(row, segment = {}) {
  return {
    'preview-text': rowUsage(row) === 'field' && ['text', 'select', 'party_list'].includes(row.type),
    'preview-reference': rowUsage(row) === 'field' && row.type === 'reference',
    'preview-date': rowUsage(row) === 'field' && row.type === 'date',
    'preview-checkbox': rowUsage(row) === 'field' && row.type === 'checkbox',
    'preview-radio': rowUsage(row) === 'field' && row.type === 'radio_group',
    'preview-checkbox-group': rowUsage(row) === 'field' && row.type === 'checkbox_group',
    'preview-prefix': rowUsage(row) === 'prefix',
    'preview-suffix': rowUsage(row) === 'suffix',
    'preview-delete-text': rowUsage(row) === 'delete_text',
    'preview-ignore': rowUsage(row) === 'ignore',
    'preview-deleted': segment.deleted,
  }
}

export function previewFormatClass(segment) {
  return {
    'source-bold': segment.bold,
    'source-italic': segment.italic,
    'source-underline': segment.underline,
  }
}

// ── Reference source helpers ─────────────────────────────────────────────────

export function referenceSourceKey(mode = 'auto', source = '', sourceIndex = null) {
  return `${mode || 'auto'}::${source || ''}::${sourceIndex == null ? '' : sourceIndex}`
}

export function parseReferenceSourceKey(key) {
  const parts = String(key || '').split('::')
  const [mode = 'auto', source = '', sourceIndexText = ''] =
    parts.length >= 3 ? parts : ['field', parts[0] || '', parts[1] || '']
  return {
    mode: mode || 'auto',
    sourceField: mode === 'field' ? source : '',
    sourceSemanticKey: mode === 'semantic' ? source : '',
    sourceIndex: sourceIndexText === '' ? null : Number(sourceIndexText),
  }
}

export function normalizedReferenceSource(row) {
  if (row.referenceSourceField || row.referenceSourceSemanticKey || row.referenceSourceMode) {
    return {
      mode:
        row.referenceSourceMode ||
        (row.referenceSourceSemanticKey ? 'semantic' : row.referenceSourceField ? 'field' : 'auto'),
      sourceField: row.referenceSourceField,
      sourceSemanticKey: row.referenceSourceSemanticKey || '',
      sourceIndex: row.referenceSourceIndex == null ? null : row.referenceSourceIndex,
    }
  }
  return parseReferenceSourceKey(row.referenceSourceKey)
}

export function referenceSourceLabel(row) {
  const source = normalizedReferenceSource(row)
  if (source.mode === 'auto') return '填写时选择'
  if (source.mode === 'semantic') return `通用字段名：${source.sourceSemanticKey}`
  if (!source.sourceField) return ''
  return source.sourceIndex == null ? source.sourceField : `${source.sourceField}第 ${source.sourceIndex + 1} 项`
}

export function syncReferenceSourceFromKey(row) {
  const parsed = parseReferenceSourceKey(row.referenceSourceKey)
  row.referenceSourceMode = parsed.mode
  row.referenceSourceField = parsed.sourceField
  row.referenceSourceSemanticKey = parsed.sourceSemanticKey
  row.referenceSourceIndex = parsed.sourceIndex
}

// ── Structure override helpers ───────────────────────────────────────────────

export function structureOverrideKey(field) {
  return field?.id || field?.name || ''
}

// ── Party / form helpers ─────────────────────────────────────────────────────

export function partyItemsToValues(value) {
  if (typeof value === 'string') return splitPartyInput(value)
  if (!Array.isArray(value)) return []
  return value
    .map((item) => {
      if (typeof item === 'string') return item.trim()
      const suffix = String(item?.suffix || '').trim()
      let text = String(item?.text || '').trim()
      if (suffix && text.endsWith(suffix)) {
        text = text.slice(0, -suffix.length).trim()
      }
      return suffix ? { name: text, suffix } : text
    })
    .filter((item) => (typeof item === 'string' ? Boolean(item) : Boolean(item.name)))
}

export function splitPartyInput(value) {
  return value
    .split(/[、\n]/)
    .map((item) => item.trim())
    .filter(Boolean)
}

export function parsePartyItem(value) {
  if (value && typeof value === 'object') {
    return {
      text: String(value.name || value.label || value.text || '').trim(),
      suffix: String(value.suffix || '').trim(),
    }
  }
  return {
    text: String(value || '').trim(),
    suffix: '',
  }
}

export function partyFieldUsesSuffix(field) {
  return fieldUsesRepeatableSuffix(field)
}

export function fieldUsesRepeatableSuffix(field) {
  return (
    field?.type === 'party_list' && (field.markRefs || []).some((markRef) => markRef.optionalRule?.removeEmptySuffix)
  )
}

export function partySuffixOptions(field) {
  return Array.from(
    new Set((field.markRefs || []).map((markRef) => markRef.optionalRule?.removeEmptySuffix).filter(Boolean)),
  )
}

export function defaultPartySuffix(field, index) {
  const options = partySuffixOptions(field)
  if (!options.length) return ''
  return options[index] || options[0]
}

export function partyFieldStructureHint(field) {
  const suffixes = Array.from(
    new Set((field.markRefs || []).map((markRef) => markRef.optionalRule?.removeEmptySuffix).filter(Boolean)),
  )
  if (!suffixes.length) return ''
  return `可为每一项选择或输入后缀，默认来自模板：${suffixes.join('、')}。`
}

export function isRenderableField(field) {
  return !['delete_text', 'prefix', 'suffix', 'ignore'].includes(field?.type)
}

export function fillFieldLabel(field) {
  if (!field) return ''
  // Auto-generated names ("字段N") prefer the saved label (highlighted source
  // text) so the fill page shows what was marked instead of a placeholder.
  if (isGeneratedFieldName(field.name)) return field.label || field.name || ''
  if (field.type === 'party_list') return field.name || field.label || ''
  if (['前缀', '后缀', '连接符', '列表项'].includes(field.label)) return field.name || field.label || ''
  return field.label || field.name || ''
}

export function previewFieldLabel(row) {
  if (isGeneratedFieldName(row.name)) return row.name.trim()
  if (row.type === 'party_list') return row.name.trim()
  return row.label || row.name.trim()
}

// ── History helpers ──────────────────────────────────────────────────────────

export function groupHistoryRuns(runs) {
  const groups = new Map()
  for (const run of runs || []) {
    const key = run.templateId || run.templatePath || run.templateName || 'unknown'
    if (!groups.has(key)) {
      groups.set(key, {
        templateId: key,
        templateName: run.templateName || '未命名模板',
        templatePath: run.templatePath || '',
        runs: [],
      })
    }
    groups.get(key).runs.push(run)
  }
  return Array.from(groups.values())
}

export function historyRunSummary(run) {
  const summaries = Array.isArray(run.fieldSummaries) ? run.fieldSummaries : []
  if (summaries.length) {
    return summaries
      .filter((item) => item.display)
      .slice(0, 6)
      .map((item) => ({
        label: item.label || item.name,
        display: item.display,
      }))
  }
  return Object.entries(run.fieldValues || {})
    .filter(([, value]) => displayValue(value))
    .slice(0, 6)
    .map(([name, value]) => ({
      label: name,
      display: displayValue(value),
    }))
}

// ── Misc helpers ─────────────────────────────────────────────────────────────

export function leadingConnectorInfo(text) {
  const value = String(text || '')
  const match = value.match(/^(?:以及|或者|[，,、;；和与及\s])+/)
  const connectorText = match?.[0] || ''
  return {
    connectorText,
    restText: connectorText ? value.slice(connectorText.length).trim() : '',
  }
}

export function knownSuffixAtEnd(text) {
  const value = String(text || '').trim()
  const suffixes = ['诉讼代理人', '实习律师', '代理人', '律师']
  for (const suffix of suffixes) {
    if (value.endsWith(suffix)) {
      const rule = suffixTargetRule(suffix)
      if (rule) return { ...rule, text: suffix }
    }
  }
  return null
}

export function findPartCharRange(text, part, cursor = 0) {
  const source = [...String(text || '')]
  const target = [...String(part || '')]
  if (!target.length) return null
  for (let start = Math.max(0, cursor); start <= source.length - target.length; start += 1) {
    let matched = true
    for (let offset = 0; offset < target.length; offset += 1) {
      if (source[start + offset] !== target[offset]) {
        matched = false
        break
      }
    }
    if (matched) return { start, end: start + target.length }
  }
  return null
}

// ── Computed field name options (accepts rows) ──────────────────────────────

export function fieldNameOptions(rows) {
  return Array.from(
    new Set(
      rows
        .filter((row) => row.enabled && rowUsage(row) === 'field' && row.name.trim())
        .map((row) => row.name.trim()),
    ),
  )
}

export function markerRowOptions(rows) {
  return rows
    .filter((row) => row.enabled && rowUsage(row) === 'field' && isMarkerType(row.type))
    .map((row) => ({
      rowId: row.rowId,
      label: `${row.name || '未命名'} · ${row.optionLabel || row.text}`,
    }))
}

export function buildOptionalRuleSummaries(rows) {
  const items = []
  for (const [index, row] of rows.entries()) {
    if (!row.enabled) continue
    const usage = rowUsage(row)
    if (usage === 'prefix' || usage === 'suffix') {
      items.push({
        key: `${row.rowId}:${usage}:${index}`,
        target: row.name || '未指定字段',
        description: `${usage === 'prefix' ? prefixSummaryAction(row) : '字段为空时删除后缀'}："${row.text}"`,
      })
    } else if (rowUsage(row) === 'field' && row.optionalWhenEmpty) {
      const parts = []
      if (row.optionalPrefix) parts.push(`前缀"${row.optionalPrefix}"`)
      if (row.optionalSuffix) parts.push(`后缀"${row.optionalSuffix}"`)
      items.push({
        key: `${row.rowId}:optional:${index}`,
        target: row.name || '未命名字段',
        description: `字段为空时删除${parts.join('、') || '周围文字'}（${row.optionalScope === 'field' ? '全部同名' : '仅此位置'}）`,
      })
    }
  }
  return items
}

function prefixSummaryAction(row) {
  if (isConnectorRow(row)) return '字段为空时删除连接符'
  return '字段为空时删除前缀'
}

export function buildPreviewSampleFields(rows) {
  const fields = []
  const seen = new Set()
  for (const row of rows) {
    if (!row.enabled || rowUsage(row) !== 'field' || isMarkerType(row.type)) continue
    if (row.type === 'reference') continue
    if (!row.name?.trim() || seen.has(row.name.trim())) continue
    seen.add(row.name.trim())
    fields.push({
      name: row.name.trim(),
      label: previewFieldLabel(row),
      type: row.type,
    })
  }
  return fields
}

// ── Reference suggestion helpers ───────────────────────────────────────────

function findReferenceTarget(row, text, rowIndex, fieldRows) {
  const previousRows = fieldRows.slice(0, rowIndex).reverse()
  const partyTotalsBeforeRow = partySourceTotalsBefore(rowIndex, fieldRows)
  for (const item of previousRows) {
    if (!item.enabled || rowUsage(item) !== 'field' || isMarkerType(item.type)) continue
    if (normalizeComparableText(item.text) === text) {
      return {
        row: item,
        kind: 'field',
        label: item.name || item.label || item.text,
      }
    }
    if (item.type === 'party_list') {
      const itemIndex = (item.partyItems || []).findIndex((party) => normalizeComparableText(party) === text)
      if (itemIndex >= 0) {
        const sourceIndex = partySourceIndexForRow(item, rowIndex, partyTotalsBeforeRow, fieldRows) + itemIndex
        return {
          row: item,
          kind: 'party_item',
          sourceIndex,
          label: `${item.name || item.label || '当事人列表'} · 第 ${sourceIndex + 1} 项`,
        }
      }
    }
  }
  return null
}

function partySourceTotalsBefore(rowIndex, fieldRows) {
  const totals = new Map()
  for (const item of fieldRows.slice(0, Math.max(0, rowIndex))) {
    if (!item.enabled || rowUsage(item) !== 'field' || item.type !== 'party_list') continue
    const name = item.name?.trim()
    if (!name) continue
    totals.set(name, (totals.get(name) || 0) + Math.max(1, item.partyItems?.length || 0))
  }
  return totals
}

function partySourceIndexForRow(row, rowIndex, totalsBeforeRow, fieldRows) {
  const name = row?.name?.trim()
  if (!name) return 0
  let cursor = (totalsBeforeRow || partySourceTotalsBefore(rowIndex, fieldRows)).get(name) || 0
  for (let index = Math.max(0, rowIndex) - 1; index >= 0; index -= 1) {
    const item = fieldRows[index]
    if (item === row) return cursor - Math.max(1, item.partyItems?.length || 0)
    if (!item.enabled || rowUsage(item) !== 'field' || item.type !== 'party_list' || item.name?.trim() !== name)
      continue
    cursor -= Math.max(1, item.partyItems?.length || 0)
  }
  return 0
}

function adjacentStructureRows(row, usage, fieldRows) {
  const index = fieldRows.indexOf(row)
  if (index < 0) return []
  const direction = usage === 'prefix' ? -1 : 1
  const rows = []
  for (let cursor = index + direction; cursor >= 0 && cursor < fieldRows.length; cursor += direction) {
    const candidate = fieldRows[cursor]
    if (rowUsage(candidate) !== usage) break
    rows.push(candidate)
  }
  return usage === 'prefix' ? rows.reverse() : rows
}

export function referenceSuggestion(row, fieldRows) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  const text = normalizeComparableText(row.text)
  if (!text) return null
  const rowIndex = fieldRows.indexOf(row)
  if (rowIndex <= 0) return null
  const target = findReferenceTarget(row, text, rowIndex, fieldRows)
  if (!target) return null
  return {
    target: target.row,
    targetLabel: target.label,
    targetKind: target.kind,
    sourceIndex: target.sourceIndex,
    prefixRows: adjacentStructureRows(row, 'prefix', fieldRows),
    suffixRows: adjacentStructureRows(row, 'suffix', fieldRows),
  }
}

export function referenceSourceOptions(row, fieldRows) {
  if (!row || rowUsage(row) !== 'field' || row.type !== 'reference') return []
  const options = [
    { key: referenceSourceKey('auto'), label: '填写时选择' },
  ]
  if (row.semanticKey?.trim()) {
    options.push({ key: referenceSourceKey('semantic', row.semanticKey.trim()), label: `通用字段名：${row.semanticKey.trim()}` })
  }
  const rowIndex = fieldRows.indexOf(row)
  const seen = new Set()
  for (let i = 0; i < rowIndex; i += 1) {
    const item = fieldRows[i]
    if (!item.enabled || rowUsage(item) !== 'field' || isMarkerType(item.type)) continue
    const name = (item.name || '').trim()
    if (!name || seen.has(name)) continue
    seen.add(name)
    if (item.type === 'party_list' && item.partyItems?.length > 1) {
      for (let p = 0; p < item.partyItems.length; p += 1) {
        options.push({
          key: referenceSourceKey('field', name, p),
          label: `${item.label || name} · 第 ${p + 1} 项`,
        })
      }
    } else {
      options.push({
        key: referenceSourceKey('field', name),
        label: item.label || name,
      })
    }
  }
  return options
}

export function allReferenceSuggestions(fieldRows) {
  return fieldRows.filter((row) => referenceSuggestion(row, fieldRows))
}

export function hasUnseenReferenceSuggestion(row, fieldRows) {
  const suggestion = referenceSuggestion(row, fieldRows)
  return suggestion && !row.referenceHintSeen
}

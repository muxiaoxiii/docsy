import { toChineseNumber } from '../../../core/numberFormat.js'

const CIRCLED = ['', '①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨', '⑩', '⑪', '⑫', '⑬', '⑭', '⑮', '⑯', '⑰', '⑱', '⑲', '⑳']
const DINGBAT = ['', '❶', '❷', '❸', '❹', '❺', '❻', '❼', '❽', '❾', '❿', '⓫', '⓬', '⓭', '⓮', '⓯', '⓰', '⓱', '⓲', '⓳', '⓴']

export const PAGE_NUMBER_STYLES = [
  { value: 'arabic', label: '数字', sample: '1, 2, 3' },
  { value: 'chinese', label: '中文数字', sample: '一, 二, 三' },
  { value: 'roman-upper', label: '大写罗马数字', sample: 'I, II, III' },
  { value: 'roman-lower', label: '小写罗马数字', sample: 'i, ii, iii' },
  { value: 'circled', label: '带圈数字', sample: '①, ②, ③（1-20）', max: 20 },
  { value: 'dingbat', label: '实心带圈数字', sample: '❶, ❷, ❸（1-20）', max: 20 },
]

export function formatPageNumber(value, style = 'arabic') {
  const number = Math.max(1, Math.trunc(Number(value) || 1))
  if (style === 'chinese') return toChineseNumber(number)
  if (style === 'roman-upper') return toRoman(number)
  if (style === 'roman-lower') return toRoman(number).toLowerCase()
  if (style === 'circled') return CIRCLED[number] || String(number)
  if (style === 'dingbat') return DINGBAT[number] || String(number)
  return String(number)
}

export function renderPageNumberTemplate(template, page, total, style = 'arabic') {
  const formattedPage = formatPageNumber(page, style)
  const formattedTotal = formatPageNumber(total, style)
  return String(template || '{page}')
    .replaceAll('{page}', formattedPage)
    .replaceAll('{total}', formattedTotal)
    .replaceAll('{range}', `${formattedPage}/${formattedTotal}`)
}

export function normalizePageNumberException(rule, index = 0) {
  if (rule?.scope && typeof rule.scope === 'object') {
    return {
      id: rule.id || `exception-${Date.now()}-${index}`,
      scope: {
        type: rule.scope.type || 'global',
        start: Math.max(1, Number(rule.scope.start) || 1),
        end: Math.max(1, Number(rule.scope.end || rule.scope.start) || 1),
        fileIds: [...(rule.scope.fileIds || [])],
      },
      overrides: { ...(rule.overrides || {}) },
    }
  }
  const overrides = {}
  if (rule?.action === 'exclude') overrides.enabled = false
  for (const key of ['style', 'template', 'align', 'region', 'marginMm', 'offsetXMm', 'fontSize', 'fontFamily', 'color', 'startOffset', 'count']) {
    if (rule?.[key] !== undefined && rule?.[key] !== null && rule?.[key] !== '') overrides[key] = rule[key]
  }
  return {
    id: rule?.id || `exception-${Date.now()}-${index}`,
    scope: {
      type: rule?.scope === 'file' ? 'file' : 'global',
      start: Math.max(1, Number(rule?.start) || 1),
      end: Math.max(1, Number(rule?.end || rule?.start) || 1),
      fileIds: [...(rule?.fileIds || [])],
    },
    overrides,
  }
}

function fileStableId(file) {
  return String(file?.id || file?.path || '')
}

// 例外是否作用于指定文件（仅看 fileIds，不看页范围）
export function exceptionTargetsFile(exception, file) {
  const fileIds = exception?.scope?.fileIds || []
  return !fileIds.length || fileIds.includes(fileStableId(file))
}

export function exceptionMatches(exception, file, globalPage, localPage) {
  if (!exceptionTargetsFile(exception, file)) return false
  const scope = exception.scope || {}
  const coordinate = scope.type === 'file' ? localPage : globalPage
  return coordinate >= Number(scope.start || 1) && coordinate <= Number(scope.end || scope.start || 1)
}

/**
 * 统一插入例外：页眉/页脚文字/页码共用一张列表，kinds 勾选决定作用类型。
 * 旧数据没有 kinds 字段时按页码例外处理（它们原本只存在于页码规则上）。
 */
export function normalizeInsertException(entry, index = 0) {
  const base = normalizePageNumberException(entry, index)
  const kinds = Array.isArray(entry?.kinds) && entry.kinds.length ? [...entry.kinds] : ['pageNumber']
  return { ...base, kinds }
}

export function exceptionsForKind(exceptions, kind) {
  return (exceptions || []).map((entry, index) => normalizeInsertException(entry, index)).filter((entry) => entry.kinds.includes(kind))
}

export function effectivePageNumberRule(baseRule, globalPage, localPage, file = null) {
  const exceptions = (baseRule?.exceptions || baseRule?.overrides || []).map(normalizePageNumberException)
  return exceptions
    .filter((entry) => exceptionMatches(entry, file, globalPage, localPage))
    .reduce((result, entry) => ({ ...result, ...entry.overrides }), { ...baseRule })
}

function documentPages(baseRule, currentFile) {
  const files = Array.isArray(baseRule.allFiles) && baseRule.allFiles.length ? baseRule.allFiles : [currentFile]
  return files.flatMap((file) => {
    const start = Number(file.pageStart || 1)
    return Array.from({ length: Number(file.pages || 0) }, (_, index) => ({ file, localPage: index + 1, globalPage: start + index }))
  })
}

function pageCountsForNumbering(baseRule, currentFile) {
  const allPages = documentPages(baseRule, currentFile)
  const localPages = allPages.filter((entry) => fileStableId(entry.file) === fileStableId(currentFile))
  const totalPages = baseRule.totalMode === 'combined'
    ? allPages
    : localPages
  const counted = totalPages.filter((entry) => pageCounts(entry, baseRule))
  return { allPages, counted }
}

function pageCounts(entry, baseRule) {
  const rule = effectivePageNumberRule(baseRule, entry.globalPage, entry.localPage, entry.file)
  const hidden = rule.enabled === false || rule.action === 'exclude'
  // 隐藏页是否参与编号由例外自身的 count 决定（默认计数）
  return !hidden || rule.count !== false
}

export function pageNumberOverlaysForFile(file, baseRule) {
  if (!baseRule?.enabled || !Number(file?.pages || 0)) return []
  const continuous = baseRule.sequence !== 'per-file'
  const pageNumberStart = Math.max(1, Number(baseRule.pageNumberStart) || 1)
  const { allPages, counted } = pageCountsForNumbering(baseRule, file)
  const total = pageNumberStart + Math.max(0, counted.length - 1)
  const overlays = []
  let active = null
  let countedBefore = continuous
    ? allPages.filter((entry) => entry.globalPage < Number(file.pageStart || 1) && pageCounts(entry, baseRule)).length
    : 0
  for (let localPage = 1; localPage <= Number(file.pages); localPage += 1) {
    const globalPage = Number(file.pageStart || 1) + localPage - 1
    const rule = effectivePageNumberRule(baseRule, globalPage, localPage, file)
    const excluded = rule.enabled === false || rule.action === 'exclude'
    const countsCurrent = !excluded || rule.count !== false
    const number = pageNumberStart + countedBefore + Number(rule.startOffset || 0)
    const signature = excluded ? 'exclude' : JSON.stringify({
      template: rule.template, style: rule.style, align: rule.align, region: rule.region,
      marginMm: rule.marginMm, offsetXMm: rule.offsetXMm, fontSize: rule.fontSize,
      fontFamily: rule.fontFamily, color: rule.color, startOffset: rule.startOffset || 0, total,
    })
    if (active && active.signature === signature && active.numberEnd + 1 === number) {
      active.pageEnd = localPage
      active.numberEnd = number
      if (countsCurrent) countedBefore += 1
      continue
    }
    if (active && !active.excluded) overlays.push(toOverlay(active, total, continuous))
    active = { signature, excluded, rule, pageStart: localPage, pageEnd: localPage, globalStart: globalPage, numberStart: number, numberEnd: number }
    if (countsCurrent) countedBefore += 1
  }
  if (active && !active.excluded) overlays.push(toOverlay(active, total, continuous))
  return overlays
}

function toOverlay(group, total, continuous) {
  const offset = group.numberStart - group.globalStart
  return {
    text: group.rule.template || '{page}/{total}', region: group.rule.region || 'footer', artifactKind: 'PageNumber',
    sequence: continuous ? 'continuous' : 'per-file', numberStyle: group.rule.style || 'arabic',
    numberOffset: offset, numberTotal: total, pageStart: group.pageStart, pageEnd: group.pageEnd,
    align: group.rule.align || 'center', fontSize: Number(group.rule.fontSize || 9),
    fontFamily: group.rule.fontFamily || 'auto', marginMm: Number(group.rule.marginMm || 10),
    offsetXMm: Number(group.rule.offsetXMm || 0), color: group.rule.color || '#000000',
  }
}

function toRoman(value) {
  const pairs = [[1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'], [100, 'C'], [90, 'XC'], [50, 'L'], [40, 'XL'], [10, 'X'], [9, 'IX'], [5, 'V'], [4, 'IV'], [1, 'I']]
  let remaining = value
  let result = ''
  for (const [amount, token] of pairs) {
    while (remaining >= amount) { result += token; remaining -= amount }
  }
  return result
}

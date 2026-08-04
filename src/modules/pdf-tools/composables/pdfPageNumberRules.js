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

export function effectivePageNumberRule(baseRule, globalPage, localPage) {
  const base = { ...baseRule }
  const matching = (baseRule?.overrides || []).filter((rule) => {
    const coordinate = rule.scope === 'file' ? localPage : globalPage
    return coordinate >= Number(rule.start || 1) && coordinate <= Number(rule.end || rule.start || 1)
  })
  return matching.reduce((result, override) => ({ ...result, ...override }), base)
}

export function pageNumberOverlaysForFile(file, baseRule) {
  if (!baseRule?.enabled || !Number(file?.pages || 0)) return []
  const continuous = baseRule.sequence !== 'per-file'
  const total = continuous ? Number(baseRule.totalPages || file.pages) : Number(file.pages)
  const overlays = []
  let active = null
  for (let localPage = 1; localPage <= Number(file.pages); localPage += 1) {
    const globalPage = Number(file.pageStart || 1) + localPage - 1
    const rule = effectivePageNumberRule(baseRule, globalPage, localPage)
    const excluded = rule.action === 'exclude' || rule.enabled === false
    const number = continuous ? globalPage + Number(rule.startOffset || 0) : localPage + Number(rule.startOffset || 0)
    const signature = excluded
      ? 'exclude'
      : JSON.stringify({
          template: rule.template,
          style: rule.style,
          align: rule.align,
          marginMm: rule.marginMm,
          offsetXMm: rule.offsetXMm,
          fontSize: rule.fontSize,
          fontFamily: rule.fontFamily,
          color: rule.color,
          startOffset: rule.startOffset || 0,
        })
    if (active && active.signature === signature && active.numberEnd + 1 === number) {
      active.pageEnd = localPage
      active.numberEnd = number
      continue
    }
    if (active && !active.excluded) overlays.push(toOverlay(active, total, continuous))
    active = {
      signature,
      excluded,
      rule,
      pageStart: localPage,
      pageEnd: localPage,
      globalStart: globalPage,
      numberStart: number,
      numberEnd: number,
    }
  }
  if (active && !active.excluded) overlays.push(toOverlay(active, total, continuous))
  return overlays
}

function toOverlay(group, total, continuous) {
  // continuous mode: backend current_page = page_start(file.pageStart) + index = globalPage
  //   numberOffset = numberStart - globalStart → page = globalPage + offset = numberStart ✓
  // per-file mode: backend current_page = page_start(1) + index = localPage
  //   numberOffset = numberStart - localPageStart = numberStart - pageStart
  const offset = continuous
    ? group.numberStart - group.globalStart
    : group.numberStart - group.pageStart
  return {
    text: group.rule.template || '{page}/{total}',
    region: group.rule.region || 'footer',
    artifactKind: 'PageNumber',
    numberStyle: group.rule.style || 'arabic',
    numberOffset: offset,
    numberTotal: total,
    pageStart: group.pageStart,
    pageEnd: group.pageEnd,
    align: group.rule.align || 'center',
    fontSize: Number(group.rule.fontSize || 9),
    fontFamily: group.rule.fontFamily || 'auto',
    marginMm: Number(group.rule.marginMm || 10),
    offsetXMm: Number(group.rule.offsetXMm || 0),
    color: group.rule.color || '#000000',
  }
}

function toRoman(value) {
  const pairs = [
    [1000, 'M'],
    [900, 'CM'],
    [500, 'D'],
    [400, 'CD'],
    [100, 'C'],
    [90, 'XC'],
    [50, 'L'],
    [40, 'XL'],
    [10, 'X'],
    [9, 'IX'],
    [5, 'V'],
    [4, 'IV'],
    [1, 'I'],
  ]
  let remaining = value
  let result = ''
  for (const [amount, token] of pairs) {
    while (remaining >= amount) {
      result += token
      remaining -= amount
    }
  }
  return result
}


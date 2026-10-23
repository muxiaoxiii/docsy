export const PAGE_NUMBER_FORMATS = [
  { label: '简洁分数', withTotal: '{page}/{total}', withoutTotal: '{page}' },
  { label: '留白分数', withTotal: '{page} / {total}', withoutTotal: '{page}' },
  { label: '中文页码', withTotal: '第{page}页，共{total}页', withoutTotal: '第{page}页' },
  { label: '中文分隔', withTotal: '第{page}页 / 共{total}页', withoutTotal: '第{page}页' },
  { label: '中文括注', withTotal: '第{page}页（共{total}页）', withoutTotal: '第{page}页' },
  { label: '英文页码', withTotal: 'Page {page} of {total}', withoutTotal: 'Page {page}' },
  { label: '简洁英文', withTotal: '{page} of {total}', withoutTotal: '{page}' },
  { label: '短横线', withTotal: '-{page}/{total}-', withoutTotal: '-{page}-' },
  { label: '长横线', withTotal: '— {page} / {total} —', withoutTotal: '— {page} —' },
  { label: '圆括号', withTotal: '（{page}/{total}）', withoutTotal: '（{page}）' },
  { label: '方括号', withTotal: '[{page}/{total}]', withoutTotal: '[{page}]' },
  { label: '页码缩写', withTotal: 'p.{page}/{total}', withoutTotal: 'p.{page}' },
]

export function pageNumberPresets(showTotal) {
  const seen = new Set()
  return PAGE_NUMBER_FORMATS.flatMap((format) => {
    const value = showTotal ? format.withTotal : format.withoutTotal
    if (seen.has(value)) return []
    seen.add(value)
    return [{ label: format.label, value }]
  })
}

export const usesPageTotal = template => template.includes('{total}') || template.includes('{range}')
export function templateForTotal(template, showTotal, previousWithTotal = '') {
  if (showTotal && usesPageTotal(template)) return template
  const previous = PAGE_NUMBER_FORMATS.find((format) => format.withTotal === previousWithTotal)
  if (
    showTotal &&
    previousWithTotal &&
    ((previous && previous.withoutTotal === template) || (!previous && template === '{page}'))
  ) {
    return previousWithTotal
  }
  const format = PAGE_NUMBER_FORMATS.find((format) => format.withTotal === template || format.withoutTotal === template)
  if (format) return showTotal ? format.withTotal : format.withoutTotal
  // A custom template can contain arbitrary prose. Removing only {total} would
  // leave dangling text such as “共页”; keep the full version for a later restore.
  if (!showTotal && usesPageTotal(template)) return '{page}'
  return showTotal ? previousWithTotal || '{page}/{total}' : template
}

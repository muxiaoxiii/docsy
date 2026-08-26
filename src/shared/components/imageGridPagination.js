function clampInteger(value, minimum = 1) {
  const number = Math.floor(Number(value) || 0)
  return Math.max(minimum, number)
}

export const PAGE_FRACTIONS = Object.freeze([0.25, 0.5, 0.75, 1])

export function percentagePageSizeOptions(totalItems) {
  const total = Math.max(0, Math.floor(Number(totalItems) || 0))
  if (!total) return [{ fraction: 1, size: 1, label: '1 张' }]
  const seen = new Set()
  return PAGE_FRACTIONS.map((fraction) => {
    const size = Math.max(1, Math.ceil(total * fraction))
    return { fraction, size, label: `${size} 张` }
  }).filter((option) => {
    if (seen.has(option.size)) return false
    seen.add(option.size)
    return true
  })
}

export function pageSizeForFraction(totalItems, fraction) {
  const options = percentagePageSizeOptions(totalItems)
  const requested = Number(fraction)
  return options.reduce((best, option) =>
    Math.abs(option.fraction - requested) < Math.abs(best.fraction - requested) ? option : best,
  ).size
}

export function pageRangeForSize(totalItems, pageSize, requestedPage = 1) {
  const total = Math.max(0, Math.floor(Number(totalItems) || 0))
  const size = clampInteger(pageSize)
  const pageCount = Math.max(1, Math.ceil(total / size))
  const page = Math.min(pageCount, clampInteger(requestedPage))
  const start = Math.min(total, (page - 1) * size)
  const end = Math.min(total, start + size)
  return { page, pageCount, start, end, size: end - start }
}

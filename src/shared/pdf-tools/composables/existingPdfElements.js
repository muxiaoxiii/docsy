import { naturalCompare } from './useEvidencePdfSession.js'

export const EXISTING_ELEMENT_KINDS = ['header', 'footerText', 'pageNumber']
export const EXISTING_ELEMENT_DECISIONS = ['keep', 'ignore', 'delete', 'edit']

export function detectedElementFromCandidate(candidate, kind, index = 0) {
  const range = candidate?.pageRange || {}
  const text = String(candidate?.text || candidate?.normalizedText || '')
  const confidence = Number(candidate?.confidence || 0)
  const source = candidate?.source || 'content-text'
  const pageStart = Number(range.start || candidate?.bbox?.page || 1)
  const pageEnd = Number(range.end || range.start || candidate?.bbox?.page || 1)
  // Low-confidence: either the score is below threshold, or this is a
  // single-page file where content-text heuristics have no repetition to
  // validate (page-number candidates from single pages are still useful for
  // cross-file sequence assembly, so they are excluded).
  const isPageNumber = (candidate?.labels || []).includes('page-number')
  const isSinglePageContentText = source === 'content-text' && pageStart === pageEnd && !isPageNumber
  const lowConfidence = confidence < 0.3 || isSinglePageContentText
  return {
    id: `${kind}|${candidateIdentity(candidate)}|${index}`,
    kind,
    detectedText: text,
    editedText: text,
    normalizedText: String(candidate?.normalizedText || text),
    source,
    artifactId: candidate?.artifactId || null,
    docsyKind: candidate?.docsyKind || null,
    bbox: candidate?.bbox || null,
    fontSize: Number(candidate?.fontSize || 0) || null,
    pageStart,
    pageEnd,
    count: Number(candidate?.count || 0),
    confidence,
    lowConfidence,
    labels: [...(candidate?.labels || [])],
    sequenceForm: candidate?.sequenceForm || null,
    hasTotal: Boolean(candidate?.hasTotal),
    decision: null,
    reasons: [...(candidate?.reasons || [])],
  }
}

export function candidateIdentity(candidate) {
  if (!candidate) return ''
  if (candidate.candidateKey) return candidate.candidateKey
  const bbox = candidate.bbox || {}
  const range = candidate.pageRange || {}
  return [
    candidate.region || 'footer',
    candidate.normalizedText || candidate.text || '',
    range.start || bbox.page || 1,
    range.end || range.start || bbox.page || 1,
    Math.round(Number(bbox.x0 || 0)),
    Math.round(Number(bbox.y0 || 0)),
    Math.round(Number(bbox.x1 || 0)),
    Math.round(Number(bbox.y1 || 0)),
  ].join('|')
}

export function mergeExistingElements(previous = [], detected = []) {
  const previousByIdentity = new Map(previous.map((element) => [elementIdentity(element), element]))
  return detected.map((element) => {
    const existing = previousByIdentity.get(elementIdentity(element))
    if (!existing) return element
    return {
      ...element,
      decision: existing.decision ?? null,
      editedText: existing.editedText || element.detectedText,
    }
  })
}

export function elementIdentity(element) {
  return [
    element?.kind || '',
    element?.normalizedText || element?.detectedText || '',
    Number(element?.pageStart || 0),
    Number(element?.pageEnd || 0),
    Math.round(Number(element?.bbox?.x0 || 0)),
    Math.round(Number(element?.bbox?.y0 || 0)),
  ].join('|')
}

export function elementDecisionText(decision) {
  return (
    {
      keep: '保留',
      ignore: '忽略识别',
      delete: '待删除',
      edit: '待编辑',
    }[decision] || '待确认'
  )
}

export function elementKindText(kind) {
  return { header: '页眉', footerText: '页脚文字', pageNumber: '页码' }[kind] || '未知'
}

export function actionableExistingElements(file, kinds = EXISTING_ELEMENT_KINDS) {
  return (file?.existingElements || []).filter(
    (element) => kinds.includes(element.kind) && ['delete', 'edit'].includes(element.decision),
  )
}

export function mergeRangeSelection(selectedKeys, rows, startIndex, endIndex) {
  const start = Math.max(0, Math.min(startIndex, endIndex))
  const end = Math.min(rows.length - 1, Math.max(startIndex, endIndex))
  const selected = new Set(selectedKeys)
  for (const row of rows.slice(start, end + 1)) {
    if (row?.key) selected.add(row.key)
  }
  return [...selected]
}

// 检测文字排序：拉丁字母开头的文本（专利号等）排在最前，其余按自然序；
// 同文本再按文件名、页段稳定次序，保证“证据X / 证据X译文”这类同族文本相邻。
export function compareDetectedTextRows(a, b) {
  const textA = a.element?.detectedText || ''
  const textB = b.element?.detectedText || ''
  const groupA = /^[A-Za-z]/.test(textA) ? 0 : 1
  const groupB = /^[A-Za-z]/.test(textB) ? 0 : 1
  if (groupA !== groupB) return groupA - groupB
  const byText = naturalCompare(textA, textB)
  if (byText !== 0) return byText
  const byFile = naturalCompare(a.fileName || '', b.fileName || '')
  if (byFile !== 0) return byFile
  return (a.element?.pageStart || 0) - (b.element?.pageStart || 0)
}

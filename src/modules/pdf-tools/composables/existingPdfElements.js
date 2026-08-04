export const EXISTING_ELEMENT_KINDS = ['header', 'footerText', 'pageNumber']
export const EXISTING_ELEMENT_DECISIONS = ['keep', 'ignore', 'delete', 'edit']

export function detectedElementFromCandidate(candidate, kind, index = 0) {
  const range = candidate?.pageRange || {}
  const text = String(candidate?.text || candidate?.normalizedText || '')
  return {
    id: `${kind}|${candidateIdentity(candidate)}|${index}`,
    kind,
    detectedText: text,
    editedText: text,
    normalizedText: String(candidate?.normalizedText || text),
    source: candidate?.source || 'content-text',
    artifactId: candidate?.artifactId || null,
    docsyKind: candidate?.docsyKind || null,
    bbox: candidate?.bbox || null,
    fontSize: Number(candidate?.fontSize || 0) || null,
    pageStart: Number(range.start || candidate?.bbox?.page || 1),
    pageEnd: Number(range.end || range.start || candidate?.bbox?.page || 1),
    count: Number(candidate?.count || 0),
    confidence: Number(candidate?.confidence || 0),
    labels: [...(candidate?.labels || [])],
    sequenceForm: candidate?.sequenceForm || null,
    hasTotal: Boolean(candidate?.hasTotal),
    decision: 'keep',
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
      decision: existing.decision || 'keep',
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
    }[decision] || '保留'
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

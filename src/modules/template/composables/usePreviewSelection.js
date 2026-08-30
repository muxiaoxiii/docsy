/**
 * Composable for preview selection state and logic in the build tab.
 */
import { ref } from 'vue'
import {
  charLength,
  sliceChars,
  rowUsage,
  previewRunSegment,
  previewPlainSegment,
  previewSourceLabel,
  previewReplacementText,
  previewRangesByRun,
  markSegmentsFromRefs,
  defaultCheckedText,
  defaultUncheckedText,
  splitPartyLabelText,
  referenceSourceKey,
  typeLabel,
  fieldIsMultiple,
} from './fieldRowUtils.js'
import { inferFieldFromText } from './useFieldNormalization.js'

export function usePreviewSelection(documentRuns, documentText, fieldRows, _previewSampleValues) {
  const sourcePreviewRef = ref(null)
  const documentPreviewRef = ref(null)
  const sourcePreviewSelection = ref(null)
  const sourcePreviewSelectionPayload = ref(null)
  const previewFocusedRowId = ref('')
  let lastPreviewAddKey = ''
  let lastPreviewAddAt = 0

  function buildTemplatePreview(runs, fallbackText, rows, sampleValues = {}) {
    if (!runs?.length) return buildTextFallbackPreview(fallbackText)
    const original = []
    const rendered = []
    let segmentIndex = 0
    let lastParagraph = null
    const rangesByRun = previewRangesByRun(rows)
    const rangesByStoredTag = storedTemplateRangesByTag(rows)
    for (const run of runs) {
      if (lastParagraph !== null && run.paragraphIndex !== lastParagraph) {
        original.push(previewPlainSegment('\n', segmentIndex++))
        rendered.push(previewPlainSegment('\n', segmentIndex++))
      }
      lastParagraph = run.paragraphIndex
      const runText = String(run.text || '')
      const storedTag = storedTemplatePlaceholderTag(runText)
      // 编辑已保存模板时以 sdt tag 为稳定身份。保存过程可能因拆分
      // 前后缀而改变后续 runIndex，旧 markId 只适用于初次导入的源 Word。
      const ranges = storedTag
        ? rangesByStoredTag.get(storedTag) || []
        : (rangesByRun.get(run.id) || []).filter((range) => !range.row._fromManifestRef)
      let cursor = 0
      for (const range of ranges) {
        // 已保存的 docsytpl 会把原文字替换为 `{{field.ref.N}}`。manifest
        // 中的 start/end 仍是第一次导入 Word 时的原文字范围（通常只有
        // 2–4 个字），若继续按旧范围截取，`.ref.1}}` 会作为正文泄漏到
        // “渲染示意”。命中字段所在 run 且整段是内部占位符时，应覆盖
        // 整个 run；普通 Word 原文和用户输入的花括号文本不受影响。
        const storedPlaceholder = Boolean(storedTag)
        const start = storedPlaceholder ? 0 : Math.max(0, Math.min(charLength(runText), range.start ?? 0))
        const end = storedPlaceholder
          ? charLength(runText)
          : Math.max(start, Math.min(charLength(runText), range.end ?? charLength(runText)))
        if (end <= cursor) continue
        const visibleStart = Math.max(cursor, start)
        if (cursor < visibleStart) {
          const plain = previewRunSegment(run, cursor, visibleStart, null, segmentIndex++)
          original.push(plain)
          rendered.push({ ...plain, id: `rendered-${plain.id}` })
        }
        const marked = previewRunSegment(run, visibleStart, end, range.row, segmentIndex++)
        original.push({
          ...marked,
          text: range.occurrence === 0 ? previewSourceLabel(range.row) : '',
          deleted: range.occurrence > 0,
        })
        const previewEachPartyItem = fieldIsMultiple(range.row) && Array.isArray(sampleValues[range.row.name])
        let replacementText =
          range.occurrence === 0 || previewEachPartyItem
            ? previewReplacementText(range.row, sampleValues, range.occurrence)
            : ''
        // 已保存模板没有示例值时，previewReplacementText 会回退到字段
        // 原文，形成“请求人请求人”这类看似重复的正文。明确显示待填标记，
        // 既不暴露内部代码，也不会冒充最终渲染内容。
        if (storedPlaceholder && replacementText === range.row.text) {
          replacementText = `【待填：${range.row.label || range.row.name || '字段'}】`
        }
        rendered.push({
          ...marked,
          id: `rendered-${marked.id}`,
          text: replacementText,
          deleted: rowUsage(range.row) === 'delete_text' || !replacementText || range.occurrence > 0,
        })
        cursor = end
      }
      if (cursor < charLength(runText)) {
        const tail = previewRunSegment(run, cursor, charLength(runText), null, segmentIndex++)
        original.push(tail)
        rendered.push({ ...tail, id: `rendered-${tail.id}` })
      }
    }
    return { original, rendered }
  }

  function storedTemplatePlaceholderTag(text) {
    return /^\{\{([A-Za-z0-9_.:-]+)\}\}$/.exec(String(text || '').trim())?.[1] || ''
  }

  function storedTemplateRangesByTag(rows) {
    const map = new Map()
    for (const row of rows || []) {
      if (!row.enabled) continue
      for (const [occurrence, markRef] of (row.markRefs || []).entries()) {
        const tag = String(markRef?.tag || '').trim()
        if (!tag) continue
        map.set(tag, [{ row, start: 0, end: undefined, occurrence }])
      }
    }
    return map
  }

  function buildTextFallbackPreview(text) {
    const source = String(text || '')
    if (!source) return { original: [], rendered: [] }
    return { original: [previewPlainSegment(source, 0)], rendered: [previewPlainSegment(source, 1)] }
  }

  function rememberSourcePreviewSelection() {
    const selection = window.getSelection?.()
    if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return
    const range = selection.getRangeAt(0)
    const resolved = resolvePreviewSelection(range)
    if (!resolved.text) return
    sourcePreviewSelection.value = range.cloneRange()
    sourcePreviewSelectionPayload.value = resolved
  }

  function collectSourcePreviewSelection() {
    const selection = window.getSelection?.()
    if (!selection || selection.rangeCount === 0) return

    if (!selection.isCollapsed) {
      const liveRange = selection.getRangeAt(0)
      const resolved = resolvePreviewSelection(liveRange)
      if (resolved.refs.length) {
        sourcePreviewSelection.value = liveRange.cloneRange()
        sourcePreviewSelectionPayload.value = resolved
        return resolved
      }
    }

    if (sourcePreviewSelection.value) {
      const resolved = resolvePreviewSelection(sourcePreviewSelection.value)
      if (resolved.refs.length) {
        sourcePreviewSelectionPayload.value = resolved
        return resolved
      }
    }

    return sourcePreviewSelectionPayload.value || { text: '', refs: [], context: '' }
  }

  function resolvePreviewSelection(range) {
    const sourceRoot = previewElement(sourcePreviewRef.value)
    if (sourceRoot && rangeIntersectsRoot(range, sourceRoot)) return resolveSourcePreviewRange(sourceRoot, range)
    const documentRoot = previewElement(documentPreviewRef.value)
    if (documentRoot && rangeIntersectsRoot(range, documentRoot))
      return resolveDocumentPreviewRange(documentRoot, range)

    // A component ref can briefly be unavailable while the preview is being
    // expanded. Resolve the nearest preview root from the live selection as a
    // fallback so the action button does not lose an otherwise valid range.
    const selectedNode = previewSelectionNode(range)
    const fallbackRoot = selectedNode?.closest?.('.source-preview-text, .document-preview')
    if (fallbackRoot?.classList?.contains('source-preview-text')) {
      return resolveSourcePreviewRange(fallbackRoot, range)
    }
    if (fallbackRoot?.classList?.contains('document-preview')) {
      return resolveDocumentPreviewRange(fallbackRoot, range)
    }
    return { text: '', refs: [], context: '' }
  }

  function previewElement(value) {
    if (value?.nodeType === 1 || value?.nodeType === 9) return value
    if (value?.value && value.value !== value) return previewElement(value.value)
    return null
  }

  function previewSelectionNode(range) {
    const node = range?.commonAncestorContainer
    if (!node) return null
    return node.nodeType === 1 ? node : node.parentElement
  }

  function resolveSourcePreviewRange(root, range) {
    const stream = sourcePreviewTextStream(root)
    const selected = sourcePreviewSelectionParts(root, range)
    const text = selected.text || range.toString()
    if (!text) return { text: '', refs: [], context: '' }
    return {
      text,
      refs: selected.refs,
      context: documentText.value || stream.text || text,
    }
  }

  function resolveDocumentPreviewRange(root, range) {
    const stream = documentPreviewTextStream()
    if (!stream.text) return { text: '', refs: [], context: '' }
    const start = previewRootOffset(root, range, true)
    const end = previewRootOffset(root, range, false)
    const streamLength = charLength(stream.text)
    const normalizedStart = Math.max(0, Math.min(streamLength, Math.min(start, end)))
    const normalizedEnd = Math.max(normalizedStart, Math.min(streamLength, Math.max(start, end)))
    const text = sliceChars(stream.text, normalizedStart, normalizedEnd)
    const refs = refsForPreviewTextStreamRange(stream, normalizedStart, normalizedEnd)
    const fallbackText = text || range.toString()
    if (!fallbackText) return { text: '', refs: [], context: '' }
    return {
      text: fallbackText,
      refs,
      context: documentText.value || stream.text || fallbackText,
    }
  }

  function documentPreviewTextStream() {
    const chunks = []
    let text = ''
    let cursor = 0
    let lastParagraph = null
    for (const run of documentRuns.value || []) {
      if (lastParagraph !== null && run.paragraphIndex !== lastParagraph) {
        text += '\n'
        cursor += 1
      }
      lastParagraph = run.paragraphIndex
      const value = String(run.text || '')
      const start = cursor
      const end = start + charLength(value)
      chunks.push({
        text: value,
        streamStart: start,
        streamEnd: end,
        runId: run.id,
        runStart: 0,
      })
      text += value
      cursor = end
    }
    return { text, chunks }
  }

  function previewRootOffset(root, range, useStart) {
    const container = useStart ? range.startContainer : range.endContainer
    const offset = useStart ? range.startOffset : range.endOffset
    if (!root.contains(container)) return useStart ? 0 : charLength(root.textContent || '')
    const before = document.createRange()
    before.selectNodeContents(root)
    before.setEnd(container, offset)
    return charLength(before.toString())
  }

  function rangeIntersectsRoot(range, root) {
    if (!root?.contains) return false
    if (root.contains(range.commonAncestorContainer)) return true
    return root.contains(range.startContainer) || root.contains(range.endContainer)
  }

  function sourcePreviewTextStream(root) {
    const chunks = []
    let text = ''
    let cursor = 0
    for (const node of root.querySelectorAll('[data-run-id]')) {
      const value = node.textContent || ''
      const start = cursor
      const end = start + charLength(value)
      chunks.push({
        node,
        text: value,
        streamStart: start,
        streamEnd: end,
        runId: node.dataset.runId,
        runStart: Number(node.dataset.start || 0),
      })
      text += value
      cursor = end
    }
    return { text, chunks }
  }

  function refsForPreviewTextStreamRange(stream, start, end) {
    const refs = []
    for (const chunk of stream.chunks) {
      if (!chunk.runId) continue
      const overlapStart = Math.max(start, chunk.streamStart)
      const overlapEnd = Math.min(end, chunk.streamEnd)
      if (overlapEnd <= overlapStart) continue
      refs.push({
        markId: chunk.runId,
        start: chunk.runStart + overlapStart - chunk.streamStart,
        end: chunk.runStart + overlapEnd - chunk.streamStart,
      })
    }
    return refs
  }

  function sourcePreviewSelectionParts(root, range) {
    const refs = []
    const texts = []
    for (const node of root.querySelectorAll('[data-run-id]')) {
      if (!rangeIntersectsNode(range, node)) continue
      const part = selectedNodeTextPart(range, node)
      if (!part.text) continue
      const runStart = Number(node.dataset.start || 0)
      refs.push({
        markId: node.dataset.runId,
        start: runStart + part.start,
        end: runStart + part.end,
      })
      texts.push(part.text)
    }
    return { text: texts.join(''), refs }
  }

  function rangeIntersectsNode(range, node) {
    try {
      return range.intersectsNode(node)
    } catch {
      return false
    }
  }

  function selectedNodeTextPart(range, node) {
    const nodeRange = document.createRange()
    nodeRange.selectNodeContents(node)
    const overlap = range.cloneRange()
    if (overlap.compareBoundaryPoints(window.Range.START_TO_START, nodeRange) < 0) {
      overlap.setStart(nodeRange.startContainer, nodeRange.startOffset)
    }
    if (overlap.compareBoundaryPoints(window.Range.END_TO_END, nodeRange) > 0) {
      overlap.setEnd(nodeRange.endContainer, nodeRange.endOffset)
    }
    const text = overlap.toString()
    if (!text) return { text: '', start: 0, end: 0 }
    const before = document.createRange()
    before.selectNodeContents(node)
    before.setEnd(overlap.startContainer, overlap.startOffset)
    const start = charLength(before.toString())
    return { text, start, end: start + charLength(text) }
  }

  function triggerPreviewSelectionAdd(type) {
    const payload = sourcePreviewSelectionPayload.value
    const key = `${type}:${payload?.text || ''}:${payload?.refs?.map((ref) => `${ref.markId}:${ref.start}:${ref.end}`).join('|') || ''}`
    const now = Date.now()
    if (key && key === lastPreviewAddKey && now - lastPreviewAddAt < 350) return
    lastPreviewAddKey = key
    lastPreviewAddAt = now
    addPreviewSelection(type)
  }

  function addPreviewSelection(type) {
    const selection = collectSourcePreviewSelection()
    if (!selection.refs.length) {
      return { success: false, hasPayload: Boolean(sourcePreviewSelectionPayload.value?.text) }
    }
    const addedRows = rowsFromPreviewSelection(selection, type)
    return { success: true, rows: addedRows, selection }
  }

  function rowsFromPreviewSelection(selection, type) {
    const inferred = inferFieldFromText(selection.text, selection.context, false, fieldRows.value.length)
    return [
      createPreviewRow(selection, {
        type,
        text: selection.text,
        refs: selection.refs,
        inferred,
        rowKey: 'single',
      }),
    ]
  }

  function createPreviewRow(selection, options) {
    const usageType = options.type
    const inferred = inferFieldFromText(selection.text, selection.context, false, fieldRows.value.length)
    const inferredField = options.inferred || inferred
    const isStructure = ['prefix', 'suffix', 'delete_text', 'ignore'].includes(usageType)
    const inferredMultiple = inferredField.type === 'party_list'
    const effectiveType = isStructure
      ? usageType
      : usageType || (inferredMultiple ? 'text' : inferredField.type) || 'text'
    const manualMeta = manualFieldMeta(selection.text, effectiveType, inferredField)
    const refs = options.refs || selection.refs
    const text = options.text ?? selection.text
    const row = {
      rowId: `preview:${Date.now()}:${fieldRows.value.length}:${options.rowKey || usageType}`,
      displayId: refs.map((ref) => ref.markId).join('+'),
      markId: refs[0]?.markId,
      markRefs: refs,
      charStart: refs[0]?.start,
      charEnd: refs[0]?.end,
      text,
      context: selection.context,
      enabled: true,
      type: effectiveType,
      name: isStructure ? options.structureName || '' : manualMeta.name,
      label: isStructure ? options.structureLabel || typeLabel(usageType) : manualMeta.label,
      semanticKey: isStructure ? '' : manualMeta.semanticKey,
      groupName: isStructure ? '' : inferredMultiple ? inferredField.semanticKey || inferredField.name : '',
      multiple: inferredMultiple,
      itemSeparator: '、',
      repeatPrefix: false,
      repeatSuffix: false,
      userSelectedType: isStructure ? '' : effectiveType,
      markSegments: markSegmentsFromRefs(refs, text),
      required: false,
      optionalWhenEmpty: false,
      optionalScope: 'position',
      optionalPrefix: '',
      optionalSuffix: '',
      optionId: '',
      optionLabel: '',
      checkedText: defaultCheckedText(text),
      uncheckedText: defaultUncheckedText(text),
      partyItems: inferredMultiple ? splitPartyLabelText(text) : [],
      referenceHintSeen: true,
      referenceIncludePrefix: true,
      referenceIncludeSuffix: true,
      referenceSourceMode: 'auto',
      referenceSourceField: '',
      referenceSourceSemanticKey: '',
      referenceSourceIndex: null,
      referenceSourceKey: referenceSourceKey('auto', '', null),
    }
    return row
  }

  function manualFieldMeta(text, effectiveType, inferredField) {
    if (
      (inferredField?.type === effectiveType || (inferredField?.type === 'party_list' && effectiveType === 'text')) &&
      !['prefix', 'suffix', 'ignore', 'delete_text'].includes(effectiveType)
    ) {
      return {
        name: inferredField.name,
        label: inferredField.label,
        semanticKey: inferredField.semanticKey || inferredField.name,
      }
    }
    const fallbackName = String(text || '').trim() || `字段${fieldRows.value.length + 1}`
    if (effectiveType === 'date') {
      return { name: '日期', label: '日期', semanticKey: '日期' }
    }
    if (effectiveType === 'reference') {
      const name = nextReferenceFieldName()
      return { name, label: name, semanticKey: '' }
    }
    return { name: fallbackName, label: fallbackName, semanticKey: fallbackName }
  }

  function nextReferenceFieldName() {
    const base = '引用'
    const used = new Set(
      fieldRows.value
        .filter((row) => row.enabled && rowUsage(row) === 'field')
        .map((row) => String(row.name || '').trim())
        .filter(Boolean),
    )
    if (!used.has(base)) return base
    for (let index = 2; index < 1000; index += 1) {
      const candidate = `${base}${index}`
      if (!used.has(candidate)) return candidate
    }
    return `${base}${Date.now()}`
  }

  function focusPreviewRow(row, fieldTableRef) {
    previewFocusedRowId.value = row.rowId
    row.referenceHintSeen = true
    fieldTableRef.value?.setCurrentRow?.(row)
  }

  return {
    sourcePreviewRef,
    documentPreviewRef,
    sourcePreviewSelection,
    sourcePreviewSelectionPayload,
    previewFocusedRowId,
    buildTemplatePreview,
    rememberSourcePreviewSelection,
    collectSourcePreviewSelection,
    triggerPreviewSelectionAdd,
    addPreviewSelection,
    focusPreviewRow,
    nextReferenceFieldName,
  }
}

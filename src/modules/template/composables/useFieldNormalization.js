/**
 * Composable for field row normalization, build, and validation logic.
 * Accepts reactive state dependencies and returns the relevant functions.
 */
import { inferTemplateField, prefixTargetRule, suffixTargetRule, looksLikeDatePrefix } from '../rules/publicRules.js'
import {
  rowUsage,
  isMarkerType,
  isGeneratedFieldName,
  isConnectorRow,
  charLength,
  markRefsForTextRange,
  refsForRowTextRange,
  markSegmentsFromRefs,
  makeRangeRow,
  dedupeRangeRows,
  findNeighborFieldRow,
  bindStructureRowToTarget,
  displayNameForFieldRow,
  isGeneratedConnectorRow,
  isPureConnectorText,
  splitPartyLabelText,
  splitPartyLabelSegments,
  stableFieldId,
  normalizedReferenceSource,
  structureTargetRow,
  leadingConnectorInfo,
  knownSuffixAtEnd,
  isLikelyPrefixMark,
  isLikelySuffixMark,
  referenceSourceKey,
  defaultCheckedText,
  defaultUncheckedText,
} from './fieldRowUtils.js'

// ── normalizeFieldRows chain ─────────────────────────────────────────────────

export function normalizeFieldRows(rows, documentRuns) {
  return reorderRowsByDocumentPosition(
    refreshPartyItemsForRows(
      autoSplitPartyListRows(
        autoAssignStructureTargets(
          dropGeneratedConnectorRows(
            autoSplitLegalCompoundRows(autoSplitKnownSuffixRows(autoSplitLeadingConnectorRows(rows)))
          )
        )
      )
    ),
    documentRuns
  )
}

function reorderRowsByDocumentPosition(rows, documentRuns) {
  const runOrder = new Map()
  for (const [index, run] of (documentRuns || []).entries()) {
    runOrder.set(run.id, Number.isFinite(run.runIndex) ? run.runIndex : index)
  }
  return [...rows]
    .map((row, index) => ({ row, index }))
    .sort((a, b) => rowDocumentOrder(a.row, runOrder) - rowDocumentOrder(b.row, runOrder) || a.index - b.index)
    .map((item) => item.row)
}

function rowDocumentOrder(row, runOrder) {
  const refs = row.markRefs?.length ? row.markRefs : [{ markId: row.markId }]
  let min = Number.POSITIVE_INFINITY
  for (const ref of refs) {
    if (!ref?.markId || !runOrder.has(ref.markId)) continue
    min = Math.min(min, runOrder.get(ref.markId))
  }
  return Number.isFinite(min) ? min : Number.MAX_SAFE_INTEGER
}

function refreshPartyItemsForRows(rows) {
  for (const row of rows) {
    row.partyItems = row.type === 'party_list' ? splitPartyLabelText(row.text) : []
  }
  return rows
}

function autoSplitPartyListRows(rows) {
  const result = []
  for (const row of rows) {
    if (rowUsage(row) !== 'field' || row.type !== 'party_list') {
      result.push(row)
      continue
    }
    const segments = splitPartyLabelSegments(row.text)
    if (segments.length <= 1) {
      result.push(row)
      continue
    }
    for (const [index, segment] of segments.entries()) {
      const refs = markRefsForTextRange(row, segment.start, segment.end)
      result.push({
        ...row,
        rowId: `${row.rowId}:party-item:${index}`,
        text: segment.text,
        label: segment.text,
        markRefs: refs,
        charStart: segment.start,
        charEnd: segment.end,
        markSegments: markSegmentsFromRefs(refs, segment.text),
        partyItems: [segment.text],
      })
    }
  }
  return result
}

function autoAssignStructureTargets(rows) {
  for (const [index, row] of rows.entries()) {
    if (rowUsage(row) === 'prefix') {
      const target = findNeighborFieldRow(rows, index, 1)
      if (target) {
        const rule = prefixTargetRule(row.text)
        if (rule && isGeneratedFieldName(target.name)) {
          target.type = rule.targetType
          target.name = rule.targetName
          target.label = rule.targetLabel
          target.semanticKey = rule.targetSemanticKey || rule.targetName
        }
        bindStructureRowToTarget(row, target)
        row.name = rule?.targetName || target.name
        row.label = prefixStructureLabel(row, target)
      }
    } else if (rowUsage(row) === 'suffix') {
      const target = findNeighborFieldRow(rows, index, -1)
      if (target) {
        const rule = suffixTargetRule(row.text)
        if (rule && isGeneratedFieldName(target.name)) {
          target.type = rule.targetType
          target.name = rule.targetName
          target.label = rule.targetLabel
          target.semanticKey = rule.targetSemanticKey || rule.targetName
        }
        bindStructureRowToTarget(row, target)
        row.name = rule?.targetName || target.name
        row.label = `${target.label || target.name}后缀`
      }
    }
  }
  return rows
}

function prefixStructureLabel(row, target) {
  return `${displayNameForFieldRow(target) || target.name}前缀`
}

function dropGeneratedConnectorRows(rows) {
  return rows.filter((row) => !(isGeneratedConnectorRow(row) && isPureConnectorText(row.text)))
}

function autoSplitLeadingConnectorRows(rows) {
  const result = []
  for (const row of rows) {
    const split = splitLeadingConnectorRow(row, rows)
    if (split) result.push(...split)
    else result.push(row)
  }
  return result
}

function splitLeadingConnectorRow(row, rows) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  const info = leadingConnectorInfo(row.text)
  if (!info.connectorText || !info.restText) return null
  const connectorLength = charLength(info.connectorText)
  const textLength = charLength(row.text)
  const inferred = inferFieldFromText(info.restText, row.context, false, rows.length)
  const fieldType = row.userSelectedType || inferred.type || row.type
  const fieldName = !row.name || isGeneratedFieldName(row.name) ? inferred.name : row.name
  const fieldLabel =
    !row.label || row.label === row.text || isGeneratedFieldName(row.label) ? inferred.label : row.label
  const fieldRefs = refsForRowTextRange(row, connectorLength, textLength)
  const fieldText = info.restText
  return [
    {
      ...row,
      rowId: `${row.rowId}:auto-field`,
      text: fieldText,
      type: fieldType,
      name: fieldName,
      label: fieldLabel,
      semanticKey: inferred.semanticKey || fieldName,
      markRefs: fieldRefs,
      charStart: connectorLength,
      charEnd: textLength,
      markSegments: markSegmentsFromRefs(fieldRefs, fieldText),
      partyItems: fieldType === 'party_list' ? splitPartyLabelText(fieldText) : [],
    },
  ]
}

function autoSplitKnownSuffixRows(rows) {
  const result = []
  for (const row of rows) {
    const split = splitKnownSuffixRow(row)
    if (split) result.push(...split)
    else result.push(row)
  }
  return result
}

function autoSplitLegalCompoundRows(rows) {
  const result = []
  for (const row of rows) {
    const split = splitLegalCompoundRow(row)
    if (split) result.push(...split)
    else result.push(row)
  }
  return result
}

function splitKnownSuffixRow(row) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  if (row.userSelectedType) return null
  if (!row.markId || !row.text) return null
  const suffixRule = knownSuffixAtEnd(row.text)
  if (!suffixRule) return null
  const suffixText = suffixRule.text
  const trimmedText = String(row.text).trimEnd()
  const suffixStartOffset = trimmedText.length - suffixText.length
  const rawFieldText = trimmedText.slice(0, suffixStartOffset)
  const fieldText = rawFieldText.trim()
  if (!fieldText || fieldText.length > 12) return null
  const fieldStartOffset = rawFieldText.length - rawFieldText.trimStart().length
  const fieldStart = charLength(trimmedText.slice(0, fieldStartOffset))
  const fieldEnd = fieldStart + charLength(fieldText)
  const suffixStart = charLength(trimmedText.slice(0, suffixStartOffset))
  const suffixEnd = suffixStart + charLength(suffixText)
  const fieldRow = {
    ...row,
    rowId: `${row.rowId}:auto-field`,
    text: fieldText,
    type: suffixRule.targetType,
    name: suffixRule.targetName,
    label: suffixRule.targetLabel,
    semanticKey: suffixRule.targetName,
    markRefs: markRefsForTextRange(row, fieldStart, fieldEnd),
    charStart: fieldStart,
    charEnd: fieldEnd,
  }
  const suffixRow = {
    ...row,
    rowId: `${row.rowId}:auto-suffix`,
    text: suffixText,
    type: 'suffix',
    name: suffixRule.targetName,
    label: `${suffixRule.targetLabel}后缀`,
    semanticKey: '',
    required: false,
    optionalWhenEmpty: false,
    markRefs: markRefsForTextRange(row, suffixStart, suffixEnd),
    charStart: suffixStart,
    charEnd: suffixEnd,
  }
  return [fieldRow, suffixRow]
}

function splitLegalCompoundRow(row) {
  if (!row || rowUsage(row) !== 'field' || isMarkerType(row.type)) return null
  if (row.userSelectedType) return null
  const text = String(row.text || '')
  if (!text || charLength(text) < 8) return null

  const pieces = []
  const causeMatch = text.match(/[一-龥A-Za-z0-9、，,（）()·]{2,48}纠纷/)
  if (causeMatch) {
    const start = charLength(text.slice(0, causeMatch.index))
    const end = start + charLength(causeMatch[0])
    pieces.push(
      makeRangeRow(row, start, end, { type: 'text', name: '案由', label: '案由', semanticKey: '案由' }, 'cause'),
    )
    const afterCause = text.slice(causeMatch.index + causeMatch[0].length)
    if (afterCause.startsWith('一案')) {
      pieces.push(
        makeRangeRow(
          row,
          end,
          end + 2,
          { type: 'ignore', name: '', label: '保留原文', semanticKey: '' },
          'cause-ignore',
        ),
      )
    }
  }

  const caseMatch = text.match(
    /[（(]\s*\d{4}\s*[）)]\s*[一-龥A-Za-z0-9]{1,12}(?:民|行|知|执|赔|破|清|申|再|终|初|保|诉前|民终|民初|行初|行终)[一-龥A-Za-z0-9-]*号/,
  )
  if (caseMatch) {
    const caseStart = charLength(text.slice(0, caseMatch.index))
    const caseEnd = caseStart + charLength(caseMatch[0])
    const prefixStart = legalCaseNumberPrefixStart(text, caseMatch.index)
    if (prefixStart >= 0 && prefixStart < caseMatch.index) {
      pieces.push(
        makeRangeRow(
          row,
          charLength(text.slice(0, prefixStart)),
          caseStart,
          { type: 'prefix', name: '案号', label: '案号前缀', semanticKey: '' },
          'case-prefix',
        ),
      )
    }
    pieces.push(
      makeRangeRow(
        row,
        caseStart,
        caseEnd,
        { type: 'text', name: '案号', label: '案号', semanticKey: '案号' },
        'case-number',
      ),
    )
    const closeChar = text.slice(caseMatch.index + caseMatch[0].length, caseMatch.index + caseMatch[0].length + 1)
    if (closeChar === '）' || closeChar === ')') {
      pieces.push(
        makeRangeRow(
          row,
          caseEnd,
          caseEnd + 1,
          { type: 'suffix', name: '案号', label: '案号后缀', semanticKey: '' },
          'case-suffix',
        ),
      )
    }
  }

  if (pieces.length <= 1) return null
  return dedupeRangeRows(pieces)
}

function legalCaseNumberPrefixStart(text, caseNumberIndex) {
  const before = text.slice(0, caseNumberIndex)
  const labelIndex = Math.max(before.lastIndexOf('案号：'), before.lastIndexOf('案号:'))
  if (labelIndex < 0) return -1
  const bracketIndex = Math.max(before.lastIndexOf('（', labelIndex), before.lastIndexOf('(', labelIndex))
  return bracketIndex >= 0 && labelIndex - bracketIndex <= 2 ? bracketIndex : labelIndex
}

// ── Field inference ──────────────────────────────────────────────────────────

export function inferFieldFromText(text, context, checkboxLike, index) {
  const publicInference = inferTemplateField({ text, context, checkboxLike, index })
  if (checkboxLike) {
    return publicInference
  }
  if (isLikelyPrefixMark(text)) {
    return {
      type: 'prefix',
      name: '',
      label: '前缀',
      semanticKey: '',
      optionalWhenEmpty: false,
      optionalScope: 'position',
      optionalPrefix: '',
      optionalSuffix: '',
    }
  }
  if (isLikelySuffixMark(text)) {
    return {
      type: 'suffix',
      name: '',
      label: '后缀',
      semanticKey: '',
      optionalWhenEmpty: false,
      optionalScope: 'position',
      optionalPrefix: '',
      optionalSuffix: '',
    }
  }
  if (publicInference?.name && !isGeneratedFieldName(publicInference.name)) {
    return publicInference
  }
  return {
    type: 'text',
    name: `字段${index + 1}`,
    label: text,
    semanticKey: `字段${index + 1}`,
    optionalWhenEmpty: false,
    optionalScope: 'position',
    optionalPrefix: '',
    optionalSuffix: '',
  }
}

// ── markToRow ────────────────────────────────────────────────────────────────

export function markToRow(mark, index) {
  const inferred = inferFieldFromText(mark.text, mark.context, mark.checkboxLike, index)
  const partyItems = inferred.type === 'party_list' ? splitPartyLabelText(mark.text) : []
  return {
    rowId: `${mark.id}:${index}`,
    displayId: mark.displayId || mark.id,
    markId: mark.id,
    markRefs: mark.markRefs || [{ markId: mark.id, start: null, end: null }],
    markSegments: mark.markSegments || [{ markId: mark.id, text: mark.text }],
    charStart: null,
    charEnd: null,
    text: mark.text,
    context: mark.context,
    enabled: true,
    type: inferred.type,
    name: inferred.name,
    label: inferred.label,
    semanticKey: inferred.semanticKey,
    required: false,
    optionalWhenEmpty: inferred.optionalWhenEmpty,
    optionalScope: inferred.optionalScope,
    optionalPrefix: inferred.optionalPrefix,
    optionalSuffix: inferred.optionalSuffix,
    optionId: `option_${index + 1}`,
    optionLabel: mark.optionLabel || mark.text,
    checkedText: defaultCheckedText(mark.text),
    uncheckedText: defaultUncheckedText(mark.text),
    partyItems,
    referenceHintSeen: false,
    referenceIncludePrefix: true,
    referenceIncludeSuffix: true,
    referenceSourceMode: 'auto',
    referenceSourceField: '',
    referenceSourceSemanticKey: '',
    referenceSourceIndex: null,
    referenceSourceKey: referenceSourceKey('auto', '', null),
    options: inferred.options || [],
    selectOptions: (inferred.options || []).map((opt) => ({
      label: opt.label || '',
      checkedText: opt.checkedText || '',
    })),
  }
}

// ── autoMergeMarks ───────────────────────────────────────────────────────────

export function autoMergeMarks(rawMarks) {
  const rows = []
  let current = null
  for (const mark of rawMarks || []) {
    const normalized = {
      ...mark,
      markRefs: [{ markId: mark.id, start: null, end: null }],
      markSegments: [{ markId: mark.id, text: mark.text }],
    }
    if (shouldMergeMark(current, normalized)) {
      current.text += normalized.text
      current.markRefs.push(...normalized.markRefs)
      current.markSegments.push(...normalized.markSegments)
      current.displayId = `${current.displayId || current.id}+${normalized.id}`
      current.checkboxLike = current.checkboxLike && normalized.checkboxLike
    } else {
      if (current) rows.push(current)
      current = normalized
    }
  }
  if (current) rows.push(current)
  return rows
}

function shouldMergeMark(current, next) {
  if (!current || !next) return false
  if (current.checkboxLike || next.checkboxLike) return false
  if (current.part !== next.part || current.context !== next.context) return false
  if (!Number.isFinite(current.runIndex) || !Number.isFinite(next.runIndex)) return false
  if (next.runIndex !== current.runIndex + current.markRefs.length) return false
  return shouldMergeText(current.text, next.text)
}

function shouldMergeText(left, right) {
  const combined = `${left}${right}`
  if (isLikelyPrefixMark(left) || isLikelySuffixMark(right)) return false
  if (looksLikeDatePrefix(combined)) return true
  if (isPunctuationFragment(right) || isPunctuationFragment(left)) return true
  if (/[A-Za-z0-9]$/.test(left) || /^[A-Za-z0-9]/.test(right)) return true
  if (isShortChineseFragment(left) || isShortChineseFragment(right)) return true
  return false
}

function isShortChineseFragment(text) {
  const value = String(text || '')
  return /^[一-龥]{1,4}$/.test(value)
}

function isPunctuationFragment(text) {
  return /^[（()）.,，.、:：;；\s_-]+$/.test(String(text || ''))
}

// ── buildFields ──────────────────────────────────────────────────────────────

export function buildFields(fieldRows) {
  const rows = fieldRows.filter(
    (row) =>
      row.enabled &&
      ((rowUsage(row) === 'field' && effectiveRowName(row, fieldRows).trim()) ||
        (rowUsage(row) === 'delete_text' && row.markRefs?.length)),
  )
  const byKey = new Map()
  for (const row of rows) {
    const type = row.type
    const currentName = effectiveRowName(row, fieldRows)
    const referenceSource = row.type === 'reference' ? normalizedReferenceSource(row) : null
    const key =
      rowUsage(row) === 'delete_text'
        ? `${type}:${row.rowId}`
        : `${type}:${currentName.trim()}`
    if (!byKey.has(key)) {
      byKey.set(key, {
        id: stableFieldId(
          rowUsage(row) === 'delete_text' ? row.rowId : currentName,
          type,
        ),
        name: rowUsage(row) === 'delete_text' ? `delete_${row.rowId}` : currentName.trim(),
        label: manifestLabelForRow(row, currentName),
        semanticKey: rowUsage(row) === 'delete_text' ? '' : row.semanticKey.trim() || currentName.trim(),
        type,
        required: row.required,
        marks: [],
        markRefs: [],
        optionalRule: null,
        options: [],
        fillAllPositions: false,
        dateFormat: type === 'date' ? (row.dateFormat || 'iso') : '',
        reference:
          row.type === 'reference'
            ? {
                sourceMode: referenceSource?.mode || 'auto',
                sourceField: referenceSource?.sourceField || '',
                sourceSemanticKey: referenceSource?.sourceSemanticKey || '',
                sourceIndex: referenceSource?.sourceIndex ?? null,
              }
            : null,
      })
    } else {
      const existing = byKey.get(key)
      if (existing && !isMarkerType(type) && rowUsage(row) !== 'delete_text') {
        existing.fillAllPositions = true
        if (existing.type !== 'reference') {
          existing.type = 'reference'
          existing.reference = {
            sourceMode: 'field',
            sourceField: existing.name,
            sourceSemanticKey: '',
            sourceIndex: null,
          }
        }
      }
    }
    const field = byKey.get(key)
    if (row.optionalWhenEmpty && row.optionalScope === 'field' && !field.optionalRule) {
      field.optionalRule = {
        enabled: true,
        removeEmptyPrefix: row.optionalPrefix || '',
        removeEmptySuffix: row.optionalSuffix || '',
      }
    }
    if (type === 'select') {
      const src = row.selectOptions?.length ? row.selectOptions : row.options
      if (src?.length && field.options.length === 0) {
        field.options = src
          .filter((opt) => (opt.label || opt.checkedText || '').trim())
          .map((opt, i) => ({
            id: `opt_${i + 1}`,
            label: opt.label || opt.checkedText || '',
            checkedText: opt.checkedText || opt.label || '',
            uncheckedText: '',
            markerMarkId: '',
            markerTag: '',
          }))
      }
    }
    if (isMarkerType(type)) {
      field.options.push({
        id: row.optionId.trim() || `option_${field.options.length + 1}`,
        label: row.optionLabel.trim() || row.text,
        markerMarkId: row.markId,
        checkedText: row.checkedText || '☑',
        uncheckedText: row.uncheckedText || '☐',
      })
    } else {
      const refs = row.markRefs?.length
        ? row.markRefs
        : [{ markId: row.markId, start: row.charStart, end: row.charEnd }]
      const structuralRule = structuralOptionalRuleForRow(row, fieldRows)
      for (const markRef of refs) {
        const normalizedRef = { ...markRef }
        if (row.optionalWhenEmpty && row.optionalScope !== 'field') {
          normalizedRef.optionalRule = {
            enabled: true,
            removeEmptyPrefix: row.optionalPrefix || '',
            removeEmptySuffix: row.optionalSuffix || '',
          }
        } else if (structuralRule.enabled) {
          normalizedRef.optionalRule = structuralRule
        }
        field.marks.push(normalizedRef.markId)
        field.markRefs.push(normalizedRef)
      }
    }
  }
  return Array.from(byKey.values())
}

function effectiveRowName(row, rows) {
  if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') return structureTargetRow(row, rows)?.name || row?.name || ''
  if (isConnectorRow(row)) return connectorTargetName(row, rows)
  return row?.name || ''
}

function connectorTargetName(row, rows) {
  const target = connectorTargetRow(row, rows)
  return target?.name || row.name || ''
}

function connectorTargetRow(row, rows) {
  if (!isConnectorRow(row)) return null
  const index = rows.indexOf(row)
  if (index < 0) return null
  return findNeighborFieldRow(rows, index, 1)
}

function structureRowTargetsField(structureRow, fieldRow, rows) {
  return structureTargetRow(structureRow, rows) === fieldRow
}

function structuralOptionalRuleForRow(fieldRow, rows) {
  const prefix = []
  const suffix = []
  for (const row of rows) {
    const currentName = effectiveRowName(row, rows)
    if (!row.enabled || !currentName.trim() || currentName.trim() !== fieldRow.name.trim()) continue
    if (rowUsage(row) === 'prefix' && structureRowTargetsField(row, fieldRow, rows)) {
      prefix.push(row.text)
    } else if (rowUsage(row) === 'suffix' && structureRowTargetsField(row, fieldRow, rows)) {
      suffix.push(row.text)
    }
  }
  return {
    enabled: Boolean(prefix.length || suffix.length),
    removeEmptyPrefix: prefix.join(''),
    removeEmptySuffix: suffix.join(''),
  }
}

function manifestLabelForRow(row, currentName) {
  const name = String(currentName || '').trim()
  // Auto-generated names ("字段N") keep the highlighted source text as the
  // display label so the fill page shows what was marked, not a placeholder.
  if (isGeneratedFieldName(name)) return String(row?.text || '').trim() || name
  if (rowUsage(row) === 'delete_text') return safeExplicitLabel(row, name)
  if (row.type === 'party_list') return name || row.label?.trim() || row.text
  return safeExplicitLabel(row, name)
}

function safeExplicitLabel(row, fallbackName) {
  const label = String(row?.label || '').trim()
  const rawText = String(row?.text || '').trim()
  if (!label || label === rawText) return fallbackName || ''
  return label
}

// ── validateFieldRowsBeforeSave ──────────────────────────────────────────────

export function validateFieldRowsBeforeSave(fieldRows, marks) {
  const coveredMarks = new Set()
  // Collect mark IDs from ALL rows (including disabled ones) — a disabled row
  // still "covers" its mark (e.g. user set it to 保留原文 / ignore).
  for (const row of fieldRows) {
    for (const ref of row.markRefs || []) {
      if (ref?.markId) coveredMarks.add(ref.markId)
    }
    if (row.markId) coveredMarks.add(row.markId)
  }
  const missingMarks = (marks || []).filter((mark) => !coveredMarks.has(mark.id))
  if (missingMarks.length) {
    const samples = missingMarks
      .slice(0, 3)
      .map((mark) => `"${mark.text}"`)
      .join('、')
    return `仍有 ${missingMarks.length} 处标黄文本未处理：${samples}。请设为字段、前缀、后缀、保留原文或删除文本后再保存`
  }
  for (const row of fieldRows) {
    if (!row.enabled || rowUsage(row) === 'ignore') continue
    if (rowUsage(row) === 'delete_text') continue
    const currentName = effectiveRowName(row, fieldRows)
    if (!currentName.trim()) {
      return `"${row.text}"还没有填写字段名`
    }
    if (rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix') {
      const targets = fieldRows.filter(
        (target) => target.enabled && rowUsage(target) === 'field' && target.name.trim() === currentName.trim(),
      )
      if (!targets.length) {
        return `"${row.text}"设为${rowUsage(row) === 'prefix' ? '前缀' : '后缀'}，但找不到同名字段`
      }
      if (targets.some((target) => isMarkerType(target.type))) {
        return `"${row.text}"不能挂到勾选字段上，请改成文本、日期、下拉或当事人列表字段`
      }
    }
  }
  return ''
}

// ── templateSeedValues ───────────────────────────────────────────────────────

export function templateSeedValues(fieldRows) {
  const values = {}
  const partyValues = new Map()
  for (const row of fieldRows) {
    if (!row.enabled || rowUsage(row) !== 'field' || isMarkerType(row.type) || isGeneratedFieldName(row.name)) continue
    const name = row.name.trim()
    const text = String(row.text || '').trim()
    if (!name || !text) continue
    if (row.type === 'party_list') {
      if (!partyValues.has(name)) partyValues.set(name, [])
      const items = row.partyItems?.length ? row.partyItems : splitPartyLabelText(text)
      for (const item of items.length ? items : [text]) {
        const value = String(item || '').trim()
        if (value && !partyValues.get(name).includes(value)) partyValues.get(name).push(value)
      }
    } else if (!(name in values)) {
      values[name] = text
    }
  }
  for (const [name, items] of partyValues.entries()) {
    if (items.length) values[name] = items
  }
  return values
}

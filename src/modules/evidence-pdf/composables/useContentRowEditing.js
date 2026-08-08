import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  buildFileContentRows,
  createDefaultFooterTextGroup,
  createDefaultHeaderGroup,
  createDefaultPageNumberGroup,
  groupsFor,
  selectedGroupFor,
} from '../../pdf-tools/composables/useEvidencePdfSession.js'
import { renderPageNumberTemplate } from '../../pdf-tools/composables/pdfPageNumberRules.js'

/**
 * Manages inline editing of content rows (header/footer/page-number) in the
 * evidence PDF workbench. Handles edit state, undo/redo, and content row
 * CRUD operations.
 *
 * Extracted from EvidencePdfWorkbench.vue to reduce component size.
 */
export function useContentRowEditing({
  overlayFiles,
  overlayRows,
  selectedOverlayFile,
  selectedOverlayIndex,
  currentRules,
  totalOverlayPages,
  fileExistingStatus,
  refreshPreview,
}) {
  // --- Edit state ---
  const editingContentRowId = ref('')
  const editingContentRowKind = ref('')
  const editingContentRowValue = ref('')

  // --- Undo/redo ---
  const editUndoStack = ref([])
  const editRedoStack = ref([])

  function syncLegacyField(file, kind, el) {
    if (kind === 'header') {
      file.existingHeaderText = el.editedText
      file.existingHeaderEdited = el.decision === 'edit'
      if (el.source !== 'artifact') file.convertPlainHeader = el.decision === 'edit'
    } else if (kind === 'footerText') {
      file.existingFooterText = el.editedText
      file.existingFooterEdited = el.decision === 'edit'
      if (el.source !== 'artifact') file.convertPlainFooter = el.decision === 'edit'
    } else {
      file.existingPageNumberText = el.editedText
      file.existingPageNumberEdited = el.decision === 'edit'
      if (el.source !== 'artifact') file.convertPlainPageNumber = el.decision === 'edit'
    }
  }

  function syncLegacyExistingElementState(file) {
    const elements = file.existingElements || []
    const applyKind = (kind, legacyName) => {
      const matches = elements.filter((element) => element.kind === kind)
      const edited = matches.find((element) => element.decision === 'edit')
      const representative = edited || matches.find((element) => element.decision !== 'ignore') || matches[0]
      file[`removeExisting${legacyName}`] = matches.some((element) => element.decision === 'delete')
      file[`existing${legacyName}Text`] = representative
        ? representative.decision === 'edit'
          ? representative.editedText
          : representative.detectedText
        : ''
      file[`existing${legacyName}Edited`] = matches.some(
        (element) => element.decision === 'edit' && element.source === 'artifact',
      )
      file[`convertPlain${legacyName}`] = matches.some(
        (element) => element.decision === 'edit' && element.source !== 'artifact',
      )
    }
    applyKind('header', 'Header')
    applyKind('footerText', 'Footer')
    applyKind('pageNumber', 'PageNumber')
  }

  function pushEditUndo(snap) {
    editUndoStack.value.push(snap)
    if (editUndoStack.value.length > 30) editUndoStack.value.shift()
    editRedoStack.value = []
  }

  function undoEdit() {
    const snap = editUndoStack.value.pop()
    if (!snap) return false
    editRedoStack.value.push(snap)
    const file = overlayFiles.value.find(f => f.path === snap.filePath)
    if (!file) return true
    if (snap.type === 'group') {
      const groups = groupsFor(file, snap.kind)
      const group = groups.find(g => g.id === snap.groupId)
      if (group) group.text = snap.oldValue
    } else if (snap.type === 'element') {
      const el = (file.existingElements || []).find(e => e.id === snap.elementId)
      if (el) {
        el.editedText = snap.oldValue
        el.decision = snap.oldDecision
        syncLegacyField(file, snap.kind, el)
      }
    }
    overlayFiles.value = [...overlayFiles.value]
    return true
  }

  function redoEdit() {
    const snap = editRedoStack.value.pop()
    if (!snap) return false
    editUndoStack.value.push(snap)
    const file = overlayFiles.value.find(f => f.path === snap.filePath)
    if (!file) return true
    if (snap.type === 'group') {
      const groups = groupsFor(file, snap.kind)
      const group = groups.find(g => g.id === snap.groupId)
      if (group) group.text = snap.newValue
    } else if (snap.type === 'element') {
      const el = (file.existingElements || []).find(e => e.id === snap.elementId)
      if (el) {
        el.editedText = snap.newValue
        el.decision = snap.newDecision
        syncLegacyField(file, snap.kind, el)
      }
    }
    overlayFiles.value = [...overlayFiles.value]
    return true
  }

  // --- Content row display helpers ---
  function contentKindLabel(kind) {
    if (kind === 'header') return '页眉'
    if (kind === 'footerText') return '页脚文字'
    return '页码'
  }

  function contentStatusLabel(status) {
    if (status === 'pending-write') return '待写入'
    if (status === 'pending-add') return '待添加'
    if (status === 'existing') return '已有'
    if (status === 'confirmed') return '已确认'
    if (status === 'pending-edit') return '待编辑'
    if (status === 'pending-delete') return '待删除'
    return status
  }

  function contentStatusTagType(status) {
    if (status === 'pending-write' || status === 'pending-add') return 'primary'
    if (status === 'confirmed') return 'success'
    if (status === 'existing') return 'info'
    if (status === 'pending-edit') return 'warning'
    if (status === 'pending-delete') return 'danger'
    return 'info'
  }

  function displayContentRowText(file, index, cr) {
    if (cr.source === 'new') {
      if (cr.kind === 'pageNumber') {
        const group = cr.group
        const seq = group.sequence || 'continuous'
        const continuous = seq !== 'per-file'
        const page = continuous ? file.pageStart || 1 : 1
        const total = continuous ? totalOverlayPages.value : file.pages || 1
        return renderPageNumberTemplate(cr.text, page, total, group.style || 'arabic')
      }
      return cr.text || '-'
    }
    return cr.text || '-'
  }

  function mainColumnContentText(file, index) {
    const rows = buildFileContentRows(file, index, currentRules.value)
    if (!rows.length) return '-'
    return displayContentRowText(file, index, rows[0])
  }

  function startMainColumnContentEdit(row, index) {
    const rows = buildFileContentRows(row, index, currentRules.value)
    if (!rows.length) return
    const cr = rows[0]
    startContentRowEdit(row, cr)
  }

  function finishMainColumnContentEdit(row) {
    const rowPrefix = `${row.path}|`
    if (!editingContentRowId.value.startsWith(rowPrefix)) return
    const crId = editingContentRowId.value.slice(rowPrefix.length)
    const index = overlayRows.value.findIndex((item) => item.path === row.path)
    const rows = buildFileContentRows(row, index, currentRules.value)
    const cr = rows.find((r) => r.id === crId)
    if (cr && cr.source === 'existing') {
      clearContentRowEdit()
    } else if (cr) {
      finishContentRowEdit(row, cr)
    } else {
      clearContentRowEdit()
    }
  }

  function cancelContentRowEdit() {
    clearContentRowEdit()
  }

  function startContentRowEdit(row, cr) {
    clearContentRowEdit()
    editingContentRowId.value = `${row.path}|${cr.id}`
    editingContentRowKind.value = cr.kind
    if (cr.source === 'existing') {
      editingContentRowValue.value = cr.element?.editedText || cr.element?.detectedText || ''
    } else if (cr.kind === 'pageNumber') {
      editingContentRowValue.value = cr.group?.template || '{page}/{total}'
    } else {
      editingContentRowValue.value = cr.text || ''
    }
  }

  function finishContentRowEdit(row, cr) {
    const value = String(editingContentRowValue.value ?? '').trim()
    if (cr.source === 'new') {
      finishNewContentRowEdit(row, cr, value)
    } else {
      finishExistingContentRowEdit(row, cr, value)
    }
    clearContentRowEdit()
    refreshPreview()
  }

  function finishNewContentRowEdit(row, cr, value) {
    let group = cr.group || selectedGroupFor(row, cr.kind) || groupsFor(row, cr.kind)[0]
    if (!group) {
      if (cr.kind === 'header') {
        group = createDefaultHeaderGroup()
        row.headerGroups = [group]
        row.selectedHeaderGroupId = group.id
      } else if (cr.kind === 'footerText') {
        group = createDefaultFooterTextGroup()
        row.footerTextGroups = [group]
        row.selectedFooterTextGroupId = group.id
      } else {
        group = createDefaultPageNumberGroup()
        row.pageNumberGroups = [group]
        row.selectedPageNumberGroupId = group.id
      }
    }
    const oldValue = cr.kind === 'pageNumber' ? (group.template || '{page}/{total}') : (group.text || '')
    if (cr.kind === 'header') {
      group.text = value
      if (group.mode === 'filename' || group.mode === 'seq' || group.mode === 'seq_cn') {
        group.mode = 'per_file'
      }
      if (group.mode === 'per_file') {
        row.header = value
      }
    } else if (cr.kind === 'footerText') {
      group.text = value
    } else if (cr.kind === 'pageNumber') {
      if (!value.includes('{page}')) {
        ElMessage.warning('页码模板必须包含 {page} 占位符')
        return
      }
      group.template = value
    }
    pushEditUndo({
      type: 'group',
      filePath: row.path,
      kind: cr.kind,
      groupId: group.id,
      oldValue,
      newValue: cr.kind === 'pageNumber' ? group.template : group.text,
    })
    overlayFiles.value = [...overlayFiles.value]
  }

  function finishExistingContentRowEdit(row, cr, value) {
    const element = cr.element
    if (!element) return
    const original = element.detectedText || ''
    const oldDecision = element.decision
    const oldEditedText = element.editedText || element.detectedText || ''
    if (!value) {
      element.decision = 'delete'
      element.editedText = ''
    } else if (value !== original) {
      element.decision = 'edit'
      element.editedText = value
      if (cr.kind === 'header') {
        row.existingHeaderText = value
        row.existingHeaderEdited = true
        if (element.source !== 'artifact') row.convertPlainHeader = true
      } else if (cr.kind === 'footerText') {
        row.existingFooterText = value
        row.existingFooterEdited = true
        if (element.source !== 'artifact') row.convertPlainFooter = true
      } else {
        row.existingPageNumberText = value
        row.existingPageNumberEdited = true
        if (element.source !== 'artifact') row.convertPlainPageNumber = true
      }
    }
    syncLegacyExistingElementState(row)
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    const newDecision = element.decision
    const newEditedText = element.editedText || element.detectedText || ''
    if (newDecision !== oldDecision || newEditedText !== oldEditedText) {
      pushEditUndo({
        type: 'element',
        filePath: row.path,
        kind: cr.kind,
        elementId: element.id,
        oldDecision,
        oldValue: oldEditedText,
        newDecision,
        newValue: newEditedText,
      })
    }
    overlayFiles.value = [...overlayFiles.value]
  }

  function cancelExistingDecision(row, cr) {
    const element = cr.element
    if (!element) return
    element.decision = 'keep'
    element.editedText = element.detectedText
    syncLegacyExistingElementState(row)
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    refreshPreview()
    overlayFiles.value = [...overlayFiles.value]
  }

  function removeContentRowNew(row, cr) {
    if (!cr.group) return
    removeGroupFromFile(row, cr.kind, cr.group.id)
  }

  async function removeContentRowExisting(row, cr) {
    const element = cr.element
    if (!element) return
    try {
      await ElMessageBox.confirm(
        '确认删除该条已检测到的内容？',
        '删除确认',
        { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
      )
    } catch {
      return
    }
    element.decision = 'delete'
    syncLegacyExistingElementState(row)
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    refreshPreview()
  }

  function clearContentRowEdit() {
    editingContentRowId.value = ''
    editingContentRowKind.value = ''
    editingContentRowValue.value = ''
  }

  function removeGroupFromFile(row, kind, groupId) {
    const groups = groupsFor(row, kind)
    if (groups.length <= 1) return
    const updated = groups.filter((g) => g.id !== groupId)
    if (kind === 'header') row.headerGroups = updated
    else if (kind === 'footerText') row.footerTextGroups = updated
    else row.pageNumberGroups = updated
    if (row.selectedHeaderGroupId === groupId) row.selectedHeaderGroupId = updated[0]?.id || ''
    if (row.selectedFooterTextGroupId === groupId) row.selectedFooterTextGroupId = updated[0]?.id || ''
    if (row.selectedPageNumberGroupId === groupId) row.selectedPageNumberGroupId = updated[0]?.id || ''
    refreshPreview()
  }

  function focusGroup(row, kind, groupId) {
    const index = overlayRows.value.findIndex((item) => item.path === row.path)
    if (index >= 0) selectedOverlayIndex.value = index
    setSelectedGroup(row, kind, groupId)
    refreshPreview()
  }

  return {
    // Edit state
    editingContentRowId,
    editingContentRowKind,
    editingContentRowValue,
    // Undo/redo
    editUndoStack,
    editRedoStack,
    pushEditUndo,
    undoEdit,
    redoEdit,
    // Display helpers
    contentKindLabel,
    contentStatusLabel,
    contentStatusTagType,
    displayContentRowText,
    mainColumnContentText,
    // Edit operations
    startMainColumnContentEdit,
    finishMainColumnContentEdit,
    cancelContentRowEdit,
    startContentRowEdit,
    finishContentRowEdit,
    finishNewContentRowEdit,
    finishExistingContentRowEdit,
    cancelExistingDecision,
    removeContentRowNew,
    removeContentRowExisting,
    clearContentRowEdit,
    removeGroupFromFile,
    focusGroup,
    // Legacy sync
    syncLegacyField,
    syncLegacyExistingElementState,
  }
}

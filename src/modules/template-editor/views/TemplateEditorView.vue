<template>
  <div class="template-editor-view" v-loading="sessionLoading">
    <div class="editor-toolbar">
      <el-button @click="handleLoadFile" :icon="FolderOpened">
        加载文档
      </el-button>
      <el-button
        @click="handleLoadExisting"
        :icon="DocumentCopy"
      >
        编辑已有
      </el-button>
      <el-divider direction="vertical" />
      <el-button
        type="primary"
        @click="handleSave"
        :loading="sessionSaving"
        :disabled="!session"
        :icon="Check"
      >
        保存模板
      </el-button>
      <div class="toolbar-spacer" />
      <el-tag v-if="session?.template_id" type="info" size="small">
        {{ session.template_id }}
      </el-tag>
    </div>

    <div v-if="!session" class="editor-empty">
      <el-empty description="加载一个 Word 文档开始制作模板">
        <el-button type="primary" @click="handleLoadFile">
          加载 .docx 文件
        </el-button>
      </el-empty>
    </div>

    <div v-else class="editor-body">
      <div class="editor-main">
        <div
          ref="previewRef"
          class="preview-area"
          @mouseup="handleTextSelect"
        >
          <div
            v-for="(para, pIdx) in paragraphs"
            :key="pIdx"
            class="preview-paragraph"
          >
            <template v-for="(seg, sIdx) in para" :key="`${pIdx}-${sIdx}`">
              <span v-if="seg.type === 'text'">{{ seg.text }}</span>
              <span
                v-else
                class="mark-tag"
                :class="{ active: activeMarkId === seg.markId }"
                @click.stop="handleMarkClick(seg.markId)"
              >
                {{ seg.text }}
                <span class="mark-key">{{ seg.fieldKey }}</span>
              </span>
            </template>
          </div>
        </div>
      </div>

      <div class="editor-sidebar">
        <div class="sidebar-header">
          <span>字段列表</span>
          <el-tag size="small" type="info">{{ fields.length }}</el-tag>
        </div>
        <div class="sidebar-body">
          <div v-if="fields.length === 0" class="sidebar-empty">
            选中文本标记字段
          </div>
          <div
            v-for="field in fields"
            :key="field.key"
            class="field-card"
            :class="{ active: activeFieldKey === field.key }"
            @click="handleFieldClick(field.key)"
          >
            <div class="field-card-header">
              <span class="field-label">{{ field.label }}</span>
              <el-tag size="small" :type="typeTag(field.type)">
                {{ typeLabel(field.type) }}
              </el-tag>
            </div>
            <div class="field-card-meta">
              <code>{{ field.key }}</code>
              <el-tag v-if="field.required" size="small" type="warning">
                必填
              </el-tag>
            </div>
            <div class="field-card-marks">
              <el-tag
                v-for="m in marksForField(field.key)"
                :key="m.id"
                size="small"
                closable
                @close.stop="removeMark(m.id)"
                class="mini-mark-tag"
              >
                {{ session.plain_text.slice(m.start, Math.min(m.end, m.start + 12)) }}
                {{ m.end - m.start > 12 ? '...' : '' }}
              </el-tag>
            </div>
          </div>
        </div>
      </div>
    </div>

    <MarkPopover
      :visible="popoverVisible"
      :position="popoverPos"
      :initial-data="popoverData"
      :selected-text="popoverSelectedText"
      :existing-keys="fields.map((f) => f.key)"
      @confirm="handlePopoverConfirm"
      @cancel="closePopover"
      @delete="handlePopoverDelete"
    />
  </div>
</template>

<script setup>
import { ref, computed, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { FolderOpened, DocumentCopy, Check } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { useSession } from '../composables/useSession.js'
import { useMarks } from '../composables/useMarks.js'
import { usePreview } from '../composables/usePreview.js'
import MarkPopover from '../components/MarkPopover.vue'

const route = useRoute()

const {
  session,
  loading: sessionLoading,
  saving: sessionSaving,
  loadFromDocx,
  loadFromTemplate,
  save,
} = useSession()

const {
  marks,
  fields,
  activeMarkId,
  addMark,
  updateMark,
  removeMark,
  selectMark,
  clearSelection,
  markOverlaps,
} = useMarks(session)

const { paragraphs } = usePreview(session)

const previewRef = ref(null)
const popoverVisible = ref(false)
const popoverPos = ref({ x: 0, y: 0 })
const popoverData = ref(null)
const popoverSelectedText = ref('')
const pendingSelection = ref(null)

const activeFieldKey = computed(() => {
  const mark = marks.value.find((m) => m.id === activeMarkId.value)
  return mark?.fieldKey || null
})

function marksForField(fieldKey) {
  return marks.value.filter((m) => m.fieldKey === fieldKey)
}

async function handleLoadFile() {
  try {
    const path = await open({
      filters: [{ name: 'Word 文档', extensions: ['docx'] }],
    })
    if (!path) return
    await loadFromDocx(path)
    ElMessage.success('文档加载成功')
  } catch (err) {
    ElMessage.error('加载失败: ' + String(err))
  }
}

async function handleLoadExisting() {
  const tid = route.params.templateId
  if (!tid) {
    ElMessage.warning('请先选择一个模板')
    return
  }
  try {
    await loadFromTemplate(tid)
    ElMessage.success('模板加载成功')
  } catch (err) {
    ElMessage.error('加载失败: ' + String(err))
  }
}

async function handleSave() {
  try {
    const id = await save()
    ElMessage.success('模板已保存: ' + id)
  } catch (err) {
    ElMessage.error('保存失败: ' + String(err))
  }
}

function handleTextSelect(e) {
  const sel = window.getSelection()
  if (!sel || sel.isCollapsed || !sel.rangeCount) return

  const range = sel.getRangeAt(0)
  const container = previewRef.value
  if (!container || !container.contains(range.commonAncestorContainer)) return

  const { startOffset, endOffset } = getSelectionOffsets(container, range)
  if (startOffset === endOffset) return
  if (markOverlaps(startOffset, endOffset)) {
    sel.removeAllRanges()
    return
  }

  const selectedText = session.value.plain_text.slice(startOffset, endOffset)
  const rect = range.getBoundingClientRect()

  pendingSelection.value = { start: startOffset, end: endOffset }
  popoverPos.value = {
    x: rect.left + rect.width / 2 - 170,
    y: rect.bottom + 8,
  }
  popoverData.value = null
  popoverSelectedText.value = selectedText
  popoverVisible.value = true

  sel.removeAllRanges()
}

function handleMarkClick(markId) {
  const mark = marks.value.find((m) => m.id === markId)
  if (!mark) return
  selectMark(markId)

  const field = session.value.fields.find((f) => f.key === mark.fieldKey)
  if (!field) return

  popoverData.value = { ...field, key: mark.fieldKey }
  popoverSelectedText.value = session.value.plain_text.slice(mark.start, mark.end)
  popoverPos.value = { x: 200, y: 200 }
  popoverVisible.value = true
}

function handleFieldClick(fieldKey) {
  const firstMark = marks.value.find((m) => m.fieldKey === fieldKey)
  if (firstMark) handleMarkClick(firstMark.id)
}

function handlePopoverConfirm(config) {
  if (popoverData.value) {
    const mark = marks.value.find((m) => m.id === activeMarkId.value)
    if (mark) {
      updateMark(mark.id, { fieldConfig: config })
    }
  } else if (pendingSelection.value) {
    addMark(pendingSelection.value.start, pendingSelection.value.end, config)
  }
  closePopover()
}

function handlePopoverDelete() {
  if (activeMarkId.value) {
    removeMark(activeMarkId.value)
  }
  closePopover()
}

function closePopover() {
  popoverVisible.value = false
  popoverData.value = null
  pendingSelection.value = null
  clearSelection()
}

function getSelectionOffsets(container, range) {
  const preRange = document.createRange()
  preRange.selectNodeContents(container)
  preRange.setEnd(range.startContainer, range.startOffset)
  const startOffset = preRange.toString().length
  const endOffset = startOffset + range.toString().length
  return { startOffset, endOffset }
}

const typeLabels = {
  text: '文本',
  textarea: '多行',
  date: '日期',
  number: '数字',
  select: '选择',
  party: '当事人',
}

function typeLabel(type) {
  return typeLabels[type] || type
}

function typeTag(type) {
  const map = { text: '', textarea: 'info', date: 'success', number: 'warning', select: 'danger', party: '' }
  return map[type] || ''
}
</script>

<style scoped>
.template-editor-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid #e4e7ed;
  background: #fafafa;
  flex-shrink: 0;
}

.toolbar-spacer {
  flex: 1;
}

.editor-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.editor-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.editor-main {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.preview-area {
  max-width: 720px;
  margin: 0 auto;
  font-size: 15px;
  line-height: 2;
  color: #303133;
  user-select: text;
  cursor: text;
}

.preview-paragraph {
  margin-bottom: 4px;
  min-height: 1.6em;
}

.mark-tag {
  background: #ecf5ff;
  border: 1px solid #b3d8ff;
  border-radius: 3px;
  padding: 1px 4px;
  cursor: pointer;
  position: relative;
  transition: all 0.15s;
}

.mark-tag:hover {
  background: #d9ecff;
  border-color: #79bbff;
}

.mark-tag.active {
  background: #409eff;
  color: #fff;
  border-color: #409eff;
}

.mark-tag.active .mark-key {
  color: #ecf5ff;
}

.mark-key {
  font-size: 10px;
  color: #79bbff;
  margin-left: 4px;
  font-family: monospace;
}

.editor-sidebar {
  width: 280px;
  border-left: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
  background: #fafafa;
  flex-shrink: 0;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  font-weight: 600;
  font-size: 14px;
  border-bottom: 1px solid #e4e7ed;
}

.sidebar-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.sidebar-empty {
  padding: 24px;
  text-align: center;
  color: #909399;
  font-size: 13px;
}

.field-card {
  padding: 10px 12px;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  margin-bottom: 8px;
  cursor: pointer;
  transition: all 0.15s;
  background: #fff;
}

.field-card:hover {
  border-color: #c0c4cc;
}

.field-card.active {
  border-color: #409eff;
  background: #ecf5ff;
}

.field-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.field-label {
  font-weight: 600;
  font-size: 14px;
  color: #303133;
}

.field-card-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.field-card-meta code {
  font-size: 11px;
  color: #909399;
  background: #f0f2f5;
  padding: 1px 4px;
  border-radius: 3px;
}

.field-card-marks {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.mini-mark-tag {
  font-size: 11px;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

<template>
  <div class="markdown-convert-view" :class="{ 'is-file-dragging': dragging }">
    <div v-if="dragging" class="document-drop-overlay">
      <div class="document-drop-message">松开以添加 Markdown / Word 文件</div>
    </div>

    <ToolWorkspaceShell
      title="MD ↔ Word"
      description="Markdown 与 Word 文档双向转换；文件在原位置生成转换副本，也可以粘贴 Markdown 直接生成文档。"
    >
      <template #toolbar>
        <el-button type="primary" @click="selectFiles">选择 Markdown / Word 文件</el-button>
      </template>

      <div class="convert-workspace">
        <section class="conversion-panel">
          <div class="panel-heading">
            <div>
              <h3>文件互转</h3>
              <p>支持 .md、.markdown、.docx 和 .doc，添加后自动开始转换。</p>
            </div>
            <span class="direction-note">MD → Word · Word → MD</span>
          </div>
          <FileQueuePanel
            :items="files"
            empty-text="选择或拖入 Markdown / Word 文件，转换结果保存在源文件旁"
            @clear="clearFiles"
            @remove="removeFile"
          >
            <template #meta="{ item }">
              <el-tag size="small" type="info">{{ item.directionTag }}</el-tag>
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <span v-if="item.status === 'done' && item.inputSize > 0">
                {{ formatFileSize(item.inputSize) }} → {{ formatFileSize(item.outputSize) }}
              </span>
            </template>
          </FileQueuePanel>
          <div v-if="summary" class="result-line">{{ summary }}</div>
        </section>

        <section class="conversion-panel paste-panel">
          <div class="panel-heading">
            <div>
              <h3>粘贴 Markdown</h3>
              <p>适合临时内容，无需先保存为 .md 文件。</p>
            </div>
          </div>
          <el-input
            v-model="pasteText"
            type="textarea"
            :rows="9"
            resize="vertical"
            placeholder="在此粘贴 Markdown 文本…"
          />
          <div class="paste-actions">
            <el-radio-group v-model="pasteFormat" :disabled="pasteConverting">
              <el-radio-button value="docx">Word 文档 (.docx)</el-radio-button>
              <el-radio-button value="html">网页 (.html)</el-radio-button>
            </el-radio-group>
            <el-input
              v-model="pasteFileName"
              placeholder="文件名（可选）"
              :disabled="pasteConverting"
              class="file-name-input"
            />
            <el-button
              type="success"
              :loading="pasteConverting"
              :disabled="pasteConverting || !pasteText.trim()"
              @click="convertPastedText"
            >
              生成文件
            </el-button>
          </div>
          <div v-if="pasteOutputPath" class="result-line output-result">
            <span>已生成：{{ pasteOutputPath }}</span>
            <el-button link type="primary" size="small" @click="openPasteOutputDir">打开所在文件夹</el-button>
          </div>
        </section>
      </div>
    </ToolWorkspaceShell>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { fileName, parentDir } from '../../../core/filePath.js'
import { openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { batchSummaryText, formatFileSize } from '../../../shared/pdf-tools/composables/pdfLosslessOptimize.js'

const files = ref([])
const converting = ref(false)
const summary = ref('')
const dragging = ref(false)
const pasteText = ref('')
const pasteFormat = ref('docx')
const pasteFileName = ref('')
const pasteConverting = ref(false)
const pasteOutputPath = ref('')

function isConvertiblePath(path) {
  return /\.(md|markdown|docx|doc)$/i.test(String(path || ''))
}

function directionTag(path) {
  return /\.(md|markdown)$/i.test(String(path || '')) ? 'MD→Word' : 'Word→MD'
}

async function selectFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'Markdown / Word', extensions: ['md', 'markdown', 'docx', 'doc'] }],
  })
  if (!selected) return
  await loadFiles(Array.isArray(selected) ? selected : [selected])
}

async function loadFiles(paths) {
  const busy = new Set(
    files.value.filter((item) => ['pending', 'processing'].includes(item.status)).map((item) => item.path),
  )
  const candidates = [...new Set(paths)].filter((path) => isConvertiblePath(path) && !busy.has(path))
  let accepted = candidates
  let docEngine = 'extract'
  const legacyDocs = candidates.filter((path) => /\.doc$/i.test(path))

  if (legacyDocs.length) {
    const [wordStatus, wpsStatus] = await Promise.all([
      tauriCallSafe('check_external_tool', { toolName: 'word' }),
      tauriCallSafe('check_external_tool', { toolName: 'wps' }),
    ])
    const officeAvailable =
      Boolean(wordStatus.ok && wordStatus.data?.available) || Boolean(wpsStatus.ok && wpsStatus.data?.available)
    try {
      await ElMessageBox.confirm(
        `${legacyDocs.length} 个旧版 .doc 文件：推荐先另存为 .docx。` +
          (officeAvailable
            ? '也可以尝试用本机 Word/WPS 自动转换，或直接提取纯文本。'
            : '未检测到本机 Word/WPS，直接转换仅保留纯文本。'),
        '旧版 .doc 转换方式',
        {
          distinguishCancelAndClose: true,
          confirmButtonText: officeAvailable ? '尝试 Word/WPS 转换' : '直接转换（仅文本）',
          cancelButtonText: officeAvailable ? '直接转换（仅文本）' : '跳过 .doc',
          type: 'warning',
        },
      )
      docEngine = officeAvailable ? 'word' : 'extract'
    } catch (action) {
      if (action === 'cancel' && officeAvailable) {
        docEngine = 'extract'
      } else {
        accepted = candidates.filter((path) => !/\.doc$/i.test(path))
        if (accepted.length) ElMessage.info('已跳过 .doc 文件，其余文件照常转换')
      }
    }
  }

  const items = accepted.map((path) => ({
    path,
    name: fileName(path),
    directionTag: directionTag(path),
    docEngine: /\.doc$/i.test(path) ? docEngine : null,
    status: 'pending',
    statusText: '等待中',
    statusType: 'info',
    inputSize: 0,
    outputSize: 0,
  }))
  if (!items.length) return
  files.value.push(...items)
  summary.value = ''
  void runQueue()
}

function clearFiles() {
  files.value = files.value.filter((item) => item.status === 'processing')
  summary.value = ''
}

function removeFile(index) {
  if (files.value[index]?.status !== 'processing') files.value.splice(index, 1)
}

async function runQueue() {
  if (converting.value) return
  converting.value = true
  const processed = []
  try {
    for (;;) {
      const item = files.value.find((candidate) => candidate.status === 'pending')
      if (!item) break
      item.status = 'processing'
      item.statusText = '转换中'
      item.statusType = 'warning'
      const result = await tauriCallSafe('convert_markdown', {
        input: item.path,
        outputDir: null,
        docEngine: item.docEngine || null,
      })
      if (result.ok) {
        item.status = 'done'
        item.statusText = '完成'
        item.statusType = 'success'
        item.inputSize = Number(result.data?.input_size) || 0
        item.outputSize = Number(result.data?.output_size) || 0
      } else {
        item.status = 'failed'
        item.statusText = userFacingError(result.error, '转换失败')
        item.statusType = 'danger'
      }
      processed.push(item)
    }
  } finally {
    converting.value = false
  }
  if (!processed.length) return
  summary.value = batchSummaryText('转换完成', processed)
  ElNotification({
    type: processed.some((item) => item.status === 'failed') ? 'warning' : 'success',
    title: summary.value,
    duration: 6000,
  })
}

async function convertPastedText() {
  if (pasteConverting.value || !pasteText.value.trim()) return
  pasteConverting.value = true
  pasteOutputPath.value = ''
  try {
    const result = await tauriCallSafe('convert_markdown_text', {
      text: pasteText.value,
      format: pasteFormat.value,
      outputDir: null,
      fileStem: pasteFileName.value.trim() || null,
    })
    if (result.ok) {
      pasteOutputPath.value = String(result.data?.output_path || '')
      ElMessage.success(`转换完成：${pasteOutputPath.value}`)
    } else {
      ElMessage.error(userFacingError(result.error, '转换失败'))
    }
  } finally {
    pasteConverting.value = false
  }
}

async function openPasteOutputDir() {
  if (pasteOutputPath.value) await openPath(parentDir(pasteOutputPath.value))
}

async function handleDroppedPaths(paths) {
  const accepted = [...new Set((paths || []).filter(isConvertiblePath))]
  if (!accepted.length) {
    ElMessage.warning('请拖入 Markdown 或 Word 文件（.md / .docx / .doc）')
    return
  }
  if (accepted.length < (paths || []).length) ElMessage.warning('已忽略不支持的文件')
  await loadFiles(accepted)
}

useWindowFileDrop({
  onEnter: () => {
    dragging.value = true
  },
  onLeave: () => {
    dragging.value = false
  },
  onDrop: handleDroppedPaths,
})
</script>

<style scoped>
.markdown-convert-view {
  position: relative;
  height: 100%;
  min-height: 0;
}

.convert-workspace {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(380px, 0.82fr);
  gap: 14px;
  min-height: 100%;
}

.conversion-panel {
  min-width: 0;
  padding: 18px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-panel);
}

.panel-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.panel-heading h3 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-size: 15px;
  font-weight: 680;
}

.panel-heading p {
  margin: 5px 0 0;
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.direction-note {
  flex: 0 0 auto;
  color: var(--docsy-primary);
  font-size: 11px;
  font-weight: 650;
  white-space: nowrap;
}

.paste-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}

.file-name-input {
  flex: 1 1 180px;
}

.result-line {
  margin-top: 12px;
  padding: 10px 12px;
  color: var(--docsy-text);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
  font-size: 12px;
  line-height: 1.5;
}

.output-result {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.output-result span {
  min-width: 0;
  overflow-wrap: anywhere;
}

.document-drop-overlay {
  position: absolute;
  z-index: 20;
  display: grid;
  inset: 12px;
  pointer-events: none;
  border: 2px dashed var(--docsy-primary);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface) 88%, transparent);
  place-items: center;
}

.document-drop-message {
  padding: 12px 18px;
  color: var(--docsy-primary);
  border-radius: var(--docsy-radius);
  background: var(--docsy-primary-soft);
  font-size: 14px;
  font-weight: 650;
}

@media (max-width: 1050px) {
  .convert-workspace {
    grid-template-columns: 1fr;
  }
}
</style>

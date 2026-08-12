<template>
  <div class="markdown-convert-view" :class="{ 'is-file-dragging': dragging }">
    <div v-if="dragging" class="document-drop-overlay">
      <div class="document-drop-message">松开以添加 Markdown、Office 或 PDF 文件</div>
    </div>

    <ToolWorkspaceShell
      title="MD 转换"
      description="Markdown、Office 与 PDF 文本层互转；生成的 Word、Excel 和演示文稿均可继续编辑。"
    >
      <template #toolbar>
        <el-button type="primary" @click="selectFiles">选择 Markdown / Office / PDF 文件</el-button>
        <el-button type="success" :disabled="converting || !hasPendingFiles" @click="runQueue">
          开始转换
        </el-button>
      </template>

      <div class="convert-workspace">
        <section class="conversion-panel">
          <div class="panel-heading">
            <div>
              <h3>文件互转</h3>
              <p>支持 Word、Excel、PowerPoint、OpenDocument、RTF、CSV、EPUB、PDF 与常见 Markdown 扩展名。</p>
            </div>
            <span class="direction-note">MD ↔ Office · PDF → MD</span>
          </div>
          <div class="file-convert-options">
            <span class="option-label">转成 Markdown</span>
            <span class="option-hint">
              Word、Excel、PowerPoint、OpenDocument、RTF、CSV、EPUB、PDF 拖入即转；暂不支持 HTML 转 Markdown（.html 文件会被忽略）。
            </span>
          </div>
          <div class="file-convert-options">
            <span class="option-label">Markdown 转出为</span>
            <el-radio-group v-model="fileOfficeFormat" :disabled="converting">
              <el-radio-button value="docx">Word</el-radio-button>
              <el-radio-button value="xlsx">Excel</el-radio-button>
              <el-radio-button value="pptx">PowerPoint</el-radio-button>
              <el-radio-button value="html">网页 (.html)</el-radio-button>
            </el-radio-group>
            <el-select
              v-if="fileOfficeFormat === 'docx'"
              v-model="docxStyle"
              :disabled="converting"
              class="style-select"
            >
              <el-option label="通用专业版式" value="professional" />
              <el-option label="法律文书版式" value="legal" />
              <el-option label="紧凑工作版式" value="compact" />
            </el-select>
          </div>
          <div class="pdf-convert-note">
            <span>PDF 当前使用文本层提取，不会改写原件。</span>
            <span>扫描件、表格、公式和复杂版面将接入可选的本地 AI 文档解析包。</span>
            <div v-if="hasPdfInQueue" class="pdf-page-range">
              <span class="option-label">PDF 页段（本批所有 PDF，留空为全文）</span>
              <el-input-number
                v-model="pdfStartPage"
                :min="1"
                :precision="0"
                placeholder="起始页"
                :disabled="converting"
                controls-position="right"
              />
              <span class="option-label">至</span>
              <el-input-number
                v-model="pdfEndPage"
                :min="1"
                :precision="0"
                placeholder="结束页"
                :disabled="converting"
                controls-position="right"
              />
            </div>
            <el-tag type="info" effect="plain" class="ai-placeholder-tag">AI 文档解析（可选包，后续提供）</el-tag>
          </div>
          <FileQueuePanel
            :items="files"
            empty-text="选择或拖入 Markdown、Office 或 PDF 文件，转换结果保存在源文件旁"
            @clear="clearFiles"
            @remove="removeFile"
          >
            <template #meta="{ item }">
              <el-tag size="small" type="info">{{ item.directionTag }}</el-tag>
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <span v-if="item.warning" class="queue-warning">{{ item.warning }}</span>
            </template>
            <template #item-actions="{ item, index }">
              <el-button v-if="item.status === 'done' && item.outputPath" link type="primary" size="small" @click="openOutput(item)">
                打开
              </el-button>
              <el-button v-if="item.status !== 'processing'" link type="danger" size="small" @click="removeFile(index)">
                删除
              </el-button>
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
              <el-radio-button value="docx">Word</el-radio-button>
              <el-radio-button value="xlsx">Excel</el-radio-button>
              <el-radio-button value="pptx">PowerPoint</el-radio-button>
              <el-radio-button value="html">网页 (.html)</el-radio-button>
            </el-radio-group>
            <el-select
              v-if="pasteFormat === 'docx'"
              v-model="docxStyle"
              :disabled="pasteConverting"
              class="style-select"
            >
              <el-option label="通用专业版式" value="professional" />
              <el-option label="法律文书版式" value="legal" />
              <el-option label="紧凑工作版式" value="compact" />
            </el-select>
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
import { computed, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { fileName, parentDir } from '../../../core/filePath.js'
import { openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'

const files = ref([])
const converting = ref(false)
const summary = ref('')
const dragging = ref(false)
const pasteText = ref('')
const pasteFormat = ref('docx')
const pasteFileName = ref('')
const pasteConverting = ref(false)
const pasteOutputPath = ref('')
const fileOfficeFormat = ref('docx')
const docxStyle = ref('professional')
const pdfStartPage = ref(null)
const pdfEndPage = ref(null)

const hasPdfInQueue = computed(() => files.value.some((item) => extensionOf(item.path) === 'pdf'))
const hasPendingFiles = computed(() => files.value.some((item) => item.status === 'pending'))

const markdownExtensions = ['md', 'markdown', 'mdown', 'mkdn', 'mdwn', 'mdtxt']
const officeExtensions = [
  'doc', 'docx', 'docm', 'xls', 'xlsx', 'xlsm', 'xlsb', 'ppt', 'pptx', 'pptm', 'pps', 'ppsx', 'ppsm',
  'pot', 'potx', 'potm', 'odt', 'ods', 'odp', 'rtf', 'csv', 'epub',
]
const pdfExtensions = ['pdf']

function extensionOf(path) {
  return String(path || '').split('.').pop()?.toLowerCase() || ''
}

function isMarkdownPath(path) {
  return markdownExtensions.includes(extensionOf(path))
}

function isConvertiblePath(path) {
  const extension = extensionOf(path)
  return markdownExtensions.includes(extension) || officeExtensions.includes(extension) || pdfExtensions.includes(extension)
}

function directionTag(path, outputFormat = fileOfficeFormat.value) {
  if (isMarkdownPath(path)) {
    return `MD→${{ docx: 'Word', xlsx: 'Excel', pptx: 'PowerPoint', html: 'HTML' }[outputFormat] || 'Office'}`
  }
  const extension = extensionOf(path)
  if (extension === 'pdf') return 'PDF→MD'
  if (extension === 'epub') return 'EPUB→MD'
  if (['xls', 'xlsx', 'xlsm', 'xlsb'].includes(extension)) return 'Excel→MD'
  if (['ppt', 'pptx', 'pptm', 'pps', 'ppsx', 'ppsm', 'pot', 'potx', 'potm'].includes(extension)) return 'PowerPoint→MD'
  if (['odt', 'ods', 'odp'].includes(extension)) return 'OpenDocument→MD'
  return 'Office→MD'
}

async function selectFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'Markdown / Office / PDF', extensions: [...markdownExtensions, ...officeExtensions, ...pdfExtensions] }],
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
        `${legacyDocs.length} 个旧版 .doc 文件：默认直接提取为 Markdown（文字、标题、表格结构均可保留，快且无额外依赖）。` +
          (officeAvailable
            ? '如文档版式复杂、需要更高保真度，可改用本机 Word/WPS 中转（较慢）。'
            : '未检测到本机 Word/WPS，仅支持直接提取。'),
        '旧版 .doc 转换方式',
        {
          distinguishCancelAndClose: true,
          confirmButtonText: '直接转换',
          cancelButtonText: officeAvailable ? '用 Word/WPS 高保真转换' : '跳过 .doc',
          type: 'warning',
        },
      )
      docEngine = 'extract'
    } catch (action) {
      if (action === 'cancel' && officeAvailable) {
        docEngine = 'word'
      } else {
        accepted = candidates.filter((path) => !/\.doc$/i.test(path))
        if (accepted.length) ElMessage.info('已跳过 .doc 文件，其余文件照常转换')
      }
    }
  }

  const items = accepted.map((path) => ({
    path,
    name: fileName(path),
    directionTag: directionTag(path, fileOfficeFormat.value),
    docEngine: /\.doc$/i.test(path) ? docEngine : null,
    outputFormat: isMarkdownPath(path) ? fileOfficeFormat.value : null,
    docxStyle: isMarkdownPath(path) ? docxStyle.value : null,
    status: 'pending',
    statusText: '等待中',
    statusType: 'info',
    inputSize: 0,
    outputSize: 0,
  }))
  if (!items.length) return
  files.value.push(...items)
  summary.value = ''
  ElMessage.info('已加入转换列表。确认转换设置后，点击“开始转换”。')
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
  const startPage = Number(pdfStartPage.value || 0)
  const endPage = Number(pdfEndPage.value || 0)
  if (hasPdfInQueue.value && startPage && endPage && endPage < startPage) {
    ElMessage.error('PDF 结束页不能早于起始页')
    return
  }
  converting.value = true
  const processed = []
  try {
    for (;;) {
      const item = files.value.find((candidate) => candidate.status === 'pending')
      if (!item) break
      item.status = 'processing'
      item.statusText = '转换中'
      item.statusType = 'warning'
      const isPdf = extensionOf(item.path) === 'pdf'
      const result = isPdf
        ? await tauriCallSafe('convert_pdf_text_layer', {
            input: item.path,
            outputDir: null,
            startPage: pdfStartPage.value || null,
            endPage: pdfEndPage.value || null,
          })
        : await tauriCallSafe('convert_markdown', {
            input: item.path,
            outputDir: null,
            docEngine: item.docEngine || null,
            outputFormat: item.outputFormat,
            docxStyle: item.docxStyle,
          })
      if (result.ok) {
        item.status = 'done'
        item.outputPath = String(result.data?.output_path || '')
        item.warning = String(result.data?.warning || '')
        item.statusText = item.warning ? '完成（有提示）' : '完成'
        if (isPdf && result.data?.pages_with_text != null) {
          const emptyPages = Array.isArray(result.data?.empty_pages) ? result.data.empty_pages.length : 0
          item.statusText = `完成（${result.data.pages_with_text} 页有文本${emptyPages ? `，${emptyPages} 页为空` : ''}）`
        }
        item.statusType = 'success'
        item.inputSize = Number(result.data?.input_size) || 0
        item.outputSize = Number(result.data?.output_size) || 0
      } else {
        const message = userFacingError(result.error, '转换失败')
        const cancelled = /操作已取消/.test(message)
        item.status = cancelled ? 'cancelled' : 'failed'
        item.statusText = cancelled ? '已取消' : message
        item.statusType = cancelled ? 'info' : 'danger'
      }
      processed.push(item)
    }
  } finally {
    converting.value = false
  }
  if (!processed.length) return
  // 格式转换的输入/输出体积没有可比性，不沿用压缩场景的"共节省 xx"文案
  const done = processed.filter((item) => item.status === 'done')
  const failed = processed.filter((item) => item.status === 'failed')
  summary.value = `转换完成 ${done.length}/${processed.length}`
  if (failed.length) summary.value += `；失败：${failed.map((item) => item.name).join('、')}`
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
      docxStyle: pasteFormat.value === 'docx' ? docxStyle.value : null,
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

async function openOutput(item) {
  if (item?.outputPath) await openPath(item.outputPath)
}

async function handleDroppedPaths(paths) {
  const accepted = [...new Set((paths || []).filter(isConvertiblePath))]
  if (!accepted.length) {
    ElMessage.warning('请拖入 Markdown、Office 或 PDF 文件')
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

.file-convert-options {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin: 0 0 12px;
  padding: 10px 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}

.pdf-convert-note {
  display: grid;
  gap: 3px;
  margin: 0 0 12px;
  padding: 9px 11px;
  color: var(--docsy-text-muted);
  border-left: 3px solid var(--docsy-primary);
  background: var(--docsy-surface-muted);
  font-size: 12px;
  line-height: 1.45;
}

.pdf-page-range {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.pdf-page-range .el-input-number {
  width: 120px;
}

.ai-placeholder-tag {
  justify-self: start;
  margin-top: 4px;
  opacity: 0.65;
}

.option-label {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.option-hint {
  color: var(--docsy-text-muted);
  font-size: 11px;
  line-height: 1.5;
  opacity: 0.85;
}

.style-select {
  width: 150px;
}

.queue-warning {
  flex-basis: 100%;
  color: var(--el-color-warning-dark-2);
  font-size: 11px;
  line-height: 1.45;
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

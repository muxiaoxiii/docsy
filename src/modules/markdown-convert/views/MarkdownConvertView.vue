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
        <el-button type="primary" plain :disabled="converting" @click="selectFiles"
          >添加 Markdown / Office / PDF 文件</el-button
        >
        <el-button type="success" :loading="converting" :disabled="!hasPendingFiles" @click="runQueue">
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
              Word、Excel、PowerPoint、OpenDocument、RTF、CSV、EPUB、PDF 添加后点击“开始转换”；暂不支持 HTML 转
              Markdown（.html 文件会被忽略）。
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
              :disabled="pasteConverting || converting"
              class="style-select"
            >
              <el-option label="通用专业版式" value="professional" />
              <el-option label="法律文书版式" value="legal" />
              <el-option label="紧凑工作版式" value="compact" />
            </el-select>
            <span v-if="fileOfficeFormat === 'docx'" class="docx-style-hint">
              {{ docxStyleHint }}
            </span>
          </div>
          <div class="file-convert-options">
            <span class="option-label">Markdown 文件编码</span>
            <el-select
              v-model="inputEncoding"
              :disabled="converting"
              class="encoding-select"
              aria-label="Markdown 文件编码"
            >
              <el-option label="自动（UTF-8 / UTF-16 BOM）" value="auto" />
              <el-option label="UTF-8" value="utf-8" />
              <el-option label="UTF-16 LE" value="utf-16le" />
              <el-option label="UTF-16 BE" value="utf-16be" />
              <el-option label="西欧 Windows-1252" value="windows-1252" />
              <el-option label="日文 Shift-JIS" value="shift_jis" />
              <el-option label="韩文 EUC-KR / CP949" value="euc-kr" />
              <el-option label="简体中文 GB18030 / GBK" value="gb18030" />
              <el-option label="繁体中文 Big5" value="big5" />
            </el-select>
            <span class="option-hint">支持多语言混排。旧编码需手动选择，适用于本批 Markdown 文件。</span>
          </div>
          <p class="rich-media-hint">
            公式支持 $…$、$$…$$、\(…\)、\[…\]；Word 优先生成可编辑公式，不支持的结构回退为图片。Mermaid
            代码块会嵌入图像。
          </p>
          <p v-if="['xlsx', 'pptx'].includes(fileOfficeFormat)" class="rich-media-hint">
            Excel / PowerPoint 中，公式和图表以图片独立成表或成页。
          </p>
          <div class="pdf-convert-note">
            <span>PDF 当前使用文本层提取，不会改写原件。</span>
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
          </div>
          <FileQueuePanel
            :items="files"
            :busy="converting"
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
              <el-button
                v-if="item.status === 'done' && item.outputPath"
                link
                type="primary"
                size="small"
                @click="openOutput(item)"
              >
                打开文件
              </el-button>
              <el-button
                v-if="item.status !== 'processing'"
                :disabled="converting"
                link
                type="danger"
                size="small"
                @click="removeFile(index)"
              >
                移除
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
            :disabled="pasteConverting"
            type="textarea"
            :rows="9"
            resize="vertical"
            placeholder="在此粘贴 Markdown 文本…"
          />
          <p class="rich-media-hint">
            支持数学公式与 Mermaid 代码块。Word 优先使用可编辑公式；Excel / PowerPoint 的图形独立成表或成页。
          </p>
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
              :disabled="pasteConverting || converting"
              class="style-select"
            >
              <el-option label="通用专业版式" value="professional" />
              <el-option label="法律文书版式" value="legal" />
              <el-option label="紧凑工作版式" value="compact" />
            </el-select>
            <span v-if="pasteFormat === 'docx'" class="docx-style-hint">
              {{ docxStyleHint }}
            </span>
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
              {{ pasteConverting ? pasteProgress : '生成文件' }}
            </el-button>
          </div>
          <el-alert v-if="pasteWarning" :title="pasteWarning" type="warning" :closable="false" show-icon />
          <div v-if="pasteOutputPath" class="result-line output-result">
            <span>已生成：{{ pasteOutputPath }}</span>
            <el-button link type="primary" size="small" @click="openPasteOutputDir">打开文件夹</el-button>
          </div>
        </section>
      </div>
    </ToolWorkspaceShell>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useQueueCancellation } from '../../../core/composables/useQueueCancellation.js'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElNotification } from 'element-plus'
import { convertWithMedia } from '../composables/convertWithMedia.js'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { fileName, parentDir } from '../../../core/filePath.js'
import { openPath, tauriCallQuiet, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'

const files = ref([])
const converting = ref(false)
const summary = ref('')
const dragging = ref(false)
const pasteText = ref('')
const pasteFormat = ref('docx')
const pasteFileName = ref('')
const pasteConverting = ref(false)
const pasteOutputPath = ref('')
const pasteWarning = ref('')
const pasteProgress = ref('')
const fileOfficeFormat = ref('docx')
const inputEncoding = ref('auto')
const docxStyle = ref('professional')
const docxStyleHints = {
  professional: '宋体正文 + Times New Roman + 商务深蓝标题',
  legal: '仿宋正文 + Times New Roman + 首行缩进两字符',
  compact: '宋体正文 + Times New Roman + 紧凑字号行距',
}
const docxStyleHint = computed(() => docxStyleHints[docxStyle.value] || '')
const pdfStartPage = ref(null)
const pdfEndPage = ref(null)
const activeBackendOperationId = ref('')
const queueCancellation = useQueueCancellation()
const PDF_CONVERT_OPERATION_ID = 'convert_pdf_text_layer:auto'
const preference = useWorkspacePreferences('markdown-convert.workspace', {
  inputEncoding,
  fileOfficeFormat,
  docxStyle,
  pasteFormat,
})

const hasPdfInQueue = computed(() => files.value.some((item) => extensionOf(item.path) === 'pdf'))
const hasPendingFiles = computed(() =>
  files.value.some((item) => ['pending', 'failed', 'cancelled'].includes(item.status)),
)

const markdownExtensions = ['md', 'markdown', 'mdown', 'mkdn', 'mdwn', 'mdtxt']
const officeExtensions = [
  'doc',
  'docx',
  'docm',
  'xls',
  'xlsx',
  'xlsm',
  'xlsb',
  'ppt',
  'pptx',
  'pptm',
  'pps',
  'ppsx',
  'ppsm',
  'pot',
  'potx',
  'potm',
  'odt',
  'ods',
  'odp',
  'rtf',
  'csv',
  'epub',
]
const pdfExtensions = ['pdf']

function extensionOf(path) {
  return (
    String(path || '')
      .split('.')
      .pop()
      ?.toLowerCase() || ''
  )
}

function isMarkdownPath(path) {
  return markdownExtensions.includes(extensionOf(path))
}

function isConvertiblePath(path) {
  const extension = extensionOf(path)
  return (
    markdownExtensions.includes(extension) || officeExtensions.includes(extension) || pdfExtensions.includes(extension)
  )
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
    filters: [
      { name: 'Markdown / Office / PDF', extensions: [...markdownExtensions, ...officeExtensions, ...pdfExtensions] },
    ],
  })
  if (!selected) return
  await loadFiles(Array.isArray(selected) ? selected : [selected])
}

async function loadFiles(paths) {
  if (converting.value) return
  const busy = new Set(
    files.value.filter((item) => ['pending', 'processing'].includes(item.status)).map((item) => item.path),
  )
  const candidates = [...new Set(paths)].filter((path) => isConvertiblePath(path) && !busy.has(path))
  // 旧版 .doc 由后端默认 doc2x 自动转换，无需再询问引擎或要求手动另存。
  const accepted = candidates

  const items = accepted.map((path) => ({
    path,
    name: fileName(path),
    directionTag: directionTag(path, fileOfficeFormat.value),
    docEngine: null,
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
  if (converting.value) return
  files.value = files.value.filter((item) => item.status === 'processing')
  summary.value = ''
}

function removeFile(index) {
  if (converting.value) return
  if (files.value[index]?.status !== 'processing') files.value.splice(index, 1)
}

function syncQueuedOptions() {
  if (converting.value) return
  for (const item of files.value) {
    if (item.status === 'done' || !isMarkdownPath(item.path)) continue
    item.outputFormat = fileOfficeFormat.value
    item.docxStyle = docxStyle.value
    item.directionTag = directionTag(item.path, fileOfficeFormat.value)
  }
}

watch([fileOfficeFormat, docxStyle], syncQueuedOptions)

async function runQueue() {
  if (converting.value) return
  let startPage = Number(pdfStartPage.value || 0)
  let endPage = Number(pdfEndPage.value || 0)
  if (startPage <= 0) startPage = 1
  if (hasPdfInQueue.value && startPage && endPage && endPage < startPage) {
    ElMessage.error('PDF 结束页不能早于起始页')
    return
  }
  for (const item of files.value) {
    if (['failed', 'cancelled'].includes(item.status)) item.status = 'pending'
  }
  syncQueuedOptions()
  converting.value = true
  queueCancellation.reset()
  const processed = []
  try {
    while (!queueCancellation.cancelled.value) {
      const item = files.value.find((candidate) => candidate.status === 'pending')
      if (!item) break
      item.status = 'processing'
      item.statusText = '转换中'
      item.statusType = 'warning'
      const isPdf = extensionOf(item.path) === 'pdf'
      activeBackendOperationId.value = isPdf ? PDF_CONVERT_OPERATION_ID : ''
      let result
      try {
        result = isPdf
          ? await tauriCallSafe('convert_pdf_text_layer', {
              input: item.path,
              outputDir: null,
              startPage: startPage || null,
              endPage: endPage || null,
            })
          : await (isMarkdownPath(item.path) ? convertWithMedia : tauriCallSafe)(
              'convert_markdown',
              {
                input: item.path,
                inputEncoding: inputEncoding.value,
                outputDir: null,
                docEngine: item.docEngine || null,
                outputFormat: item.outputFormat,
                docxStyle: item.docxStyle,
              },
              (current, total) => {
                if (queueCancellation.cancelled.value) throw new Error('操作已取消')
                item.statusText = total && current < total ? `正在渲染公式与图表 ${current}/${total}` : '转换中'
              },
            )
      } finally {
        activeBackendOperationId.value = ''
      }
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
        if (cancelled) queueCancellation.cancel()
        item.status = cancelled ? 'cancelled' : 'failed'
        item.statusText = cancelled ? '已取消' : message
        item.statusType = cancelled ? 'info' : 'danger'
      }
      processed.push(item)
      if (item.status === 'cancelled') break
    }
  } finally {
    converting.value = false
  }
  if (!processed.length) return
  // 格式转换的输入/输出体积没有可比性，不沿用压缩场景的"共节省 xx"文案
  const done = processed.filter((item) => item.status === 'done')
  const failed = processed.filter((item) => item.status === 'failed')
  const cancelled = queueCancellation.cancelled.value || processed.some((item) => item.status === 'cancelled')
  summary.value = `${cancelled ? '转换已停止，已完成' : '转换完成'} ${done.length}/${processed.length}`
  if (failed.length) summary.value += `；失败：${failed.map((item) => item.name).join('、')}`
  ElNotification({
    type: failed.length ? 'warning' : cancelled ? 'info' : 'success',
    title: summary.value,
    duration: 6000,
  })
}

async function convertPastedText() {
  if (pasteConverting.value || !pasteText.value.trim()) return
  pasteConverting.value = true
  pasteOutputPath.value = ''
  pasteWarning.value = ''
  pasteProgress.value = '正在转换'
  try {
    const result = await convertWithMedia(
      'convert_markdown_text',
      {
        text: pasteText.value,
        format: pasteFormat.value,
        outputDir: null,
        fileStem: pasteFileName.value.trim() || null,
        docxStyle: pasteFormat.value === 'docx' ? docxStyle.value : null,
      },
      (current, total) => {
        pasteProgress.value = total && current < total ? `正在渲染公式与图表 ${current}/${total}` : '正在转换'
      },
    )
    if (result.ok) {
      pasteOutputPath.value = String(result.data?.output_path || '')
      pasteWarning.value = String(result.data?.warning || '')
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

onMounted(() => void preference.start())
onBeforeUnmount(() => {
  if (activeBackendOperationId.value) {
    void tauriCallQuiet('cancel_operation', { operationId: activeBackendOperationId.value })
  }
  void preference.stop()
})
</script>

<style scoped>
.encoding-select {
  width: 250px;
  max-width: 100%;
}
.rich-media-hint {
  font-size: 12px;
  line-height: 1.6;
  color: var(--docsy-text-muted);
  margin: 8px 0;
}
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
  padding: clamp(14px, 2.8dvh, 18px);
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
  margin-bottom: clamp(10px, 2.1dvh, 14px);
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
  margin: 0 0 clamp(8px, 1.7dvh, 12px);
  padding-block: clamp(8px, 1.8dvh, 10px);
  padding-inline: 12px;
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

.docx-style-hint {
  color: var(--docsy-text-muted);
  font-size: 11px;
  line-height: 1.4;
  opacity: 0.85;
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
  margin-top: clamp(8px, 1.7dvh, 12px);
  padding-block: clamp(8px, 1.8dvh, 10px);
  padding-inline: 12px;
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

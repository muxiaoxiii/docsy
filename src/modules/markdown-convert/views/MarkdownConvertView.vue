<template>
  <div class="markdown-convert-view" :class="{ 'is-file-dragging': dragging }">
    <div v-if="dragging" class="document-drop-overlay">
      <div class="document-drop-message">松开以添加 Markdown、Office 或 PDF 文件</div>
    </div>

    <ToolWorkspaceShell
      title="MD 转换"
      description="Markdown、Office 与 PDF 文本层互转；生成的 Word、Excel 和演示文稿均可继续编辑。"
      :hide-toolbar="!files.length"
    >
      <template #toolbar>
        <el-button type="primary" plain :disabled="converting" @click="selectFiles">
          添加文件
        </el-button>
        <el-button type="success" :loading="converting" :disabled="!hasPendingFiles" @click="runQueue">
          开始转换
        </el-button>
      </template>

      <div
        ref="convertWorkspaceRef"
        class="convert-workspace"
        :class="{ 'is-resizing': isResizing }"
        :style="convertGridStyle"
      >
        <!-- ── Left Panel: File Conversion ── -->
        <section class="conversion-panel file-panel">
          <div class="panel-header-row">
            <div class="title-group">
              <div class="title-with-badge">
                <h3>文件互转</h3>
                <span class="direction-badge">MD ⇄ Office / PDF</span>
              </div>
              <p class="panel-desc">Office / PDF 自动转 Markdown，Markdown 转出为排版文书</p>
            </div>
          </div>

          <!-- Format & Style Toolbar -->
          <div class="format-toolbar">
            <div class="format-group">
              <span class="field-label">转出格式</span>
              <el-radio-group v-model="fileOfficeFormat" :disabled="converting" size="default">
                <el-radio-button value="docx">Word</el-radio-button>
                <el-radio-button value="xlsx">Excel</el-radio-button>
                <el-radio-button value="pptx">PowerPoint</el-radio-button>
                <el-radio-button value="html">网页</el-radio-button>
              </el-radio-group>
            </div>

            <div v-if="fileOfficeFormat === 'docx'" class="style-group">
              <span class="field-label">版式</span>
              <el-select v-model="docxStyle" :disabled="converting" class="style-select" size="default">
                <el-option label="通用专业版式" value="professional" />
                <el-option label="法律文书版式" value="legal" />
                <el-option label="紧凑工作版式" value="compact" />
              </el-select>
            </div>

            <div class="feature-chips">
              <el-tooltip
                :content="
                  fileOfficeFormat === 'docx'
                    ? `${docxStyleHint} · 公式自动生成可编辑公式，Mermaid 代码块自动嵌入图表`
                    : '支持数学公式与图表自动渲染生成'
                "
                placement="top"
              >
                <span class="feature-pill">
                  <el-icon><Compass /></el-icon>
                  <span>公式与图表</span>
                </span>
              </el-tooltip>
            </div>
          </div>

          <!-- PDF Page Range Settings (appears only when PDF present) -->
          <div v-if="hasPdfInQueue" class="pdf-convert-note">
            <span class="pdf-note-title">PDF 文本层提取：</span>
            <div class="pdf-page-range">
              <span class="option-label">页码范围（留空为全文）：</span>
              <el-input-number
                v-model="pdfStartPage"
                :min="1"
                :precision="0"
                placeholder="起始页"
                :disabled="converting"
                size="small"
                controls-position="right"
              />
              <span class="option-sep">至</span>
              <el-input-number
                v-model="pdfEndPage"
                :min="1"
                :precision="0"
                placeholder="结束页"
                :disabled="converting"
                size="small"
                controls-position="right"
              />
            </div>
          </div>

          <!-- Main File Queue with Lively Empty Scene -->
          <FileQueuePanel
            fill
            :items="files"
            :busy="converting"
            @clear="clearFiles"
            @remove="removeFile"
          >
            <!-- Custom Rich Scene for Empty State -->
            <template #empty>
              <MdConvertScene
                :converting="converting"
                :dragging="dragging"
                :celebrating="celebrating"
                :progress-text="activeConvertingStatus"
                :target-format="fileOfficeFormat"
                @select-files="selectFiles"
              />
            </template>

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
                v-if="isMarkdownPath(item.path)"
                link
                type="primary"
                size="small"
                :loading="copyingPath === item.path"
                :disabled="converting"
                title="复制为富文本和纯文本（Foxmail 自动用富文本，微信自动用纯文本）"
                @click="copyMarkdownFile(item)"
              >
                复制
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

          <!-- Subtle Bottom Footer: Compact Encoding Setting -->
          <div class="panel-bottom-bar">
            <div class="encoding-compact-setting">
              <span class="compact-label">编码</span>
              <el-select
                v-model="inputEncoding"
                :disabled="converting"
                size="small"
                class="encoding-compact-select"
                aria-label="Markdown 文件编码"
              >
                <el-option label="自动识别 (UTF-8 / BOM)" value="auto" />
                <el-option label="简体中文 (GB18030 / GBK)" value="gb18030" />
                <el-option label="UTF-8" value="utf-8" />
                <el-option label="UTF-16 LE" value="utf-16le" />
                <el-option label="UTF-16 BE" value="utf-16be" />
                <el-option label="繁体中文 (Big5)" value="big5" />
                <el-option label="日文 (Shift-JIS)" value="shift_jis" />
                <el-option label="韩文 (EUC-KR)" value="euc-kr" />
                <el-option label="西欧 (Windows-1252)" value="windows-1252" />
              </el-select>
              <span class="encoding-subtle-hint">默认自动识别，极少数旧编码可手动切换</span>
            </div>

            <div v-if="summary" class="summary-badge">
              <span>{{ summary }}</span>
            </div>
          </div>
        </section>

        <!-- ── Resizer Grip ── -->
        <div
          class="convert-resizer"
          :class="{ 'is-resizing': isResizing }"
          role="separator"
          aria-label="拖拽调整左右分栏宽度"
          @pointerdown="startConvertResize"
        >
          <div class="resizer-thumb">
            <span class="grip-dot" />
            <span class="grip-dot" />
            <span class="grip-dot" />
          </div>
        </div>

        <!-- ── Right Panel: Paste Markdown ── -->
        <section class="conversion-panel paste-panel">
          <div class="panel-header-row">
            <div class="title-group">
              <div class="title-with-badge">
                <h3>粘贴 Markdown</h3>
                <span v-if="pasteText.trim()" class="char-count-badge">
                  {{ pasteText.length }} 字符
                </span>
              </div>
              <p class="panel-desc">适合临时内容，无需保存为 .md 文件；可一键生成文档或复制到邮件/聊天</p>
            </div>

            <el-button
              v-if="pasteText"
              link
              size="small"
              class="clear-text-btn"
              :disabled="pasteConverting"
              @click="pasteText = ''"
            >
              清空
            </el-button>
          </div>

          <!-- Textarea Editor Area -->
          <div class="paste-editor-wrapper">
            <el-input
              v-model="pasteText"
              :disabled="pasteConverting"
              type="textarea"
              placeholder="在此粘贴 Markdown 文本… 支持公式 $...$、表格、代码块与 Mermaid 图表"
            />
          </div>

          <!-- Paste Controls & Actions -->
          <div class="paste-footer-controls">
            <div class="format-toolbar compact">
              <div class="format-group">
                <span class="field-label">转出格式</span>
                <el-radio-group v-model="pasteFormat" :disabled="pasteConverting" size="default">
                  <el-radio-button value="docx">Word</el-radio-button>
                  <el-radio-button value="xlsx">Excel</el-radio-button>
                  <el-radio-button value="pptx">PowerPoint</el-radio-button>
                  <el-radio-button value="html">网页</el-radio-button>
                </el-radio-group>
              </div>

              <div v-if="pasteFormat === 'docx'" class="style-group">
                <span class="field-label">版式</span>
                <el-select
                  v-model="docxStyle"
                  :disabled="pasteConverting || converting"
                  class="style-select"
                  size="default"
                >
                  <el-option label="通用专业版式" value="professional" />
                  <el-option label="法律文书版式" value="legal" />
                  <el-option label="紧凑工作版式" value="compact" />
                </el-select>
              </div>
            </div>

            <div class="paste-action-row">
              <el-input
                v-model="pasteFileName"
                placeholder="文件名（可选）"
                :disabled="pasteConverting"
                class="file-name-input"
                size="default"
              />

              <el-dropdown trigger="click" @command="copyPastedMarkdown">
                <el-button :disabled="pasteConverting || !pasteText.trim()" size="default">
                  <el-icon><CopyDocument /></el-icon>
                  <span style="margin: 0 4px">复制</span>
                  <el-icon class="el-icon--right"><ArrowDown /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="rich">
                      <div class="dropdown-item-meta">
                        <strong>复制为富文本</strong>
                        <small>适用于 Foxmail / 邮件客户端（保留表格、加粗与样式）</small>
                      </div>
                    </el-dropdown-item>
                    <el-dropdown-item command="plain">
                      <div class="dropdown-item-meta">
                        <strong>复制为纯文本</strong>
                        <small>适用于 微信 / 聊天窗口（自动去除 Markdown 符号）</small>
                      </div>
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>

              <el-button
                type="success"
                :loading="pasteConverting"
                :disabled="pasteConverting || !pasteText.trim()"
                size="default"
                @click="convertPastedText"
              >
                {{ pasteConverting ? (pasteProgress || '转换中…') : '生成文件' }}
              </el-button>
            </div>

            <div class="clipboard-explanation">
              <span class="clip-tag">双模复制</span>
              <span>Foxmail / 邮件自动呈现富文本排版，微信 / 聊天自动使用整洁纯文本</span>
            </div>

            <el-alert v-if="pasteWarning" :title="pasteWarning" type="warning" :closable="false" show-icon />

            <div v-if="pasteOutputPath" class="paste-output-card">
              <span class="output-path" :title="pasteOutputPath">已生成：{{ pasteOutputPath }}</span>
              <el-button link type="primary" size="small" @click="openPasteOutputDir">打开文件夹</el-button>
            </div>
          </div>
        </section>
      </div>
    </ToolWorkspaceShell>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { readFile } from '@tauri-apps/plugin-fs'
import { ArrowDown, Compass, CopyDocument } from '@element-plus/icons-vue'
import { useQueueCancellation } from '../../../core/composables/useQueueCancellation.js'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage, ElNotification } from 'element-plus'
import { convertWithMedia } from '../composables/convertWithMedia.js'
import { copyMarkdownContent, decodeMarkdownBytes } from '../composables/markdownClipboard.js'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import MdConvertScene from '../components/MdConvertScene.vue'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { fileName, parentDir } from '../../../core/filePath.js'
import { openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'

const files = ref([])
const converting = ref(false)
const celebrating = ref(false)
const summary = ref('')
const dragging = ref(false)
const pasteText = ref('')
const pasteFormat = ref('docx')
const pasteFileName = ref('')
const pasteConverting = ref(false)
const pasteOutputPath = ref('')
const pasteWarning = ref('')
const pasteProgress = ref('')
const copyingPath = ref('')
const fileOfficeFormat = ref('docx')
const inputEncoding = ref('auto')
const docxStyle = ref('professional')
const isResizing = ref(false)
let celebrateTimer = null

const docxStyleHints = {
  professional: '宋体正文 + Times New Roman + 商务深蓝标题',
  legal: '仿宋正文 + Times New Roman + 首行缩进两字符',
  compact: '宋体正文 + Times New Roman + 紧凑字号行距',
}
const docxStyleHint = computed(() => docxStyleHints[docxStyle.value] || '')

const pdfStartPage = ref(null)
const pdfEndPage = ref(null)
const activeBackendOperationId = ref('')
const pastePanelRatio = ref(0.48)
const convertWorkspaceRef = ref(null)
let stopConvertResize = null

const queueCancellation = useQueueCancellation()
const PDF_CONVERT_OPERATION_ID = 'convert_pdf_text_layer:auto'

const preference = useWorkspacePreferences('markdown-convert.workspace', {
  inputEncoding,
  fileOfficeFormat,
  docxStyle,
  pasteFormat,
  pastePanelRatio,
})

const hasPdfInQueue = computed(() => files.value.some((item) => extensionOf(item.path) === 'pdf'))
const hasPendingFiles = computed(() =>
  files.value.some((item) => ['pending', 'failed', 'cancelled'].includes(item.status)),
)

const activeConvertingStatus = computed(() => {
  const current = files.value.find((f) => f.status === 'processing')
  return current ? current.statusText : 'Doclet 正在排版生成文档…'
})

const convertGridStyle = computed(() => {
  const paste = Math.round(clampPastePanelRatio(pastePanelRatio.value) * 100)
  return { gridTemplateColumns: `minmax(320px, ${100 - paste}fr) 14px minmax(320px, ${paste}fr)` }
})

function clampPastePanelRatio(value) {
  return Math.min(0.65, Math.max(0.30, Number(value) || 0.48))
}

function setPastePanelRatio(value) {
  pastePanelRatio.value = clampPastePanelRatio(value)
}

function triggerCelebration() {
  if (celebrateTimer) clearTimeout(celebrateTimer)
  celebrating.value = true
  celebrateTimer = setTimeout(() => {
    celebrating.value = false
    celebrateTimer = null
  }, 2800)
}

function startConvertResize(event) {
  if (event.button !== 0 || !convertWorkspaceRef.value) return
  event.preventDefault()
  isResizing.value = true

  const resizerEl = event.currentTarget
  if (resizerEl?.setPointerCapture) {
    try {
      resizerEl.setPointerCapture(event.pointerId)
    } catch {
      // ignore
    }
  }

  const update = (pointerEvent) => {
    if (!convertWorkspaceRef.value) return
    const rect = convertWorkspaceRef.value.getBoundingClientRect()
    if (rect.width <= 0) return
    setPastePanelRatio((rect.right - pointerEvent.clientX) / rect.width)
  }

  const finish = (pointerEvent) => {
    isResizing.value = false
    if (resizerEl?.releasePointerCapture && pointerEvent?.pointerId) {
      try {
        resizerEl.releasePointerCapture(pointerEvent.pointerId)
      } catch {
        // ignore
      }
    }
    window.removeEventListener('pointermove', update)
    window.removeEventListener('pointerup', finish)
    window.removeEventListener('pointercancel', finish)
    stopConvertResize = null
    void preference.save()
  }

  stopConvertResize?.()
  stopConvertResize = () => finish(event)
  window.addEventListener('pointermove', update)
  window.addEventListener('pointerup', finish, { once: true })
  window.addEventListener('pointercancel', finish, { once: true })
}

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
const htmlExtensions = ['html', 'htm']
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
    markdownExtensions.includes(extension) ||
    officeExtensions.includes(extension) ||
    htmlExtensions.includes(extension) ||
    pdfExtensions.includes(extension)
  )
}

function directionTag(path, outputFormat = fileOfficeFormat.value) {
  if (isMarkdownPath(path)) {
    return `MD→${{ docx: 'Word', xlsx: 'Excel', pptx: 'PowerPoint', html: 'HTML' }[outputFormat] || 'Office'}`
  }
  const extension = extensionOf(path)
  if (extension === 'pdf') return 'PDF→MD'
  if (htmlExtensions.includes(extension)) return 'HTML→MD'
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
      { name: 'Markdown / Office / HTML / PDF', extensions: [...markdownExtensions, ...officeExtensions, ...htmlExtensions, ...pdfExtensions] },
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
  const done = processed.filter((item) => item.status === 'done')
  const failed = processed.filter((item) => item.status === 'failed')
  const cancelled = queueCancellation.cancelled.value || processed.some((item) => item.status === 'cancelled')
  summary.value = `${cancelled ? '转换已停止，已完成' : '转换完成'} ${done.length}/${processed.length}`
  if (failed.length) summary.value += `；失败：${failed.map((item) => item.name).join('、')}`

  if (done.length > 0) {
    triggerCelebration()
  }

  ElNotification({
    type: failed.length ? 'warning' : cancelled ? 'info' : 'success',
    title: summary.value,
    duration: 6000,
  })
}

async function copyPastedMarkdown(mode) {
  const text = pasteText.value.trim()
  if (!text || pasteConverting.value) return
  try {
    await copyMarkdownContent(text, mode)
    ElMessage.success(
      mode === 'plain'
        ? '已复制为纯文本，可直接粘贴到微信 / 聊天窗口'
        : '已复制为富文本，可直接粘贴到 Foxmail / 邮件编辑器',
    )
  } catch (err) {
    ElMessage.error(userFacingError(err, '复制失败，请检查剪贴板权限'))
  }
}

async function copyMarkdownFile(item) {
  if (!item?.path || copyingPath.value) return
  copyingPath.value = item.path
  try {
    const bytes = await readFile(item.path)
    const text = decodeMarkdownBytes(bytes, inputEncoding.value)
    if (!text.trim()) {
      ElMessage.warning('这个 Markdown 文件没有可复制的内容')
      return
    }
    await copyMarkdownContent(text, 'rich')
    ElMessage.success('已复制为富文本+纯文本，可直接粘贴到 Foxmail 或微信')
  } catch (err) {
    ElMessage.error(userFacingError(err, '复制失败，请检查文件编码或剪贴板权限'))
  } finally {
    copyingPath.value = ''
  }
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
      triggerCelebration()
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
  stopConvertResize?.()
  if (celebrateTimer) clearTimeout(celebrateTimer)
  void preference.stop()
})

defineExpose({
  files,
  converting,
  hasPendingFiles,
  fileOfficeFormat,
  docxStyle,
  loadFiles,
  runQueue,
  clearFiles,
  removeFile,
})
</script>

<style scoped>
.markdown-convert-view {
  position: relative;
  height: 100%;
  min-height: 0;
}

/* ── Main Two-Column Workspace ── */
.convert-workspace {
  display: grid;
  grid-template-columns: minmax(320px, 1.08fr) 14px minmax(320px, 0.92fr);
  gap: 0;
  min-height: 100%;
  height: 100%;
}

.convert-workspace.is-resizing {
  cursor: col-resize !important;
  user-select: none !important;
}

.convert-workspace.is-resizing * {
  pointer-events: none !important;
  user-select: none !important;
}

.convert-workspace.is-resizing .convert-resizer {
  pointer-events: auto !important;
}

/* ── Conversion Panel Shell ── */
.conversion-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  padding: clamp(14px, 2.5dvh, 18px);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-panel);
  overflow: hidden;
}

/* ── Panel Header Row ── */
.panel-header-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.title-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.title-with-badge {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-with-badge h3 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.2px;
}

.direction-badge {
  display: inline-flex;
  align-items: center;
  padding: 1.5px 7px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: color-mix(in srgb, var(--docsy-primary) 10%, transparent);
  color: var(--docsy-primary);
  border: 1px solid color-mix(in srgb, var(--docsy-primary) 20%, transparent);
}

.char-count-badge {
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-family: var(--docsy-font-mono, monospace);
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-muted);
}

.panel-desc {
  margin: 0;
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.clear-text-btn {
  font-size: 12px;
  color: var(--docsy-text-muted);
  padding: 0;
}

.clear-text-btn:hover {
  color: var(--el-color-danger);
}

/* ── Format & Settings Toolbar ── */
.format-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px 14px;
  padding: 8px 10px;
  margin-bottom: 10px;
  border-radius: var(--docsy-radius-sm);
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
}

.format-toolbar.compact {
  padding: 8px 10px;
  margin-bottom: 10px;
}

.format-group,
.style-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--docsy-text-muted);
  white-space: nowrap;
}

.style-select {
  width: 140px;
}

.feature-chips {
  display: flex;
  align-items: center;
  margin-left: auto;
}

.feature-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 999px;
  font-size: 11.5px;
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-elevated);
  border: 1px solid var(--docsy-border-subtle);
  cursor: help;
  transition: all 0.2s ease;
}

.feature-pill:hover {
  color: var(--docsy-primary);
  border-color: color-mix(in srgb, var(--docsy-primary) 30%, transparent);
}

/* ── PDF Convert Note ── */
.pdf-convert-note {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 12px;
  padding: 7px 10px;
  margin-bottom: 10px;
  border-radius: var(--docsy-radius-sm);
  background: color-mix(in srgb, var(--docsy-primary) 6%, transparent);
  border-left: 3px solid var(--docsy-primary);
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.pdf-note-title {
  font-weight: 600;
  color: var(--docsy-text-strong);
}

.pdf-page-range {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.pdf-page-range .el-input-number {
  width: 90px;
}

.option-sep {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

/* ── File Queue Panel in Left Column ── */
.file-panel > :deep(.file-queue-panel) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* ── Left Panel Bottom Footer Bar ── */
.panel-bottom-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 10px;
  padding-top: 8px;
  border-top: 1px solid var(--docsy-border-subtle);
}

.encoding-compact-setting {
  display: flex;
  align-items: center;
  gap: 8px;
}

.compact-label {
  font-size: 11.5px;
  color: var(--docsy-text-muted);
  white-space: nowrap;
}

.encoding-compact-select {
  width: 175px;
}

.encoding-subtle-hint {
  font-size: 11px;
  color: var(--docsy-text-muted);
  opacity: 0.8;
}

.summary-badge {
  font-size: 12px;
  font-weight: 500;
  color: var(--docsy-primary);
  background: color-mix(in srgb, var(--docsy-primary) 8%, transparent);
  padding: 3px 8px;
  border-radius: 4px;
}

.queue-warning {
  flex-basis: 100%;
  color: var(--el-color-warning-dark-2);
  font-size: 11px;
  line-height: 1.45;
}

/* ── Center Resizer Divider ── */
.convert-resizer {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  cursor: col-resize;
  user-select: none;
  touch-action: none;
  position: relative;
  z-index: 5;
  transition: background 0.2s ease;
}

.convert-resizer:hover,
.convert-resizer.is-resizing {
  background: color-mix(in srgb, var(--docsy-primary) 6%, transparent);
}

.resizer-thumb {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  width: 6px;
  height: 38px;
  border-radius: 999px;
  background: var(--docsy-border-strong);
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}

.grip-dot {
  width: 2px;
  height: 2px;
  border-radius: 50%;
  background: var(--docsy-surface-elevated);
}

.convert-resizer:hover .resizer-thumb,
.convert-resizer.is-resizing .resizer-thumb {
  width: 8px;
  height: 48px;
  background: var(--docsy-primary);
  box-shadow: 0 0 10px color-mix(in srgb, var(--docsy-primary) 40%, transparent);
}

/* ── Right Panel: Paste Markdown ── */
.paste-panel {
  display: flex;
  flex-direction: column;
}

.paste-editor-wrapper {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 220px;
}

.paste-editor-wrapper :deep(.el-textarea) {
  display: flex;
  flex: 1;
  height: 100%;
}

.paste-editor-wrapper :deep(.el-textarea__inner) {
  flex: 1;
  height: 100% !important;
  min-height: 200px;
  resize: none;
  font-family: var(--docsy-font-mono, monospace), system-ui;
  font-size: 13px;
  line-height: 1.6;
  padding: 12px 14px;
  border-radius: var(--docsy-radius-sm);
  background: var(--docsy-surface);
  border-color: var(--docsy-border-subtle);
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.paste-editor-wrapper :deep(.el-textarea__inner:focus) {
  border-color: var(--docsy-primary);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--docsy-primary) 16%, transparent);
}

.paste-footer-controls {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
}

.paste-action-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.file-name-input {
  flex: 1 1 160px;
}

.dropdown-item-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 2px 0;
}

.dropdown-item-meta strong {
  font-size: 13px;
  color: var(--docsy-text-strong);
}

.dropdown-item-meta small {
  font-size: 11px;
  color: var(--docsy-text-muted);
}

.clipboard-explanation {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--docsy-text-muted);
  line-height: 1.4;
}

.clip-tag {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 3px;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  color: var(--docsy-primary);
  white-space: nowrap;
}

.paste-output-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  border-radius: var(--docsy-radius-sm);
  background: color-mix(in srgb, var(--docsy-primary) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--docsy-primary) 20%, transparent);
  font-size: 12px;
}

.output-path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--docsy-text-strong);
}

/* ── Full Window Drop Overlay ── */
.document-drop-overlay {
  position: absolute;
  z-index: 50;
  display: grid;
  inset: 12px;
  pointer-events: none;
  border: 2px dashed var(--docsy-primary);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface) 90%, transparent);
  place-items: center;
}

.document-drop-message {
  padding: 12px 24px;
  color: var(--docsy-primary);
  border-radius: var(--docsy-radius);
  background: var(--docsy-primary-soft);
  font-size: 14px;
  font-weight: 600;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}

/* ── Responsive Stacking ── */
@media (max-width: 1180px) {
  .convert-workspace {
    grid-template-columns: 1fr !important;
    gap: 14px;
  }

  .convert-resizer {
    display: none;
  }
}
</style>

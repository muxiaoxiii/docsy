<template>
  <div class="pdf-tools-view" :class="{ 'is-file-dragging': pdfDragging }">
    <div v-if="pdfDragging" class="pdf-drop-overlay">
      <div class="pdf-drop-message">松开以添加 PDF 文件</div>
    </div>
    <el-tabs v-model="activeTab" tab-position="left" class="pdf-tabs">
      <el-tab-pane label="解锁" name="unlock" lazy>
        <ToolWorkspaceShell title="PDF 解锁" description="移除 PDF 文件的密码保护，原文件旁会生成已解锁副本。">
          <template #toolbar>
            <el-button type="primary" @click="selectUnlockFiles">选择 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            :items="unlockFiles"
            empty-text="选择一个或多个需要解锁的 PDF 文件"
            @clear="clearUnlockFiles"
            @remove="removeUnlockFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
            </template>
          </FileQueuePanel>
          <template #actions>
            <el-button
              type="success"
              @click="batchUnlock"
              :loading="unlocking"
              :disabled="unlockReadyCount === 0 || unlockInspecting"
            >
              解锁 {{ unlockReadyCount }} 个加密文件
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="合并" name="merge" lazy>
        <ToolWorkspaceShell title="PDF 合并" description="按列表顺序合并多个 PDF，输出文件由你选择保存位置。">
          <template #toolbar>
            <el-button type="primary" @click="selectMergeFiles">添加 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            :items="mergeFiles"
            sortable
            empty-text="添加至少两个需要合并的 PDF 文件"
            @clear="clearMergeFiles"
            @remove="removeMergeFile"
            @reorder="reorderMergeFiles"
          />
          <template #actions>
            <el-button type="success" @click="doMerge" :loading="merging" :disabled="mergeFiles.length < 2">
              合并为一个 PDF
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="提取页面" name="extract" lazy>
        <ToolWorkspaceShell
          title="快速提取页面"
          description="从一个 PDF 中挑选若干页导出，支持输入 3,7,12-15 这样的页码。"
        >
          <template #toolbar>
            <el-button type="primary" @click="selectExtractFile">选择 PDF</el-button>
            <el-button :disabled="!extractFile" @click="selectExtractOutputDir">输出文件夹</el-button>
          </template>
          <div v-if="extractFile" class="path-line">{{ extractFile }}</div>
          <div v-if="extractOutputDir" class="path-line">{{ extractOutputDir }}</div>
          <div v-if="extractFile" class="simple-tool-form">
            <el-input v-model="extractPageText" placeholder="例如：3,7,12-15" clearable @keyup.enter="doExtractPages">
              <template #prepend>页码</template>
            </el-input>
            <div class="path-hint">共 {{ extractTotalPages || '-' }} 页；重复页会自动忽略。</div>
            <el-button
              type="success"
              :loading="extractingPages"
              :disabled="!extractFile || !extractPageText.trim()"
              @click="doExtractPages"
            >
              导出选中页面
            </el-button>
          </div>
          <el-empty v-else description="先选择需要提取页面的 PDF 文件" />
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="压缩" name="compress" lazy>
        <ToolWorkspaceShell
          title="PDF 压缩整理"
          description="使用 qpdf 重新压缩流、整理对象并移除未引用资源，不改变页面内容。"
        >
          <template #toolbar>
            <el-button type="primary" @click="selectCompressFile">选择 PDF</el-button>
            <el-button :disabled="!compressFile" @click="selectCompressOutputDir">输出文件夹</el-button>
          </template>
          <div v-if="compressFile" class="path-line">{{ compressFile }}</div>
          <div v-if="compressOutputDir" class="path-line">{{ compressOutputDir }}</div>
          <el-empty v-if="!compressFile" description="先选择需要压缩整理的 PDF 文件" />
          <template #actions>
            <el-button type="success" :loading="compressing" :disabled="!compressFile" @click="doCompressPdf">
              压缩 PDF
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="拆分" name="split" lazy>
        <ToolWorkspaceShell title="PDF 拆分" description="翻页核对内容，按页码范围生成多个独立 PDF 文件。">
          <template #toolbar>
            <el-button type="primary" @click="selectSplitFile">选择 PDF</el-button>
            <el-button :disabled="!splitFile" @click="selectSplitOutputDir">输出文件夹</el-button>
            <el-button :disabled="!splitFile" @click="addSplitRange">添加页段</el-button>
          </template>
          <div v-if="splitFile" class="path-line">{{ splitFile }}</div>
          <div v-if="splitOutputDir" class="path-line">{{ splitOutputDir }}</div>

          <div v-if="splitFile" class="split-main">
            <section class="split-list-panel">
              <div class="split-cleanup-panel">
                <div class="split-cleanup-title">拆分后处理</div>
                <el-checkbox v-model="splitCleanupHeader">删除页眉区内容</el-checkbox>
                <el-checkbox v-model="splitCleanupFooter">删除原页码/页脚区内容</el-checkbox>
                <div v-if="splitCleanupHeader || splitCleanupFooter" class="split-cleanup-zones">
                  <el-input-number
                    v-if="splitCleanupHeader"
                    v-model="splitCleanupHeaderHeightMm"
                    :min="6"
                    :max="60"
                    :step="1"
                    size="small"
                  />
                  <span v-if="splitCleanupHeader">页眉区 mm</span>
                  <el-input-number
                    v-if="splitCleanupFooter"
                    v-model="splitCleanupFooterHeightMm"
                    :min="6"
                    :max="60"
                    :step="1"
                    size="small"
                  />
                  <span v-if="splitCleanupFooter">页脚区 mm</span>
                </div>
              </div>
              <el-alert v-if="splitWarnings.length" type="warning" :closable="false" show-icon class="split-warning">
                <template #title>页段需要核对（{{ splitWarnings.length }}）</template>
                <ul class="warning-list">
                  <li v-for="warning in splitWarnings" :key="warning">{{ warning }}</li>
                </ul>
              </el-alert>
              <el-table
                v-if="splitRanges.length"
                :data="splitRanges"
                size="small"
                border
                class="range-table"
                highlight-current-row
                @row-click="previewSplitRange"
              >
                <el-table-column width="42" align="center">
                  <template #default="{ $index }">
                    <button
                      type="button"
                      class="range-drag-handle"
                      :data-split-reorder-index="$index"
                      title="拖动调整页段顺序"
                      @pointerdown.stop="splitReorder.start($index, $event)"
                      @pointermove.stop="splitReorder.move"
                      @pointerup.stop="splitReorder.finish"
                      @pointercancel.stop="splitReorder.reset"
                    >
                      <el-icon><Rank /></el-icon>
                    </button>
                  </template>
                </el-table-column>
                <el-table-column type="index" label="#" width="44" />
                <el-table-column label="文件名" min-width="170">
                  <template #default="{ row }">
                    <el-input v-model="row.name" size="small" />
                  </template>
                </el-table-column>
                <el-table-column label="起始页" width="112">
                  <template #default="{ row }">
                    <el-input-number v-model="row.pageStart" :min="1" :max="splitPreviewMaxPage" size="small" />
                  </template>
                </el-table-column>
                <el-table-column label="结束页" width="112">
                  <template #default="{ row }">
                    <el-input-number v-model="row.pageEnd" :min="1" :max="splitPreviewMaxPage" size="small" />
                  </template>
                </el-table-column>
                <el-table-column label="页数" width="64">
                  <template #default="{ row }">{{ splitRangePageCount(row) || '-' }}</template>
                </el-table-column>
                <el-table-column label="状态" width="72">
                  <template #default="{ row }">
                    <el-tag :type="splitRangeStatus(row).type" size="small">{{ splitRangeStatus(row).text }}</el-tag>
                  </template>
                </el-table-column>
                <el-table-column label="操作" width="148">
                  <template #default="{ row, $index }">
                    <el-button link type="primary" size="small" @click.stop="previewSplitRange(row)">预览</el-button>
                    <el-button link type="primary" size="small" @click.stop="insertSplitRangeAfter($index)"
                      >续段</el-button
                    >
                    <el-button link type="danger" size="small" @click.stop="removeSplitRange($index)">删除</el-button>
                  </template>
                </el-table-column>
              </el-table>
              <el-button
                v-if="splitRanges.length"
                type="success"
                @click="doSplitMerged"
                :loading="splittingMerged"
                :disabled="!splitFile || !splitOutputDir || splitWarnings.length > 0"
              >
                执行拆分
              </el-button>
            </section>

            <section class="split-preview">
              <div class="split-preview-head">
                <div>
                  <span>页面预览</span>
                  <div class="split-preview-status">
                    当前页 {{ splitPreviewPage }} / {{ splitPreviewMaxPage }}
                    <span v-if="selectedSplitRange"
                      >；选中 {{ selectedSplitRange.name || '未命名' }} {{ selectedSplitRange.pageStart }}-{{
                        selectedSplitRange.pageEnd
                      }}</span
                    >
                  </div>
                </div>
                <div class="split-preview-actions">
                  <el-button size="small" :disabled="splitPreviewPage <= 1" @click="moveSplitPreviewPage(-1)"
                    >上一页</el-button
                  >
                  <el-input-number v-model="splitPreviewPage" :min="1" :max="splitPreviewMaxPage" size="small" />
                  <el-button
                    size="small"
                    :disabled="splitPreviewPage >= splitPreviewMaxPage"
                    @click="moveSplitPreviewPage(1)"
                    >下一页</el-button
                  >
                  <el-button size="small" :disabled="!selectedSplitRange" @click="setSelectedSplitStart"
                    >设为起始页</el-button
                  >
                  <el-button size="small" :disabled="!selectedSplitRange" @click="setSelectedSplitEnd"
                    >设为结束页</el-button
                  >
                </div>
              </div>
              <PdfJsPreview
                v-if="activeTab === 'split' && splitFile"
                :file-path="splitFile"
                :page="splitPreviewPage"
                :scale="0.9"
                @error="(message) => ElMessage.error(message)"
              />
            </section>
          </div>
          <div v-else class="split-empty">
            <p>选择 PDF 后，可以翻页预览并设置每个拆分文件的起止页。</p>
          </div>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="防OCR" name="anti-ocr" lazy>
        <ToolWorkspaceShell title="防OCR处理" description="检测或添加 PDF 文字防提取保护。保留视觉效果，干扰自动化文字提取。">
          <template #toolbar>
            <el-button type="primary" @click="selectAntiOcrFiles">选择 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            :items="antiOcrFiles"
            empty-text="选择一个或多个 PDF 文件进行防OCR检测"
            @clear="clearAntiOcrFiles"
            @remove="removeAntiOcrFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <el-tag v-if="item.hasAntiOcr" type="warning" size="small">已防护</el-tag>
            </template>
          </FileQueuePanel>
          <template #actions>
            <el-button
              type="success"
              @click="batchAntiOcrApply"
              :loading="antiOcrProcessing"
              :disabled="antiOcrReadyCount === 0"
            >
              添加防OCR {{ antiOcrReadyCount }} 个文件
            </el-button>
            <el-button
              @click="batchAntiOcrRemove"
              :loading="antiOcrProcessing"
              :disabled="antiOcrProtectedCount === 0"
            >
              移除防OCR {{ antiOcrProtectedCount }} 个文件
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Rank } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../components/PdfJsPreview.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import { splitRangeWarnings } from '../composables/usePdfSplitRanges.js'
import { getPdfPageCount, tauriCallSafe } from '../../../core/tauriBridge.js'
import { fileName, stripPdf } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { usePointerReorder } from '../../../core/composables/usePointerReorder.js'
import {
  buildRangeAfter,
  insertRangeAfter,
  navigatePage,
  parsePageSelection,
  removeRangeAt,
  setRangeEnd,
  setRangeStart,
} from '../../../core/pdfUtils.js'

const activeTab = ref('unlock')
const pdfDragging = ref(false)
const splitReorder = usePointerReorder({
  itemCount: () => splitRanges.value.length,
  itemAttribute: 'data-split-reorder-index',
  onReorder: ({ from, to }) => reorderSplitRanges(from, to),
})

const unlockFiles = ref([])
const unlocking = ref(false)
const unlockReadyCount = computed(() => unlockFiles.value.filter((file) => file.encrypted === true).length)
const unlockInspecting = computed(() => unlockFiles.value.some((file) => file.inspecting))

async function selectUnlockFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    addUnlockFiles(paths, true)
  }
}

function addUnlockFiles(paths, replace = false) {
  const existing = replace ? new Set() : new Set(unlockFiles.value.map((file) => file.path))
  const candidates = [...new Set(paths)].filter((path) => !existing.has(path))
  const items = makeQueueItems(candidates).map((item) => ({
    ...item,
    encrypted: null,
    inspecting: true,
    statusText: '检测中',
  }))
  unlockFiles.value = replace ? items : [...unlockFiles.value, ...items]
  if (items.length) void inspectUnlockFiles(candidates)
}

async function inspectUnlockFiles(paths) {
  const reactive = unlockFiles.value
  let nextIndex = 0
  const workerCount = Math.min(4, paths.length)
  const workers = Array.from({ length: workerCount }, async () => {
    while (nextIndex < paths.length) {
      const path = paths[nextIndex]
      nextIndex += 1
      try {
        const result = await Promise.race([
          tauriCallSafe('inspect_pdf', { input: path }),
          new Promise((_, reject) => setTimeout(() => reject(new Error('检测超时（10秒）')), 10000)),
        ])
        const item = reactive.find((f) => f.path === path)
        if (!item) continue
        item.inspecting = false
        if (!result.ok) {
          item.encrypted = null
          item.statusText = result.error || '检测失败'
          item.statusType = 'danger'
        } else if (result.data.encrypted) {
          item.encrypted = true
          item.statusText = '已加密，待解锁'
          item.statusType = 'warning'
        } else {
          item.encrypted = false
          item.statusText = '未加密，无需处理'
          item.statusType = 'info'
        }
      } catch (err) {
        const item = reactive.find((f) => f.path === path)
        if (item) {
          item.inspecting = false
          item.encrypted = null
          item.statusText = userFacingError(err?.message || err, '检测异常')
          item.statusType = 'danger'
        }
      }
    }
  })
  await Promise.all(workers)
}

function clearUnlockFiles() {
  unlockFiles.value = []
}

function removeUnlockFile(index) {
  unlockFiles.value.splice(index, 1)
}

async function batchUnlock() {
  const encryptedFiles = unlockFiles.value.filter((file) => file.encrypted === true)
  if (!encryptedFiles.length) {
    ElMessage.info('所选文件均未加密，无需处理')
    return
  }
  unlocking.value = true
  let successCount = 0
  try {
    for (const file of encryptedFiles) {
      file.statusText = '处理中'
      file.statusType = 'warning'
      const result = await tauriCallSafe('unlock_pdf', { input: file.path })
      if (result.ok && !result.data.skipped) {
        file.statusText = '解锁成功'
        file.statusType = 'success'
        file.encrypted = false
        successCount += 1
      } else if (result.ok) {
        file.statusText = '未加密，已跳过'
        file.statusType = 'info'
        file.encrypted = false
      } else {
        file.statusText = result.error || '解锁失败'
        file.statusType = 'danger'
      }
    }
  } finally {
    unlocking.value = false
  }
  if (successCount) ElMessage.success(`已解锁 ${successCount} 个文件`)
}

const mergeFiles = ref([])
const merging = ref(false)
const extractFile = ref('')
const extractOutputDir = ref('')
const extractPageText = ref('')
const extractTotalPages = ref(0)
const extractingPages = ref(false)
const compressFile = ref('')
const compressOutputDir = ref('')
const compressing = ref(false)
const splitFile = ref('')
const splitOutputDir = ref('')
const splitRanges = ref([])
const selectedSplitRangeIndex = ref(0)
const splittingMerged = ref(false)
const splitPreviewPage = ref(1)
const splitTotalPages = ref(1)
const splitRunWarnings = ref([])
const splitCleanupHeader = ref(false)
const splitCleanupFooter = ref(false)
const splitCleanupHeaderHeightMm = ref(18)
const splitCleanupFooterHeightMm = ref(18)
const splitWarnings = computed(() => [
  ...splitRangeWarnings(splitRanges.value, splitTotalPages.value),
  ...splitRunWarnings.value,
])
const splitPreviewMaxPage = computed(() => Math.max(1, splitTotalPages.value || 1))
const selectedSplitRange = computed(() => splitRanges.value[selectedSplitRangeIndex.value] || null)

// Anti-OCR
const antiOcrFiles = ref([])
const antiOcrProcessing = ref(false)
const antiOcrReadyCount = computed(() => antiOcrFiles.value.filter((f) => !f.hasAntiOcr).length)
const antiOcrProtectedCount = computed(() => antiOcrFiles.value.filter((f) => f.hasAntiOcr).length)

function addAntiOcrFiles(paths) {
  const existing = new Set(antiOcrFiles.value.map((f) => f.path))
  const newItems = paths.filter((p) => !existing.has(p)).map((p) => ({
    path: p,
    name: fileName(p),
    statusText: '检测中',
    statusType: 'info',
    hasAntiOcr: null,
  }))
  antiOcrFiles.value = [...antiOcrFiles.value, ...newItems]
  const newPaths = newItems.map((i) => i.path)
  if (newPaths.length) void inspectAntiOcrFiles(newPaths)
}

async function inspectAntiOcrFiles(paths) {
  const reactive = antiOcrFiles.value
  for (const path of paths) {
    try {
      const result = await Promise.race([
        tauriCallSafe('detect_anti_ocr', { input: path }),
        new Promise((_, reject) => setTimeout(() => reject(new Error('检测超时（10秒）')), 10000)),
      ])
      const item = reactive.find((f) => f.path === path)
      if (!item) continue
      if (!result.ok) {
        item.statusText = result.error || '检测失败'
        item.statusType = 'danger'
      } else {
        item.hasAntiOcr = result.data.has_anti_ocr
        const { total_pages, pages_with_scrambled_cmap } = result.data
        if (result.data.has_anti_ocr) {
          item.statusText = `已防护 (${pages_with_scrambled_cmap}/${total_pages}页)`
          item.statusType = 'warning'
        } else {
          item.statusText = `正常 (${total_pages}页)`
          item.statusType = 'success'
        }
      }
    } catch (err) {
      const item = reactive.find((f) => f.path === path)
      if (item) {
        item.statusText = userFacingError(err?.message || err, '检测异常')
        item.statusType = 'danger'
      }
    }
  }
}

async function selectAntiOcrFiles() {
  const selected = await open({ multiple: true, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!selected) return
  const paths = Array.isArray(selected) ? selected : [selected]
  addAntiOcrFiles(paths)
}

function clearAntiOcrFiles() { antiOcrFiles.value = [] }
function removeAntiOcrFile(index) { antiOcrFiles.value.splice(index, 1) }

async function batchAntiOcrApply() {
  antiOcrProcessing.value = true
  const targets = antiOcrFiles.value.filter((f) => !f.hasAntiOcr)
  for (const file of targets) {
    file.statusText = '处理中'
    file.statusType = 'warning'
    const dir = parentDir(file.path)
    const stem = stripPdf(file.name)
    const output = `${dir}/${stem}_anti_ocr.pdf`
    const result = await tauriCallSafe('apply_anti_ocr', { input: file.path, output })
    if (!result.ok) {
      file.statusText = userFacingError(result.error, '处理失败')
      file.statusType = 'danger'
    } else {
      file.hasAntiOcr = true
      file.statusText = `已防护`
      file.statusType = 'warning'
    }
  }
  antiOcrProcessing.value = false
}

async function batchAntiOcrRemove() {
  antiOcrProcessing.value = true
  const targets = antiOcrFiles.value.filter((f) => f.hasAntiOcr)
  for (const file of targets) {
    file.statusText = '处理中'
    file.statusType = 'warning'
    const dir = parentDir(file.path)
    const stem = stripPdf(file.name)
    const output = `${dir}/${stem}_restored.pdf`
    const result = await tauriCallSafe('remove_anti_ocr', { input: file.path, output })
    if (!result.ok) {
      file.statusText = userFacingError(result.error, '处理失败')
      file.statusType = 'danger'
    } else {
      file.hasAntiOcr = false
      file.statusText = '已恢复'
      file.statusType = 'success'
    }
  }
  antiOcrProcessing.value = false
}

async function selectMergeFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    addMergeFiles(paths)
  }
}

function addMergeFiles(paths) {
  const existing = new Set(mergeFiles.value.map((file) => file.path))
  const candidates = [...new Set(paths)].filter((path) => !existing.has(path))
  mergeFiles.value.push(...makeQueueItems(candidates))
}

function clearMergeFiles() {
  mergeFiles.value = []
}

function removeMergeFile(index) {
  mergeFiles.value.splice(index, 1)
}

function reorderMergeFiles({ from, to }) {
  if (from === to || from < 0 || to < 0 || from >= mergeFiles.value.length || to >= mergeFiles.value.length) return
  const [item] = mergeFiles.value.splice(from, 1)
  mergeFiles.value.splice(to, 0, item)
}

async function doMerge() {
  merging.value = true
  const output = await open({ directory: true })
  if (output) {
    const outputPath = `${output}/merged.pdf`
    const result = await tauriCallSafe('merge_pdfs', {
      inputs: mergeFiles.value.map((file) => file.path),
      output: outputPath,
    })
    result.ok ? ElMessage.success('合并完成') : ElMessage.error(result.error || 'PDF 合并失败，请确认文件未损坏且未被其他程序占用')
  }
  merging.value = false
}

function makeQueueItems(paths) {
  return paths.map((path) => ({
    path,
    name: fileName(path),
    statusText: '等待处理',
    statusType: 'info',
  }))
}

async function selectExtractFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!selected) return
  await loadExtractFile(normalizeSelectedPath(selected))
}

async function loadExtractFile(path) {
  extractFile.value = path
  extractPageText.value = ''
  extractTotalPages.value = 0
  const pageCount = await getPdfPageCount(extractFile.value)
  if (pageCount.ok) {
    extractTotalPages.value = pageCount.data || 0
  } else {
    ElMessage.error(pageCount.error || '读取页数失败')
  }
}

async function selectExtractOutputDir() {
  const selected = await open({ directory: true })
  if (selected) extractOutputDir.value = normalizeSelectedPath(selected)
}

async function doExtractPages() {
  if (!extractFile.value) return
  const pages = parsePageSelection(extractPageText.value, extractTotalPages.value)
  if (!pages.length) {
    ElMessage.warning('请输入有效页码，例如：3,7,12-15')
    return
  }
  extractingPages.value = true
  const result = await tauriCallSafe('extract_pdf_pages', {
    input: extractFile.value,
    pages,
    output_dir: extractOutputDir.value || null,
  })
  extractingPages.value = false
  if (result.ok) {
    ElMessage.success(`已导出：${result.data.output_path}`)
  } else {
    ElMessage.error(result.error || 'PDF 页面提取失败，请确认文件未损坏')
  }
}

async function selectCompressFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) compressFile.value = normalizeSelectedPath(selected)
}

async function selectCompressOutputDir() {
  const selected = await open({ directory: true })
  if (selected) compressOutputDir.value = normalizeSelectedPath(selected)
}

async function doCompressPdf() {
  if (!compressFile.value) return
  compressing.value = true
  const result = await tauriCallSafe('compress_pdf', {
    input: compressFile.value,
    output_dir: compressOutputDir.value || null,
  })
  compressing.value = false
  if (result.ok) {
    ElMessage.success(`已压缩：${result.data.output_path}`)
  } else {
    ElMessage.error(result.error || 'PDF 压缩失败，请确认文件未损坏且磁盘空间充足')
  }
}

async function selectSplitFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) await loadSplitFile(normalizeSelectedPath(selected))
}

async function loadSplitFile(path) {
  splitFile.value = path
  splitPreviewPage.value = 1
  splitTotalPages.value = 1
  splitRunWarnings.value = []
  const pageCount = await getPdfPageCount(path)
  if (pageCount.ok) {
    splitTotalPages.value = pageCount.data || 1
  }
  splitRanges.value = [
    {
      name: stripPdf(fileName(path)),
      pageStart: 1,
      pageEnd: splitTotalPages.value,
    },
  ]
  selectedSplitRangeIndex.value = 0
}

async function selectSplitOutputDir() {
  const selected = await open({ directory: true })
  if (selected) splitOutputDir.value = selected
}

function normalizeSelectedPath(value) {
  return Array.isArray(value) ? value[0] : value
}

function isPdfPath(path) {
  return /\.pdf$/i.test(String(path || ''))
}

async function handleDroppedPdfPaths(paths) {
  const pdfPaths = [...new Set((paths || []).filter(isPdfPath))]
  if (!pdfPaths.length) {
    ElMessage.warning('请拖入 PDF 文件')
    return
  }
  if (pdfPaths.length < (paths || []).length) ElMessage.warning('已忽略非 PDF 文件')

  if (activeTab.value === 'unlock') {
    addUnlockFiles(pdfPaths)
  } else if (activeTab.value === 'anti-ocr') {
    addAntiOcrFiles(pdfPaths)
  } else if (activeTab.value === 'merge') {
    addMergeFiles(pdfPaths)
  } else {
    if (pdfPaths.length > 1) ElMessage.info('当前工具一次处理一个 PDF，已使用第一个文件')
    const path = pdfPaths[0]
    if (activeTab.value === 'extract') await loadExtractFile(path)
    if (activeTab.value === 'compress') compressFile.value = path
    if (activeTab.value === 'split') await loadSplitFile(path)
  }
}

useWindowFileDrop({
  onEnter: () => {
    pdfDragging.value = true
  },
  onLeave: () => {
    pdfDragging.value = false
  },
  onDrop: handleDroppedPdfPaths,
})

function addSplitRange() {
  const item = buildRangeAfter(splitRanges.value, splitRanges.value.length - 1, splitPreviewMaxPage.value)
  splitRanges.value.push(item)
  previewSplitRange(item)
}

function previewSplitRange(row) {
  const index = splitRanges.value.indexOf(row)
  if (index >= 0) selectedSplitRangeIndex.value = index
  splitPreviewPage.value = Math.min(Math.max(1, row.pageStart || 1), splitPreviewMaxPage.value)
}

function moveSplitPreviewPage(delta) {
  splitPreviewPage.value = navigatePage(splitPreviewPage.value, delta, splitPreviewMaxPage.value)
}

function setSelectedSplitStart() {
  setRangeStart(selectedSplitRange.value, splitPreviewPage.value)
}

function setSelectedSplitEnd() {
  setRangeEnd(selectedSplitRange.value, splitPreviewPage.value)
}

function insertSplitRangeAfter(index) {
  const item = insertRangeAfter(splitRanges.value, index, splitPreviewMaxPage.value)
  previewSplitRange(item)
}

function removeSplitRange(index) {
  selectedSplitRangeIndex.value = removeRangeAt(splitRanges.value, index, selectedSplitRangeIndex.value)
}

function reorderSplitRanges(from, to) {
  if (from === to || from < 0 || to < 0 || from >= splitRanges.value.length || to >= splitRanges.value.length) return
  const [item] = splitRanges.value.splice(from, 1)
  splitRanges.value.splice(to, 0, item)
  selectedSplitRangeIndex.value = to
}

async function doSplitMerged() {
  if (!splitFile.value || !splitOutputDir.value || !splitRanges.value.length) return
  const warnings = splitRangeWarnings(splitRanges.value, splitTotalPages.value)
  if (warnings.length) {
    ElMessage.warning(`请先核对页段：${warnings[0]}`)
    return
  }
  splittingMerged.value = true
  const result = await tauriCallSafe('split_merged_evidence_pdf', {
    args: {
      inputPath: splitFile.value,
      outputDir: splitOutputDir.value,
      items: splitRanges.value,
      cleanup: {
        headerEnabled: splitCleanupHeader.value,
        footerEnabled: splitCleanupFooter.value,
        headerHeightMm: splitCleanupHeaderHeightMm.value,
        footerHeightMm: splitCleanupFooterHeightMm.value,
      },
    },
  })
  if (result.ok) {
    splitTotalPages.value = result.data.totalPages || splitTotalPages.value
    splitRunWarnings.value = result.data.warnings || []
    const failed = result.data.failed?.length || 0
    const outputs = result.data.outputs?.length || 0
    failed
      ? ElMessage.warning(`已拆分 ${outputs} 个，失败 ${failed} 个`)
      : ElMessage.success(`已拆分 ${outputs} 个 PDF`)
  } else {
    ElMessage.error(result.error || 'PDF 拆分失败，请确认文件未损坏且页码范围正确')
  }
  splittingMerged.value = false
}

function splitRangePageCount(row) {
  const pageStart = Number(row?.pageStart || 0)
  const pageEnd = Number(row?.pageEnd || 0)
  return pageStart > 0 && pageEnd >= pageStart ? pageEnd - pageStart + 1 : 0
}

function splitRangeStatus(row) {
  const pageStart = Number(row?.pageStart || 0)
  const pageEnd = Number(row?.pageEnd || 0)
  const total = Number(splitTotalPages.value || 0)
  if (!String(row?.name || '').trim() || !pageStart || !pageEnd || pageStart > pageEnd || pageEnd > total) {
    return { type: 'danger', text: '错误' }
  }
  return { type: 'success', text: '正常' }
}
</script>

<style scoped>
.pdf-tools-view {
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: var(--docsy-surface);
}

.pdf-drop-overlay {
  position: absolute;
  z-index: 30;
  display: grid;
  inset: 12px;
  pointer-events: none;
  border: 2px dashed var(--docsy-primary);
  border-radius: 8px;
  background: color-mix(in srgb, var(--docsy-surface) 88%, transparent);
  place-items: center;
}

.pdf-drop-message {
  padding: 12px 18px;
  color: var(--docsy-primary);
  font-size: 14px;
  font-weight: 600;
  border-radius: 6px;
  background: var(--docsy-primary-soft);
}

.pdf-tabs {
  height: 100%;
}

:deep(.pdf-tabs > .el-tabs__content),
:deep(.pdf-tabs > .el-tabs__content > .el-tab-pane) {
  height: 100%;
  min-width: 0;
}

.tab-content {
  min-height: 100%;
  padding: 20px 24px 32px;
  max-width: 1040px;
  background: var(--docsy-surface);
}

.split-workspace {
  max-width: none;
  height: 100%;
  overflow: auto;
}

h3 {
  margin: 0 0 6px;
  color: var(--docsy-text-strong);
}

.hint {
  color: var(--docsy-text-muted);
  font-size: 13px;
  margin: 0 0 16px;
}

.range-table {
  margin-top: 16px;
}

.range-drag-handle {
  display: inline-grid;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--docsy-text-muted);
  cursor: grab;
  touch-action: none;
  place-items: center;
}

.range-drag-handle:active {
  color: var(--docsy-primary);
  cursor: grabbing;
}

.toolbar-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 10px;
}

.path-line {
  color: var(--docsy-text);
  font-size: 13px;
  margin: 6px 0;
}

.path-hint {
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.4;
}

.simple-tool-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-width: 520px;
  margin-top: 12px;
}

.split-options {
  display: grid;
  grid-template-columns: minmax(150px, 1fr) 120px minmax(170px, 1fr) 120px;
  align-items: center;
  gap: 8px;
  margin: 12px 0;
}

.split-main {
  display: grid;
  grid-template-columns: minmax(520px, 0.95fr) minmax(380px, 1.05fr);
  gap: 16px;
  align-items: start;
}

.split-list-panel,
.split-preview {
  min-width: 0;
}

.split-warning {
  margin: 10px 0;
}

.split-cleanup-panel {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 14px;
  margin-bottom: 10px;
  padding: 8px 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
}

.split-cleanup-title {
  color: var(--docsy-text-strong);
  font-size: 13px;
  font-weight: 600;
}

.split-cleanup-zones {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  color: var(--docsy-text);
  font-size: 12px;
}

.warning-list {
  margin: 4px 0 0;
  padding-left: 18px;
  line-height: 1.5;
}

.split-preview-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
  color: var(--docsy-text);
  font-size: 13px;
}

.split-preview-status {
  margin-top: 4px;
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.4;
}

.split-preview-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.split-empty {
  display: flex;
  align-items: center;
  min-height: 280px;
  margin-top: 16px;
  padding: 16px;
  border: 1px dashed var(--docsy-border-strong);
  border-radius: 6px;
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-muted);
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--docsy-surface-muted);
  border-radius: 4px;
}

.file-name {
  flex: 1;
  font-size: 13px;
}

@media (max-width: 1180px) {
  .split-main {
    grid-template-columns: 1fr;
  }

  .split-options {
    grid-template-columns: 1fr 120px;
  }
}

.anti-ocr-content {
  padding: 12px 0;
}
.file-path {
  font-size: 13px;
  color: var(--docsy-text-muted);
  margin-bottom: 12px;
  word-break: break-all;
}
.anti-ocr-stats {
  display: flex;
  gap: 16px;
  margin: 12px 0;
  font-size: 13px;
  color: var(--docsy-text);
}
.anti-ocr-message {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
}
.anti-ocr-message.info {
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
}
.anti-ocr-message.success {
  background: #f0fdf4;
  color: #16a34a;
}
.anti-ocr-message.danger {
  background: #fef2f2;
  color: #dc2626;
}
</style>

<template>
  <div class="pdf-tools-view" :class="{ 'is-file-dragging': pdfDragging }">
    <div v-if="pdfDragging" class="pdf-drop-overlay">
      <div class="pdf-drop-message">松开以添加 PDF 文件</div>
    </div>
    <el-tabs v-model="activeTab" tab-position="left" class="pdf-tabs">
      <el-tab-pane label="PDF解锁" name="unlock" lazy>
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

      <el-tab-pane label="PDF合并" name="merge" lazy>
        <ToolWorkspaceShell
          title="PDF 合并"
          description="按列表顺序合并多个 PDF；合并时会压平页面注释和电子签章外观以便打印，不保留签章证书和交互。"
        >
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
            <div class="merge-options">
              <el-checkbox v-model="duplexSeparate">双面打印分隔模式</el-checkbox>
              <span class="merge-option-hint">文件页数为奇数时在末尾补一页空白，让每份文件独立占满双面打印的整张纸</span>
            </div>
            <el-button type="success" @click="doMerge" :loading="merging" :disabled="mergeFiles.length < 2">
              合并为一个 PDF
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="提取PDF页面" name="extract" lazy>
        <ToolWorkspaceShell
          title="提取 PDF 页面"
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
          <WorkspaceEmptyState
            v-else
            title="等待选择 PDF"
            description="选择文件后，可以输入单页或连续页码范围并导出。"
          />
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="PDF压缩" name="compress" lazy>
        <ToolWorkspaceShell
          title="PDF 压缩整理"
          description="默认只做无损结构整理；需要进一步压缩图片时再单独开启，扫描件不会被默认重编码。"
        >
          <template #toolbar>
            <el-button type="primary" @click="selectCompressFiles">选择 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            :items="compressFiles"
            empty-text="选择或拖入一个或多个 PDF，立即压缩并保存到源文件夹"
            @clear="clearCompressFiles"
            @remove="removeCompressFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <span v-if="item.status === 'done' && item.inputSize > 0">{{
                sizeSavingText(item.inputSize, item.outputSize)
              }}</span>
            </template>
          </FileQueuePanel>
          <div v-if="compressSummary" class="path-line">{{ compressSummary }}</div>
          <template #actions>
            <div class="compress-options">
              <el-checkbox v-model="compressImageReencode"> 进一步压缩图片（可能耗时较长） </el-checkbox>
              <div v-if="compressImageReencode" class="compress-level-row">
                <span class="compress-level-label">图片压缩级别：</span>
                <el-radio-group v-model="compressLevel" size="default">
                  <el-radio-button :value="1">清晰优先</el-radio-button>
                  <el-radio-button :value="2">均衡</el-radio-button>
                  <el-radio-button :value="3">体积最小</el-radio-button>
                </el-radio-group>
              </div>
            </div>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="PDF拆分" name="split" lazy>
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
              <div class="split-options-row">
                <el-checkbox v-model="removeBlankPages">删除空白页</el-checkbox>
                <span class="split-option-note">拆分时自动移除无可视内容的空白页（分隔页、扫描背面等）。</span>
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
          <WorkspaceEmptyState
            v-else
            class="split-empty"
            title="等待选择 PDF"
            description="选择文件后，可以翻页预览并设置每个拆分文件的起止页。"
          />
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="PDF防复制" name="anti-ocr" lazy>
        <ToolWorkspaceShell
          title="PDF 防复制"
          description="防止 PDF 文字被复制提取。保留视觉效果，干扰文字选择和复制。"
        >
          <template #toolbar>
            <el-button type="primary" @click="selectAntiOcrFiles">选择 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            :items="antiOcrFiles"
            empty-text="选择一个或多个 PDF 文件进行防复制检测"
            @clear="clearAntiOcrFiles"
            @remove="removeAntiOcrFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <el-tag v-if="item.hasAntiOcr" type="warning" size="small">已防护</el-tag>
            </template>
          </FileQueuePanel>
          <template #actions>
            <el-select v-model="antiCopyMethod" size="small" style="width: 120px">
              <el-option label="CMap 篡改" value="cmap_scramble" />
              <el-option label="CMap 移除" value="cmap_remove" />
            </el-select>
            <el-button
              type="success"
              @click="batchAntiOcrApply"
              :loading="antiOcrProcessing"
              :disabled="antiOcrReadyCount === 0"
            >
              添加防复制 {{ antiOcrReadyCount }} 个文件
            </el-button>
            <el-button @click="batchAntiOcrRemove" :loading="antiOcrProcessing" :disabled="antiOcrProtectedCount === 0">
              移除防复制 {{ antiOcrProtectedCount }} 个文件
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { ElMessage, ElNotification } from 'element-plus'
import { Rank } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../../../shared/pdf-tools/components/PdfJsPreview.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import { splitRangeWarnings } from '../../../shared/pdf-tools/composables/usePdfSplitRanges.js'
import { sizeSavingText, batchSummaryText } from '../../../shared/pdf-tools/composables/pdfLosslessOptimize.js'
import { getPdfPageCount, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { fileName, parentDir, stripPdf } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { usePointerReorder } from '../../../core/composables/usePointerReorder.js'
import {
  buildRangeAfter,
  insertRangeAfter,
  navigatePage,
  parsePageSelection,
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
  if (items.length) void inspectUnlockFiles(candidates).then(autoUnlockReadyFiles)
}

// 检测完成后直接解锁加密文件；队列里没有加密文件时静默跳过
async function autoUnlockReadyFiles() {
  if (unlocking.value) return
  if (!unlockFiles.value.some((file) => file.encrypted === true)) return
  await batchUnlock()
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
  if (unlocking.value) return
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
const duplexSeparate = ref(false)
const extractFile = ref('')
const extractOutputDir = ref('')
const extractPageText = ref('')
const extractTotalPages = ref(0)
const extractingPages = ref(false)
const compressFiles = ref([])
const compressSummary = ref('')
const compressing = ref(false)
const compressImageReencode = ref(false)
const compressLevel = ref(1)
const splitFile = ref('')
const splitOutputDir = ref('')
const splitRanges = ref([])
const selectedSplitRangeIndex = ref(0)
const splittingMerged = ref(false)
const splitPreviewPage = ref(1)
const splitTotalPages = ref(1)
const splitRunWarnings = ref([])
const removeBlankPages = ref(false)
const splitWarnings = computed(() => [
  ...splitRangeWarnings(splitRanges.value, splitTotalPages.value),
  ...splitRunWarnings.value,
])
const splitPreviewMaxPage = computed(() => Math.max(1, splitTotalPages.value || 1))
const selectedSplitRange = computed(() => splitRanges.value[selectedSplitRangeIndex.value] || null)

// Anti-OCR
const antiOcrFiles = ref([])
const antiOcrProcessing = ref(false)
const antiCopyMethod = ref('cmap_scramble')
const antiOcrReadyCount = computed(() => antiOcrFiles.value.filter((f) => !f.hasAntiOcr).length)
const antiOcrProtectedCount = computed(() => antiOcrFiles.value.filter((f) => f.hasAntiOcr).length)

function addAntiOcrFiles(paths) {
  const existing = new Set(antiOcrFiles.value.map((f) => f.path))
  const newItems = paths
    .filter((p) => !existing.has(p))
    .map((p) => ({
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
  let nextIndex = 0
  const workerCount = Math.min(4, paths.length)
  const workers = Array.from({ length: workerCount }, async () => {
    while (nextIndex < paths.length) {
      const path = paths[nextIndex]
      nextIndex += 1
      try {
        const result = await Promise.race([
          tauriCallSafe('detect_anti_copy', { input: path }),
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
  })
  await Promise.all(workers)
}

async function selectAntiOcrFiles() {
  const selected = await open({ multiple: true, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!selected) return
  const paths = Array.isArray(selected) ? selected : [selected]
  addAntiOcrFiles(paths)
}

function clearAntiOcrFiles() {
  antiOcrFiles.value = []
}
function removeAntiOcrFile(index) {
  antiOcrFiles.value.splice(index, 1)
}

async function batchAntiOcrApply() {
  antiOcrProcessing.value = true
  const targets = antiOcrFiles.value.filter((f) => !f.hasAntiOcr)
  for (const file of targets) {
    file.statusText = '处理中'
    file.statusType = 'warning'
    const dir = parentDir(file.path)
    const stem = stripPdf(file.name)
    const output = `${dir}/${stem}_anti_copy.pdf`
    const result = await Promise.race([
      tauriCallSafe('apply_anti_copy', { input: file.path, output, method: antiCopyMethod.value }),
      new Promise((_, reject) => setTimeout(() => reject(new Error('处理超时（30秒）')), 30000)),
    ]).catch((err) => ({ ok: false, error: err?.message || '处理超时' }))
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
    const result = await Promise.race([
      tauriCallSafe('remove_anti_copy', { input: file.path, output }),
      new Promise((_, reject) => setTimeout(() => reject(new Error('处理超时（30秒）')), 30000)),
    ]).catch((err) => ({ ok: false, error: err?.message || '处理超时' }))
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
      duplexSeparate: duplexSeparate.value,
    })
    result.ok
      ? ElMessage.success('合并完成')
      : ElMessage.error(userFacingError(result.error, 'PDF 合并失败，请确认文件未损坏且未被其他程序占用'))
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
  extractOutputDir.value = parentDir(path)
  const pageCount = await getPdfPageCount(extractFile.value)
  if (pageCount.ok) {
    extractTotalPages.value = pageCount.data || 0
  } else {
    ElMessage.error(userFacingError(pageCount.error, '读取页数失败'))
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
    outputDir: extractOutputDir.value || null,
  })
  extractingPages.value = false
  if (result.ok) {
    ElMessage.success(`已导出：${result.data.output_path}`)
  } else {
    ElMessage.error(userFacingError(result.error, 'PDF 页面提取失败，请确认文件未损坏'))
  }
}

async function selectCompressFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!selected) return
  loadCompressFiles(Array.isArray(selected) ? selected : [selected])
}

// select 与 drop 共用入口：入队并立即顺序执行；执行中新文件追加到队列尾部
function loadCompressFiles(paths) {
  const busy = new Set(
    compressFiles.value.filter((f) => f.status === 'pending' || f.status === 'processing').map((f) => f.path),
  )
  const items = [...new Set(paths)]
    .filter((path) => !busy.has(path))
    .map((path) => ({
      path,
      name: fileName(path),
      status: 'pending',
      statusText: '等待中',
      statusType: 'info',
      inputSize: 0,
      outputSize: 0,
    }))
  if (!items.length) return
  compressFiles.value = [...compressFiles.value, ...items]
  compressSummary.value = ''
  void runCompressQueue()
}

function clearCompressFiles() {
  compressFiles.value = compressFiles.value.filter((f) => f.status === 'processing')
  compressSummary.value = ''
}

function removeCompressFile(index) {
  const item = compressFiles.value[index]
  if (item && item.status !== 'processing') compressFiles.value.splice(index, 1)
}

// 队列全部结束后切换级别：重新入队并重跑全部；执行中切换只影响下一批（级别在批次开始时捕获）
watch([compressLevel, compressImageReencode], () => {
  if (compressing.value) return
  const items = compressFiles.value
  if (!items.length || items.some((f) => f.status === 'pending' || f.status === 'processing')) return
  for (const item of items) {
    item.status = 'pending'
    item.statusText = '等待中'
    item.statusType = 'info'
    item.inputSize = 0
    item.outputSize = 0
  }
  compressSummary.value = ''
  void runCompressQueue()
})

async function runCompressQueue() {
  if (compressing.value) return
  compressing.value = true
  const level = compressLevel.value
  const imageReencode = compressImageReencode.value
  const processed = []
  try {
    for (;;) {
      const item = compressFiles.value.find((f) => f.status === 'pending')
      if (!item) break
      item.status = 'processing'
      item.statusText = '压缩中'
      item.statusType = 'warning'
      const result = await tauriCallSafe('compress_pdf', {
        input: item.path,
        outputDir: null,
        level: imageReencode ? level : null,
      })
      if (result.ok) {
        const data = result.data || {}
        item.status = 'done'
        item.statusText = '完成'
        item.statusType = 'success'
        item.inputSize = Number(data.input_size) || 0
        item.outputSize = Number(data.output_size) || 0
      } else {
        item.status = 'failed'
        item.statusText = userFacingError(result.error, '压缩失败')
        item.statusType = 'danger'
      }
      processed.push(item)
    }
  } finally {
    compressing.value = false
  }
  reportCompressSummary(processed)
}

function reportCompressSummary(processed) {
  if (!processed.length) return
  const text = batchSummaryText('压缩完成', processed)
  compressSummary.value = text
  const hasFailed = processed.some((item) => item.status === 'failed')
  ElNotification({ type: hasFailed ? 'warning' : 'success', title: text, duration: 6000 })
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
  splitOutputDir.value = parentDir(path)
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
  } else if (activeTab.value === 'compress') {
    loadCompressFiles(pdfPaths)
  } else {
    if (pdfPaths.length > 1) ElMessage.info('当前工具一次处理一个 PDF，已使用第一个文件')
    const path = pdfPaths[0]
    if (activeTab.value === 'extract') await loadExtractFile(path)
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
  const items = splitRanges.value
  if (index < 0 || index >= items.length) return
  const [removed] = items.splice(index, 1)
  // 删除页段后不留页面空洞：删的是首页段则并入下一段，否则并入上一个页段
  if (removed && items.length) {
    if (index === 0) {
      items[0].pageStart = Math.min(Number(items[0].pageStart || 1), Number(removed.pageStart || 1))
    } else {
      items[index - 1].pageEnd = Math.max(Number(items[index - 1].pageEnd || 0), Number(removed.pageEnd || 0))
    }
  }
  selectedSplitRangeIndex.value = Math.min(selectedSplitRangeIndex.value, Math.max(0, items.length - 1))
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
      removeBlankPages: removeBlankPages.value,
    },
  })
  if (result.ok) {
    splitTotalPages.value = result.data.totalPages || splitTotalPages.value
    splitRunWarnings.value = result.data.warnings || []
    const failed = result.data.failed?.length || 0
    const outputs = result.data.outputs?.length || 0
    const removedBlanks = (result.data.outputs || []).reduce(
      (sum, output) => sum + Number(output.removedBlankPages || 0),
      0,
    )
    const blankSuffix = removedBlanks > 0 ? `（已删除 ${removedBlanks} 个空白页）` : ''
    failed
      ? ElMessage.warning(`已拆分 ${outputs} 个，失败 ${failed} 个${blankSuffix}`)
      : ElMessage.success(`已拆分 ${outputs} 个 PDF${blankSuffix}`)
  } else {
    ElMessage.error(userFacingError(result.error, 'PDF 拆分失败，请确认文件未损坏且页码范围正确'))
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
  background: var(--docsy-canvas);
}

.pdf-drop-overlay {
  position: absolute;
  z-index: 30;
  display: grid;
  inset: 12px;
  pointer-events: none;
  border: 2px dashed var(--docsy-primary);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface) 88%, transparent);
  place-items: center;
}

.pdf-drop-message {
  padding: 12px 18px;
  color: var(--docsy-primary);
  font-size: 14px;
  font-weight: 600;
  border-radius: var(--docsy-radius);
  background: var(--docsy-primary-soft);
}

.pdf-tabs {
  height: 100%;
}

:deep(.pdf-tabs .tool-workspace) {
  padding-block: clamp(12px, 2.2dvh, 20px);
  padding-inline: 20px;
  background: var(--docsy-canvas);
}

:deep(.pdf-tabs .workspace-header),
:deep(.pdf-tabs .workspace-content),
:deep(.pdf-tabs .workspace-actions) {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: 0 6px 18px rgba(48, 41, 32, 0.035);
}

:deep(.pdf-tabs .workspace-header) {
  min-height: 0;
  padding-block: clamp(14px, 2.1dvh, 18px);
  padding-inline: 20px;
}

:deep(.pdf-tabs .workspace-toolbar) {
  min-height: clamp(46px, 5.8dvh, 52px);
  margin-block: clamp(8px, 1.5dvh, 12px);
  padding-block: clamp(7px, 1.2dvh, 9px);
  padding-inline: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface-muted) 68%, var(--docsy-surface-elevated));
}

:deep(.pdf-tabs .workspace-content) {
  margin-top: 0;
  padding: clamp(12px, 2.4dvh, 16px);
  background: color-mix(in srgb, var(--docsy-surface-elevated) 82%, var(--docsy-canvas));
}

:deep(.pdf-tabs .workspace-actions) {
  min-height: clamp(50px, 6.5dvh, 58px);
  margin-top: clamp(8px, 1.5dvh, 12px);
  padding-block: clamp(10px, 1.5dvh, 12px);
  padding-inline: 16px;
  box-shadow: none;
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

.split-options-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 14px;
  margin-bottom: 10px;
  padding: 11px 13px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}

.split-option-note {
  color: var(--docsy-text-muted);
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
  min-height: 100%;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--docsy-surface-elevated);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  box-shadow: 0 7px 20px rgba(48, 41, 32, 0.04);
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

.anti-copy-content {
  padding: 12px 0;
}
.file-path {
  font-size: 13px;
  color: var(--docsy-text-muted);
  margin-bottom: 12px;
  word-break: break-all;
}
.anti-copy-stats {
  display: flex;
  gap: 16px;
  margin: 12px 0;
  font-size: 13px;
  color: var(--docsy-text);
}
.anti-copy-message {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: var(--docsy-radius);
  font-size: 13px;
}
.anti-copy-message.info {
  background: var(--docsy-surface-muted);
  color: var(--docsy-text);
}
.anti-copy-message.success {
  background: #f0fdf4;
  color: var(--el-color-success);
}
.anti-copy-message.danger {
  background: #fef2f2;
  color: var(--el-color-danger);
}
.compress-level-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.compress-level-label {
  font-size: 13px;
  color: var(--docsy-text);
  white-space: nowrap;
}
.merge-options {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-right: auto;
}
.merge-options .el-checkbox {
  margin-right: 0;
}
.merge-option-hint {
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.4;
}
</style>

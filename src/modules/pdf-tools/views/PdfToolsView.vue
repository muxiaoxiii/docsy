<template>
  <div class="pdf-tools-view">
    <el-tabs v-model="activeTab" tab-position="left" class="pdf-tabs">
      <el-tab-pane label="解锁" name="unlock" lazy>
        <div class="tab-content">
          <h3>PDF 解锁</h3>
          <p class="hint">移除 PDF 文件的密码保护</p>
          <el-button type="primary" @click="selectUnlockFiles">选择 PDF 文件</el-button>
          <div class="file-list" v-if="unlockFiles.length">
            <div v-for="(file, idx) in unlockFiles" :key="idx" class="file-item">
              <span class="file-name">{{ file.name }}</span>
              <el-tag :type="file.statusType" size="small">{{ file.statusText }}</el-tag>
            </div>
            <el-button type="success" @click="batchUnlock" :loading="unlocking" :disabled="!unlockFiles.length">
              批量解锁
            </el-button>
          </div>
        </div>
      </el-tab-pane>

      <el-tab-pane label="合并" name="merge" lazy>
        <div class="tab-content">
          <h3>PDF 合并</h3>
          <p class="hint">将多个 PDF 简单合并为一个文件</p>
          <el-button @click="selectMergeFiles">添加 PDF 文件</el-button>
          <div class="merge-list" v-if="mergeFiles.length">
            <div v-for="(file, idx) in mergeFiles" :key="idx" class="file-item">
              <el-icon class="drag-handle"><Rank /></el-icon>
              <span class="file-name">{{ file }}</span>
              <el-button text type="danger" size="small" @click="mergeFiles.splice(idx, 1)">删除</el-button>
            </div>
            <el-button type="success" @click="doMerge" :loading="merging" :disabled="mergeFiles.length < 2">
              合并为一个 PDF
            </el-button>
          </div>
        </div>
      </el-tab-pane>

      <el-tab-pane label="提取页面" name="extract" lazy>
        <div class="tab-content">
          <h3>快速提取页面</h3>
          <p class="hint">从一个 PDF 中挑选若干页，导出为新的 PDF。支持输入 3,7,12-15 这样的页码。</p>
          <div class="toolbar-row">
            <el-button type="primary" @click="selectExtractFile">选择 PDF</el-button>
            <el-button :disabled="!extractFile" @click="selectExtractOutputDir">输出文件夹</el-button>
          </div>
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
        </div>
      </el-tab-pane>

      <el-tab-pane label="压缩" name="compress" lazy>
        <div class="tab-content">
          <h3>PDF 压缩整理</h3>
          <p class="hint">使用 qpdf 重新压缩流、整理对象并移除未引用资源，不改变页面内容。</p>
          <div class="toolbar-row">
            <el-button type="primary" @click="selectCompressFile">选择 PDF</el-button>
            <el-button :disabled="!compressFile" @click="selectCompressOutputDir">输出文件夹</el-button>
          </div>
          <div v-if="compressFile" class="path-line">{{ compressFile }}</div>
          <div v-if="compressOutputDir" class="path-line">{{ compressOutputDir }}</div>
          <el-button type="success" :loading="compressing" :disabled="!compressFile" @click="doCompressPdf">
            压缩 PDF
          </el-button>
        </div>
      </el-tab-pane>

      <el-tab-pane label="拆分" name="split" lazy>
        <div class="tab-content split-workspace">
          <h3>PDF 拆分</h3>
          <p class="hint">按页码范围拆分 PDF</p>
          <div class="toolbar-row">
            <el-button type="primary" @click="selectSplitFile">选择 PDF</el-button>
            <el-button :disabled="!splitFile" @click="selectSplitOutputDir">输出文件夹</el-button>
            <el-button :disabled="!splitFile" @click="addSplitRange">添加页段</el-button>
          </div>
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
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../components/PdfJsPreview.vue'
import { splitRangeWarnings } from '../composables/usePdfSplitRanges.js'
import { getPdfPageCount, tauriCallSafe } from '../../../core/tauriBridge.js'
import { fileName, stripPdf } from '../../../core/filePath.js'
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

const unlockFiles = ref([])
const unlocking = ref(false)

async function selectUnlockFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    unlockFiles.value = paths.map((p) => ({
      path: p,
      name: fileName(p),
      statusText: '等待',
      statusType: 'info',
    }))
  }
}

async function batchUnlock() {
  unlocking.value = true
  for (const file of unlockFiles.value) {
    file.statusText = '处理中'
    file.statusType = 'warning'
    const result = await tauriCallSafe('unlock_pdf', { input: file.path })
    file.statusText = result.ok ? '成功' : result.error || '失败'
    file.statusType = result.ok ? 'success' : 'danger'
  }
  unlocking.value = false
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

async function selectMergeFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    mergeFiles.value.push(...paths)
  }
}

async function doMerge() {
  merging.value = true
  const output = await open({ directory: true })
  if (output) {
    const outputPath = `${output}/merged.pdf`
    const result = await tauriCallSafe('merge_pdfs', { inputs: mergeFiles.value, output: outputPath })
    result.ok ? ElMessage.success('合并完成') : ElMessage.error(result.error || '合并失败')
  }
  merging.value = false
}

async function selectExtractFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!selected) return
  extractFile.value = normalizeSelectedPath(selected)
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
    ElMessage.error(result.error || '页面提取失败')
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
    ElMessage.error(result.error || 'PDF 压缩失败')
  }
}

async function selectSplitFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    splitFile.value = selected
    splitPreviewPage.value = 1
    splitTotalPages.value = 1
    splitRunWarnings.value = []
    const pageCount = await getPdfPageCount(selected)
    if (pageCount.ok) {
      splitTotalPages.value = pageCount.data || 1
    }
    splitRanges.value = [
      {
        name: stripPdf(fileName(selected)),
        pageStart: 1,
        pageEnd: splitTotalPages.value,
      },
    ]
    selectedSplitRangeIndex.value = 0
  }
}

async function selectSplitOutputDir() {
  const selected = await open({ directory: true })
  if (selected) splitOutputDir.value = selected
}

function normalizeSelectedPath(value) {
  return Array.isArray(value) ? value[0] : value
}

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
    ElMessage.error(result.error || '拆分失败')
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
  height: 100%;
}

.pdf-tabs {
  height: 100%;
}

.tab-content {
  padding: 16px;
  max-width: 680px;
}

.split-workspace {
  max-width: none;
  height: calc(100vh - 120px);
  overflow: auto;
}

h3 {
  margin: 0 0 6px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 13px;
  margin: 0 0 16px;
}

.file-list,
.merge-list,
.range-table {
  margin-top: 16px;
}

.file-list,
.merge-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toolbar-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 10px;
}

.path-line {
  color: #606266;
  font-size: 13px;
  margin: 6px 0;
}

.path-hint {
  color: #909399;
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
  border: 1px solid #ebeef5;
  border-radius: 6px;
  background: #fafafa;
}

.split-cleanup-title {
  color: #303133;
  font-size: 13px;
  font-weight: 600;
}

.split-cleanup-zones {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  color: #606266;
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
  color: #606266;
  font-size: 13px;
}

.split-preview-status {
  margin-top: 4px;
  color: #909399;
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
  border: 1px dashed #dcdfe6;
  border-radius: 6px;
  color: #909399;
  background: #fafafa;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 4px;
}

.file-name {
  flex: 1;
  font-size: 13px;
}

.drag-handle {
  cursor: move;
  color: #c0c4cc;
}

@media (max-width: 1180px) {
  .split-main {
    grid-template-columns: 1fr;
  }

  .split-options {
    grid-template-columns: 1fr 120px;
  }
}
</style>

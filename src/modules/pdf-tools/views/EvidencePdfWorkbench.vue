<template>
  <div class="hf-workbench">
    <section class="hf-panel">
      <div class="section-head">
        <div>
          <h3>{{ workflowTitle }}</h3>
          <p class="hint">{{ workflowHint }}</p>
        </div>
        <div class="section-actions">
          <el-button v-if="workflowMode !== 'split'" type="primary" @click="selectOverlayFiles">导入单独证据 PDF</el-button>
          <el-button v-if="workflowMode !== 'merge'" :loading="importingMergedPdf" @click="importMergedPdfAsEvidence">导入已合并证据 PDF</el-button>
        </div>
      </div>

      <div v-if="importingMergedPdf" class="local-processing">
        <span class="processing-spinner" />
        <div>
          <strong>正在分析合并 PDF</strong>
          <p>大文件可能需要一段时间，当前只占用这个任务区域；其他标签和窗口仍可继续操作。</p>
        </div>
      </div>

      <div v-if="overlayFiles.length && !mergedImportPlan" class="session-summary">
        <div class="summary-item">
          <span>证据文件</span>
          <strong>{{ overlayFiles.length }}</strong>
        </div>
        <div class="summary-item">
          <span>总页数</span>
          <strong>{{ totalOverlayPages || '-' }}</strong>
        </div>
        <div class="summary-item">
          <span>页眉样例</span>
          <strong>{{ firstHeaderPreview || '不插入' }}</strong>
        </div>
        <div class="summary-item">
          <span>页码样例</span>
          <strong>{{ firstFooterPreview || '不插入' }}</strong>
        </div>
      </div>

      <div v-if="overlayFiles.length && !mergedImportPlan" class="preset-bar">
        <el-button size="small" @click="applyStandardEvidencePreset">常规证据包</el-button>
        <el-button size="small" @click="applyCleanupPreset">替换旧页眉页码</el-button>
        <el-button size="small" @click="applySplitOnlyPreset">仅输出单独证据文件</el-button>
      </div>

      <div v-if="showProcessingControls" class="rule-block">
        <div class="block-title">页面处理</div>
        <div class="rule-grid">
          <div class="rule-item">
            <label>A4 规范化</label>
            <el-switch v-model="normalizeA4" active-text="启用" inactive-text="关闭" />
            <span class="field-hint">小页补白到 A4，大页等比缩小</span>
          </div>
          <div class="rule-item">
            <label>A4 方向</label>
            <el-select v-model="a4Orientation" :disabled="!normalizeA4">
              <el-option label="自动" value="auto" />
              <el-option label="纵向" value="portrait" />
              <el-option label="横向" value="landscape" />
            </el-select>
          </div>
          <div class="rule-item">
            <label>删除批注对象</label>
            <el-switch v-model="removeAnnotations" active-text="启用" inactive-text="关闭" />
          </div>
        </div>
      </div>

      <div v-if="showProcessingControls" class="rule-block">
        <div class="block-title">删除已有页眉页脚</div>
        <div class="rule-grid">
          <div class="rule-item">
            <label>清除页眉区域</label>
            <el-switch v-model="cleanupHeaderEnabled" active-text="启用" inactive-text="关闭" />
          </div>
          <div class="rule-item">
            <label>页眉清除高度 mm</label>
            <el-input-number v-model="cleanupHeaderHeightMm" :min="4" :max="60" :step="1" :disabled="!cleanupHeaderEnabled" />
          </div>
          <div class="rule-item">
            <label>清除页脚区域</label>
            <el-switch v-model="cleanupFooterEnabled" active-text="启用" inactive-text="关闭" />
          </div>
          <div class="rule-item">
            <label>页脚清除高度 mm</label>
            <el-input-number v-model="cleanupFooterHeightMm" :min="4" :max="60" :step="1" :disabled="!cleanupFooterEnabled" />
          </div>
        </div>
      </div>

      <div v-if="showProcessingControls" class="rule-block">
        <div class="block-title">插入新页眉页脚</div>
        <div class="rule-grid">
          <div class="rule-item">
            <label>页眉来源</label>
            <el-select v-model="headerMode">
              <el-option label="不插入页眉" value="none" />
              <el-option label="文件名" value="filename" />
              <el-option label="按证据列表名称" value="per_file" />
              <el-option label="自定义文本" value="custom" />
              <el-option label="序号（证据1）" value="seq" />
              <el-option label="中文序号（证据一）" value="seq_cn" />
              <el-option label="固定前缀+序号" value="prefix_seq" />
            </el-select>
          </div>
          <div class="rule-item" v-if="headerMode === 'custom' || headerMode === 'prefix_seq'">
            <label>{{ headerMode === 'custom' ? '页眉文本' : '固定前缀' }}</label>
            <el-input v-model="headerText" placeholder="输入页眉文本" />
          </div>
          <div class="rule-item">
            <label>页眉位置</label>
            <el-select v-model="headerAlign" :disabled="headerMode === 'none'">
              <el-option label="居中" value="center" />
              <el-option label="左侧" value="left" />
              <el-option label="右侧" value="right" />
            </el-select>
          </div>
          <div class="rule-item">
            <label>页眉字号</label>
            <el-input-number v-model="headerFontSize" :min="6" :max="24" :step="1" :disabled="headerMode === 'none'" />
          </div>
          <div class="rule-item">
            <label>页眉距顶 mm</label>
            <el-input-number v-model="headerMarginMm" :min="3" :max="60" :step="1" :disabled="headerMode === 'none'" />
          </div>
          <div class="rule-item">
            <label>页脚页码</label>
            <el-switch v-model="footerEnabled" active-text="启用" inactive-text="关闭" />
          </div>
          <div class="rule-item">
            <label>页脚格式</label>
            <el-input v-model="footerText" :disabled="!footerEnabled" />
          </div>
          <div class="rule-item">
            <label>页脚位置</label>
            <el-select v-model="footerAlign" :disabled="!footerEnabled">
              <el-option label="居中" value="center" />
              <el-option label="左侧" value="left" />
              <el-option label="右侧" value="right" />
            </el-select>
          </div>
          <div class="rule-item">
            <label>页脚字号</label>
            <el-input-number v-model="footerFontSize" :min="6" :max="24" :step="1" :disabled="!footerEnabled" />
          </div>
          <div class="rule-item">
            <label>页脚距底 mm</label>
            <el-input-number v-model="footerMarginMm" :min="3" :max="60" :step="1" :disabled="!footerEnabled" />
          </div>
        </div>
      </div>

      <div v-if="showProcessingControls" class="rule-block">
        <div class="block-title">输出</div>
        <div class="rule-grid">
          <div class="rule-item">
            <label>输出模式</label>
            <el-select v-model="outputMode">
              <el-option label="单文件并合并" value="files_and_merge" />
              <el-option label="仅输出单独证据文件" value="files_only" />
              <el-option label="只生成合并 PDF" value="merge_only" />
            </el-select>
          </div>
          <div class="rule-item">
            <label>合并文件名</label>
            <el-input v-model="mergeFileName" :disabled="outputMode === 'files_only'" />
          </div>
        </div>
      </div>

      <div v-if="showProcessingControls" class="toolbar">
        <el-button :disabled="!overlayFiles.length" @click="selectOverlayOutputDir">输出文件夹</el-button>
        <el-button :disabled="!overlayFiles.length" @click="openPlannedOutputDir">打开输出文件夹</el-button>
        <el-button :disabled="!overlayFiles.length" @click="refreshOverlayPageCounts" :loading="checkingOverlayPages">刷新页数</el-button>
        <el-button :disabled="!overlayFiles.length" @click="detectAllHeaderFooter" :loading="detectingAllHeaderFooter">批量检测</el-button>
        <el-button type="success" @click="applyHeaderFooter" :loading="overlaying" :disabled="!canApplyOverlay">
          {{ processButtonText }}
        </el-button>
      </div>
      <div v-if="showProcessingControls && overlayOutputDir" class="path-text">{{ overlayOutputDir }}</div>
      <div v-if="showProcessingControls && overlayFiles.length" class="output-plan">
        <span>单文件输出：{{ plannedOutputDir }}</span>
        <span v-if="outputMode !== 'files_only'">合并输出：{{ plannedMergeOutputPath }}</span>
        <span v-if="outputMode === 'merge_only'">合并完成后会清理中间单文件副本</span>
      </div>
      <el-alert
        v-if="showProcessingControls && processingNotes.length"
        type="warning"
        :closable="false"
        show-icon
        class="processing-notes"
      >
        <template #title>{{ processingNotes.join('；') }}</template>
      </el-alert>

      <div v-if="mergedImportPlan" class="merged-import-plan">
        <div class="plan-head">
          <div>
            <div class="block-title">证据拆分确认</div>
            <p class="hint">核对页段后拆成证据列表</p>
          </div>
          <div class="plan-actions">
            <el-button size="small" @click="addMergedImportRange">添加页段</el-button>
            <el-button size="small" @click="selectMergedImportOutputDir">输出目录</el-button>
            <el-button size="small" @click="cancelMergedImportPlan">取消</el-button>
            <el-button size="small" type="primary" :loading="splittingMergedImport" @click="executeMergedImportPlan">
              确认拆分
            </el-button>
          </div>
        </div>
        <div class="import-plan-meta">
          <span>总页数：{{ mergedImportPlan.totalPages || '-' }}</span>
          <span>输出目录：{{ mergedImportPlan.outputDir }}</span>
        </div>
        <el-alert
          v-if="mergedImportWarnings.length"
          type="warning"
          :closable="false"
          show-icon
          class="import-plan-warning"
        >
          <template #title>{{ mergedImportWarnings.join('；') }}</template>
        </el-alert>
        <el-table
          :data="mergedImportPlan.items"
          size="small"
          border
          highlight-current-row
          @row-click="selectMergedImportRange"
        >
          <el-table-column type="index" label="#" width="44" />
          <el-table-column label="文件名" min-width="160">
            <template #default="{ row }">
              <el-input v-model="row.name" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="起始页" width="108">
            <template #default="{ row }">
              <el-input-number v-model="row.pageStart" :min="1" :max="mergedImportPlan.totalPages || 999999" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="结束页" width="108">
            <template #default="{ row }">
              <el-input-number v-model="row.pageEnd" :min="1" :max="mergedImportPlan.totalPages || 999999" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="页数" width="64">
            <template #default="{ row }">{{ mergedImportRangePageCount(row) || '-' }}</template>
          </el-table-column>
          <el-table-column label="识别来源" width="96">
            <template #default="{ row }">
              <el-tag :type="mergedImportSourceType(row)" size="small">
                {{ mergedImportSourceText(row) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="92">
            <template #default="{ row }">
              <el-tag :type="mergedImportRangeStatus(row).type" size="small">
                {{ mergedImportRangeStatus(row).text }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="148">
            <template #default="{ row, $index }">
              <el-button link type="primary" size="small" @click.stop="selectMergedImportRange(row)">跳转</el-button>
              <el-button link type="primary" size="small" @click.stop="insertMergedImportRangeAfter($index)">续段</el-button>
              <el-button link type="danger" size="small" @click.stop="removeMergedImportRange($index)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <div v-if="detectionPlanRows.length" class="detection-plan">
        <div class="plan-head">
          <div>
            <div class="block-title">检测确认</div>
            <p class="hint">确认旧页眉页脚识别结果后，再批量回填证据名称和清除区域</p>
          </div>
          <div class="plan-actions">
            <el-button size="small" @click="acceptAllDetectedHeaders">采用全部页眉</el-button>
            <el-button size="small" @click="acceptAllDetectedCleanup">采用全部清除区</el-button>
            <el-button size="small" type="primary" @click="acceptAllDetectionPlan">全部采用</el-button>
          </div>
        </div>
        <el-table :data="detectionPlanRows" size="small" border>
          <el-table-column prop="fileName" label="文件" min-width="150" show-overflow-tooltip />
          <el-table-column label="推荐页眉" min-width="150" show-overflow-tooltip>
            <template #default="{ row }">{{ row.headerText || '-' }}</template>
          </el-table-column>
          <el-table-column label="页脚格式" width="110">
            <template #default="{ row }">{{ row.footerText || '-' }}</template>
          </el-table-column>
          <el-table-column label="清除区" width="118">
            <template #default="{ row }">
              {{ row.headerCleanupMm ? `页眉 ${row.headerCleanupMm}mm` : '' }}
              {{ row.footerCleanupMm ? `页脚 ${row.footerCleanupMm}mm` : '' }}
            </template>
          </el-table-column>
          <el-table-column label="风险" width="78">
            <template #default="{ row }">
              <el-tag :type="row.riskType" size="small">{{ row.riskText }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="92">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click="acceptDetectionPlanRow(row)">采用</el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <el-table
        v-if="overlayFiles.length"
        :data="overlayRows"
        size="small"
        border
        class="overlay-table"
        highlight-current-row
        @row-click="selectPreviewRow"
      >
        <el-table-column type="index" label="#" width="44" />
        <el-table-column prop="name" label="文件" min-width="180" show-overflow-tooltip />
        <el-table-column label="页眉" min-width="170">
          <template #default="{ row, $index }">
            <el-input v-if="headerMode === 'per_file'" v-model="row.header" size="small" @click.stop />
            <span v-else class="table-text">{{ rowHeaderPreview(row, $index) || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="页数" width="70">
          <template #default="{ row }">{{ row.pages || '-' }}</template>
        </el-table-column>
        <el-table-column label="页码范围" width="105">
          <template #default="{ row }">{{ pageRangeText(row) }}</template>
        </el-table-column>
        <el-table-column label="来源页段" width="105">
          <template #default="{ row }">{{ sourceRangeText(row) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="92">
          <template #default="{ row }">
            <el-tag :type="row.statusType" size="small">{{ row.statusText }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ $index }">
            <el-button link size="small" :disabled="$index === 0" @click.stop="moveOverlayFile($index, -1)">上移</el-button>
            <el-button link size="small" :disabled="$index === overlayRows.length - 1" @click.stop="moveOverlayFile($index, 1)">下移</el-button>
            <el-button link type="danger" size="small" @click.stop="removeOverlayFile($index)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <section class="preview-panel">
      <div class="preview-head">
        <div>
          <h3>位置预览</h3>
          <p class="hint">{{ previewHint }}</p>
        </div>
        <div class="preview-controls">
          <template v-if="mergedImportPlan">
            <el-button size="small" :disabled="previewPage <= 1" @click="movePreviewPage(-1)">上一页</el-button>
            <el-button size="small" :disabled="previewPage >= previewMaxPage" @click="movePreviewPage(1)">下一页</el-button>
            <el-button size="small" :disabled="!selectedMergedImportRange" @click="setSelectedMergedRangeStart">设为起始页</el-button>
            <el-button size="small" :disabled="!selectedMergedImportRange" @click="setSelectedMergedRangeEnd">设为结束页</el-button>
          </template>
          <el-input-number v-model="previewPage" :min="1" :max="previewMaxPage" :disabled="!activePreviewFilePath" size="small" />
          <el-button size="small" :disabled="!activePreviewFilePath" @click="refreshPreview">刷新底图</el-button>
          <el-button size="small" :loading="detectingHeaderFooter" :disabled="!selectedOverlayFile || Boolean(mergedImportPlan)" @click="detectHeaderFooter">
            检测
          </el-button>
          <el-button size="small" type="primary" :loading="truePreviewLoading" :disabled="!selectedOverlayFile || Boolean(mergedImportPlan)" @click="renderTruePreview">
            真实预览
          </el-button>
        </div>
      </div>
      <div v-if="detectionSummary" class="detection-summary">{{ detectionSummary }}</div>
      <el-table
        v-if="detectionCandidates.length"
        :data="detectionCandidates"
        size="small"
        border
        class="detection-table"
      >
        <el-table-column label="区域" width="66">
          <template #default="{ row }">{{ row.region === 'header' ? '页眉' : '页脚' }}</template>
        </el-table-column>
        <el-table-column prop="text" label="候选文本" min-width="150" show-overflow-tooltip />
        <el-table-column label="页段" width="86">
          <template #default="{ row }">{{ row.pageRange?.start }}-{{ row.pageRange?.end }}</template>
        </el-table-column>
        <el-table-column prop="count" label="次数" width="58" />
        <el-table-column label="可信度" width="76">
          <template #default="{ row }">{{ Math.round((row.confidence || 0) * 100) }}%</template>
        </el-table-column>
        <el-table-column label="操作" width="118">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="applyDetectionCandidate(row)">回填</el-button>
            <el-button size="small" link type="primary" @click="applyCleanupFromCandidate(row)">清除区</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div v-if="truePreview" class="true-preview-stage">
        <div class="true-preview-page" :style="truePreviewFrameStyle">
          <img :src="truePreview.imageDataUrl" alt="PDF 真实预览" />
        </div>
      </div>
      <PdfJsPreview
        v-else
        :file-path="activePreviewFilePath"
        :page="previewPage"
        :reload-key="previewReloadKey"
        @loaded="handlePreviewLoaded"
        @error="handlePreviewError"
      >
        <template #default>
          <div v-if="showRulePreviewOverlays && cleanupHeaderEnabled" class="cleanup-zone cleanup-top" :style="cleanupHeaderStyle" />
          <div v-if="showRulePreviewOverlays && cleanupFooterEnabled" class="cleanup-zone cleanup-bottom" :style="cleanupFooterStyle" />
          <div v-if="showRulePreviewOverlays && previewHeaderText" class="preview-text preview-header-text" :style="previewHeaderStyle">
            {{ previewHeaderText }}
          </div>
          <div v-if="showRulePreviewOverlays && previewFooterText" class="preview-text preview-footer-text" :style="previewFooterStyle">
            {{ previewFooterText }}
          </div>
          <div
            v-for="candidate in showRulePreviewOverlays ? visibleDetectionCandidates : []"
            :key="`${candidate.region}-${candidate.normalizedText}-${candidate.bbox.page}`"
            class="detection-box"
            :class="candidate.region === 'header' ? 'detection-header' : 'detection-footer'"
            :style="detectionBoxStyle(candidate)"
          />
        </template>
      </PdfJsPreview>
    </section>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../components/PdfJsPreview.vue'
import {
  assignPageRanges,
  buildEvidencePdfRulePayload,
  buildHeaderFooterItems,
  buildHeaderText as buildSessionHeaderText,
  buildMergeOutputPath,
  buildOutputDir,
  createEvidenceFile,
  expandPlaceholders,
  fileName,
  pageRangeText,
  parentDir,
  stripPdf,
  totalPages,
} from '../composables/useEvidencePdfSession.js'
import {
  bboxOverlayStyle,
  cleanupZoneStyle,
  ptToMm,
  textOverlayStyle,
} from '../composables/pdfPreviewCoordinates.js'
import { splitRangeWarnings } from '../composables/usePdfSplitRanges.js'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

const props = defineProps({
  workflow: {
    type: String,
    default: 'all',
  },
})

const workflowMode = computed(() => (['merge', 'split'].includes(props.workflow) ? props.workflow : 'all'))
const workflowTitle = computed(() => {
  if (workflowMode.value === 'merge') return '证据合并'
  if (workflowMode.value === 'split') return '证据拆分'
  return '证据处理'
})
const workflowHint = computed(() => {
  if (workflowMode.value === 'merge') return '处理单独证据 PDF 的页眉、连续页码、A4、批注，并按需合并输出'
  if (workflowMode.value === 'split') return '读取已合并证据 PDF 的页眉页码，核对页段后拆回证据列表'
  return '按法律证据包流程处理页眉、页码、A4、批注、合并与反向拆分'
})

const overlayFiles = ref([])
const overlayOutputDir = ref('')
const checkingOverlayPages = ref(false)
const overlaying = ref(false)
const importingMergedPdf = ref(false)
const splittingMergedImport = ref(false)
const selectedOverlayIndex = ref(0)
const selectedMergedImportIndex = ref(0)
const mergedImportPlan = ref(null)

const normalizeA4 = ref(false)
const a4Orientation = ref('auto')
const rasterDpi = ref(200)
const removeAnnotations = ref(false)
const annotationKinds = ref([
  'Text',
  'FreeText',
  'Highlight',
  'Underline',
  'StrikeOut',
  'Squiggly',
  'Ink',
  'Stamp',
  'Square',
  'Circle',
  'Line',
  'Polygon',
  'PolyLine',
])
const cleanupHeaderEnabled = ref(false)
const cleanupFooterEnabled = ref(false)
const cleanupHeaderHeightMm = ref(18)
const cleanupFooterHeightMm = ref(18)
const headerMode = ref('filename')
const headerText = ref('')
const headerAlign = ref('center')
const headerFontSize = ref(10)
const headerMarginMm = ref(10)
const footerEnabled = ref(true)
const footerText = ref('{page}/{total}')
const footerAlign = ref('center')
const footerFontSize = ref(9)
const footerMarginMm = ref(10)
const outputMode = ref('files_and_merge')
const mergeFileName = ref('merged_evidence.pdf')
const previewPage = ref(1)
const previewReloadKey = ref(0)
const previewData = ref({})
const truePreview = ref(null)
const truePreviewLoading = ref(false)
let truePreviewRefreshTimer = null
const detectingHeaderFooter = ref(false)
const detectingAllHeaderFooter = ref(false)
const detectionSummary = ref('')
const detectionCandidates = ref([])
const MERGED_IMPORT_AUTO_SCAN_PAGES = 300

applyWorkflowDefaults()

const overlayRows = computed(() => {
  return assignPageRanges(overlayFiles.value)
})

const selectedOverlayFile = computed(() => overlayRows.value[selectedOverlayIndex.value] || null)
const activePreviewFilePath = computed(() => mergedImportPlan.value?.inputPath || selectedOverlayFile.value?.path || '')
const previewMaxPage = computed(() => {
  if (mergedImportPlan.value) return Math.max(1, Number(mergedImportPlan.value.totalPages || 1))
  return selectedOverlayFile.value?.pages || 1
})
const previewHint = computed(() => mergedImportPlan.value
  ? '合并 PDF 原文预览'
  : '当前预览基于实际 PDF 页面渲染，清除区域和新文字会叠加显示'
)
const showRulePreviewOverlays = computed(() => !mergedImportPlan.value)
const totalOverlayPages = computed(() => totalPages(overlayFiles.value))
const plannedOutputDir = computed(() => buildOutputDir(overlayRows.value, overlayOutputDir.value))
const plannedMergeOutputPath = computed(() => buildMergeOutputPath(overlayRows.value, overlayOutputDir.value, mergeFileName.value))
const firstHeaderPreview = computed(() => {
  const first = overlayRows.value[0]
  return first ? rowHeaderPreview(first, 0) : ''
})
const firstFooterPreview = computed(() => {
  const first = overlayRows.value[0]
  if (!first || !footerEnabled.value || !footerText.value || !totalOverlayPages.value) return ''
  return expandPlaceholders(footerText.value, first.pageStart || 1, totalOverlayPages.value)
})
const processingNotes = computed(() => {
  const notes = []
  if (cleanupHeaderEnabled.value || cleanupFooterEnabled.value) {
    notes.push('清除页眉页脚时会先尝试标准语义删除，无法语义删除的内容将用区域覆盖兜底')
  }
  if (normalizeA4.value) {
    notes.push('A4 规范化会把小页面居中补白到 A4，超过 A4 的页面才等比缩小；会尽量保留原 PDF 内容层')
  }
  if (removeAnnotations.value) {
    notes.push('删除批注只处理评论、高亮等批注对象，已扁平化到正文的标记不会被对象删除')
  }
  if (outputMode.value === 'merge_only') {
    notes.push('只生成合并 PDF 时，中间单文件副本会在合并成功后清理')
  }
  return notes
})
const showProcessingControls = computed(() =>
  !mergedImportPlan.value &&
  (workflowMode.value !== 'split' || overlayFiles.value.length > 0)
)
const processButtonText = computed(() =>
  workflowMode.value === 'split' ? '执行拆分后处理' : '执行合并处理'
)
const canApplyOverlay = computed(() =>
  overlayFiles.value.length > 0 &&
  totalOverlayPages.value > 0 &&
  (normalizeA4.value || removeAnnotations.value || cleanupHeaderEnabled.value || cleanupFooterEnabled.value || headerMode.value !== 'none' || footerEnabled.value)
)

const currentRules = computed(() => ({
  normalizeA4: normalizeA4.value,
  a4Orientation: a4Orientation.value,
  rasterDpi: rasterDpi.value,
  removeAnnotations: removeAnnotations.value,
  annotationKinds: annotationKinds.value,
  cleanupHeaderEnabled: cleanupHeaderEnabled.value,
  cleanupFooterEnabled: cleanupFooterEnabled.value,
  cleanupHeaderHeightMm: cleanupHeaderHeightMm.value,
  cleanupFooterHeightMm: cleanupFooterHeightMm.value,
  headerMode: headerMode.value,
  headerText: headerText.value,
  headerAlign: headerAlign.value,
  headerFontSize: headerFontSize.value,
  headerMarginMm: headerMarginMm.value,
  footerEnabled: footerEnabled.value,
  footerText: footerText.value,
  footerAlign: footerAlign.value,
  footerFontSize: footerFontSize.value,
  footerMarginMm: footerMarginMm.value,
  outputMode: outputMode.value,
  mergeAfterProcessing: outputMode.value !== 'files_only',
  mergeFileName: mergeFileName.value,
}))

const previewHeaderText = computed(() => {
  if (!selectedOverlayFile.value || headerMode.value === 'none') return ''
  return buildSessionHeaderText(selectedOverlayFile.value, selectedOverlayIndex.value, currentRules.value)
})

const previewFooterText = computed(() => {
  if (!selectedOverlayFile.value || !footerEnabled.value || !footerText.value) return ''
  const page = selectedOverlayFile.value.pageStart + previewPage.value - 1
  return expandPlaceholders(footerText.value, page, totalOverlayPages.value)
})

const cleanupHeaderStyle = computed(() => ({
  ...cleanupZoneStyle(cleanupHeaderHeightMm.value, previewData.value),
}))

const cleanupFooterStyle = computed(() => ({
  ...cleanupZoneStyle(cleanupFooterHeightMm.value, previewData.value),
}))

const previewHeaderStyle = computed(() => textOverlayStyle('header', previewData.value, {
  align: headerAlign.value,
  marginMm: headerMarginMm.value,
  fontSize: headerFontSize.value,
}))
const previewFooterStyle = computed(() => textOverlayStyle('footer', previewData.value, {
  align: footerAlign.value,
  marginMm: footerMarginMm.value,
  fontSize: footerFontSize.value,
}))
const truePreviewFrameStyle = computed(() => ({
  aspectRatio: truePreview.value?.widthPx && truePreview.value?.heightPx
    ? `${truePreview.value.widthPx} / ${truePreview.value.heightPx}`
    : `${truePreview.value?.widthPt || 595.28} / ${truePreview.value?.heightPt || 841.89}`,
}))
const visibleDetectionCandidates = computed(() => detectionCandidates.value.filter((candidate) =>
  candidate.bbox?.page === previewPage.value
))
const detectionPlanRows = computed(() => overlayRows.value
  .map((file, index) => buildDetectionPlanRow(file, index))
  .filter((row) => row.headerCandidate || row.footerCandidate)
)
const mergedImportWarnings = computed(() => {
  if (!mergedImportPlan.value) return []
  return [
    ...(mergedImportPlan.value.warnings || []),
    ...splitRangeWarnings(mergedImportPlan.value.items || [], mergedImportPlan.value.totalPages),
  ]
})
const selectedMergedImportRange = computed(() =>
  mergedImportPlan.value?.items?.[selectedMergedImportIndex.value] || null
)

watch(selectedOverlayFile, () => {
  previewPage.value = 1
  truePreview.value = null
  clearTruePreviewRefresh()
  detectionSummary.value = selectedOverlayFile.value?.detectionSummary || ''
  detectionCandidates.value = selectedOverlayFile.value?.detectionCandidates || []
})

watch([previewPage, currentRules], () => {
  if (truePreview.value) {
    scheduleTruePreviewRefresh()
  }
}, {
  deep: true,
})

onBeforeUnmount(() => {
  clearTruePreviewRefresh()
})

function applyWorkflowDefaults() {
  if (workflowMode.value === 'split') {
    headerMode.value = 'none'
    footerEnabled.value = false
    outputMode.value = 'files_only'
    mergeFileName.value = 'split_evidence.pdf'
    return
  }
  headerMode.value = 'filename'
  footerEnabled.value = true
  outputMode.value = 'files_and_merge'
  mergeFileName.value = 'merged_evidence.pdf'
}

async function selectOverlayFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!selected) return
  const paths = Array.isArray(selected) ? selected : [selected]
  mergedImportPlan.value = null
  overlayFiles.value = paths.map(createEvidenceFile)
  selectedOverlayIndex.value = 0
  await refreshOverlayPageCounts()
}

async function importMergedPdfAsEvidence() {
  if (importingMergedPdf.value) return
  const input = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!input) return
  const outputDir = defaultMergedImportOutputDir(input)

  importingMergedPdf.value = true
  let knownTotalPages = 1
  try {
    const countResult = await tauriCallSafe('get_pdf_page_count', { input })
    const totalPages = countResult.ok ? Number(countResult.data || 0) : 0
    knownTotalPages = Math.max(1, totalPages || 1)
    if (totalPages > MERGED_IMPORT_AUTO_SCAN_PAGES) {
      ElMessage.warning(`该 PDF 共 ${totalPages} 页，为避免卡顿，先自动识别前 ${MERGED_IMPORT_AUTO_SCAN_PAGES} 页；后续页段可手动补充。`)
    }
    const headerScanMm = mergedImportScanZoneMm(cleanupHeaderHeightMm.value)
    const footerScanMm = mergedImportScanZoneMm(cleanupFooterHeightMm.value)
    const inspect = await tauriCallSafe('inspect_merged_evidence_pdf', {
      args: {
        inputPath: input,
        maxPages: MERGED_IMPORT_AUTO_SCAN_PAGES,
        headerZoneMm: headerScanMm,
        footerZoneMm: footerScanMm,
      },
    })
    const detectedItems = inspect.ok ? (inspect.data.items || []) : []
    const detectedTotalPages = inspect.ok ? Number(inspect.data.totalPages || 0) : 0
    const warnings = inspect.ok
      ? [...(inspect.data.warnings || [])]
      : [inspect.error || '合并 PDF 页眉分析失败，已保留手动拆分页段']
    const planTotalPages = Math.max(1, detectedTotalPages || totalPages || 1)
    const items = detectedItems
      .filter((item) => Number(item.pageStart) > 0 && Number(item.pageEnd) >= Number(item.pageStart))
      .map((item, index) => ({
        name: String(item.name || '').trim() || defaultMergedImportName(input, index),
        pageStart: Number(item.pageStart),
        pageEnd: Number(item.pageEnd),
        source: item.source || 'unknown',
      }))
    if (!items.length) {
      items.push(defaultMergedImportRange(input, planTotalPages))
      warnings.push('未识别到可用页眉页段，已生成一个覆盖全文的手动页段')
    }

    mergedImportPlan.value = {
      inputPath: input,
      outputDir,
      totalPages: planTotalPages,
      warnings,
      items,
    }
    selectedMergedImportIndex.value = 0
    previewPage.value = items[0]?.pageStart || 1
    truePreview.value = null
    safeRefreshPreview()
    if (!inspect.ok) {
      ElMessage.warning('自动分析失败，已进入手动拆分页段确认')
    } else if (items.some(item => item.source === 'manual')) {
      ElMessage.warning('未识别到页眉页段，请手动调整拆分范围')
    } else {
      ElMessage.success(`已识别 ${items.length} 个页段，请核对后确认拆分`)
    }
  } catch (err) {
    mergedImportPlan.value = buildManualMergedImportPlan(input, outputDir, knownTotalPages, [
      `分析流程中断：${String(err?.message || err || '未知错误')}`,
      '已生成一个覆盖全文的手动页段',
    ])
    selectedMergedImportIndex.value = 0
    previewPage.value = 1
    truePreview.value = null
    safeRefreshPreview()
    ElMessage.warning('分析中断，已进入手动拆分页段确认')
  } finally {
    importingMergedPdf.value = false
  }
}

async function executeMergedImportPlan() {
  if (!mergedImportPlan.value || splittingMergedImport.value) return
  const items = normalizedMergedImportItems()
  if (!items.length) {
    ElMessage.warning('没有可拆分的页段')
    return
  }
  const invalid = items.find((item) => !item.name || item.pageStart < 1 || item.pageEnd < item.pageStart)
  if (invalid) {
    ElMessage.warning('请先修正文件名或页码范围')
    return
  }
  const blockingWarnings = splitRangeWarnings(items, mergedImportPlan.value.totalPages)
  if (blockingWarnings.length) {
    ElMessage.warning(`请先核对页段：${blockingWarnings[0]}`)
    return
  }
  if (overlayFiles.value.length) {
    try {
      await ElMessageBox.confirm('确认拆分后会替换当前证据列表。', '替换当前列表', {
        confirmButtonText: '替换并拆分',
        cancelButtonText: '取消',
        type: 'warning',
      })
    } catch {
      return
    }
  }

  splittingMergedImport.value = true
  try {
    const split = await tauriCallSafe('split_merged_evidence_pdf', {
      args: {
        inputPath: mergedImportPlan.value.inputPath,
        outputDir: mergedImportPlan.value.outputDir,
        items,
        cleanup: {
          headerEnabled: false,
          footerEnabled: false,
          headerHeightMm: cleanupHeaderHeightMm.value,
          footerHeightMm: cleanupFooterHeightMm.value,
        },
      },
    })
    if (!split.ok) {
      ElMessage.error(split.error || '拆分合并 PDF 失败')
      return
    }

    const outputs = split.data.outputs || []
    if (!outputs.length) {
      ElMessage.warning('没有生成可导入的拆分文件')
      return
    }
    const itemByRange = new Map(items.map((item) => [sourceRangeKey(item), item]))
    overlayFiles.value = outputs.map((output) => {
      const sourceItem = itemByRange.get(sourceRangeKey(output)) || {}
      const pages = Math.max(0, Number(output.pageEnd || 0) - Number(output.pageStart || 0) + 1)
      const needsReview = sourceItem.source === 'fallback' || sourceItem.source === 'manual' || hasSplitWarning(split.data.warnings || [], output)
      return {
        ...createEvidenceFile(output.outputPath),
        header: output.name,
        pages,
        sourcePageStart: Number(output.pageStart || sourceItem.pageStart || 0),
        sourcePageEnd: Number(output.pageEnd || sourceItem.pageEnd || 0),
        sourceDetectionSource: sourceItem.source || 'unknown',
        detectionSummary: `来自合并 PDF 第 ${output.pageStart}-${output.pageEnd} 页`,
        statusText: needsReview ? '需核对' : '就绪',
        statusType: needsReview ? 'warning' : 'success',
      }
    })
    overlayOutputDir.value = mergedImportPlan.value.outputDir
    selectedOverlayIndex.value = 0
    applyWorkflowDefaults()
    mergedImportPlan.value = null
    refreshPreview()

    const failed = split.data.failed?.length || 0
    const warnings = split.data.warnings || []
    if (failed) {
      ElMessage.warning(`已生成 ${outputs.length} 个证据，失败 ${failed} 个`)
    } else if (warnings.length) {
      ElMessage.warning(`已生成 ${outputs.length} 个证据，需核对页段提示`)
    } else {
      ElMessage.success(`已生成 ${outputs.length} 个证据`)
    }
  } finally {
    splittingMergedImport.value = false
  }
}

async function selectMergedImportOutputDir() {
  if (!mergedImportPlan.value) return
  const selected = await open({ directory: true })
  if (!selected) return
  mergedImportPlan.value.outputDir = selected
}

function cancelMergedImportPlan() {
  mergedImportPlan.value = null
  selectedMergedImportIndex.value = 0
  previewPage.value = 1
  refreshPreview()
}

function normalizedMergedImportItems() {
  return (mergedImportPlan.value?.items || []).map((item, index) => ({
    name: String(item.name || `文件${index + 1}`).trim(),
    pageStart: Number(item.pageStart || 0),
    pageEnd: Number(item.pageEnd || 0),
    source: item.source || 'unknown',
  }))
}

function defaultMergedImportName(inputPath, index) {
  if (index === 0) {
    return stripPdf(fileName(inputPath)) || '文件1'
  }
  return `文件${index + 1}`
}

function defaultMergedImportOutputDir(inputPath) {
  const stem = stripPdf(fileName(inputPath)) || '合并PDF'
  return `${parentDir(inputPath)}/${stem}-分项`
}

function defaultMergedImportRange(inputPath, total) {
  return {
    name: defaultMergedImportName(inputPath, 0),
    pageStart: 1,
    pageEnd: Math.max(1, Number(total || 1)),
    source: 'manual',
  }
}

function buildManualMergedImportPlan(inputPath, outputDir, total, warnings = []) {
  return {
    inputPath,
    outputDir,
    totalPages: Math.max(1, Number(total || 1)),
    warnings,
    items: [defaultMergedImportRange(inputPath, total)],
  }
}

function safeRefreshPreview() {
  try {
    refreshPreview()
  } catch (err) {
    console.warn('刷新拆分预览失败', err)
  }
}

function selectMergedImportRange(row) {
  if (!row) return
  const index = mergedImportPlan.value?.items?.indexOf(row) ?? -1
  if (index >= 0) selectedMergedImportIndex.value = index
  previewPage.value = Math.min(previewMaxPage.value, Math.max(1, Number(row.pageStart || 1)))
  truePreview.value = null
  refreshPreview()
}

function movePreviewPage(delta) {
  previewPage.value = Math.min(previewMaxPage.value, Math.max(1, Number(previewPage.value || 1) + delta))
  truePreview.value = null
  refreshPreview()
}

function setSelectedMergedRangeStart() {
  const range = selectedMergedImportRange.value
  if (!range) return
  range.pageStart = Number(previewPage.value || 1)
  if (Number(range.pageEnd || 0) < range.pageStart) {
    range.pageEnd = range.pageStart
  }
}

function setSelectedMergedRangeEnd() {
  const range = selectedMergedImportRange.value
  if (!range) return
  range.pageEnd = Number(previewPage.value || 1)
  if (Number(range.pageStart || 0) > range.pageEnd) {
    range.pageStart = range.pageEnd
  }
}

function addMergedImportRange() {
  if (!mergedImportPlan.value) return
  const items = mergedImportPlan.value.items
  const previous = items[items.length - 1]
  const start = Math.min(previewMaxPage.value, Math.max(1, Number(previous?.pageEnd || 0) + 1))
  items.push({
    name: `文件${items.length + 1}`,
    pageStart: start,
    pageEnd: start,
    source: 'manual',
  })
  selectMergedImportRange(items[items.length - 1])
}

function insertMergedImportRangeAfter(index) {
  if (!mergedImportPlan.value) return
  const items = mergedImportPlan.value.items
  const previous = items[index]
  const next = items[index + 1]
  const start = Math.min(previewMaxPage.value, Math.max(1, Number(previous?.pageEnd || 0) + 1))
  const endLimit = next ? Math.max(start, Number(next.pageStart || start) - 1) : start
  const item = {
    name: `文件${items.length + 1}`,
    pageStart: start,
    pageEnd: endLimit,
    source: 'manual',
  }
  items.splice(index + 1, 0, item)
  selectMergedImportRange(item)
}

function removeMergedImportRange(index) {
  if (!mergedImportPlan.value) return
  mergedImportPlan.value.items.splice(index, 1)
  selectedMergedImportIndex.value = Math.min(
    selectedMergedImportIndex.value,
    Math.max(0, mergedImportPlan.value.items.length - 1),
  )
}

function mergedImportRangePageCount(row) {
  const pageStart = Number(row?.pageStart || 0)
  const pageEnd = Number(row?.pageEnd || 0)
  return pageStart > 0 && pageEnd >= pageStart ? pageEnd - pageStart + 1 : 0
}

function mergedImportRangeStatus(row) {
  const pageStart = Number(row?.pageStart || 0)
  const pageEnd = Number(row?.pageEnd || 0)
  const total = Number(mergedImportPlan.value?.totalPages || 0)
  if (!String(row?.name || '').trim() || !pageStart || !pageEnd || pageStart > pageEnd || pageEnd > total) {
    return { type: 'danger', text: '错误' }
  }
  if (row.source === 'fallback' || row.source === 'manual') {
    return { type: 'warning', text: '核对' }
  }
  return { type: 'success', text: '正常' }
}

function mergedImportSourceType(row) {
  return row?.source === 'fallback' || row?.source === 'manual' ? 'warning' : 'success'
}

function mergedImportSourceText(row) {
  if (row?.source === 'fallback') return '需核对'
  if (row?.source === 'manual') return '手动'
  return '页眉'
}

function mergedImportScanZoneMm(value) {
  return Math.max(25, Math.min(60, Number(value || 0) || 25))
}

function sourceRangeKey(item) {
  return `${Number(item.pageStart || 0)}-${Number(item.pageEnd || 0)}-${item.name || ''}`
}

function hasSplitWarning(warnings, output) {
  const name = String(output.name || '').trim()
  if (!name) return false
  return warnings.some((warning) => String(warning || '').includes(name))
}

async function selectOverlayOutputDir() {
  const selected = await open({ directory: true })
  if (selected) overlayOutputDir.value = selected
}

async function openPlannedOutputDir() {
  if (!overlayFiles.value.length) return
  const result = await tauriCallSafe('open_path', { path: plannedOutputDir.value })
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开输出文件夹')
  }
}

async function refreshOverlayPageCounts() {
  checkingOverlayPages.value = true
  for (const file of overlayFiles.value) {
    file.statusText = '读取页数'
    file.statusType = 'warning'
    const result = await tauriCallSafe('get_pdf_page_count', { input: file.path })
    if (result.ok) {
      file.pages = result.data
      file.statusText = '就绪'
      file.statusType = 'success'
    } else {
      file.pages = 0
      file.statusText = '页数失败'
      file.statusType = 'danger'
    }
  }
  checkingOverlayPages.value = false
}

function refreshPreview() {
  truePreview.value = null
  previewReloadKey.value += 1
}

function applyStandardEvidencePreset() {
  normalizeA4.value = false
  removeAnnotations.value = false
  cleanupHeaderEnabled.value = false
  cleanupFooterEnabled.value = false
  headerMode.value = 'filename'
  headerAlign.value = 'right'
  headerFontSize.value = 10
  headerMarginMm.value = 10
  footerEnabled.value = true
  footerText.value = '{page}/{total}'
  footerAlign.value = 'center'
  footerFontSize.value = 9
  footerMarginMm.value = 10
  outputMode.value = 'files_and_merge'
  refreshPreview()
}

function applyCleanupPreset() {
  cleanupHeaderEnabled.value = true
  cleanupFooterEnabled.value = true
  cleanupHeaderHeightMm.value = 18
  cleanupFooterHeightMm.value = 18
  headerMode.value = 'filename'
  headerAlign.value = 'right'
  footerEnabled.value = true
  footerText.value = '{page}/{total}'
  outputMode.value = 'files_and_merge'
  refreshPreview()
}

function applySplitOnlyPreset() {
  outputMode.value = 'files_only'
  headerMode.value = 'filename'
  footerEnabled.value = true
  footerText.value = '{page}/{total}'
  refreshPreview()
}

function handlePreviewLoaded(info) {
  previewData.value = info
}

function handlePreviewError(message) {
  previewData.value = {}
  ElMessage.error(message)
}

async function applyHeaderFooter() {
  if (!canApplyOverlay.value) return
  overlaying.value = true
  const payload = buildEvidencePdfRulePayload(overlayRows.value, currentRules.value, overlayOutputDir.value)
  overlayRows.value.forEach((file) => {
    file.statusText = '处理中'
    file.statusType = 'warning'
  })

  const result = await tauriCallSafe('apply_evidence_pdf_rules', { args: payload })
  if (!result.ok) {
    ElMessage.error(result.error || 'PDF 处理失败')
    overlayFiles.value.forEach((file) => {
      file.statusText = '失败'
      file.statusType = 'danger'
    })
    overlaying.value = false
    return
  }

  const successByInput = new Map((result.data.results || []).map((item) => [item.inputPath, item]))
  const failedByInput = new Map((result.data.failed || []).map((item) => [item.path, item]))
  overlayFiles.value.forEach((file) => {
    const success = successByInput.get(file.path)
    const failed = failedByInput.get(file.path)
    if (success) {
      file.outputPath = success.outputPath
      file.statusText = '完成'
      file.statusType = 'success'
    } else if (failed) {
      file.statusText = '失败'
      file.statusType = 'danger'
    }
  })

  const failedCount = result.data.failed?.length || 0
  const successCount = result.data.results?.length || 0
  const merge = result.data.merge
  if (merge?.status === 'done') {
    const cleanupText = merge.outputMode === 'merge_only' ? `，已清理 ${merge.removedIntermediates || 0} 个中间副本` : ''
    ElMessage.success(`已完成 ${successCount} 个 PDF，并已合并${cleanupText}`)
  } else if (failedCount) {
    ElMessage.warning(`已完成 ${successCount} 个，失败 ${failedCount} 个`)
  } else if (merge?.status === 'skipped') {
    ElMessage.warning(`已完成 ${successCount} 个 PDF，${merge.message || '未合并'}`)
  } else {
    ElMessage.success(`已完成 ${successCount} 个 PDF`)
  }
  overlaying.value = false
}

async function renderTruePreview() {
  if (!selectedOverlayFile.value || truePreviewLoading.value) return
  truePreviewLoading.value = true
  const items = buildHeaderFooterItems(overlayRows.value, currentRules.value, overlayOutputDir.value)
  const item = items.find((candidate) => candidate.inputPath === selectedOverlayFile.value.path)
  if (!item) {
    truePreviewLoading.value = false
    return
  }

  const result = await tauriCallSafe('preview_pdf_header_footer', {
    args: {
      job: item,
      page: previewPage.value,
      dpi: 120,
      annotationRule: {
        removeAnnotations: removeAnnotations.value,
        kinds: annotationKinds.value,
      },
    },
  })
  if (result.ok) {
    truePreview.value = result.data
    previewData.value = result.data
  } else {
    ElMessage.error(result.error || '真实预览生成失败')
  }
  truePreviewLoading.value = false
}

function scheduleTruePreviewRefresh() {
  clearTruePreviewRefresh()
  truePreviewRefreshTimer = window.setTimeout(() => {
    truePreviewRefreshTimer = null
    renderTruePreview()
  }, 600)
}

function clearTruePreviewRefresh() {
  if (truePreviewRefreshTimer) {
    window.clearTimeout(truePreviewRefreshTimer)
    truePreviewRefreshTimer = null
  }
}

async function detectHeaderFooter() {
  if (!selectedOverlayFile.value || detectingHeaderFooter.value) return
  detectingHeaderFooter.value = true
  detectionSummary.value = ''
  const result = await detectFileHeaderFooter(selectedOverlayFile.value)

  if (!result.ok) {
    ElMessage.error(result.error || '页眉页脚检测失败')
    detectingHeaderFooter.value = false
    return
  }

  applyDetectionResultToFile(selectedOverlayFile.value, result.data)
  detectingHeaderFooter.value = false
}

async function detectAllHeaderFooter() {
  if (!overlayRows.value.length || detectingAllHeaderFooter.value) return
  detectingAllHeaderFooter.value = true
  let success = 0
  let failed = 0
  for (const file of overlayRows.value) {
    file.statusText = '检测中'
    file.statusType = 'warning'
    const result = await detectFileHeaderFooter(file)
    if (result.ok) {
      applyDetectionResultToFile(file, result.data)
      file.statusText = '已检测'
      file.statusType = 'success'
      success += 1
    } else {
      file.statusText = '检测失败'
      file.statusType = 'danger'
      failed += 1
    }
  }
  if (selectedOverlayFile.value) {
    detectionSummary.value = selectedOverlayFile.value.detectionSummary || ''
    detectionCandidates.value = selectedOverlayFile.value.detectionCandidates || []
  }
  failed ? ElMessage.warning(`已检测 ${success} 个，失败 ${failed} 个`) : ElMessage.success(`已检测 ${success} 个 PDF`)
  detectingAllHeaderFooter.value = false
}

async function detectFileHeaderFooter(file) {
  return tauriCallSafe('detect_pdf_header_footer', {
    args: {
      inputPath: file.path,
      maxPages: 20,
      headerZoneMm: cleanupHeaderHeightMm.value,
      footerZoneMm: cleanupFooterHeightMm.value,
    },
  })
}

function applyDetectionResultToFile(file, data) {
  const header = data.headerCandidates?.[0]
  const footer = data.footerCandidates?.[0]
  const candidates = [
    ...(data.headerCandidates || []).slice(0, 6),
    ...(data.footerCandidates || []).slice(0, 6),
  ]
  detectionCandidates.value = candidates
  const parts = []
  if (data.artifact?.hasHeader) parts.push(`发现结构化页眉 ${data.artifact.headerCount} 处`)
  if (data.artifact?.hasFooter) parts.push(`发现结构化页脚 ${data.artifact.footerCount} 处`)
  if (header) parts.push(`页眉候选：${header.text}`)
  if (footer) parts.push(`页脚候选：${footer.text}`)
  if (candidates.length) parts.push(`候选 ${candidates.length} 个`)
  file.detectionSummary = parts.length ? parts.join('；') : '未发现稳定的文本型页眉页脚候选'
  file.detectionCandidates = candidates
  if (file.path === selectedOverlayFile.value?.path) {
    detectionSummary.value = file.detectionSummary
    detectionCandidates.value = candidates
  }
}

function applyDetectionCandidate(candidate) {
  if (!candidate) return
  if (candidate.region === 'header' && selectedOverlayFile.value) {
    headerMode.value = 'per_file'
    selectedOverlayFile.value.header = candidate.text
    ElMessage.success('已回填页眉名称')
    return
  }
  if (candidate.region === 'footer') {
    footerEnabled.value = true
    footerText.value = candidate.normalizedText || candidate.text
    ElMessage.success('已回填页脚格式')
  }
}

function buildDetectionPlanRow(file, index) {
  const candidates = file.detectionCandidates || []
  const headerCandidate = pickRecommendedHeader(candidates)
  const footerCandidate = pickRecommendedFooter(candidates)
  const headerCleanupMm = candidateCleanupHeightMm(headerCandidate)
  const footerCleanupMm = candidateCleanupHeightMm(footerCandidate)
  const confidence = Math.max(
    Number(headerCandidate?.confidence || 0),
    Number(footerCandidate?.confidence || 0),
  )
  return {
    file,
    index,
    fileName: file.name,
    headerCandidate,
    footerCandidate,
    headerText: headerCandidate?.text || '',
    footerText: footerCandidate?.normalizedText || footerCandidate?.text || '',
    headerCleanupMm,
    footerCleanupMm,
    confidence,
    headerCandidateScore: candidateScore(headerCandidate, 'header'),
    footerCandidateScore: candidateScore(footerCandidate, 'footer'),
    riskType: confidence >= 0.75 ? 'success' : confidence >= 0.5 ? 'warning' : 'danger',
    riskText: confidence >= 0.75 ? '较高' : confidence >= 0.5 ? '核对' : '低',
  }
}

function pickRecommendedHeader(candidates) {
  return candidates
    .filter((candidate) => candidate.region === 'header')
    .filter((candidate) => !candidateLabels(candidate).includes('page-number'))
    .sort((a, b) => candidateScore(b, 'header') - candidateScore(a, 'header'))
    .at(0)
}

function pickRecommendedFooter(candidates) {
  return candidates
    .filter((candidate) => candidate.region === 'footer')
    .sort((a, b) => candidateScore(b, 'footer') - candidateScore(a, 'footer'))
    .at(0)
}

function candidateScore(candidate, region) {
  if (!candidate) return 0
  const labels = candidateLabels(candidate)
  let score = Number(candidate.confidence || 0)
  if (region === 'header' && labels.includes('evidence-label')) score += 0.35
  if (region === 'header' && looksLikeEvidenceHeader(candidate.text)) score += 0.25
  if (region === 'footer' && labels.includes('page-number')) score += 0.45
  if (region === 'footer' && (candidate.normalizedText || '').includes('{page}')) score += 0.30
  return score
}

function candidateLabels(candidate) {
  return Array.isArray(candidate?.labels) ? candidate.labels : []
}

function looksLikeEvidenceHeader(text) {
  const value = String(text || '').trim()
  return /证据|附件|材料|exhibit|evidence/i.test(value)
}

function acceptAllDetectionPlan() {
  acceptAllDetectedHeaders(true)
  acceptAllDetectedCleanup(true)
  acceptBestDetectedFooter()
  ElMessage.success('已采用检测确认表')
}

function acceptAllDetectedHeaders(silent = false) {
  const rows = detectionPlanRows.value.filter((row) => row.headerText)
  if (!rows.length) return
  headerMode.value = 'per_file'
  rows.forEach((row) => {
    row.file.header = row.headerText
  })
  if (!silent) ElMessage.success(`已回填 ${rows.length} 个页眉`)
}

function acceptAllDetectedCleanup(silent = false) {
  const { headerHeights, footerHeights } = mergeCleanupHeights(detectionPlanRows.value)
  if (headerHeights.length) {
    cleanupHeaderEnabled.value = true
    cleanupHeaderHeightMm.value = Math.max(cleanupHeaderHeightMm.value, ...headerHeights)
  }
  if (footerHeights.length) {
    cleanupFooterEnabled.value = true
    cleanupFooterHeightMm.value = Math.max(cleanupFooterHeightMm.value, ...footerHeights)
  }
  if (!silent && (headerHeights.length || footerHeights.length)) {
    ElMessage.success('已设置清除区域')
  }
}

function acceptBestDetectedFooter() {
  const row = [...detectionPlanRows.value]
    .filter((item) => item.footerText)
    .sort((a, b) => b.footerCandidateScore - a.footerCandidateScore)
    .at(0)
  if (!row) return
  footerEnabled.value = true
  footerText.value = row.footerText
}

function mergeCleanupHeights(rows) {
  return {
    headerHeights: rows
      .filter((row) => row.headerCandidate)
      .map((row) => row.headerCleanupMm)
      .filter(Boolean),
    footerHeights: rows
      .filter((row) => row.footerCandidate)
      .map((row) => row.footerCleanupMm)
      .filter(Boolean),
  }
}

function acceptDetectionPlanRow(row) {
  if (row.headerText) {
    headerMode.value = 'per_file'
    row.file.header = row.headerText
  }
  if (row.footerText) {
    footerEnabled.value = true
    footerText.value = row.footerText
  }
  if (row.headerCleanupMm) {
    cleanupHeaderEnabled.value = true
    cleanupHeaderHeightMm.value = Math.max(cleanupHeaderHeightMm.value, row.headerCleanupMm)
  }
  if (row.footerCleanupMm) {
    cleanupFooterEnabled.value = true
    cleanupFooterHeightMm.value = Math.max(cleanupFooterHeightMm.value, row.footerCleanupMm)
  }
  selectedOverlayIndex.value = row.index
  ElMessage.success('已采用该文件的检测结果')
}

function applyCleanupFromCandidate(candidate) {
  const bbox = candidate?.bbox
  if (!bbox) return
  if (candidate.region === 'header') {
    cleanupHeaderEnabled.value = true
    cleanupHeaderHeightMm.value = boundedCleanupHeight(ptToMm(bbox.y1) + 2)
    ElMessage.success('已设置页眉清除区域')
    return
  }
  if (candidate.region === 'footer') {
    cleanupFooterEnabled.value = true
    cleanupFooterHeightMm.value = boundedCleanupHeight(ptToMm((bbox.height || 0) - bbox.y0) + 2)
    ElMessage.success('已设置页脚清除区域')
  }
}

function candidateCleanupHeightMm(candidate) {
  const bbox = candidate?.bbox
  if (!bbox) return 0
  if (candidate.region === 'header') {
    return boundedCleanupHeight(ptToMm(bbox.y1) + 2)
  }
  if (candidate.region === 'footer') {
    return boundedCleanupHeight(ptToMm((bbox.height || 0) - bbox.y0) + 2)
  }
  return 0
}

function boundedCleanupHeight(value) {
  return Math.min(60, Math.max(4, Math.ceil(Number(value || 0))))
}

function detectionBoxStyle(candidate) {
  return bboxOverlayStyle(candidate.bbox)
}

function selectPreviewRow(row) {
  const index = overlayRows.value.findIndex((item) => item.path === row.path)
  if (index >= 0) selectedOverlayIndex.value = index
}

function rowHeaderPreview(row, index) {
  return buildSessionHeaderText(row, index, currentRules.value)
}

function sourceRangeText(row) {
  if (!row.sourcePageStart || !row.sourcePageEnd) return '-'
  return `${row.sourcePageStart}-${row.sourcePageEnd}`
}

function moveOverlayFile(index, direction) {
  const target = index + direction
  if (target < 0 || target >= overlayFiles.value.length) return
  const items = [...overlayFiles.value]
  const [item] = items.splice(index, 1)
  items.splice(target, 0, item)
  overlayFiles.value = items
  selectedOverlayIndex.value = target
  refreshPreview()
}

function removeOverlayFile(index) {
  overlayFiles.value.splice(index, 1)
  selectedOverlayIndex.value = Math.min(selectedOverlayIndex.value, Math.max(0, overlayFiles.value.length - 1))
  refreshPreview()
}

</script>

<style scoped>
.hf-workbench {
  display: grid;
  grid-template-columns: minmax(420px, 0.95fr) minmax(340px, 1.05fr);
  gap: 16px;
  height: calc(100vh - 112px);
  padding: 16px;
  overflow: hidden;
}

.hf-panel,
.preview-panel {
  min-height: 0;
  overflow: auto;
}

.section-head,
.preview-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.section-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.local-processing {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
  padding: 12px;
  border: 1px solid #d9ecff;
  border-radius: 6px;
  background: #ecf5ff;
  color: #303133;
}

.local-processing p {
  margin: 3px 0 0;
  color: #606266;
  font-size: 12px;
}

.processing-spinner {
  width: 18px;
  height: 18px;
  border: 2px solid #a0cfff;
  border-top-color: #409eff;
  border-radius: 50%;
  animation: docsy-spin 0.8s linear infinite;
}

@keyframes docsy-spin {
  to {
    transform: rotate(360deg);
  }
}

h3 {
  margin: 0 0 6px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 13px;
  margin: 0;
}

.rule-block {
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  padding: 12px;
  margin-bottom: 12px;
  background: #fafafa;
}

.session-summary {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  margin-bottom: 10px;
}

.summary-item {
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  background: #fff;
}

.summary-item span {
  display: block;
  color: #909399;
  font-size: 12px;
  margin-bottom: 4px;
}

.summary-item strong {
  display: block;
  color: #303133;
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preset-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}

.block-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 10px;
  color: #303133;
}

.rule-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 10px;
}

.rule-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-item label {
  font-size: 12px;
  color: #606266;
}

.field-hint {
  font-size: 11px;
  line-height: 1.4;
  color: #909399;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin: 12px 0;
}

.path-text {
  color: #606266;
  font-size: 13px;
  margin-bottom: 8px;
}

.output-plan {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: #606266;
  font-size: 12px;
  line-height: 1.4;
  margin-bottom: 8px;
}

.processing-notes {
  margin: 8px 0 12px;
}

.detection-plan,
.merged-import-plan {
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  background: #fff;
  margin: 12px 0;
  padding: 10px;
}

.import-plan-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 10px;
  color: #606266;
  font-size: 12px;
}

.import-plan-warning {
  margin-bottom: 10px;
}

.plan-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.plan-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.table-text {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  vertical-align: middle;
  white-space: nowrap;
}

.detection-summary {
  margin-bottom: 10px;
  padding: 8px 10px;
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  background: #fafafa;
  color: #606266;
  font-size: 13px;
  line-height: 1.5;
}

.detection-table {
  margin-bottom: 10px;
}

.overlay-table {
  margin-top: 10px;
}

.preview-controls {
  display: flex;
  gap: 8px;
  align-items: center;
}

.preview-stage {
  display: flex;
  justify-content: center;
  padding: 12px;
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  min-height: 520px;
}

.true-preview-stage {
  display: flex;
  justify-content: center;
  padding: 12px;
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  min-height: 520px;
}

.page-preview {
  position: relative;
  width: min(100%, 620px);
  background: #fff;
  box-shadow: 0 2px 14px rgba(0, 0, 0, 0.16);
}

.true-preview-page {
  width: min(100%, 620px);
  background: #fff;
  box-shadow: 0 2px 14px rgba(0, 0, 0, 0.16);
}

.page-preview img,
.true-preview-page img {
  display: block;
  width: 100%;
  height: 100%;
}

.cleanup-zone {
  position: absolute;
  left: 0;
  width: 100%;
  background: rgba(255, 255, 255, 0.76);
  border: 1px dashed rgba(230, 126, 34, 0.85);
}

.cleanup-top {
  top: 0;
}

.cleanup-bottom {
  bottom: 0;
}

.preview-text {
  position: absolute;
  z-index: 2;
  color: #111827;
  white-space: nowrap;
  max-width: 90%;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: Arial, "PingFang SC", "Microsoft YaHei", sans-serif;
}

.detection-box {
  position: absolute;
  z-index: 3;
  border: 2px solid rgba(64, 158, 255, 0.92);
  background: rgba(64, 158, 255, 0.12);
}

.detection-footer {
  border-color: rgba(103, 194, 58, 0.92);
  background: rgba(103, 194, 58, 0.12);
}

.preview-error {
  padding: 12px;
  color: #b42318;
  background: #fff2f0;
  border: 1px solid #ffccc7;
  border-radius: 6px;
}

@media (max-width: 920px) {
  .hf-workbench {
    grid-template-columns: 1fr;
    height: auto;
    overflow: auto;
  }

  .session-summary {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>

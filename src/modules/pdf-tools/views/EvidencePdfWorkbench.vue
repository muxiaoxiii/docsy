<template>
  <div class="hf-workbench">
    <section class="hf-panel">
      <div class="section-head">
        <div>
          <h3>{{ workflowTitle }}</h3>
          <p class="hint">{{ workflowHint }}</p>
        </div>
        <div class="section-actions">
          <el-button v-if="workflowMode !== 'split'" type="primary" @click="selectOverlayFiles">{{
            splitImportButtonText
          }}</el-button>
          <el-button v-if="workflowMode !== 'merge'" :loading="importingMergedPdf" @click="importMergedPdfAsEvidence">{{
            mergedImportButtonText
          }}</el-button>
        </div>
      </div>

      <div v-if="importingMergedPdf" class="local-processing">
        <span class="processing-spinner" />
        <div>
          <strong>正在分析合并 PDF</strong>
          <p>大文件可能需要一段时间，当前只占用这个任务区域；其他标签和窗口仍可继续操作。</p>
        </div>
      </div>
      <div v-if="splittingMergedImport" class="local-processing">
        <span class="processing-spinner" />
        <div>
          <strong>正在拆分 PDF</strong>
          <p>大文件会按页段逐个输出，当前只占用合并证据处理区域；请先不要重复点击确认拆分。</p>
        </div>
      </div>
      <div v-if="overlaying" class="local-processing">
        <span class="processing-spinner" />
        <div>
          <strong>正在处理证据 PDF</strong>
          <p>页眉页脚、A4、批注和合并会在后台执行；文件较大时请等待当前批次完成。</p>
        </div>
      </div>
      <div v-if="detectingAllHeaderFooter" class="local-processing">
        <span class="processing-spinner" />
        <div>
          <strong>正在检测导入的文件</strong>
          <p>正在读取页眉页脚信息，完成后会显示在下方文件列表中。</p>
        </div>
      </div>

      <div v-if="showSessionSummary" class="session-summary">
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

      <div v-if="showSplitResultActions" class="split-result-actions">
        <div>
          <div class="block-title">{{ splitResultActionTitle }}</div>
          <p class="hint">{{ splitResultActionHint }}</p>
          <p class="path-text">输出文件夹：{{ splitReplacementOutputDirValue }}</p>
        </div>
        <div class="plan-actions">
          <el-button size="small" @click="selectSplitReplacementOutputDir">输出目录</el-button>
          <el-button size="small" @click="openHeaderFooterSettings">设置页眉页脚</el-button>
          <el-button
            size="small"
            type="primary"
            :loading="overlaying"
            :disabled="!canApplySplitReplacement"
            @click="applySplitHeaderFooterReplacement"
          >
            生成处理后文件
          </el-button>
        </div>
      </div>

      <div v-if="showExistingHeaderFooterControls" class="rule-block existing-hf-block">
        <div class="block-title-row">
          <div class="block-title">原页眉页脚</div>
          <div class="block-actions">
            <el-button size="small" :loading="detectingAllHeaderFooter" @click="detectAllHeaderFooter"
              >重新检测</el-button
            >
            <el-button size="small" :disabled="!hasDetectedExistingHeaderFooter" @click="markRemoveExistingHeaderFooter"
              >删除现有</el-button
            >
            <el-button size="small" :disabled="!hasExistingRemovalRule" @click="restoreExistingHeaderFooterMarks"
              >恢复删除标记</el-button
            >
          </div>
        </div>
        <div class="existing-summary-grid">
          <div class="summary-pill">
            <span>原页眉</span>
            <strong>{{ existingHeaderCount }}</strong>
          </div>
          <div class="summary-pill">
            <span>原页脚</span>
            <strong>{{ existingFooterCount }}</strong>
          </div>
          <div class="summary-pill">
            <span>原页码</span>
            <strong>{{ existingPageNumberCount }}</strong>
          </div>
          <div class="summary-pill warning" :class="{ active: hasExistingRemovalRule }">
            <span>待删除</span>
            <strong>{{ existingRemovalCount }}</strong>
          </div>
          <div class="summary-pill" :class="{ active: existingEditCount || existingConvertCount }">
            <span>待编辑/转换</span>
            <strong>{{ existingEditCount + existingConvertCount }}</strong>
          </div>
        </div>
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
        <div class="block-title-row">
          <div class="block-title">插入新页眉页脚</div>
          <el-switch v-model="insertHeaderFooterEnabled" active-text="插入" inactive-text="不插入" />
        </div>
        <HeaderFooterRuleFields
          v-if="insertHeaderFooterEnabled"
          class="rule-grid"
          v-model:header-mode="headerMode"
          v-model:header-text="headerText"
          v-model:header-prefix="headerPrefix"
          v-model:header-suffix="headerSuffix"
          v-model:header-align="headerAlign"
          v-model:header-font-size="headerFontSize"
          v-model:header-font-family="headerFontFamily"
          v-model:header-margin-mm="headerMarginMm"
          v-model:header-offset-x-mm="headerOffsetXMm"
          v-model:header-color="headerColor"
          v-model:footer-enabled="footerEnabled"
          v-model:footer-continuous="footerContinuous"
          v-model:footer-text="footerText"
          v-model:footer-align="footerAlign"
          v-model:footer-font-size="footerFontSize"
          v-model:footer-font-family="footerFontFamily"
          v-model:footer-margin-mm="footerMarginMm"
          v-model:footer-offset-x-mm="footerOffsetXMm"
          v-model:footer-color="footerColor"
          :offset-limit-mm="HORIZONTAL_OFFSET_LIMIT_MM"
        />
        <el-alert
          v-if="insertHeaderFooterEnabled && headerFooterOverflowWarnings.length"
          type="warning"
          :closable="false"
          show-icon
          class="processing-notes"
        >
          <template #title>{{ headerFooterOverflowWarnings.join('；') }}</template>
        </el-alert>
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
        <el-button :disabled="!overlayFiles.length" @click="refreshOverlayPageCounts" :loading="checkingOverlayPages"
          >刷新页数</el-button
        >
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
      <el-alert v-if="showRuleActionNotes" type="warning" :closable="false" show-icon class="processing-notes">
        <template #title>{{ processingNotes.join('；') }}</template>
      </el-alert>

      <div v-if="mergedImportPlan" class="merged-import-plan">
        <div class="plan-head">
          <div>
            <div class="block-title">合并证据页段确认</div>
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
          <span>已扫描：{{ mergedImportPlan.pagesAnalyzed || '-' }} 页</span>
          <span>页眉：{{ mergedImportPlan.headerPages || 0 }} 页</span>
          <span>页码页脚：{{ mergedImportPlan.pageNumberFooterPages || 0 }} 页</span>
          <span>输出目录：{{ mergedImportPlan.outputDir }}</span>
        </div>
        <div class="split-name-options">
          <div class="block-title">拆分文件名</div>
          <div class="rule-grid">
            <div class="rule-item">
              <label>前缀</label>
              <el-input v-model="splitNamePrefix" placeholder="可选" />
            </div>
            <div class="rule-item">
              <label>后缀</label>
              <el-input v-model="splitNameSuffix" placeholder="例如 [YYYYMMDD]、-[YYYYMMDD]、[##]" />
            </div>
            <div class="rule-item">
              <label>日期值</label>
              <el-input v-model="splitNameDateValue" placeholder="YYYYMMDD" />
            </div>
            <div class="rule-item">
              <label>分隔符</label>
              <el-select v-model="splitNameSeparator">
                <el-option label="-" value="-" />
                <el-option label="_" value="_" />
                <el-option label="空格" value=" " />
                <el-option label="无" value="" />
                <el-option label="自定义" value="custom" />
              </el-select>
            </div>
            <div class="rule-item" v-if="splitNameSeparator === 'custom'">
              <label>自定义分隔符</label>
              <el-input v-model="splitNameCustomSeparator" placeholder="输入分隔符" />
            </div>
          </div>
        </div>
        <div class="split-cleanup-options">
          <div class="block-title">拆分后处理</div>
          <el-checkbox v-model="splitCleanupHeader">删除页眉区内容</el-checkbox>
          <el-checkbox v-model="splitCleanupFooter">删除原页码/页脚区内容</el-checkbox>
          <span class="split-cleanup-note">仅在拆分输出文件时执行，不修改导入的合并 PDF。</span>
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
          @sort-change="sortMergedImportItems"
        >
          <el-table-column type="index" label="#" width="44" />
          <el-table-column label="文件名" prop="name" sortable="custom" min-width="160">
            <template #default="{ row }">
              <el-input v-model="row.name" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="输出文件名" prop="outputName" sortable="custom" min-width="180" show-overflow-tooltip>
            <template #default="{ row, $index }">{{ splitOutputNamePreview(row, $index) }}.pdf</template>
          </el-table-column>
          <el-table-column label="起始页" prop="pageStart" sortable="custom" width="108">
            <template #default="{ row }">
              <el-input-number
                v-model="row.pageStart"
                :min="1"
                :max="mergedImportPlan.totalPages || 999999"
                size="small"
              />
            </template>
          </el-table-column>
          <el-table-column label="结束页" prop="pageEnd" sortable="custom" width="108">
            <template #default="{ row }">
              <el-input-number
                v-model="row.pageEnd"
                :min="1"
                :max="mergedImportPlan.totalPages || 999999"
                size="small"
              />
            </template>
          </el-table-column>
          <el-table-column label="页数" prop="pageCount" sortable="custom" width="64">
            <template #default="{ row }">{{ mergedImportRangePageCount(row) || '-' }}</template>
          </el-table-column>
          <el-table-column label="识别来源" prop="source" sortable="custom" width="96">
            <template #default="{ row }">
              <el-tag :type="mergedImportSourceType(row)" size="small">
                {{ mergedImportSourceText(row) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="148">
            <template #default="{ row, $index }">
              <el-button link type="primary" size="small" @click.stop="selectMergedImportRange(row)">跳转</el-button>
              <el-button link type="primary" size="small" @click.stop="insertMergedImportRangeAfter($index)"
                >续段</el-button
              >
              <el-button link type="danger" size="small" @click.stop="removeMergedImportRange($index)">删除</el-button>
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
        @sort-change="sortOverlayFiles"
      >
        <el-table-column type="index" label="#" width="44" />
        <el-table-column label="文件" prop="name" sortable="custom" min-width="180" show-overflow-tooltip>
          <template #default="{ row }">
            <button class="file-link" type="button" @click.stop="openEvidenceFile(row)">
              {{ row.name }}
            </button>
          </template>
        </el-table-column>
        <el-table-column label="原页眉" prop="existingHeader" sortable="custom" min-width="140" show-overflow-tooltip>
          <template #default="{ row }">
            <el-input
              v-if="isEditingExistingHeader(row)"
              v-model="row.existingHeaderText"
              size="small"
              @click.stop
              @blur="finishExistingHeaderEdit(row)"
              @keyup.enter="finishExistingHeaderEdit(row)"
            />
            <span
              v-else
              class="table-text editable-text"
              :class="{ 'deleted-existing-text': row.removeExistingHeader }"
              @dblclick.stop="startExistingHeaderEdit(row)"
            >
              {{ displayExistingHeader(row) || '-' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="原页脚" prop="existingFooter" sortable="custom" min-width="140" show-overflow-tooltip>
          <template #default="{ row }">
            <el-input
              v-if="isEditingExistingFooter(row)"
              v-model="row.existingFooterText"
              size="small"
              @click.stop
              @blur="finishExistingFooterEdit(row)"
              @keyup.enter="finishExistingFooterEdit(row)"
            />
            <span
              v-else
              class="table-text editable-text"
              :class="{ 'deleted-existing-text': row.removeExistingFooter }"
              @dblclick.stop="startExistingFooterEdit(row)"
            >
              {{ displayExistingFooter(row) || '-' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column
          label="原页码"
          prop="existingPageNumber"
          sortable="custom"
          min-width="120"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            <el-input
              v-if="isEditingExistingPageNumber(row)"
              v-model="row.existingPageNumberText"
              size="small"
              @click.stop
              @blur="finishExistingPageNumberEdit(row)"
              @keyup.enter="finishExistingPageNumberEdit(row)"
            />
            <span
              v-else
              class="table-text editable-text"
              :class="{ 'deleted-existing-text': row.removeExistingPageNumber }"
              @dblclick.stop="startExistingPageNumberEdit(row)"
            >
              {{ displayExistingPageNumber(row) || '-' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="新页眉" prop="header" sortable="custom" min-width="160" show-overflow-tooltip>
          <template #default="{ row, $index }">
            <el-input
              v-if="isEditingHeader(row)"
              v-model="row.header"
              size="small"
              @click.stop
              @blur="finishHeaderEdit(row)"
              @keyup.enter="finishHeaderEdit(row)"
            />
            <span v-else class="table-text editable-text" @dblclick.stop="startHeaderEdit(row, $index)">
              {{ displayRowHeader(row, $index) || '-' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="新页脚" prop="footer" sortable="custom" min-width="135" show-overflow-tooltip>
          <template #default="{ row, $index }">
            <el-input
              v-if="isEditingFooter(row)"
              v-model="row.footer"
              size="small"
              @click.stop
              @blur="finishFooterEdit(row)"
              @keyup.enter="finishFooterEdit(row)"
            />
            <span v-else class="table-text editable-text" @dblclick.stop="startFooterEdit(row, $index)">
              {{ displayRowFooter(row, $index) || '-' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="页数" prop="pages" sortable="custom" width="70">
          <template #default="{ row }">{{ row.pages || '-' }}</template>
        </el-table-column>
        <el-table-column label="页码范围" prop="pageRange" sortable="custom" width="105">
          <template #default="{ row }">{{ pageRangeText(row) }}</template>
        </el-table-column>
        <el-table-column label="现有处理" prop="existingHandling" sortable="custom" width="150">
          <template #default="{ row }">
            <span class="table-text">{{ headerFooterHandlingText(row) }}</span>
          </template>
        </el-table-column>
        <el-table-column
          v-if="workflowMode === 'split'"
          label="来源页段"
          prop="sourceRange"
          sortable="custom"
          width="105"
        >
          <template #default="{ row }">{{ sourceRangeText(row) }}</template>
        </el-table-column>
        <el-table-column label="状态" prop="status" sortable="custom" width="92">
          <template #default="{ row }">
            <el-tag :type="row.statusType" size="small">{{ row.statusText }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ $index }">
            <el-button link size="small" :disabled="$index === 0" @click.stop="moveOverlayFile($index, -1)"
              >上移</el-button
            >
            <el-button
              link
              size="small"
              :disabled="$index === overlayRows.length - 1"
              @click.stop="moveOverlayFile($index, 1)"
              >下移</el-button
            >
            <el-button link type="danger" size="small" @click.stop="removeOverlayFile($index)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <el-dialog v-model="headerFooterSettingsVisible" width="760px" destroy-on-close>
      <template #header>
        <div class="dialog-title-row">
          <span>页眉页脚格式</span>
          <el-switch v-model="insertHeaderFooterEnabled" active-text="插入" inactive-text="不插入" />
        </div>
      </template>
      <HeaderFooterRuleFields
        v-if="insertHeaderFooterEnabled"
        class="dialog-rule-grid"
        v-model:header-mode="headerMode"
        v-model:header-text="headerText"
        v-model:header-prefix="headerPrefix"
        v-model:header-suffix="headerSuffix"
        v-model:header-align="headerAlign"
        v-model:header-font-size="headerFontSize"
        v-model:header-font-family="headerFontFamily"
        v-model:header-margin-mm="headerMarginMm"
        v-model:header-offset-x-mm="headerOffsetXMm"
        v-model:header-color="headerColor"
        v-model:footer-enabled="footerEnabled"
        v-model:footer-continuous="footerContinuous"
        v-model:footer-text="footerText"
        v-model:footer-align="footerAlign"
        v-model:footer-font-size="footerFontSize"
        v-model:footer-font-family="footerFontFamily"
        v-model:footer-margin-mm="footerMarginMm"
        v-model:footer-offset-x-mm="footerOffsetXMm"
        v-model:footer-color="footerColor"
        :show-footer-continuous="true"
        :offset-limit-mm="HORIZONTAL_OFFSET_LIMIT_MM"
      />
      <template #footer>
        <el-button @click="headerFooterSettingsVisible = false">关闭</el-button>
        <el-button type="primary" @click="applyHeaderFooterSettings">应用到预览</el-button>
      </template>
    </el-dialog>

    <section class="preview-panel">
      <div class="preview-head">
        <div>
          <h3>位置预览</h3>
          <p class="hint">{{ previewHint }}</p>
        </div>
        <div class="preview-controls">
          <template v-if="mergedImportPlan">
            <el-button size="small" :disabled="previewPage <= 1" @click="movePreviewPage(-1)">上一页</el-button>
            <el-button size="small" :disabled="previewPage >= previewMaxPage" @click="movePreviewPage(1)"
              >下一页</el-button
            >
            <el-button size="small" :disabled="!selectedMergedImportRange" @click="setSelectedMergedRangeStart"
              >设为起始页</el-button
            >
            <el-button size="small" :disabled="!selectedMergedImportRange" @click="setSelectedMergedRangeEnd"
              >设为结束页</el-button
            >
          </template>
          <el-input-number
            v-model="previewPage"
            :min="1"
            :max="previewMaxPage"
            :disabled="!activePreviewFilePath"
            size="small"
          />
          <el-button size="small" :disabled="!activePreviewFilePath" @click="refreshPreview">重新渲染当前页</el-button>
          <el-button
            size="small"
            type="primary"
            :loading="truePreviewLoading"
            :disabled="!selectedOverlayFile || Boolean(mergedImportPlan)"
            @click="renderTruePreview"
          >
            生成真实预览
          </el-button>
        </div>
      </div>

      <div v-if="footerCandidatePanelVisible" class="footer-candidate-panel">
        <div class="candidate-panel-head">
          <div>
            <div class="block-title">页脚候选确认</div>
            <p class="hint">检测到多个底部文本候选，请指定它是页脚、页码或忽略</p>
          </div>
        </div>
        <div class="footer-candidate-list">
          <div
            v-for="candidate in selectedFooterCandidates"
            :key="candidateKey(candidate)"
            class="footer-candidate-item"
            :class="{ active: selectedFooterCandidateKey === candidateKey(candidate) }"
          >
            <button class="candidate-main" type="button" @click="previewFooterCandidate(candidate)">
              <strong>{{ candidate.text || candidate.normalizedText }}</strong>
              <span>{{ footerCandidateMeta(candidate) }}</span>
            </button>
            <el-tag size="small" :type="footerCandidateRoleType(candidate)">{{
              footerCandidateRoleText(candidate)
            }}</el-tag>
            <el-button link size="small" type="primary" @click="assignFooterCandidate(candidate, 'footer')"
              >设为页脚</el-button
            >
            <el-button link size="small" type="primary" @click="assignFooterCandidate(candidate, 'pageNumber')"
              >设为页码</el-button
            >
            <el-button link size="small" @click="assignFooterCandidate(candidate, 'ignore')">忽略</el-button>
          </div>
        </div>
      </div>

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
          <div
            v-for="marker in deletionPreviewMarkers"
            :key="marker.key"
            class="delete-preview-marker"
            :style="marker.style"
          >
            <span>{{ marker.label }}</span>
          </div>
          <div
            v-if="footerCandidatePreviewMarker"
            class="footer-candidate-marker"
            :style="footerCandidatePreviewMarker.style"
          >
            <span>{{ footerCandidatePreviewMarker.label }}</span>
          </div>
          <div
            v-for="overlay in convertedExistingPreviewOverlays"
            :key="overlay.key"
            class="preview-text"
            :class="overlay.region === 'header' ? 'preview-header-text' : 'preview-footer-text'"
            :style="overlay.style"
          >
            {{ overlay.text }}
          </div>
          <div
            v-if="showRulePreviewOverlays && previewHeaderText"
            class="preview-text preview-header-text"
            :class="{ 'with-delete-background': deletionPreviewMarkers.length }"
            :style="previewHeaderStyle"
          >
            {{ previewHeaderText }}
          </div>
          <div
            v-if="showRulePreviewOverlays && previewFooterText"
            class="preview-text preview-footer-text"
            :class="{ 'with-delete-background': deletionPreviewMarkers.length }"
            :style="previewFooterStyle"
          >
            {{ previewFooterText }}
          </div>
        </template>
      </PdfJsPreview>
    </section>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../components/PdfJsPreview.vue'
import HeaderFooterRuleFields from '../components/HeaderFooterRuleFields.vue'
import {
  buildEvidencePdfRulePayload,
  buildMergeOutputPath,
  buildOutputDir,
  createEvidenceFile,
  expandPlaceholders,
  fileName,
  pageRangeText,
  parentDir,
  sortByNatural,
  totalPages,
  updatePageRanges,
} from '../composables/useEvidencePdfSession.js'
import { todayCompact } from '../composables/splitFileName.js'
import { useEvidencePdfDetection } from '../composables/useEvidencePdfDetection.js'
import { useEvidencePdfPreview } from '../composables/useEvidencePdfPreview.js'
import { useEvidencePdfMergedImport } from '../composables/useEvidencePdfMergedImport.js'
import { useEvidencePdfExistingEditing } from '../composables/useEvidencePdfExistingEditing.js'
import { openPath, tauriCallSafe } from '../../../core/tauriBridge.js'

const props = defineProps({
  workflow: {
    type: String,
    default: 'all',
  },
})

const workflowMode = computed(() => (['merge', 'split'].includes(props.workflow) ? props.workflow : 'all'))
const workflowTitle = computed(() => {
  if (workflowMode.value === 'merge') return '分项证据处理'
  if (workflowMode.value === 'split') return '合并证据处理'
  return '证据处理'
})
const workflowHint = computed(() => {
  if (workflowMode.value === 'merge') return '处理分项证据 PDF 的页眉、连续页码、A4、批注，并按需合并输出'
  if (workflowMode.value === 'split') return '处理已合并证据 PDF；单个文件可识别页段拆分，多个文件可按统一规则批量处理'
  return '按法律证据包流程处理页眉、页码、A4、批注、合并与反向拆分'
})
const splitImportButtonText = computed(() => '导入分项证据 PDF')
const mergedImportButtonText = computed(() => '导入合并证据 PDF')
const splitResultActionTitle = computed(() => (hasSourceSplitRanges.value ? '分项文件后处理' : '合并证据批量处理'))
const splitResultActionHint = computed(() =>
  hasSourceSplitRanges.value
    ? '处理后文件会输出到新文件夹，不覆盖原分项文件。'
    : '多个合并证据 PDF 会按同一规则批量输出到新文件夹，不覆盖原文件。',
)

const overlayFiles = ref([])
const overlayOutputDir = ref('')
const checkingOverlayPages = ref(false)
const overlaying = ref(false)
const importingMergedPdf = ref(false)
const splittingMergedImport = ref(false)
const selectedOverlayIndex = ref(0)
const selectedMergedImportIndex = ref(0)
const mergedImportPlan = ref(null)
const splitNamePrefix = ref('')
const splitNameSuffix = ref('[YYYYMMDD]')
const splitNameDateValue = ref(todayCompact())
const splitNameSeparator = ref('-')
const splitNameCustomSeparator = ref('')
const headerFooterSettingsVisible = ref(false)
const splitReplacementOutputDir = ref('')
const splitCleanupHeader = ref(false)
const splitCleanupFooter = ref(false)

const DEFAULT_RASTER_DPI = 200
const HORIZONTAL_OFFSET_LIMIT_MM = 120

const normalizeA4 = ref(false)
const a4Orientation = ref('auto')
const rasterDpi = ref(DEFAULT_RASTER_DPI)
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
const cleanupHeaderHeightMm = ref(18)
const cleanupFooterHeightMm = ref(18)
const insertHeaderFooterEnabled = ref(true)
const headerMode = ref('filename')
const headerText = ref('')
const headerPrefix = ref('')
const headerSuffix = ref('')
const headerAlign = ref('right')
const headerFontSize = ref(10)
const headerFontFamily = ref('auto')
const headerMarginMm = ref(10)
const headerOffsetXMm = ref(0)
const headerColor = ref('#000000')
const footerEnabled = ref(true)
const footerText = ref('{page}/{total}')
const footerContinuous = ref(true)
const footerAlign = ref('center')
const footerFontSize = ref(9)
const footerFontFamily = ref('auto')
const footerMarginMm = ref(10)
const footerOffsetXMm = ref(0)
const footerColor = ref('#000000')
const outputMode = ref('files_and_merge')
const mergeFileName = ref('merged_evidence.pdf')
const previewPage = ref(1)
const previewReloadKey = ref(0)
const previewData = ref({})
const truePreview = ref(null)
const truePreviewLoading = ref(false)
const detectingAllHeaderFooter = ref(false)
const editingHeaderPath = ref('')
const editingFooterPath = ref('')
const editingExistingHeaderPath = ref('')
const editingExistingFooterPath = ref('')
const editingExistingPageNumberPath = ref('')
const selectedFooterCandidateKey = ref('')

watch(
  headerMode,
  (mode) => {
    if (mode === 'template') {
      headerMode.value = 'custom'
    } else if (mode === 'seq') {
      headerText.value = '证据[序号]'
      headerMode.value = 'custom'
    } else if (mode === 'seq_cn') {
      headerText.value = '证据[中文序号]'
      headerMode.value = 'custom'
    } else if (mode === 'prefix_seq') {
      headerText.value = `${headerText.value || ''}证据[序号]`
      headerMode.value = 'custom'
    }
  },
  { immediate: true },
)

applyWorkflowDefaults()

const overlayRows = computed(() => {
  return updatePageRanges(overlayFiles.value)
})
const hasSourceSplitRanges = computed(() => overlayFiles.value.some((file) => Number(file.sourcePageStart || 0) > 0))
const hasMergedBatchImports = computed(
  () => workflowMode.value === 'split' && overlayFiles.value.length > 0 && !hasSourceSplitRanges.value,
)

const selectedOverlayFile = computed(() => overlayRows.value[selectedOverlayIndex.value] || null)
const activePreviewFilePath = computed(() => mergedImportPlan.value?.inputPath || selectedOverlayFile.value?.path || '')
const previewMaxPage = computed(() => {
  if (mergedImportPlan.value) return Math.max(1, Number(mergedImportPlan.value.totalPages || 1))
  return selectedOverlayFile.value?.pages || 1
})
const previewHint = computed(() => (mergedImportPlan.value ? '合并 PDF 原文预览' : '实时位置；真实预览需手动生成'))
const totalOverlayPages = computed(() => totalPages(overlayFiles.value))
const plannedOutputDir = computed(() => buildOutputDir(overlayRows.value, overlayOutputDir.value))
const plannedMergeOutputPath = computed(() =>
  buildMergeOutputPath(overlayRows.value, overlayOutputDir.value, mergeFileName.value),
)
const firstHeaderPreview = computed(() => {
  if (!insertHeaderFooterEnabled.value) return ''
  const first = overlayRows.value[0]
  return first ? rowHeaderPreview(first, 0) : ''
})
const firstFooterPreview = computed(() => {
  if (!insertHeaderFooterEnabled.value) return ''
  const first = overlayRows.value[0]
  if (!first || !footerEnabled.value || !footerText.value || !totalOverlayPages.value) return ''
  return expandPlaceholders(
    footerText.value,
    footerContinuous.value ? first.pageStart || 1 : 1,
    footerContinuous.value ? totalOverlayPages.value : first.pages || 1,
  )
})
const processingNotes = computed(() => {
  const notes = []
  if (normalizeA4.value) {
    notes.push('A4 规范化会把小页面居中补白到 A4，超过 A4 的页面才等比缩小；会尽量保留原 PDF 内容层')
  }
  if (removeAnnotations.value) {
    notes.push('删除批注只处理评论、高亮等批注对象，已扁平化到正文的标记不会被对象删除')
  }
  if (outputMode.value === 'merge_only') {
    notes.push('只生成合并 PDF 时，中间单文件副本会在合并成功后清理')
  }
  if (hasExistingRemovalRule.value) {
    notes.push('删除现有页眉页脚不会使用白色遮盖；只能删除标准结构或已确认匹配的普通文本')
  }
  if (hasExistingEditRule.value) {
    notes.push('原页眉、原页脚、原页码列中的标准结构编辑会尽量原位处理；普通文本型旧内容会先删除匹配文本再按原位置重建')
  }
  if (hasExistingConvertRule.value) {
    notes.push('普通文本型旧页眉页脚页码会先删除匹配文本，再按检测到的位置重建')
  }
  if (hasUnresolvedExistingOverlapRisk.value) {
    notes.push('存在未处理的原页眉页脚，新插入内容可能与旧内容重叠')
  }
  return notes
})
const showProcessingControls = computed(
  () => !mergedImportPlan.value && (workflowMode.value !== 'split' || hasMergedBatchImports.value),
)
const showSessionSummary = computed(
  () =>
    overlayFiles.value.length > 0 &&
    !mergedImportPlan.value &&
    (workflowMode.value !== 'split' || hasMergedBatchImports.value),
)
const showSplitResultActions = computed(
  () =>
    workflowMode.value === 'split' &&
    overlayFiles.value.length > 0 &&
    !mergedImportPlan.value &&
    hasSourceSplitRanges.value,
)
const showExistingHeaderFooterControls = computed(() => overlayFiles.value.length > 0 && !mergedImportPlan.value)
const showRuleActionNotes = computed(
  () => (showProcessingControls.value || showSplitResultActions.value) && processingNotes.value.length > 0,
)
const splitReplacementOutputDirValue = computed(
  () => splitReplacementOutputDir.value || defaultSplitReplacementOutputDir(),
)
const processButtonText = computed(() => (workflowMode.value === 'merge' ? '执行分项证据处理' : '执行合并证据处理'))
const autoCleanupHeaderEnabled = computed(() =>
  overlayFiles.value.some(
    (file) => file.existingHeaderArtifact && file.existingHeaderEdited && !file.removeExistingHeader,
  ),
)
const autoCleanupFooterEnabled = computed(() =>
  overlayFiles.value.some(
    (file) => file.existingFooterArtifact && file.existingFooterEdited && !file.removeExistingFooter,
  ),
)
const hasDetectedExistingHeaderFooter = computed(() => overlayFiles.value.some((file) => hasExistingHeaderFooter(file)))
const existingHeaderCount = computed(() => overlayFiles.value.filter((file) => hasExistingHeader(file)).length)
const existingFooterCount = computed(() => overlayFiles.value.filter((file) => hasExistingFooter(file)).length)
const existingPageNumberCount = computed(() => overlayFiles.value.filter((file) => hasExistingPageNumber(file)).length)
const hasExistingRemovalRule = computed(() =>
  overlayFiles.value.some(
    (file) => file.removeExistingHeader || file.removeExistingFooter || file.removeExistingPageNumber,
  ),
)
const existingRemovalCount = computed(() =>
  overlayFiles.value.reduce(
    (sum, file) =>
      sum +
      Number(Boolean(file.removeExistingHeader)) +
      Number(Boolean(file.removeExistingFooter)) +
      Number(Boolean(file.removeExistingPageNumber)),
    0,
  ),
)
const hasExistingEditRule = computed(() =>
  overlayFiles.value.some(
    (file) =>
      (file.existingHeaderArtifact && file.existingHeaderEdited && !file.removeExistingHeader) ||
      (file.existingFooterArtifact && file.existingFooterEdited && !file.removeExistingFooter),
  ),
)
const hasExistingConvertRule = computed(() =>
  overlayFiles.value.some(
    (file) =>
      (file.convertPlainHeader && !file.removeExistingHeader) ||
      (file.convertPlainFooter && !file.removeExistingFooter) ||
      (file.convertPlainPageNumber && !file.removeExistingPageNumber),
  ),
)
const existingEditCount = computed(() =>
  overlayFiles.value.reduce(
    (sum, file) =>
      sum +
      Number(Boolean(file.existingHeaderArtifact && file.existingHeaderEdited && !file.removeExistingHeader)) +
      Number(Boolean(file.existingFooterArtifact && file.existingFooterEdited && !file.removeExistingFooter)),
    0,
  ),
)
const existingConvertCount = computed(() =>
  overlayFiles.value.reduce(
    (sum, file) =>
      sum +
      Number(Boolean(file.convertPlainHeader && !file.removeExistingHeader)) +
      Number(Boolean(file.convertPlainFooter && !file.removeExistingFooter)) +
      Number(Boolean(file.convertPlainPageNumber && !file.removeExistingPageNumber)),
    0,
  ),
)
const hasUnresolvedExistingOverlapRisk = computed(() => {
  const insertsHeader = insertHeaderFooterEnabled.value && headerMode.value !== 'none'
  const insertsFooter = insertHeaderFooterEnabled.value && footerEnabled.value
  return overlayFiles.value.some(
    (file) =>
      (insertsHeader && hasExistingHeader(file) && !file.removeExistingHeader && !file.convertPlainHeader) ||
      (insertsFooter && hasExistingFooter(file) && !file.removeExistingFooter && !file.convertPlainFooter) ||
      (insertsFooter && hasExistingPageNumber(file) && !file.removeExistingPageNumber && !file.convertPlainPageNumber),
  )
})
const canApplyOverlay = computed(
  () => overlayFiles.value.length > 0 && totalOverlayPages.value > 0 && hasApplicableProcessingRule.value,
)
const canApplySplitReplacement = computed(
  () =>
    showSplitResultActions.value &&
    totalOverlayPages.value > 0 &&
    !overlaying.value &&
    hasApplicableProcessingRule.value,
)
const hasApplicableProcessingRule = computed(
  () =>
    normalizeA4.value ||
    removeAnnotations.value ||
    hasExistingEditRule.value ||
    hasExistingConvertRule.value ||
    hasExistingRemovalRule.value ||
    (insertHeaderFooterEnabled.value && (headerMode.value !== 'none' || footerEnabled.value)),
)

const currentRules = computed(() => ({
  normalizeA4: normalizeA4.value,
  a4Orientation: a4Orientation.value,
  rasterDpi: rasterDpi.value,
  removeAnnotations: removeAnnotations.value,
  annotationKinds: annotationKinds.value,
  cleanupHeaderEnabled: autoCleanupHeaderEnabled.value,
  cleanupFooterEnabled: autoCleanupFooterEnabled.value,
  cleanupHeaderHeightMm: cleanupHeaderHeightMm.value,
  cleanupFooterHeightMm: cleanupFooterHeightMm.value,
  headerMode: insertHeaderFooterEnabled.value ? headerMode.value : 'none',
  headerText: headerText.value,
  headerPrefix: headerPrefix.value,
  headerSuffix: headerSuffix.value,
  headerDateValue: splitNameDateValue.value,
  headerAlign: headerAlign.value,
  headerFontSize: headerFontSize.value,
  headerFontFamily: headerFontFamily.value,
  headerMarginMm: headerMarginMm.value,
  headerOffsetXMm: headerOffsetXMm.value,
  headerColor: headerColor.value,
  footerEnabled: insertHeaderFooterEnabled.value && footerEnabled.value,
  footerText: footerText.value,
  footerContinuous: footerContinuous.value,
  footerAlign: footerAlign.value,
  footerFontSize: footerFontSize.value,
  footerFontFamily: footerFontFamily.value,
  footerMarginMm: footerMarginMm.value,
  footerOffsetXMm: footerOffsetXMm.value,
  footerColor: footerColor.value,
  outputMode: outputMode.value,
  mergeAfterProcessing: outputMode.value !== 'files_only',
  mergeFileName: mergeFileName.value,
}))

const {
  detectAllHeaderFooter,
  candidateKey,
  footerCandidateMeta,
  footerCandidateRoleText: detectionFooterCandidateRoleText,
  footerCandidateRoleType: detectionFooterCandidateRoleType,
  previewFooterCandidate: detectionPreviewFooterCandidate,
  assignFooterCandidate: detectionAssignFooterCandidate,
  fileExistingStatus,
  hasExistingHeader,
  hasExistingFooter,
  hasExistingPageNumber,
} = useEvidencePdfDetection({
  overlayRows,
  detectingAllHeaderFooter,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
})

const {
  showRulePreviewOverlays,
  previewHeaderText,
  previewFooterText,
  previewHeaderStyle,
  previewFooterStyle,
  truePreviewFrameStyle,
  selectedFooterCandidates,
  footerCandidatePanelVisible,
  footerCandidatePreviewMarker,
  deletionPreviewMarkers,
  convertedExistingPreviewOverlays,
  headerFooterOverflowWarnings,
  refreshPreview,
  safeRefreshPreview,
  movePreviewPage,
  selectPreviewRow,
  renderTruePreview,
  handlePreviewLoaded,
  handlePreviewError,
} = useEvidencePdfPreview({
  selectedOverlayFile,
  selectedOverlayIndex,
  previewPage,
  previewReloadKey,
  previewData,
  truePreview,
  truePreviewLoading,
  previewMaxPage,
  mergedImportPlan,
  overlayRows,
  currentRules,
  overlayOutputDir,
  insertHeaderFooterEnabled,
  headerMode,
  footerEnabled,
  footerContinuous,
  totalOverlayPages,
  headerAlign,
  headerMarginMm,
  headerFontSize,
  headerFontFamily,
  headerOffsetXMm,
  headerColor,
  footerAlign,
  footerMarginMm,
  footerFontSize,
  footerFontFamily,
  footerOffsetXMm,
  footerColor,
  footerText,
  removeAnnotations,
  annotationKinds,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
  selectedFooterCandidateKey,
})

function footerCandidateRoleText(candidate) {
  return detectionFooterCandidateRoleText(candidate, selectedOverlayFile)
}

function footerCandidateRoleType(candidate) {
  return detectionFooterCandidateRoleType(candidate, selectedOverlayFile)
}

function previewFooterCandidate(candidate) {
  selectedFooterCandidateKey.value = candidateKey(candidate)
  detectionPreviewFooterCandidate(candidate, selectedOverlayFile, previewMaxPage, previewPage, truePreview)
}

function assignFooterCandidate(candidate, role) {
  detectionAssignFooterCandidate(
    candidate,
    role,
    selectedOverlayFile,
    selectedFooterCandidateKey,
    truePreview,
    refreshPreview,
  )
}

const {
  mergedImportWarnings,
  selectedMergedImportRange,
  importMergedPdfAsEvidence,
  executeMergedImportPlan,
  selectMergedImportOutputDir,
  cancelMergedImportPlan,
  splitOutputNamePreview,
  selectMergedImportRange,
  setSelectedMergedRangeStart,
  setSelectedMergedRangeEnd,
  addMergedImportRange,
  insertMergedImportRangeAfter,
  removeMergedImportRange,
  sortMergedImportItems,
  mergedImportRangePageCount,
  mergedImportSourceType,
  mergedImportSourceText,
} = useEvidencePdfMergedImport({
  overlayFiles,
  overlayOutputDir,
  importingMergedPdf,
  splittingMergedImport,
  mergedImportPlan,
  selectedMergedImportIndex,
  selectedOverlayIndex,
  previewPage,
  truePreview,
  splitNamePrefix,
  splitNameSuffix,
  splitNameDateValue,
  splitNameSeparator,
  splitNameCustomSeparator,
  splitReplacementOutputDir,
  splitCleanupHeader,
  splitCleanupFooter,
  previewMaxPage,
  cleanupHeaderHeightMm,
  cleanupFooterHeightMm,
  detectAllHeaderFooter,
  refreshPreview,
  safeRefreshPreview,
  applyWorkflowDefaults,
  refreshOverlayPageCounts,
})

watch(selectedOverlayFile, () => {
  previewPage.value = 1
  truePreview.value = null
  selectedFooterCandidateKey.value = selectedOverlayFile.value?.footerCandidateChoices?.[0]
    ? candidateKey(selectedOverlayFile.value.footerCandidateChoices[0])
    : ''
})

watch(
  [previewPage, currentRules],
  () => {
    truePreview.value = null
  },
  {
    deep: true,
  },
)

function applyWorkflowDefaults() {
  if (workflowMode.value === 'split') {
    insertHeaderFooterEnabled.value = false
    headerMode.value = 'none'
    footerEnabled.value = false
    footerContinuous.value = true
    outputMode.value = 'files_only'
    mergeFileName.value = 'split_evidence.pdf'
    return
  }
  insertHeaderFooterEnabled.value = true
  headerMode.value = 'filename'
  footerEnabled.value = true
  footerContinuous.value = true
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
  await detectAllHeaderFooter({ silent: true })
}
async function selectOverlayOutputDir() {
  const selected = await open({ directory: true })
  if (selected) overlayOutputDir.value = selected
}

async function openPlannedOutputDir() {
  if (!overlayFiles.value.length) return
  const result = await openPath(plannedOutputDir.value)
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开输出文件夹')
  }
}

async function openEvidenceFile(row) {
  const path = row?.outputPath || row?.path
  if (!path) return
  const result = await openPath(path)
  if (!result.ok) {
    ElMessage.error(result.error || '无法打开 PDF 文件')
  }
}

async function selectSplitReplacementOutputDir() {
  const selected = await open({ directory: true })
  if (selected) splitReplacementOutputDir.value = selected
}

function defaultSplitReplacementOutputDir() {
  const base = overlayOutputDir.value || parentDir(overlayFiles.value[0]?.path || '.')
  return `${base}/页眉页码替换`
}

async function refreshOverlayPageCounts() {
  if (checkingOverlayPages.value) return
  checkingOverlayPages.value = true
  try {
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
  } finally {
    checkingOverlayPages.value = false
  }
}

function openHeaderFooterSettings() {
  ensureReplacementPreset()
  headerFooterSettingsVisible.value = true
}

function applyHeaderFooterSettings() {
  headerFooterSettingsVisible.value = false
  refreshPreview()
}

function applyReplacementPreset() {
  insertHeaderFooterEnabled.value = true
  normalizeA4.value = false
  removeAnnotations.value = false
  cleanupHeaderHeightMm.value = 18
  cleanupFooterHeightMm.value = 18
  headerMode.value = workflowMode.value === 'split' ? 'per_file' : 'filename'
  headerAlign.value = 'right'
  headerFontSize.value = 10
  headerFontFamily.value = 'auto'
  headerMarginMm.value = 10
  footerEnabled.value = true
  footerContinuous.value = true
  footerText.value = '{page}/{total}'
  footerAlign.value = 'center'
  footerFontSize.value = 9
  footerFontFamily.value = 'auto'
  footerMarginMm.value = 10
  outputMode.value = 'files_and_merge'
  refreshPreview()
}

function hasReplacementRule() {
  return (
    (insertHeaderFooterEnabled.value && (headerMode.value !== 'none' || footerEnabled.value)) ||
    hasExistingEditRule.value ||
    hasExistingConvertRule.value ||
    hasExistingRemovalRule.value
  )
}

function ensureReplacementPreset() {
  if (!hasReplacementRule()) {
    applyReplacementPreset()
  }
}

async function applySplitHeaderFooterReplacement() {
  if (!canApplySplitReplacement.value) return
  ensureReplacementPreset()
  headerFooterSettingsVisible.value = false
  overlaying.value = true
  try {
    const outputDir = splitReplacementOutputDirValue.value
    const rules = {
      ...currentRules.value,
      outputMode: 'files_only',
      mergeAfterProcessing: false,
    }
    const payload = buildEvidencePdfRulePayload(overlayRows.value, rules, outputDir)
    overlayRows.value.forEach((file) => {
      file.statusText = '替换中'
      file.statusType = 'warning'
    })

    const result = await tauriCallSafe('apply_evidence_pdf_rules', { args: payload })
    if (!result.ok) {
      ElMessage.error(result.error || '页眉页码替换失败')
      overlayFiles.value.forEach((file) => {
        file.statusText = '失败'
        file.statusType = 'danger'
      })
      return
    }

    const successByInput = new Map((result.data.results || []).map((item) => [item.inputPath, item]))
    const failedByInput = new Map((result.data.failed || []).map((item) => [item.path, item]))
    overlayFiles.value.forEach((file) => {
      const success = successByInput.get(file.path)
      const failed = failedByInput.get(file.path)
      if (success) {
        file.path = success.outputPath
        file.outputPath = success.outputPath
        file.name = fileName(success.outputPath)
        const warnings = success.warnings || []
        file.statusText = warnings.length ? '已替换，需注意' : '已替换'
        file.statusType = warnings.length ? 'warning' : 'success'
      } else if (failed) {
        file.statusText = '失败'
        file.statusType = 'danger'
      }
    })
    overlayOutputDir.value = outputDir
    splitReplacementOutputDir.value = outputDir
    selectedOverlayIndex.value = 0
    previewPage.value = 1
    refreshPreview()

    const failedCount = result.data.failed?.length || 0
    const successCount = result.data.results?.length || 0
    const warningCount = (result.data.results || []).filter((item) => item.warnings?.length).length
    if (failedCount) {
      ElMessage.warning(`已替换 ${successCount} 个，失败 ${failedCount} 个`)
    } else if (warningCount) {
      ElMessage.warning(`已替换 ${successCount} 个 PDF，其中 ${warningCount} 个有字体降级提示`)
    } else {
      ElMessage.success(`已替换 ${successCount} 个 PDF`)
    }
  } catch (err) {
    ElMessage.error(`页眉页码替换失败：${String(err?.message || err || '未知错误')}`)
  } finally {
    overlaying.value = false
  }
}

async function applyHeaderFooter() {
  if (!canApplyOverlay.value) return
  overlaying.value = true
  try {
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
      return
    }

    const successByInput = new Map((result.data.results || []).map((item) => [item.inputPath, item]))
    const failedByInput = new Map((result.data.failed || []).map((item) => [item.path, item]))
    overlayFiles.value.forEach((file) => {
      const success = successByInput.get(file.path)
      const failed = failedByInput.get(file.path)
      if (success) {
        file.outputPath = success.outputPath
        const warnings = success.warnings || []
        file.statusText = warnings.length ? '完成，需注意' : '完成'
        file.statusType = warnings.length ? 'warning' : 'success'
      } else if (failed) {
        file.statusText = '失败'
        file.statusType = 'danger'
      }
    })

    const failedCount = result.data.failed?.length || 0
    const successCount = result.data.results?.length || 0
    const warningCount = (result.data.results || []).filter((item) => item.warnings?.length).length
    const merge = result.data.merge
    if (merge?.status === 'done') {
      const cleanupText =
        merge.outputMode === 'merge_only' ? `，已清理 ${merge.removedIntermediates || 0} 个中间副本` : ''
      const warningText = warningCount ? `，其中 ${warningCount} 个有字体降级提示` : ''
      ElMessage.success(`已完成 ${successCount} 个 PDF，并已合并${cleanupText}${warningText}`)
    } else if (failedCount) {
      ElMessage.warning(`已完成 ${successCount} 个，失败 ${failedCount} 个`)
    } else if (warningCount) {
      ElMessage.warning(`已完成 ${successCount} 个 PDF，其中 ${warningCount} 个有字体降级提示`)
    } else if (merge?.status === 'skipped') {
      ElMessage.warning(`已完成 ${successCount} 个 PDF，${merge.message || '未合并'}`)
    } else {
      ElMessage.success(`已完成 ${successCount} 个 PDF`)
    }
  } catch (err) {
    ElMessage.error(`PDF 处理失败：${String(err?.message || err || '未知错误')}`)
    overlayFiles.value.forEach((file) => {
      file.statusText = '失败'
      file.statusType = 'danger'
    })
  } finally {
    overlaying.value = false
  }
}

const {
  isEditingHeader,
  startHeaderEdit,
  finishHeaderEdit,
  isEditingFooter,
  startFooterEdit,
  finishFooterEdit,
  isEditingExistingHeader,
  startExistingHeaderEdit,
  finishExistingHeaderEdit,
  isEditingExistingFooter,
  startExistingFooterEdit,
  finishExistingFooterEdit,
  isEditingExistingPageNumber,
  startExistingPageNumberEdit,
  finishExistingPageNumberEdit,
  rowHeaderPreview,
  displayRowHeader,
  displayRowFooter,
  displayExistingHeader,
  displayExistingFooter,
  displayExistingPageNumber,
} = useEvidencePdfExistingEditing({
  editingHeaderPath,
  editingFooterPath,
  editingExistingHeaderPath,
  editingExistingFooterPath,
  editingExistingPageNumberPath,
  insertHeaderFooterEnabled,
  headerMode,
  footerEnabled,
  footerContinuous,
  totalOverlayPages,
  currentRules,
  workflowMode,
  footerText,
  hasExistingHeader,
  hasExistingFooter,
  hasExistingPageNumber,
  fileExistingStatus,
  refreshPreview,
})
function hasExistingHeaderFooter(row) {
  return hasExistingHeader(row) || hasExistingFooter(row) || hasExistingPageNumber(row)
}

async function markRemoveExistingHeaderFooter() {
  const targets = overlayFiles.value.filter(hasExistingHeaderFooter)
  if (!targets.length) {
    ElMessage.info('当前列表没有检测到现有页眉页脚')
    return
  }
  try {
    await ElMessageBox.confirm(
      '将标记删除检测到的现有页眉页脚。标准结构会直接删除；普通文本只删除页眉页脚区域内匹配内容，匹配不到的文本会保留。是否插入新页眉页脚由当前设置决定。',
      '删除现有页眉页脚',
      {
        confirmButtonText: '标记删除',
        cancelButtonText: '取消',
        type: 'warning',
      },
    )
  } catch {
    return
  }
  targets.forEach((file) => {
    if (hasExistingHeader(file)) {
      file.removeExistingHeader = true
      file.existingHeaderEdited = false
    }
    if (hasExistingFooter(file)) {
      file.removeExistingFooter = true
      file.existingFooterEdited = false
    }
    if (hasExistingPageNumber(file)) {
      file.removeExistingPageNumber = true
      file.existingPageNumberEdited = false
    }
    const status = fileExistingStatus(file)
    file.statusText = status.text
    file.statusType = status.type
  })
  truePreview.value = null
  refreshPreview()
}

function restoreExistingHeaderFooterMarks() {
  overlayFiles.value.forEach((file) => {
    file.removeExistingHeader = false
    file.removeExistingFooter = false
    file.removeExistingPageNumber = false
    const status = fileExistingStatus(file)
    file.statusText = status.text
    file.statusType = status.type
  })
  truePreview.value = null
  refreshPreview()
}

function headerFooterHandlingText(row) {
  const parts = []
  if (row?.removeExistingHeader) parts.push('页眉删除')
  else if (row?.existingHeaderEdited) parts.push('页眉已编辑')
  else if (row?.existingHeaderArtifact) parts.push('页眉可编辑')
  else if (row?.existingHeaderText && row?.convertPlainHeader) parts.push('页眉转换')
  else if (row?.existingHeaderText) parts.push('页眉可转换')
  else parts.push('页眉新增')
  if (row?.removeExistingFooter) parts.push('页脚删除')
  else if (row?.existingFooterEdited) parts.push('页脚已编辑')
  else if (row?.existingFooterArtifact) parts.push('页脚可编辑')
  else if (row?.existingFooterText && row?.convertPlainFooter) parts.push('页脚转换')
  else if (row?.existingFooterText) parts.push('页脚可转换')
  else parts.push('页脚新增')
  if (row?.removeExistingPageNumber) parts.push('页码删除')
  else if (row?.existingPageNumberEdited) parts.push('页码已编辑')
  else if (row?.existingPageNumberText && row?.convertPlainPageNumber) parts.push('页码转换')
  else if (row?.existingPageNumberText) parts.push('页码可转换')
  return parts.join(' / ')
}

function sourceRangeText(row) {
  if (!row.sourcePageStart || !row.sourcePageEnd) return '-'
  return `${row.sourcePageStart}-${row.sourcePageEnd}`
}

function sortOverlayFiles({ prop, order }) {
  if (!prop || !order) return
  const selectedPath = selectedOverlayFile.value?.path
  overlayFiles.value = sortByNatural(overlayFiles.value, (row, index) => overlaySortValue(row, prop, index), order)
  if (selectedPath) {
    selectedOverlayIndex.value = Math.max(
      0,
      overlayFiles.value.findIndex((file) => file.path === selectedPath),
    )
  } else {
    selectedOverlayIndex.value = 0
  }
  refreshPreview()
}

function overlaySortValue(row, prop, index) {
  if (prop === 'existingHeader') return displayExistingHeader(row)
  if (prop === 'existingFooter') return displayExistingFooter(row)
  if (prop === 'existingPageNumber') return displayExistingPageNumber(row)
  if (prop === 'header') return displayRowHeader(row, index)
  if (prop === 'footer') return displayRowFooter(row, index)
  if (prop === 'pages') return Number(row?.pages || 0)
  if (prop === 'pageRange') return Number(row?.pageStart || 0)
  if (prop === 'existingHandling') return headerFooterHandlingText(row)
  if (prop === 'sourceRange') return Number(row?.sourcePageStart || 0)
  if (prop === 'status') return row?.statusText || ''
  return row?.[prop] ?? ''
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

.preview-head {
  flex-direction: column;
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

.block-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.block-title-row .block-title {
  margin-bottom: 0;
}

.block-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.existing-summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}

.summary-pill {
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  background: #fff;
}

.summary-pill span {
  display: block;
  color: #909399;
  font-size: 12px;
  line-height: 1.2;
}

.summary-pill strong {
  display: block;
  margin-top: 4px;
  color: #303133;
  font-size: 16px;
  line-height: 1;
}

.summary-pill.active {
  border-color: #f3d19e;
  background: #fdf6ec;
}

.summary-pill.warning.active strong {
  color: #b42318;
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

.split-name-options {
  margin-bottom: 10px;
}

.split-cleanup-options {
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

.split-cleanup-note {
  color: #909399;
  font-size: 12px;
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

.file-link {
  appearance: none;
  border: 0;
  padding: 0;
  color: #2563eb;
  background: transparent;
  cursor: pointer;
  font: inherit;
  text-align: left;
}

.file-link:hover {
  color: #1d4ed8;
  text-decoration: underline;
}

.dialog-rule-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.dialog-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-right: 28px;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.table-text {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  vertical-align: middle;
  white-space: nowrap;
}

.editable-text {
  cursor: text;
}

.deleted-existing-text {
  padding: 1px 5px;
  border: 1px dashed #d93025;
  border-radius: 3px;
  background: #fff7f7;
  color: #b42318;
  text-decoration: line-through;
}

.overlay-table {
  margin-top: 10px;
}

.preview-controls {
  display: flex;
  gap: 8px;
  align-items: center;
}

.footer-candidate-panel {
  margin-bottom: 10px;
  padding: 10px;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  background: #fff;
}

.candidate-panel-head {
  margin-bottom: 8px;
}

.footer-candidate-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.footer-candidate-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto auto auto;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
  border: 1px solid #ebeef5;
  border-radius: 6px;
  background: #fafafa;
}

.footer-candidate-item.active {
  border-color: #409eff;
  background: #ecf5ff;
}

.candidate-main {
  appearance: none;
  min-width: 0;
  border: 0;
  padding: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.candidate-main strong {
  display: block;
  overflow: hidden;
  color: #303133;
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.candidate-main span {
  display: block;
  margin-top: 2px;
  color: #909399;
  font-size: 12px;
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
  align-items: flex-start;
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
  position: relative;
  flex: 0 0 auto;
  width: min(100%, 620px);
  background: #fff;
  box-shadow: 0 2px 14px rgba(0, 0, 0, 0.16);
  overflow: hidden;
}

.page-preview img,
.true-preview-page img {
  display: block;
  width: 100%;
  height: auto;
  object-fit: contain;
}

.preview-text {
  position: absolute;
  z-index: 2;
  color: #111827;
  white-space: nowrap;
  max-width: 90%;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: Arial, 'PingFang SC', 'Microsoft YaHei', sans-serif;
}

.preview-text.with-delete-background {
  padding: 2px 5px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.94);
}

.delete-preview-marker {
  position: absolute;
  z-index: 1;
  box-sizing: border-box;
  min-width: 56px;
  min-height: 16px;
  padding: 1px 4px;
  border: 1px dashed #d93025;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.9);
  color: #b42318;
  font-size: 11px;
  line-height: 1.2;
  text-decoration: line-through;
  pointer-events: none;
}

.delete-preview-marker::after {
  content: '';
  position: absolute;
  left: 4px;
  right: 4px;
  top: 50%;
  border-top: 2px solid rgba(217, 48, 37, 0.72);
  transform: translateY(-50%);
}

.delete-preview-marker span {
  position: relative;
  z-index: 1;
}

.footer-candidate-marker {
  position: absolute;
  z-index: 3;
  box-sizing: border-box;
  min-width: 56px;
  min-height: 16px;
  padding: 1px 4px;
  border: 2px solid #2563eb;
  border-radius: 2px;
  background: rgba(219, 234, 254, 0.42);
  color: #1d4ed8;
  font-size: 11px;
  line-height: 1.2;
  pointer-events: none;
}

.footer-candidate-marker span {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

  .existing-summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>

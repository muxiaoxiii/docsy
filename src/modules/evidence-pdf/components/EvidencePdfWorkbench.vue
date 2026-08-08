<template>
  <div class="hf-workbench" :class="{ 'is-evidence-dragging': evidenceDragging }">
    <div v-if="evidenceDragging" class="evidence-drop-overlay">
      <div class="evidence-drop-message">松开以导入 PDF 文件</div>
    </div>
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

      <div v-if="importingMergedPdf" v-loading="true" element-loading-text="正在分析合并 PDF" class="local-processing">
        <p>大文件可能需要一段时间，当前只占用这个任务区域；其他标签和窗口仍可继续操作。</p>
      </div>
      <div v-if="splittingMergedImport" v-loading="true" element-loading-text="正在拆分 PDF" class="local-processing">
        <p>大文件会按页段逐个输出，当前只占用合并证据处理区域；请先不要重复点击确认拆分。</p>
      </div>
      <div v-if="overlaying" v-loading="true" element-loading-text="正在处理证据 PDF" class="local-processing">
        <p>页眉页脚、A4、批注和合并会在后台执行；文件较大时请等待当前批次完成。</p>
      </div>
      <div v-if="detectingAllHeaderFooter" v-loading="true" :element-loading-text="detectionProgressText || '正在检测导入的文件...'" class="local-processing">
        <p>正在读取页眉页脚信息，完成后统一显示结果。</p>
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
            <el-button size="small" @click="confirmAllExistingElements"
              >一键确认</el-button
            >
            <el-button
              size="small"
              type="warning"
              :loading="quickCleanupRunning"
              @click="quickCleanupExistingHeaderFooter"
            >
              一键清除页眉页脚
            </el-button>
            <el-button size="small" :disabled="!hasDetectedExistingHeaderFooter" @click="markRemoveExistingHeaderFooter"
              >删除现有</el-button
            >
            <el-button size="small" :disabled="!hasExistingRemovalRule" @click="restoreExistingHeaderFooterMarks"
              >恢复删除标记</el-button
            >
          </div>
        </div>
        <div class="existing-summary-grid">
          <button v-if="existingHeaderCount" type="button" class="summary-pill" @click="openExistingElements('header')">
            <span>原页眉</span>
            <strong>{{ existingHeaderCount }}</strong>
          </button>
          <button
            v-if="existingFooterCount"
            type="button"
            class="summary-pill"
            @click="openExistingElements('footerText')"
          >
            <span>原页脚文字</span>
            <strong>{{ existingFooterCount }}</strong>
          </button>
          <button
            v-if="existingPageNumberCount"
            type="button"
            class="summary-pill"
            @click="openExistingElements('pageNumber')"
          >
            <span>原页码</span>
            <strong>{{ existingPageNumberCount }}</strong>
          </button>
          <button
            v-if="existingRemovalCount"
            type="button"
            class="summary-pill warning active"
            @click="openExistingElements('delete')"
          >
            <span>待删除</span>
            <strong>{{ existingRemovalCount }}</strong>
          </button>
          <button
            v-if="existingEditCount"
            type="button"
            class="summary-pill active"
            @click="openExistingElements('edit')"
          >
            <span>待编辑</span>
            <strong>{{ existingEditCount }}</strong>
          </button>
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
              <el-option label="保持原页方向" value="preserve" />
              <el-option label="纵向" value="portrait" />
              <el-option label="横向" value="landscape" />
            </el-select>
            <span class="field-hint">保持不判断内容方向；横向或纵向会先旋转整页</span>
          </div>
          <div class="rule-item">
            <label>删除批注对象</label>
            <el-switch v-model="removeAnnotations" active-text="启用" inactive-text="关闭" />
          </div>
        </div>
      </div>

      <div v-if="showProcessingControls" class="rule-block">
        <div class="block-title-row">
          <div class="block-title">插入新页眉、页脚文字和页码</div>
          <el-checkbox v-model="globalApplyEnabled" size="small">全局应用</el-checkbox>
          <el-switch v-model="insertHeaderFooterEnabled" active-text="插入" inactive-text="不插入" />
          </div>
          <HeaderFooterRuleFields
            v-if="insertHeaderFooterEnabled"
            ref="inlineHfFieldsRef"
            class="rule-grid" 
          v-model:header-groups="headerGroupsModel"
          v-model:selected-header-group-id="selectedHeaderGroupId"
          v-model:header-mode="headerMode"
          v-model:header-insert-enabled="headerInsertEnabled"
          v-model:header-text="headerText"
          v-model:header-prefix="headerPrefix"
          v-model:header-suffix="headerSuffix"
          v-model:header-align="headerAlign"
          v-model:header-font-size="headerFontSize"
          v-model:header-font-family="headerFontFamily"
          v-model:header-margin-mm="headerMarginMm"
          v-model:header-offset-x-mm="headerOffsetXMm"
          v-model:header-color="headerColor"
          v-model:footer-text-groups="footerTextGroupsModel"
          v-model:selected-footer-text-group-id="selectedFooterTextGroupId"
          v-model:footer-insert-enabled="footerInsertEnabled"
          v-model:footer-text-content="footerTextContent"
          v-model:footer-text-align="footerTextAlign"
          v-model:footer-text-font-size="footerTextFontSize"
          v-model:footer-text-font-family="footerTextFontFamily"
          v-model:footer-text-margin-mm="footerTextMarginMm"
          v-model:footer-text-offset-x-mm="footerTextOffsetXMm"
          v-model:footer-text-color="footerTextColor"
          v-model:page-number-groups="pageNumberGroupsModel"
          v-model:selected-page-number-group-id="selectedPageNumberGroupId"
          v-model:page-number-enabled="footerEnabled"
          v-model:page-number-sequence="pageNumberSequence"
          v-model:page-number-style="pageNumberStyle"
          v-model:page-number-template="pageNumberTemplate"
          v-model:page-number-region="pageNumberRegion"
          v-model:page-number-align="footerAlign"
          v-model:page-number-font-size="footerFontSize"
          v-model:page-number-font-family="footerFontFamily"
          v-model:page-number-margin-mm="footerMarginMm"
          v-model:page-number-offset-x-mm="footerOffsetXMm"
          v-model:page-number-color="footerColor"
          :page-number-override-count="pageNumberOverrides.length"
          v-model:page-number-show-total="pageNumberShowTotal"
          :page-number-sample-page="previewSamplePage"
          :page-number-sample-total="totalOverlayPages"
          :page-height-mm="previewHeightMm"
          @edit-page-number-rules="pageNumberRulesVisible = true"
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
          <div class="rule-item merge-row">
            <label>合并文件名</label>
            <el-input v-model="mergeFileName" :disabled="outputMode === 'files_only'" />
          </div>
          <div v-if="outputMode === 'files_only'" class="rule-item">
            <el-checkbox v-model="fileSuffixEnabled">添加后缀</el-checkbox>
            <el-input
              v-model="fileSuffixText"
              :disabled="!fileSuffixEnabled"
              placeholder="后缀文字"
              style="width: 140px"
            />
          </div>
        </div>
      </div>

      <div v-if="showProcessingControls && outputMode !== 'files_only'" class="rule-block">
        <div class="block-title">PDF 书签</div>
        <div class="rule-grid bookmark-rule-grid">
          <div class="rule-item">
            <el-checkbox v-model="bookmarkEnabled">添加书签</el-checkbox>
          </div>
          <div class="rule-item">
            <el-checkbox v-model="bookmarkRemoveExisting">删除已有书签</el-checkbox>
          </div>
          <div v-if="bookmarkEnabled" class="rule-item">
            <label>书签文字</label>
            <el-radio-group v-model="bookmarkLabelSource">
              <el-radio value="header">用页眉文字</el-radio>
              <el-radio value="filename">用文件名</el-radio>
            </el-radio-group>
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
          <el-table-column width="42" align="center">
            <template #default="{ $index }">
              <button
                type="button"
                class="table-drag-handle"
                :data-merged-reorder-index="$index"
                title="拖动调整页段顺序"
                @pointerdown.stop="startMergedReorder($index, $event)"
                @pointermove.stop="moveMergedReorder"
                @pointerup.stop="finishMergedReorder"
                @pointercancel.stop="resetMergedReorder"
              >
                <el-icon><Rank /></el-icon>
              </button>
            </template>
          </el-table-column>
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

      <el-alert
        v-if="existingBookmarkCount > 0 && existingBookmarkAlertVisible"
        title="检测到已有书签"
        type="warning"
        show-icon
        closable
        @close="existingBookmarkAlertVisible = false"
        class="bookmark-alert"
      >
        <template #default>
          <span>{{ existingBookmarkCount }} 个文件包含已有书签，新书签会叠加在已有书签上。</span>
          <el-button size="small" type="warning" @click="clearExistingBookmarks" style="margin-left: 8px">
            清除已有书签
          </el-button>
        </template>
      </el-alert>

      <el-table
        v-if="overlayFiles.length"
        :data="overlayRows"
        :row-key="row => row.path"
        size="small"
        border
        ref="overlayTableRef"
        class="overlay-table"
        highlight-current-row

        @row-click="selectPreviewRow"
        @sort-change="sortOverlayFiles"
        @expand-change="handleExpandChange"
      >
        <el-table-column type="expand" width="36">
          <template #default="{ row, $index }">
            <div class="content-subrows">
              <template v-for="contentRows in [buildFileContentRows(row, $index, currentRules)]" :key="row.path">
                <el-table
                  :data="contentRows"
                  size="small"
                  border
                  class="content-subtable"
                  @click.stop
                >
                  <el-table-column label="类型" width="80">
                    <template #default="{ row: cr }">
                      <span class="content-kind-tag" :class="`source-${cr.source}`">{{ contentKindLabel(cr.kind) }}</span>
                    </template>
                  </el-table-column>
                  <el-table-column label="文本" min-width="200">
                    <template #default="{ row: cr }">
                      <el-input
                        v-if="editingContentRowId === `${row.path}|${cr.id}`"
                        v-model="editingContentRowValue"
                        size="small"
                        @click.stop
                        @blur="cr.source === 'existing' ? cancelContentRowEdit() : finishContentRowEdit(row, cr)"
                        @keyup.enter="finishContentRowEdit(row, cr)"
                      />
                      <el-tooltip
                        v-else
                        :content="displayContentRowText(row, $index, cr) || '-'"
                        placement="top"
                        :show-after="300"
                        :disabled="!displayContentRowText(row, $index, cr)"
                      >
                        <span
                          class="editable-text"
                          @dblclick.stop="startContentRowEdit(row, cr)"
                        >{{ displayContentRowText(row, $index, cr) || '-' }}</span>
                      </el-tooltip>
                    </template>
                  </el-table-column>
                  <el-table-column label="状态" width="80">
                    <template #default="{ row: cr }">
                      <el-tag size="small" :type="contentStatusTagType(cr.status)">
                        {{ contentStatusLabel(cr.status) }}
                      </el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column label="操作" width="70">
                    <template #default="{ row: cr }">
                      <el-button
                        v-if="cr.source === 'existing' && cr.status !== 'existing'"
                        link
                        size="small"
                        @click.stop="cancelExistingDecision(row, cr)"
                      >
                        <el-icon><RefreshLeft /></el-icon>
                      </el-button>
                      <el-button
                        v-if="cr.source === 'new'"
                        link
                        size="small"
                        type="danger"
                        @click.stop="removeContentRowNew(row, cr)"
                      >
                        <el-icon><Delete /></el-icon>
                      </el-button>
                      <el-button
                        v-if="cr.source === 'existing'"
                        link
                        size="small"
                        type="danger"
                        @click.stop="removeContentRowExisting(row, cr)"
                      >
                        <el-icon><Delete /></el-icon>
                      </el-button>
                    </template>
                  </el-table-column>
                </el-table>

              </template>
            </div>
          </template>
        </el-table-column>
        <el-table-column width="44" align="center" class-name="index-drag-col">
          <template #default="{ $index }">
            <span class="index-drag-cell">
              <span class="index-number">{{ $index + 1 }}</span>
              <button
                type="button"
                class="table-drag-handle index-drag-overlay"
                :data-reorder-index="$index"
                title="拖动调整处理顺序"
                @pointerdown.stop="startOverlayReorder($index, $event)"
                @pointermove.stop="moveOverlayReorder"
                @pointerup.stop="finishOverlayReorder"
                @pointercancel.stop="resetOverlayReorder"
              >
                <el-icon><Rank /></el-icon>
              </button>
            </span>
          </template>
        </el-table-column>
        <el-table-column label="文件" prop="name" sortable="custom" min-width="180" show-overflow-tooltip :tooltip-props="{ placement: 'right' }">
          <template #default="{ row, $index }">
            <button class="file-link" type="button" :data-reorder-index="$index" @click.stop="openEvidenceFile(row)">
              {{ row.name }}
            </button>
          </template>
        </el-table-column>
        <el-table-column label="页眉/页脚文本" min-width="200" show-overflow-tooltip :tooltip-props="{ placement: 'right' }">
          <template #default="{ row, $index }">
            <el-input
              v-if="editingContentRowId && editingContentRowId.startsWith(`${row.path}|`)"
              v-model="editingContentRowValue"
              size="small"
              @click.stop
              @blur="finishMainColumnContentEdit(row)"
              @keyup.enter="finishMainColumnContentEdit(row)"
            />
            <span v-else class="table-text editable-text" @dblclick.stop="startMainColumnContentEdit(row, $index)">
              {{ mainColumnContentText(row, $index) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="页数" prop="pages" sortable="custom" width="70">
          <template #default="{ row }">{{ row.pages || '-' }}</template>
        </el-table-column>
        <el-table-column label="状态" prop="statusText" sortable="custom" width="100">
          <template #default="{ row }">
            <el-tooltip
              v-if="row.statusDetail"
              :content="row.statusDetail"
              placement="right"
              :show-after="300"
            >
              <el-tag :type="row.statusType || 'info'" size="small">
                {{ row.statusText || '待处理' }}
              </el-tag>
            </el-tooltip>
            <el-tag v-else :type="row.statusType || 'info'" size="small">
              {{ row.statusText || '待处理' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="页码范围" prop="pageRange" sortable="custom" width="105">
          <template #default="{ row }">{{ pageRangeText(row, pageNumberSequence) }}</template>
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
        <el-table-column label="操作" width="70" fixed="right">
          <template #default="{ $index }">
            <el-button link size="small" :disabled="$index === 0" @click.stop="moveOverlayFile($index, -1)">
              <el-icon><Top /></el-icon>
            </el-button>
            <el-button
              link
              size="small"
              :disabled="$index === overlayRows.length - 1"
              @click.stop="moveOverlayFile($index, 1)"
            >
              <el-icon><Bottom /></el-icon>
            </el-button>
            <el-button link type="danger" size="small" @click.stop="removeOverlayFile($index)">
              <el-icon><Delete /></el-icon>
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <el-dialog v-model="headerFooterSettingsVisible" width="760px" destroy-on-close>
      <template #header>
        <div class="dialog-title-row">
          <span>页眉页脚格式</span>
          <el-checkbox v-model="globalApplyEnabled" size="small">全局应用</el-checkbox>
          <el-switch v-model="insertHeaderFooterEnabled" active-text="插入" inactive-text="不插入" />
        </div>
      </template>
      <HeaderFooterRuleFields
        v-if="insertHeaderFooterEnabled"
        class="dialog-rule-grid"
        v-model:header-groups="headerGroupsModel"
        v-model:selected-header-group-id="selectedHeaderGroupId"
        v-model:header-mode="headerMode"
        v-model:header-insert-enabled="headerInsertEnabled"
        v-model:header-text="headerText"
        v-model:header-prefix="headerPrefix"
        v-model:header-suffix="headerSuffix"
        v-model:header-align="headerAlign"
        v-model:header-font-size="headerFontSize"
        v-model:header-font-family="headerFontFamily"
        v-model:header-margin-mm="headerMarginMm"
        v-model:header-offset-x-mm="headerOffsetXMm"
        v-model:header-color="headerColor"
        v-model:footer-text-groups="footerTextGroupsModel"
        v-model:selected-footer-text-group-id="selectedFooterTextGroupId"
        v-model:footer-insert-enabled="footerInsertEnabled"
        v-model:footer-text-content="footerTextContent"
        v-model:footer-text-align="footerTextAlign"
        v-model:footer-text-font-size="footerTextFontSize"
        v-model:footer-text-font-family="footerTextFontFamily"
        v-model:footer-text-margin-mm="footerTextMarginMm"
        v-model:footer-text-offset-x-mm="footerTextOffsetXMm"
        v-model:footer-text-color="footerTextColor"
        v-model:page-number-groups="pageNumberGroupsModel"
        v-model:selected-page-number-group-id="selectedPageNumberGroupId"
        v-model:page-number-enabled="footerEnabled"
        v-model:page-number-sequence="pageNumberSequence"
        v-model:page-number-style="pageNumberStyle"
        v-model:page-number-template="pageNumberTemplate"
        v-model:page-number-region="pageNumberRegion"
        v-model:page-number-align="footerAlign"
        v-model:page-number-font-size="footerFontSize"
        v-model:page-number-font-family="footerFontFamily"
        v-model:page-number-margin-mm="footerMarginMm"
        v-model:page-number-offset-x-mm="footerOffsetXMm"
        v-model:page-number-color="footerColor"
        :page-number-override-count="pageNumberOverrides.length"
        v-model:page-number-show-total="pageNumberShowTotal"
        :page-number-sample-page="previewSamplePage"
        :page-number-sample-total="totalOverlayPages"
        :page-height-mm="previewHeightMm"
        @edit-page-number-rules="pageNumberRulesVisible = true"
        :offset-limit-mm="HORIZONTAL_OFFSET_LIMIT_MM"
      />
      <template #footer>
        <el-button @click="headerFooterSettingsVisible = false">关闭</el-button>
        <el-button type="primary" @click="applyHeaderFooterSettings">应用到预览</el-button>
      </template>
    </el-dialog>
    <PageNumberRuleDialog v-model:visible="pageNumberRulesVisible" v-model:rules="pageNumberOverrides" />
    <ExistingPdfElementsDialog
      v-model:visible="existingElementsVisible"
      :rows="existingElementRows"
      :filter="existingElementsFilter"
      @change="handleExistingElementChange"
      @preview="previewExistingElement"
      @jump-to-settings="handleJumpToSettings"
    />

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
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus'
import { Delete, Bottom, Plus, Rank, RefreshLeft, Top } from '@element-plus/icons-vue'
import { exists } from '@tauri-apps/plugin-fs'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../../pdf-tools/components/PdfJsPreview.vue'
import HeaderFooterRuleFields from '../../pdf-tools/components/HeaderFooterRuleFields.vue'
import PageNumberRuleDialog from '../../pdf-tools/components/PageNumberRuleDialog.vue'
import ExistingPdfElementsDialog from '../../pdf-tools/components/ExistingPdfElementsDialog.vue'
import EvidenceMergedImportPlan from './EvidenceMergedImportPlan.vue'
import EvidenceOverlayTable from './EvidenceOverlayTable.vue'
import {
  buildEvidencePdfRulePayload,
  buildFileContentRows,
  buildMergeOutputPath,
  buildOutputDir,
  createEvidenceFile,
  fileName,
  groupsFor,
  pageRangeText,
  parentDir,
  selectedGroupFor,
  setSelectedGroup,
  sortByNatural,
  totalPages,
  updatePageRanges,
} from '../../pdf-tools/composables/useEvidencePdfSession.js'
import { todayCompact } from '../../pdf-tools/composables/splitFileName.js'
import { useEvidencePdfDetection } from '../../pdf-tools/composables/useEvidencePdfDetection.js'
import { useEvidencePdfPreview } from '../../pdf-tools/composables/useEvidencePdfPreview.js'
import { useEvidencePdfMergedImport } from '../../pdf-tools/composables/useEvidencePdfMergedImport.js'
import { useEvidencePdfExistingEditing } from '../../pdf-tools/composables/useEvidencePdfExistingEditing.js'
import { renderPageNumberTemplate } from '../../pdf-tools/composables/pdfPageNumberRules.js'
import { elementIdentity } from '../../pdf-tools/composables/existingPdfElements.js'
import { usePointerReorder } from '../../../core/composables/usePointerReorder.js'
import { useHistory } from '../../../core/composables/useHistory.js'
import { openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { useHeaderFooterRules } from '../composables/useHeaderFooterRules.js'
import { useContentRowEditing } from '../composables/useContentRowEditing.js'
import { useFileOrdering } from '../composables/useFileOrdering.js'

/** Parse a page number value from detected text (e.g. "1/10 页" → 1, "第3页" → 3). */
function parsePageNumberValue(text) {
  if (!text) return null
  const trimmed = String(text).trim()
  // Fraction: "1/10 页" or "1/10"
  const slashMatch = trimmed.match(/^(\d+)\s*[/／∕]\s*\d+/)
  if (slashMatch) return parseInt(slashMatch[1], 10)
  // Chinese: "第3页"
  const cnMatch = trimmed.match(/第\s*(\d+)\s*页/)
  if (cnMatch) return parseInt(cnMatch[1], 10)
  // English: "Page 5 of 20"
  const enMatch = trimmed.match(/(?:page|p)\s*(\d+)\s*(?:of|\/)/i)
  if (enMatch) return parseInt(enMatch[1], 10)
  // Standalone number (1-4 digits)
  const numMatch = trimmed.match(/^(\d{1,4})$/)
  if (numMatch) return parseInt(numMatch[1], 10)
  // Roman numerals
  const upper = trimmed.toUpperCase()
  if (/^[IVXLCDM]+$/.test(upper) && upper.length <= 8) {
    const vals = { I: 1, V: 5, X: 10, L: 50, C: 100, D: 500, M: 1000 }
    let result = 0
    for (let i = 0; i < upper.length; i++) {
      const cur = vals[upper[i]] || 0
      const nxt = vals[upper[i + 1]] || 0
      result += nxt > cur ? -cur : cur
    }
    if (result > 0) return result
  }
  return null
}

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
const quickCleanupRunning = ref(false)
const evidenceDragging = ref(false)
let quickCleanupPipeline = false
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

const existingBookmarkCount = ref(0)
const existingBookmarkAlertVisible = ref(true)

// Per-file overlay rows (must be declared before group computeds that reference selectedOverlayFile)
const overlayRows = computed(() => {
  return updatePageRanges(overlayFiles.value)
})
const selectedOverlayFile = computed(() => overlayRows.value[selectedOverlayIndex.value] || null)

const headerGroups = computed(() =>
  globalApplyEnabled.value ? [globalHeaderGroup.value] : groupsFor(selectedOverlayFile.value, 'header'),
)
const footerTextGroups = computed(() =>
  globalApplyEnabled.value ? [globalFooterTextGroup.value] : groupsFor(selectedOverlayFile.value, 'footerText'),
)
const pageNumberGroups = computed(() =>
  globalApplyEnabled.value ? [globalPageNumberGroup.value] : groupsFor(selectedOverlayFile.value, 'pageNumber'),
)
// Writable models so HeaderFooterRuleFields can add/remove groups per file
const headerGroupsModel = computed({
  get: () => globalApplyEnabled.value
    ? [globalHeaderGroup.value]
    : selectedOverlayFile.value?.headerGroups || [],
  set: (v) => {
    if (globalApplyEnabled.value) {
      globalHeaderGroup.value = v[0] || createDefaultHeaderGroup()
    } else if (selectedOverlayFile.value) {
      selectedOverlayFile.value.headerGroups = v
    }
  },
})
const footerTextGroupsModel = computed({
  get: () => globalApplyEnabled.value
    ? [globalFooterTextGroup.value]
    : selectedOverlayFile.value?.footerTextGroups || [],
  set: (v) => {
    if (globalApplyEnabled.value) {
      globalFooterTextGroup.value = v[0] || createDefaultFooterTextGroup()
    } else if (selectedOverlayFile.value) {
      selectedOverlayFile.value.footerTextGroups = v
    }
  },
})
const pageNumberGroupsModel = computed({
  get: () => globalApplyEnabled.value
    ? [globalPageNumberGroup.value]
    : selectedOverlayFile.value?.pageNumberGroups || [],
  set: (v) => {
    if (globalApplyEnabled.value) {
      globalPageNumberGroup.value = v[0] || createDefaultPageNumberGroup()
    } else if (selectedOverlayFile.value) {
      selectedOverlayFile.value.pageNumberGroups = v
    }
  },
})
const pageNumberTemplate = computed({
  get: () => selectedPageNumberGroup.value.template || '{page}/{total}',
  set: (v) => { selectedPageNumberGroup.value.template = v },
})
const selectedHeaderGroup = computed(() =>
  globalApplyEnabled.value
    ? globalHeaderGroup.value
    : selectedGroupFor(selectedOverlayFile.value, 'header') || createDefaultHeaderGroup(),
)
const selectedFooterTextGroup = computed(() =>
  globalApplyEnabled.value
    ? globalFooterTextGroup.value
    : selectedGroupFor(selectedOverlayFile.value, 'footerText') || createDefaultFooterTextGroup(),
)
const selectedPageNumberGroup = computed(() =>
  globalApplyEnabled.value
    ? globalPageNumberGroup.value
    : selectedGroupFor(selectedOverlayFile.value, 'pageNumber') || createDefaultPageNumberGroup(),
)
// Per-file selected group ids (bind to HeaderFooterRuleFields v-model)
const selectedHeaderGroupId = computed({
  get: () => globalApplyEnabled.value
    ? globalHeaderGroup.value.id
    : selectedOverlayFile.value?.selectedHeaderGroupId || 'h1',
  set: (v) => {
    if (!globalApplyEnabled.value) setSelectedGroup(selectedOverlayFile.value, 'header', v)
  },
})
const selectedFooterTextGroupId = computed({
  get: () => globalApplyEnabled.value
    ? globalFooterTextGroup.value.id
    : selectedOverlayFile.value?.selectedFooterTextGroupId || 'ft1',
  set: (v) => {
    if (!globalApplyEnabled.value) setSelectedGroup(selectedOverlayFile.value, 'footerText', v)
  },
})
const selectedPageNumberGroupId = computed({
  get: () => globalApplyEnabled.value
    ? globalPageNumberGroup.value.id
    : selectedOverlayFile.value?.selectedPageNumberGroupId || 'pn1',
  set: (v) => {
    if (!globalApplyEnabled.value) setSelectedGroup(selectedOverlayFile.value, 'pageNumber', v)
  },
})
// Legacy compat refs pointing at selected file/group. Setters update the file's group.
const headerMode = computed({
  get: () => selectedHeaderGroup.value.mode,
  set: (v) => { selectedHeaderGroup.value.mode = v },
})
const headerText = computed({
  get: () => selectedHeaderGroup.value.text,
  set: (v) => { selectedHeaderGroup.value.text = v },
})
const headerPrefix = computed({
  get: () => selectedHeaderGroup.value.prefix,
  set: (v) => { selectedHeaderGroup.value.prefix = v },
})
const headerSuffix = computed({
  get: () => selectedHeaderGroup.value.suffix,
  set: (v) => { selectedHeaderGroup.value.suffix = v },
})
const headerAlign = computed({
  get: () => selectedHeaderGroup.value.align,
  set: (v) => { selectedHeaderGroup.value.align = v },
})
const headerFontSize = computed({
  get: () => selectedHeaderGroup.value.fontSize,
  set: (v) => { selectedHeaderGroup.value.fontSize = v },
})
const headerFontFamily = computed({
  get: () => selectedHeaderGroup.value.fontFamily,
  set: (v) => { selectedHeaderGroup.value.fontFamily = v },
})
const headerMarginMm = computed({
  get: () => selectedHeaderGroup.value.marginMm,
  set: (v) => { selectedHeaderGroup.value.marginMm = v },
})
const headerOffsetXMm = computed({
  get: () => selectedHeaderGroup.value.offsetXMm,
  set: (v) => { selectedHeaderGroup.value.offsetXMm = v },
})
const headerColor = computed({
  get: () => selectedHeaderGroup.value.color,
  set: (v) => { selectedHeaderGroup.value.color = v },
})
const footerTextContent = computed({
  get: () => selectedFooterTextGroup.value.text,
  set: (v) => { selectedFooterTextGroup.value.text = v },
})
const footerTextAlign = computed({
  get: () => selectedFooterTextGroup.value.align,
  set: (v) => { selectedFooterTextGroup.value.align = v },
})
const footerTextFontSize = computed({
  get: () => selectedFooterTextGroup.value.fontSize,
  set: (v) => { selectedFooterTextGroup.value.fontSize = v },
})
const footerTextFontFamily = computed({
  get: () => selectedFooterTextGroup.value.fontFamily,
  set: (v) => { selectedFooterTextGroup.value.fontFamily = v },
})
const footerTextMarginMm = computed({
  get: () => selectedFooterTextGroup.value.marginMm,
  set: (v) => { selectedFooterTextGroup.value.marginMm = v },
})
const footerTextOffsetXMm = computed({
  get: () => selectedFooterTextGroup.value.offsetXMm,
  set: (v) => { selectedFooterTextGroup.value.offsetXMm = v },
})
const footerTextColor = computed({
  get: () => selectedFooterTextGroup.value.color,
  set: (v) => { selectedFooterTextGroup.value.color = v },
})
const pageNumberSequence = computed({
  get: () => selectedPageNumberGroup.value.sequence,
  set: (v) => { selectedPageNumberGroup.value.sequence = v },
})
const pageNumberStyle = computed({
  get: () => selectedPageNumberGroup.value.style,
  set: (v) => { selectedPageNumberGroup.value.style = v },
})
const pageNumberRegion = computed({
  get: () => selectedPageNumberGroup.value.region,
  set: (v) => { selectedPageNumberGroup.value.region = v },
})
const footerEnabled = ref(false)
const footerText = ref('{page}/{total}')
const footerContinuous = ref(true)
// pageNumberSequence, pageNumberStyle, pageNumberRegion are computed from selectedPageNumberGroup above
const pageNumberOverrides = ref([])
const pageNumberRulesVisible = ref(false)
// footerAlign..footerColor are page number placement, backed by selectedPageNumberGroup
const footerAlign = computed({
  get: () => selectedPageNumberGroup.value.align,
  set: (v) => { selectedPageNumberGroup.value.align = v },
})
const footerFontSize = computed({
  get: () => selectedPageNumberGroup.value.fontSize,
  set: (v) => { selectedPageNumberGroup.value.fontSize = v },
})
const footerFontFamily = computed({
  get: () => selectedPageNumberGroup.value.fontFamily,
  set: (v) => { selectedPageNumberGroup.value.fontFamily = v },
})
const footerMarginMm = computed({
  get: () => selectedPageNumberGroup.value.marginMm,
  set: (v) => { selectedPageNumberGroup.value.marginMm = v },
})
const footerOffsetXMm = computed({
  get: () => selectedPageNumberGroup.value.offsetXMm,
  set: (v) => { selectedPageNumberGroup.value.offsetXMm = v },
})
const footerColor = computed({
  get: () => selectedPageNumberGroup.value.color,
  set: (v) => { selectedPageNumberGroup.value.color = v },
})
// footerTextContent..footerTextColor are computed from selectedFooterTextGroup above
const outputMode = ref('files_and_merge')
const mergeFileName = ref('merged_evidence.pdf')
const fileSuffixEnabled = ref(true)
const fileSuffixText = ref('processed')
const previewPage = ref(1)
const previewReloadKey = ref(0)
const previewData = ref({})
const previewHeightMm = computed(() => {
  const heightPt = Number(previewData.value?.heightPt || 0)
  return heightPt > 0 ? +((heightPt * 25.4) / 72).toFixed(1) : 297
})
const truePreview = ref(null)
const truePreviewLoading = ref(false)
const detectingAllHeaderFooter = ref(false)
const detectionProgressText = ref('')
const editingHeaderPath = ref('')
const editingFooterPath = ref('')
const editingExistingHeaderPath = ref('')
const editingExistingFooterPath = ref('')
const editingExistingPageNumberPath = ref('')
const editingContentRowId = ref('')
const editingContentRowKind = ref('')
const editingContentRowValue = ref('')
const selectedFooterCandidateKey = ref('')
const existingElementsVisible = ref(false)
const existingElementsFilter = ref('all')

// --- Text edit undo/redo ---
const editUndoStack = ref([])
const editRedoStack = ref([])
const inlineHfFieldsRef = ref(null)

function pushEditUndo(snap) {
  editUndoStack.value.push(snap)
  if (editUndoStack.value.length > 30) editUndoStack.value.shift()
  editRedoStack.value = []
}

function undoEdit() {
  const snap = editUndoStack.value.pop()
  if (!snap) return false
  editRedoStack.value.push(snap)
  const file = overlayFiles.value.find(f => f.path === snap.filePath)
  if (!file) return true
  if (snap.type === 'group') {
    const groups = groupsFor(file, snap.kind)
    const group = groups.find(g => g.id === snap.groupId)
    if (group) group.text = snap.oldValue
  } else if (snap.type === 'element') {
    const el = (file.existingElements || []).find(e => e.id === snap.elementId)
    if (el) {
      el.editedText = snap.oldValue
      el.decision = snap.oldDecision
      syncLegacyField(file, snap.kind, el)
    }
  }
  overlayFiles.value = [...overlayFiles.value]
  return true
}

function redoEdit() {
  const snap = editRedoStack.value.pop()
  if (!snap) return false
  editUndoStack.value.push(snap)
  const file = overlayFiles.value.find(f => f.path === snap.filePath)
  if (!file) return true
  if (snap.type === 'group') {
    const groups = groupsFor(file, snap.kind)
    const group = groups.find(g => g.id === snap.groupId)
    if (group) group.text = snap.newValue
  } else if (snap.type === 'element') {
    const el = (file.existingElements || []).find(e => e.id === snap.elementId)
    if (el) {
      el.editedText = snap.newValue
      el.decision = snap.newDecision
      syncLegacyField(file, snap.kind, el)
    }
  }
  overlayFiles.value = [...overlayFiles.value]
  return true
}

function syncLegacyField(file, kind, el) {
  if (kind === 'header') {
    file.existingHeaderText = el.editedText
    file.existingHeaderEdited = el.decision === 'edit'
    if (el.source !== 'artifact') file.convertPlainHeader = el.decision === 'edit'
  } else if (kind === 'footerText') {
    file.existingFooterText = el.editedText
    file.existingFooterEdited = el.decision === 'edit'
    if (el.source !== 'artifact') file.convertPlainFooter = el.decision === 'edit'
  } else {
    file.existingPageNumberText = el.editedText
    file.existingPageNumberEdited = el.decision === 'edit'
    if (el.source !== 'artifact') file.convertPlainPageNumber = el.decision === 'edit'
  }
}

watch(
  () => selectedHeaderGroup.value.mode,
  (mode) => {
    const group = selectedHeaderGroup.value
    if (mode === 'template') {
      group.mode = 'custom'
    } else if (mode === 'seq') {
      group.text = '证据[序号]'
      group.mode = 'custom'
    } else if (mode === 'seq_cn') {
      group.text = '证据[中文序号]'
      group.mode = 'custom'
    } else if (mode === 'prefix_seq') {
      group.text = `${group.text || ''}证据[序号]`
      group.mode = 'custom'
    }
  },
  { immediate: true },
)

watch(pageNumberSequence, (value) => {
  footerContinuous.value = value !== 'per-file'
})

watch(outputMode, (mode) => {
  if (mode === 'files_only') bookmarkEnabled.value = false
})

// Pipeline: when dialog closes during quick cleanup, proceed with processing
watch(existingElementsVisible, (visible) => {
  if (!visible && quickCleanupPipeline) {
    quickCleanupPipeline = false
    finishQuickCleanupPipeline()
  }
})

applyWorkflowDefaults()
const {
  start: startOverlayReorder,
  move: moveOverlayReorder,
  finish: finishOverlayReorder,
  reset: resetOverlayReorder,
} = usePointerReorder({
  itemCount: () => overlayFiles.value.length,
  onReorder: ({ from, to }) => reorderOverlayFiles(from, to),
})
const {
  start: startMergedReorder,
  move: moveMergedReorder,
  finish: finishMergedReorder,
  reset: resetMergedReorder,
} = usePointerReorder({
  itemCount: () => mergedImportPlan.value?.items?.length || 0,
  itemAttribute: 'data-merged-reorder-index',
  onReorder: ({ from, to }) => reorderMergedImportItems(from, to),
})
const hasSourceSplitRanges = computed(() => overlayFiles.value.some((file) => Number(file.sourcePageStart || 0) > 0))
const hasMergedBatchImports = computed(
  () => workflowMode.value === 'split' && overlayFiles.value.length > 0 && !hasSourceSplitRanges.value,
)

const activePreviewFilePath = computed(() => mergedImportPlan.value?.inputPath || selectedOverlayFile.value?.path || '')
const previewMaxPage = computed(() => {
  if (mergedImportPlan.value) return Math.max(1, Number(mergedImportPlan.value.totalPages || 1))
  return selectedOverlayFile.value?.pages || 1
})
const previewHint = computed(() => (mergedImportPlan.value ? '合并 PDF 原文预览' : '实时位置；真实预览需手动生成'))
const totalOverlayPages = computed(() => totalPages(overlayFiles.value))
const previewSamplePage = computed(() => previewPage.value)
const plannedOutputDir = computed(() => buildOutputDir(overlayRows.value, overlayOutputDir.value))
const plannedMergeOutputPath = computed(() =>
  buildMergeOutputPath(overlayRows.value, overlayOutputDir.value, mergeFileName.value),
)
const firstHeaderPreview = computed(() => {
  if (!insertHeaderFooterEnabled.value || !headerInsertEnabled.value) return ''
  const first = overlayRows.value[0]
  return first ? rowHeaderPreview(first, 0) : ''
})
const firstFooterPreview = computed(() => {
  if (!insertHeaderFooterEnabled.value) return ''
  const first = overlayRows.value[0]
  const pnGroup = globalApplyEnabled.value
    ? globalPageNumberGroup.value
    : selectedGroupFor(first, 'pageNumber')
  if (!first || !footerEnabled.value || !pnGroup || !totalOverlayPages.value) return ''
  const continuous = (pnGroup.sequence || 'continuous') !== 'per-file'
  let tpl = pnGroup.template || '{page}/{total}'
  if (!pageNumberShowTotal.value) tpl = tpl
    .replace(/共\s*\{total\}\s*页/g, '')
    .replace(/\bof\s*\{total\}/gi, '')
    .replaceAll('{total}', '')
    .replace(/[，,]\s*$/, '')
    .replace(/\s+$/, '')
    .replaceAll('//', '/')
    .replace(/\/+$/, '')
  return renderPageNumberTemplate(
    tpl,
    continuous ? first.pageStart || 1 : 1,
    continuous ? totalOverlayPages.value : first.pages || 1,
    pnGroup.style || 'arabic',
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
const existingElementRows = computed(() => {
  // Cross-file page number sequence validation for single-page files:
  // collect all page number values across files, then for each single-page
  // file's page-number candidate, check if it forms a continuous sequence
  // (±1) with another file's page number.  If not, mark as low confidence.
  const allPnValues = new Set()
  for (const file of overlayFiles.value) {
    for (const el of file.existingElements || []) {
      if (el.kind === 'pageNumber' && el.pageStart === el.pageEnd) {
        const value = parsePageNumberValue(el.detectedText)
        if (value != null) allPnValues.add(value)
      }
    }
  }
  return overlayFiles.value.flatMap((file) =>
    (file.existingElements || []).map((element) => {
      let lowConfidence = element.lowConfidence
      if (element.kind === 'pageNumber' && element.pageStart === element.pageEnd) {
        const value = parsePageNumberValue(element.detectedText)
        if (value != null) {
          const hasAdjacent = allPnValues.has(value - 1) || allPnValues.has(value + 1)
          lowConfidence = !hasAdjacent
        }
      }
      return {
        key: `${file.path}|${elementIdentity(element)}`,
        file,
        fileName: file.name,
        element,
        lowConfidence,
      }
    }),
  )
})
const existingHeaderCount = computed(
  () => existingElementRows.value.filter((row) => row.element.kind === 'header').length,
)
const existingFooterCount = computed(
  () => existingElementRows.value.filter((row) => row.element.kind === 'footerText').length,
)
const existingPageNumberCount = computed(
  () => existingElementRows.value.filter((row) => row.element.kind === 'pageNumber').length,
)
const hasExistingRemovalRule = computed(() =>
  existingElementRows.value.some((row) => row.element.decision === 'delete'),
)
const existingRemovalCount = computed(
  () => existingElementRows.value.filter((row) => row.element.decision === 'delete').length,
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
const existingEditCount = computed(
  () => existingElementRows.value.filter((row) => row.element.decision === 'edit').length,
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
    bookmarkEnabled.value ||
    bookmarkRemoveExisting.value ||
    hasExistingEditRule.value ||
    hasExistingConvertRule.value ||
    hasExistingRemovalRule.value ||
    (insertHeaderFooterEnabled.value &&
      ((headerInsertEnabled.value && headerMode.value !== 'none') || footerInsertEnabled.value || footerEnabled.value)),
)

const currentRules = computed(() => ({
  normalizeA4: normalizeA4.value,
  a4Orientation: a4Orientation.value,
  rasterDpi: rasterDpi.value,
  removeAnnotations: removeAnnotations.value,
  bookmarkEnabled: bookmarkEnabled.value,
  bookmarkRemoveExisting: bookmarkRemoveExisting.value,
  bookmarkLabelSource: bookmarkLabelSource.value,
  annotationKinds: annotationKinds.value,
  cleanupHeaderEnabled: autoCleanupHeaderEnabled.value,
  cleanupFooterEnabled: autoCleanupFooterEnabled.value,
  cleanupHeaderHeightMm: cleanupHeaderHeightMm.value,
  cleanupFooterHeightMm: cleanupFooterHeightMm.value,
  headerMode: insertHeaderFooterEnabled.value ? headerMode.value : 'none',
  headerInsertEnabled: insertHeaderFooterEnabled.value && headerInsertEnabled.value,
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
  headerGroups: insertHeaderFooterEnabled.value ? headerGroups.value : [],
  footerTextGroups: insertHeaderFooterEnabled.value && footerInsertEnabled.value ? footerTextGroups.value : [],
  pageNumberGroups: insertHeaderFooterEnabled.value && footerEnabled.value ? pageNumberGroups.value : [],
  // Global group: shared group instances used by ALL files when enabled
  _globalApply: globalApplyEnabled.value,
  _globalHeaderGroup: globalApplyEnabled.value ? globalHeaderGroup.value : null,
  _globalFooterTextGroup: globalApplyEnabled.value ? globalFooterTextGroup.value : null,
  _globalPageNumberGroup: globalApplyEnabled.value ? globalPageNumberGroup.value : null,
  footerEnabled: insertHeaderFooterEnabled.value && footerEnabled.value,
  footerText: footerText.value,
  footerContinuous: footerContinuous.value,
  footerAlign: footerAlign.value,
  footerFontSize: footerFontSize.value,
  footerFontFamily: footerFontFamily.value,
  footerMarginMm: footerMarginMm.value,
  footerOffsetXMm: footerOffsetXMm.value,
  footerColor: footerColor.value,
  footerInsertEnabled: insertHeaderFooterEnabled.value && footerInsertEnabled.value,
  footerTextContent: footerTextContent.value,
  footerTextAlign: footerTextAlign.value,
  footerTextFontSize: footerTextFontSize.value,
  footerTextFontFamily: footerTextFontFamily.value,
  footerTextMarginMm: footerTextMarginMm.value,
  footerTextOffsetXMm: footerTextOffsetXMm.value,
  footerTextColor: footerTextColor.value,
  pageNumberEnabled: insertHeaderFooterEnabled.value && footerEnabled.value,
  pageNumberSequence: pageNumberSequence.value,
  pageNumberStyle: pageNumberStyle.value,
  pageNumberTemplate: pageNumberTemplate.value,
  pageNumberRegion: pageNumberRegion.value,
  pageNumberAlign: footerAlign.value,
  pageNumberFontSize: footerFontSize.value,
  pageNumberFontFamily: footerFontFamily.value,
  pageNumberMarginMm: footerMarginMm.value,
  pageNumberOffsetXMm: footerOffsetXMm.value,
  pageNumberColor: footerColor.value,
  pageNumberOverrides: pageNumberOverrides.value,
  pageNumberShowTotal: pageNumberShowTotal.value,
  selectedHeaderGroupId: selectedHeaderGroupId.value,
  selectedFooterTextGroupId: selectedFooterTextGroupId.value,
  selectedPageNumberGroupId: selectedPageNumberGroupId.value,
  outputMode: outputMode.value,
  mergeAfterProcessing: outputMode.value !== 'files_only',
  mergeFileName: mergeFileName.value,
  fileSuffixEnabled: fileSuffixEnabled.value,
  fileSuffixText: fileSuffixText.value,
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
  detectionProgressText,
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
  headerInsertEnabled,
  headerMode,
  footerInsertEnabled,
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
  selectedFooterTextGroup,
  pageNumberStyle,
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

watch(selectedOverlayFile, (newFile, oldFile) => {
  previewPage.value = 1
  truePreview.value = null
  selectedFooterCandidateKey.value = newFile?.footerCandidateChoices?.[0]
    ? candidateKey(newFile.footerCandidateChoices[0])
    : ''
  // Parameter following: when switching to a new file, sync its group params
  // from the first file (overlayFiles[0]) if the new file still has defaults.
  if (newFile && oldFile && newFile !== oldFile) {
    const source = overlayFiles.value[0]
    if (source && source !== newFile) {
      syncGroupParamsFromSource(source, newFile)
    }
  }
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

// Parameter following: copy style params from source file's group to target,
// only if target still has default values (never been explicitly set).
const STYLE_KEYS = ['align', 'fontSize', 'fontFamily', 'marginMm', 'offsetXMm', 'color']
const DEFAULTS = {
  header: { align: 'center', fontSize: 12, fontFamily: 'auto', marginMm: 15, offsetXMm: 0, color: '#000000' },
  footerText: { align: 'left', fontSize: 9, fontFamily: 'auto', marginMm: 10, offsetXMm: 0, color: '#000000' },
  pageNumber: { align: 'center', fontSize: 9, fontFamily: 'auto', marginMm: 10, offsetXMm: 0, color: '#000000' },
}

function syncGroupParamsFromSource(source, target) {
  for (const kind of ['header', 'footerText', 'pageNumber']) {
    const srcGroup = selectedGroupFor(source, kind)
    const tgtGroup = selectedGroupFor(target, kind)
    if (!srcGroup || !tgtGroup) continue
    const defaults = DEFAULTS[kind]
    // Only sync if target group still has all default values
    const isDefault = STYLE_KEYS.every(k => tgtGroup[k] === defaults[k] || tgtGroup[k] == null)
    if (!isDefault) continue
    // Copy style params from source
    for (const k of STYLE_KEYS) {
      if (srcGroup[k] != null) tgtGroup[k] = srcGroup[k]
    }
  }
  // Trigger reactivity
  overlayFiles.value = [...overlayFiles.value]
}

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
  footerEnabled.value = false
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
  await loadEvidenceFiles(paths)
}

async function handleEvidenceDrop(paths) {
  const pdfPaths = paths.filter((p) => p.toLowerCase().endsWith('.pdf'))
  if (!pdfPaths.length) return
  await loadEvidenceFiles(pdfPaths)
}

async function loadEvidenceFiles(paths) {
  mergedImportPlan.value = null
  overlayFiles.value = paths.map(createEvidenceFile)
  selectedOverlayIndex.value = 0
  await refreshOverlayPageCounts()
  await detectAllHeaderFooter({ silent: true })
  await checkExistingBookmarks()
}

async function checkExistingBookmarks() {
  existingBookmarkCount.value = 0
  existingBookmarkAlertVisible.value = true
  const files = overlayFiles.value
  if (!files.length) return
  let count = 0
  for (const file of files) {
    try {
      const result = await tauriCallSafe('has_pdf_bookmarks', { input: file.path })
      if (result.ok && result.data) count++
    } catch {
      // 忽略单个文件检查失败
    }
  }
  existingBookmarkCount.value = count
}

async function clearExistingBookmarks() {
  const files = overlayFiles.value
  let cleared = 0
  for (const file of files) {
    try {
      const result = await tauriCallSafe('has_pdf_bookmarks', { input: file.path })
      if (result.ok && result.data) {
        const rm = await tauriCallSafe('remove_pdf_bookmarks', { input: file.path })
        if (rm.ok) cleared++
      }
    } catch {
      // 忽略单个文件失败
    }
  }
  existingBookmarkCount.value = 0
  existingBookmarkAlertVisible.value = false
  ElMessage.success(`已清除 ${cleared} 个文件的书签`)
}

useWindowFileDrop({
  onEnter: () => { evidenceDragging.value = true },
  onLeave: () => { evidenceDragging.value = false },
  onDrop: handleEvidenceDrop,
})

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
  headerInsertEnabled.value = true
  footerInsertEnabled.value = true
  pageNumberShowTotal.value = true
  normalizeA4.value = false
  removeAnnotations.value = false
  cleanupHeaderHeightMm.value = 18
  cleanupFooterHeightMm.value = 18
  // Reset the selected file's groups to a single default each
  const file = selectedOverlayFile.value
  if (file) {
    const headerGroup = createDefaultHeaderGroup()
    headerGroup.mode = workflowMode.value === 'split' ? 'per_file' : 'filename'
    file.headerGroups = [headerGroup]
    file.footerTextGroups = [createDefaultFooterTextGroup()]
    file.pageNumberGroups = [createDefaultPageNumberGroup()]
    file.selectedHeaderGroupId = headerGroup.id
    file.selectedFooterTextGroupId = 'ft1'
    file.selectedPageNumberGroupId = 'pn1'
  }
  footerEnabled.value = true
  footerContinuous.value = true
  footerText.value = '{page}/{total}'
  outputMode.value = 'files_and_merge'
  refreshPreview()
}

function hasReplacementRule() {
  return (
    (insertHeaderFooterEnabled.value &&
      (headerMode.value !== 'none' || footerInsertEnabled.value || footerEnabled.value)) ||
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

/** When suffix is disabled, check if output files already exist and prompt user. */
async function resolveSuffixConflicts(payload) {
  if (fileSuffixEnabled.value) return payload
  const items = payload.items || []
  const conflicts = []
  for (const item of items) {
    try {
      if (await exists(item.outputPath)) conflicts.push(item)
    } catch {
      /* ignore check errors */
    }
  }
  if (!conflicts.length) return payload

  let action
  try {
    await ElMessageBox.confirm(
      `${conflicts.length} 个输出文件已存在同名文件。`,
      '文件名冲突',
      {
        confirmButtonText: '覆盖已有文件',
        cancelButtonText: '共存（自动加序号）',
        distinguishCancelAndClose: true,
        type: 'warning',
      },
    )
    action = 'overwrite'
  } catch (e) {
    action = e === 'cancel' ? 'coexist' : null
  }
  if (!action) return null // user closed dialog → abort

  if (action === 'coexist') {
    for (const item of items) {
      if (!conflicts.includes(item)) continue
      const dot = item.outputPath.lastIndexOf('.')
      const base = dot > 0 ? item.outputPath.slice(0, dot) : item.outputPath
      const ext = dot > 0 ? item.outputPath.slice(dot) : '.pdf'
      let seq = 1
      let candidate
      do {
        candidate = `${base}_${seq}${ext}`
        seq++
        // eslint-disable-next-line no-await-in-loop
      } while (await exists(candidate).catch(() => false))
      item.outputPath = candidate
    }
    // Also update session items
    for (const sessionItem of payload.session?.items || []) {
      const matched = items.find((it) => it.inputPath === sessionItem.sourcePath)
      if (matched) sessionItem.outputPath = matched.outputPath
    }
  }
  return payload
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
    let payload = buildEvidencePdfRulePayload(overlayRows.value, rules, outputDir)
    payload = await resolveSuffixConflicts(payload)
    if (!payload) { overlaying.value = false; return }
    overlayRows.value.forEach((file) => {
      file.statusText = '替换中'
      file.statusType = 'warning'
      file.statusDetail = ''
    })

    const result = await tauriCallSafe('apply_evidence_pdf_rules', { args: payload })
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '证据 PDF 处理失败'))
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
        file.statusDetail = warnings.join('；')
        file.statusText = warnings.length ? '已替换，需注意' : '已替换'
        file.statusType = warnings.length ? 'warning' : 'success'
      } else if (failed) {
        file.statusText = '失败'
        file.statusType = 'danger'
        file.statusDetail = userFacingError(failed.message, '处理失败', 300)
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
      ElMessage.warning(`已替换 ${successCount} 个 PDF，其中 ${warningCount} 个有处理提示`)
    } else {
      ElMessage.success(`已替换 ${successCount} 个 PDF`)
    }
  } catch (err) {
    ElMessage.error(userFacingError(err?.message || err, '页眉页码替换失败'))
  } finally {
    overlaying.value = false
  }
}

async function applyHeaderFooter() {
  if (!canApplyOverlay.value) return
  overlaying.value = true
  try {
    let payload = buildEvidencePdfRulePayload(overlayRows.value, currentRules.value, overlayOutputDir.value)
    payload = await resolveSuffixConflicts(payload)
    if (!payload) { overlaying.value = false; return }
    overlayRows.value.forEach((file) => {
      file.statusText = '处理中'
      file.statusType = 'warning'
      file.statusDetail = ''
    })

    const result = await tauriCallSafe('apply_evidence_pdf_rules', { args: payload })
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, 'PDF 处理失败'))
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
        file.statusDetail = warnings.join('；')
        file.statusText = warnings.length ? '完成，需注意' : '完成'
        file.statusType = warnings.length ? 'warning' : 'success'
      } else if (failed) {
        file.statusText = '失败'
        file.statusType = 'danger'
        file.statusDetail = userFacingError(failed.message, '处理失败', 300)
      }
    })

    const failedCount = result.data.failed?.length || 0
    const successCount = result.data.results?.length || 0
    const warningCount = (result.data.results || []).filter((item) => item.warnings?.length).length
    const merge = result.data.merge
    if (merge?.status === 'done') {
      const cleanupText =
        merge.outputMode === 'merge_only' ? `，已清理 ${merge.removedIntermediates || 0} 个中间副本` : ''
      const warningText = warningCount ? `，其中 ${warningCount} 个有处理提示` : ''
      const msg = `已完成 ${successCount} 个 PDF，并已合并${cleanupText}${warningText}`
      ElNotification({ title: '处理完成', message: msg, type: 'success', duration: 0 })
    } else if (failedCount) {
      ElMessage.warning(`已完成 ${successCount} 个，失败 ${failedCount} 个`)
    } else if (warningCount) {
      ElMessage.warning(`已完成 ${successCount} 个 PDF，其中 ${warningCount} 个有处理提示`)
    } else if (merge?.status === 'skipped') {
      ElMessage.warning(`已完成 ${successCount} 个 PDF，${merge.message || '未合并'}`)
    } else {
      ElMessage.success(`已完成 ${successCount} 个 PDF`)
    }
  } catch (err) {
    ElMessage.error(userFacingError(err?.message || err, 'PDF 处理失败'))
    overlayFiles.value.forEach((file) => {
      file.statusText = '失败'
      file.statusType = 'danger'
    })
  } finally {
    overlaying.value = false
  }
}

const {
  rowHeaderPreview,
  displayRowHeader,
  displayRowFooter,
} = useEvidencePdfExistingEditing({
  editingHeaderPath,
  editingFooterPath,
  editingExistingHeaderPath,
  editingExistingFooterPath,
  editingExistingPageNumberPath,
  insertHeaderFooterEnabled,
  headerInsertEnabled,
  headerMode,
  footerInsertEnabled,
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

function openExistingElements(filter = 'all') {
  existingElementsFilter.value = filter
  existingElementsVisible.value = true
}

async function quickCleanupExistingHeaderFooter() {
  if (!overlayFiles.value.length) {
    ElMessage.info('请先导入 PDF 文件')
    return
  }
  quickCleanupRunning.value = true
  try {
    // Step 1: Ensure detection is done
    if (!hasDetectedExistingHeaderFooter.value) {
      await detectAllHeaderFooter({ silent: true })
    }
    if (!hasDetectedExistingHeaderFooter.value) {
      ElMessage.info('未检测到现有页眉页脚或页码')
      return
    }
    // Step 2: Open dialog for user to select what to delete
    quickCleanupPipeline = true
    existingElementsFilter.value = 'all'
    existingElementsVisible.value = true
    // The watcher will call finishQuickCleanupPipeline when dialog closes
  } catch (err) {
    quickCleanupPipeline = false
    ElMessage.error(userFacingError(err?.message || err, '检测失败'))
  } finally {
    quickCleanupRunning.value = false
  }
}

async function finishQuickCleanupPipeline() {
  // Check if anything was marked for deletion
  const deleteCount = overlayFiles.value.reduce(
    (sum, file) => sum + (file.existingElements || []).filter((e) => e.decision === 'delete').length,
    0,
  )
  if (!deleteCount) {
    ElMessage.info('未选择删除任何内容')
    return
  }

  // Step 3: Confirm
  try {
    await ElMessageBox.confirm(
      `将删除 ${deleteCount} 项已检测到的页眉页脚内容。标准结构直接删除，普通文本删除区域内匹配内容。处理后的文件将放在源文件旁边的 _cleaned 文件夹中。`,
      '确认删除页眉页脚',
      { confirmButtonText: '执行删除', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }

  // Step 4: Auto-set output to _cleaned subfolder next to source files
  const firstFile = overlayFiles.value[0]
  if (!firstFile?.path) return
  const sourceDir = parentDir(firstFile.path)
  const outputDir = `${sourceDir}/_cleaned`

  // Step 5: Sync element decisions to legacy flags the payload builder needs
  overlayFiles.value.forEach((file) => syncLegacyExistingElementState(file))

  // Step 6: Process with minimal settings (only delete, no new headers/footers)
  quickCleanupRunning.value = true
  overlaying.value = true
  try {
    const hasHeaderDelete = overlayFiles.value.some(f =>
      (f.existingElements || []).some(e => e.kind === 'header' && e.decision === 'delete'),
    )
    const hasFooterDelete = overlayFiles.value.some(f =>
      (f.existingElements || []).some(e => (e.kind === 'footerText' || e.kind === 'pageNumber') && e.decision === 'delete'),
    )
    const cleanupRules = {
      headerMode: 'none',
      headerInsertEnabled: false,
      footerInsertEnabled: false,
      pageNumberEnabled: false,
      footerEnabled: false,
      normalizeA4: false,
      removeAnnotations: false,
      outputMode: 'files_only',
      cleanupHeaderEnabled: hasHeaderDelete,
      cleanupFooterEnabled: hasFooterDelete,
      cleanupHeaderHeightMm: 18,
      cleanupFooterHeightMm: 18,
    }
    const payload = buildEvidencePdfRulePayload(overlayRows.value, cleanupRules, outputDir)
    overlayRows.value.forEach((file) => {
      file.statusText = '处理中'
      file.statusType = 'warning'
      file.statusDetail = ''
    })

    const result = await tauriCallSafe('apply_evidence_pdf_rules', { args: payload })
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '页眉页脚删除失败'))
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
        file.statusText = '已完成'
        file.statusType = 'success'
        file.statusDetail = (success.warnings || []).join('；')
      } else if (failed) {
        file.statusText = '失败'
        file.statusType = 'danger'
        file.statusDetail = userFacingError(failed.message, '处理失败', 300)
      }
    })

    const successCount = result.data.results?.length || 0
    const failedCount = result.data.failed?.length || 0
    if (failedCount) {
      ElMessage.warning(`已完成 ${successCount} 个，失败 ${failedCount} 个。输出目录：${outputDir}`)
    } else {
      ElMessage.success(`已完成 ${successCount} 个 PDF，输出目录：${outputDir}`)
    }
  } catch (err) {
    ElMessage.error(userFacingError(err?.message || err, '页眉页脚删除失败'))
  } finally {
    quickCleanupRunning.value = false
    overlaying.value = false
  }
}

function handleJumpToSettings(kind) {
  existingElementsVisible.value = false
  // Ensure the target section toggle is on
  if (kind === 'header') headerInsertEnabled.value = true
  else if (kind === 'footerText') footerInsertEnabled.value = true
  else if (kind === 'pageNumber') footerEnabled.value = true
  // Scroll to the header/footer rule fields section
  nextTick(() => {
    const el = document.querySelector('.rule-grid')
    if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  })
}

function handleExistingElementChange(row) {
  const { file } = row
  syncLegacyExistingElementState(file)
  file.statusText = fileExistingStatus(file).text
  file.statusType = fileExistingStatus(file).type
  truePreview.value = null
  refreshPreview()
}

function syncLegacyExistingElementState(file) {
  const elements = file.existingElements || []
  const applyKind = (kind, legacyName) => {
    const matches = elements.filter((element) => element.kind === kind)
    const edited = matches.find((element) => element.decision === 'edit')
    const representative = edited || matches.find((element) => element.decision !== 'ignore') || matches[0]
    file[`removeExisting${legacyName}`] = matches.some((element) => element.decision === 'delete')
    file[`existing${legacyName}Text`] = representative
      ? representative.decision === 'edit'
        ? representative.editedText
        : representative.detectedText
      : ''
    file[`existing${legacyName}Edited`] = matches.some(
      (element) => element.decision === 'edit' && element.source === 'artifact',
    )
    file[`convertPlain${legacyName}`] = matches.some(
      (element) => element.decision === 'edit' && element.source !== 'artifact',
    )
  }
  applyKind('header', 'Header')
  applyKind('footerText', 'Footer')
  applyKind('pageNumber', 'PageNumber')
}

function previewExistingElement(row) {
  // Previewing closes the dialog; do not let that trigger the quick-cleanup pipeline
  quickCleanupPipeline = false
  const index = overlayFiles.value.findIndex((file) => file.path === row.file.path)
  if (index >= 0) selectedOverlayIndex.value = index
  previewPage.value = Math.max(1, Number(row.element.pageStart || 1))
  selectedFooterCandidateKey.value = row.element.id
  truePreview.value = null
  refreshPreview()
}

function confirmAllExistingElements() {
  let count = 0
  for (const file of overlayFiles.value) {
    for (const el of file.existingElements || []) {
      if (!el.decision && !el.lowConfidence) {
        el.decision = 'keep'
        count++
      }
    }
    const status = fileExistingStatus(file)
    file.statusText = status.text
    file.statusType = status.type
  }
  if (count > 0) {
    ElMessage.success(`已确认 ${count} 个检测项`)
  } else {
    ElMessage.info('没有待确认的检测项')
  }
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
    ;(file.existingElements || []).forEach((element) => {
      element.decision = 'delete'
    })
    syncLegacyExistingElementState(file)
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
    ;(file.existingElements || []).forEach((element) => {
      element.decision = 'keep'
      element.editedText = element.detectedText
    })
    syncLegacyExistingElementState(file)
    const status = fileExistingStatus(file)
    file.statusText = status.text
    file.statusType = status.type
  })
  truePreview.value = null
  refreshPreview()
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
  if (prop === 'header') return displayRowHeader(row, index)
  if (prop === 'footer') return displayRowFooter(row, index)
  if (prop === 'pages') return Number(row?.pages || 0)
  if (prop === 'pageRange') return Number(row?.pageStart || 0)
  if (prop === 'sourceRange') return Number(row?.sourcePageStart || 0)
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

function reorderOverlayFiles(from, to) {
  if (from === to || from < 0 || to < 0 || from >= overlayFiles.value.length || to >= overlayFiles.value.length) return
  const selectedPath = selectedOverlayFile.value?.path
  const items = [...overlayFiles.value]
  const [item] = items.splice(from, 1)
  items.splice(to, 0, item)
  overlayFiles.value = items
  selectedOverlayIndex.value = selectedPath
    ? Math.max(0, items.findIndex((file) => file.path === selectedPath))
    : to
  truePreview.value = null
  refreshPreview()
}

function reorderMergedImportItems(from, to) {
  const items = mergedImportPlan.value?.items
  if (!items || from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) return
  const [item] = items.splice(from, 1)
  items.splice(to, 0, item)
  selectedMergedImportIndex.value = to
}

function removeOverlayFile(index) {
  overlayFiles.value.splice(index, 1)
  selectedOverlayIndex.value = Math.min(selectedOverlayIndex.value, Math.max(0, overlayFiles.value.length - 1))
  refreshPreview()
}

function focusGroup(row, kind, groupId) {
  const index = overlayRows.value.findIndex((item) => item.path === row.path)
  if (index >= 0) selectedOverlayIndex.value = index
  setSelectedGroup(row, kind, groupId)
  refreshPreview()
}

function removeGroupFromFile(row, kind, groupId) {
  const groups = groupsFor(row, kind)
  if (groups.length <= 1) return
  const updated = groups.filter((g) => g.id !== groupId)
  if (kind === 'header') row.headerGroups = updated
  else if (kind === 'footerText') row.footerTextGroups = updated
  else row.pageNumberGroups = updated
  if (row.selectedHeaderGroupId === groupId) row.selectedHeaderGroupId = updated[0]?.id || ''
  if (row.selectedFooterTextGroupId === groupId) row.selectedFooterTextGroupId = updated[0]?.id || ''
  if (row.selectedPageNumberGroupId === groupId) row.selectedPageNumberGroupId = updated[0]?.id || ''
  refreshPreview()
}

function contentKindLabel(kind) {
  if (kind === 'header') return '页眉'
  if (kind === 'footerText') return '页脚文字'
  return '页码'
}

function contentStatusLabel(status) {
  if (status === 'pending-write') return '待写入'
  if (status === 'pending-add') return '待添加'
  if (status === 'existing') return '已有'
  if (status === 'confirmed') return '已确认'
  if (status === 'pending-edit') return '待编辑'
  if (status === 'pending-delete') return '待删除'
  return status
}

function contentStatusTagType(status) {
  if (status === 'pending-write' || status === 'pending-add') return 'primary'
  if (status === 'confirmed') return 'success'
  if (status === 'existing') return 'info'
  if (status === 'pending-edit') return 'warning'
  if (status === 'pending-delete') return 'danger'
  return 'info'
}

function displayContentRowText(file, index, cr) {
  if (cr.source === 'new') {
    if (cr.kind === 'pageNumber') {
      // Show rendered page number
      const group = cr.group
      const seq = group.sequence || 'continuous'
      const continuous = seq !== 'per-file'
      const page = continuous ? file.pageStart || 1 : 1
      const total = continuous ? totalOverlayPages.value : file.pages || 1
      return renderPageNumberTemplate(cr.text, page, total, group.style || 'arabic')
    }
    return cr.text || '-'
  }
  // existing
  return cr.text || '-'
}

function mainColumnContentText(file, index) {
  const rows = buildFileContentRows(file, index, currentRules.value)
  if (!rows.length) return '-'
  return displayContentRowText(file, index, rows[0])
}

function startMainColumnContentEdit(row, index) {
  const rows = buildFileContentRows(row, index, currentRules.value)
  if (!rows.length) return
  const cr = rows[0]
  startContentRowEdit(row, cr)
}

function finishMainColumnContentEdit(row) {
  // Find the currently editing content row by id prefix
  const rowPrefix = `${row.path}|`
  if (!editingContentRowId.value.startsWith(rowPrefix)) return
  const crId = editingContentRowId.value.slice(rowPrefix.length)
  // Rebuild rows to find the matching content row
  const index = overlayRows.value.findIndex((item) => item.path === row.path)
  const rows = buildFileContentRows(row, index, currentRules.value)
  const cr = rows.find((r) => r.id === crId)
  if (cr && cr.source === 'existing') {
    // Blur on an existing row discards the edit (恢复原文)
    clearContentRowEdit()
  } else if (cr) {
    finishContentRowEdit(row, cr)
  } else {
    clearContentRowEdit()
  }
}

/** Discard an in-progress edit (existing rows blur outside → 放弃编辑). */
function cancelContentRowEdit() {
  clearContentRowEdit()
}

function startContentRowEdit(row, cr) {
  // Cancel any existing edit first
  clearContentRowEdit()
  editingContentRowId.value = `${row.path}|${cr.id}`
  editingContentRowKind.value = cr.kind
  if (cr.source === 'existing') {
    editingContentRowValue.value = cr.element?.editedText || cr.element?.detectedText || ''
  } else if (cr.kind === 'pageNumber') {
    // Show template in edit mode
    editingContentRowValue.value = cr.group?.template || '{page}/{total}'
  } else {
    editingContentRowValue.value = cr.text || ''
  }
}

function finishContentRowEdit(row, cr) {
  const value = String(editingContentRowValue.value ?? '').trim()
  if (cr.source === 'new') {
    finishNewContentRowEdit(row, cr, value)
  } else {
    finishExistingContentRowEdit(row, cr, value)
  }
  clearContentRowEdit()
  refreshPreview()
}

function finishNewContentRowEdit(row, cr, value) {
  // Fallback: if group reference is stale (e.g. groups array was rebuilt),
  // resolve it from the file's current groups.
  let group = cr.group || selectedGroupFor(row, cr.kind) || groupsFor(row, cr.kind)[0]
  if (!group) {
    // No groups exist for this kind — auto-create a default one
    if (cr.kind === 'header') {
      group = createDefaultHeaderGroup()
      row.headerGroups = [group]
      row.selectedHeaderGroupId = group.id
    } else if (cr.kind === 'footerText') {
      group = createDefaultFooterTextGroup()
      row.footerTextGroups = [group]
      row.selectedFooterTextGroupId = group.id
    } else {
      group = createDefaultPageNumberGroup()
      row.pageNumberGroups = [group]
      row.selectedPageNumberGroupId = group.id
    }
  }
  // Record old value for undo
  const oldValue = cr.kind === 'pageNumber' ? (group.template || '{page}/{total}') : (group.text || '')
  if (cr.kind === 'header') {
    group.text = value
    if (group.mode === 'filename' || group.mode === 'seq' || group.mode === 'seq_cn') {
      group.mode = 'per_file'
    }
    if (group.mode === 'per_file') {
      row.header = value
    }
  } else if (cr.kind === 'footerText') {
    group.text = value
  } else if (cr.kind === 'pageNumber') {
    if (!value.includes('{page}')) {
      ElMessage.warning('页码模板必须包含 {page} 占位符')
      return
    }
    group.template = value
  }
  pushEditUndo({
    type: 'group',
    filePath: row.path,
    kind: cr.kind,
    groupId: group.id,
    oldValue,
    newValue: cr.kind === 'pageNumber' ? group.template : group.text,
  })
  // Force Vue reactivity: ref([]) doesn't track deep property mutations,
  // so we trigger a shallow update to make the table re-render.
  overlayFiles.value = [...overlayFiles.value]
}

function finishExistingContentRowEdit(row, cr, value) {
  const element = cr.element
  if (!element) return
  const original = element.detectedText || ''
  const oldDecision = element.decision
  const oldEditedText = element.editedText || element.detectedText || ''
  if (!value) {
    // Empty → mark delete
    element.decision = 'delete'
    element.editedText = ''
  } else if (value !== original) {
    // Changed → mark edit
    element.decision = 'edit'
    element.editedText = value
    // Also update legacy fields
    if (cr.kind === 'header') {
      row.existingHeaderText = value
      row.existingHeaderEdited = true
      if (element.source !== 'artifact') row.convertPlainHeader = true
    } else if (cr.kind === 'footerText') {
      row.existingFooterText = value
      row.existingFooterEdited = true
      if (element.source !== 'artifact') row.convertPlainFooter = true
    } else {
      row.existingPageNumberText = value
      row.existingPageNumberEdited = true
      if (element.source !== 'artifact') row.convertPlainPageNumber = true
    }
  }
  syncLegacyExistingElementState(row)
  const status = fileExistingStatus(row)
  row.statusText = status.text
  row.statusType = status.type
  // Record for undo
  const newDecision = element.decision
  const newEditedText = element.editedText || element.detectedText || ''
  if (newDecision !== oldDecision || newEditedText !== oldEditedText) {
    pushEditUndo({
      type: 'element',
      filePath: row.path,
      kind: cr.kind,
      elementId: element.id,
      oldDecision,
      oldValue: oldEditedText,
      newDecision,
      newValue: newEditedText,
    })
  }
  // Force Vue reactivity for deep property mutations
  overlayFiles.value = [...overlayFiles.value]
}

function cancelExistingDecision(row, cr) {
  const element = cr.element
  if (!element) return
  element.decision = 'keep'
  element.editedText = element.detectedText
  syncLegacyExistingElementState(row)
  const status = fileExistingStatus(row)
  row.statusText = status.text
  row.statusType = status.type
  refreshPreview()
  overlayFiles.value = [...overlayFiles.value]
}

function removeContentRowNew(row, cr) {
  if (!cr.group) return
  removeGroupFromFile(row, cr.kind, cr.group.id)
}

async function removeContentRowExisting(row, cr) {
  const element = cr.element
  if (!element) return
  try {
    await ElMessageBox.confirm(
      '确认删除该条已检测到的内容？',
      '删除确认',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }
  element.decision = 'delete'
  syncLegacyExistingElementState(row)
  const status = fileExistingStatus(row)
  row.statusText = status.text
  row.statusType = status.type
  refreshPreview()
}

function clearContentRowEdit() {
  editingContentRowId.value = ''
  editingContentRowKind.value = ''
  editingContentRowValue.value = ''
}

// --- Global keyboard shortcut: Ctrl+Z / Ctrl+Shift+Z ---
function handleGlobalKeydown(e) {
  if (!(e.ctrlKey || e.metaKey) || e.key !== 'z') return
  const tag = (e.target?.tagName || '').toLowerCase()
  if (tag === 'input' || tag === 'textarea') return
  e.preventDefault()
  if (e.shiftKey) {
    if (!redoEdit() && inlineHfFieldsRef.value) inlineHfFieldsRef.value.redo()
  } else {
    if (!undoEdit() && inlineHfFieldsRef.value) inlineHfFieldsRef.value.undo()
  }
}

onMounted(() => window.addEventListener('keydown', handleGlobalKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleGlobalKeydown))
</script>

<style scoped>
.hf-workbench {
  display: flex;
  flex-direction: row;
  gap: 0;
  height: 100%;
  min-height: 0;
  padding: 18px 20px;
  overflow: hidden;
  background: var(--docsy-canvas);
}

.group-subrows {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 12px;
}

.group-subrow {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  color: var(--docsy-text-muted);
}

.group-subrow:hover {
  background: var(--docsy-surface-hover);
}

.group-subrow.selected {
  background: var(--docsy-surface-active);
  color: var(--docsy-text-strong);
  font-weight: 500;
}

.group-subrow.muted {
  cursor: default;
}

.group-kind {
  display: inline-block;
  min-width: 52px;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 12px;
  text-align: center;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
}

.group-subrow.selected .group-kind {
  background: var(--docsy-accent-subtle, rgba(64, 158, 255, 0.15));
  color: var(--docsy-accent, #409eff);
}

.group-label {
  min-width: 80px;
  font-weight: 500;
}

.group-summary {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hf-panel {
  flex: 1 1 0;
  min-width: 400px;
  max-width: 55%;
  overflow: auto;
  scrollbar-gutter: stable;
  padding-right: 8px;
}

.preview-panel {
  flex: 1.3 1 0;
  min-width: 320px;
  overflow: auto;
  scrollbar-gutter: stable;
  padding-left: 14px;
  border-left: 1px solid var(--docsy-border-subtle);
  resize: horizontal;
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
  margin-bottom: 12px;
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-primary-soft);
  color: var(--docsy-text-strong);
  min-height: 60px;
}

.local-processing p {
  margin: 3px 0 0;
  color: var(--docsy-text);
  font-size: 12px;
}


h3 {
  margin: 0 0 6px;
  color: var(--docsy-text-strong);
}

.hint {
  color: var(--docsy-text-muted);
  font-size: 13px;
  margin: 0;
}

.rule-block {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  padding: 12px;
  margin-bottom: 12px;
  background: var(--docsy-surface-muted);
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
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
}

.summary-item span {
  display: block;
  color: var(--docsy-text-muted);
  font-size: 12px;
  margin-bottom: 4px;
}

.summary-item strong {
  display: block;
  color: var(--docsy-text-strong);
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
  color: var(--docsy-text-strong);
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
  appearance: none;
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.summary-pill:hover {
  border-color: var(--docsy-primary);
}

.summary-pill span {
  display: block;
  color: var(--docsy-text-muted);
  font-size: 12px;
  line-height: 1.2;
}

.summary-pill strong {
  display: block;
  margin-top: 4px;
  color: var(--docsy-text-strong);
  font-size: 16px;
  line-height: 1;
}

.summary-pill.active {
  border-color: #e5bd92;
  background: var(--docsy-accent-soft);
}

.summary-pill.warning.active strong {
  color: #b42318;
}

.rule-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 8px 12px;
}

.rule-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-item label {
  font-size: 12px;
  color: var(--docsy-text);
}

.field-hint {
  font-size: 11px;
  line-height: 1.4;
  color: var(--docsy-text-muted);
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin: 12px 0;
}

.path-text {
  color: var(--docsy-text);
  font-size: 13px;
  margin-bottom: 8px;
}

.output-plan {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--docsy-text);
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
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
}

.split-cleanup-note {
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.detection-plan,
.merged-import-plan {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
  margin: 12px 0;
  padding: 10px;
}

.import-plan-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 10px;
  color: var(--docsy-text);
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
  color: var(--docsy-primary);
  background: transparent;
  cursor: pointer;
  font: inherit;
  text-align: left;
}

.file-link:hover {
  color: var(--docsy-primary-hover);
  text-decoration: underline;
}

.table-drag-handle {
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

.index-drag-cell {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.index-number {
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.index-drag-overlay {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: 0;
  transition: opacity 0.15s;
  display: inline-grid;
  place-items: center;
}

.index-drag-cell:hover .index-number {
  visibility: hidden;
}

.index-drag-cell:hover .index-drag-overlay {
  opacity: 1;
}

.table-drag-handle:active {
  cursor: grabbing;
  color: var(--docsy-primary);
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
  color: var(--docsy-text-strong);
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
  display: inline-block;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: middle;
}

.overlay-table {
  margin-top: 0;
  width: 100%;
}
/* Force auto-width columns to shrink when empty */
.overlay-table .el-table__body {
  table-layout: auto;
}
/* Ensure columns with min-width can shrink when content is short */
.overlay-table .el-table__header th .cell {
  overflow: hidden;
  text-overflow: ellipsis;
}

.bookmark-rule-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  align-items: center;
}

.content-subrows {
  padding: 6px 12px;
}

.content-subtable {
  width: 100%;
}

.content-kind-tag {
  display: inline-block;
  min-width: 52px;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 12px;
  text-align: center;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
}

.content-kind-tag.source-new {
  background: var(--docsy-accent-subtle, rgba(64, 158, 255, 0.15));
  color: var(--docsy-accent, #409eff);
}

.preview-controls {
  display: flex;
  gap: 8px;
  align-items: center;
}

.footer-candidate-panel {
  margin-bottom: 10px;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-elevated);
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
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
}

.footer-candidate-item.active {
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
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
  color: var(--docsy-text-strong);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.candidate-main span {
  display: block;
  margin-top: 2px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.preview-stage {
  display: flex;
  justify-content: center;
  padding: 12px;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  min-height: 520px;
}

.true-preview-stage {
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding: 12px;
  background: var(--docsy-surface-muted);
  border: 1px solid var(--docsy-border-subtle);
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


.evidence-drop-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(37, 99, 235, 0.08);
  border: 2px dashed var(--docsy-primary, #2563eb);
  border-radius: 12px;
  pointer-events: none;
}

.evidence-drop-message {
  padding: 16px 32px;
  background: var(--docsy-surface-elevated, #fff);
  border-radius: 8px;
  font-size: 15px;
  font-weight: 500;
  color: var(--docsy-primary, #2563eb);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
}
</style>

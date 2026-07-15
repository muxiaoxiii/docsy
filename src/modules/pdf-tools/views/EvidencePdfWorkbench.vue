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
      <div v-if="splittingMergedImport" class="local-processing">
        <span class="processing-spinner" />
        <div>
          <strong>正在拆分 PDF</strong>
          <p>大文件会按页段逐个输出，当前只占用证据拆分区域；请先不要重复点击确认拆分。</p>
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
          <div class="block-title">拆分后处理</div>
          <p class="hint">处理后文件会输出到新文件夹，不覆盖原拆分文件。</p>
          <p class="path-text">输出文件夹：{{ splitReplacementOutputDirValue }}</p>
        </div>
        <div class="plan-actions">
          <el-button size="small" @click="selectSplitReplacementOutputDir">输出目录</el-button>
          <el-button size="small" @click="openHeaderFooterSettings">设置页眉页脚</el-button>
          <el-button size="small" :disabled="!hasDetectedExistingHeaderFooter" @click="markRemoveExistingHeaderFooter">删除现有页眉页脚</el-button>
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
            <label>页眉前缀</label>
            <el-input v-model="headerPrefix" placeholder="可用 [##]、[YYYYMMDD]" :disabled="headerMode === 'none'" />
          </div>
          <div class="rule-item">
            <label>页眉后缀</label>
            <el-input v-model="headerSuffix" placeholder="可用 [##]、[YYYYMMDD]" :disabled="headerMode === 'none'" />
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
            <label>页眉水平偏移 mm</label>
            <el-input-number v-model="headerOffsetXMm" :min="-120" :max="120" :step="1" :disabled="headerMode === 'none'" />
          </div>
          <div class="rule-item">
            <label>页眉颜色</label>
            <el-color-picker v-model="headerColor" :disabled="headerMode === 'none'" />
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
          <div class="rule-item">
            <label>页脚水平偏移 mm</label>
            <el-input-number v-model="footerOffsetXMm" :min="-120" :max="120" :step="1" :disabled="!footerEnabled" />
          </div>
          <div class="rule-item">
            <label>页脚颜色</label>
            <el-color-picker v-model="footerColor" :disabled="!footerEnabled" />
          </div>
        </div>
        <el-alert
          v-if="headerFooterOverflowWarnings.length"
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
        <el-button :disabled="!overlayFiles.length" @click="refreshOverlayPageCounts" :loading="checkingOverlayPages">刷新页数</el-button>
        <el-button :disabled="!hasDetectedExistingHeaderFooter" @click="markRemoveExistingHeaderFooter">删除现有页眉页脚</el-button>
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
              <el-input-number v-model="row.pageStart" :min="1" :max="mergedImportPlan.totalPages || 999999" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="结束页" prop="pageEnd" sortable="custom" width="108">
            <template #default="{ row }">
              <el-input-number v-model="row.pageEnd" :min="1" :max="mergedImportPlan.totalPages || 999999" size="small" />
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
              <el-button link type="primary" size="small" @click.stop="insertMergedImportRangeAfter($index)">续段</el-button>
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
        <el-table-column label="页眉" prop="header" sortable="custom" min-width="170">
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
        <el-table-column label="页脚" prop="footer" sortable="custom" min-width="150">
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
        <el-table-column v-if="workflowMode === 'split'" label="来源页段" prop="sourceRange" sortable="custom" width="105">
          <template #default="{ row }">{{ sourceRangeText(row) }}</template>
        </el-table-column>
        <el-table-column label="状态" prop="status" sortable="custom" width="92">
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

    <el-dialog v-model="headerFooterSettingsVisible" title="页眉页脚格式" width="760px" destroy-on-close>
      <div class="dialog-rule-grid">
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
          <el-input v-model="headerText" />
        </div>
        <div class="rule-item">
          <label>页眉前缀</label>
          <el-input v-model="headerPrefix" placeholder="可用 [##]、[YYYYMMDD]" :disabled="headerMode === 'none'" />
        </div>
        <div class="rule-item">
          <label>页眉后缀</label>
          <el-input v-model="headerSuffix" placeholder="可用 [##]、[YYYYMMDD]" :disabled="headerMode === 'none'" />
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
          <label>页眉水平偏移 mm</label>
          <el-input-number v-model="headerOffsetXMm" :min="-120" :max="120" :step="1" :disabled="headerMode === 'none'" />
        </div>
        <div class="rule-item">
          <label>页眉颜色</label>
          <el-color-picker v-model="headerColor" :disabled="headerMode === 'none'" />
        </div>
        <div class="rule-item">
          <label>页脚页码</label>
          <el-switch v-model="footerEnabled" active-text="启用" inactive-text="关闭" />
        </div>
        <div class="rule-item">
          <label>页码方式</label>
          <el-select v-model="footerContinuous" :disabled="!footerEnabled">
            <el-option :value="true" label="拼接连续页码" />
            <el-option :value="false" label="每个文件单独页码" />
          </el-select>
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
        <div class="rule-item">
          <label>页脚水平偏移 mm</label>
          <el-input-number v-model="footerOffsetXMm" :min="-120" :max="120" :step="1" :disabled="!footerEnabled" />
        </div>
        <div class="rule-item">
          <label>页脚颜色</label>
          <el-color-picker v-model="footerColor" :disabled="!footerEnabled" />
        </div>
      </div>
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
            <el-button size="small" :disabled="previewPage >= previewMaxPage" @click="movePreviewPage(1)">下一页</el-button>
            <el-button size="small" :disabled="!selectedMergedImportRange" @click="setSelectedMergedRangeStart">设为起始页</el-button>
            <el-button size="small" :disabled="!selectedMergedImportRange" @click="setSelectedMergedRangeEnd">设为结束页</el-button>
          </template>
          <el-input-number v-model="previewPage" :min="1" :max="previewMaxPage" :disabled="!activePreviewFilePath" size="small" />
          <el-button size="small" :disabled="!activePreviewFilePath" @click="refreshPreview">重新渲染当前页</el-button>
          <el-button size="small" type="primary" :loading="truePreviewLoading" :disabled="!selectedOverlayFile || Boolean(mergedImportPlan)" @click="renderTruePreview">
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
          <div v-if="showRulePreviewOverlays && previewHeaderText" class="preview-text preview-header-text" :style="previewHeaderStyle">
            {{ previewHeaderText }}
          </div>
          <div v-if="showRulePreviewOverlays && previewFooterText" class="preview-text preview-footer-text" :style="previewFooterStyle">
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
import {
  assignPageRanges,
  buildEvidencePdfRulePayload,
  buildHeaderFooterItems,
  buildHeaderText as buildSessionHeaderText,
  buildMergeOutputPath,
  buildOutputDir,
  canWriteFooter,
  canWriteHeader,
  candidateTargetRange,
  createEvidenceFile,
  expandPlaceholders,
  fileName,
  pageRangeText,
  parentDir,
  sortByNatural,
  stripPdf,
  totalPages,
} from '../composables/useEvidencePdfSession.js'
import { textOverlayStyle } from '../composables/pdfPreviewCoordinates.js'
import {
  formatSplitFileName,
  todayCompact,
} from '../composables/splitFileName.js'
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
const splitNamePrefix = ref('')
const splitNameSuffix = ref('[YYYYMMDD]')
const splitNameDateValue = ref(todayCompact())
const splitNameSeparator = ref('-')
const splitNameCustomSeparator = ref('')
const headerFooterSettingsVisible = ref(false)
const splitReplacementOutputDir = ref('')

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
const cleanupHeaderHeightMm = ref(18)
const cleanupFooterHeightMm = ref(18)
const headerMode = ref('filename')
const headerText = ref('')
const headerPrefix = ref('')
const headerSuffix = ref('')
const headerAlign = ref('center')
const headerFontSize = ref(10)
const headerMarginMm = ref(10)
const headerOffsetXMm = ref(0)
const headerColor = ref('#000000')
const footerEnabled = ref(true)
const footerText = ref('{page}/{total}')
const footerContinuous = ref(true)
const footerAlign = ref('center')
const footerFontSize = ref(9)
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
  : '普通预览会即时显示新页眉页脚位置；真实预览需手动生成'
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
  return notes
})
const showProcessingControls = computed(() =>
  !mergedImportPlan.value &&
  workflowMode.value !== 'split'
)
const showSessionSummary = computed(() =>
  overlayFiles.value.length > 0 &&
  !mergedImportPlan.value &&
  workflowMode.value !== 'split'
)
const showSplitResultActions = computed(() =>
  workflowMode.value === 'split' &&
  overlayFiles.value.length > 0 &&
  !mergedImportPlan.value
)
const splitReplacementOutputDirValue = computed(() =>
  splitReplacementOutputDir.value || defaultSplitReplacementOutputDir()
)
const processButtonText = computed(() =>
  workflowMode.value === 'merge' ? '执行合并处理' : '执行证据处理'
)
const autoCleanupHeaderEnabled = computed(() =>
  headerMode.value !== 'none' && overlayFiles.value.some((file) => file.existingHeaderArtifact)
)
const autoCleanupFooterEnabled = computed(() =>
  footerEnabled.value && overlayFiles.value.some((file) => file.existingFooterArtifact)
)
const hasDetectedExistingHeaderFooter = computed(() =>
  overlayFiles.value.some((file) => hasExistingHeaderFooter(file))
)
const hasExistingRemovalRule = computed(() =>
  overlayFiles.value.some((file) => file.removeExistingHeader || file.removeExistingFooter)
)
const canApplyOverlay = computed(() =>
  overlayFiles.value.length > 0 &&
  totalOverlayPages.value > 0 &&
  (normalizeA4.value || removeAnnotations.value || autoCleanupHeaderEnabled.value || autoCleanupFooterEnabled.value || hasExistingRemovalRule.value || headerMode.value !== 'none' || footerEnabled.value)
)
const canApplySplitReplacement = computed(() =>
  showSplitResultActions.value &&
  totalOverlayPages.value > 0 &&
  !overlaying.value
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
  headerMode: headerMode.value,
  headerText: headerText.value,
  headerPrefix: headerPrefix.value,
  headerSuffix: headerSuffix.value,
  headerDateValue: splitNameDateValue.value,
  headerAlign: headerAlign.value,
  headerFontSize: headerFontSize.value,
  headerMarginMm: headerMarginMm.value,
  headerOffsetXMm: headerOffsetXMm.value,
  headerColor: headerColor.value,
  footerEnabled: footerEnabled.value,
  footerText: footerText.value,
  footerContinuous: footerContinuous.value,
  footerAlign: footerAlign.value,
  footerFontSize: footerFontSize.value,
  footerMarginMm: footerMarginMm.value,
  footerOffsetXMm: footerOffsetXMm.value,
  footerColor: footerColor.value,
  outputMode: outputMode.value,
  mergeAfterProcessing: outputMode.value !== 'files_only',
  mergeFileName: mergeFileName.value,
}))

const previewHeaderText = computed(() => {
  if (!selectedOverlayFile.value || headerMode.value === 'none') return ''
  if (!shouldShowLiveHeader(selectedOverlayFile.value)) return ''
  return buildSessionHeaderText(selectedOverlayFile.value, selectedOverlayIndex.value, currentRules.value)
})

const previewFooterText = computed(() => {
  const footerTemplate = selectedOverlayFile.value?.footer ?? footerText.value
  if (!selectedOverlayFile.value || !footerEnabled.value || !footerTemplate) return ''
  if (!shouldShowLiveFooter(selectedOverlayFile.value)) return ''
  const page = footerContinuous.value
    ? selectedOverlayFile.value.pageStart + previewPage.value - 1
    : previewPage.value
  const total = footerContinuous.value
    ? totalOverlayPages.value
    : selectedOverlayFile.value.pages || 1
  return expandPlaceholders(footerTemplate, page, total)
})

const previewHeaderStyle = computed(() => textOverlayStyle('header', previewData.value, {
  align: headerAlign.value,
  marginMm: headerMarginMm.value,
  fontSize: headerFontSize.value,
  offsetXMm: headerOffsetXMm.value,
  color: headerColor.value,
}))
const previewFooterStyle = computed(() => textOverlayStyle('footer', previewData.value, {
  align: footerAlign.value,
  marginMm: footerMarginMm.value,
  fontSize: footerFontSize.value,
  offsetXMm: footerOffsetXMm.value,
  color: footerColor.value,
}))
const truePreviewFrameStyle = computed(() => ({
  aspectRatio: truePreview.value?.widthPx && truePreview.value?.heightPx
    ? `${truePreview.value.widthPx} / ${truePreview.value.heightPx}`
    : `${truePreview.value?.widthPt || 595.28} / ${truePreview.value?.heightPt || 841.89}`,
}))
const headerFooterOverflowWarnings = computed(() => {
  const warnings = []
  const widthPt = previewData.value?.widthPt || 595.28
  if (previewHeaderText.value && estimateTextWidthPt(previewHeaderText.value, headerFontSize.value) > widthPt * 0.92) {
    warnings.push('当前页眉可能超出页面宽度，请缩短文本、调整位置或减小字号')
  }
  if (previewFooterText.value && estimateTextWidthPt(previewFooterText.value, footerFontSize.value) > widthPt * 0.92) {
    warnings.push('当前页脚可能超出页面宽度，请缩短文本、调整位置或减小字号')
  }
  return warnings
})
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
})

watch([previewPage, currentRules], () => {
  truePreview.value = null
}, {
  deep: true,
})

function applyWorkflowDefaults() {
  if (workflowMode.value === 'split') {
    headerMode.value = 'none'
    footerEnabled.value = false
    footerContinuous.value = true
    outputMode.value = 'files_only'
    mergeFileName.value = 'split_evidence.pdf'
    return
  }
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
    const detectedPagesAnalyzed = inspect.ok ? Number(inspect.data.pagesAnalyzed || 0) : 0
    const detectedHeaderPages = inspect.ok ? Number(inspect.data.headerPages || 0) : 0
    const detectedPageNumberFooterPages = inspect.ok ? Number(inspect.data.pageNumberFooterPages || 0) : 0
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
      pagesAnalyzed: detectedPagesAnalyzed,
      headerPages: detectedHeaderPages,
      pageNumberFooterPages: detectedPageNumberFooterPages,
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
    const rawItemByRange = new Map((mergedImportPlan.value.items || []).map((item) => [sourcePageRangeKey(item), item]))
    overlayFiles.value = outputs.map((output) => {
      const sourceItem = rawItemByRange.get(sourcePageRangeKey(output)) || {}
      const pages = Math.max(0, Number(output.pageEnd || 0) - Number(output.pageStart || 0) + 1)
      const needsReview = sourceItem.source === 'fallback' || sourceItem.source === 'manual' || hasSplitWarning(split.data.warnings || [], output)
      return {
        ...createEvidenceFile(output.outputPath),
        header: sourceItem.name || output.name,
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
    splitReplacementOutputDir.value = ''
    selectedOverlayIndex.value = 0
    previewPage.value = 1
    applyWorkflowDefaults()
    mergedImportPlan.value = null
    refreshPreview()
    await detectAllHeaderFooter({ silent: true })

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
    name: formatSplitOutputName(item, index),
    pageStart: Number(item.pageStart || 0),
    pageEnd: Number(item.pageEnd || 0),
    source: item.source || 'unknown',
  }))
}

function defaultMergedImportName(inputPath, index) {
  if (index === 0) {
    return '目录'
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

function splitOutputNamePreview(row, index) {
  return formatSplitOutputName(row, index)
}

function formatSplitOutputName(row, index) {
  const base = String(row?.name || defaultMergedImportName('', index)).trim() || defaultMergedImportName('', index)
  return formatSplitFileName({
    base,
    index,
    prefix: splitNamePrefix.value,
    suffix: splitNameSuffix.value,
    dateValue: splitNameDateValue.value,
    separator: splitNameSeparator.value,
    customSeparator: splitNameCustomSeparator.value,
  })
}

function buildManualMergedImportPlan(inputPath, outputDir, total, warnings = []) {
  return {
    inputPath,
    outputDir,
    totalPages: Math.max(1, Number(total || 1)),
    pagesAnalyzed: 0,
    headerPages: 0,
    pageNumberFooterPages: 0,
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

function sortMergedImportItems({ prop, order }) {
  if (!mergedImportPlan.value || !prop || !order) return
  const selected = selectedMergedImportRange.value
  mergedImportPlan.value.items = sortByNatural(
    mergedImportPlan.value.items,
    (row, index) => mergedImportSortValue(row, prop, index),
    order,
  )
  if (selected) {
    selectedMergedImportIndex.value = Math.max(0, mergedImportPlan.value.items.indexOf(selected))
  }
}

function mergedImportSortValue(row, prop, index) {
  if (prop === 'outputName') return splitOutputNamePreview(row, index)
  if (prop === 'pageStart') return Number(row?.pageStart || 0)
  if (prop === 'pageEnd') return Number(row?.pageEnd || 0)
  if (prop === 'pageCount') return mergedImportRangePageCount(row)
  if (prop === 'source') return mergedImportSourceText(row)
  return row?.[prop] ?? ''
}

function mergedImportRangePageCount(row) {
  const pageStart = Number(row?.pageStart || 0)
  const pageEnd = Number(row?.pageEnd || 0)
  return pageStart > 0 && pageEnd >= pageStart ? pageEnd - pageStart + 1 : 0
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

function sourcePageRangeKey(item) {
  return `${Number(item.pageStart || 0)}-${Number(item.pageEnd || 0)}`
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

async function openEvidenceFile(row) {
  const path = row?.outputPath || row?.path
  if (!path) return
  const result = await tauriCallSafe('open_path', { path })
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

function openHeaderFooterSettings() {
  ensureReplacementPreset()
  headerFooterSettingsVisible.value = true
}

function applyHeaderFooterSettings() {
  headerFooterSettingsVisible.value = false
  refreshPreview()
}

function applyReplacementPreset() {
  normalizeA4.value = false
  removeAnnotations.value = false
  cleanupHeaderHeightMm.value = 18
  cleanupFooterHeightMm.value = 18
  headerMode.value = workflowMode.value === 'split' ? 'per_file' : 'filename'
  headerAlign.value = 'right'
  headerFontSize.value = 10
  headerMarginMm.value = 10
  footerEnabled.value = true
  footerContinuous.value = true
  footerText.value = '{page}/{total}'
  footerAlign.value = 'center'
  footerFontSize.value = 9
  footerMarginMm.value = 10
  outputMode.value = 'files_and_merge'
  refreshPreview()
}

function hasReplacementRule() {
  return headerMode.value !== 'none' ||
    footerEnabled.value ||
    hasExistingRemovalRule.value
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
    overlaying.value = false
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
      file.statusText = '已替换'
      file.statusType = 'success'
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
  if (failedCount) {
    ElMessage.warning(`已替换 ${successCount} 个，失败 ${failedCount} 个`)
  } else {
    ElMessage.success(`已替换 ${successCount} 个 PDF`)
  }
  overlaying.value = false
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

async function detectAllHeaderFooter(options = {}) {
  const silent = Boolean(options.silent)
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
      const status = fileExistingStatus(file)
      file.statusText = status.text
      file.statusType = status.type
      success += 1
    } else {
      file.statusText = '检测失败'
      file.statusType = 'danger'
      failed += 1
    }
  }
  if (!silent) {
    failed ? ElMessage.warning(`已检测 ${success} 个，失败 ${failed} 个`) : ElMessage.success(`已检测 ${success} 个 PDF`)
  }
  detectingAllHeaderFooter.value = false
}

function fileExistingStatus(file) {
  if (file.removeExistingHeader || file.removeExistingFooter) {
    return { text: '删除待处理', type: 'warning' }
  }
  if (file.convertPlainHeader || file.convertPlainFooter) {
    return { text: '转换待处理', type: 'warning' }
  }
  if (file.existingHeaderArtifact || file.existingFooterArtifact) {
    return { text: '现有可编辑', type: 'warning' }
  }
  if (file.existingHeaderText || file.existingFooterText) {
    return { text: '普通文本可转换', type: 'warning' }
  }
  return { text: '无旧页眉页码', type: 'success' }
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
  const parts = []
  if (data.artifact?.hasHeader) parts.push(`发现结构化页眉 ${data.artifact.headerCount} 处`)
  if (data.artifact?.hasFooter) parts.push(`发现结构化页脚 ${data.artifact.footerCount} 处`)
  if (header) parts.push(`页眉候选：${header.text}`)
  if (footer) parts.push(`页脚候选：${footer.text}`)
  if (candidates.length) parts.push(`候选 ${candidates.length} 个`)
  file.existingHeaderText = header?.text || ''
  file.existingFooterText = footer?.text || footer?.normalizedText || ''
  file.existingHeaderNormalizedText = header?.normalizedText || header?.text || ''
  file.existingFooterNormalizedText = footer?.normalizedText || footer?.text || ''
  const headerTargetRange = candidateTargetRange(header, file.pages)
  const footerTargetRange = candidateTargetRange(footer, file.pages)
  file.existingHeaderPageStart = headerTargetRange.start
  file.existingHeaderPageEnd = headerTargetRange.end
  file.existingFooterPageStart = footerTargetRange.start
  file.existingFooterPageEnd = footerTargetRange.end
  file.existingHeaderArtifact = Boolean(data.artifact?.hasHeader)
  file.existingFooterArtifact = Boolean(data.artifact?.hasFooter)
  if (!file.existingHeaderText || file.existingHeaderArtifact) file.convertPlainHeader = false
  if (!file.existingFooterText || file.existingFooterArtifact) file.convertPlainFooter = false
  if (!hasExistingHeader(file)) file.removeExistingHeader = false
  if (!hasExistingFooter(file)) file.removeExistingFooter = false
  if (header?.text && !file.headerEdited) {
    file.header = header.text
  }
  if ((footer?.normalizedText || footer?.text) && !file.footerEdited) {
    file.footer = footer.normalizedText || footer.text
  }
  file.detectionSummary = parts.length ? parts.join('；') : '未发现稳定的文本型页眉页脚候选'
  file.detectionCandidates = candidates
}

function estimateTextWidthPt(text, fontSize) {
  return String(text || '')
    .split('')
    .reduce((sum, ch) => sum + estimatePreviewCharWidth(ch) * Number(fontSize || 10), 0)
}

function estimatePreviewCharWidth(ch) {
  if (/[\u3400-\u9fff\uf900-\ufaff]/.test(ch)) return 1
  if (/\s/.test(ch)) return 0.28
  if (/[0-9A-Za-z]/.test(ch)) return 0.56
  return 0.5
}

function selectPreviewRow(row) {
  const index = overlayRows.value.findIndex((item) => item.path === row.path)
  if (index >= 0) {
    selectedOverlayIndex.value = index
    previewPage.value = 1
    truePreview.value = null
    previewReloadKey.value += 1
  }
}

function isEditingHeader(row) {
  return editingHeaderPath.value && editingHeaderPath.value === row.path
}

async function startHeaderEdit(row, index) {
  if (!row) return
  if (!canWriteHeader(row)) {
    const confirmed = await confirmPlainTextConversion(row, 'header')
    if (!confirmed) return
  } else if (row.existingHeaderArtifact && !row.removeExistingHeader) {
    const confirmed = await confirmStandardArtifactEdit(row, 'header')
    if (!confirmed) return
  }
  headerMode.value = 'per_file'
  if (row.header === null || row.header === undefined) {
    row.header = displayRowHeader(row, index) || stripPdf(row.name)
  }
  editingHeaderPath.value = row.path
}

function finishHeaderEdit(row) {
  if (row) {
    row.header = String(row.header ?? '').trim()
    row.headerEdited = true
  }
  editingHeaderPath.value = ''
}

function isEditingFooter(row) {
  return editingFooterPath.value && editingFooterPath.value === row.path
}

async function startFooterEdit(row, index) {
  if (!row) return
  if (!canWriteFooter(row)) {
    const confirmed = await confirmPlainTextConversion(row, 'footer')
    if (!confirmed) return
  } else if (row.existingFooterArtifact && !row.removeExistingFooter) {
    const confirmed = await confirmStandardArtifactEdit(row, 'footer')
    if (!confirmed) return
  }
  if (row.footer === null || row.footer === undefined) {
    row.footer = displayRowFooter(row, index)
  }
  editingFooterPath.value = row.path
}

function finishFooterEdit(row) {
  if (row) {
    row.footer = String(row.footer ?? '').trim()
    row.footerEdited = true
  }
  editingFooterPath.value = ''
}

function rowHeaderPreview(row, index) {
  return buildSessionHeaderText(row, index, currentRules.value)
}

function displayRowHeader(row, index) {
  if (row?.removeExistingHeader) return rowHeaderPreview(row, index)
  if (row?.existingHeaderText && !row?.headerEdited) return row.existingHeaderText
  if (workflowMode.value === 'split') {
    return row?.header ?? ''
  }
  return rowHeaderPreview(row, index)
}

function rowFooterPreview(row, index) {
  const footerTemplate = row?.footer ?? footerText.value
  if (!footerEnabled.value || !footerTemplate || !row) return ''
  const page = footerContinuous.value
    ? row.pageStart || 1
    : 1
  const total = footerContinuous.value
    ? totalOverlayPages.value || row.pages || 1
    : row.pages || 1
  return expandPlaceholders(footerTemplate, page, total)
}

function displayRowFooter(row, index) {
  if (row?.removeExistingFooter) return rowFooterPreview(row, index)
  if (row?.existingFooterText && !row?.footerEdited) return row.existingFooterText
  if (row?.footer !== null && row?.footer !== undefined) return row.footer
  return rowFooterPreview(row, index)
}

function isPlainHeader(row) {
  return Boolean(row?.existingHeaderText && !row?.existingHeaderArtifact)
}

function isPlainFooter(row) {
  return Boolean(row?.existingFooterText && !row?.existingFooterArtifact)
}

function shouldShowLiveHeader(row) {
  if (!row) return false
  if (row.existingHeaderArtifact && !row.removeExistingHeader) return false
  if (isPlainHeader(row) && !row.convertPlainHeader && !row.removeExistingHeader) return false
  return true
}

function shouldShowLiveFooter(row) {
  if (!row) return false
  if (row.existingFooterArtifact && !row.removeExistingFooter) return false
  if (isPlainFooter(row) && !row.convertPlainFooter && !row.removeExistingFooter) return false
  return true
}

function hasExistingHeader(row) {
  return Boolean(row?.existingHeaderText || row?.existingHeaderArtifact)
}

function hasExistingFooter(row) {
  return Boolean(row?.existingFooterText || row?.existingFooterArtifact)
}

function hasExistingHeaderFooter(row) {
  return hasExistingHeader(row) || hasExistingFooter(row)
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
    if (hasExistingHeader(file)) file.removeExistingHeader = true
    if (hasExistingFooter(file)) file.removeExistingFooter = true
    const status = fileExistingStatus(file)
    file.statusText = status.text
    file.statusType = status.type
  })
  truePreview.value = null
  refreshPreview()
}

async function confirmPlainTextConversion(row, region) {
  const isHeader = region === 'header'
  const text = isHeader ? row.existingHeaderText : row.existingFooterText
  const start = isHeader ? row.existingHeaderPageStart : row.existingFooterPageStart
  const end = isHeader ? row.existingHeaderPageEnd : row.existingFooterPageEnd
  const label = isHeader ? '页眉' : '页脚'
  const range = `${start || 1}-${end || row.pages || 1}`
  try {
    await ElMessageBox.confirm(
      `将删除第 ${range} 页${label}区域内匹配的普通文本“${text}”，并转换为 Docsy 标准${label}。匹配不到的文本会保留，不会遮盖原文。`,
      `转换普通文本${label}`,
      {
        confirmButtonText: '确认转换',
        cancelButtonText: '取消',
        type: 'warning',
      },
    )
    if (isHeader) row.convertPlainHeader = true
    else row.convertPlainFooter = true
    const status = fileExistingStatus(row)
    row.statusText = status.text
    row.statusType = status.type
    truePreview.value = null
    return true
  } catch {
    return false
  }
}

async function confirmStandardArtifactEdit(row, region) {
  const label = region === 'header' ? '页眉' : '页脚'
  try {
    await ElMessageBox.confirm(
      `检测到现有标准${label}。继续编辑会尝试修改原有${label}对象；如果要删除旧${label}后重新插入，请取消并点击“删除现有页眉页脚”。`,
      `编辑现有${label}`,
      {
        confirmButtonText: '继续编辑',
        cancelButtonText: '取消',
        type: 'warning',
      },
    )
    return true
  } catch {
    return false
  }
}

function headerFooterHandlingText(row) {
  const parts = []
  if (row?.removeExistingHeader) parts.push('页眉删除')
  else if (row?.existingHeaderArtifact) parts.push('页眉可编辑')
  else if (row?.existingHeaderText && row?.convertPlainHeader) parts.push('页眉转换')
  else if (row?.existingHeaderText) parts.push('页眉可转换')
  else parts.push('页眉新增')
  if (row?.removeExistingFooter) parts.push('页脚删除')
  else if (row?.existingFooterArtifact) parts.push('页脚可编辑')
  else if (row?.existingFooterText && row?.convertPlainFooter) parts.push('页脚转换')
  else if (row?.existingFooterText) parts.push('页脚可转换')
  else parts.push('页脚新增')
  return parts.join(' / ')
}

function sourceRangeText(row) {
  if (!row.sourcePageStart || !row.sourcePageEnd) return '-'
  return `${row.sourcePageStart}-${row.sourcePageEnd}`
}

function sortOverlayFiles({ prop, order }) {
  if (!prop || !order) return
  const selectedPath = selectedOverlayFile.value?.path
  overlayFiles.value = sortByNatural(
    overlayFiles.value,
    (row, index) => overlaySortValue(row, prop, index),
    order,
  )
  if (selectedPath) {
    selectedOverlayIndex.value = Math.max(0, overlayFiles.value.findIndex((file) => file.path === selectedPath))
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

.split-name-options {
  margin-bottom: 10px;
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

.table-text {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  vertical-align: middle;
  white-space: nowrap;
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
  font-family: Arial, "PingFang SC", "Microsoft YaHei", sans-serif;
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

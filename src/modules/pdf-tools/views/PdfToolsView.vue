<template>
  <div class="pdf-tools-view" :class="{ 'is-file-dragging': pdfDragging }">
    <IncomingFilesNotice :batch="incoming.pending.value" :error="incoming.error.value" :receiving="incoming.receiving.value" :disabled="incoming.blocked.value" @retry="incoming.retry" @discard="incoming.discard" />
    <div v-if="pdfDragging" class="pdf-drop-overlay">
      <div class="pdf-drop-message">{{ activeTab === 'merge' ? '松开以添加 PDF、Word 或图片' : '松开以添加 PDF 文件' }}</div>
    </div>
    <el-tabs v-model="activeTab" v-motion-tabs="activeTab" :tab-position="tabPosition" class="pdf-tabs">
      <el-tab-pane label="PDF 解锁" name="unlock" lazy>
        <ToolWorkspaceShell title="PDF 解锁" description="移除 PDF 文件的密码保护，原文件旁会生成已解锁副本。" :hide-toolbar="!unlockFiles.length">
          <template #toolbar>
            <el-button v-if="unlockFiles.length" type="primary" plain :disabled="unlocking || unlockInspecting" @click="selectUnlockFiles">添加 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            fill
            scene="pdf-unlock"
            :celebrating="unlockCelebrating"
            empty-action-label="添加 PDF 文件"
            :items="unlockFiles"
            :busy="unlocking || unlockInspecting"
            empty-text="选择一个或多个需要解锁的 PDF 文件"
            @empty-action="selectUnlockFiles"
            @clear="clearUnlockFiles"
            @remove="removeUnlockFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <el-button v-if="item.outputPath" link type="primary" size="small" @click="openPath(item.outputPath)">打开文件</el-button>
              <FileContinueMenu v-if="item.outputPath" :paths="[item.outputPath]" source="文档工具" size="small" />
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

      <el-tab-pane label="PDF 合并" name="merge" lazy>
        <ToolWorkspaceShell
          title="PDF 合并"
          :hide-toolbar="!mergeFiles.length"
          description="按列表顺序合并 PDF、Word 或图片；Word 自动转 PDF，图片先预览再排入 A4，可调整方向、缩放、裁剪和每页排列；合并时会压平页面注释和电子签章外观以便打印，不保留签章证书和交互。"
        >
          <template #toolbar>
            <el-button v-if="mergeFiles.length" type="primary" plain :disabled="merging" @click="selectMergeFiles">添加文件</el-button>
            <el-button v-if="mergeFiles.length > 1" :disabled="merging" @click="selectMergeOutputDir">{{ mergeOutputDir ? '更改输出目录' : '选择输出目录' }}</el-button>
            <el-button v-if="mergeOutputDir" text class="merge-directory" :title="mergeOutputDir" :disabled="merging" @click="selectMergeOutputDir">{{ mergeOutputDir }}</el-button>
          </template>
          <div v-if="mergeFiles.length > 1" class="merge-output-settings">
            <label>合并文件名</label><MergeFileNameField v-model="mergeFileName" :files="mergeFiles" :disabled="merging" />
          </div>
          <FileQueuePanel
            fill
            scene="pdf-merge"
            :celebrating="mergeCelebrating"
            empty-action-label="添加文件"
            :items="mergeFiles"
            :busy="merging"
            sortable
            empty-text="添加 PDF、Word 或图片（JPG / PNG / WebP / BMP）"
            @empty-action="selectMergeFiles"
            @clear="clearMergeFiles"
            @remove="removeMergeFile"
            @reorder="reorderMergeFiles"
          >
            <template #meta="{ item }">
              <el-tag :type="isWordPath(item.path) ? 'warning' : 'info'" size="small">
                {{ isWordPath(item.path) ? 'Word' : documentKind(item.path) === 'image' ? '图片' : 'PDF' }}
              </el-tag>
              <span v-if="item.statusText" class="merge-item-status">{{ item.statusText }}</span>
              <span v-if="item.conversionError" class="merge-conversion-error" role="status">{{ item.conversionError }}</span>
              <span v-if="item.manualPdf" class="merge-item-status">使用：{{ fileName(item.outputPath) }}</span>
              <el-button v-if="item.outputPath" link type="primary" size="small" @click="openPath(item.outputPath)">打开文件</el-button>
              <FileContinueMenu v-if="item.outputPath" :paths="[item.outputPath]" source="文档工具" size="small" />
            </template>
            <template #item-actions="{ item, index }">
              <el-button
                v-if="!isPdfPath(item.path) && !item.outputPath"
                link
                size="small"
                type="primary"
                :disabled="merging"
                @click="convertSingleMergeFile(item)"
              >
                转为 PDF
              </el-button>
              <el-button v-if="item.conversionError" :disabled="merging" link size="small" type="primary" @click="selectManualMergePdf(item)">选择手动转换的 PDF</el-button>
              <el-button :disabled="merging" link size="small" type="danger" @click="removeMergeFile(index)">
                移除
              </el-button>
            </template>
          </FileQueuePanel>
          <template #actions>
            <div class="merge-options">
              <el-checkbox v-model="duplexSeparate" :disabled="merging">双面打印分隔模式</el-checkbox>
              <HelpTooltip label="双面打印分隔" content="文件页数为奇数时在末尾补一页空白，让每份文件独立占满双面打印的整张纸，起印页始终在正面。" />
            </div>
            <el-button
              type="success"
              @click="doMerge"
              :loading="merging"
              :disabled="mergeFiles.length === 0 || (mergeFiles.length === 1 && isPdfPath(mergeFiles[0].path))"
            >
              {{ mergeFiles.length === 1 && !isPdfPath(mergeFiles[0].path) ? '转换为 PDF' : '合并为一个 PDF' }}
            </el-button>
            <el-button v-if="mergeOutputPath" @click="openPath(mergeOutputPath)">打开文件</el-button>
            <FileContinueMenu v-if="mergeOutputPath" :paths="[mergeOutputPath]" source="PDF 合并" exclude="pdf-merge" />
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="裁剪 PDF 页面" name="crop" lazy>
        <PdfCropWorkspace ref="cropWorkspace" />
      </el-tab-pane>

      <el-tab-pane label="提取 PDF 页面" name="extract" lazy>
        <ToolWorkspaceShell
          title="提取 PDF 页面"
          description="从一个 PDF 中挑选若干页导出，支持输入 3,7,12-15 这样的页码。"
        >
          <template #toolbar>
            <el-button type="primary" plain :disabled="extractingPages" @click="selectExtractFile">选择 PDF 文件</el-button>
            <el-button :disabled="!extractFile || extractingPages" @click="selectExtractOutputDir">选择输出文件夹</el-button>
          </template>
          <div v-if="extractFile" class="path-line"><span class="path-badge">源文件</span><span>{{ extractFile }}</span></div>
          <div v-if="extractOutputDir" class="path-line"><span class="path-badge">输出目录</span><span>{{ extractOutputDir }}</span></div>
          <div v-if="extractFile" class="simple-tool-form">
            <el-input v-model="extractPageText" :disabled="extractingPages" placeholder="例如：3,7,12-15" clearable @keyup.enter="doExtractPages">
              <template #prepend>页码</template>
            </el-input>
            <div class="path-hint">共 {{ extractTotalPages || '-' }} 页；重复页会自动忽略。</div>

          </div>
          <WorkspaceEmptyState
            v-else
            scene="pdf-extract"
            :celebrating="extractCelebrating"
            class="extract-empty"
            title="等待选择 PDF"
            description="选择文件后，可以输入单页或连续页码范围并导出。"
          >
            <template #actions>
              <el-button type="primary" plain :disabled="extractingPages" @click="selectExtractFile">
                选择 PDF 文件
              </el-button>
            </template>
          </WorkspaceEmptyState>
          <template #actions>
            <el-button
              type="success"
              :loading="extractingPages"
              :disabled="!extractFile || !extractTotalPages || !extractPageText.trim()"
              @click="doExtractPages"
            >
              导出选中页面
            </el-button>
            <el-button v-if="extractResultPath" @click="openPath(extractResultPath)">打开文件</el-button>
            <FileContinueMenu v-if="extractResultPath" :paths="[extractResultPath]" source="页面提取" exclude="pdf-extract" />
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="PDF 压缩" name="compress" lazy>
        <ToolWorkspaceShell
          title="PDF 压缩整理"
          :hide-toolbar="!compressFiles.length"
          description="默认只做无损结构整理；需要进一步压缩图片时再单独开启，扫描件不会被默认重编码。"
        >
          <template #toolbar>
            <el-button v-if="compressFiles.length" type="primary" plain :disabled="compressing" @click="selectCompressFiles">添加 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            fill
            scene="pdf-compress"
            :celebrating="compressCelebrating"
            empty-action-label="添加 PDF 文件"
            :items="compressFiles"
            :busy="compressing"
            empty-text="添加或拖入 PDF，调整下方设置后点击“开始压缩”；结果保存在源文件夹"
            @empty-action="selectCompressFiles"
            @clear="clearCompressFiles"
            @remove="removeCompressFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <el-button v-if="item.outputPath" link type="primary" size="small" @click="openPath(item.outputPath)">打开文件</el-button>
              <FileContinueMenu v-if="item.outputPath" :paths="[item.outputPath]" source="文档工具" size="small" />
              <span v-if="item.status === 'done' && item.inputSize > 0">{{
                sizeSavingText(item.inputSize, item.outputSize)
              }}</span>
            </template>
          </FileQueuePanel>
          <div v-if="compressSummary" class="path-line path-line--summary">{{ compressSummary }}</div>
          <template #actions>
            <div class="compress-options">

              <el-checkbox v-model="compressImageReencode" :disabled="compressing"> 进一步压缩图片（可能耗时较长） </el-checkbox>
              <div v-if="compressImageReencode" class="compress-level-row">
                <span class="compress-level-label">图片压缩级别：</span>
                <el-radio-group v-model="compressLevel" :disabled="compressing" size="default">
                  <el-radio-button :value="1">清晰优先</el-radio-button>
                  <el-radio-button :value="2">均衡</el-radio-button>
                  <el-radio-button :value="3">体积最小</el-radio-button>
                </el-radio-group>
              </div>
            </div>
            <el-button type="success" :loading="compressing" :disabled="!compressFiles.some(file => ['pending', 'failed', 'cancelled'].includes(file.status))" @click="runCompressQueue">开始压缩</el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="PDF 拆分" name="split" lazy>
        <ToolWorkspaceShell title="PDF 拆分" description="翻页核对内容，按页码范围生成多个独立 PDF 文件。">
          <template #toolbar>
            <el-button type="primary" plain :disabled="splittingMerged" @click="selectSplitFile">选择 PDF 文件</el-button>
            <el-button :disabled="!splitFile || splittingMerged" @click="selectSplitOutputDir">选择输出文件夹</el-button>
            <el-button :disabled="!splitFile || splittingMerged" @click="addSplitRange">添加页段</el-button>
            <ClearFileListButton v-if="splitFile" :disabled="splittingMerged" @clear="clearSplitFile" />
          </template>
          <div v-if="splitFile" class="path-line"><span class="path-badge">源文件</span><span>{{ splitFile }}</span></div>
          <div v-if="splitOutputDir" class="path-line"><span class="path-badge">输出目录</span><span>{{ splitOutputDir }}</span></div>

          <div v-if="splitFile" ref="splitMainRef" :inert="splittingMerged" class="split-main" :style="splitGridStyle">
            <section class="split-list-panel">
              <div class="split-options-row">
                <el-checkbox v-model="removeBlankPages">删除空白页</el-checkbox><HelpTooltip content="勾选后移除确认空白的页面。Docsy 分隔页带专属标记；其他页面保守检查，含批注、表单或无法解析内容的页会保留。默认不删除。" />
                <span class="split-option-note">含批注或无法确认空白的页会保留。</span>
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
                <el-table-column label="起始页" width="120">
                  <template #default="{ row }">
                    <el-input-number
                      v-model="row.pageStart"
                      :min="1"
                      :max="splitPreviewMaxPage"
                      size="small"
                      controls-position="right"
                      style="width: 96px"
                    />
                  </template>
                </el-table-column>
                <el-table-column label="结束页" width="120">
                  <template #default="{ row }">
                    <el-input-number
                      v-model="row.pageEnd"
                      :min="1"
                      :max="splitPreviewMaxPage"
                      size="small"
                      controls-position="right"
                      style="width: 96px"
                    />
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
                    <el-button link type="danger" size="small" @click.stop="removeSplitRange($index)">移除</el-button>
                  </template>
                </el-table-column>
              </el-table>

            </section>

            <div
              class="split-resizer"
              role="separator"
              aria-label="调整页段列表与预览宽度"
              aria-orientation="vertical"
              @pointerdown="startSplitResize"
            />

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
                <div class="split-preview-actions action-row">
<div class="control-cluster" role="group" aria-label="预览翻页">
<span class="toolbar-caption">浏览</span><el-button-group>
<el-button size="small" :disabled="splitPreviewPage <= 1" @click="moveSplitPreviewPage(-1)"
                    >上一页</el-button
                  ><el-button
                    size="small"
                    :disabled="splitPreviewPage >= splitPreviewMaxPage"
                    @click="moveSplitPreviewPage(1)"
                    >下一页</el-button
                  >
</el-button-group><el-input-number v-model="splitPreviewPage" :min="1" :max="splitPreviewMaxPage" size="small" aria-label="预览页码" />
</div>
<div class="control-cluster" role="group" aria-label="拆分页段">
<span class="toolbar-caption">拆分</span><el-button-group>
<el-button size="small" type="primary" plain @click="splitFromCurrentPage">从本页拆</el-button><el-button
                    size="small"
                    type="primary"
                    plain
                    :disabled="splitPreviewPage <= 1"
                    @click="splitFromPreviousPage"
                    >从上一页拆</el-button
                  >
</el-button-group>
</div>
<div class="control-cluster" role="group" aria-label="设置页段边界">
<el-button-group>
<el-button size="small" :disabled="!selectedSplitRange" @click="setSelectedSplitStart"
                    >设为起始页</el-button
                  ><el-button size="small" :disabled="!selectedSplitRange" @click="setSelectedSplitEnd"
                    >设为结束页</el-button
                  >
</el-button-group>
</div>
<el-button
                    size="small"
                    :disabled="!showSplitNextPreview && splitPreviewPage >= splitPreviewMaxPage"
                    :aria-pressed="showSplitNextPreview"
                    @click="showSplitNextPreview = !showSplitNextPreview"
                  >
                    {{ showSplitNextPreview ? '隐藏下一页预览' : '显示下一页预览' }}
                  </el-button>
                </div>
              </div>
              <div class="split-preview-pages" :class="{ 'has-next-preview': showSplitNextPreview }">
                <PdfJsPreview
                  v-if="activeTab === 'split' && splitFile"
                  class="split-primary-preview"
                  :file-path="splitFile"
                  :page="splitPreviewPage"
                  :scale="0.9"
                  @error="(message) => ElMessage.error(message)"
                />
                <NextPageThumbnail
                  v-if="showSplitNextPreview && activeTab === 'split' && splitFile"
                  :file-path="splitFile"
                  :page="splitPreviewPage"
                  :max-page="splitPreviewMaxPage"
                  @select="(page) => (splitPreviewPage = page)"
                  @error="(message) => ElMessage.error(message)"
                />
              </div>
            </section>
          </div>
          <WorkspaceEmptyState
            v-else
            class="split-empty"
            title="等待选择 PDF"
            description="选择文件后，可以翻页预览并设置每个拆分文件的起止页。"
          />
          <template #actions>
              <el-button
                type="success"
                @click="doSplitMerged"
                :loading="splittingMerged"
                :disabled="!splitRanges.length || !splitFile || !splitOutputDir || splitWarnings.length > 0"
              >
                执行拆分
              </el-button>
            <el-button v-if="splitResultDir" @click="openPath(splitResultDir)">打开文件夹</el-button>
            <FileContinueMenu :paths="splitResultPaths" source="PDF 拆分" exclude="pdf-split" :disabled="splittingMerged" />
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>

      <el-tab-pane label="PDF 防复制" name="anti-ocr" lazy>
        <ToolWorkspaceShell
          title="PDF 防复制"
          :hide-toolbar="!antiOcrFiles.length"
          description="防止 PDF 文字被复制提取。保留视觉效果，干扰文字选择和复制。"
        >
          <template #toolbar>
            <el-button v-if="antiOcrFiles.length" type="primary" plain :disabled="antiOcrProcessing" @click="selectAntiOcrFiles">添加 PDF 文件</el-button>
          </template>
          <FileQueuePanel
            fill
            scene="pdf-anticopy"
            :celebrating="antiOcrCelebrating"
            empty-action-label="添加 PDF 文件"
            :items="antiOcrFiles"
            :busy="antiOcrProcessing"
            empty-text="选择一个或多个 PDF 文件进行防复制检测"
            @empty-action="selectAntiOcrFiles"
            @clear="clearAntiOcrFiles"
            @remove="removeAntiOcrFile"
          >
            <template #meta="{ item }">
              <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
              <el-tag v-if="item.hasAntiOcr" type="warning" size="small">已防护</el-tag>
              <el-button v-if="item.outputPath" link type="primary" @click="openPath(item.outputPath)">打开文件</el-button>
              <FileContinueMenu v-if="item.outputPath" :paths="[item.outputPath]" source="文档工具" size="small" />
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

      <el-tab-pane label="长图处理" name="long-slice" lazy>
        <ToolWorkspaceShell
          title="长图处理"
          :hide-toolbar="!longSliceFiles.length"
          description="将网页长截图、手机长截屏及长图 PDF 智能分页切片，按标准 A4 纸张排版生成易打印文档。"
        >
          <template #toolbar>
            <el-button v-if="longSliceFiles.length" type="primary" plain :disabled="longSliceProcessing" @click="selectLongSliceFiles">添加文件</el-button>
            <el-button v-if="longSliceFiles.length" :disabled="longSliceProcessing" @click="selectLongSliceOutputDir">
              {{ longSliceOutputDir ? '更改输出目录' : '选择输出目录' }}
            </el-button>
            <el-button
              v-if="longSliceEffectiveOutputDir"
              text
              class="merge-directory"
              :title="longSliceEffectiveOutputDir"
              :disabled="longSliceProcessing"
              @click="selectLongSliceOutputDir"
            >{{ longSliceEffectiveOutputDir }}</el-button>
          </template>

          <div class="long-slice-container" :class="{ 'has-files': longSliceFiles.length }">
            <FileQueuePanel
              class="long-slice-queue"
              :fill="!longSliceFiles.length"
              max-height="min(28vh, 260px)"
              scene="pdf-long-slice"
              :celebrating="longSliceCelebrating"
              empty-action-label="选择长图或文档"
              :items="longSliceFiles"
              :busy="longSliceProcessing"
              selectable
              :selected-index="selectedLongSliceIndex"
              empty-text="选择或拖入长截图图片（PNG/JPG/WebP/BMP）或长图 PDF"
              @empty-action="selectLongSliceFiles"
              @clear="clearLongSliceFiles"
              @remove="removeLongSliceFile"
              @select="onSelectLongSliceItem"
            >
              <template #meta="{ item }">
                <el-tag v-if="item.dimText" size="small" type="info">{{ item.dimText }}</el-tag>
                <el-tag :type="item.statusType" size="small">{{ item.statusText }}</el-tag>
                <el-button v-if="item.outputPath" link type="primary" @click="openPath(item.outputPath)">打开文件</el-button>
              <FileContinueMenu v-if="item.outputPath" :paths="[item.outputPath]" source="文档工具" size="small" />
              </template>
            </FileQueuePanel>

            <div v-if="longSliceFiles.length" class="long-slice-preview-card" v-loading="previewLoading">
                <div class="preview-header">
                  <span class="preview-title">A4 打印预览</span>
                  <el-checkbox v-if="currentLongSlicePreview && currentLongSlicePreview.totalPages > 1" v-model="compareNextSlice" @change="refreshPreview">对照下一页</el-checkbox>
                  <div class="preview-pagination" v-if="currentLongSlicePreview && currentLongSlicePreview.totalPages > 1">
                    <el-button
                      size="small"
                      circle
                      :disabled="currentPreviewPage <= 1"
                      @click="changePreviewPage(-1)"
                    >&lt;</el-button>
                    <span class="page-indicator">{{ currentPreviewPage }} / {{ currentLongSlicePreview.totalPages }}</span>
                    <el-button
                      size="small"
                      circle
                      :disabled="currentPreviewPage >= currentLongSlicePreview.totalPages"
                      @click="changePreviewPage(1)"
                    >&gt;</el-button>
                  </div>
                </div>

                <div class="a4-sheet-viewport" :class="{ 'is-comparing': compareNextSlice && nextLongSlicePreview }">
                  <div class="a4-sheet-canvas" :class="[longSliceOrientation]">
                    <img
                      v-if="currentLongSlicePreview && currentLongSlicePreview.previewDataUrl"
                      :src="currentLongSlicePreview.previewDataUrl"
                      class="preview-image"
                      alt="A4 Preview"
                    />
                    <div v-else class="preview-placeholder">正在生成预览…</div>
                  </div>
                  <div v-if="compareNextSlice && nextLongSlicePreview" class="a4-sheet-canvas" :class="[longSliceOrientation]">
                    <img :src="nextLongSlicePreview.previewDataUrl" class="preview-image" alt="下一页打印预览" />
                  </div>
                </div>

                <div class="preview-footer-info" v-if="currentLongSlicePreview">
                  <span v-if="currentLongSlicePreview.isPdf && currentLongSlicePreview.sourcePageCount > 1">
                    PDF 来源第 {{ currentLongSlicePreview.sourcePageIndex }} / {{ currentLongSlicePreview.sourcePageCount }} 页
                  </span>
                  <span v-if="currentLongSlicePreview.isPdf && currentLongSlicePreview.sourcePageCount > 1">·</span>
                  <span>{{ currentLongSlicePreview.sourceWidth }} × {{ currentLongSlicePreview.sourceHeight }} px</span>
                  <span>·</span>
                  <span>预计 {{ currentLongSlicePreview.totalPages }} 页</span>
                </div>
              </div>

              <div class="long-slice-options-panel">
                <el-form label-position="top" size="small">
                  <el-form-item label="纸张方向与留白">
                    <div class="opt-row">
                      <el-radio-group v-model="longSliceOrientation" @change="schedulePreviewRefresh">
                        <el-radio-button label="portrait">纵向 (A4)</el-radio-button>
                        <el-radio-button label="landscape">横向 (A4)</el-radio-button>
                      </el-radio-group>
                      <el-select v-model="longSliceMarginMm" style="width: 130px" @change="schedulePreviewRefresh">
                        <el-option label="无留白 (0mm)" :value="0" />
                        <el-option label="窄边距 (5mm)" :value="5" />
                        <el-option label="标准边距 (10mm)" :value="10" />
                        <el-option label="装订边距 (15mm)" :value="15" />
                      </el-select>
                    </div>
                  </el-form-item>

                  <el-form-item label="内容宽度与对齐">
                    <div class="opt-row">
                      <el-radio-group v-model="longSliceWidthMode" @change="schedulePreviewRefresh">
                        <el-radio-button label="fit">满版宽度</el-radio-button>
                        <el-radio-button label="phone">手机适中 (55%)</el-radio-button>
                        <el-radio-button label="custom">自定义</el-radio-button>
                      </el-radio-group>
                      <el-radio-group v-model="longSliceAlignment" @change="schedulePreviewRefresh">
                        <el-radio-button label="left">靠左</el-radio-button>
                        <el-radio-button label="center">居中</el-radio-button>
                        <el-radio-button label="right">靠右</el-radio-button>
                      </el-radio-group>
                    </div>
                    <div v-if="longSliceWidthMode === 'custom'" class="slider-row">
                      <el-slider v-model="longSliceCustomWidth" :min="40" :max="100" @change="schedulePreviewRefresh" />
                      <span class="slider-val">{{ longSliceCustomWidth }}%</span>
                    </div>
                  </el-form-item>

                  <el-form-item label="切片防腰斩与重叠">
                    <div class="opt-row align-center">
                      <el-checkbox v-model="longSliceSmartCut" @change="schedulePreviewRefresh">
                        智能防腰斩 (自动寻找空白缝隙)
                      </el-checkbox>
                      <div class="overlap-box">
                        <span>相邻重叠:</span>
                        <el-input-number v-model="longSliceOverlapMm" :min="0" :max="15" :step="1" style="width: 80px" @change="schedulePreviewRefresh" />
                        <span>mm</span>
                      </div>
                    </div>
                    <div class="long-slice-note">重叠内容会同时出现在上一页末尾和下一页开头；照片或无明显空白的页面仍可能被切开，请翻页核对。</div>
                  </el-form-item>

                  <div class="header-footer-settings">
                    <div class="settings-group-title">页眉与页脚设置</div>
                      <div class="collapse-block">
                        <div class="sub-heading">
                          <el-checkbox v-model="longSliceHeaderEnabled" @change="schedulePreviewRefresh">开启页眉</el-checkbox>
                        </div>
                        <div v-if="longSliceHeaderEnabled" class="sub-fields">
                          <el-input
                            v-model="longSliceHeaderTitle"
                            placeholder="网页标题或来源网址（可选）"
                            clearable
                            size="small"
                            @change="schedulePreviewRefresh"
                          />
                          <div class="opt-row mt-2">
                            <el-checkbox v-model="longSliceHeaderDate" @change="schedulePreviewRefresh">打印日期</el-checkbox>
                            <el-checkbox v-model="longSliceHeaderDatePrefix" :disabled="!longSliceHeaderDate" @change="schedulePreviewRefresh">
                              带“打印日期：”前缀
                            </el-checkbox>
                          </div>
                        </div>
                      </div>

                      <div class="collapse-block mt-3">
                        <div class="sub-heading">
                          <el-checkbox v-model="longSliceFooterEnabled" @change="schedulePreviewRefresh">开启页脚页码</el-checkbox>
                        </div>
                        <div v-if="longSliceFooterEnabled" class="sub-fields">
                          <label class="field-label" for="long-slice-footer-template">页码模板</label>
                          <el-select id="long-slice-footer-template" v-model="longSliceFooterNumberStyle" style="width: 100%" @change="schedulePreviewRefresh">
                            <el-option v-for="preset in longSliceFooterPresetOptions" :key="preset.value" :label="preset.label" :value="preset.value" />
                          </el-select>
                          <label class="field-label" for="long-slice-footer-digits">数字形式</label>
                          <el-select id="long-slice-footer-digits" v-model="longSliceFooterDigitStyle" style="width: 100%" @change="schedulePreviewRefresh">
                            <el-option v-for="style in PAGE_NUMBER_STYLES" :key="style.value" :label="`${style.label}（${style.sample}）`" :value="style.value" />
                          </el-select>
                          <el-checkbox v-model="longSliceFooterShowTotal" class="footer-total-toggle" @change="onLongSliceFooterShowTotalChange">显示总页数</el-checkbox>
                          <div class="footer-format-preview">示例：{{ longSliceFooterPreview }}</div>
                        </div>
                      </div>
                  </div>
                </el-form>
              </div>
          </div>

          <template #actions>
            <el-button
              type="success"
              @click="batchProcessLongSlice"
              :loading="longSliceProcessing"
              :disabled="longSliceFiles.length === 0"
            >
              切片导出 {{ longSliceFiles.length }} 个文件
            </el-button>
          </template>
        </ToolWorkspaceShell>
      </el-tab-pane>
    </el-tabs>
    <ImagePdfImportDialog ref="imageImportDialog" />
  </div>
</template>

<script setup>
import ImagePdfImportDialog from '../../../shared/pdf-tools/components/ImagePdfImportDialog.vue'
import PdfCropWorkspace from '../components/PdfCropWorkspace.vue'
import FileContinueMenu from '../../../shared/components/FileContinueMenu.vue'
import ClearFileListButton from '../../../shared/components/ClearFileListButton.vue'
import IncomingFilesNotice from '../../../shared/components/IncomingFilesNotice.vue'
import { useIncomingFiles } from '../../../core/composables/useIncomingFiles.js'
import { FILE_TRANSFER_TARGETS } from '../../../core/fileTransfers.js'
const imageImportDialog = ref(null)
import { useResponsiveTabs } from '../../../shared/composables/useResponsiveTabs.js'
import HelpTooltip from '../../../shared/components/HelpTooltip.vue'
import MergeFileNameField from '../../../shared/pdf-tools/components/MergeFileNameField.vue'
import { commonSourceDirectory, pdfOutputFileName, joinOutputPath } from '../../../core/outputFiles.js'
import { DOCUMENT_FILTERS, MANUAL_PDF_HINT, convertImportDocument, documentKind, reviewImageImports } from '../../../shared/pdf-tools/documentImport.js'
import { computed, getCurrentInstance, nextTick, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useQueueCancellation } from '../../../core/composables/useQueueCancellation.js'
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus'
import { Rank } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import PdfJsPreview from '../../../shared/pdf-tools/components/PdfJsPreview.vue'
import NextPageThumbnail from '../../../shared/pdf-tools/components/NextPageThumbnail.vue'
import FileQueuePanel from '../../../shared/components/FileQueuePanel.vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import { withMinimumDuration, createCelebrationTrigger } from '../../../shared/animation.js'
import { splitRangeWarnings } from '../../../shared/pdf-tools/composables/usePdfSplitRanges.js'
import { pageNumberPresets, templateForTotal, usesPageTotal } from '../../../shared/pdf-tools/pageNumberFormats.js'
import { PAGE_NUMBER_STYLES, renderPageNumberTemplate } from '../../../shared/pdf-tools/composables/pdfPageNumberRules.js'
import { sizeSavingText, batchSummaryText } from '../../../shared/pdf-tools/composables/pdfLosslessOptimize.js'
import { getPdfPageCount, openPath, tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'
import { fileName, parentDir, stripPdf } from '../../../core/filePath.js'
import { useWindowFileDrop } from '../../../core/composables/useWindowFileDrop.js'
import { usePointerReorder } from '../../../core/composables/usePointerReorder.js'
import { useWorkspacePreferences } from '../../../core/composables/useWorkspacePreferences.js'
import {
  buildRangeAfter,
  insertRangeAtPage,
  insertRangeAfter,
  navigatePage,
  parsePageSelection,
  setRangeEnd,
  setRangeStart,
} from '../../../core/pdfUtils.js'

const tabPosition = useResponsiveTabs()

const PDF_TABS = new Set(['unlock', 'merge', 'crop', 'extract', 'compress', 'split', 'anti-ocr', 'long-slice'])
const cropWorkspace = ref(null)
const currentInstance = getCurrentInstance()
const route = currentInstance ? useRoute() : null
const router = currentInstance ? useRouter() : null
const tabFromRoute = (tab) => (PDF_TABS.has(tab) ? tab : 'unlock')
const activeTab = ref(tabFromRoute(route?.params?.tab))
onActivated(() => {
  if (route?.name !== 'pdf-tools') return
  if (PDF_TABS.has(route.params.tab)) activeTab.value = route.params.tab
  else if (activeTab.value !== 'unlock') router.replace({ name: 'pdf-tools', params: { tab: activeTab.value } })
})
onDeactivated(() => { stopSplitResize?.() })

watch(
  () => [route?.name, route?.params?.tab],
  ([, tab]) => {
    if (route?.name !== 'pdf-tools') return
    if (!tab && activeTab.value !== 'unlock') {
      router.replace({ name: 'pdf-tools', params: { tab: activeTab.value } })
      return
    }
    const next = tabFromRoute(tab)
    if (next !== activeTab.value) activeTab.value = next
  },
  { flush: 'post' },
)

watch(activeTab, (tab) => {
  if (!route || !router) return
  if (route.name !== 'pdf-tools') return
  const param = tab === 'unlock' ? undefined : tab
  if (route.params.tab === param) return
  router.replace({ name: 'pdf-tools', params: param ? { tab: param } : {} })
})
const pdfDragging = ref(false)
const unlockCelebrating = ref(false)
const mergeCelebrating = ref(false)
const extractCelebrating = ref(false)
const compressCelebrating = ref(false)
const antiOcrCelebrating = ref(false)
const longSliceCelebrating = ref(false)

const triggerUnlockCelebration = createCelebrationTrigger(unlockCelebrating)
const triggerMergeCelebration = createCelebrationTrigger(mergeCelebrating)
const triggerExtractCelebration = createCelebrationTrigger(extractCelebrating)
const triggerCompressCelebration = createCelebrationTrigger(compressCelebrating)
const triggerAntiOcrCelebration = createCelebrationTrigger(antiOcrCelebrating)
const triggerLongSliceCelebration = createCelebrationTrigger(longSliceCelebrating)
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
    addUnlockFiles(paths)
  }
}

function addUnlockFiles(paths) {
  if (unlocking.value || unlockInspecting.value) return
  const existing = new Set(unlockFiles.value.map((file) => file.path))
  const candidates = [...new Set(paths)].filter((path) => isPdfPath(path) && !existing.has(path))
  const items = makeQueueItems(candidates).map((item) => ({
    ...item,
    encrypted: null,
    inspecting: true,
    statusText: '检测中',
  }))
  unlockFiles.value = [...unlockFiles.value, ...items]
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
        let timer
        const result = await Promise.race([
          tauriCallSafe('inspect_pdf', { input: path }),
          new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error('检测超时（10秒）')), 10000)
          }),
        ]).finally(() => clearTimeout(timer))
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
  if (unlocking.value || unlockInspecting.value) return
  unlockFiles.value = []
}

function removeUnlockFile(index) {
  if (unlocking.value || unlockInspecting.value) return
  unlockFiles.value.splice(index, 1)
}

async function batchUnlock() {
  if (unlocking.value || unlockInspecting.value) return
  const encryptedFiles = unlockFiles.value.filter((file) => file.encrypted === true)
  if (!encryptedFiles.length) {
    ElMessage.info('所选文件均未加密，无需处理')
    return
  }
  unlocking.value = true
  unlockCancellation.reset()
  let successCount = 0
  try {
    for (const file of encryptedFiles) {
      if (unlockCancellation.cancelled.value) break
      file.statusText = '处理中'
      file.statusType = 'warning'
      const result = await withMinimumDuration(
        () => tauriCallSafe('unlock_pdf', { input: file.path }),
        450,
      )
      if (result.ok && !result.data.skipped) {
        file.outputPath = result.data.output_path
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
        if (/操作已取消/.test(String(result.error))) unlockCancellation.cancel()
      }
    }
  } finally {
    unlocking.value = false
  }
  if (successCount) {
    triggerUnlockCelebration()
    ElMessage.success(`已解锁 ${successCount} 个文件`)
  }
}

const unlockCancellation = useQueueCancellation()
const compressCancellation = useQueueCancellation()
const antiCopyCancellation = useQueueCancellation()
const mergeFiles = ref([])
const merging = ref(false)
const mergeOutputPath = ref('')
const mergeOutputDir = ref('')
const mergeOutputDirManual = ref(false)
const mergeFileName = ref('合并文件.pdf')
function syncMergeOutputDirectory() {
  if (!mergeOutputDirManual.value) mergeOutputDir.value = commonSourceDirectory(mergeFiles.value)
}
const extractResultPath = ref('')
const splitResultDir = ref('')
const splitResultPaths = ref([])
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
const splitPreviewRatio = ref(0.42)
const showSplitNextPreview = ref(false)
const splitMainRef = ref(null)
let stopSplitResize = null
const splitWarnings = computed(() => [
  ...splitRangeWarnings(splitRanges.value, splitTotalPages.value),
  ...splitRunWarnings.value,
])
const splitPreviewMaxPage = computed(() => Math.max(1, splitTotalPages.value || 1))
const selectedSplitRange = computed(() => splitRanges.value[selectedSplitRangeIndex.value] || null)
const splitGridStyle = computed(() => {
  const preview = Math.round(clampSplitPreviewRatio(splitPreviewRatio.value) * 100)
  return { gridTemplateColumns: `minmax(360px, ${100 - preview}fr) 10px minmax(320px, ${preview}fr)` }
})

// Anti-OCR
const antiOcrFiles = ref([])
const antiOcrProcessing = ref(false)
const antiCopyMethod = ref('cmap_scramble')
const preference = useWorkspacePreferences('pdf-tools.workspace', {
  activeTab,
  duplexSeparate,
  compressImageReencode,
  compressLevel,
  removeBlankPages,
  antiCopyMethod,
  splitPreviewRatio,
  showSplitNextPreview,
})
const antiOcrReadyCount = computed(() => antiOcrFiles.value.filter((f) => f.hasAntiOcr === false).length)
const antiOcrProtectedCount = computed(() => antiOcrFiles.value.filter((f) => f.hasAntiOcr && f.hasBackup).length)

function addAntiOcrFiles(paths) {
  if (antiOcrProcessing.value) return
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
        let timer
        const result = await Promise.race([
          tauriCallSafe('detect_anti_copy', { input: path }),
          new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error('检测超时（10秒）')), 10000)
          }),
        ]).finally(() => clearTimeout(timer))
        const item = reactive.find((f) => f.path === path)
        if (!item) continue
        if (!result.ok) {
          item.statusText = result.error || '检测失败'
          item.statusType = 'danger'
        } else {
          item.hasAntiOcr = result.data.has_protection
          item.hasBackup = result.data.has_backup
          const { total_fonts, protected_fonts } = result.data
          if (result.data.has_protection) {
            item.statusText = `已防护（${protected_fonts}/${total_fonts} 个字体${item.hasBackup ? '' : '，无恢复备份'}）`
            item.statusType = 'warning'
          } else {
            item.statusText = `正常（${total_fonts} 个字体）`
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
  if (antiOcrProcessing.value) return
  antiOcrFiles.value = []
}
function removeAntiOcrFile(index) {
  if (antiOcrProcessing.value) return
  antiOcrFiles.value.splice(index, 1)
}

async function batchAntiOcrApply() {
  await processAntiCopy(false)
}

async function batchAntiOcrRemove() {
  await processAntiCopy(true)
}

async function processAntiCopy(removeProtection) {
  if (antiOcrProcessing.value) return
  antiOcrProcessing.value = true
  antiCopyCancellation.reset()
  const method = antiCopyMethod.value
  const targets = antiOcrFiles.value.filter(file => removeProtection
    ? file.hasAntiOcr && file.hasBackup
    : file.hasAntiOcr === false)
  try {
    for (const file of targets) {
      if (antiCopyCancellation.cancelled.value) break
      file.statusText = '处理中'
      file.statusType = 'warning'
      const input = file.outputPath || file.path
      const output = `${parentDir(file.path)}/${stripPdf(file.name)}_${removeProtection ? 'restored' : 'anti_copy'}.pdf`
      const result = await withMinimumDuration(
        () =>
          tauriCallSafe(removeProtection ? 'remove_anti_copy' : 'apply_anti_copy', {
            input,
            output,
            ...(removeProtection ? {} : { method }),
          }),
        500,
      )
      if (!result.ok) {
        file.statusText = userFacingError(result.error, '处理失败')
        file.statusType = 'danger'
        if (/操作已取消/.test(String(result.error))) antiCopyCancellation.cancel()
        continue
      }
      file.outputPath = result.data.output_path
      file.hasAntiOcr = result.data.detection.has_protection
      file.hasBackup = result.data.detection.has_backup
      file.statusText = removeProtection ? '已恢复到副本' : file.hasAntiOcr ? '已生成防复制副本' : '没有可处理的文字映射'
      file.statusType = file.hasAntiOcr ? 'warning' : 'success'
    }
  } finally {
    antiOcrProcessing.value = false
  }
  if (targets.length) {
    triggerAntiOcrCelebration()
  }
}

// ── Long Slice (长图处理) ──
const LONG_SLICE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp', 'bmp', 'pdf'])
const isLongSliceSupportedPath = (path) => {
  const ext = (String(path || '').split('.').pop() || '').toLowerCase()
  return LONG_SLICE_EXTENSIONS.has(ext)
}

const longSliceFiles = ref([])
const longSliceProcessing = ref(false)
const longSliceOutputDir = ref('')
const selectedLongSliceIndex = ref(0)
const currentLongSlicePreview = ref(null)
const nextLongSlicePreview = ref(null)
const compareNextSlice = ref(false)
const currentPreviewPage = ref(1)
const previewLoading = ref(false)
let longSlicePreviewRequestId = 0
let longSlicePreviewTimer = null
const longSliceEffectiveOutputDir = computed(() => {
  if (longSliceOutputDir.value) return longSliceOutputDir.value
  const current = longSliceFiles.value[selectedLongSliceIndex.value] || longSliceFiles.value[0]
  return parentDir(current?.path || '')
})

const longSliceOrientation = ref('portrait')
const longSliceMarginMm = ref(5)
const longSliceWidthMode = ref('fit')
const longSliceCustomWidth = ref(55)
const longSliceAlignment = ref('center')
const longSliceSmartCut = ref(true)
const longSliceOverlapMm = ref(0)
const longSliceHeaderEnabled = ref(false)
const longSliceHeaderTitle = ref('')
const longSliceHeaderDate = ref(true)
const longSliceHeaderDatePrefix = ref(true)
const longSliceFooterEnabled = ref(true)
const longSliceFooterShowTotal = ref(true)
const longSliceLastTemplateWithTotal = ref('')
const longSliceFooterPresets = computed(() => pageNumberPresets(longSliceFooterShowTotal.value))
const longSliceFooterNumberStyle = ref('第{page}页 / 共{total}页')
const longSliceFooterDigitStyle = ref('arabic')
const longSliceFooterPresetOptions = computed(() =>
  longSliceFooterPresets.value.map((preset) => ({
    ...preset,
    label: `${preset.label}：${renderPageNumberTemplate(preset.value, 1, 5, longSliceFooterDigitStyle.value)}`,
  })),
)
const longSliceFooterPreview = computed(() =>
  renderPageNumberTemplate(longSliceFooterNumberStyle.value, 1, 5, longSliceFooterDigitStyle.value),
)

function onLongSliceFooterShowTotalChange() {
  const current = longSliceFooterNumberStyle.value
  if (!longSliceFooterShowTotal.value && usesPageTotal(current)) {
    longSliceLastTemplateWithTotal.value = current
  }
  longSliceFooterNumberStyle.value = templateForTotal(
    current,
    longSliceFooterShowTotal.value,
    longSliceLastTemplateWithTotal.value,
  )
  if (longSliceFooterShowTotal.value && usesPageTotal(longSliceFooterNumberStyle.value)) {
    longSliceLastTemplateWithTotal.value = longSliceFooterNumberStyle.value
  }
  schedulePreviewRefresh()
}

function getLongSliceArgs(path, outputDir = null) {
  return {
    inputPath: path,
    outputDir: outputDir || longSliceOutputDir.value || null,
    outputFilename: null,
    orientation: longSliceOrientation.value,
    marginMm: Number(longSliceMarginMm.value) || 0,
    contentWidthMode: longSliceWidthMode.value,
    customWidthPercent: Number(longSliceCustomWidth.value) || 55,
    alignment: longSliceAlignment.value,
    smartCut: Boolean(longSliceSmartCut.value),
    overlapMm: Number(longSliceOverlapMm.value) || 0,
    headerEnabled: Boolean(longSliceHeaderEnabled.value),
    headerTitle: longSliceHeaderTitle.value?.trim() || null,
    headerDate: Boolean(longSliceHeaderDate.value),
    headerDatePrefix: Boolean(longSliceHeaderDatePrefix.value),
    footerEnabled: Boolean(longSliceFooterEnabled.value),
    footerPageNumberStyle: longSliceFooterNumberStyle.value,
    footerDigitStyle: longSliceFooterDigitStyle.value,
  }
}

async function refreshPreview() {
  clearTimeout(longSlicePreviewTimer)
  longSlicePreviewTimer = null
  const requestId = ++longSlicePreviewRequestId
  const current = longSliceFiles.value[selectedLongSliceIndex.value]
  if (!current) {
    currentLongSlicePreview.value = null
    previewLoading.value = false
    return
  }
  previewLoading.value = true
  nextLongSlicePreview.value = null
  try {
    const args = getLongSliceArgs(current.path)
    const res = await tauriCallSafe('preview_long_slice', {
      args,
      previewPage: currentPreviewPage.value,
    })
    if (requestId !== longSlicePreviewRequestId) return
    if (res.ok && res.data) {
      currentLongSlicePreview.value = res.data
      currentPreviewPage.value = res.data.previewPage || res.data.preview_page || 1
      const w = res.data.sourceWidth ?? res.data.source_width
      const h = res.data.sourceHeight ?? res.data.source_height
      const pages = res.data.totalPages ?? res.data.total_pages
      current.dimText = `${w} × ${h}`
      current.statusText = `预计 ${pages} 页`
      current.statusType = 'info'
      if (compareNextSlice.value && currentPreviewPage.value < pages) {
        const next = await tauriCallSafe('preview_long_slice', { args, previewPage: currentPreviewPage.value + 1 })
        if (requestId !== longSlicePreviewRequestId) return
        if (next.ok) nextLongSlicePreview.value = next.data
      }
    } else if (!res.ok) {
      current.statusText = '解析失败'
      current.statusType = 'danger'
    }
  } catch (err) {
    if (requestId === longSlicePreviewRequestId) console.error('preview error', err)
  } finally {
    if (requestId === longSlicePreviewRequestId) previewLoading.value = false
  }
}

function schedulePreviewRefresh() {
  longSlicePreviewRequestId++
  previewLoading.value = true
  clearTimeout(longSlicePreviewTimer)
  longSlicePreviewTimer = setTimeout(() => {
    longSlicePreviewTimer = null
    void refreshPreview()
  }, 250)
}

function cancelScheduledPreview() {
  clearTimeout(longSlicePreviewTimer)
  longSlicePreviewTimer = null
  longSlicePreviewRequestId++
}

function onSelectLongSliceItem(index) {
  selectedLongSliceIndex.value = index
  currentPreviewPage.value = 1
  void refreshPreview()
}

function changePreviewPage(delta) {
  if (!currentLongSlicePreview.value) return
  const max = currentLongSlicePreview.value.totalPages || 1
  const target = Math.max(1, Math.min(max, currentPreviewPage.value + delta))
  if (target !== currentPreviewPage.value) {
    currentPreviewPage.value = target
    void refreshPreview()
  }
}

function addLongSliceFiles(paths) {
  if (longSliceProcessing.value) return
  const existing = new Set(longSliceFiles.value.map((f) => f.path))
  const newItems = paths
    .filter((p) => !existing.has(p) && isLongSliceSupportedPath(p))
    .map((p) => ({
      path: p,
      name: fileName(p),
      statusText: '待处理',
      statusType: 'info',
      dimText: '',
      outputPath: null,
    }))
  if (!newItems.length) return
  const wasEmpty = longSliceFiles.value.length === 0
  longSliceFiles.value = [...longSliceFiles.value, ...newItems]
  if (wasEmpty) {
    selectedLongSliceIndex.value = 0
    currentPreviewPage.value = 1
    void refreshPreview()
  }
}

async function selectLongSliceFiles() {
  const selected = await open({
    multiple: true,
    filters: [
      { name: '长图与PDF文档', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'pdf'] },
      { name: '图片文件', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp'] },
      { name: 'PDF 文档', extensions: ['pdf'] },
    ],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    addLongSliceFiles(paths)
  }
}

async function selectLongSliceOutputDir() {
  const selected = await open({ directory: true })
  if (selected) {
    longSliceOutputDir.value = selected
  }
}

function clearLongSliceFiles() {
  if (longSliceProcessing.value) return
  cancelScheduledPreview()
  longSliceFiles.value = []
  currentLongSlicePreview.value = null
  nextLongSlicePreview.value = null
  selectedLongSliceIndex.value = 0
  currentPreviewPage.value = 1
  previewLoading.value = false
}

function removeLongSliceFile(index) {
  if (longSliceProcessing.value) return
  longSliceFiles.value.splice(index, 1)
  if (selectedLongSliceIndex.value >= longSliceFiles.value.length) {
    selectedLongSliceIndex.value = Math.max(0, longSliceFiles.value.length - 1)
  }
  if (longSliceFiles.value.length) {
    void refreshPreview()
  } else {
    cancelScheduledPreview()
    currentLongSlicePreview.value = null
    previewLoading.value = false
  }
}

async function batchProcessLongSlice() {
  if (longSliceProcessing.value || !longSliceFiles.value.length) return
  longSliceProcessing.value = true
  let successCount = 0
  try {
    for (const file of longSliceFiles.value) {
      file.statusText = '切片中…'
      file.statusType = 'warning'
      const args = getLongSliceArgs(file.path)
      const res = await withMinimumDuration(
        () => tauriCallSafe('process_long_slice', { args }),
        450,
      )
      if (res.ok && res.data) {
        file.outputPath = res.data.outputPath
        file.statusText = `已生成 ${res.data.totalPages} 页`
        file.statusType = 'success'
        successCount++
      } else {
        file.statusText = '处理失败'
        file.statusType = 'danger'
        ElMessage.error(userFacingError(res.error, '长图切片处理失败'))
      }
    }
    if (successCount > 0) {
      triggerLongSliceCelebration()
      ElNotification({
        title: '长图处理完成',
        message: `成功切片导出 ${successCount} 个 A4 文档`,
        type: 'success',
      })
    }
  } catch (err) {
    ElMessage.error('处理过程出现异常: ' + String(err))
  } finally {
    longSliceProcessing.value = false
  }
}

async function selectMergeFiles() {
  const selected = await open({
    multiple: true,
    filters: DOCUMENT_FILTERS,
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    await addMergeFiles(paths)
  }
}

async function addMergeFiles(paths) {
  if (merging.value) return
  const existing = new Set(mergeFiles.value.map(file => file.path))
  const raw = [...new Set(paths)].filter(path => isMergeSupportedPath(path) && !existing.has(path))
  let candidates = raw
  const sourcePaths = new Map()
  if (raw.some(path => documentKind(path) === 'image')) {
    merging.value = true
    try { candidates = await reviewImageImports(raw, async group => {
      const path = await imageImportDialog.value.open(group)
      if (path) sourcePaths.set(path, group)
      return path
    }) }
    finally { merging.value = false }
  }
  if (candidates.length) mergeOutputPath.value = ''
  const items = makeQueueItems(candidates).map(item => raw.includes(item.path) ? item : { ...item, sourcePaths: sourcePaths.get(item.path) || [item.path], outputPath: item.path, statusText: '图片 PDF 已就绪', statusType: 'success' })
  mergeFiles.value.push(...items)
  syncMergeOutputDirectory()
  if (mergeFiles.value.length === 1 && items[0]?.outputPath) mergeOutputPath.value = items[0].outputPath
}

function clearMergeFiles() {
  if (merging.value) return
  mergeOutputDir.value = ''
  mergeOutputDirManual.value = false
  mergeFileName.value = '合并文件.pdf'
  mergeOutputPath.value = ''
  mergeFiles.value = []
}

function removeMergeFile(index) {
  if (merging.value) return
  mergeOutputPath.value = ''
  mergeFiles.value.splice(index, 1)
  syncMergeOutputDirectory()
}

function reorderMergeFiles({ from, to }) {
  if (merging.value) return
  if (from === to || from < 0 || to < 0 || from >= mergeFiles.value.length || to >= mergeFiles.value.length) return
  const [item] = mergeFiles.value.splice(from, 1)
  mergeFiles.value.splice(to, 0, item)
}

async function prepareMergeItem(item) {
  if (isPdfPath(item.path) || item.outputPath) return true
  item.statusText = '正在转为 PDF…'
  item.statusType = 'warning'
  item.conversionError = ''
  try {
    item.outputPath = await convertImportDocument(item.path, tauriCallSafe)
    item.statusText = 'PDF 已就绪'
    item.statusType = 'success'
    return true
  } catch (error) {
    item.statusText = '转换失败，等待补充 PDF'
    item.statusType = 'danger'
    item.conversionError = `${userFacingError(error, '转换失败')}。${isWordPath(item.path) ? MANUAL_PDF_HINT : '请在图片排版中生成 PDF，再选择生成的 PDF 继续。'}`
    return false
  }
}

async function convertSingleMergeFile(item) {
  if (merging.value) return
  merging.value = true
  try {
    if (await prepareMergeItem(item)) {
      mergeOutputPath.value = item.outputPath
      ElMessage.success(`转换完成：${fileName(item.outputPath)}`)
    } else ElMessage.warning('自动转换未完成，请查看该文件的提示并补充 PDF')
  } finally { merging.value = false }
}

async function selectManualMergePdf(item) {
  if (merging.value) return
  const selected = normalizeSelectedPath(await open({ multiple: false, filters: [{ name: '手动转换后的 PDF', extensions: ['pdf'] }] }))
  if (!selected || merging.value || !mergeFiles.value.includes(item)) return
  item.outputPath = selected
  item.manualPdf = true
  item.conversionError = ''
  item.statusText = '手动 PDF 已就绪'
  item.statusType = 'success'
  mergeOutputPath.value = ''
}

async function selectMergeOutputDir() {
  if (merging.value) return
  const selected = normalizeSelectedPath(await open({ directory: true }))
  if (selected) { mergeOutputDir.value = selected; mergeOutputDirManual.value = true; mergeOutputPath.value = '' }
}

async function doMerge() {
  if (merging.value || !mergeFiles.value.length) return
  if (mergeFiles.value.length === 1 && isPdfPath(mergeFiles.value[0].path)) return

  if (mergeFiles.value.length === 1 && !isPdfPath(mergeFiles.value[0].path)) {
    await convertSingleMergeFile(mergeFiles.value[0])
    return
  }

  try { pdfOutputFileName(mergeFileName.value) } catch (error) { ElMessage.warning(error.message); return }
  merging.value = true
  try {
    let output = mergeOutputDir.value || commonSourceDirectory(mergeFiles.value)
    if (!output) {
      output = normalizeSelectedPath(await open({ directory: true }))
      if (output) mergeOutputDirManual.value = true
    }
    if (output) {
      mergeOutputDir.value = output
      const outputPath = joinOutputPath(output, pdfOutputFileName(mergeFileName.value))
      let failed = 0
      for (const file of mergeFiles.value) if (!await prepareMergeItem(file)) failed += 1
      if (failed) {
        ElMessage.warning(`${failed} 个文件未能转为 PDF，请补充手动转换的 PDF 后继续合并；列表已保留`)
        return
      }
      for (const file of mergeFiles.value) {
        file.statusText = isWordPath(file.path) && !file.outputPath ? '转换并合并中…' : '合并中…'
        file.statusType = 'warning'
      }
      const result = await withMinimumDuration(
        () =>
          tauriCallSafe('merge_pdfs', {
            inputs: mergeFiles.value.map((file) => file.outputPath || file.path),
            output: outputPath,
            duplexSeparate: duplexSeparate.value,
          }),
        650,
      )
      if (result.ok) {
        mergeOutputPath.value = result.data?.output_path || result.data || outputPath
        for (const file of mergeFiles.value) {
          file.statusText = '已合并'
          file.statusType = 'success'
        }
        triggerMergeCelebration()
      } else {
        for (const file of mergeFiles.value) {
          file.statusText = '合并失败'
          file.statusType = 'danger'
        }
      }
      result.ok
        ? ElMessage.success('合并完成')
        : ElMessage.error(userFacingError(result.error, '合并失败，请确认文件未损坏且未被其他程序占用'))
    }
  } finally {
    merging.value = false
  }
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

let extractLoadRequest = 0
async function loadExtractFile(path) {
  if (extractingPages.value) return false
  const request = ++extractLoadRequest
  const pageCount = await getPdfPageCount(path)
  if (request !== extractLoadRequest) return false
  if (!pageCount.ok || Number(pageCount.data) < 1) {
    ElMessage.error(userFacingError(pageCount.error, '读取页数失败，已保留原文件和设置'))
    return false
  }
  extractResultPath.value = ''
  extractFile.value = path
  extractPageText.value = ''
  extractTotalPages.value = Number(pageCount.data)
  extractOutputDir.value = parentDir(path)
  return true
}

async function selectExtractOutputDir() {
  const selected = await open({ directory: true })
  if (selected) extractOutputDir.value = normalizeSelectedPath(selected)
}

async function doExtractPages() {
  if (extractingPages.value || !extractFile.value || !extractTotalPages.value) return
  const pages = parsePageSelection(extractPageText.value, extractTotalPages.value)
  if (!pages.length) {
    ElMessage.warning('请输入有效页码，例如：3,7,12-15')
    return
  }
  extractingPages.value = true
  try {
    const result = await withMinimumDuration(
      () =>
        tauriCallSafe('extract_pdf_pages', {
          input: extractFile.value,
          pages,
          outputDir: extractOutputDir.value || null,
        }),
      600,
    )
    if (result.ok) {
      extractResultPath.value = result.data.output_path
      triggerExtractCelebration()
      ElMessage.success(`已导出：${result.data.output_path}`)
    } else {
      ElMessage.error(userFacingError(result.error, 'PDF 页面提取失败，请确认文件未损坏'))
    }
  } finally {
    extractingPages.value = false
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

// 选择与拖入只添加到队列，确认设置后由用户开始执行。
function loadCompressFiles(paths) {
  if (compressing.value) return
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
}

function clearCompressFiles() {
  if (compressing.value) return
  compressFiles.value = []
  compressSummary.value = ''
}

function removeCompressFile(index) {
  if (compressing.value) return
  const item = compressFiles.value[index]
  if (item && item.status !== 'processing') compressFiles.value.splice(index, 1)
}

// 已完成后调整设置只重新入队；再次点击开始压缩才会处理。
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
})

async function runCompressQueue() {
  if (compressing.value) return
  for (const item of compressFiles.value) {
    if (['failed', 'cancelled'].includes(item.status)) item.status = 'pending'
  }
  if (!compressFiles.value.some(item => item.status === 'pending')) return
  compressing.value = true
  compressCancellation.reset()
  const level = compressLevel.value
  const imageReencode = compressImageReencode.value
  const processed = []
  try {
    while (!compressCancellation.cancelled.value) {
      const item = compressFiles.value.find((f) => f.status === 'pending')
      if (!item) break
      item.status = 'processing'
      item.statusText = '压缩中'
      item.statusType = 'warning'
      const result = await withMinimumDuration(
        () =>
          tauriCallSafe('compress_pdf', {
            input: item.path,
            outputDir: null,
            level: imageReencode ? level : null,
          }),
        500,
      )
      if (result.ok) {
        const data = result.data || {}
        item.status = 'done'
        item.statusText = '完成'
        item.statusType = 'success'
        item.inputSize = Number(data.input_size) || 0
        item.outputSize = Number(data.output_size) || 0
        item.outputPath = data.output_path
      } else {
        const cancelled = /操作已取消/.test(String(result.error))
        if (cancelled) compressCancellation.cancel()
        item.status = cancelled ? 'cancelled' : 'failed'
        item.statusText = cancelled ? '已取消' : userFacingError(result.error, '压缩失败')
        item.statusType = cancelled ? 'info' : 'danger'
      }
      processed.push(item)
    }
  } finally {
    compressing.value = false
  }
  if (processed.some((i) => i.status === 'done')) {
    triggerCompressCelebration()
  }
  reportCompressSummary(processed)
}

function reportCompressSummary(processed) {
  if (!processed.length) return
  const text = batchSummaryText(compressCancellation.cancelled.value ? '压缩已停止' : '压缩完成', processed)
  compressSummary.value = text
  const hasFailed = processed.some((item) => item.status === 'failed')
  ElNotification({ type: hasFailed ? 'warning' : compressCancellation.cancelled.value ? 'info' : 'success', title: text, duration: 6000 })
}

async function selectSplitFile() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) await loadSplitFile(normalizeSelectedPath(selected))
}

let splitLoadRequest = 0
function clearSplitFile() {
  if (splittingMerged.value) return
  splitLoadRequest += 1
  splitFile.value = ''
  splitRanges.value = []
  splitTotalPages.value = 0
  splitPreviewPage.value = 1
  selectedSplitRangeIndex.value = 0
  splitOutputDir.value = ''
  splitResultDir.value = ''
  splitResultPaths.value = []
  splitRunWarnings.value = []
}
async function loadSplitFile(path) {
  if (splittingMerged.value) return false
  const request = ++splitLoadRequest
  const pageCount = await getPdfPageCount(path)
  if (request !== splitLoadRequest) return false
  if (!pageCount.ok || Number(pageCount.data) < 1) {
    ElMessage.error(userFacingError(pageCount.error, '读取页数失败，已保留原拆分页段'))
    return false
  }
  splitResultDir.value = ''
  splitResultPaths.value = []
  splitFile.value = path
  splitPreviewPage.value = 1
  splitTotalPages.value = Number(pageCount.data)
  splitRunWarnings.value = []
  splitOutputDir.value = parentDir(path)
  splitRanges.value = [{ name: stripPdf(fileName(path)), pageStart: 1, pageEnd: splitTotalPages.value }]
  selectedSplitRangeIndex.value = 0
  return true
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

function isWordPath(path) {
  return /\.(docx|doc|docm)$/i.test(String(path || ''))
}

function isMergeSupportedPath(path) {
  return documentKind(path) !== null
}

async function handleDroppedPdfPaths(paths) {
  if (activeTab.value === 'long-slice') {
    const slicePaths = [...new Set((paths || []).filter(isLongSliceSupportedPath))]
    if (!slicePaths.length) {
      ElMessage.warning('请拖入长截屏图片（PNG/JPG/WebP/BMP）或 PDF 文件')
      return
    }
    if (slicePaths.length < (paths || []).length) ElMessage.warning('已忽略不支持的文件格式（仅支持静态图片和 PDF）')
    addLongSliceFiles(slicePaths)
    return
  }

  if (activeTab.value === 'merge') {
    const mergePaths = [...new Set((paths || []).filter(isMergeSupportedPath))]
    if (!mergePaths.length) {
      ElMessage.warning('请拖入 PDF、Word 或图片（JPG / PNG / WebP / BMP）')
      return
    }
    if (mergePaths.length < (paths || []).length) ElMessage.warning('已忽略不支持的文件格式（仅支持 PDF、Word 和静态图片）')
    await addMergeFiles(mergePaths)
    return
  }

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
  } else if (activeTab.value === 'compress') {
    loadCompressFiles(pdfPaths)
  } else {
    if (pdfPaths.length > 1) ElMessage.info('当前工具一次处理一个 PDF，已使用第一个文件')
    const path = pdfPaths[0]
    if (activeTab.value === 'extract') await loadExtractFile(path)
    if (activeTab.value === 'split') await loadSplitFile(path)
    if (activeTab.value === 'crop') await cropWorkspace.value?.loadFile(path)
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

function clampSplitPreviewRatio(value) {
  return Math.min(0.65, Math.max(0.3, Number(value) || 0.42))
}

function setSplitPreviewRatio(value) {
  splitPreviewRatio.value = clampSplitPreviewRatio(value)
}

function startSplitResize(event) {
  if (event.button !== 0 || !splitMainRef.value) return
  event.preventDefault()
  const rect = splitMainRef.value.getBoundingClientRect()
  const update = (pointerEvent) => {
    setSplitPreviewRatio((rect.right - pointerEvent.clientX) / Math.max(1, rect.width))
  }
  const finish = () => {
    window.removeEventListener('pointermove', update)
    window.removeEventListener('pointerup', finish)
    window.removeEventListener('pointercancel', finish)
    stopSplitResize = null
  }
  stopSplitResize?.()
  stopSplitResize = finish
  window.addEventListener('pointermove', update)
  window.addEventListener('pointerup', finish, { once: true })
  window.addEventListener('pointercancel', finish, { once: true })
  update(event)
}

function splitAtPreviewPage(page) {
  const target = Math.min(splitPreviewMaxPage.value, Math.max(1, Number(page || 1)))
  const index = insertRangeAtPage(splitRanges.value, target, splitPreviewMaxPage.value, {
    name: `文件${splitRanges.value.length + 1}`,
    reuseBoundary: true,
  })
  if (index < 0) return
  selectedSplitRangeIndex.value = index
  splitPreviewPage.value = target
}

function splitFromCurrentPage() {
  splitAtPreviewPage(splitPreviewPage.value)
}

function splitFromPreviousPage() {
  if (splitPreviewPage.value <= 1) return
  splitAtPreviewPage(splitPreviewPage.value - 1)
}

function handleSplitShortcut(event) {
  if (route && route.name !== 'pdf-tools') return
  if (activeTab.value !== 'split' || !splitFile.value || event.ctrlKey || event.metaKey || event.altKey) return
  const tag = String(event.target?.tagName || '').toLowerCase()
  if (tag === 'input' || tag === 'textarea' || tag === 'select' || event.target?.isContentEditable) return
  if (event.key === 'ArrowLeft') moveSplitPreviewPage(-1)
  else if (event.key === 'ArrowRight') moveSplitPreviewPage(1)
  else if (event.key === '[') splitFromPreviousPage()
  else if (event.key === ']') splitFromCurrentPage()
  else return
  event.preventDefault()
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
  if (splittingMerged.value) return
  if (!splitFile.value || !splitOutputDir.value || !splitRanges.value.length) return
  const warnings = splitRangeWarnings(splitRanges.value, splitTotalPages.value)
  if (warnings.length) {
    ElMessage.warning(`请先核对页段：${warnings[0]}`)
    return
  }
  splittingMerged.value = true
  splitResultPaths.value = []
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
    splitResultPaths.value = (result.data.outputs || []).map(item => item.outputPath).filter(Boolean)
    splitRunWarnings.value = result.data.warnings || []
    const failed = result.data.failed?.length || 0
    const outputs = result.data.outputs?.length || 0
    if (outputs) splitResultDir.value = splitOutputDir.value
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

const incomingReady = ref(false)
const incoming = useIncomingFiles('pdf-tools', async ({ paths, target }) => {
  const tab = target === 'long-slice' ? target : target.replace('pdf-', '')
  const previousPath = tab === 'extract' ? extractFile.value : tab === 'split' ? splitFile.value : tab === 'crop' ? cropWorkspace.value?.getPath() : ''
  if (previousPath && previousPath !== paths[0]) {
    const label = FILE_TRANSFER_TARGETS.find(item => item.id === target)?.label || '目标工具'
    try { await ElMessageBox.confirm(`${label}已有文件“${fileName(previousPath)}”。是否替换这份文件及其设置？`, '切换文件', { confirmButtonText: '替换当前文件', cancelButtonText: '保留当前文件' }) } catch { return false }
  }
  activeTab.value = tab
  await nextTick()
  if (previousPath === paths[0]) return true
  if (tab === 'merge') {
    await addMergeFiles(paths)
    return paths.every(path => mergeFiles.value.some(file => file.path === path || file.sourcePaths?.includes(path)))
  }
  if (tab === 'crop') return (await cropWorkspace.value?.loadFile(paths[0])) === true
  if (tab === 'extract') return loadExtractFile(paths[0])
  if (tab === 'split') return loadSplitFile(paths[0])
  if (tab === 'compress') { loadCompressFiles(paths); return paths.every(path => compressFiles.value.some(file => file.path === path)) }
  if (tab === 'long-slice') { addLongSliceFiles(paths); return true }
  return false
}, { ready: () => incomingReady.value, busy: () => merging.value || compressing.value || splittingMerged.value || extractingPages.value || longSliceProcessing.value || cropWorkspace.value?.isBusy() })

onMounted(() => {
  window.addEventListener('keydown', handleSplitShortcut)
  void preference.start().finally(() => { incomingReady.value = true })
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleSplitShortcut)
  stopSplitResize?.()
  void preference.stop()
})
</script>

<style scoped>
.merge-output-settings { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 10px; align-items: start; padding: 10px 0; }
.merge-output-settings > label { line-height: 32px; white-space: nowrap; font-size: 13px; }

.pdf-tools-view {
  display: flex;
  flex-direction: column;
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: var(--docsy-canvas);
}
.pdf-tools-view > .pdf-tabs { flex: 1; min-height: 0; }

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
  min-height: 40px;
  margin-block: clamp(8px, 1.5dvh, 12px) 0;
  padding: 0 2px;
  border: 0;
  border-radius: 0;
  background: transparent;
}

:deep(.pdf-tabs .workspace-content) {
  margin-top: clamp(8px, 1.5dvh, 12px);
  padding: 0;
  background: transparent;
}

:deep(.pdf-tabs .workspace-actions) {
  min-height: clamp(50px, 6.5dvh, 58px);
  margin-top: clamp(8px, 1.5dvh, 12px);
  padding-block: 12px;
  padding-inline: 14px;
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
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  margin: 0 0 8px;
  border: 0;
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface-muted) 52%, transparent);
  color: var(--docsy-text);
  font-size: 12.5px;
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.path-line > span:last-child {
  min-width: 0;
}

.path-line--summary {
  margin-top: 10px;
  background: var(--docsy-primary-soft);
  color: var(--docsy-text-strong);
  font-weight: 500;
}

.path-badge {
  flex-shrink: 0;
  padding: 2px 7px;
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  color: var(--docsy-text-muted);
  font-size: 11px;
  font-weight: 600;
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
  max-width: 540px;
  margin-top: 12px;
  padding: 0 2px;
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
  gap: 0;
  align-items: start;
  margin-top: 4px;
}

.split-list-panel,
.split-preview {
  min-width: 0;
}

.split-list-panel {
  margin-right: 8px;
}

.split-preview {
  margin-left: 8px;
  padding: 12px 14px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
}

.split-resizer {
  align-self: stretch;
  min-height: 360px;
  border-radius: 999px;
  background: linear-gradient(
    90deg,
    transparent 3px,
    var(--docsy-border-subtle) 3px,
    var(--docsy-border-subtle) 7px,
    transparent 7px
  );
  cursor: col-resize;
  touch-action: none;
}

.split-resizer:hover {
  background: linear-gradient(
    90deg,
    transparent 3px,
    var(--docsy-primary) 3px,
    var(--docsy-primary) 7px,
    transparent 7px
  );
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
  padding: 10px 13px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
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
  flex-direction: column;
  align-items: stretch;
  gap: 10px;
  margin-bottom: 10px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  color: var(--docsy-text-strong);
  font-size: 13px;
  font-weight: 600;
}

.split-preview-status {
  margin-top: 3px;
  color: var(--docsy-text-muted);
  font-size: 12px;
  font-weight: 400;
  line-height: 1.4;
}

.split-preview-actions {
  display: flex;
  width: 100%;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: flex-start;
}

.split-preview-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.split-preview-pages {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.split-primary-preview {
  flex: 1 1 auto;
  min-width: 0;
}

.split-empty {
  min-height: 100%;
}

.extract-empty {
  flex: 1;
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
    grid-template-columns: 1fr !important;
  }

  .split-resizer {
    display: none;
  }

  .split-list-panel,
  .split-preview {
    margin: 0;
  }

  .split-preview {
    margin-top: 16px;
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
  overflow-wrap: anywhere;
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
.compress-options {
  display: flex;
  align-items: center;
  gap: 10px 18px;
  flex-wrap: wrap;
  margin-right: auto;
}
.compress-level-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
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
.merge-directory { max-width: min(420px, 45vw); color: var(--docsy-text-muted); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.merge-item-status {
  font-size: 12px;
  color: var(--docsy-text-muted);
  margin-left: 8px;
}
</style>

<style scoped>
.split-preview-actions { align-items: center; gap: 8px 14px; }
.split-preview-actions :deep(.el-button-group .el-button + .el-button) { margin-left: -1px; }
.merge-options { display: flex; align-items: center; gap: 2px; }
</style>

<style scoped>
.merge-conversion-error { flex-basis: 100%; color: var(--docsy-danger); font-size: 12px; line-height: 1.6; white-space: normal; overflow-wrap: anywhere; }

/* ── Long Slice Styles ── */
.long-slice-container.has-files {
  display: grid;
  grid-template-columns: minmax(340px, 1fr) minmax(360px, 1fr);
  gap: 16px;
  align-items: start;
}
.long-slice-queue {
  grid-column: 1;
  grid-row: 2;
  min-width: 0;
}
.long-slice-preview-card {
  grid-column: 2;
  grid-row: 1 / span 2;
  display: flex;
  min-width: 0;
  flex-direction: column;
  padding: 12px 14px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
}
.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.preview-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--docsy-text);
}
.preview-pagination {
  display: flex;
  align-items: center;
  gap: 6px;
}
.page-indicator {
  font-size: 12px;
  color: var(--docsy-text-subtle);
}
.a4-sheet-viewport {
  display: flex;
  flex: 1;
  justify-content: center;
  align-items: flex-start;
  width: 100%;
  height: clamp(320px, calc(100vh - 340px), 720px);
  background: var(--docsy-surface-muted);
  padding: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  min-height: 320px;
  overflow: auto;
  gap: 12px;
}
.a4-sheet-viewport.is-comparing .a4-sheet-canvas { width: calc((100% - 12px) / 2); }
.long-slice-note { color: var(--docsy-text-muted); font-size: 12px; line-height: 1.5; }
.a4-sheet-canvas {
  flex: 0 0 auto;
  max-width: 100%;
  box-shadow: 0 2px 14px rgba(0, 0, 0, 0.16);
  background: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}
.a4-sheet-canvas.portrait {
  width: min(100%, calc((100vh - 380px) * 210 / 297));
  max-width: calc(720px * 210 / 297);
  aspect-ratio: 210 / 297;
}
.a4-sheet-canvas.landscape {
  width: min(100%, calc((100vh - 380px) * 297 / 210));
  max-width: 720px;
  aspect-ratio: 297 / 210;
}
.preview-image {
  display: block;
  width: 100%;
  height: auto;
  object-fit: contain;
}
.preview-placeholder {
  font-size: 12px;
  color: var(--docsy-text-muted);
}
.preview-footer-info {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: center;
  align-items: center;
  margin-top: 10px;
  font-size: 11px;
  color: var(--docsy-text-muted);
}
.long-slice-options-panel {
  grid-column: 1;
  grid-row: 1;
  min-width: 0;
  padding: 14px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
}
.field-hint {
  margin-top: 6px;
  color: var(--docsy-text-muted);
  font-size: 11px;
  line-height: 1.5;
}
.field-label {
  display: block;
  margin: 8px 0 4px;
  color: var(--docsy-text);
  font-size: 12px;
}
.footer-total-toggle {
  margin-top: 8px;
}
.footer-format-preview {
  margin-top: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
  font-size: 12px;
  text-align: center;
}
.opt-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}
.opt-row.align-center {
  align-items: center;
  justify-content: space-between;
}
.slider-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 4px;
  width: 100%;
}
.slider-val {
  font-size: 12px;
  font-weight: 500;
  width: 40px;
}
.overlap-box {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--docsy-text);
}
.header-footer-settings {
  border-top: 1px solid var(--docsy-border-subtle);
  margin-top: 8px;
}
.settings-group-title {
  margin: 10px 0 8px;
  color: var(--docsy-text-strong);
  font-size: 12px;
  font-weight: 600;
}
.sub-fields {
  margin-top: 8px;
  padding-left: 4px;
}
.mt-2 { margin-top: 8px; }
.mt-3 { margin-top: 12px; }

@media (max-width: 960px) {
  .long-slice-container.has-files {
    grid-template-columns: 1fr;
  }
  .long-slice-queue,
  .long-slice-options-panel,
  .long-slice-preview-card {
    grid-column: 1;
    position: static;
  }
  .long-slice-options-panel { grid-row: 1; }
  .long-slice-preview-card { grid-row: 2; }
  .long-slice-queue { grid-row: 3; }
}
</style>

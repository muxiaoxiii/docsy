<template>
  <ToolWorkspaceShell title="图片排版" description="将图片批量排版为 A4 文档">
    <div class="paddler-layout">
      <!-- Settings Panel -->
      <div class="settings-panel">
        <el-form label-width="80px" size="small">
          <el-form-item :label="isFrameSequence ? '来源' : '文件夹'">
            <el-button @click="selectFolder">选择文件夹</el-button>
            <span v-if="isFrameSequence" class="folder-path transfer-source">
              {{ inputContext.sourceLabel || '视频抽帧筛选结果' }}
              <template v-if="analysis"> · {{ analysis.images.length }} 张</template>
            </span>
            <span v-else-if="folders.length" class="folder-path">{{ folders.join('；') }}</span>
          </el-form-item>

          <el-form-item label="输出格式">
            <el-select v-model="settings.output_format">
              <el-option label="DOCX" value="docx" />
              <el-option label="PDF" value="pdf" />
            </el-select>
          </el-form-item>

          <el-form-item v-if="!isFrameSequence && folders.length > 1" label="多文件夹">
            <el-select v-model="settings.output_mode">
              <el-option label="合并为一个文档" value="merged" />
              <el-option label="每个文件夹单独生成" value="per_folder" />
            </el-select>
          </el-form-item>

          <el-form-item label="每页最多">
            <el-select v-model="settings.layout">
              <el-option label="1 张" value="1" />
              <el-option label="2 张（左右）" value="1x2" />
              <el-option label="2 张（上下）" value="2x1" />
              <el-option label="3 张（左右）" value="1x3" />
              <el-option label="4 张" value="4" />
              <el-option label="6 张" value="2x3" />
              <el-option label="9 张" value="3x3" />
              <el-option label="自定义" value="custom" />
            </el-select>
          </el-form-item>

          <el-form-item v-if="settings.layout === 'custom'" label="行列">
            <div class="inline-controls">
              <el-input-number v-model="settings.custom_rows" :min="1" :max="8" />
              <span>行</span>
              <el-input-number v-model="settings.custom_cols" :min="1" :max="8" />
              <span>列</span>
            </div>
          </el-form-item>

          <el-form-item label="缩放模式">
            <el-select v-model="settings.scale_mode">
              <el-option label="适应页面" value="fit" />
              <el-option label="不缩放" value="original" />
            </el-select>
          </el-form-item>

          <el-form-item label="方向">
            <el-select v-model="settings.orientation">
              <el-option label="自动" value="auto" />
              <el-option label="竖向" value="portrait" />
              <el-option label="横向" value="landscape" />
            </el-select>
          </el-form-item>

          <el-form-item label="页边距">
            <el-input-number v-model="settings.margin_mm" :min="0" :max="30" :step="1" />
            <span class="unit-label">mm</span>
          </el-form-item>

          <el-form-item label="文件名">
            <div class="filename-panel">
              <div class="filename-panel-row">
                <el-switch v-model="settings.show_filename" active-text="显示" inactive-text="隐藏" />
                <el-switch
                  v-model="settings.filename_without_ext"
                  active-text="隐藏扩展名"
                  inactive-text="保留扩展名"
                />
                <label class="filename-font-control">
                  <span>字体</span>
                  <el-select v-model="settings.filename_font_family" :disabled="!settings.show_filename">
                    <el-option label="无衬线" value="sans" />
                    <el-option label="宋体" value="serif" />
                    <el-option label="楷体" value="kaiti" />
                    <el-option label="仿宋" value="fangsong" />
                  </el-select>
                </label>
                <label class="filename-size-control">
                  <span>字号</span>
                  <el-input-number
                    v-model="settings.filename_font_size_pt"
                    :min="6"
                    :max="24"
                    :step="1"
                    :disabled="!settings.show_filename"
                    controls-position="right"
                  />
                  <span>pt</span>
                </label>
              </div>
              <div class="filename-rules">
                <div
                  v-for="(rule, idx) in settings.filename_rules"
                  :key="rule.id"
                  class="filename-rule"
                  :class="{ 'filename-rule-keep': rule.kind === 'keep' }"
                >
                  <el-select v-model="rule.kind" size="small" class="rule-kind">
                    <el-option label="删除" value="remove" />
                    <el-option label="替换" value="replace" />
                    <el-option label="加前缀" value="prefix" />
                    <el-option label="加后缀" value="suffix" />
                    <el-option label="保留成分" value="keep" />
                  </el-select>
                  <template v-if="rule.kind === 'replace'">
                    <el-input v-model="rule.value" size="small" placeholder="原文字" />
                    <el-input v-model="rule.replacement" size="small" placeholder="替换为" />
                  </template>
                  <template v-else-if="rule.kind === 'keep'">
                    <el-checkbox v-model="rule.keep_time" size="small">时间</el-checkbox>
                    <el-checkbox v-model="rule.keep_number" size="small">编号</el-checkbox>
                    <el-checkbox v-model="rule.keep_text" size="small">文本</el-checkbox>
                    <el-input v-model="rule.replacement" size="small" placeholder="自定义名称" />
                    <el-select v-model="rule.separator" size="small" class="separator-select">
                      <el-option label="_" value="_" />
                      <el-option label="-" value="-" />
                      <el-option label="空格" value=" " />
                    </el-select>
                  </template>
                  <template v-else>
                    <el-input v-model="rule.value" size="small" :placeholder="rulePlaceholder(rule.kind)" />
                  </template>
                  <el-button size="small" text type="danger" @click="removeFilenameRule(idx)">-</el-button>
                </div>
              </div>
              <el-button size="small" plain @click="addFilenameRule">+ 添加规则</el-button>
            </div>
          </el-form-item>

          <el-form-item label="排列">
            <el-select v-model="settings.order_mode">
              <el-option label="Z 字" value="z" />
              <el-option label="N 字" value="n" />
              <el-option label="倒 N 字" value="reverse_n" />
              <el-option label="自定义顺序" value="custom" />
            </el-select>
          </el-form-item>

          <el-form-item label="边框">
            <div class="inline-controls">
              <el-switch v-model="settings.border_enabled" />
              <el-select v-model="settings.border_color" :disabled="!settings.border_enabled">
                <el-option label="黑色" value="black" />
                <el-option label="白色" value="white" />
                <el-option label="深灰" value="dark_gray" />
                <el-option label="浅灰" value="light_gray" />
                <el-option label="红色" value="red" />
                <el-option label="黄色" value="yellow" />
                <el-option label="蓝色" value="blue" />
              </el-select>
            </div>
          </el-form-item>

          <el-form-item class="workspace-action-row">
            <el-button
              class="primary-workspace-action"
              type="success"
              @click="run"
              :loading="generating"
              :disabled="!analysis || !includedImages.length"
            >
              生成文档
            </el-button>
            <span v-if="analyzing" class="analyze-hint">正在分析...</span>
          </el-form-item>
        </el-form>
      </div>

      <!-- Analysis Result -->
      <div class="result-panel">
        <template v-if="analysis">
          <div v-if="generatedResult" class="generated-result">
            <div>
              <strong>已生成</strong>
              <div v-for="path in generatedOutputPaths" :key="path" class="output-path">{{ path }}</div>
            </div>
            <el-button size="small" type="primary" @click="openGeneratedOutput">打开文件</el-button>
          </div>

          <div v-if="isFrameSequence" class="sequence-mode-note">
            <strong>抽帧序列模式</strong>
            <span>保持原顺序和原图比例；当前排版 {{ includedImages.length }} 张，排除 {{ excludedCount }} 张。</span>
            <el-switch
              v-model="settings.use_source_exclusions"
              inline-prompt
              active-text="沿用抽帧排除"
              inactive-text="全部载入"
            />
          </div>

          <div class="analysis-summary">
            <el-descriptions :column="2" border size="small">
              <el-descriptions-item label="图片数量">
                {{ includedImages.length }} 张参与排版
                <template v-if="excludedCount">，{{ excludedCount }} 张排除</template>
              </el-descriptions-item>
              <el-descriptions-item label="推荐方向">{{
                orientationLabel(analysis.recommended.orientation)
              }}</el-descriptions-item>
              <el-descriptions-item label="推荐布局">{{
                layoutLabel(analysis.recommended.layout)
              }}</el-descriptions-item>
              <el-descriptions-item label="推荐缩放">{{
                scaleModeLabel(analysis.recommended.scale_mode)
              }}</el-descriptions-item>
              <el-descriptions-item label="推荐边距">{{ analysis.recommended.margin_mm }} mm</el-descriptions-item>
              <el-descriptions-item label="当前方向">{{ resolvedOrientationLabel }}</el-descriptions-item>
              <el-descriptions-item label="当前布局"
                >{{ layoutGrid.rows }} 行 × {{ layoutGrid.cols }} 列</el-descriptions-item
              >
            </el-descriptions>
            <div class="recommendation-bar">
              <span>{{ analysis.recommended.reason }}</span>
              <el-button size="small" type="primary" @click="applyRecommendedSettings">应用推荐参数</el-button>
            </div>
          </div>

          <div class="preview-section">
            <div class="section-head">
              <h4>第一页预览</h4>
              <div class="preview-toolbar">
                <span>当前页 {{ previewImages.length }} 张</span>
                <el-button size="small" text @click="adjustPageZoom(-10)">-</el-button>
                <el-slider v-model="pageZoom" :min="50" :max="180" :step="5" class="zoom-slider" />
                <el-button size="small" text @click="adjustPageZoom(10)">+</el-button>
                <span class="zoom-value">{{ pageZoom }}%</span>
              </div>
            </div>
            <div class="page-preview-shell">
              <div class="page-preview" :class="resolvedOrientation" :style="previewPageStyle">
                <div class="preview-grid" :style="previewGridStyle">
                  <div
                    v-for="(img, idx) in previewSlots"
                    :key="idx"
                    class="preview-cell"
                    :class="{
                      'preview-cell-bordered': settings.border_enabled,
                      'preview-cell-white-border': settings.border_enabled && settings.border_color === 'white',
                      'preview-cell-no-name': !settings.show_filename,
                    }"
                    :style="previewCellStyle"
                  >
                    <template v-if="img">
                      <div class="preview-image-area" :style="previewImageAreaStyle">
                        <img :src="imageSrc(img.path)" :alt="fileName(img.path)" :style="previewImageStyle(img)" />
                      </div>
                      <div v-if="settings.show_filename" class="preview-name" :style="previewNameStyle">
                        <span v-for="(line, lineIdx) in fileNameLines(img.path)" :key="`${lineIdx}-${line}`">{{
                          line
                        }}</span>
                      </div>
                    </template>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Image List -->
          <div class="image-list">
            <div class="section-head">
              <h4>图片预览</h4>
            </div>
            <ReorderableImageGrid
              :items="orderedImages"
              :name-resolver="imageItemName"
              :meta-resolver="imageItemMeta"
              :excluded-resolver="isImageExcluded"
              preserve-aspect-ratio
              :initial-zoom="130"
              empty-description="暂无图片"
              @reorder="reorderLayoutImages"
              @toggle-excluded="toggleImageExclusion"
            />
          </div>
        </template>
        <WorkspaceEmptyState
          v-else
          class="result-empty-state"
          :icon-url="imageLayoutIconUrl"
          :title="analyzing ? '正在分析图片' : '等待选择图片文件夹'"
          :description="analyzing ? '正在读取图片尺寸。' : '选择文件夹后，Docsy 会自动分析并生成第一页排版预览。'"
        />
      </div>
    </div>
  </ToolWorkspaceShell>
</template>

<script setup>
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import ReorderableImageGrid from '../../../shared/components/ReorderableImageGrid.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import imageLayoutIconUrl from '../../../assets/icons/image-layout.svg?url'
import { useImagePaddlerState } from '../composables/useImagePaddlerState.js'
import { useWorkspaceStore } from '../../../stores/workspace.js'

const workspaceStore = useWorkspaceStore()

const {
  folders,
  analyzing,
  generating,
  analysis,
  generatedResult,
  inputContext,
  isFrameSequence,
  pageZoom,
  settings,
  layoutGrid,
  resolvedOrientation,
  resolvedOrientationLabel,
  orderedImages,
  includedImages,
  excludedCount,
  previewImages,
  previewSlots,
  generatedOutputPaths,
  previewPageStyle,
  previewGridStyle,
  previewCellStyle,
  previewImageAreaStyle,
  previewNameStyle,
  selectFolder,
  run,
  reorderLayoutImages,
  openGeneratedOutput,
  imageSrc,
  fileName,
  fileNameLines,
  previewImageStyle,
  imageItemName,
  imageItemMeta,
  isImageExcluded,
  toggleImageExclusion,
  addFilenameRule,
  removeFilenameRule,
  rulePlaceholder,
  applyRecommendedSettings,
  adjustPageZoom,
  orientationLabel,
  layoutLabel,
  scaleModeLabel,
} = useImagePaddlerState({
  initialTransfer: () => workspaceStore.mediaTransfer,
  onInitialPathsLoaded: () => workspaceStore.clearMediaTransfer(),
})
</script>

<style scoped>
:deep(.workspace-content) {
  overflow: hidden;
}

.paddler-layout {
  display: grid;
  grid-template-columns: 382px minmax(0, 1fr);
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-panel-radius);
  box-shadow: var(--docsy-shadow-panel);
}

.settings-panel {
  min-width: 0;
  overflow-y: auto;
  padding-block: clamp(16px, 3.1dvh, 22px) clamp(20px, 3.9dvh, 28px);
  padding-inline: 24px;
  border-right: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-elevated);
}

.folder-path {
  font-size: 12px;
  color: var(--docsy-text-muted);
  word-break: break-all;
  display: block;
  margin-top: 4px;
}

.inline-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.inline-controls :deep(.el-input-number) {
  width: 82px;
}

.filename-panel {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.filename-panel-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.filename-font-control,
.filename-size-control {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.filename-font-control :deep(.el-select) {
  width: 104px;
}

.filename-size-control :deep(.el-input-number) {
  width: 82px;
}

.filename-rules {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.filename-rule {
  display: grid;
  grid-template-columns: 78px minmax(0, 1fr) auto;
  gap: 6px;
  align-items: center;
}

.filename-rule-keep {
  grid-template-columns: 78px auto auto auto minmax(0, 1fr) 64px auto;
}

.rule-kind {
  width: 78px;
}

.separator-select {
  width: 64px;
}

.result-panel {
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding: clamp(18px, 3.4dvh, 24px);
  background: color-mix(in srgb, var(--docsy-surface-muted) 82%, var(--docsy-canvas));
}

.result-empty-state {
  height: 100%;
  min-height: clamp(280px, 40dvh, 420px);
  box-sizing: border-box;
}

.workspace-action-row {
  margin-top: clamp(14px, 2.8dvh, 20px);
  padding-top: clamp(14px, 2.5dvh, 18px);
  border-top: 1px solid var(--docsy-border-subtle);
}

.primary-workspace-action {
  width: 100%;
}

.analysis-summary {
  margin-bottom: 16px;
}

.sequence-mode-note {
  display: flex;
  margin-bottom: 14px;
  padding: 11px 13px;
  align-items: baseline;
  gap: 10px;
  border: 1px solid color-mix(in srgb, var(--docsy-primary) 28%, var(--docsy-border-subtle));
  border-radius: var(--docsy-radius);
  color: var(--docsy-text-muted);
  background: var(--docsy-primary-soft);
  font-size: 12px;
  line-height: 1.55;
}

.sequence-mode-note :deep(.el-switch) {
  margin-left: auto;
  flex: 0 0 auto;
}

.sequence-mode-note strong {
  flex: 0 0 auto;
  color: var(--docsy-text-strong);
}

.recommendation-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 10px;
  padding: 12px 14px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-primary-soft);
  border-radius: var(--docsy-radius);
  color: var(--docsy-text);
  font-size: 12px;
}

.preview-section {
  margin-bottom: 18px;
}

.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.section-head h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 680;
  color: var(--docsy-text-strong);
}

.preview-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.zoom-slider {
  width: 130px;
}

.zoom-value {
  width: 42px;
  text-align: right;
  color: var(--docsy-text);
}

.page-size-select {
  width: 82px;
}

.page-preview-shell {
  display: block;
  padding: 20px;
  background: #deddd8;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  overflow: auto;
}

.page-preview {
  box-sizing: border-box;
  background: #fff;
  border: 1px solid var(--docsy-border-strong);
  box-shadow: 0 16px 38px rgba(42, 39, 34, 0.14);
  margin: 0 auto;
}

.preview-grid {
  width: 100%;
  height: 100%;
  display: grid;
  gap: 0;
}

.preview-cell {
  box-sizing: border-box;
  min-width: 0;
  min-height: 0;
  border: 1px solid transparent;
  background: var(--docsy-surface-elevated);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  overflow: hidden;
}

.preview-cell-bordered {
  border: 2px solid var(--docsy-text-strong);
}

.preview-cell-white-border {
  border-color: white;
  box-shadow: inset 0 0 0 1px var(--docsy-border-subtle);
}

.preview-image-area {
  box-sizing: border-box;
  width: 100%;
  min-height: 0;
  flex-shrink: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.preview-cell img {
  display: block;
  object-fit: contain;
}

.preview-cell-no-name img {
  max-height: 100%;
}

.preview-name {
  box-sizing: border-box;
  width: 100%;
  flex-shrink: 0;
  line-height: 14px;
  padding: 0 4px;
  color: var(--docsy-text);
  font-size: 11px;
  text-align: center;
  display: flex;
  flex-direction: column;
  justify-content: center;
  overflow: hidden;
  overflow-wrap: anywhere;
}

.preview-name span {
  display: block;
}

.generated-result {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  padding: 12px 14px;
  margin-bottom: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: 0 8px 24px rgba(48, 41, 32, 0.05);
  font-size: 13px;
}

.output-path {
  margin-top: 4px;
  color: var(--docsy-text);
  word-break: break-all;
  font-size: 12px;
}

.image-list h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.unit-label {
  margin-left: 8px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

.analyze-hint {
  margin-left: 8px;
  color: var(--docsy-text-muted);
  font-size: 12px;
}

@media (max-width: 1180px) {
  :deep(.workspace-content) {
    overflow: auto;
  }

  .paddler-layout {
    display: block;
    height: auto;
    overflow: visible;
  }

  .settings-panel,
  .result-panel {
    overflow: visible;
  }

  .settings-panel {
    border-right: 0;
    border-bottom: 1px solid var(--docsy-border-subtle);
  }

  .filename-rule-keep {
    grid-template-columns: 78px minmax(0, 1fr) auto;
  }
}
</style>

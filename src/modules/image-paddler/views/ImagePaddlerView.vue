<template>
  <ToolWorkspaceShell title="图片排版" description="将图片批量排版为 A4 文档">
    <div ref="layoutRef" class="paddler-layout">
      <div class="settings-panel" :style="settingsPanelStyle">
        <el-form label-width="72px" size="small">
          <div class="settings-group-title">页面</div>

          <el-form-item :label="isFrameSequence ? '来源' : '素材'">
            <div class="source-actions">
              <el-button size="small" @click="addFolders">文件夹</el-button>
              <el-button size="small" @click="addImages">图片</el-button>
              <el-button v-if="folders.length" size="small" text type="danger" @click="clearAllSources">清空</el-button>
            </div>
            <div v-if="isFrameSequence" class="folder-path">
              {{ inputContext.sourceLabel || '抽帧结果' }}
              <template v-if="analysis"> · {{ analysis.images.length }} 张</template>
            </div>
            <div v-else-if="folders.length" class="source-tags">
              <el-tag
                v-for="(f, idx) in folders"
                :key="f"
                closable
                size="small"
                type="info"
                class="source-tag"
                @close="removeFolder(idx)"
              >
                {{ baseFileName(f) || f }}
              </el-tag>
            </div>
          </el-form-item>

          <el-form-item label="输出">
            <el-radio-group v-model="settings.output_format" size="small">
              <el-radio-button label="pdf">PDF</el-radio-button>
              <el-radio-button label="docx">DOCX</el-radio-button>
            </el-radio-group>
          </el-form-item>

          <el-form-item v-if="settings.output_format === 'docx'" label="排版">
            <el-radio-group v-model="settings.use_table" size="small">
              <el-radio-button :label="true">表格</el-radio-button>
              <el-radio-button :label="false">段落</el-radio-button>
            </el-radio-group>
            <div class="field-hint">{{ settings.use_table ? '可并排' : '仅上下' }}</div>
          </el-form-item>

          <el-form-item label="方向">
            <el-select v-model="settings.orientation" size="small">
              <el-option label="自动" value="auto" />
              <el-option label="竖向" value="portrait" />
              <el-option label="横向" value="landscape" />
            </el-select>
          </el-form-item>

          <el-form-item label="边距">
            <el-input-number v-model="settings.margin_mm" size="small" :min="0" :max="30" :step="1" controls-position="right" />
            <span class="unit-label">mm</span>
          </el-form-item>

          <div class="settings-group-title">每页布局</div>

          <el-form-item label="张数">
            <el-radio-group
              :model-value="settings.images_per_page"
              size="small"
              class="per-page-group"
              @update:model-value="setImagesPerPage"
            >
              <el-radio-button
                v-for="option in PER_PAGE_CHOICES"
                :key="String(option.value)"
                :label="option.value"
              >{{ option.value === 'custom' ? '自定' : option.value }}</el-radio-button>
            </el-radio-group>
            <div class="field-hint">末页可不足</div>
          </el-form-item>

          <el-form-item label="网格">
            <div class="arrange-row">
              <el-select
                :model-value="settings.arrange_mode"
                size="small"
                :disabled="isFlowLayout"
                @update:model-value="setArrangeMode"
              >
                <el-option
                  v-for="option in arrangeOptions"
                  :key="option.value"
                  :label="option.label"
                  :value="option.value"
                />
              </el-select>
              <span class="grid-chip">
                <i v-for="n in Math.min(perPage, 9)" :key="n">{{ n }}</i>
              </span>
            </div>
            <div class="field-hint">{{ layoutGrid.rows }}×{{ layoutGrid.cols }} · {{ perPage }} 张/页</div>
          </el-form-item>

          <el-form-item v-if="settings.images_per_page === 'custom' && !isFlowLayout" label="行列">
            <div class="inline-controls">
              <el-input-number
                :model-value="settings.custom_rows"
                size="small"
                :min="1"
                :max="8"
                controls-position="right"
                @update:model-value="(v) => { settings.custom_rows = v }"
              />
              <span>×</span>
              <el-input-number
                :model-value="settings.custom_cols"
                size="small"
                :min="1"
                :max="8"
                controls-position="right"
                @update:model-value="(v) => { settings.custom_cols = v }"
              />
            </div>
          </el-form-item>

          <el-form-item label="顺序">
            <el-select v-model="settings.order_mode" size="small" :disabled="isFlowLayout || perPage <= 1">
              <el-option label="Z 字" value="z" />
              <el-option v-if="!isFlowLayout" label="N 字" value="n" />
              <el-option v-if="!isFlowLayout" label="倒 N" value="reverse_n" />
              <el-option label="自定义" value="custom" />
            </el-select>
          </el-form-item>

          <el-form-item label="末页">
            <el-radio-group v-model="settings.last_page_mode" size="small">
              <el-radio-button label="keep">原网格</el-radio-button>
              <el-radio-button label="reflow">铺满</el-radio-button>
            </el-radio-group>
          </el-form-item>

          <el-form-item v-if="isPairLayout" label="双图">
            <el-select v-model="settings.pair_mode" size="small">
              <el-option label="居中" value="cell-center" />
              <el-option label="靠拢" value="page-gather" />
              <el-option label="分散" value="page-spread" />
            </el-select>
          </el-form-item>

          <el-form-item v-if="!isFrameSequence && folders.length > 1" label="多文件夹">
            <el-select v-model="settings.output_mode" size="small">
              <el-option label="合并" value="merged" />
              <el-option label="分文件夹" value="per_folder" />
            </el-select>
          </el-form-item>

          <div class="settings-group-title">图片尺寸</div>

          <el-form-item label="模式">
            <el-radio-group :model-value="settings.size_mode" size="small" @update:model-value="setSizeMode">
              <el-radio-button label="smart">智能</el-radio-button>
              <el-radio-button label="manual">手动</el-radio-button>
              <el-radio-button label="fit">适应</el-radio-button>
              <el-radio-button label="original">原图</el-radio-button>
            </el-radio-group>
            <div class="field-hint">{{ layoutAwareRecommendation?.reason }}</div>
          </el-form-item>

          <el-form-item v-if="settings.scale_mode === 'fixed_width'" label="宽度">
            <div class="fixed-width-control">
              <div class="width-input-row">
                <el-slider
                  v-model="settings.fixed_width_mm"
                  :min="0.1"
                  :max="Math.max(safeColumnWidthValue, 300)"
                  :step="0.1"
                  class="width-slider"
                  @input="markManualWidth"
                />
                <el-input-number
                  v-model="settings.fixed_width_mm"
                  size="small"
                  :min="0.1"
                  :max="500"
                  :step="0.1"
                  :precision="1"
                  controls-position="right"
                  class="width-num"
                  @change="markManualWidth"
                />
                <span class="unit-label">mm</span>
                <el-button size="small" text type="primary" @click="applyCurrentLayoutRecommendation">
                  {{ currentRecommendedWidth }}
                </el-button>
                <el-button size="small" text @click="restoreSmartSize">智能</el-button>
              </div>
              <div class="field-hint">
                {{ actualImageWidth.toFixed(0) }} / 栏宽 {{ safeColumnWidthValue.toFixed(0) }} mm
                <template v-if="widthIsLimited"> · 超宽</template>
              </div>
            </div>
          </el-form-item>

          <el-form-item label="边框">
            <div class="inline-controls">
              <el-switch v-model="settings.border_enabled" size="small" />
              <el-select v-model="settings.border_color" size="small" :disabled="!settings.border_enabled" class="border-select">
                <el-option label="黑" value="black" />
                <el-option label="白" value="white" />
                <el-option label="深灰" value="dark_gray" />
                <el-option label="浅灰" value="light_gray" />
                <el-option label="红" value="red" />
                <el-option label="黄" value="yellow" />
                <el-option label="蓝" value="blue" />
              </el-select>
            </div>
          </el-form-item>

          <div class="settings-group-title">标题说明</div>

          <el-form-item label="标题">
            <div class="filename-panel">
              <div class="filename-panel-row">
                <el-switch v-model="settings.show_filename" size="small" active-text="显示" />
                <el-switch v-model="settings.filename_without_ext" size="small" active-text="无扩展名" />
                <el-select v-model="settings.caption_position" size="small" :disabled="!settings.show_filename" class="cap-select">
                  <el-option label="图下" value="below" />
                  <el-option label="图上" value="above" />
                </el-select>
              </div>
              <div class="filename-panel-row">
                <label class="mini-label">间距</label>
                <el-slider
                  v-model="settings.caption_gap_mm"
                  :min="0"
                  :max="20"
                  :step="0.5"
                  :disabled="!settings.show_filename && !settings.reserve_note_placeholder"
                  class="gap-slider"
                />
                <el-input-number
                  v-model="settings.caption_gap_mm"
                  size="small"
                  :min="0"
                  :max="20"
                  :step="0.5"
                  :precision="1"
                  controls-position="right"
                  class="gap-num"
                  :disabled="!settings.show_filename && !settings.reserve_note_placeholder"
                />
                <span class="unit-label">mm</span>
              </div>
              <div class="filename-panel-row">
                <el-select v-model="settings.filename_font_family" size="small" :disabled="!settings.show_filename" class="font-select">
                  <el-option label="无衬线" value="sans" />
                  <el-option label="宋体" value="serif" />
                  <el-option label="楷体" value="kaiti" />
                  <el-option label="仿宋" value="fangsong" />
                </el-select>
                <el-input-number
                  v-model="settings.filename_font_size_pt"
                  size="small"
                  :min="6"
                  :max="24"
                  :step="1"
                  controls-position="right"
                  :disabled="!settings.show_filename"
                  class="font-size"
                />
                <span class="unit-label">pt</span>
                <el-select v-model="settings.filename_color" size="small" :disabled="!settings.show_filename" class="color-select">
                  <el-option label="深灰" value="dark_gray" />
                  <el-option label="黑" value="black" />
                  <el-option label="灰" value="gray" />
                  <el-option label="蓝" value="blue" />
                </el-select>
              </div>
              <div class="filename-panel-row">
                <el-switch v-model="settings.reserve_note_placeholder" size="small" active-text="说明栏" />
                <template v-if="settings.reserve_note_placeholder">
                  <el-select v-model="settings.note_font_family" size="small" class="font-select">
                    <el-option label="楷体" value="kaiti" />
                    <el-option label="仿宋" value="fangsong" />
                    <el-option label="宋体" value="serif" />
                    <el-option label="无衬线" value="sans" />
                  </el-select>
                  <el-input-number
                    v-model="settings.note_font_size_pt"
                    size="small"
                    :min="6"
                    :max="18"
                    controls-position="right"
                    class="font-size"
                  />
                  <span class="unit-label">pt</span>
                </template>
              </div>
              <el-input
                v-if="settings.reserve_note_placeholder"
                v-model="settings.note_placeholder_text"
                size="small"
                placeholder="未填说明时的占位文字"
              />
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
                    <el-option label="前缀" value="prefix" />
                    <el-option label="后缀" value="suffix" />
                    <el-option label="保留" value="keep" />
                  </el-select>
                  <template v-if="rule.kind === 'replace'">
                    <el-input v-model="rule.value" size="small" placeholder="原文" />
                    <el-input v-model="rule.replacement" size="small" placeholder="替换为" />
                  </template>
                  <template v-else-if="rule.kind === 'keep'">
                    <el-checkbox v-model="rule.keep_time" size="small">时</el-checkbox>
                    <el-checkbox v-model="rule.keep_number" size="small">号</el-checkbox>
                    <el-checkbox v-model="rule.keep_text" size="small">文</el-checkbox>
                    <el-input v-model="rule.replacement" size="small" placeholder="名称" />
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
              <el-button size="small" text @click="addFilenameRule">+ 规则</el-button>
            </div>
          </el-form-item>

          <el-form-item class="workspace-action-row">
            <div class="action-row">
              <el-button
                type="success"
                size="small"
                @click="handleStartGenerate"
                :loading="generating"
                :disabled="analyzing || !analysis || !includedImages.length"
              >
                生成文档
              </el-button>
              <el-switch v-model="settings.doclet_layout_tips" size="small" active-text="提示" />
              <span v-if="analyzing" class="analyze-hint">分析中…</span>
            </div>
          </el-form-item>
        </el-form>
      </div>

      <div
        class="panel-resizer"
        role="separator"
        aria-label="调整设置区宽度"
        aria-orientation="vertical"
        @pointerdown="onPanelResize"
      />

      <div class="result-panel">
        <template v-if="analysis">
          <div v-if="generatedResult" class="generated-result">
            <div class="generated-meta">
              <strong>已生成</strong>
              <div v-for="path in generatedOutputPaths" :key="path" class="output-path">{{ path }}</div>
              <div v-for="warning in generatedResult.warnings || []" :key="warning" role="alert" class="field-hint">{{ warning }}</div>
            </div>
            <el-button size="small" type="primary" text @click="openGeneratedOutput">打开</el-button>
          </div>

          <div v-if="isFrameSequence" class="sequence-mode-note">
            <strong>抽帧</strong>
            <span>{{ includedImages.length }} 张 / 排除 {{ excludedCount }}</span>
            <el-switch
              v-model="settings.use_source_exclusions"
              size="small"
              inline-prompt
              active-text="沿用排除"
              inactive-text="全载入"
            />
          </div>

          <div class="recommendation-bar">
            <div class="recommendation-block">
              <span class="recommendation-title">导入</span>
              <span class="recommendation-text">{{ analysis.recommended.reason }}</span>
              <el-button size="small" text @click="applyImportRecommendation">应用</el-button>
            </div>
            <div class="recommendation-block is-current">
              <span class="recommendation-title">当前</span>
              <span class="recommendation-text">
                {{ layoutAwareRecommendation?.reason }}
                <template v-if="importRecommendationDrifted"> · 已偏离导入方案</template>
              </span>
              <el-button size="small" text type="primary" @click="applyCurrentLayoutRecommendation">应用</el-button>
            </div>
          </div>

          <div class="preview-section">
            <div class="section-head">
              <div class="page-nav-group">
                <h4>预览</h4>
                <div class="page-nav-controls">
                  <el-button size="small" text :disabled="currentPageIndex <= 0" @click="prevPage">‹</el-button>
                  <span class="page-indicator">{{ currentPageIndex + 1 }}/{{ totalPages }}</span>
                  <el-button size="small" text :disabled="currentPageIndex >= totalPages - 1" @click="nextPage">›</el-button>
                </div>
              </div>
              <div class="preview-toolbar">
                <span>{{ previewImages.length }} 图</span>
                <el-button size="small" text @click="adjustPageZoom(-10)">−</el-button>
                <el-slider v-model="pageZoom" :min="20" :max="200" :step="5" class="zoom-slider" />
                <el-button size="small" text @click="adjustPageZoom(10)">+</el-button>
                <span class="zoom-value">{{ pageZoom }}%</span>
              </div>
            </div>

            <div class="page-scale-bar">
              <el-radio-group v-model="scaleScope" size="small">
                <el-radio-button label="all">全局</el-radio-button>
                <el-radio-button label="current">本页</el-radio-button>
              </el-radio-group>

              <template v-if="scaleScope === 'all'">
                <el-slider
                  :model-value="globalScalePercent"
                  :min="30"
                  :max="140"
                  :step="1"
                  class="page-scale-slider"
                  @input="setGlobalScale"
                />
                <span class="scale-percent">{{ globalScalePercent }}%</span>
                <el-button size="small" text type="primary" @click="autoFitAllPagesScale">智能</el-button>
                <el-button v-if="hasAnySavedScales || globalScalePercent !== 100" size="small" text type="danger" @click="resetAllPageScales">重置</el-button>
              </template>

              <template v-else>
                <el-slider
                  v-model="activePageScale"
                  :min="30"
                  :max="140"
                  :step="1"
                  class="page-scale-slider"
                />
                <span class="scale-percent">{{ activePageScale }}%</span>
                <el-button size="small" text type="primary" @click="autoFitCurrentPageScale">智能</el-button>
                <el-button size="small" text :disabled="!isCurrentPageDirty && !hasSavedScale" @click="saveCurrentPageScale">应用</el-button>
                <el-button size="small" text @click="cancelCurrentPageScale">取消</el-button>
                <el-button v-if="hasSavedScale" size="small" text type="info" @click="resetCurrentPageScale">恢复</el-button>
              </template>
            </div>

            <div
              v-if="settings.doclet_layout_tips && currentPageConflictState.worstColor !== 'ok'"
              class="doclet-tip-bar"
              :class="`tip-${currentPageConflictState.worstColor}`"
            >
              <DocletSprite :size="22" :motion="currentPageConflictState.worstColor === 'blue' ? 'review' : 'working'" />
              <span class="doclet-tip-text">{{ currentPageConflictState.docletTip }}</span>
            </div>

            <div class="page-preview-shell">
              <div class="page-preview" :class="resolvedOrientation" :style="previewPageStyle">
                <div
                  class="preview-grid"
                  :class="{ 'preview-grid-flow': settings.output_format === 'docx' && !settings.use_table }"
                  :style="previewGridStyle"
                >
                  <div
                    v-for="(img, idx) in previewSlots"
                    :key="idx"
                    class="preview-cell"
                    :class="{
                      'preview-cell-bordered': settings.border_enabled,
                      'preview-cell-white-border': settings.border_enabled && settings.border_color === 'white',
                      'preview-cell-no-name': !hasCellCaption(img),
                      'preview-cell-flow': settings.output_format === 'docx' && !settings.use_table,
                    }"
                    :style="previewCellStyle"
                  >
                    <template v-if="img">
                      <div
                        v-if="settings.caption_position === 'above' && hasCellCaption(img)"
                        class="preview-caption preview-caption-above"
                        :style="previewCaptionGapStyle"
                      >
                        <div v-if="settings.show_filename" class="preview-name preview-name-above" :style="previewNameStyle">
                          <span v-for="(line, lineIdx) in fileNameLines(img.path)" :key="`${lineIdx}-${line}`">{{
                            line
                          }}</span>
                        </div>
                        <div v-if="noteLines(img.path).length" class="preview-note" :style="previewNoteStyle">
                          <span v-for="(line, lineIdx) in noteLines(img.path)" :key="`note-${lineIdx}-${line}`">{{
                            line
                          }}</span>
                        </div>
                      </div>
                      <div
                        class="preview-image-area"
                        :style="[previewImageAreaStyle, previewImageAreaContainerStyle(idx)]"
                      >
                        <img :src="imageSrc(img.path)" :alt="fileName(img.path)" :style="previewImageStyle(img)" />
                      </div>
                      <div
                        v-if="settings.caption_position !== 'above' && hasCellCaption(img)"
                        class="preview-caption"
                        :style="previewCaptionGapStyle"
                      >
                        <div v-if="settings.show_filename" class="preview-name" :style="previewNameStyle">
                          <span v-for="(line, lineIdx) in fileNameLines(img.path)" :key="`${lineIdx}-${line}`">{{
                            line
                          }}</span>
                        </div>
                        <div v-if="noteLines(img.path).length" class="preview-note" :style="previewNoteStyle">
                          <span v-for="(line, lineIdx) in noteLines(img.path)" :key="`note-${lineIdx}-${line}`">{{
                            line
                          }}</span>
                        </div>
                      </div>
                    </template>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="image-list">
            <div class="section-head">
              <h4>图片</h4>
            </div>
            <ReorderableImageGrid
              :items="orderedImages"
              :name-resolver="imageItemName"
              :meta-resolver="imageItemMeta"
              :excluded-resolver="isImageExcluded"
              :page-badge-resolver="imageBadgeResolver"
              :annotation-resolver="getImageAnnotation"
              preserve-aspect-ratio
              :initial-zoom="130"
              v-model:page-fraction="previewPageFraction"
              empty-description="暂无图片"
              @reorder="reorderLayoutImages"
              @toggle-excluded="toggleImageExclusion"
              @select-page="goToPage"
              @update-annotation="handleUpdateAnnotation"
            />
          </div>
        </template>
        <WorkspaceEmptyState
          v-else
          class="result-empty-state"
          :icon-url="imageLayoutIconUrl"
          :title="analyzing ? '正在分析图片' : '等待选择图片'"
          :description="analyzing ? '读取尺寸中…' : '选择文件夹或图片后自动分析'"
        />
      </div>
    </div>

    <el-dialog
      v-model="conflictDialogVisible"
      title="排版冲突"
      width="520px"
      append-to-body
      destroy-on-close
    >
      <div class="conflict-dialog-intro">
        {{ conflictSummaryList.length }} 处可能重叠/超界，可直接生成或返回调整。
      </div>
      <el-table :data="conflictSummaryList" max-height="220" size="small" style="width: 100%">
        <el-table-column prop="pageNumber" label="页" width="56" align="center">
          <template #default="{ row }">P{{ row.pageNumber }}</template>
        </el-table-column>
        <el-table-column prop="displayName" label="图片" min-width="140" show-overflow-tooltip />
        <el-table-column prop="reason" label="情况" min-width="120">
          <template #default="{ row }">
            <span :class="`conflict-tag-${row.color}`">{{ row.reason }}</span>
          </template>
        </el-table-column>
        <el-table-column label="" width="56" align="center">
          <template #default="{ row }">
            <el-button link type="primary" size="small" @click="goToConflictPage(row.pageIndex)">查看</el-button>
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <span class="dialog-footer">
          <el-button size="small" @click="conflictDialogVisible = false">返回</el-button>
          <el-button size="small" type="primary" @click="executeForceGenerate">直接生成</el-button>
        </span>
      </template>
    </el-dialog>
  </ToolWorkspaceShell>
</template>
<script setup>
import { computed, ref } from 'vue'
import ToolWorkspaceShell from '../../../shared/components/ToolWorkspaceShell.vue'
import ReorderableImageGrid from '../../../shared/components/ReorderableImageGrid.vue'
import WorkspaceEmptyState from '../../../shared/components/WorkspaceEmptyState.vue'
import DocletSprite from '../../../shared/components/DocletSprite.vue'
import imageLayoutIconUrl from '../../../assets/icons/image-layout.svg?url'
import { useImagePaddlerState } from '../composables/useImagePaddlerState.js'
import { useWorkspaceStore } from '../../../stores/workspace.js'

const workspaceStore = useWorkspaceStore()
const layoutRef = ref(null)

const {
  isFlowLayout,
  arrangeOptions,
  PER_PAGE_CHOICES,
  safeColumnWidthValue,
  actualImageWidth,
  widthIsLimited,
  folders,
  analyzing,
  generating,
  analysis,
  generatedResult,
  inputContext,
  isFrameSequence,
  pageZoom,
  previewPageFraction,
  settings,
  layoutGrid,
  resolvedOrientation,
  orderedImages,
  includedImages,
  excludedCount,
  perPage,
  isPairLayout,
  totalPages,
  currentPageIndex,
  activePageScale,
  scaleScope,
  globalScalePercent,
  setGlobalScale,
  hasSavedScale,
  hasAnySavedScales,
  isCurrentPageDirty,
  saveCurrentPageScale,
  cancelCurrentPageScale,
  resetCurrentPageScale,
  autoFitCurrentPageScale,
  autoFitAllPagesScale,
  resetAllPageScales,
  layoutAwareRecommendation,
  importRecommendationDrifted,
  applyImportRecommendation,
  applyCurrentLayoutRecommendation,
  restoreSmartSize,
  markManualWidth,
  setImagesPerPage,
  setArrangeMode,
  setSizeMode,
  settingsPanelWidth,
  startSettingsPanelResize,
  nextPage,
  prevPage,
  goToPage,
  previewImages,
  previewSlots,
  generatedOutputPaths,
  previewPageStyle,
  previewGridStyle,
  previewCellStyle,
  previewImageAreaStyle,
  previewImageAreaContainerStyle,
  previewCaptionGapStyle,
  previewNameStyle,
  previewNoteStyle,
  addFolders,
  addImages,
  removeFolder,
  clearAllSources,
  currentRecommendedWidth,
  baseFileName,
  reorderLayoutImages,
  openGeneratedOutput,
  imageSrc,
  fileName,
  fileNameLines,
  previewImageStyle,
  imageItemName,
  imageItemMeta,
  getImageAnnotation,
  setImageAnnotation,
  noteLines,
  isImageExcluded,
  toggleImageExclusion,
  addFilenameRule,
  removeFilenameRule,
  rulePlaceholder,
  adjustPageZoom,
  imageBadgeResolver,
  currentPageConflictState,
  conflictDialogVisible,
  conflictSummaryList,
  handleStartGenerate,
  run,
} = useImagePaddlerState({
  initialTransfer: () => workspaceStore.mediaTransfer,
  onInitialPathsLoaded: () => workspaceStore.clearMediaTransfer(),
})

const settingsPanelStyle = computed(() => ({
  flex: `0 0 ${settingsPanelWidth.value}px`,
  width: `${settingsPanelWidth.value}px`,
}))

function onPanelResize(event) {
  startSettingsPanelResize(event, layoutRef.value)
}

function goToConflictPage(pageIndex) {
  currentPageIndex.value = pageIndex
  conflictDialogVisible.value = false
}

async function executeForceGenerate() {
  conflictDialogVisible.value = false
  await run()
}

function handleUpdateAnnotation({ path, title, description }) {
  setImageAnnotation(path, { title, description })
}

function hasCellCaption(img) {
  if (!img) return false
  return Boolean(settings.show_filename || noteLines(img.path).length)
}
</script>

<style scoped>
:deep(.workspace-content) {
  overflow: hidden;
}

.paddler-layout {
  display: flex;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-panel-radius);
  box-shadow: var(--docsy-shadow-panel);
}

.settings-panel {
  min-width: 280px;
  max-width: 520px;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 12px 14px 16px;
  background: var(--docsy-surface-elevated);
  flex-shrink: 0;
}

.panel-resizer {
  flex: 0 0 8px;
  align-self: stretch;
  cursor: col-resize;
  touch-action: none;
  background: linear-gradient(
    90deg,
    transparent 2px,
    var(--docsy-border-subtle) 2px,
    var(--docsy-border-subtle) 6px,
    transparent 6px
  );
}

.panel-resizer:hover {
  background: linear-gradient(
    90deg,
    transparent 2px,
    var(--docsy-primary) 2px,
    var(--docsy-primary) 6px,
    transparent 6px
  );
}

.result-panel {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding: 12px 14px;
  background: color-mix(in srgb, var(--docsy-surface-muted) 82%, var(--docsy-canvas));
}

.settings-group-title {
  margin: 10px 0 6px;
  padding-top: 6px;
  border-top: 1px dashed var(--docsy-border-subtle);
  font-size: 12px;
  font-weight: 700;
  color: var(--docsy-text-strong);
}

.settings-group-title:first-child {
  margin-top: 0;
  padding-top: 0;
  border-top: none;
}

.settings-panel :deep(.el-form-item) {
  margin-bottom: 10px;
}

.settings-panel :deep(.el-form-item__content) {
  line-height: 1.4;
}

.field-hint {
  font-size: 11px;
  color: var(--docsy-text-muted);
  line-height: 1.3;
  margin-top: 2px;
  width: 100%;
}

.unit-label {
  margin-left: 4px;
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.folder-path {
  font-size: 11px;
  color: var(--docsy-text-muted);
  word-break: break-all;
}

.source-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.source-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
  max-height: 64px;
  overflow-y: auto;
}

.source-tag {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.inline-controls {
  display: flex;
  align-items: center;
  gap: 6px;
}

.inline-controls :deep(.el-input-number) {
  width: 72px;
}

.per-page-group {
  display: flex;
  flex-wrap: wrap;
}

.arrange-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.arrange-row :deep(.el-select) {
  flex: 1;
  min-width: 0;
}

.grid-chip {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 3px;
  max-width: 88px;
}

.grid-chip i {
  width: 18px;
  height: 18px;
  border-radius: 3px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-size: 10px;
  font-style: normal;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.border-select {
  width: 78px;
}

.fixed-width-control {
  width: 100%;
}

.width-input-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}

.width-slider {
  flex: 1;
  min-width: 0;
  margin: 0;
}

.width-num {
  width: 78px;
}

.action-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.analyze-hint {
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.filename-panel {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.filename-panel-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}

.mini-label {
  font-size: 11px;
  color: var(--docsy-text-muted);
  flex: 0 0 auto;
}

.gap-slider {
  flex: 1;
  min-width: 60px;
  margin: 0;
}

.gap-num {
  width: 72px;
}

.cap-select {
  width: 72px;
}

.font-select {
  width: 88px;
}

.font-size {
  width: 68px;
}

.color-select {
  width: 72px;
}

.filename-rules {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.filename-rule {
  display: grid;
  grid-template-columns: 64px minmax(0, 1fr) auto;
  gap: 4px;
  align-items: center;
}

.filename-rule-keep {
  grid-template-columns: 64px auto auto auto minmax(0, 1fr) 56px auto;
}

.rule-kind {
  width: 64px;
}

.separator-select {
  width: 56px;
}

.workspace-action-row {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--docsy-border-subtle);
}

.generated-result {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: flex-start;
  padding: 8px 10px;
  margin-bottom: 8px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
  font-size: 12px;
}

.output-path {
  margin-top: 2px;
  color: var(--docsy-text);
  word-break: break-all;
  font-size: 11px;
}

.sequence-mode-note {
  display: flex;
  margin-bottom: 8px;
  padding: 6px 10px;
  align-items: center;
  gap: 8px;
  border: 1px solid color-mix(in srgb, var(--docsy-primary) 28%, var(--docsy-border-subtle));
  border-radius: var(--docsy-radius);
  color: var(--docsy-text-muted);
  background: var(--docsy-primary-soft);
  font-size: 12px;
}

.sequence-mode-note :deep(.el-switch) {
  margin-left: auto;
}

.recommendation-bar {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
}

.recommendation-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-elevated);
  border-radius: var(--docsy-radius);
  font-size: 12px;
}

.recommendation-block.is-current {
  background: var(--docsy-primary-soft);
  border-color: color-mix(in srgb, var(--docsy-primary) 28%, var(--docsy-border-subtle));
}

.recommendation-title {
  font-weight: 650;
  color: var(--docsy-text-strong);
  flex: 0 0 auto;
}

.recommendation-text {
  flex: 1 1 auto;
  min-width: 0;
  color: var(--docsy-text);
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-section {
  margin-bottom: 10px;
}

.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
  font-size: 12px;
  color: var(--docsy-text-muted);
}

.section-head h4 {
  margin: 0;
  font-size: 13px;
  font-weight: 680;
  color: var(--docsy-text-strong);
}

.page-nav-group,
.page-nav-controls,
.preview-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
}

.page-indicator {
  font-size: 12px;
  font-weight: 600;
  color: var(--docsy-text);
  min-width: 36px;
  text-align: center;
}

.zoom-slider {
  width: 90px;
}

.zoom-value {
  width: 36px;
  text-align: right;
  font-size: 11px;
}

.page-scale-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  margin-bottom: 6px;
  background: var(--docsy-surface-elevated);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
}

.page-scale-slider {
  flex: 1;
  min-width: 80px;
}

.scale-percent {
  font-size: 12px;
  font-weight: 600;
  min-width: 36px;
  color: var(--docsy-primary);
}

.doclet-tip-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  margin-bottom: 6px;
  border-radius: var(--docsy-radius);
  font-size: 12px;
  border: 1px solid transparent;
}

.doclet-tip-bar.tip-yellow {
  background: rgba(253, 246, 236, 0.95);
  border-color: rgba(183, 121, 52, 0.35);
  color: #8a541c;
}

.doclet-tip-bar.tip-green {
  background: rgba(234, 243, 222, 0.95);
  border-color: rgba(79, 125, 90, 0.35);
  color: #395c41;
}

.doclet-tip-bar.tip-red {
  background: rgba(252, 235, 235, 0.95);
  border-color: rgba(181, 82, 75, 0.35);
  color: #943b35;
}

.doclet-tip-bar.tip-blue {
  background: rgba(237, 244, 248, 0.95);
  border-color: rgba(82, 121, 153, 0.35);
  color: #2d5873;
}

.page-preview-shell {
  display: block;
  padding: 14px;
  background: #deddd8;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  overflow: auto;
}

.page-preview {
  position: relative;
  container-type: inline-size;
  box-sizing: border-box;
  background: #fff;
  border: 1px solid var(--docsy-border-strong);
  box-shadow: 0 10px 24px rgba(42, 39, 34, 0.12);
  margin: 0 auto;
}

.preview-grid {
  position: absolute;
  display: grid;
  gap: 0;
}

.preview-grid-flow {
  row-gap: 0;
}

.preview-cell {
  box-sizing: border-box;
  min-width: 0;
  min-height: 0;
  background: var(--docsy-surface-elevated);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  overflow: visible;
  position: relative;
}

.preview-cell-flow {
  border: none !important;
}

.preview-image-area {
  box-sizing: border-box;
  width: 100%;
  min-height: 0;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: visible;
  position: relative;
}

.preview-cell img {
  display: block;
}

.preview-caption {
  box-sizing: border-box;
  width: 100%;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  overflow: hidden;
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

.preview-name span,
.preview-note span {
  display: block;
}

.preview-note {
  box-sizing: border-box;
  width: 100%;
  flex-shrink: 0;
  padding: 0 4px;
  text-align: center;
  overflow: hidden;
  overflow-wrap: anywhere;
}

.result-empty-state {
  height: 100%;
  min-height: clamp(240px, 36dvh, 360px);
  box-sizing: border-box;
}

.conflict-dialog-intro {
  margin-bottom: 8px;
  font-size: 12px;
  color: var(--docsy-text);
}

.conflict-tag-red {
  color: #943b35;
}

.conflict-tag-yellow {
  color: #8a541c;
}

.conflict-tag-green {
  color: #395c41;
}

.image-list h4 {
  margin: 0 0 6px;
  font-size: 13px;
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
    width: 100% !important;
    max-width: none;
    flex: none !important;
    overflow: visible;
  }

  .settings-panel {
    border-bottom: 1px solid var(--docsy-border-subtle);
  }

  .panel-resizer {
    display: none;
  }

  .filename-rule-keep {
    grid-template-columns: 64px minmax(0, 1fr) auto;
  }
}
</style>

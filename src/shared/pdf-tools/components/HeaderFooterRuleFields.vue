<template>
  <!-- 根元素为普通纵向容器，不再由使用方挂 grid class；
       每个区块独立一张网格，区块内 v-if 行增删只重排本区块，避免整体跳动 -->
  <div class="hf-rule-fields compact-options">
    <section class="hf-section">
      <div class="rule-item section-label">
        <div class="section-title">
          <strong>页眉文字</strong
          ><el-switch v-model="headerInsertEnabledModel" size="small" aria-label="启用页眉文字" />
        </div>
        <div class="section-actions">
          <el-button
            size="small"
            aria-label="添加页眉规则"
            title="添加页眉规则"
            plain
            :disabled="!headerInsertEnabled"
            @click="addGroup"
          >
            <el-icon><Plus /></el-icon><span>添加规则</span>
          </el-button>
          <UndoRedoButtons
            :can-undo="headerHistory.canUndo.value"
            :can-redo="headerHistory.canRedo.value"
            @undo="headerHistory.undo()"
            @redo="headerHistory.redo()"
            compact
          />
        </div>
      </div>
      <template v-if="headerInsertEnabled">
        <!-- Overlap warnings -->
        <el-alert v-if="overlapWarnings.length" type="warning" :closable="false" show-icon class="overlap-warnings">
          <template #title>{{ overlapWarnings.join('；') }}</template>
        </el-alert>
        <!-- Group list (only when >1 group) -->
        <div v-if="headerGroups.length > 1" class="header-group-list">
          <div
            v-for="group in headerGroups"
            :key="group.id"
            class="header-group-item"
            :class="{ active: group.id === selectedHeaderGroupId }"
            @click="$emit('update:selectedHeaderGroupId', group.id)"
          >
            <span class="group-label">
              <el-icon v-if="group.id === selectedHeaderGroupId"><ArrowRight /></el-icon>
              {{ group.label || '页眉' }} · {{ groupModeLabel(group) }}
            </span>
            <el-button
              size="small"
              link
              type="danger"
              :disabled="headerGroups.length <= 1"
              @click.stop="removeGroup(group.id)"
            >
              删除
            </el-button>
          </div>
        </div>
        <!-- Settings for selected group -->
        <div class="hf-section-grid">
          <div class="rule-item">
            <label>页眉来源</label>
            <el-select v-model="headerModeModel">
              <el-option label="文件名" value="filename" />
              <el-option label="固定文本" value="custom" />
              <el-option label="按证据列表名称" value="per_file" />
            </el-select>
          </div>
          <div v-if="headerMode === 'per_file'" class="rule-item">
            <label>名称前缀</label>
            <el-input v-model="headerPerFilePrefixModel" placeholder="证据" />
          </div>
          <div v-if="headerMode === 'per_file'" class="rule-item">
            <label>序号类型</label>
            <el-select v-model="headerPerFileSeqTypeModel">
              <el-option label="数字（1, 2, 3）" value="numeric" />
              <el-option label="中文（一, 二, 三）" value="chinese" />
            </el-select>
          </div>
          <div v-if="headerMode === 'per_file'" class="rule-item">
            <label>编号设置</label>
            <el-select v-model="headerNumberingSourceModel">
              <el-option label="跟随全局编号" value="default" />
              <el-option label="本规则单独设置" value="custom" />
            </el-select>
          </div>
          <div v-if="headerMode === 'per_file'" class="rule-item">
            <label
              >序号起始<HelpTooltip
                :content="
                  headerNumberingSourceModel === 'default' ? '跟随全局，选择单独设置后可修改。' : '仅覆盖本规则。'
                "
            /></label>
            <el-input-number
              v-model="effectiveHeaderSeqStartModel"
              :min="0"
              :max="9999"
              size="small"
              controls-position="right"
              :disabled="headerNumberingSourceModel === 'default'"
            />
          </div>
          <div v-if="headerMode === 'custom'" class="rule-item">
            <label>页眉文本</label>
            <div class="text-input-with-info">
              <el-input v-model="headerTextModel" placeholder="输入文本或模板标记" />
              <el-tooltip placement="top" :show-after="200" class="template-info-tip">
                <template #content>
                  <div class="template-help">
                    <p><strong>模板标记（自动替换）：</strong></p>
                    <p><code>[文件名]</code> — 去掉扩展名的文件名</p>
                    <p><code>[序号]</code> — 文件序号（1, 2, 3...）</p>
                    <p><code>[中文序号]</code> — 中文序号（一、二、三...）</p>
                    <p><code>[#]</code> / <code>[##]</code> / <code>[###]</code> — 序号，位数=#个数（01, 001...）</p>
                    <p><code>[日期]</code> — 当前日期（YYYYMMDD）</p>
                    <p><code>[YYYY-MM-DD]</code> — 自定义日期格式</p>
                    <p style="margin-top: 6px; color: var(--el-text-color-placeholder)">
                      可与固定文字混合使用，如"证据[#]-[文件名]"
                    </p>
                  </div>
                </template>
                <el-icon class="info-icon"><InfoFilled /></el-icon>
              </el-tooltip>
            </div>
          </div>
          <div class="rule-item">
            <label>页眉前缀</label><el-input v-model="headerPrefixModel" :disabled="headerMode === 'none'" />
          </div>
          <div class="rule-item">
            <label>页眉后缀</label><el-input v-model="headerSuffixModel" :disabled="headerMode === 'none'" />
          </div>
        </div>
        <div class="settings-group-label">文字与位置</div>
        <div class="hf-section-grid">
          <TextPlacementFields
            prefix="页眉"
            :disabled="headerMode === 'none'"
            v-model:align="headerAlignModel"
            v-model:font-size="headerFontSizeModel"
            v-model:font-family="headerFontFamilyModel"
            v-model:margin-mm="headerMarginMmModel"
            v-model:offset-x-mm="headerOffsetXMmModel"
            v-model:color="headerColorModel"
            :offset-limit-mm="offsetLimitMm"
            margin-label="距顶"
          />
          <div class="rule-item page-range-row">
            <label>添加范围<HelpTooltip content="结束页填 0，表示一直添加到文档末页。" /></label>
            <div class="page-range-inputs">
              <el-checkbox v-model="headerAllPages" size="small">全部页面</el-checkbox>
              <template v-if="!headerAllPages">
                <el-input-number
                  v-model="headerPageStartModel"
                  :min="1"
                  :max="9999"
                  size="small"
                  controls-position="right"
                />
                <span class="range-sep">–</span>
                <el-input-number
                  v-model="headerPageEndModel"
                  :min="0"
                  :max="filePages || 9999"
                  size="small"
                  controls-position="right"
                />
              </template>
            </div>
          </div>
        </div>
      </template>
    </section>

    <section class="hf-section">
      <div class="rule-item section-label">
        <div class="section-title">
          <strong>页脚文字</strong
          ><el-switch v-model="footerInsertEnabledModel" size="small" aria-label="启用页脚文字" />
        </div>
        <div class="section-actions">
          <el-button
            size="small"
            aria-label="添加页脚规则"
            title="添加页脚规则"
            plain
            :disabled="!footerInsertEnabled"
            @click="addFooterTextGroup"
          >
            <el-icon><Plus /></el-icon><span>添加规则</span>
          </el-button>
          <UndoRedoButtons
            :can-undo="footerTextHistory.canUndo.value"
            :can-redo="footerTextHistory.canRedo.value"
            @undo="footerTextHistory.undo()"
            @redo="footerTextHistory.redo()"
            compact
          />
        </div>
      </div>
      <template v-if="footerInsertEnabled">
        <!-- Footer text group list (only when >1 group) -->
        <div v-if="footerTextGroups.length > 1" class="header-group-list">
          <div
            v-for="group in footerTextGroups"
            :key="group.id"
            class="header-group-item"
            :class="{ active: group.id === selectedFooterTextGroupId }"
            @click="$emit('update:selectedFooterTextGroupId', group.id)"
          >
            <span class="group-label">
              <el-icon v-if="group.id === selectedFooterTextGroupId"><ArrowRight /></el-icon>
              {{ group.label || '页脚文字' }} · {{ group.text || '（空）' }}
            </span>
            <el-button
              size="small"
              link
              type="danger"
              :disabled="footerTextGroups.length <= 1"
              @click.stop="removeFooterTextGroup(group.id)"
            >
              删除
            </el-button>
          </div>
        </div>
        <div class="hf-section-grid">
          <div class="rule-item">
            <label>页脚文本</label>
            <div class="text-input-with-info">
              <el-input v-model="footerTextContentModel" placeholder="输入文本或模板标记" />
              <el-tooltip placement="top" :show-after="200" class="template-info-tip">
                <template #content>
                  <div class="template-help">
                    <p><strong>模板标记（自动替换）：</strong></p>
                    <p><code>[文件名]</code> — 去掉扩展名的文件名</p>
                    <p><code>[序号]</code> — 文件序号（1, 2, 3...）</p>
                    <p><code>[中文序号]</code> — 中文序号（一、二、三...）</p>
                    <p><code>[#]</code> / <code>[##]</code> / <code>[###]</code> — 序号，位数=#个数（01, 001...）</p>
                    <p><code>[日期]</code> — 当前日期（YYYYMMDD）</p>
                    <p><code>[YYYY-MM-DD]</code> — 自定义日期格式</p>
                    <p style="margin-top: 6px; color: var(--el-text-color-placeholder)">
                      可与固定文字混合使用，如"证据[#]-[文件名]"
                    </p>
                  </div>
                </template>
                <el-icon class="info-icon"><InfoFilled /></el-icon>
              </el-tooltip>
            </div>
          </div>
        </div>
        <div class="settings-group-label">文字与位置</div>
        <div class="hf-section-grid">
          <TextPlacementFields
            prefix="页脚"
            v-model:align="footerTextAlignModel"
            v-model:font-size="footerTextFontSizeModel"
            v-model:font-family="footerTextFontFamilyModel"
            v-model:margin-mm="footerTextMarginMmModel"
            v-model:offset-x-mm="footerTextOffsetXMmModel"
            v-model:color="footerTextColorModel"
            :offset-limit-mm="offsetLimitMm"
            margin-label="距底"
          />
          <div class="rule-item page-range-row">
            <label>添加范围<HelpTooltip content="结束页填 0，表示一直添加到文档末页。" /></label>
            <div class="page-range-inputs">
              <el-checkbox v-model="footerTextAllPages" size="small">全部页面</el-checkbox>
              <template v-if="!footerTextAllPages">
                <el-input-number
                  v-model="footerTextPageStartModel"
                  :min="1"
                  :max="9999"
                  size="small"
                  controls-position="right"
                />
                <span class="range-sep">–</span>
                <el-input-number
                  v-model="footerTextPageEndModel"
                  :min="0"
                  :max="filePages || 9999"
                  size="small"
                  controls-position="right"
                />
              </template>
            </div>
          </div>
        </div>
      </template>
    </section>

    <section class="hf-section">
      <div class="rule-item section-label">
        <div class="section-title">
          <strong>页码</strong><el-switch v-model="pageNumberEnabledModel" size="small" aria-label="启用页码" />
        </div>
        <div class="section-actions">
          <el-button
            size="small"
            aria-label="添加页码规则"
            title="添加页码规则"
            plain
            :disabled="!pageNumberEnabled"
            @click="addPageNumberGroup"
          >
            <el-icon><Plus /></el-icon><span>添加规则</span>
          </el-button>
          <UndoRedoButtons
            :can-undo="pageNumberHistory.canUndo.value"
            :can-redo="pageNumberHistory.canRedo.value"
            @undo="pageNumberHistory.undo()"
            @redo="pageNumberHistory.redo()"
            compact
          />
        </div>
      </div>
      <template v-if="pageNumberEnabled">
        <!-- Page number group list (only when >1 group) -->
        <div v-if="pageNumberGroups.length > 1" class="header-group-list">
          <div
            v-for="group in pageNumberGroups"
            :key="group.id"
            class="header-group-item"
            :class="{ active: group.id === selectedPageNumberGroupId }"
            @click="$emit('update:selectedPageNumberGroupId', group.id)"
          >
            <span class="group-label">
              <el-icon v-if="group.id === selectedPageNumberGroupId"><ArrowRight /></el-icon>
              {{ group.label || '页码' }} · {{ group.template || '{page}/{total}' }}
            </span>
            <el-button
              size="small"
              link
              type="danger"
              :disabled="pageNumberGroups.length <= 1"
              @click.stop="removePageNumberGroup(group.id)"
            >
              删除
            </el-button>
          </div>
        </div>
        <div class="settings-group-label">编号与格式</div>
        <div class="hf-section-grid">
          <div class="rule-item">
            <label>页码起始<HelpTooltip content="总页数随起始页码自动偏移。" /></label>
            <el-input-number v-model="pageStartModel" :min="1" :max="999999" controls-position="right" />
          </div>
          <div class="rule-item">
            <label>连续方式</label>
            <el-select v-model="pageSequenceModel"
              ><el-option label="全部文件连续" value="continuous" /><el-option
                label="每个文件单独编号"
                value="per-file"
            /></el-select>
          </div>
          <div class="rule-item">
            <label>总页数口径</label>
            <el-select v-model="pageTotalModeModel"
              ><el-option label="合并后总页数" value="combined" /><el-option label="单个文件页数" value="per-file"
            /></el-select>
          </div>
          <div class="rule-item">
            <label>数字样式</label>
            <el-select v-model="pageNumberStyleModel"
              ><el-option
                v-for="style in PAGE_NUMBER_STYLES"
                :key="style.value"
                :value="style.value"
                :label="`${style.label} · ${style.sample}`"
            /></el-select>
          </div>
          <div class="rule-item">
            <label>显示总页数</label
            ><el-switch v-model="pageNumberShowTotalModel" active-text="开" inactive-text="关" />
          </div>
          <div class="rule-item">
            <label>页码格式</label>
            <el-select v-model="pageNumberPresetModel" aria-label="页码格式" @change="applyPresetTemplate">
              <el-option
                v-for="preset in filteredPageNumberPresets"
                :key="preset.value"
                :value="preset.value"
                :label="renderPageNumberTemplate(preset.value, 1, 35, pageNumberStyle)"
              />
              <el-option label="自定义格式" value="custom" />
            </el-select>
          </div>
          <div v-if="pageNumberPresetModel === 'custom'" class="rule-item custom-format-field">
            <label
              >自定义格式<HelpTooltip
                content="{page} 为当前页码，{total} 为总页数，{range} 为当前页码/总页数；数字按所选样式显示。"
            /></label>
            <el-input
              v-model="pageNumberTemplateModel"
              placeholder="例如 第{page}页，共{total}页"
              aria-label="自定义页码格式"
            />
          </div>
        </div>
        <div class="settings-group-label">文字与位置</div>
        <div class="hf-section-grid">
          <div class="rule-item">
            <label>页码区域</label
            ><el-select v-model="pageNumberRegionModel"
              ><el-option label="页脚区域" value="footer" /><el-option label="页眉区域" value="header"
            /></el-select>
          </div>
          <TextPlacementFields
            prefix="页码"
            v-model:align="pageNumberAlignModel"
            v-model:font-size="pageNumberFontSizeModel"
            v-model:font-family="pageNumberFontFamilyModel"
            v-model:margin-mm="pageNumberMarginMmModel"
            v-model:offset-x-mm="pageNumberOffsetXMmModel"
            v-model:color="pageNumberColorModel"
            :offset-limit-mm="offsetLimitMm"
            :margin-label="pageNumberRegion === 'header' ? '距顶' : '距底'"
          />
          <div class="rule-item">
            <label
              >双面装订<HelpTooltip
                content="根据所选翻页方式调整奇偶页页码的位置；请与实际打印的翻页设置保持一致。" /></label
            ><el-select v-model="pageNumberDuplexModeModel"
              ><el-option label="单面固定（不镜像）" value="none" /><el-option
                label="长边翻页（左右外侧对开）"
                value="long-edge" /><el-option label="短边翻页（对开翻转）" value="short-edge"
            /></el-select>
          </div>
        </div>
        <section class="page-number-sample" aria-label="页码格式预览">
          <div class="sample-heading">
            <span>格式预览</span
            ><el-button v-if="showRuleEditor" size="small" text @click="$emit('editPageNumberRules')"
              >分段与例外{{ pageNumberOverrideCount ? `（${pageNumberOverrideCount}）` : '' }}</el-button
            >
          </div>
          <output class="page-number-preview" aria-live="polite">{{ pageNumberPreviewText }}</output>
          <span class="sample-caption">当前编号示例，纸面位置以页面预览为准</span>
        </section>
      </template>
    </section>
  </div>
</template>

<script setup>
import HelpTooltip from '../../components/HelpTooltip.vue'
import { computed, ref, watch } from 'vue'
import { pageNumberPresets, templateForTotal, usesPageTotal } from '../pageNumberFormats.js'
import { Plus, InfoFilled, ArrowRight } from '@element-plus/icons-vue'
import TextPlacementFields from './TextPlacementFields.vue'
import { PAGE_NUMBER_STYLES, renderPageNumberTemplate } from '../composables/pdfPageNumberRules.js'
import { useHistory } from '../../../core/composables/useHistory.js'
import UndoRedoButtons from '../../components/UndoRedoButtons.vue'

function debounce(fn, ms) {
  let timer
  return (...args) => {
    clearTimeout(timer)
    timer = setTimeout(() => fn(...args), ms)
  }
}

const MODE_LABELS = {
  filename: '文件名',
  per_file: '按列表名称',
  custom: '固定文本',
}

const props = defineProps({
  showRuleEditor: { type: Boolean, default: true },
  headerGroups: { type: Array, required: true },
  selectedHeaderGroupId: { type: String, required: true },
  headerInsertEnabled: { type: Boolean, default: true },
  // Legacy single-group props (still used for selected group via parent computed)
  headerMode: { type: String, required: true },
  headerText: { type: String, required: true },
  headerPrefix: { type: String, required: true },
  headerSuffix: { type: String, required: true },
  headerAlign: { type: String, required: true },
  headerFontSize: { type: Number, required: true },
  headerFontFamily: { type: String, required: true },
  headerMarginMm: { type: Number, required: true },
  headerOffsetXMm: { type: Number, required: true },
  headerColor: { type: String, required: true },
  headerPerFilePrefix: { type: String, default: '证据' },
  headerPerFileSeqType: { type: String, default: 'numeric' },
  headerPerFileSeqStart: { type: Number, default: 1 },
  headerPageStart: { type: Number, default: 1 },
  headerPageEnd: { type: Number, default: 0 },
  filePages: { type: Number, default: 0 },
  footerTextGroups: { type: Array, default: () => [] },
  selectedFooterTextGroupId: { type: String, default: '' },
  footerInsertEnabled: { type: Boolean, default: true },
  footerTextContent: { type: String, required: true },
  footerTextAlign: { type: String, required: true },
  footerTextFontSize: { type: Number, required: true },
  footerTextFontFamily: { type: String, required: true },
  footerTextMarginMm: { type: Number, required: true },
  footerTextOffsetXMm: { type: Number, required: true },
  footerTextColor: { type: String, required: true },
  footerTextPageStart: { type: Number, default: 1 },
  footerTextPageEnd: { type: Number, default: 0 },
  pageNumberGroups: { type: Array, default: () => [] },
  numberingDefaults: { type: Object, default: () => ({ evidenceStart: 1, pageStart: 1 }) },
  selectedPageNumberGroupId: { type: String, default: '' },
  pageNumberEnabled: { type: Boolean, required: true },
  pageNumberSequence: { type: String, required: true },
  pageNumberStyle: { type: String, required: true },
  pageNumberTemplate: { type: String, required: true },
  pageNumberRegion: { type: String, required: true },
  pageNumberAlign: { type: String, required: true },
  pageNumberFontSize: { type: Number, required: true },
  pageNumberFontFamily: { type: String, required: true },
  pageNumberMarginMm: { type: Number, required: true },
  pageNumberOffsetXMm: { type: Number, required: true },
  pageNumberColor: { type: String, required: true },
  pageNumberDuplexMode: { type: String, default: 'none' },
  pageNumberOverrideCount: { type: Number, default: 0 },
  pageNumberShowTotal: { type: Boolean, default: true },
  pageNumberSamplePage: { type: Number, default: 1 },
  pageNumberSampleTotal: { type: Number, default: 35 },
  pageHeightMm: { type: Number, default: 297 },
  offsetLimitMm: { type: Number, default: 120 },
})

const emit = defineEmits([
  'editPageNumberRules',
  'update:pageNumberShowTotal',
  'update:headerGroups',
  'update:selectedHeaderGroupId',
  'update:footerTextGroups',
  'update:selectedFooterTextGroupId',
  'update:pageNumberGroups',
  'update:numberingDefaults',
  'update:selectedPageNumberGroupId',
  ...[
    'headerMode',
    'headerInsertEnabled',
    'headerText',
    'headerPrefix',
    'headerSuffix',
    'headerAlign',
    'headerFontSize',
    'headerFontFamily',
    'headerMarginMm',
    'headerOffsetXMm',
    'headerColor',
    'headerPerFilePrefix',
    'headerPerFileSeqType',
    'headerPerFileSeqStart',
    'headerPageStart',
    'headerPageEnd',
    'footerInsertEnabled',
    'footerTextContent',
    'footerTextAlign',
    'footerTextFontSize',
    'footerTextFontFamily',
    'footerTextMarginMm',
    'footerTextOffsetXMm',
    'footerTextColor',
    'footerTextPageStart',
    'footerTextPageEnd',
    'pageNumberEnabled',
    'pageNumberSequence',
    'pageNumberStyle',
    'pageNumberTemplate',
    'pageNumberRegion',
    'pageNumberAlign',
    'pageNumberFontSize',
    'pageNumberFontFamily',
    'pageNumberMarginMm',
    'pageNumberOffsetXMm',
    'pageNumberColor',
    'pageNumberDuplexMode',
  ].map((key) => `update:${key}`),
])

// --- Undo/Redo history for each section ---
const headerHistory = useHistory({
  snapshot: () => ({
    align: props.headerAlign,
    fontSize: props.headerFontSize,
    fontFamily: props.headerFontFamily,
    marginMm: props.headerMarginMm,
    offsetXMm: props.headerOffsetXMm,
    color: props.headerColor,
  }),
  restore: (s) => {
    emit('update:headerAlign', s.align)
    emit('update:headerFontSize', s.fontSize)
    emit('update:headerFontFamily', s.fontFamily)
    emit('update:headerMarginMm', s.marginMm)
    emit('update:headerOffsetXMm', s.offsetXMm)
    emit('update:headerColor', s.color)
  },
  onChange: () => emit('change'),
})

const footerTextHistory = useHistory({
  snapshot: () => ({
    align: props.footerTextAlign,
    fontSize: props.footerTextFontSize,
    fontFamily: props.footerTextFontFamily,
    marginMm: props.footerTextMarginMm,
    offsetXMm: props.footerTextOffsetXMm,
    color: props.footerTextColor,
  }),
  restore: (s) => {
    emit('update:footerTextAlign', s.align)
    emit('update:footerTextFontSize', s.fontSize)
    emit('update:footerTextFontFamily', s.fontFamily)
    emit('update:footerTextMarginMm', s.marginMm)
    emit('update:footerTextOffsetXMm', s.offsetXMm)
    emit('update:footerTextColor', s.color)
  },
  onChange: () => emit('change'),
})

const pageNumberHistory = useHistory({
  snapshot: () => ({
    align: props.pageNumberAlign,
    fontSize: props.pageNumberFontSize,
    fontFamily: props.pageNumberFontFamily,
    marginMm: props.pageNumberMarginMm,
    offsetXMm: props.pageNumberOffsetXMm,
    color: props.pageNumberColor,
  }),
  restore: (s) => {
    emit('update:pageNumberAlign', s.align)
    emit('update:pageNumberFontSize', s.fontSize)
    emit('update:pageNumberFontFamily', s.fontFamily)
    emit('update:pageNumberMarginMm', s.marginMm)
    emit('update:pageNumberOffsetXMm', s.offsetXMm)
    emit('update:pageNumberColor', s.color)
  },
  onChange: () => emit('change'),
})

// Debounced watches: push history when placement params change
watch(
  () => [
    props.headerAlign,
    props.headerFontSize,
    props.headerFontFamily,
    props.headerMarginMm,
    props.headerOffsetXMm,
    props.headerColor,
  ],
  debounce(() => headerHistory.push(), 500),
  { deep: true },
)
watch(
  () => [
    props.footerTextAlign,
    props.footerTextFontSize,
    props.footerTextFontFamily,
    props.footerTextMarginMm,
    props.footerTextOffsetXMm,
    props.footerTextColor,
  ],
  debounce(() => footerTextHistory.push(), 500),
  { deep: true },
)
watch(
  () => [
    props.pageNumberAlign,
    props.pageNumberFontSize,
    props.pageNumberFontFamily,
    props.pageNumberMarginMm,
    props.pageNumberOffsetXMm,
    props.pageNumberColor,
  ],
  debounce(() => pageNumberHistory.push(), 500),
  { deep: true },
)

// Expose undo/redo for parent-level keyboard shortcut dispatch
function undo() {
  if (headerHistory.canUndo.value) {
    headerHistory.undo()
    return true
  }
  if (footerTextHistory.canUndo.value) {
    footerTextHistory.undo()
    return true
  }
  if (pageNumberHistory.canUndo.value) {
    pageNumberHistory.undo()
    return true
  }
  return false
}
function redo() {
  if (headerHistory.canRedo.value) {
    headerHistory.redo()
    return true
  }
  if (footerTextHistory.canRedo.value) {
    footerTextHistory.redo()
    return true
  }
  if (pageNumberHistory.canRedo.value) {
    pageNumberHistory.redo()
    return true
  }
  return false
}
defineExpose({ undo, redo })

function model(key) {
  return computed({ get: () => props[key], set: (value) => emit(`update:${key}`, value) })
}
const headerModeModel = model('headerMode'),
  headerInsertEnabledModel = model('headerInsertEnabled'),
  headerTextModel = model('headerText'),
  headerPrefixModel = model('headerPrefix'),
  headerSuffixModel = model('headerSuffix')
const headerAlignModel = model('headerAlign'),
  headerFontSizeModel = model('headerFontSize'),
  headerFontFamilyModel = model('headerFontFamily'),
  headerMarginMmModel = model('headerMarginMm'),
  headerOffsetXMmModel = model('headerOffsetXMm'),
  headerColorModel = model('headerColor')
const headerPerFilePrefixModel = model('headerPerFilePrefix')
const headerPerFileSeqTypeModel = model('headerPerFileSeqType')
const headerPageStartModel = model('headerPageStart')
const headerPageEndModel = model('headerPageEnd')
const headerAllPages = computed({
  get: () => props.headerPageEnd === 0,
  set: (v) => {
    if (v) {
      emit('update:headerPageEnd', 0)
    } else {
      emit('update:headerPageEnd', 1)
    }
  },
})
const footerInsertEnabledModel = model('footerInsertEnabled'),
  footerTextContentModel = model('footerTextContent'),
  footerTextAlignModel = model('footerTextAlign'),
  footerTextFontSizeModel = model('footerTextFontSize'),
  footerTextFontFamilyModel = model('footerTextFontFamily'),
  footerTextMarginMmModel = model('footerTextMarginMm'),
  footerTextOffsetXMmModel = model('footerTextOffsetXMm'),
  footerTextColorModel = model('footerTextColor')
const footerTextPageStartModel = model('footerTextPageStart')
const footerTextPageEndModel = model('footerTextPageEnd')
const footerTextAllPages = computed({
  get: () => props.footerTextPageEnd === 0,
  set: (v) => {
    if (v) {
      emit('update:footerTextPageEnd', 0)
    } else {
      emit('update:footerTextPageEnd', 1)
    }
  },
})
const pageNumberEnabledModel = model('pageNumberEnabled'),
  pageNumberStyleModel = model('pageNumberStyle'),
  pageNumberTemplateModel = model('pageNumberTemplate'),
  pageNumberRegionModel = model('pageNumberRegion'),
  pageNumberAlignModel = model('pageNumberAlign'),
  pageNumberFontSizeModel = model('pageNumberFontSize'),
  pageNumberFontFamilyModel = model('pageNumberFontFamily'),
  pageNumberMarginMmModel = model('pageNumberMarginMm'),
  pageNumberOffsetXMmModel = model('pageNumberOffsetXMm'),
  pageNumberColorModel = model('pageNumberColor'),
  pageNumberDuplexModeModel = model('pageNumberDuplexMode')
const pageNumberShowTotalModel = model('pageNumberShowTotal')
const selectedHeaderGroup = computed(
  () => props.headerGroups.find((group) => group.id === props.selectedHeaderGroupId) || props.headerGroups[0] || null,
)
const selectedPageNumberGroup = computed(
  () =>
    props.pageNumberGroups.find((group) => group.id === props.selectedPageNumberGroupId) ||
    props.pageNumberGroups[0] ||
    null,
)
function updateSelectedGroup(kind, patch) {
  const groups = kind === 'header' ? props.headerGroups : props.pageNumberGroups
  const selectedId = kind === 'header' ? props.selectedHeaderGroupId : props.selectedPageNumberGroupId
  const updated = groups.map((group) => (group.id === selectedId ? { ...group, ...patch } : group))
  emit(kind === 'header' ? 'update:headerGroups' : 'update:pageNumberGroups', updated)
}
function updateGroupNumbering(kind, patch) {
  const group = kind === 'header' ? selectedHeaderGroup.value : selectedPageNumberGroup.value
  if (!group) return
  updateSelectedGroup(kind, { numbering: { source: 'default', ...(group.numbering || {}), ...patch } })
}
const headerNumberingSourceModel = computed({
  get: () => selectedHeaderGroup.value?.numbering?.source || 'default',
  set: (value) => updateGroupNumbering('header', { source: value }),
})
const effectiveHeaderSeqStartModel = computed({
  get: () =>
    headerNumberingSourceModel.value === 'custom'
      ? Number(selectedHeaderGroup.value?.numbering?.evidenceStart ?? props.headerPerFileSeqStart ?? 1)
      : Number(props.numberingDefaults.evidenceStart ?? 1),
  set: (value) => updateGroupNumbering('header', { source: 'custom', evidenceStart: Number(value) }),
})
// 页码编号收归规则自含：读取时兼容旧数据（pageStart 为空则回落旧全局值），写入只改规则本身
const pageStartModel = computed({
  get: () => Number(selectedPageNumberGroup.value?.numbering?.pageStart ?? props.numberingDefaults.pageStart ?? 1),
  set: (value) => updateGroupNumbering('pageNumber', { pageStart: Math.max(1, Number(value) || 1) }),
})
const pageSequenceModel = computed({
  get: () => selectedPageNumberGroup.value?.numbering?.sequence || 'continuous',
  set: (value) => updateGroupNumbering('pageNumber', { sequence: value }),
})
const pageTotalModeModel = computed({
  get: () => selectedPageNumberGroup.value?.numbering?.totalMode || 'combined',
  set: (value) => updateGroupNumbering('pageNumber', { totalMode: value }),
})
const customFormatEditing = ref(false)
const filteredPageNumberPresets = computed(() => pageNumberPresets(props.pageNumberShowTotal))
const pageNumberPresetModel = computed({
  get: () =>
    customFormatEditing.value || !filteredPageNumberPresets.value.some((p) => p.value === props.pageNumberTemplate)
      ? 'custom'
      : props.pageNumberTemplate,
  set: (value) => {
    customFormatEditing.value = value === 'custom'
  },
})
function applyPresetTemplate(value) {
  if (value && value !== 'custom') pageNumberTemplateModel.value = value
}
watch(
  () => props.selectedPageNumberGroupId,
  () => {
    customFormatEditing.value = false
  },
)
let lastTemplateWithTotal = ''
const pageNumberPreviewText = computed(() =>
  renderPageNumberTemplate(
    templateForTotal(props.pageNumberTemplate || '{page}', props.pageNumberShowTotal, lastTemplateWithTotal),
    props.pageNumberSamplePage,
    props.pageNumberSampleTotal,
    props.pageNumberStyle,
  ),
)
watch(
  () => props.pageNumberShowTotal,
  (showTotal) => {
    const template = props.pageNumberTemplate || '{page}'
    if (!showTotal && usesPageTotal(template)) lastTemplateWithTotal = template
    pageNumberTemplateModel.value = templateForTotal(template, showTotal, lastTemplateWithTotal)
  },
)
watch(() => props.pageNumberTemplate, template => {
  if (customFormatEditing.value && usesPageTotal(template) !== props.pageNumberShowTotal) {
    pageNumberShowTotalModel.value = usesPageTotal(template)
  }
})

// Overlap detection
const overlapWarnings = computed(() => {
  const warnings = []
  const headerGroups = props.headerGroups || []
  const footerTextGroups = props.footerTextGroups || []
  const pageNumberGroups = props.pageNumberGroups || []
  const pageHeight = Number(props.pageHeightMm || 297)
  const enabledHeader = headerGroups.filter((g) => g.enabled)
  const enabledFooterText = footerTextGroups.filter((g) => g.enabled)
  const enabledPnFooter = pageNumberGroups.filter((g) => g.enabled && g.region !== 'header')
  const enabledPnHeader = pageNumberGroups.filter((g) => g.enabled && g.region === 'header')
  const label = (g, fallback) => g.label || fallback
  // Header-to-header overlap (±2mm tolerance)
  for (let i = 0; i < enabledHeader.length; i++) {
    for (let j = i + 1; j < enabledHeader.length; j++) {
      if (Math.abs(enabledHeader[i].marginMm - enabledHeader[j].marginMm) <= 2) {
        warnings.push(
          `"${label(enabledHeader[i], '页眉 ' + (i + 1))}" 与 "${label(enabledHeader[j], '页眉 ' + (j + 1))}" 距顶距离接近，可能重叠`,
        )
      }
    }
  }
  // Header bottom vs page number area (footer-region page numbers) and header-region page numbers
  for (const h of enabledHeader) {
    const headerBottom = (h.marginMm || 10) + (h.fontSize || 10) * 0.4
    for (const pn of enabledPnFooter) {
      const pnTop = pageHeight - (pn.marginMm || 10) - (pn.fontSize || 9) * 0.4
      if (headerBottom > pnTop - 5) {
        warnings.push(`"${label(h, '页眉')}" 底部与页码区域可能碰撞`)
        break
      }
    }
    for (const pn of enabledPnHeader) {
      const pnBottom = (pn.marginMm || 10) + (pn.fontSize || 9) * 0.4
      if (Math.abs(headerBottom - pnBottom) <= 5) {
        warnings.push(`"${label(h, '页眉')}" 与页眉区页码"${label(pn, '页码')}"可能重叠`)
      }
    }
  }
  // Footer text vs page number (±3mm), all group combinations
  for (const ft of enabledFooterText) {
    for (const pn of enabledPnFooter) {
      if (Math.abs((ft.marginMm || 10) - (pn.marginMm || 10)) <= 3) {
        warnings.push(`"${label(ft, '页脚文字')}" 与 "${label(pn, '页码')}" 距底距离接近，可能重叠`)
      }
    }
  }
  return warnings
})

function groupModeLabel(group) {
  return MODE_LABELS[group.mode] || group.mode
}

function addGroup() {
  const newGroup = {
    id: `h${Date.now()}`,
    label: `页眉 ${props.headerGroups.length + 1}`,
    enabled: true,
    mode: 'filename',
    text: '',
    prefix: '',
    suffix: '',
    align: 'right',
    fontSize: 10,
    fontFamily: 'auto',
    marginMm: 10,
    offsetXMm: 0,
    color: '#000000',
    pageStart: 1,
    pageEnd: 0,
    fileIds: [],
    perFilePrefix: '证据',
    perFileSeqType: 'numeric',
    perFileSeqStart: 1,
    numbering: { source: 'default', evidenceStart: null },
  }
  const updated = [...props.headerGroups, newGroup]
  emit('update:headerGroups', updated)
  emit('update:selectedHeaderGroupId', newGroup.id)
}

function removeGroup(id) {
  if (props.headerGroups.length <= 1) return
  const updated = props.headerGroups.filter((g) => g.id !== id)
  emit('update:headerGroups', updated)
  if (props.selectedHeaderGroupId === id) {
    emit('update:selectedHeaderGroupId', updated[0].id)
  }
}

function addFooterTextGroup() {
  const newGroup = {
    id: `ft${Date.now()}`,
    label: `页脚文字 ${props.footerTextGroups.length + 1}`,
    enabled: true,
    text: '',
    align: 'left',
    fontSize: 9,
    fontFamily: 'auto',
    marginMm: 10,
    offsetXMm: 0,
    color: '#000000',
    pageStart: 1,
    pageEnd: 0,
    fileIds: [],
  }
  const updated = [...props.footerTextGroups, newGroup]
  emit('update:footerTextGroups', updated)
  emit('update:selectedFooterTextGroupId', newGroup.id)
}

function removeFooterTextGroup(id) {
  if (props.footerTextGroups.length <= 1) return
  const updated = props.footerTextGroups.filter((g) => g.id !== id)
  emit('update:footerTextGroups', updated)
  if (props.selectedFooterTextGroupId === id) {
    emit('update:selectedFooterTextGroupId', updated[0].id)
  }
}

function addPageNumberGroup() {
  const newGroup = {
    id: `pn${Date.now()}`,
    label: `页码 ${props.pageNumberGroups.length + 1}`,
    enabled: true,
    sequence: 'continuous',
    style: 'arabic',
    template: '{page}/{total}',
    region: 'footer',
    align: 'center',
    fontSize: 9,
    fontFamily: 'auto',
    marginMm: 10,
    offsetXMm: 0,
    color: '#000000',
    fileIds: [],
    numbering: { source: 'default', pageStart: null, sequence: null, totalMode: null },
    exceptions: [],
  }
  const updated = [...props.pageNumberGroups, newGroup]
  emit('update:pageNumberGroups', updated)
  emit('update:selectedPageNumberGroupId', newGroup.id)
}

function removePageNumberGroup(id) {
  if (props.pageNumberGroups.length <= 1) return
  const updated = props.pageNumberGroups.filter((g) => g.id !== id)
  emit('update:pageNumberGroups', updated)
  if (props.selectedPageNumberGroupId === id) {
    emit('update:selectedPageNumberGroupId', updated[0].id)
  }
}
</script>

<style scoped>
.section-title {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.settings-group-label {
  margin-top: 5px;
  color: var(--docsy-text-muted);
  font-size: 11px;
  font-weight: 600;
}
.custom-format-field {
  grid-column: 1 / -1;
}
.page-number-sample {
  display: grid;
  gap: 4px;
  padding: 8px 12px;
  margin-top: 6px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-muted);
}
.sample-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--docsy-text-muted);
  font-size: 11px;
}
.sample-caption {
  color: var(--docsy-text-muted);
  font-size: 11px;
  text-align: center;
}
.page-number-sample .page-number-preview {
  display: block;
  padding: 6px;
  background: transparent;
  font-size: 20px;
  text-align: center;
}
/* 根容器：纵向排列三个区块，区块间留固定间距 */
.hf-rule-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
/* 单个区块：标题/整宽元素/网格纵向排列 */
.hf-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: var(--docsy-surface-elevated);
}
/* 区块内部网格：v-if 行增删只重排本区块，不影响其它区块 */
.hf-section-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 220px), 1fr));
  gap: 6px 12px;
  align-items: start;
}
/* 网格内每个选项项：统一结构和对齐 */
.hf-section-grid :deep(.rule-item) {
  display: grid;
  grid-template-columns: 94px minmax(0, 1fr);
  align-items: center;
  gap: 4px 8px;
  min-height: 28px;
}
.hf-section-grid :deep(.rule-item label) {
  font-size: 12px;
  color: var(--docsy-text);
  line-height: 1.3;
}
.hf-section-grid :deep(.field-hint) {
  font-size: 11px;
  line-height: 1.4;
  color: var(--docsy-text-muted);
}
.section-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 4px;
  padding-top: 8px;
  border-top: 0;
}
.section-label strong {
  font-size: 12px;
  font-weight: 600;
  color: var(--docsy-text-strong);
}
.section-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.overlap-warnings {
  margin-bottom: 4px;
}
.header-group-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: 4px;
  padding: 4px 0;
  border-bottom: 1px solid var(--docsy-border-subtle);
}
.header-group-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-radius: var(--docsy-radius);
  cursor: pointer;
  font-size: 13px;
  color: var(--docsy-text-muted);
  transition: background 0.15s;
}
.header-group-item:hover {
  background: var(--docsy-surface-hover);
}
.header-group-item.active {
  background: var(--docsy-surface-active);
  color: var(--docsy-text-strong);
  font-weight: 500;
}
.group-label {
  display: flex;
  align-items: center;
  gap: 4px;
}
.page-number-preview {
  display: inline-block;
  padding: 2px 8px;
  font-size: 13px;
  color: var(--docsy-text-strong);
  background: var(--docsy-surface-muted);
  border-radius: var(--docsy-radius);
}
.template-presets {
  margin-bottom: 4px;
}
.template-custom-input {
  margin-top: 4px;
}
.page-range-row {
  grid-column: 1 / -1;
}
.page-range-inputs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}
.page-range-inputs .el-input-number {
  width: 100px;
}
.range-sep {
  color: var(--docsy-text-muted);
  font-size: 13px;
}
.text-input-with-info {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: nowrap;
}
.text-input-with-info .el-input {
  flex: 1;
}
.text-input-with-info .info-icon {
  flex-shrink: 0;
  font-size: 16px;
  color: var(--docsy-text-muted);
  cursor: pointer;
  transition: color 0.15s;
}
.text-input-with-info .info-icon:hover {
  color: var(--docsy-text-strong);
}
.template-help {
  max-width: 280px;
  font-size: 12px;
  line-height: 1.6;
}
.template-help p {
  margin: 2px 0;
}
.template-help code {
  background: rgba(255, 255, 255, 0.1);
  padding: 0 3px;
  border-radius: var(--docsy-radius);
  font-size: 11px;
}
.duplex-page-hint {
  grid-column: 1 / -1;
  font-size: 11.5px;
  color: var(--docsy-primary);
  background: color-mix(in srgb, var(--docsy-primary) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--docsy-primary) 18%, transparent);
  border-radius: var(--docsy-radius);
  padding: 4px 8px;
  line-height: 1.4;
  margin-top: -2px;
}
</style>

<style scoped>
.hf-section-grid :deep(.rule-item > :is(.el-input, .el-select, .el-input-number)) {
  width: 100%;
  min-width: 0;
}
.hf-section-grid :deep(.rule-item > label) {
  display: inline-flex;
  align-items: center;
}
.section-label {
  min-height: 28px;
  margin-top: 0;
  padding-top: 6px;
}
.hf-section {
  gap: 6px;
}
</style>

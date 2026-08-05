<template>
  <section class="workspace">
    <div class="panel">
      <div class="panel-header">
        <div>
          <h3>导入标黄 Word</h3>
          <p>在 Word 里把可替换文字或勾选符号标黄，再导入确认字段。</p>
        </div>
        <el-button type="primary" :loading="scanning" @click="$emit('select-source-docx')">选择 Word</el-button>
      </div>

      <el-descriptions v-if="sourceDocx" :column="1" size="small" border>
        <el-descriptions-item label="文件">{{ sourceDocx }}</el-descriptions-item>
        <el-descriptions-item label="识别">
          {{ marks.length }} 个标黄片段，{{ checkboxLikeCount }} 个疑似勾选符号
        </el-descriptions-item>
      </el-descriptions>
    </div>

    <div v-if="fieldRows.length" class="panel field-panel">
      <div class="panel-header compact">
        <div>
          <h3>确认字段</h3>
          <p>同名字段会共用一个值；空值处理是字段属性，可以设为全部同名共用或仅当前位置生效。</p>
        </div>
        <div class="panel-actions">
          <el-popover placement="bottom-end" trigger="hover" width="420" popper-class="field-rules-popper">
            <template #reference>
              <el-button size="small" text class="help-button">
                <el-icon><QuestionFilled /></el-icon>
                字段规则
              </el-button>
            </template>
            <div class="field-rules">
              <section>
                <h4>类型怎么选</h4>
                <div v-for="item in typeHelpItems" :key="item.value" class="type-help-item">
                  <strong>{{ item.label }}</strong>
                  <span>{{ item.description }}</span>
                </div>
              </section>
              <section>
                <h4>前缀 / 后缀</h4>
                <p>
                  前缀和后缀不是填写字段，只在归属字段为空时随字段一起消失。前缀可以包含语法连接符，例如"，第三人"。
                </p>
                <p>例：标黄"（案号："设为前缀，案号设为文本，标黄"）"设为后缀，三行字段名都填"案号"。</p>
              </section>
              <section>
                <h4>列表与勾选</h4>
                <p>当事人列表会按填写顺序用顿号连接；勾选组只改方框符号，选项文字保留 Word 原文。</p>
              </section>
            </div>
          </el-popover>
          <el-input
            :model-value="templateName"
            class="template-name"
            placeholder="模板名称"
            @update:model-value="$emit('update:templateName', $event)"
          />
        </div>
      </div>

      <div v-if="selectedRows.length" class="selection-tools">
        <span class="selection-count">已选 {{ selectedRows.length }} 个片段</span>
        <el-input
          :model-value="groupName"
          class="group-input"
          size="small"
          placeholder="字段名"
          @update:model-value="$emit('update:groupName', $event)"
        />
        <el-input
          :model-value="groupLabel"
          class="group-input"
          size="small"
          placeholder="显示名"
          @update:model-value="$emit('update:groupLabel', $event)"
        />
        <el-select
          :model-value="groupType"
          class="type-input"
          size="small"
          @update:model-value="$emit('update:groupType', $event)"
        >
          <el-option label="文本" value="text" />
          <el-option label="日期" value="date" />
          <el-option label="下拉选择" value="select" />
          <el-option label="列表" value="party_list" />
          <el-option label="引用" value="reference" />
          <el-option label="互斥勾选组" value="radio_group" />
          <el-option label="多选勾选组" value="checkbox_group" />
        </el-select>
        <el-button size="small" type="primary" @click="$emit('group-selected-rows')">合并为字段</el-button>
        <el-button size="small" @click="$emit('set-selected-rows-usage', 'prefix')">设为前缀</el-button>
        <el-button size="small" @click="$emit('set-selected-rows-usage', 'suffix')">设为后缀</el-button>
        <el-button size="small" text @click="$emit('clear-selected-rows')">取消选择</el-button>
      </div>

      <el-table
        ref="fieldTableRef"
        :data="fieldTableRows"
        size="small"
        border
        row-key="rowId"
        :row-class-name="buildFieldRowClassName"
        @selection-change="$emit('selection-change', $event.filter((row) => !row.displayOnly))"
        @wheel="$emit('field-table-wheel', $event)"
      >
        <el-table-column type="selection" width="38" :selectable="(row) => !row.displayOnly" />
        <el-table-column label="拆分" width="58" align="center">
          <template #default="{ row }">
            <el-button
              link
              type="primary"
              :disabled="
                row.displayOnly ||
                rowUsage(row) !== 'field' ||
                isMarkerType(row.type) ||
                row.type === 'party_list'
              "
              @click="$emit('open-split-dialog', row)"
            >
              拆分
            </el-button>
          </template>
        </el-table-column>
        <el-table-column label="标黄文字" min-width="220" show-overflow-tooltip>
          <template #default="{ row }">
            <div class="mark-cell" :class="{ 'structure-mark': rowUsage(row) !== 'field' }">
              <span v-if="row.displayOnly && !row.virtualPartyGroup" class="party-item-arrow">→</span>
              <span v-else-if="row.partyGroupChild" class="party-item-arrow">→</span>
              <span v-else-if="rowUsage(row) === 'prefix'" class="relation-arrow">↳</span>
              <span v-else-if="rowUsage(row) === 'suffix'" class="relation-arrow">↰</span>
              <span class="mark-text">{{ displayMarkText(row) }}</span>
              <span v-if="row.optionLabel && isMarkerType(row.type)" class="option-preview">
                {{ row.optionLabel }}
              </span>
              <span v-if="row.type === 'party_list' && row.partyItems?.length > 1" class="option-preview">
                已识别 {{ row.partyItems.length }} 项
              </span>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="类型" width="132">
          <template #default="{ row }">
            <span v-if="row.virtualPartyGroup" class="muted">当事人列表</span>
            <span v-else-if="row.displayOnly" class="muted">当事人项</span>
            <span v-else-if="isConnectorRow(row)" class="muted">连接符</span>
            <el-select v-else v-model="row.type" size="small" @change="$emit('row-type-change', row)">
              <el-option
                v-for="item in typeHelpItems"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="字段" min-width="210">
          <template #default="{ row }">
            <div v-if="row.virtualPartyGroup" class="field-cell party-child-field">
              <span>{{ row.name }}</span>
              <span class="field-label">共 {{ row.partyItemCount }} 项</span>
            </div>
            <div v-else-if="row.displayOnly" class="field-cell party-child-field">
              <span>{{ row.name }}</span>
              <span class="field-label">第 {{ row.partyItemIndex + 1 }} 项，共 {{ row.partyItemCount }} 项</span>
            </div>
            <div v-else-if="rowUsage(row) === 'ignore'" class="muted">保留原文，不生成字段</div>
            <div v-else-if="rowUsage(row) === 'delete_text'" class="muted">删除此段文字</div>
            <div v-else-if="isConnectorRow(row)" class="field-cell">
              <span>连接符</span>
              <span class="field-label">跟随右侧字段：{{ structureTargetDisplayName(row) }}</span>
            </div>
            <div v-else-if="row.type === 'reference'" class="field-cell">
              <el-input
                v-model="row.name"
                size="small"
                placeholder="引用字段名"
                @input="$emit('field-name-input', row)"
              />
              <span class="field-label">引用：{{ referenceSourceLabel(row) || '未指定来源' }}</span>
            </div>
            <div v-else class="field-cell">
              <el-input
                v-model="row.name"
                size="small"
                :placeholder="rowUsage(row) === 'field' ? '法院' : '归属字段名'"
                @input="$emit('field-name-input', row)"
              />
              <span class="field-label">{{ fieldCellSecondaryLabel(row) }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="必填" width="58" align="center">
          <template #default="{ row }">
            <el-checkbox
              v-if="!row.displayOnly && rowUsage(row) === 'field' && !isMarkerType(row.type)"
              v-model="row.required"
            />
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column label="关系" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">
            <el-tag v-if="row.virtualPartyGroup" size="small" type="success">当事人列表</el-tag>
            <el-tag v-else-if="row.displayOnly" size="small" type="success">列表项</el-tag>
            <el-tag v-else-if="rowUsage(row) === 'prefix'" size="small" type="info">{{
              relationSummary(row)
            }}</el-tag>
            <el-tag v-else-if="rowUsage(row) === 'suffix'" size="small" type="info">{{
              relationSummary(row)
            }}</el-tag>
            <el-tag v-else-if="rowUsage(row) === 'delete_text'" size="small" type="danger">删除文本</el-tag>
            <el-tag v-else-if="row.type === 'reference'" size="small" type="warning">
              引用 {{ referenceSourceLabel(row) || '未指定' }}
            </el-tag>
            <el-tag v-else-if="isGroupedField(row)" size="small" type="success">{{
              groupedFieldSummary(row)
            }}</el-tag>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
        <el-table-column label="设置" width="64" align="center">
          <template #default="{ row }">
            <span v-if="row.displayOnly" class="muted">-</span>
            <template v-else>
              <el-popover placement="left-start" trigger="click" width="420">
                <template #reference>
                  <el-badge
                    v-if="hasUnseenReferenceSuggestion(row)"
                    value="!"
                    type="danger"
                    class="settings-badge"
                  >
                    <el-button size="small" circle @click="row.referenceHintSeen = true">⋯</el-button>
                  </el-badge>
                  <el-button v-else size="small" circle @click="row.referenceHintSeen = true">⋯</el-button>
                </template>
                <div class="row-settings">
                  <h4>{{ row.text || row.name }}</h4>
                  <p class="setting-caption">{{ relationSummary(row) }}</p>

                  <div v-if="referenceSuggestion(row)" class="reference-suggestion">
                    <p>
                      字段内容与前面的"{{
                        referenceSuggestion(row).targetLabel
                      }}"相同，建议改成引用，共用同一个填写值。
                      <span v-if="referenceSuggestion(row).targetKind === 'party_item'">
                        这是当事人列表中的单个成员。
                      </span>
                    </p>
                    <el-checkbox v-model="row.referenceIncludePrefix">
                      包含当前这处的前缀
                      <span class="setting-caption">
                        {{ referenceSuggestion(row).prefixRows.length ? '会一并归属到引用字段' : '当前未检测到' }}
                      </span>
                    </el-checkbox>
                    <el-checkbox v-model="row.referenceIncludeSuffix">
                      包含当前这处的后缀
                      <span class="setting-caption">
                        {{ referenceSuggestion(row).suffixRows.length ? '会一并归属到引用字段' : '当前未检测到' }}
                      </span>
                    </el-checkbox>
                    <div class="reference-actions">
                      <el-button size="small" type="primary" @click="$emit('apply-reference-suggestion', row)">
                        改成引用
                      </el-button>
                      <el-button
                        size="small"
                        :disabled="!allReferenceSuggestions().length"
                        @click="$emit('apply-all-reference-suggestions')"
                      >
                        全部应用
                      </el-button>
                    </div>
                  </div>

                  <el-form label-width="84px" size="small">
                    <el-form-item label="显示名" v-if="rowUsage(row) === 'field'">
                      <el-input v-model="row.label" />
                    </el-form-item>
                    <el-form-item label="通用字段名" v-if="rowUsage(row) === 'field'">
                      <el-input
                        v-model="row.semanticKey"
                        :placeholder="`默认跟随字段名：${row.name || '未命名'}`"
                      />
                    </el-form-item>

                    <template v-if="rowUsage(row) === 'field' && row.type === 'reference'">
                      <el-form-item label="引用来源">
                        <el-select
                          v-model="row.referenceSourceKey"
                          filterable
                          @change="$emit('sync-reference-source-from-key', row)"
                        >
                          <el-option
                            v-for="item in referenceSourceOptions(row)"
                            :key="item.key"
                            :label="item.label"
                            :value="item.key"
                          />
                        </el-select>
                      </el-form-item>
                      <el-alert
                        title="留空时会在填写页选择来源；选择通用字段名时会按通用字段名取值。"
                        type="info"
                        show-icon
                        :closable="false"
                      />
                    </template>

                    <template
                      v-if="rowUsage(row) === 'field' && !isMarkerType(row.type) && row.type !== 'reference'"
                    >
                      <el-form-item
                        v-if="row.type === 'party_list' && row.partyItems?.length > 1"
                        label="列表成员"
                      >
                        <div class="party-detected-items">
                          <el-tag v-for="item in row.partyItems" :key="item" size="small">{{ item }}</el-tag>
                        </div>
                      </el-form-item>
                      <el-form-item label="空值规则">
                        <el-checkbox v-model="row.optionalWhenEmpty">字段为空时处理周围文字</el-checkbox>
                      </el-form-item>
                      <el-form-item v-if="row.optionalWhenEmpty" label="范围">
                        <el-select v-model="row.optionalScope">
                          <el-option label="仅此位置" value="position" />
                          <el-option label="全部同名" value="field" />
                        </el-select>
                      </el-form-item>
                      <el-form-item v-if="row.optionalWhenEmpty" label="空值前缀">
                        <el-input v-model="row.optionalPrefix" placeholder="如 原告、（案号：" />
                      </el-form-item>
                      <el-form-item v-if="row.optionalWhenEmpty" label="空值后缀">
                        <el-input v-model="row.optionalSuffix" placeholder="如 律师、）" />
                      </el-form-item>
                    </template>

                    <template v-if="rowUsage(row) === 'field' && isMarkerType(row.type)">
                      <el-form-item label="选项ID">
                        <el-input v-model="row.optionId" />
                      </el-form-item>
                      <el-form-item label="选项名">
                        <el-input v-model="row.optionLabel" />
                      </el-form-item>
                      <el-form-item label="选中符号">
                        <el-select v-model="row.checkedText" allow-create filterable>
                          <el-option
                            v-for="item in checkedSymbolOptions"
                            :key="item"
                            :label="item"
                            :value="item"
                          />
                        </el-select>
                      </el-form-item>
                      <el-form-item label="未选符号">
                        <el-select v-model="row.uncheckedText" allow-create filterable>
                          <el-option
                            v-for="item in uncheckedSymbolOptions"
                            :key="item"
                            :label="item"
                            :value="item"
                          />
                        </el-select>
                      </el-form-item>
                      <el-form-item label="批量符号">
                        <el-button size="small" @click="$emit('sync-marker-symbols', row)">同步到同组</el-button>
                      </el-form-item>
                      <el-form-item label="组成员">
                        <el-select
                          :model-value="markerGroupMembers(row)"
                          multiple
                          filterable
                          collapse-tags
                          collapse-tags-tooltip
                          @change="(members) => $emit('apply-marker-group-members', row, members)"
                        >
                          <el-option
                            v-for="item in markerRowOptions"
                            :key="item.rowId"
                            :label="item.label"
                            :value="item.rowId"
                          />
                        </el-select>
                      </el-form-item>
                    </template>

                    <template v-if="rowUsage(row) === 'field' && row.type === 'select'">
                      <el-form-item label="下拉选项">
                        <div class="select-options-editor">
                          <div
                            v-for="(opt, optIdx) in row.selectOptions || []"
                            :key="optIdx"
                            class="select-option-row"
                          >
                            <el-input
                              v-model="opt.label"
                              size="small"
                              placeholder="选项文本"
                              class="select-option-input"
                            />
                            <el-input
                              v-model="opt.checkedText"
                              size="small"
                              placeholder="输出值（留空同文本）"
                              class="select-option-input"
                            />
                            <el-button
                              size="small"
                              text
                              type="danger"
                              @click="(row.selectOptions || []).splice(optIdx, 1)"
                            >
                              删除
                            </el-button>
                          </div>
                          <el-button size="small" @click="$emit('add-select-option', row)"> 添加选项 </el-button>
                        </div>
                      </el-form-item>
                    </template>

                    <template v-if="rowUsage(row) === 'prefix' || rowUsage(row) === 'suffix'">
                      <el-form-item label="归属字段">
                        <el-select
                          v-model="row.name"
                          filterable
                          allow-create
                          @change="$emit('structure-target-name-change', row)"
                        >
                          <el-option v-for="name in fieldNameOptions" :key="name" :label="name" :value="name" />
                        </el-select>
                      </el-form-item>
                      <el-alert
                        :title="
                          rowUsage(row) === 'prefix'
                            ? '字段为空时，此前缀随字段删除。'
                            : '字段为空时，此后缀随字段删除。'
                        "
                        type="info"
                        show-icon
                        :closable="false"
                      />
                    </template>
                  </el-form>
                </div>
              </el-popover>
            </template>
          </template>
        </el-table-column>
      </el-table>

      <el-collapse v-if="optionalRuleSummaries.length" class="rule-summary-collapse">
        <el-collapse-item :title="`空值规则概览（${optionalRuleSummaries.length} 条）`" name="optional-rules">
          <div class="rule-summary-list">
            <div v-for="item in optionalRuleSummaries" :key="item.key" class="rule-summary-item">
              <strong>{{ item.target }}</strong>
              <span>{{ item.description }}</span>
            </div>
          </div>
        </el-collapse-item>
      </el-collapse>

      <div class="template-build-actions">
        <el-button :disabled="!documentText" @click="$emit('update:showDocumentText', !showDocumentText)">
          {{ showDocumentText ? '收起全文' : '查看模板全文' }}
        </el-button>
        <el-button :disabled="!documentText" @click="$emit('update:showTemplatePreview', !showTemplatePreview)">
          {{ showTemplatePreview ? '收起预览' : '预览模板' }}
        </el-button>
        <el-button :disabled="!undoStack.length" @click="$emit('undo-last-action')">撤销</el-button>
        <el-button
          :disabled="!fieldRows.length"
          type="success"
          :loading="saving"
          @click="$emit('save-template')"
        >
          保存模板
        </el-button>
      </div>

      <div v-if="showDocumentText" class="preview-panel">
        <div class="preview-panel-header">
          <h3>模板全文</h3>
        </div>
        <pre
          ref="documentPreviewRef"
          class="document-preview"
          @mouseup="$emit('remember-source-preview-selection')"
          @keyup="$emit('remember-source-preview-selection')"
          >{{ documentText }}</pre>
      </div>

      <div v-if="showTemplatePreview" class="preview-panel">
        <div class="preview-panel-header">
          <h3>模板预览</h3>
          <p>文本级预览用于检查字段、前缀和后缀；Word 版式以最终生成文件为准。</p>
        </div>
        <div class="template-preview-grid">
          <section>
            <h4>原文标记</h4>
            <div
              ref="sourcePreviewRef"
              class="template-preview-text source-preview-text"
              @mouseup="$emit('remember-source-preview-selection')"
              @keyup="$emit('remember-source-preview-selection')"
            >
              <template v-for="segment in templatePreview.original" :key="segment.id">
                <span
                  v-if="segment.row"
                  class="preview-token"
                  :class="[previewTokenClass(segment.row), previewFormatClass(segment)]"
                  :data-run-id="segment.runId"
                  :data-start="segment.start"
                  :data-end="segment.end"
                  @click="$emit('focus-preview-row', segment.row)"
                >
                  {{ segment.text }}
                </span>
                <span
                  v-else
                  class="source-run"
                  :class="previewFormatClass(segment)"
                  :data-run-id="segment.runId"
                  :data-start="segment.start"
                  :data-end="segment.end"
                  >{{ segment.text }}</span
                >
              </template>
            </div>
          </section>
          <section>
            <h4>渲染示意</h4>
            <div v-if="previewSampleFields.length" class="preview-sample-form">
              <el-input
                v-for="field in previewSampleFields"
                :key="field.name"
                :model-value="previewSampleValues[field.name] || ''"
                size="small"
                :placeholder="fillFieldLabel(field)"
                @input="(value) => $emit('set-preview-sample-value', field.name, value)"
                @clear="$emit('set-preview-sample-value', field.name, '')"
              >
                <template #prepend>{{ fillFieldLabel(field) }}</template>
              </el-input>
            </div>
            <div class="template-preview-text">
              <template v-for="segment in templatePreview.rendered" :key="segment.id">
                <button
                  v-if="segment.row"
                  class="preview-token"
                  :class="[previewTokenClass(segment.row, segment), previewFormatClass(segment)]"
                  type="button"
                  @click="$emit('focus-preview-row', segment.row)"
                >
                  {{ segment.text }}
                </button>
                <span v-else :class="previewFormatClass(segment)">{{ segment.text }}</span>
              </template>
            </div>
          </section>
        </div>
        <div class="preview-legend">
          <span v-if="sourcePreviewSelectionPayload?.text" class="preview-selection-status">
            已选中：{{ sourcePreviewSelectionPayload.text }}
          </span>
          <button
            v-for="item in previewLegendItems"
            :key="item.className"
            class="legend-token"
            :class="item.className"
            type="button"
            @pointerdown.capture="$emit('remember-source-preview-selection')"
            @click.prevent.stop="$emit('trigger-preview-selection-add', item.type)"
          >
            {{ item.label }}
          </button>
          <button
            class="legend-token preview-delete-text"
            type="button"
            @pointerdown.capture="$emit('remember-source-preview-selection')"
            @click.prevent.stop="$emit('trigger-preview-selection-add', 'delete_text')"
          >
            删除文本
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup>
import { computed, ref } from 'vue'
import { QuestionFilled } from '@element-plus/icons-vue'
import {
  rowUsage,
  isMarkerType,
  isConnectorRow,
  displayMarkText,
  isGroupedField,
  groupedFieldSummary,
  referenceSourceLabel,
  hasUnseenReferenceSuggestion as hasUnseenReferenceSuggestionFn,
  previewTokenClass,
  previewFormatClass,
  fillFieldLabel,
  typeHelpItems,
  previewLegendItems,
  checkedSymbolOptions,
  uncheckedSymbolOptions,
  buildFieldTableRows,
  buildOptionalRuleSummaries,
  buildPreviewSampleFields,
  fieldNameOptions as fieldNameOptionsFn,
  markerRowOptions as markerRowOptionsFn,
  markerGroupMembers as markerGroupMembersFn,
  structureTargetDisplayName as structureTargetDisplayNameFn,
  fieldCellSecondaryLabel as fieldCellSecondaryLabelFn,
  fieldRowClassName as fieldRowClassNameFn,
  relationSummary as relationSummaryFn,
  referenceSuggestion as referenceSuggestionFn,
  referenceSourceOptions as referenceSourceOptionsFn,
  allReferenceSuggestions as allReferenceSuggestionsFn,
} from '../composables/fieldRowUtils.js'

const props = defineProps({
  // Source document state
  sourceDocx: { type: String, default: '' },
  scanning: { type: Boolean, default: false },
  marks: { type: Array, default: () => [] },
  documentText: { type: String, default: '' },
  // Template state
  templateName: { type: String, default: '' },
  fieldRows: { type: Array, default: () => [] },
  selectedRows: { type: Array, default: () => [] },
  saving: { type: Boolean, default: false },
  // UI state
  showDocumentText: { type: Boolean, default: false },
  showTemplatePreview: { type: Boolean, default: false },
  groupName: { type: String, default: '' },
  groupLabel: { type: String, default: '' },
  groupType: { type: String, default: 'text' },
  undoStack: { type: Array, default: () => [] },
  previewSampleValues: { type: Object, default: () => ({}) },
  sourcePreviewSelectionPayload: { type: Object, default: null },
  previewFocusedRowId: { type: String, default: '' },
  // Computed from parent
  templatePreview: { type: Object, default: () => ({ original: [], rendered: [] }) },
})

defineEmits([
  'select-source-docx',
  'update:templateName',
  'update:groupName',
  'update:groupLabel',
  'update:groupType',
  'update:showDocumentText',
  'update:showTemplatePreview',
  'group-selected-rows',
  'set-selected-rows-usage',
  'clear-selected-rows',
  'selection-change',
  'field-table-wheel',
  'open-split-dialog',
  'row-type-change',
  'field-name-input',
  'structure-target-name-change',
  'apply-reference-suggestion',
  'apply-all-reference-suggestions',
  'sync-reference-source-from-key',
  'sync-marker-symbols',
  'apply-marker-group-members',
  'add-select-option',
  'undo-last-action',
  'save-template',
  'remember-source-preview-selection',
  'focus-preview-row',
  'trigger-preview-selection-add',
  'set-preview-sample-value',
])

// Computed properties that depend on fieldRows
const checkboxLikeCount = computed(() => props.marks.filter((mark) => mark.checkboxLike).length)
const fieldTableRows = computed(() => buildFieldTableRows(props.fieldRows))
const optionalRuleSummaries = computed(() => buildOptionalRuleSummaries(props.fieldRows))
const previewSampleFields = computed(() => buildPreviewSampleFields(props.fieldRows))
const fieldNameOptions = computed(() => fieldNameOptionsFn(props.fieldRows))
const markerRowOptions = computed(() => markerRowOptionsFn(props.fieldRows))

// The three DOM refs are owned here; the parent reaches them through the
// component instance ref (defineExpose below).
const fieldTableRef = ref(null)
const sourcePreviewRef = ref(null)
const documentPreviewRef = ref(null)

// Functions that need fieldRows
function relationSummary(row) {
  return relationSummaryFn(row, props.fieldRows)
}

function structureTargetDisplayName(row) {
  return structureTargetDisplayNameFn(row, props.fieldRows)
}

function fieldCellSecondaryLabel(row) {
  return fieldCellSecondaryLabelFn(row, props.fieldRows)
}

function buildFieldRowClassName({ row }) {
  return fieldRowClassNameFn({ row }, props.fieldRows)
}

function referenceSuggestion(row) {
  return referenceSuggestionFn(row, props.fieldRows)
}

function referenceSourceOptions(row) {
  return referenceSourceOptionsFn(row, props.fieldRows)
}

function allReferenceSuggestions() {
  return allReferenceSuggestionsFn(props.fieldRows)
}

function markerGroupMembers(row) {
  return markerGroupMembersFn(row, props.fieldRows)
}

function hasUnseenReferenceSuggestion(row) {
  return hasUnseenReferenceSuggestionFn(row, props.fieldRows)
}

defineExpose({ fieldTableRef, sourcePreviewRef, documentPreviewRef })
</script>

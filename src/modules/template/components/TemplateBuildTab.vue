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
          v-if="editingLibraryTemplatePath"
          :disabled="!fieldRows.length"
          type="warning"
          :loading="saving"
          @click="$emit('save-template', true)"
        >
          覆盖保存
        </el-button>
        <el-button
          :disabled="!fieldRows.length"
          :type="editingLibraryTemplatePath ? 'primary' : 'success'"
          :loading="saving"
          @click="$emit('save-template', false)"
        >
          {{ editingLibraryTemplatePath ? '另存为' : '保存模板' }}
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
  isGroupedField as isGroupedFieldFn,
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
  // Editing state
  editingLibraryTemplatePath: { type: String, default: '' },
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

function isGroupedField(row) {
  return isGroupedFieldFn(row, props.fieldRows)
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

<style scoped>
.workspace {
  display: grid;
  gap: 14px;
  padding-top: 8px;
}

.panel {
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  padding: 14px;
  background: var(--docsy-surface-elevated);
  box-shadow: 0 3px 14px rgba(54, 45, 36, 0.035);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 12px;
}

.panel-header.compact {
  align-items: center;
}

.panel-actions {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.help-button {
  color: var(--docsy-text);
}

h3 {
  margin: 0 0 4px;
  font-size: 16px;
  color: var(--docsy-text-strong);
}

p {
  margin: 0;
  color: var(--docsy-text-muted);
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.actions.inline {
  margin-top: 0;
}

.template-name {
  max-width: 260px;
}

.document-collapse {
  margin-top: 12px;
}

.document-preview {
  max-height: 260px;
  overflow: auto;
  margin: 0;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-family: inherit;
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
}

.selection-tools {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-bottom: 10px;
  padding: 8px 10px;
  border: 1px solid #cddfd9;
  border-radius: 6px;
  background: var(--docsy-primary-soft);
}

.selection-count {
  color: var(--docsy-primary);
  font-size: 13px;
  font-weight: 600;
}

.suggestion-panel {
  display: grid;
  gap: 8px;
  margin-bottom: 10px;
  padding: 10px;
  border: 1px solid #d7e4d1;
  border-radius: 6px;
  background: #f2f7ef;
}

.suggestion-header,
.suggestion-item {
  display: flex;
  gap: 10px;
  align-items: center;
  justify-content: space-between;
}

.suggestion-header strong {
  color: var(--docsy-success);
  font-size: 13px;
}

.suggestion-list {
  display: grid;
  gap: 4px;
}

.suggestion-item {
  color: var(--docsy-text);
  font-size: 13px;
}

.group-input {
  max-width: 180px;
}

.type-input {
  width: 150px;
}

.type-help-item {
  display: grid;
  gap: 2px;
  color: var(--docsy-text);
  line-height: 1.45;
}

.type-help-item strong {
  color: var(--docsy-text-strong);
}

.field-rules {
  display: grid;
  gap: 12px;
  color: var(--docsy-text);
}

.field-rules h4 {
  margin: 0 0 6px;
  color: var(--docsy-text-strong);
  font-size: 13px;
}

.field-rules p {
  margin: 4px 0 0;
  color: var(--docsy-text);
  line-height: 1.55;
}

.rule-summary-collapse {
  margin-top: 10px;
}

.rule-summary-list {
  display: grid;
  gap: 8px;
}

.rule-summary-item {
  display: grid;
  grid-template-columns: minmax(120px, 180px) minmax(0, 1fr);
  gap: 10px;
  align-items: start;
  color: var(--docsy-text);
  font-size: 13px;
}

.rule-summary-item strong {
  color: var(--docsy-text-strong);
}

.template-build-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 12px;
}

.preview-panel {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--docsy-border-subtle);
}

.preview-panel-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: baseline;
  margin-bottom: 8px;
}

.preview-panel-header h3,
.template-preview-grid h4 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-size: 14px;
}

.template-preview-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 12px;
}

.template-preview-grid section {
  min-width: 0;
}

.template-preview-text {
  min-height: 160px;
  max-height: 360px;
  overflow: auto;
  padding: 10px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-strong);
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}

.source-preview-text {
  user-select: text;
}

.source-run {
  white-space: pre-wrap;
}

.source-bold {
  font-weight: 700;
}

.source-italic {
  font-style: italic;
}

.source-underline {
  text-decoration: underline;
}

.preview-token {
  display: inline;
  margin: 0 1px;
  padding: 1px 3px;
  border: 0;
  border-radius: 3px;
  font: inherit;
  line-height: inherit;
  cursor: pointer;
}

.preview-text {
  background: var(--docsy-primary-soft);
  color: var(--docsy-primary-hover);
}

.preview-date {
  background: #f3e8ff;
  color: #6b2fa0;
}

.preview-select {
  background: #e6fffb;
  color: #0f766e;
}

.preview-party {
  background: #f0f9eb;
  color: #2f6f1f;
}

.preview-reference {
  background: #edf2ff;
  color: #364fc7;
  border-bottom: 1px dashed currentcolor;
}

.preview-checkbox {
  background: #fff7e6;
  color: #9a5b13;
}

.preview-radio {
  background: #fff1f0;
  color: #b42318;
}

.preview-checkbox-group {
  background: #eef2ff;
  color: #4338ca;
}

.preview-prefix {
  background: #fdf6ec;
  color: #9a5b13;
}

.preview-suffix {
  background: #f6f6f6;
  color: var(--docsy-text);
}

.preview-delete-text {
  background: #fef0f0;
  color: #c45656;
  text-decoration: line-through;
}

.preview-ignore {
  background: var(--docsy-surface-muted);
  color: var(--docsy-text-muted);
}

.preview-deleted {
  color: var(--docsy-text-muted);
  text-decoration: line-through;
  opacity: 0.7;
}

.preview-focused {
  outline: 2px solid var(--docsy-primary);
}

.preview-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 8px;
}

.preview-selection-status {
  max-width: 360px;
  padding: 3px 8px;
  border: 1px solid var(--docsy-border-strong);
  border-radius: 4px;
  color: var(--docsy-text);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-action-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--docsy-border-subtle);
  color: var(--docsy-text);
  font-size: 13px;
}

.legend-token {
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.settings-badge {
  line-height: 1;
}

.reference-suggestion {
  display: grid;
  gap: 6px;
  margin-bottom: 12px;
  padding: 10px;
  border: 1px solid #fcd3d3;
  border-radius: 6px;
  background: #fef0f0;
  color: var(--docsy-text);
}

.reference-suggestion p {
  color: var(--docsy-text);
  line-height: 1.5;
}

.reference-actions {
  margin-top: 2px;
}

.mark-cell {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  max-width: 100%;
}

.mark-text {
  display: -webkit-box;
  min-width: 0;
  overflow: hidden;
  line-height: 1.35;
  word-break: break-word;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.structure-mark {
  color: var(--docsy-text-muted);
}

.relation-arrow {
  color: var(--docsy-text-muted);
  flex: 0 0 auto;
}

.party-item-arrow {
  color: #67c23a;
  flex: 0 0 auto;
  font-weight: 700;
}

.field-cell {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.field-label {
  color: var(--docsy-text-muted);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stack-input {
  margin-top: 4px;
}

.symbol-pair {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.muted {
  color: #c0c4cc;
}

.party-child-field > span:first-child {
  color: var(--docsy-primary);
  font-weight: 600;
}

.ignored-field-row {
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-muted);
  opacity: 0.72;
}

.ignored-field-row .el-input__wrapper,
.ignored-field-row .el-select__wrapper {
  background: var(--docsy-surface-muted);
}

.delete-field-row {
  color: #c45656;
  background: #fef0f0;
}

.party-child-row {
  color: var(--docsy-text);
}

.party-group-row {
  font-weight: 600;
}

.party-child-row .el-table-column--selection .cell {
  visibility: hidden;
}

.party-group-row .el-table-column--selection .cell {
  visibility: hidden;
}

.grouped-field-row .mark-cell {
  padding-left: 14px;
}

.party-child-row.grouped-field-row .mark-cell {
  padding-left: 28px;
}

.grouped-field-row td:first-child {
  border-left-width: 4px;
  border-left-style: solid;
}

.grouped-field-row-0 td {
  background: #f2f8ff;
}

.grouped-field-row-0 td:first-child {
  border-left-color: var(--docsy-primary);
}

.grouped-field-row-1 td {
  background: #f1fbf3;
}

.grouped-field-row-1 td:first-child {
  border-left-color: #67c23a;
}

.grouped-field-row-2 td {
  background: #f9f4ff;
}

.grouped-field-row-2 td:first-child {
  border-left-color: #8e5cf7;
}

.grouped-field-row-3 td {
  background: #fff8ed;
}

.grouped-field-row-3 td:first-child {
  border-left-color: #e6a23c;
}

.grouped-field-row-4 td {
  background: #fff2f2;
}

.grouped-field-row-4 td:first-child {
  border-left-color: #f56c6c;
}

.grouped-field-row-5 td {
  background: #f0fbff;
}

.grouped-field-row-5 td:first-child {
  border-left-color: #14b8c5;
}

.select-options-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}

.select-option-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.select-option-input {
  flex: 1;
}

.preview-sample-form {
  display: grid;
  gap: 8px;
}

.field-panel {
  min-height: 0;
}

@media (max-width: 1180px) {
  .template-preview-grid {
    grid-template-columns: 1fr;
  }

  .panel-header,
  .panel-header.compact {
    align-items: flex-start;
    flex-direction: column;
  }

  .panel-actions {
    display: flex;
    flex-wrap: wrap;
    align-self: stretch;
  }

  .template-name {
    flex: 1 1 220px;
    max-width: none;
  }
}
</style>

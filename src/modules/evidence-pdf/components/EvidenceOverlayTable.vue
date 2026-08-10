<template>
  <el-table
    :data="overlayRows"
    :row-key="row => row.path"
    size="small"
    border
    class="overlay-table"
    highlight-current-row
    @row-click="(row) => $emit('row-click', row)"
    @sort-change="(sort) => $emit('sort-change', sort)"
    @expand-change="(row, expanded) => $emit('expand-change', row, expanded)"
  >
    <el-table-column type="expand" width="36">
      <template #default="{ row, $index }">
        <div class="content-subrows">
          <template
            v-for="contentRows in [buildFileContentRows(row, $index, currentRules)]"
            :key="`${row.path}:${contentRows.length}`"
          >
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
                    :model-value="editingContentRowValue"
                    size="small"
                    @click.stop
                    @update:model-value="$emit('update:editingContentRowValue', $event)"
                    @blur="cr.source === 'existing' ? $emit('cancel-content-edit') : $emit('finish-content-edit', row, cr)"
                    @keyup.enter="$emit('finish-content-edit', row, cr)"
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
                      @dblclick.stop="$emit('start-content-edit', row, cr)"
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
                    @click.stop="$emit('cancel-existing-decision', row, cr)"
                  >
                    <el-icon><RefreshLeft /></el-icon>
                  </el-button>
                  <el-button
                    v-if="cr.source === 'new'"
                    link
                    size="small"
                    type="danger"
                    @click.stop="$emit('remove-content-new', row, cr)"
                  >
                    <el-icon><Delete /></el-icon>
                  </el-button>
                  <el-button
                    v-if="cr.source === 'existing'"
                    link
                    size="small"
                    type="danger"
                    @click.stop="$emit('remove-content-existing', row, cr)"
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
        <button class="file-link" type="button" :data-reorder-index="$index" @click.stop="$emit('open-file', row)">
          {{ row.name }}
        </button>
      </template>
    </el-table-column>
    <el-table-column label="页眉/页脚文本" min-width="200" show-overflow-tooltip :tooltip-props="{ placement: 'right' }">
      <template #default="{ row, $index }">
        <el-input
          v-if="editingContentRowId && editingContentRowId.startsWith(`${row.path}|`)"
          :model-value="editingContentRowValue"
          size="small"
          @click.stop
          @update:model-value="$emit('update:editingContentRowValue', $event)"
          @blur="$emit('finish-main-column-edit', row)"
          @keyup.enter="$emit('finish-main-column-edit', row)"
        />
        <span v-else class="table-text editable-text" @dblclick.stop="$emit('start-main-column-edit', row, $index)">
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
        <el-button link size="small" :disabled="$index === 0" @click.stop="$emit('move-file', $index, -1)">
          <el-icon><Top /></el-icon>
        </el-button>
        <el-button
          link
          size="small"
          :disabled="$index === overlayRows.length - 1"
          @click.stop="$emit('move-file', $index, 1)"
        >
          <el-icon><Bottom /></el-icon>
        </el-button>
        <el-button link type="danger" size="small" @click.stop="$emit('remove-file', $index)">
          <el-icon><Delete /></el-icon>
        </el-button>
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup>
import { Delete, Bottom, Rank, RefreshLeft, Top } from '@element-plus/icons-vue'

defineProps({
  overlayRows: { type: Array, required: true },
  currentRules: { type: Object, required: true },
  workflowMode: { type: String, required: true },
  pageNumberSequence: { type: String, default: 'continuous' },
  editingContentRowId: { type: String, default: '' },
  editingContentRowValue: { type: String, default: '' },
  // Utility functions
  buildFileContentRows: { type: Function, required: true },
  pageRangeText: { type: Function, required: true },
  sourceRangeText: { type: Function, required: true },
  contentKindLabel: { type: Function, required: true },
  contentStatusLabel: { type: Function, required: true },
  contentStatusTagType: { type: Function, required: true },
  displayContentRowText: { type: Function, required: true },
  mainColumnContentText: { type: Function, required: true },
  // Reorder functions
  startOverlayReorder: { type: Function, required: true },
  moveOverlayReorder: { type: Function, required: true },
  finishOverlayReorder: { type: Function, required: true },
  resetOverlayReorder: { type: Function, required: true },
})

defineEmits([
  'row-click',
  'sort-change',
  'expand-change',
  'open-file',
  'move-file',
  'remove-file',
  'start-content-edit',
  'finish-content-edit',
  'cancel-content-edit',
  'start-main-column-edit',
  'finish-main-column-edit',
  'cancel-existing-decision',
  'remove-content-new',
  'remove-content-existing',
  'update:editingContentRowValue',
])
</script>

<template>
  <div class="merged-import-plan">
    <div class="plan-head">
      <div>
        <div class="block-title">合并证据页段确认</div>
        <p class="hint">核对页段后拆成证据列表</p>
      </div>
      <div class="plan-actions">
        <el-button size="small" @click="$emit('add-range')">添加页段</el-button>
        <el-button size="small" @click="$emit('select-output-dir')">输出目录</el-button>
        <el-button size="small" @click="$emit('cancel')">取消</el-button>
        <el-button size="small" type="primary" :loading="splittingMergedImport" @click="$emit('execute')">
          确认拆分
        </el-button>
      </div>
    </div>
    <div class="import-plan-meta">
      <span>总页数：{{ plan.totalPages || '-' }}</span>
      <span>已扫描：{{ plan.pagesAnalyzed || '-' }} 页</span>
      <span>页眉：{{ plan.headerPages || 0 }} 页</span>
      <span>页码页脚：{{ plan.pageNumberFooterPages || 0 }} 页</span>
      <span>输出目录：{{ plan.outputDir }}</span>
    </div>
    <div class="split-name-options">
      <div class="block-title">拆分文件名</div>
      <div class="rule-grid">
        <div class="rule-item">
          <label>前缀</label>
          <el-input :model-value="splitNamePrefix" @update:model-value="$emit('update:splitNamePrefix', $event)" placeholder="可选" />
        </div>
        <div class="rule-item">
          <label>后缀</label>
          <el-input :model-value="splitNameSuffix" @update:model-value="$emit('update:splitNameSuffix', $event)" placeholder="例如 [YYYYMMDD]、-[YYYYMMDD]、[##]" />
        </div>
        <div class="rule-item">
          <label>日期值</label>
          <el-input :model-value="splitNameDateValue" @update:model-value="$emit('update:splitNameDateValue', $event)" placeholder="YYYYMMDD" />
        </div>
        <div class="rule-item">
          <label>分隔符</label>
          <el-select :model-value="splitNameSeparator" @update:model-value="$emit('update:splitNameSeparator', $event)">
            <el-option label="-" value="-" />
            <el-option label="_" value="_" />
            <el-option label="空格" value=" " />
            <el-option label="无" value="" />
            <el-option label="自定义" value="custom" />
          </el-select>
        </div>
        <div class="rule-item" v-if="splitNameSeparator === 'custom'">
          <label>自定义分隔符</label>
          <el-input :model-value="splitNameCustomSeparator" @update:model-value="$emit('update:splitNameCustomSeparator', $event)" placeholder="输入分隔符" />
        </div>
      </div>
    </div>
    <div class="split-cleanup-options">
      <div class="block-title">拆分后处理</div>
      <el-checkbox :model-value="splitCleanupHeader" @update:model-value="$emit('update:splitCleanupHeader', $event)">删除页眉区内容</el-checkbox>
      <el-checkbox :model-value="splitCleanupFooter" @update:model-value="$emit('update:splitCleanupFooter', $event)">删除原页码/页脚区内容</el-checkbox>
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
      :data="plan.items"
      size="small"
      border
      highlight-current-row
      @row-click="(row) => $emit('select-range', row)"
      @sort-change="(sort) => $emit('sort-items', sort)"
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
            :max="plan.totalPages || 999999"
            size="small"
          />
        </template>
      </el-table-column>
      <el-table-column label="结束页" prop="pageEnd" sortable="custom" width="108">
        <template #default="{ row }">
          <el-input-number
            v-model="row.pageEnd"
            :min="1"
            :max="plan.totalPages || 999999"
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
          <el-button link type="primary" size="small" @click.stop="$emit('select-range', row)">跳转</el-button>
          <el-button link type="primary" size="small" @click.stop="$emit('insert-range-after', $index)"
            >续段</el-button
          >
          <el-button link type="danger" size="small" @click.stop="$emit('remove-range', $index)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup>
import { Rank } from '@element-plus/icons-vue'

defineProps({
  plan: { type: Object, required: true },
  splittingMergedImport: { type: Boolean, default: false },
  mergedImportWarnings: { type: Array, default: () => [] },
  splitNamePrefix: { type: String, default: '' },
  splitNameSuffix: { type: String, default: '' },
  splitNameDateValue: { type: String, default: '' },
  splitNameSeparator: { type: String, default: '-' },
  splitNameCustomSeparator: { type: String, default: '' },
  splitCleanupHeader: { type: Boolean, default: false },
  splitCleanupFooter: { type: Boolean, default: false },
  splitOutputNamePreview: { type: Function, required: true },
  mergedImportRangePageCount: { type: Function, required: true },
  mergedImportSourceType: { type: Function, required: true },
  mergedImportSourceText: { type: Function, required: true },
  startMergedReorder: { type: Function, required: true },
  moveMergedReorder: { type: Function, required: true },
  finishMergedReorder: { type: Function, required: true },
  resetMergedReorder: { type: Function, required: true },
})

defineEmits([
  'add-range',
  'select-output-dir',
  'cancel',
  'execute',
  'select-range',
  'sort-items',
  'insert-range-after',
  'remove-range',
  'update:splitNamePrefix',
  'update:splitNameSuffix',
  'update:splitNameDateValue',
  'update:splitNameSeparator',
  'update:splitNameCustomSeparator',
  'update:splitCleanupHeader',
  'update:splitCleanupFooter',
])
</script>

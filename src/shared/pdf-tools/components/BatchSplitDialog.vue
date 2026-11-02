<template>
  <el-dialog
    :model-value="modelValue"
    :title="mode === 'starts' ? '按起点拆开当前段' : '批量分段'"
    width="640px"
    destroy-on-close
    @update:model-value="emit('update:modelValue', $event)"
  >
    <p class="batch-source">
      将「{{ original.name }}」PDF 第 {{ original.pageStart }}–{{ original.pageEnd }} 页分成多段，其余页段保持原样。
    </p>
    <div class="batch-fields">
      <label for="batch-mode">分段方式</label>
      <el-select id="batch-mode" v-model="mode" aria-label="批量分段方式">
        <el-option value="starts" label="按起点拆开当前段" />
        <el-option value="count" label="按段数均分" />
        <el-option value="pages" label="每段固定页数" />
        <el-option value="lengths" label="填写各段页数" />
      </el-select>
      <template v-if="mode === 'count'">
        <label>分为几段</label
        ><SplitRangeBoundaryInput
          :value="Number(count)"
          :min="2"
          :max="Math.max(2, sourcePages)"
          label="批量段数"
          @commit="count = $event"
        />
      </template>
      <template v-else-if="mode === 'pages'">
        <label>每段页数</label
        ><SplitRangeBoundaryInput :value="Number(pages)" :max="sourcePages" label="每段页数" @commit="pages = $event" />
      </template>
      <template v-else-if="mode === 'lengths'">
        <label for="batch-lengths">各段页数</label
        ><el-input
          id="batch-lengths"
          v-model="lengths"
          placeholder="例如 1, 4, 27；支持逗号、空格或换行"
          type="textarea"
          :rows="2"
        />
      </template>
      <template v-else>
        <label for="batch-starts">新增起始页</label>
        <el-input
          id="batch-starts"
          v-model="starts"
          placeholder="例如 208；可输入多个起点：208, 262"
          aria-label="新增页段起始页"
        />
      </template>
      <label for="batch-prefix">名称前缀</label><el-input id="batch-prefix" v-model="prefix" placeholder="文件" />
    </div>
    <p class="batch-note">
      {{
        mode === 'starts'
          ? '原段保留到首个新起点的前一页；每个新段延续至下一起点前一页，最后一段到原段末页。'
          : '末尾不足一段的余页会单独保留。'
      }}可在列表继续改名，整批操作可撤销。
    </p>
    <el-alert v-if="result.error" :title="result.error" type="warning" :closable="false" show-icon />
    <template v-else>
      <div class="batch-summary">
        共 {{ result.items.length }} 段 · {{ sourcePages }} 页<span v-if="result.items.length > 30"
          >（先展示前 30 段）</span
        >
      </div>
      <el-table :data="result.items.slice(0, 30)" size="small" border max-height="260">
        <el-table-column label="文件名" prop="name" min-width="160" />
        <el-table-column label="PDF 页序" width="150"
          ><template #default="{ row }">{{ row.pageStart }}–{{ row.pageEnd }}</template></el-table-column
        >
        <el-table-column label="页数" width="70"
          ><template #default="{ row }">{{ row.pageEnd - row.pageStart + 1 }}</template></el-table-column
        >
      </el-table>
    </template>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :disabled="Boolean(result.error)" @click="apply">{{
        result.items.length ? `应用 ${result.items.length} 个页段` : '应用分段'
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import SplitRangeBoundaryInput from './SplitRangeBoundaryInput.vue'
import { batchSplitRanges } from '../composables/batchSplitRanges.js'

const props = defineProps({
  modelValue: Boolean,
  source: { type: Object, default: null },
  inputPath: { type: String, default: '' },
  initialMode: { type: String, default: 'count' },
  initialPage: { type: Number, default: 0 },
})
const emit = defineEmits(['update:modelValue', 'apply'])
const original = ref({}),
  originalPath = ref('')
const mode = ref('count'),
  count = ref(2),
  pages = ref(10),
  lengths = ref(''),
  starts = ref(''),
  prefix = ref('')
watch(
  () => props.modelValue,
  (visible) => {
    if (!visible) return
    original.value = { ...props.source }
    originalPath.value = props.inputPath
    mode.value = props.initialMode
    prefix.value = props.initialMode === 'starts' ? '新页段' : props.source?.name || '文件'
    starts.value =
      props.initialPage > props.source?.pageStart && props.initialPage <= props.source?.pageEnd
        ? String(props.initialPage)
        : ''
    count.value = 2
    lengths.value = ''
  },
  { immediate: true },
)
const sourcePages = computed(() => Number(original.value.pageEnd || 0) - Number(original.value.pageStart || 1) + 1)
const result = computed(() => {
  try {
    return {
      items: batchSplitRanges(original.value, {
        mode: mode.value,
        count: count.value,
        pages: pages.value,
        lengths: lengths.value,
        starts: starts.value,
        prefix: prefix.value,
      }),
      error: '',
    }
  } catch (error) {
    return { items: [], error: error.message }
  }
})
function apply() {
  if (result.value.error) return
  emit('apply', {
    inputPath: originalPath.value,
    pageStart: original.value.pageStart,
    pageEnd: original.value.pageEnd,
    rangeId: original.value.rangeId,
    items: result.value.items,
  })
}
</script>

<style scoped>
.batch-source,
.batch-note {
  margin: 0 0 12px;
  line-height: 1.6;
}
.batch-note {
  font-size: 12px;
  color: var(--docsy-text-muted);
  margin-top: 10px;
}
.batch-fields {
  display: grid;
  grid-template-columns: 84px minmax(0, 1fr);
  gap: 10px 12px;
  align-items: center;
}
.batch-summary {
  margin: 12px 0 8px;
  font-size: 12px;
  color: var(--docsy-text-muted);
}
</style>

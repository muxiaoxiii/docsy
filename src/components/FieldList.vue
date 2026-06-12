<template>
  <div class="field-list">
    <div
      v-for="(item, i) in items"
      :key="i"
      class="field-list-row"
    >
      <el-autocomplete
        :model-value="item"
        :placeholder="field.placeholder || '输入或检索'"
        :fetch-suggestions="fetch"
        clearable
        fit-input-width
        style="flex: 1"
        @input="(v) => update(i, v)"
        @select="(s) => update(i, s.value)"
      />
      <el-button
        size="small"
        link
        type="danger"
        @click="remove(i)"
        :disabled="items.length === 1"
      >
        删除
      </el-button>
    </div>
    <el-button size="small" link type="primary" @click="add">
      + 添加一项
    </el-button>
  </div>
</template>

<script setup>
import { computed } from "vue";

const props = defineProps({
  modelValue: { type: Array, default: () => [] },
  field: { type: Object, required: true },
  options: { type: Array, default: () => [] },
});
const emit = defineEmits(["update:modelValue"]);

const items = computed(() =>
  props.modelValue.length === 0 ? [""] : props.modelValue
);

function fetch(query, cb) {
  const all = props.options.length ? props.options : props.field.options || [];
  if (!all.length) {
    cb([]);
    return;
  }
  const q = (query || "").toLowerCase();
  const filtered = q
    ? all.filter((s) => String(s).toLowerCase().includes(q))
    : all;
  cb(filtered.map((s) => ({ value: s })));
}
function update(i, v) {
  const next = [...items.value];
  next[i] = v;
  emit("update:modelValue", next);
}
function add() {
  emit("update:modelValue", [...items.value, ""]);
}
function remove(i) {
  const next = items.value.filter((_, idx) => idx !== i);
  emit("update:modelValue", next.length ? next : [""]);
}
</script>

<style scoped>
.field-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-list-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>

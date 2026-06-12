<template>
  <div class="field-party">
    <div
      v-for="(item, i) in items"
      :key="i"
      class="field-party-row"
    >
      <el-autocomplete
        :model-value="item"
        placeholder="姓名或公司全称（可检索）"
        :fetch-suggestions="fetch"
        clearable
        fit-input-width
        style="flex: 1"
        @input="(v) => update(i, v)"
        @select="(s) => update(i, s.value)"
      />
      <el-button size="small" link type="danger" @click="remove(i)">
        删除
      </el-button>
    </div>
    <el-button size="small" link type="primary" @click="add">
      + 添加一{{ field.default_role || '位' }}
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

const items = computed(() => props.modelValue);

function fetch(query, cb) {
  if (!props.options.length) {
    cb([]);
    return;
  }
  const q = (query || "").toLowerCase();
  const filtered = q
    ? props.options.filter((s) => String(s).toLowerCase().includes(q))
    : props.options;
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
  emit(
    "update:modelValue",
    items.value.filter((_, idx) => idx !== i)
  );
}
</script>

<style scoped>
.field-party {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-party-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>

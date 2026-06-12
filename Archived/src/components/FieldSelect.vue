<template>
  <el-autocomplete
    :model-value="modelValue"
    :placeholder="field.placeholder || '请选择或输入'"
    :fetch-suggestions="fetch"
    clearable
    fit-input-width
    style="width: 100%"
    @input="(v) => emit('update:modelValue', v)"
    @select="(item) => emit('update:modelValue', item.value)"
  />
</template>

<script setup>
const props = defineProps({
  modelValue: { type: String, default: "" },
  field: { type: Object, required: true },
  options: { type: Array, default: () => [] },
});
const emit = defineEmits(["update:modelValue"]);

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
</script>

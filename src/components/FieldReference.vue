<template>
  <el-autocomplete
    :model-value="modelValue"
    :placeholder="placeholder"
    :fetch-suggestions="fetch"
    clearable
    fit-input-width
    style="width: 100%"
    @input="(v) => emit('update:modelValue', v)"
    @select="(item) => emit('update:modelValue', item.value)"
  />
</template>

<script setup>
import { computed } from "vue";

const props = defineProps({
  modelValue: { type: String, default: "" },
  field: { type: Object, required: true },
  allValues: { type: Object, required: true },
  allFields: { type: Array, default: () => [] },
});
const emit = defineEmits(["update:modelValue"]);

const placeholder = computed(() => {
  if (props.field.references_role) return "选择身份或自行输入";
  if (props.field.references_name) return "从当事人中选或输入";
  return "请选择或输入";
});

const candidates = computed(() => {
  const result = [];
  const seen = new Set();
  const push = (v) => {
    if (v && !seen.has(v)) {
      seen.add(v);
      result.push(v);
    }
  };

  // references_role：从相关字段的 default_role 提取，不重复
  if (props.field.references_role) {
    for (const k of props.field.references_role) {
      const f = props.allFields.find((x) => x.key === k);
      if (f?.default_role) push(f.default_role);
    }
    if (!result.length) {
      ["原告", "被告", "第三人"].forEach(push);
    }
  }

  // references_name：从原告/被告/第三人的当前值合并
  if (props.field.references_name) {
    for (const k of props.field.references_name) {
      const v = props.allValues[k];
      if (Array.isArray(v)) v.forEach(push);
      else if (v) push(String(v));
    }
  }

  return result;
});

function fetch(query, cb) {
  const all = candidates.value;
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

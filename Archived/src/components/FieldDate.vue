<template>
  <div class="field-date">
    <el-input
      v-model="parts.y"
      placeholder="年"
      style="width: 90px"
      @input="emitChange"
    />
    <span class="unit">年</span>
    <el-input
      v-model="parts.m"
      placeholder="月"
      style="width: 70px"
      @input="emitChange"
    />
    <span class="unit">月</span>
    <el-input
      v-model="parts.d"
      placeholder="日"
      style="width: 70px"
      @input="emitChange"
    />
    <span class="unit">日</span>
    <el-button size="small" link @click="fillToday">今天</el-button>
    <el-button size="small" link type="info" @click="clearAll">清空</el-button>
  </div>
</template>

<script setup>
import { reactive, watch } from "vue";

const props = defineProps({
  modelValue: { type: String, default: "" },
  field: { type: Object, required: true },
});
const emit = defineEmits(["update:modelValue"]);

const parts = reactive({ y: "", m: "", d: "" });

watch(
  () => props.modelValue,
  (v) => {
    const parsed = parseDate(v);
    parts.y = parsed.y;
    parts.m = parsed.m;
    parts.d = parsed.d;
  },
  { immediate: true }
);

function parseDate(s) {
  if (!s) return { y: "", m: "", d: "" };
  // 已存的格式："YYYY 年 MM 月 DD 日"，留空时为空字符串
  const m = String(s).match(
    /^\s*(\d{0,4})\s*年\s*(\d{0,2})\s*月\s*(\d{0,2})\s*日\s*$/
  );
  if (m) return { y: m[1] || "", m: m[2] || "", d: m[3] || "" };
  return { y: "", m: "", d: "" };
}

function emitChange() {
  emit("update:modelValue", `${parts.y} 年 ${parts.m} 月 ${parts.d} 日`);
}

function fillToday() {
  const dt = new Date();
  parts.y = String(dt.getFullYear());
  parts.m = String(dt.getMonth() + 1);
  parts.d = String(dt.getDate());
  emitChange();
}

function clearAll() {
  parts.y = "";
  parts.m = "";
  parts.d = "";
  emitChange();
}
</script>

<style scoped>
.field-date {
  display: flex;
  align-items: center;
  gap: 4px;
}
.unit {
  color: #4b5563;
  font-size: 13px;
  margin-right: 4px;
}
</style>

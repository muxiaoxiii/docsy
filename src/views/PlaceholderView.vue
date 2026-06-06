<template>
  <el-empty :description="`${name} 待开发`" />
</template>

<script setup>
defineProps({
  name: { type: String, default: "页面" },
});
</script>

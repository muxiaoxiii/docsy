<template>
  <el-button link size="small" type="danger" :disabled="disabled" title="仅清空当前列表，不删除磁盘文件" @click="!disabled && $emit('clear')">清空列表</el-button>
</template>
<script setup>
defineProps({ disabled: Boolean })
defineEmits(['clear'])
</script>

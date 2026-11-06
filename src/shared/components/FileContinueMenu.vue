<template>
  <el-dropdown v-if="targets.length" trigger="click" :disabled="disabled || sending" @command="send">
    <el-button :size="size" :disabled="disabled" :loading="sending"
      >{{ label }}<el-icon class="el-icon--right"><ArrowDown /></el-icon
    ></el-button>
    <template #dropdown
      ><el-dropdown-menu
        ><el-dropdown-item v-for="target in targets" :key="target.id" :command="target.id"
          >{{ target.label }}{{ normalized.length > 1 ? `（${normalized.length} 个文件）` : '' }}</el-dropdown-item
        ></el-dropdown-menu
      ></template
    >
  </el-dropdown>
</template>
<script setup>
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowDown } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { transferPaths, transferTargets } from '../../core/fileTransfers.js'
import { useWorkspaceStore } from '../../stores/workspace.js'
const props = defineProps({
  paths: { type: [Array, String], default: () => [] },
  source: { type: String, default: '' },
  exclude: { type: String, default: '' },
  disabled: Boolean,
  size: { type: String, default: 'default' },
  label: { type: String, default: '继续处理' },
})
const normalized = computed(() => transferPaths(props.paths))
const targets = computed(() => transferTargets(normalized.value, props.exclude))
const sending = ref(false),
  router = useRouter(),
  store = useWorkspaceStore()
async function send(targetId) {
  if (props.disabled || sending.value) return
  const target = targets.value.find((t) => t.id === targetId)
  if (!target) return
  if (store.fileTransfer) {
    try {
      await ElMessageBox.confirm(
        `还有 ${store.fileTransfer.paths.length} 个文件尚未完成导入。是否改为发送这批文件？`,
        '更换待导入文件',
        { confirmButtonText: '更换', cancelButtonText: '保留原来的文件' },
      )
    } catch {
      return
    }
  }
  sending.value = true
  try {
    if (targetId === 'image-layout') {
      store.sendImagesToLayout(normalized.value, null, { sourceLabel: props.source, sourceKind: 'images' })
      if (store.fileTransfer) store.clearFileTransfer(store.fileTransfer.id)
    } else store.sendFiles(normalized.value, targetId, props.source)
    const failure = await router.push(target.route)
    if (failure && router.currentRoute.value.name !== target.route.name)
      throw new Error('未能打开目标工具，文件仍保留在待导入列表')
  } catch (error) {
    ElMessage.error(error.message || '发送失败，原文件仍可继续使用')
  } finally {
    sending.value = false
  }
}
</script>

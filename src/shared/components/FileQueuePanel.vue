<template>
  <section class="file-queue-panel" :style="{ '--queue-max-height': maxHeight }">
    <div class="queue-summary">
      <span>{{ items.length }} 个文件</span>
      <el-button v-if="items.length && clearable" link size="small" type="danger" @click="$emit('clear')">
        清空列表
      </el-button>
    </div>

    <el-scrollbar v-if="items.length" class="queue-scrollbar">
      <div class="queue-list">
        <article v-for="(item, index) in items" :key="itemKey(item, index)" class="queue-item">
          <slot name="leading" :item="item" :index="index" />
          <div class="queue-item-main">
            <div class="queue-item-name" :title="itemLabel(item)">{{ itemLabel(item) }}</div>
            <div v-if="$slots.meta" class="queue-item-meta">
              <slot name="meta" :item="item" :index="index" />
            </div>
          </div>
          <div class="queue-item-actions">
            <slot name="item-actions" :item="item" :index="index">
              <el-button v-if="removable" link size="small" type="danger" @click="$emit('remove', index)">
                删除
              </el-button>
            </slot>
          </div>
        </article>
      </div>
    </el-scrollbar>

    <div v-else class="queue-empty">
      {{ emptyText }}
    </div>
  </section>
</template>

<script setup>
defineProps({
  items: {
    type: Array,
    default: () => [],
  },
  emptyText: {
    type: String,
    default: '尚未添加文件',
  },
  maxHeight: {
    type: String,
    default: 'min(52vh, 460px)',
  },
  clearable: {
    type: Boolean,
    default: true,
  },
  removable: {
    type: Boolean,
    default: true,
  },
})

defineEmits(['clear', 'remove'])

function itemLabel(item) {
  return String(item?.name || item?.path || item || '')
}

function itemKey(item, index) {
  return String(item?.id || item?.path || item || index)
}
</script>

<style scoped>
.file-queue-panel {
  overflow: hidden;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 6px;
  background: var(--docsy-surface-muted);
}

.queue-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 38px;
  padding: 0 12px;
  color: var(--docsy-text-muted);
  font-size: 12px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface-elevated);
}

.queue-scrollbar {
  max-height: var(--queue-max-height);
}

.queue-list {
  display: grid;
  gap: 1px;
  background: var(--docsy-border-subtle);
}

.queue-item {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  padding: 10px 12px;
  background: var(--docsy-surface-elevated);
}

.queue-item-main {
  flex: 1;
  min-width: 0;
}

.queue-item-name {
  overflow: hidden;
  color: var(--docsy-text-strong);
  font-size: 13px;
  line-height: 1.4;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.queue-item-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
}

.queue-item-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
}

.queue-empty {
  display: grid;
  min-height: 132px;
  place-items: center;
  padding: 18px;
  color: var(--docsy-text-muted);
  font-size: 13px;
}
</style>

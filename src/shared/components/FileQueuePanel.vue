<template>
  <section class="file-queue-panel">
    <div class="queue-summary">
      <span>{{ items.length }} 个文件</span>
      <el-button v-if="items.length && clearable" link size="small" type="danger" @click="$emit('clear')">
        清空列表
      </el-button>
    </div>

    <el-scrollbar v-if="items.length" class="queue-scrollbar" :max-height="maxHeight">
      <div class="queue-list">
        <article
          v-for="(item, index) in items"
          :key="itemKey(item, index)"
          class="queue-item"
          :data-reorder-index="index"
          :class="itemClasses(index)"
        >
          <button
            v-if="sortable"
            type="button"
            class="queue-drag-handle"
            title="拖动调整顺序"
            aria-label="拖动调整顺序"
            @pointerdown.stop="start(index, $event)"
            @pointermove.stop="move"
            @pointerup.stop="finish"
            @pointercancel.stop="reset"
          >
            <el-icon><Rank /></el-icon>
          </button>
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
import { Rank } from '@element-plus/icons-vue'
import { usePointerReorder } from '../../core/composables/usePointerReorder.js'

const props = defineProps({
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
  sortable: {
    type: Boolean,
    default: false,
  },
})

const emit = defineEmits(['clear', 'remove', 'reorder'])
const { start, move, finish, reset, itemClasses } = usePointerReorder({
  itemCount: () => props.items.length,
  onReorder: (payload) => emit('reorder', payload),
})

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

.queue-item.is-reorder-dragging {
  opacity: 0.55;
}

.queue-item.is-reorder-before {
  box-shadow: inset 0 2px 0 var(--docsy-primary);
}

.queue-item.is-reorder-after {
  box-shadow: inset 0 -2px 0 var(--docsy-primary);
}

.queue-drag-handle {
  display: inline-grid;
  flex: 0 0 28px;
  width: 28px;
  height: 28px;
  padding: 0;
  color: var(--docsy-text-muted);
  cursor: grab;
  touch-action: none;
  user-select: none;
  border: 0;
  background: transparent;
  place-items: center;
}

.queue-drag-handle:active {
  cursor: grabbing;
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

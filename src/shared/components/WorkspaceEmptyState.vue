<template>
  <section class="workspace-empty-state" :class="{ compact }">
    <span
      class="empty-state-icon"
      :style="{ '--empty-icon-url': `url(${iconUrl || defaultIconUrl})` }"
      aria-hidden="true"
    ></span>
    <strong>{{ title }}</strong>
    <p v-if="description">{{ description }}</p>
    <div v-if="$slots.actions" class="empty-state-actions">
      <slot name="actions" />
    </div>
  </section>
</template>

<script setup>
import defaultIconUrl from '../../assets/icons/documents.svg?url'

defineProps({
  title: {
    type: String,
    default: '等待添加文件',
  },
  description: {
    type: String,
    default: '',
  },
  iconUrl: {
    type: String,
    default: '',
  },
  compact: {
    type: Boolean,
    default: false,
  },
})
</script>

<style scoped>
.workspace-empty-state {
  display: flex;
  min-height: 260px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px 24px;
  color: var(--docsy-text-muted);
  text-align: center;
  border: 1px dashed var(--docsy-border-strong);
  border-radius: var(--docsy-panel-radius);
  background: color-mix(in srgb, var(--docsy-surface-muted) 76%, var(--docsy-surface-elevated));
}

.workspace-empty-state.compact {
  min-height: 132px;
  padding: 20px;
}

.empty-state-icon {
  width: 58px;
  height: 58px;
  margin-bottom: 16px;
  color: color-mix(in srgb, var(--docsy-primary) 78%, var(--docsy-text-muted));
  background: currentColor;
  mask-image: var(--empty-icon-url);
  mask-position: center;
  mask-repeat: no-repeat;
  mask-size: contain;
  -webkit-mask-image: var(--empty-icon-url);
  -webkit-mask-position: center;
  -webkit-mask-repeat: no-repeat;
  -webkit-mask-size: contain;
  opacity: 0.78;
}

.compact .empty-state-icon {
  width: 38px;
  height: 38px;
  margin-bottom: 10px;
}

.workspace-empty-state strong {
  color: var(--docsy-text-strong);
  font-size: 14px;
  font-weight: 680;
}

.workspace-empty-state p {
  max-width: 460px;
  margin: 7px 0 0;
  font-size: 12px;
  line-height: 1.65;
}

.empty-state-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  margin-top: 16px;
}
</style>

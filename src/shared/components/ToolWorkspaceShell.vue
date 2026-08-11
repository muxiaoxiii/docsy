<template>
  <section class="tool-workspace">
    <header class="workspace-header">
      <div class="workspace-heading">
        <h2>{{ title }}</h2>
        <p v-if="description">{{ description }}</p>
      </div>
      <div v-if="$slots['header-actions']" class="workspace-header-actions">
        <slot name="header-actions" />
      </div>
    </header>

    <div v-if="$slots.toolbar" class="workspace-toolbar">
      <slot name="toolbar" />
    </div>

    <div class="workspace-content">
      <slot />
    </div>

    <footer v-if="$slots.actions" class="workspace-actions">
      <slot name="actions" />
    </footer>
  </section>
</template>

<script setup>
defineProps({
  title: {
    type: String,
    required: true,
  },
  description: {
    type: String,
    default: '',
  },
})
</script>

<style scoped>
.tool-workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: 20px;
  background: var(--docsy-canvas);
}

.workspace-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  min-height: 0;
  padding: 18px 20px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-panel-radius);
  background: var(--docsy-surface-elevated);
  box-shadow: var(--docsy-shadow-soft);
}

.workspace-heading {
  min-width: 0;
}

.workspace-heading h2 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-family:
    ui-rounded,
    'SF Pro Rounded',
    -apple-system,
    'PingFang SC',
    sans-serif;
  font-size: 20px;
  font-weight: 720;
  letter-spacing: -0.02em;
  line-height: 1.35;
}

.workspace-heading p {
  max-width: 680px;
  margin: 7px 0 0;
  color: var(--docsy-text-muted);
  font-size: 13px;
  line-height: 1.5;
}

.workspace-header-actions,
.workspace-toolbar,
.workspace-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.workspace-toolbar {
  flex-wrap: wrap;
  min-height: 52px;
  margin-top: 12px;
  padding: 9px 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface-muted) 68%, var(--docsy-surface-elevated));
}

.workspace-content {
  flex: 1;
  min-height: 0;
  margin-top: 12px;
  overflow: auto;
}

.workspace-actions {
  flex-shrink: 0;
  flex-wrap: wrap;
  gap: 10px;
  min-height: 58px;
  margin-top: 12px;
  padding: 12px 16px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-panel-radius);
  background: var(--docsy-surface-elevated);
}

@media (max-width: 760px) {
  .tool-workspace {
    padding: 16px;
  }

  .workspace-header {
    flex-direction: column;
    gap: 12px;
  }
}
</style>

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
  padding-block: clamp(12px, 2.2dvh, 20px);
  padding-inline: 20px;
  background: var(--docsy-canvas);
}

.workspace-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  min-height: 0;
  padding-block: clamp(14px, 2.1dvh, 18px);
  padding-inline: 20px;
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

.workspace-header-actions {
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.workspace-toolbar {
  flex-wrap: wrap;
  min-height: clamp(46px, 5.8dvh, 52px);
  margin-top: clamp(8px, 1.5dvh, 12px);
  padding-block: clamp(7px, 1.2dvh, 9px);
  padding-inline: 12px;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  background: color-mix(in srgb, var(--docsy-surface-muted) 68%, var(--docsy-surface-elevated));
}

.workspace-content {
  flex: 1;
  min-height: 0;
  margin-top: clamp(8px, 1.5dvh, 12px);
  overflow: auto;
}

.workspace-actions {
  flex-shrink: 0;
  flex-wrap: wrap;
  gap: 10px;
  min-height: clamp(50px, 6.5dvh, 58px);
  margin-top: clamp(8px, 1.5dvh, 12px);
  padding-block: clamp(10px, 1.5dvh, 12px);
  padding-inline: 16px;
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

  .workspace-header-actions {
    width: 100%;
    justify-content: flex-start;
  }

  .workspace-toolbar,
  .workspace-actions {
    align-items: stretch;
  }
}
</style>

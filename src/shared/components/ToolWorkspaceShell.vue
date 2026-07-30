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
  min-height: 100%;
  padding: 22px 24px 24px;
  background: var(--docsy-surface);
}

.workspace-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  padding-bottom: 18px;
  border-bottom: 1px solid var(--docsy-border-subtle);
}

.workspace-heading {
  min-width: 0;
}

.workspace-heading h2 {
  margin: 0;
  color: var(--docsy-text-strong);
  font-size: 18px;
  font-weight: 650;
  line-height: 1.35;
}

.workspace-heading p {
  margin: 6px 0 0;
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
  padding: 14px 0;
}

.workspace-content {
  min-height: 0;
}

.workspace-actions {
  flex-wrap: wrap;
  gap: 10px;
  padding-top: 16px;
  border-top: 1px solid var(--docsy-border-subtle);
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

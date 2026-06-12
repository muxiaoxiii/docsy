<template>
  <TemplateEditorView
    :edit-template-id="editTemplateId"
    @templates-changed="$emit('templates-changed')"
  />
</template>

<script setup>
import TemplateEditorView from "../features/template-editor/TemplateEditorView.vue";

defineProps({
  editTemplateId: { type: String, default: null },
});

defineEmits(["templates-changed"]);
</script>

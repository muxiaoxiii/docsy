<template>
  <div class="doclet-working" role="status" aria-live="polite">
    <div class="doclet-working__sprite" :style="{ backgroundImage: `url(${spritesheet})` }" aria-hidden="true" />
    <div v-if="message" class="doclet-working__message">{{ message }}</div>
  </div>
</template>

<script setup>
import spritesheet from '../../assets/doclet-v2-spritesheet.webp'

defineProps({
  message: {
    type: String,
    default: 'Doclet 正在处理…',
  },
})
</script>

<style scoped>
.doclet-working {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-height: 156px;
  padding: 8px 14px;
  border: 1px solid #dbeafe;
  border-radius: 14px;
  background: linear-gradient(135deg, #f8fbff 0%, #eef6ff 100%);
  color: #36526f;
}

.doclet-working__sprite {
  width: 144px;
  height: 156px;
  flex: 0 0 144px;
  background-repeat: no-repeat;
  background-size: 1152px 1716px;
  background-position: 0 -1404px;
  image-rendering: auto;
  animation: doclet-look-around 5.6s steps(1, end) infinite;
}

.doclet-working__message {
  max-width: 180px;
  font-size: 13px;
  line-height: 1.5;
}

@keyframes doclet-look-around {
  0%,
  6.24% {
    background-position: 0 -1404px;
  }
  6.25%,
  12.49% {
    background-position: -144px -1404px;
  }
  12.5%,
  18.74% {
    background-position: -288px -1404px;
  }
  18.75%,
  24.99% {
    background-position: -432px -1404px;
  }
  25%,
  31.24% {
    background-position: -576px -1404px;
  }
  31.25%,
  37.49% {
    background-position: -720px -1404px;
  }
  37.5%,
  43.74% {
    background-position: -864px -1404px;
  }
  43.75%,
  49.99% {
    background-position: -1008px -1404px;
  }
  50%,
  56.24% {
    background-position: 0 -1560px;
  }
  56.25%,
  62.49% {
    background-position: -144px -1560px;
  }
  62.5%,
  68.74% {
    background-position: -288px -1560px;
  }
  68.75%,
  74.99% {
    background-position: -432px -1560px;
  }
  75%,
  81.24% {
    background-position: -576px -1560px;
  }
  81.25%,
  87.49% {
    background-position: -720px -1560px;
  }
  87.5%,
  93.74% {
    background-position: -864px -1560px;
  }
  93.75%,
  100% {
    background-position: -1008px -1560px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .doclet-working__sprite {
    animation: none;
  }
}
</style>

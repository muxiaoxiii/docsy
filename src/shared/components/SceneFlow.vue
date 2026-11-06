<template>
  <svg class="scene-flow" viewBox="0 0 120 40" fill="none" aria-hidden="true">
    <path d="M 10 20 C 40 6, 80 34, 110 20" stroke="currentColor" stroke-width="2" stroke-dasharray="4 4" />
    <g v-if="active">
      <circle r="3" />
      <circle r="2.4" />
      <circle r="2" />
    </g>
  </svg>
</template>
<script setup>
defineProps({ active: Boolean })
</script>
<style scoped>
.scene-flow {
  width: 100%;
  height: 100%;
  overflow: visible;
}
.scene-flow circle {
  fill: currentColor;
  animation: scene-flow-travel 2.2s linear infinite;
}
.scene-flow circle:nth-child(2) {
  animation-delay: -0.73s;
}
.scene-flow circle:nth-child(3) {
  animation-delay: -1.46s;
}
@keyframes scene-flow-travel {
  0% {
    transform: translate(10px, 20px);
    opacity: 0;
  }
  15% {
    opacity: 1;
  }
  35% {
    transform: translate(42px, 17px);
  }
  65% {
    transform: translate(78px, 23px);
  }
  85% {
    opacity: 1;
  }
  100% {
    transform: translate(110px, 20px);
    opacity: 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .scene-flow g {
    display: none;
  }
}
</style>

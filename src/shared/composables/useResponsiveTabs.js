import { onBeforeUnmount, onMounted, ref } from 'vue'

export function useResponsiveTabs() {
  const tabPosition = ref('left')
  let media
  const sync = () => { tabPosition.value = media.matches ? 'top' : 'left' }
  onMounted(() => {
    media = window.matchMedia('(max-width: 800px)')
    sync()
    media.addEventListener('change', sync)
  })
  onBeforeUnmount(() => media?.removeEventListener('change', sync))
  return tabPosition
}

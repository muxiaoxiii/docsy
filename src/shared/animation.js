/**
 * 保证异步操作具有最小视觉展示时长，避免极快完成时（如 30ms）动画一闪而过、界面抽搐。
 * @template T
 * @param {() => Promise<T>} task 实际执行的异步任务
 * @param {number} [minMs=650] 最小保留毫秒数
 * @returns {Promise<T>} 任务执行结果
 */
export async function withMinimumDuration(task, minMs = 650) {
  const start = Date.now()
  try {
    return await task()
  } finally {
    const elapsed = Date.now() - start
    if (elapsed < minMs) {
      await new Promise((resolve) => setTimeout(resolve, minMs - elapsed))
    }
  }
}

/**
 * 状态机庆祝动效控制器
 * @param {import('vue').Ref<boolean>} celebratingRef 庆祝状态响应式引用
 * @param {number} [duration=2200] 庆祝持续时长
 */
export function createCelebrationTrigger(celebratingRef, duration = 2200) {
  let timer = null
  const trigger = () => {
    if (timer) clearTimeout(timer)
    celebratingRef.value = true
    timer = setTimeout(() => {
      celebratingRef.value = false
      timer = null
    }, duration)
  }
  trigger.cancel = () => {
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
    celebratingRef.value = false
  }
  return trigger
}

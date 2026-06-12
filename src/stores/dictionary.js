import { defineStore } from 'pinia'
import { ref } from 'vue'
import { tauriCall } from '../core/tauriBridge.js'

export const useDictionaryStore = defineStore('dictionary', () => {
  const cache = ref({})

  async function query(dictName, options = {}) {
    const cacheKey = `${dictName}-${options.templateId || ''}-${options.fieldKey || ''}`
    if (cache.value[cacheKey]) {
      return cache.value[cacheKey]
    }

    try {
      const entries = await tauriCall('query_dictionary', {
        query: {
          dict_name: dictName,
          template_id: options.templateId,
          field_key: options.fieldKey,
          search: options.search,
          limit: options.limit,
        },
      })
      cache.value[cacheKey] = entries
      return entries
    } catch (err) {
      console.error('Dictionary query failed:', err)
      return []
    }
  }

  function clearCache() {
    cache.value = {}
  }

  return { query, clearCache }
})

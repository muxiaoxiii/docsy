import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'

export function useEvidenceFolder() {
  const evidenceFolder = ref('')
  const evidenceGroups = ref([])
  const scanning = ref(false)
  const building = ref(false)
  const conversionFailures = ref([])
  const groupFailures = ref([])
  const evidenceOutputDir = ref('')

  async function selectEvidenceFolder() {
    if (scanning.value || building.value) return
    const selected = await open({ directory: true })
    if (!selected || scanning.value || building.value) return
    evidenceFolder.value = selected
    evidenceGroups.value = []
    conversionFailures.value = []
    groupFailures.value = []
    evidenceOutputDir.value = ''
    await scanEvidence()
  }

  async function scanEvidence() {
    if (!evidenceFolder.value || scanning.value || building.value) return
    scanning.value = true
    evidenceGroups.value = []
    conversionFailures.value = []
    groupFailures.value = []
    evidenceOutputDir.value = ''
    try {
      const result = await tauriCallSafe('scan_evidence_folder', { root: evidenceFolder.value })
      if (result.ok) evidenceGroups.value = result.data.groups || []
      else ElMessage.error(userFacingError(result.error, '扫描失败'))
    } finally {
      scanning.value = false
    }
  }

  async function buildEvidence() {
    if (scanning.value || building.value || !evidenceGroups.value.length) return
    building.value = true
    conversionFailures.value = []
    groupFailures.value = []
    evidenceOutputDir.value = ''
    try {
      const result = await tauriCallSafe('build_evidence_group_pdfs', {
        args: { root: evidenceFolder.value, groups: JSON.parse(JSON.stringify(evidenceGroups.value)) },
      })
      if (!result.ok) {
        ElMessage.error(userFacingError(result.error, '证据 PDF 生成失败'))
        return
      }
      conversionFailures.value = result.data.failedConversions || []
      groupFailures.value = result.data.failedGroups || []
      const count = result.data.results?.length || 0
      const groupFailCount = groupFailures.value.length
      const convFailCount = conversionFailures.value.length

      if (!count) {
        ElMessage.warning(
          groupFailCount
            ? `所有分组生成均失败（共 ${groupFailCount} 个分组失败，详见错误列表）`
            : '未生成任何证据 PDF，请检查文件类型和转换失败详情',
        )
        return
      }
      evidenceOutputDir.value = result.data.evidenceDir || ''
      const failParts = []
      if (groupFailCount) failParts.push(`${groupFailCount} 个分组处理失败`)
      if (convFailCount) failParts.push(`${convFailCount} 个 Word 文件转换失败`)
      const failSuffix = failParts.length ? `，另有 ${failParts.join('、')}` : ''
      const message = `已生成 ${count} 个分组 PDF${failSuffix}`
      if (failParts.length) ElMessage.warning(message)
      else ElMessage.success(message)
    } finally {
      building.value = false
    }
  }

  return {
    evidenceFolder,
    evidenceGroups,
    scanning,
    building,
    conversionFailures,
    groupFailures,
    evidenceOutputDir,
    selectEvidenceFolder,
    scanEvidence,
    buildEvidence,
  }
}

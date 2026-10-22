import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { documentKind, reviewImageImports } from '../documentImport.js'
import { tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'

export function useEvidenceFolder({ reviewImages } = {}) {
  const outputMode = ref('per_folder')
  const duplexFiles = ref(false)
  const duplexGroups = ref(false)
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
      // Review image batches before starting the backend merge. Cancellation
      // leaves the unprocessed group intact rather than omitting its images.
      for (const group of evidenceGroups.value) {
        if (!group.files.some(file => documentKind(file.path) === 'image')) continue
        if (!reviewImages) { ElMessage.warning('请先确认图片排版'); return }
        let cancelled = false
        const paths = await reviewImageImports(group.files.map(file => file.path), async images => {
          const pdf = await reviewImages(images, { outputDir: `${evidenceFolder.value}/_evidence_output` })
          if (!pdf) { cancelled = true; throw new Error('cancelled') }
          return pdf
        }).catch(error => { if (!cancelled) throw error; return null })
        if (!paths) return
        group.files = paths.map(path => group.files.find(file => file.path === path) || { path, name: path.split(/[/\\]/).at(-1), fileType: 'pdf', fromImages: true })
      }
      const result = await tauriCallSafe('build_evidence_group_pdfs', {
        args: { root: evidenceFolder.value, groups: JSON.parse(JSON.stringify(evidenceGroups.value)), outputMode: outputMode.value, duplexFiles: duplexFiles.value, duplexGroups: outputMode.value === 'all' && duplexGroups.value },
      })
      if (!result.ok) {
        ElMessage.error(userFacingError(result.error, '证据 PDF 生成失败'))
        return
      }
      conversionFailures.value = result.data.failedConversions || []
      groupFailures.value = result.data.failedGroups || []
      const count = result.data.results?.length || 0
      if (!count) {
        ElMessage.warning('未生成任何证据 PDF，请检查文件类型和转换失败详情')
        return
      }
      evidenceOutputDir.value = result.data.evidenceDir || ''
      const message = `已生成 ${count} 份 PDF${groupFailures.value.length ? `，${groupFailures.value.length} 个分组未生成，请查看失败详情` : ''}${conversionFailures.value.length ? `，${conversionFailures.value.length} 个 Word 文件转换失败` : ''}`
      if (conversionFailures.value.length || groupFailures.value.length) ElMessage.warning(message)
      else ElMessage.success(message)
    } catch (error) {
      ElMessage.error(userFacingError(error, '生成失败，文件列表已保留'))
    } finally {
      building.value = false
    }
  }

  async function replaceFailedConversion(failure) {
    if (scanning.value || building.value) return
    const selected = await open({ multiple: false, filters: [{ name: '手动转换后的 PDF', extensions: ['pdf'] }] })
    const path = Array.isArray(selected) ? selected[0] : selected
    if (!path || scanning.value || building.value) return
    const group = evidenceGroups.value.find(group => group.id === failure.groupId)
    const index = group?.files.findIndex(file => file.path === failure.path) ?? -1
    if (index < 0) return
    group.files = group.files.map((file, position) => position === index
      ? { ...file, sourcePath: failure.path, path, name: path.split(/[\\/]/).at(-1), fileType: 'pdf' }
      : file).filter((file, position) => file.path !== path || position === index)
    conversionFailures.value = conversionFailures.value.filter(item => item !== failure)
    ElMessage.success('已替换该位置的文件，无需重新扫描；再次生成即可')
  }

  return { outputMode, duplexFiles, duplexGroups, evidenceFolder, evidenceGroups, scanning, building, conversionFailures, groupFailures, evidenceOutputDir, selectEvidenceFolder, scanEvidence, buildEvidence, replaceFailedConversion }
}

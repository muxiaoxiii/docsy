/**
 * Composable for template settings: separator, trash, clear history, template database.
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { tauriCallSafe, userFacingError } from '../../../core/tauriBridge.js'

export function useTemplateSettings(loadHistoryContext, loadTemplateHistoryRuns) {
  const itemSeparatorSetting = ref(window.localStorage.getItem('docsy.template.itemSeparator') || '、')
  const templateTrash = ref([])
  const templateTrashLoading = ref(false)
  const clearingHistory = ref(false)
  const templateDatabase = ref([])
  const templateDatabaseLoading = ref(false)

  function saveItemSeparatorSetting() {
    window.localStorage.setItem('docsy.template.itemSeparator', itemSeparatorSetting.value || '、')
    ElMessage.success('已保存多项字段连接符设置')
  }

  async function loadTemplateTrash() {
    templateTrashLoading.value = true
    const result = await tauriCallSafe('list_template_trash')
    templateTrashLoading.value = false
    if (result.ok) {
      templateTrash.value = result.data || []
    } else {
      ElMessage.error(userFacingError(result.error, '读取模板回收站失败'))
    }
  }

  async function restoreTemplate(row) {
    const result = await tauriCallSafe('restore_template_from_trash', { args: { path: row.path } })
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '恢复失败'))
      return
    }
    ElMessage.success('模板已恢复')
    await loadTemplateTrash()
    window.dispatchEvent(new CustomEvent('docsy-template-library-changed'))
  }

  async function permanentlyDeleteTemplate(row) {
    let migrateToCommon = false
    try {
      await ElMessageBox.confirm(
        `彻底删除"${row.name}"？可以先把该模板的内部填写数据迁移为模板通用数据，供其他模板按通用字段名继续检索。`,
        '彻底删除模板',
        {
          confirmButtonText: '迁移数据并删除',
          cancelButtonText: '直接删除数据',
          distinguishCancelAndClose: true,
          type: 'warning',
        },
      )
      migrateToCommon = true
    } catch (action) {
      if (action !== 'cancel') return
    }
    const result = await tauriCallSafe('permanently_delete_template', {
      args: { path: row.path, migrateToCommon },
    })
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '彻底删除失败'))
      return
    }
    ElMessage.success(migrateToCommon ? '模板已删除，数据已迁移为模板通用数据' : '模板和内部数据已删除')
    await loadTemplateTrash()
    window.dispatchEvent(new CustomEvent('docsy-template-library-changed'))
  }

  async function clearAllHistory() {
    try {
      await ElMessageBox.confirm('确定清空全部填写历史？删除后无法恢复，模板库文件不受影响。', '清空填写历史', {
        confirmButtonText: '清空',
        type: 'warning',
      })
    } catch {
      return
    }
    clearingHistory.value = true
    const result = await tauriCallSafe('clear_template_history')
    clearingHistory.value = false
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '清空历史失败'))
      return
    }
    ElMessage.success(`已清空 ${result.data ?? 0} 条填写记录`)
    if (loadHistoryContext) await loadHistoryContext(true)
    if (loadTemplateHistoryRuns) await loadTemplateHistoryRuns()
  }

  async function loadTemplateDatabase() {
    templateDatabaseLoading.value = true
    const result = await tauriCallSafe('list_template_database')
    templateDatabaseLoading.value = false
    if (result.ok) {
      templateDatabase.value = result.data || []
    } else {
      ElMessage.error(userFacingError(result.error, '读取模板数据库失败'))
    }
  }

  async function deleteTemplateDatabaseEntry(row) {
    try {
      await ElMessageBox.confirm(`删除"${row.name}"的所有填写历史数据？删除后无法恢复。`, '删除模板数据', {
        confirmButtonText: '删除',
        type: 'warning',
      })
    } catch {
      return
    }
    const result = await tauriCallSafe('delete_template_database_entry', { templatePath: row.templateId })
    if (!result.ok) {
      ElMessage.error(userFacingError(result.error, '删除失败'))
      return
    }
    ElMessage.success('模板数据已删除')
    await loadTemplateDatabase()
    if (loadHistoryContext) await loadHistoryContext(true)
    if (loadTemplateHistoryRuns) await loadTemplateHistoryRuns()
  }

  return {
    itemSeparatorSetting,
    templateTrash,
    templateTrashLoading,
    clearingHistory,
    templateDatabase,
    templateDatabaseLoading,
    saveItemSeparatorSetting,
    loadTemplateTrash,
    restoreTemplate,
    permanentlyDeleteTemplate,
    clearAllHistory,
    loadTemplateDatabase,
    deleteTemplateDatabaseEntry,
  }
}

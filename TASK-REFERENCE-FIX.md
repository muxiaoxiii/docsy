修复模板填写页面引用源选项和模板编辑自动引用类型的问题。

## 问题 1：填写页面引用源选项只有日期

TemplateRenderTab.vue 中 `referenceFillOptions(field)` 函数返回的选项只有日期类字段。
用户期望：填写页面的引用源选项应该跟模板编辑页面一样，显示所有可引用的字段。

排查：
- 找到 `referenceFillOptions` 函数，看它的过滤逻辑
- 对比模板编辑页面（TemplateBuildTab）的引用源选项逻辑（如 `referenceSourceOptions`、`allReferenceSuggestions`）
- 确保填写页面用同样的数据源

## 问题 2：模板编辑时同名同文本字段应自动设为引用类型

当前行为：useFieldNormalization.js 中，当两个字段同名时，设置 `fillAllPositions = true`，
UI 上 follower position 显示为引用类型，但字段的 `field.type` 属性仍然是 'text'。

用户期望：同名字段的 follower position 应该自动将 field.type 设为 'reference'，
而不是仅在 UI 层面显示为引用。

排查：
- 找到 useFieldNormalization.js 中设置 `fillAllPositions = true` 的位置
- 在那里同时设置 `field.type = 'reference'` 和 `field.reference` 属性
- 确保引用源指向同名的第一个位置

## 验证
npm test + npm run build 全过。

修复两个问题，只改相关文件。

## 问题 1：单页文件页眉检测不到

文件：src-tauri/src/pdf/detection.rs

在 build_candidates 函数末尾，当前代码（约 line 1244-1246）：
```rust
if pages_analyzed <= 1 {
    candidates.retain(|c| c.labels.iter().any(|l| l == "page-number"));
}
```

改为：不删除候选，而是降低 confidence 到 0.15。
前端已有低置信度处理逻辑（排序末尾 + 灰色 + 默认不勾选）。

```rust
if pages_analyzed <= 1 {
    for c in &mut candidates {
        if !c.labels.iter().any(|l| l == "page-number") {
            c.confidence = c.confidence.min(0.15);
        }
    }
}
```

同时更新对应测试：single-page 测试改为检查 confidence <= 0.15 而不是 is_empty。

## 问题 2：页脚文字 [文件名] 不解析

文件：src/modules/pdf-tools/composables/useEvidencePdfSession.js

expandPlaceholders（约 line 458）当前只处理 {page}/{total}/{range}。
需要增加对 [文件名]、[#]、[##]、[序号]、[中文序号]、[日期] 的支持。

改 expandPlaceholders 签名增加 file/index/rules 可选参数，
内部先调用 resolveTextTemplate 再处理 {page} 等：

```js
export function expandPlaceholders(template, page, total, file, index, rules) {
  let text = template || ''
  // Resolve [文件名], [#], [序号], [日期] etc.
  if (file != null) {
    text = resolveTextTemplate(text, file, index || 0, rules || {})
  }
  // Then resolve {page}, {total}, {range}
  return text
    .replaceAll('{page}', String(page))
    .replaceAll('{total}', String(total))
    .replaceAll('{range}', `${page}/${total}`)
}
```

然后更新所有调用 expandPlaceholders 的地方传入 file/index/rules：

1. useEvidencePdfPreview.js line 77:
   `expandPlaceholders(group.text, page, total)` →
   `expandPlaceholders(group.text, page, total, selectedOverlayFile.value, selectedOverlayIndex.value, currentRules.value)`

2. useEvidencePdfPreview.js line 217:
   同样补上 file/index/rules 参数

3. useEvidencePdfExistingEditing.js line 229:
   同样补上 file/index/rules 参数

## 验证
cargo test + npm test + npm run build 全过。

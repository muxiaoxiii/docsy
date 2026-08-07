# MDG-003: 代码重复消除

## 状态：🔴 未开始
## 优先级：P1
## 关联：BUG-004, BUG-005, BUG-008

## Rust 端
| 函数 | 重复位置 | 建议提取到 |
|------|----------|------------|
| same_path | annotations.rs / artifacts.rs / evidence_session.rs | pdf/mod.rs |
| temp_named_path | preview.rs / annotations.rs / split.rs / normalize.rs | pdf/mod.rs |
| fnv1a_hash | docx_template/mod.rs / pdf/evidence.rs | sort_utils.rs |

## 前端
| 函数 | 重复位置 | 建议 |
|------|----------|------|
| ensureExtension | TemplateView.vue / fieldRowUtils.js | 只保留 fieldRowUtils |
| splitPartyLabelSegments | TemplateView.vue / fieldRowUtils.js | 只保留 fieldRowUtils |
| effectiveFieldType | TemplateView.vue / TemplateRenderTab.vue | 统一逻辑后提取 |
| fieldFormKey | TemplateView.vue / TemplateRenderTab.vue | 只保留一处 |

## 变更日志
- 2026-08-07 — 🔴 未开始

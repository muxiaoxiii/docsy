# MDG-017: GLM 全项目审计问题修复

## 状态：🟢 P0+P1 已完成，P2 待后续 MDG
## 优先级：P0/P1/P2/P3
## 审计来源：GLM 5.2 全项目审阅（2026-08-08）+ Claude 背靠背验证
## 验证结果：所有发现确认正确，0 误报

---

## 一、问题清单（按修复顺序）

### P0 — 必须修复（3 个）

| ID | 模块 | 问题 | 文件:行号 | 工作量 |
|---|------|------|-----------|--------|
| GLM-T1 | template | XML 注入漏洞 | `ooxml.rs:190` | 0.5 天 |
| GLM-A1+X1 | evidence-pdf | 模块边界破裂（EvidencePdfWorkbench 放错模块） | `EvidencePdfView.vue:68` | 0.5 天 |
| GLM-A2 | 全局 | 错误处理全链路 String 化 | 50+ 处 `.map_err(\|e\| e.to_string())` | 2-3 天 |

### P1 — 高优先级（18 个）

| ID | 模块 | 问题 | 文件:行号 | 工作量 |
|---|------|------|-----------|--------|
| GLM-T3 | template | 裸 invoke 绕过 tauriBridge | `TemplateView.vue:176,1299` | 0.2 天 |
| GLM-U4 | UI | 未定义 CSS 变量 | `DocumentPreview.vue:88-89` | 0.1 天 |
| GLM-A3 | 架构 | App.vue 与 useAppStore settings 重复 | `App.vue:76-106` | 0.5 天 |
| GLM-X3 | settings | 读取类函数静默失败 | `SettingsView.vue:275-284` | 0.2 天 |
| GLM-X2 | settings | 无表单验证 | `SettingsView.vue:281-282` | 0.3 天 |
| GLM-P1 | pdf-tools | 路径函数重复 5 处 | `evidence.rs:1029`, `split.rs:206` 等 | 0.5 天 |
| GLM-P2 | pdf-tools | 旧 overlay 路径残留 | `evidence.rs:111,391,778-997` | 1 天 |
| GLM-U3 | UI | 硬编码颜色绕过变量体系 | `FilenameTokenInput.vue:273-349` | 0.5 天 |
| GLM-U2 | UI | 三种 spinner 并存 | 多处 | 0.5 天 |
| GLM-U1 | UI | 三派布局分裂 | `image-paddler:774`, `video-extract:476` | 2 天 |
| GLM-T2 | template | TemplateView.vue 2651 行超载 | `TemplateView.vue` | 2-3 天 |
| GLM-P3 | pdf-tools | EvidencePdfWorkbench.vue 3635 行超载 | `EvidencePdfWorkbench.vue` | 2-3 天 |
| GLM-M1 | image-paddler | 前端单文件 1076 行 | `ImagePaddlerView.vue` | 1-2 天 |
| GLM-M2 | image-paddler | 后端 17 参数函数 | `image_paddler.rs` | 0.5 天 |
| GLM-M3 | video-extract | 无进度反馈 | `extract.rs` | 1 天 |
| GLM-M4 | video-extract | 无取消机制 | `extract.rs` | 0.5 天 |
| GLM-A4 | 架构 | 取消机制双轨 | `lib.rs:21-131`, `operations.rs:64` | 2 天 |
| GLM-U5 | UI | video-extract 拖放双轨 | `VideoExtractView.vue:231,455` | 0.2 天 |

### P2 — 中优先级（4 个抽样）

| ID | 模块 | 问题 | 状态 |
|---|------|------|------|
| GLM-T4 | template | 10 处 `#[allow(dead_code)]` | 确认 |
| GLM-P4 | pdf-tools | AntiCopyMethod::TextOverlay 死代码 | 确认 |
| GLM-P5 | pdf-tools | anti-ocr 检测串行 | 确认 |
| GLM-U6 | UI | 死 CSS | 确认 |

---

## 二、实施计划（按修复顺序）

### Phase 1: P0 安全修复（1 天）
1. **GLM-T1**: XML 注入修复 — `BytesText::from_escaped` + 回归测试
2. **GLM-A1+X1**: 模块边界修复 — 移动 EvidencePdfWorkbench

### Phase 2: P1 快速修复（1.5 天）
3. **GLM-T3**: 裸 invoke → tauriCallSafe
4. **GLM-U4**: 定义缺失 CSS 变量
5. **GLM-X3**: settings 静默失败加 ElMessage.warning
6. **GLM-X2**: settings 表单验证
7. **GLM-U5**: video-extract 拖放统一
8. **GLM-P1**: 路径函数统一

### Phase 3: P1 中等修复（3 天）
9. **GLM-A3**: App.vue settings 统一
10. **GLM-P2**: 旧 overlay 路径清理
11. **GLM-U3**: 硬编码颜色迁移
12. **GLM-U2**: spinner 统一
13. **GLM-M2**: image-paddler 参数重构

### Phase 4: P1 大型重构（7-10 天）
14. **GLM-A2**: 错误类型结构化（DocsyError 枚举）
15. **GLM-T2**: TemplateView 拆分
16. **GLM-P3**: EvidencePdfWorkbench 拆分
17. **GLM-M1**: image-paddler 拆分
18. **GLM-U1**: 布局统一
19. **GLM-M3+M4**: video-extract 进度+取消
20. **GLM-A4**: 取消机制统一

### Phase 5: P2 清理（持续）
21-24: dead_code 清理、死代码删除、anti-ocr 并行化、死 CSS 清理

---

## 三、测试计划
- 每个 Phase 完成后 `cargo test` + `npm test`
- P0 修复后增加回归测试（XML 注入 + 模块导入验证）

## 四、回退方案
- 每个 Phase 独立 commit，失败可回退
- P0 修复优先，不影响其他 Phase

## 五、变更日志
- 2026-08-08 04:20 — 初始版本，基于 Claude 背靠背验证结果

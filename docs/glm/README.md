# Docsy 全项目代码审阅（GLM）

> 审阅日期：2026-08-08
> 分支：codex/template-quickxml-0.8（MDG 工作分支）
> 审阅范围：前端 7 模块 + 后端 11 模块，约 40000 行代码
> 审阅标准：与 MDG-016 页眉页脚审计相同

---

## 文档索引

| 文档 | 范围 | 核心发现 |
|------|------|---------|
| [GLM-architecture-review.md](GLM-architecture-review.md) | 全项目架构 | 7 个架构级问题（错误 String 化、模块边界破裂、取消机制双轨等） |
| [GLM-template-review.md](GLM-template-review.md) | template 模块 | P0 XML 注入漏洞、TemplateView.vue 2651 行超载 |
| [GLM-pdf-tools-review.md](GLM-pdf-tools-review.md) | pdf-tools（非页眉页脚） | 路径函数重复 3 处、旧 overlay 路径残留、anti-ocr 死代码 |
| [GLM-media-review.md](GLM-media-review.md) | image-paddler + video-extract | 文件选择器未统一、未用 ToolWorkspaceShell、无进度/取消 |
| [GLM-auxiliary-review.md](GLM-auxiliary-review.md) | settings + evidence-pdf + home | EvidencePdfWorkbench 放错模块、settings 无表单验证 |
| [GLM-ui-consistency-review.md](GLM-ui-consistency-review.md) | UI 一致性专项 | 三派布局分裂、三种 spinner、硬编码颜色 |

---

## 全项目问题汇总（按优先级）

### P0 — 必须修复

| # | 模块 | 问题 | 工作量 |
|---|------|------|--------|
| 1 | template | XML 注入漏洞（ooxml.rs:190 BytesText::new 不转义） | 0.5 天 |
| 2 | pdf-tools | 页眉页脚删除失败（MDG-016 已详述） | 1-2 天 |
| 3 | 全局 | 错误处理全链路 String 化，前端无法区分错误类型 | 3-5 天 |
| 4 | evidence-pdf | EvidencePdfWorkbench 放错模块（跨模块 import） | 0.5 天 |

### P1 — 高优先级

| # | 模块 | 问题 | 工作量 |
|---|------|------|--------|
| 5 | template | TemplateView.vue 2651 行超载 | 2-3 天 |
| 6 | pdf-tools | 路径函数重复 3 处 | 0.5 天 |
| 7 | pdf-tools | 旧 overlay 路径残留 | 1 天 |
| 8 | pdf-tools | EvidencePdfWorkbench.vue 3635 行超载 | 2-3 天 |
| 9 | image-paddler | 前端单文件 1076 行无拆分 | 1-2 天 |
| 10 | image-paddler | 后端 17 参数函数 | 0.5 天 |
| 11 | video-extract | 无进度反馈 | 1 天 |
| 12 | video-extract | 无取消机制 | 0.5 天 |
| 13 | settings | 无表单验证 | 0.3 天 |
| 14 | settings | 读取类函数静默失败 | 0.2 天 |
| 15 | 全局 | TemplateView 裸 invoke 绕过 tauriBridge | 0.2 天 |
| 16 | 全局 | App.vue 与 useAppStore settings 重复 | 0.5 天 |
| 17 | UI | 三派布局分裂 | 2 天 |
| 18 | UI | 三种 spinner 并存 | 0.5 天 |
| 19 | UI | 硬编码颜色绕过变量体系 | 0.5 天 |

### P2 — 中优先级

| # | 模块 | 问题 | 工作量 |
|---|------|------|--------|
| 20 | template | 7 处 #[allow(dead_code)] | 0.3 天 |
| 21 | template | checkbox 符号前后端不一致 | 0.2 天 |
| 22 | pdf-tools | anti-ocr 检测串行 | 1 天 |
| 23 | pdf-tools | TextOverlay 死代码 | 0.3 天 |
| 24 | pdf-tools | 超时前后端不一致 | 0.5 天 |
| 25 | image-paddler | 后端单文件 1587 行 | 1 天 |
| 26 | image-paddler | 颜色映射三处重复 | 0.3 天 |
| 27 | video-extract | drawtext 未指定 fontfile | 0.3 天 |
| 28 | video-extract | 双重拖放绑定 | 0.2 天 |
| 29 | settings | 版本号硬编码 | 0.2 天 |
| 30 | 全局 | 文件选择器未统一 | 0.5 天 |
| 31 | 全局 | 取消机制双轨（SubprocessRegistry + OperationManager） | 3-5 天 |
| 32 | UI | el-button size 不统一 | 0.3 天 |
| 33 | UI | el-card shadow 不统一 | 0.2 天 |
| 34 | UI | el-dialog width 单位混用 | 0.2 天 |
| 35 | UI | ImagePreviewGrid 冗余 | 0.1 天 |
| 36 | UI | video-extract 拖放双轨 | 0.2 天 |

### P3 — 低优先级

| # | 模块 | 问题 | 工作量 |
|---|------|------|--------|
| 37 | pdf-tools | overlay 批量 spawn | 1 天 |
| 38 | pdf-tools | 预览无缓存 | 0.5 天 |
| 39 | image-paddler | 无分页预览 | 0.5 天 |
| 40 | video-extract | 无打开输出目录按钮 | 0.1 天 |
| 41 | settings | 单文件偏大 | 1 天 |
| 42 | 全局 | 日志系统自造轮子 | 2-3 天 |
| 43 | 全局 | pdf/ 模块拆分 | 2-3 天 |
| 44 | UI | 间距不完全一致 | 0.2 天 |
| 45 | UI | template 不用 el-card | 0.3 天 |

---

## 统计

- **P0 问题**：4 个（约 5-8 天）
- **P1 问题**：15 个（约 15-20 天）
- **P2 问题**：17 个（约 8-10 天）
- **P3 问题**：9 个（约 6-8 天）
- **总计**：45 个问题，约 34-46 天

---

## 好的方面

项目经过 MDG-001 至 016 的重构，整体质量已有显著提升：

1. **模块注册机制**优秀（moduleRegistry.js 自动发现，零改 core）
2. **core 层**职责清晰无重复（7 个文件各司其职）
3. **外部工具封装**完善（ExternalTool trait + SHA256 校验 + 自动安装）
4. **Tauri 桥接**三档封装（tauriCall/tauriCallQuiet/tauriCallSafe）
5. **安全防护**到位（文件大小限制、TOCTOU 防护、敏感部件拒绝、路径安全）
6. **CSS 变量体系**基础扎实（20 个 --docsy-* 变量 + Element Plus 桥接）
7. **共享组件层**职责清晰（8 个组件覆盖主要复用场景）
8. **命令命名规范**良好（84 个命令分 6 组，动词_名词 规范）
9. **template_history** 建议机制设计精巧（四种互补建议 + SQLite 迁移）
10. **image-paddler** 测试覆盖良好（14 个测试）+ 防抖/防竞态设计

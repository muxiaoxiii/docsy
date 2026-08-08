# MDG-020: UI 动效与交互质感改进

## 状态：🟢 已完成
## 优先级：P1
## 来源：GLM-ui-motion-review.md + GLM-ui-consistency-review.md
## 依据：emilkowalski/skills emil-design-eng 审查清单 + apple-design 原则

---

## 一、修改目标

Docsy 的 CSS 基础质量不错（无 `transition: all`、无 `scale(0)`、无 `ease-in`），但缺少交互反馈层（按钮按下、悬停动效）和材质质感层（毛玻璃、光学排版）。本次变更在不改变布局结构的前提下，补全这些"看不见的细节"。

## 二、问题清单

### P1 — 交互反馈缺失

| ID | 问题 | 当前状态 | 修复方案 |
|----|------|---------|---------|
| UI-1 | 按钮全项目无 :active 按下反馈 | 0 处 | 全局 `.el-button:active { transform: scale(0.97) }` |
| UI-2 | macOS 应用无毛玻璃材质 | 0 处 backdrop-filter | 跨平台方案：macOS 用 backdrop-filter，Windows 用半透明实色 |
| UI-3 | video-extract spinner 不统一 | el-icon Loading 1 处 | 统一到按钮 :loading 或 DocletWorkingPet |

### P2 — 动画基础设施

| ID | 问题 | 当前状态 | 修复方案 |
|----|------|---------|---------|
| UI-4 | 无自定义缓动曲线 | 全用默认 ease | 全局 `--ease-out` / `--ease-in-out` 变量 |
| UI-5 | prefers-reduced-motion 覆盖不全 | 仅 DocletWorkingPet | styles.css 全局降级规则 |
| UI-6 | 无 @media (hover: hover) 保护 | 0 处 | 触摸设备安全的悬停规则 |
| UI-7 | 弹出层 transform-origin 未设置 | 默认 center | el-popper 设置 origin |

### P3 — 质感打磨

| ID | 问题 | 当前状态 | 修复方案 |
|----|------|---------|---------|
| UI-8 | 首页卡片无悬停动效 | 仅 shadow 变化 | translateY + shadow |
| UI-9 | ImagePreviewGrid 冗余组件 | 仍存在 | 删除 |

## 三、影响分析

### styles.css 变更（主要）
- 新增 CSS 变量：`--ease-out`、`--ease-in-out`、`--ease-drawer`
- 新增全局规则：`.el-button:active`、`@media (hover: hover)`、`@media (prefers-reduced-motion)`
- 新增材质规则：`[data-os="macos"]` 下的 backdrop-filter
- 新增弹出层规则：`.el-popper` transform-origin

### 组件变更（少量）
- App.vue：添加平台检测 `document.documentElement.dataset.os`
- VideoExtractView.vue：移除 el-icon Loading，改用 :loading
- HomeView.vue：卡片悬停动效
- 删除 ImagePreviewGrid.vue

### 不受影响
- 所有 composables / 后端代码
- 布局结构（ToolWorkspaceShell 已统一）
- Element Plus 组件的默认行为

## 四、测试计划
- [ ] `npm test` 82/82 passed
- [ ] `cargo test` 160/160 passed
- [ ] `npm run build` 成功
- [ ] 手动验证：按钮按下有缩放反馈
- [ ] 手动验证：macOS 上侧边栏有毛玻璃效果

## 五、回退方案
- 全部变更在 styles.css + 少量组件中，可 `git revert` 回退
- 不涉及逻辑变更，无数据风险

## 六、变更日志
- 2026-08-08 12:40 — 变更单创建

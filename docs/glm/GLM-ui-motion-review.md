# UI 动效与交互质感改进清单

> 审阅日期：2026-08-08
> 审阅依据：emilkowalski/skills 的 emil-design-eng 审查清单 + apple-design 原则
> 审阅范围：src/ 下全部 CSS 和 Vue 组件样式
> 输出目录：docs/glm/

---

## 1. 审查清单对照

用 emil-design-eng 的标准审查清单逐条检查 Docsy 全项目 CSS：

| 检查项 | Docsy 状态 | 评估 |
|--------|-----------|------|
| `transition: all` | ✅ 未使用 | 合格 |
| `scale(0)` 入场动画 | ✅ 未使用 | 合格 |
| UI 元素上用 `ease-in` | ✅ 未使用 | 合格 |
| 持续时间 > 300ms | ✅ 全部 0.15s | 合格 |
| 按钮无 `:active` 反馈 | ❌ **全项目无按钮 active 状态** | **P1 缺失** |
| 弹出层 `transform-origin: center` | ❌ 未设置 | P2 |
| 无 `@media (hover: hover)` 保护 | ❌ 完全没有 | P2 |
| 无自定义缓动曲线 | ⚠️ 全部用默认 `ease` | P2 |
| `prefers-reduced-motion` 覆盖不全 | ❌ 只有 DocletWorkingPet | P2 |
| 硬编码颜色 | ❌ 4 个文件共 6 处 | P1 |
| 无 `backdrop-filter` 材质 | ❌ macOS 应用无毛玻璃 | P1 |
| spinner 不统一 | ❌ 两种实现并存 | P1 |

**合格的方面**：没有犯 `transition: all`、`scale(0)`、`ease-in`、超长持续时间这些常见错误。说明基础 CSS 质量不错。

**缺失的方面**：缺少按钮按下反馈、毛玻璃材质、自定义缓动曲线、触摸设备保护、统一的 spinner。

---

## 2. 具体问题和修复建议

### P1-1：按钮完全没有 `:active` 按下反馈

**问题**：全项目搜索 `:active`，只有 4 处拖拽手柄（`range-drag-handle`、`table-drag-handle`、`reorder-image-handle`、`queue-drag-handle`），**没有任何一处按钮有 `:active` 状态**。

emil-design-eng 原则："按钮必须在 `:active` 上添加 `transform: scale(0.97)`，给即时反馈，让 UI 感觉真正在倾听用户。这适用于任何可按元素。"

apple-design 原则："在 pointer-down 时响应，而非 release 时。按钮按下的瞬间就高亮，等 click/touch-up 才反馈感觉死气沉沉。"

**修复**：在 `styles.css` 中添加全局规则：

```css
/* 全局按钮按下反馈 */
.el-button:active {
  transform: scale(0.97);
}

/* 可点击卡片 */
.el-card[role="button"]:active,
.home-card:active {
  transform: scale(0.98);
}

/* 添加过渡（仅 transform，不用 all） */
.el-button {
  transition: transform 120ms cubic-bezier(0.23, 1, 0.32, 1);
}
```

**工作量**：0.2 天

---

### P1-2：macOS 应用无毛玻璃材质（跨平台方案）

**问题**：全项目没有一处 `backdrop-filter`。Docsy 是 macOS + Windows 双平台应用，侧边栏、工具栏、对话框都是实色背景，缺少原生质感。

apple-design 原则："用 `backdrop-filter` 构建半透明的导航栏/工具栏/Sheet。材质重量编码层级：深色/重材质分隔结构区域；浅色材质突出交互元素。"

**跨平台约束**：`backdrop-filter` 在 Windows Chromium 上性能开销大——每帧需要采样并模糊背后内容，低端设备上会导致滚动卡顿和 CPU 占用升高。macOS 有系统级硬件加速优化，Windows 没有。因此**只在 macOS 上启用毛玻璃，Windows 用半透明实色降级**。

**方案**：通过 Tauri 后端检测平台，在前端根元素加 `data-os="mac"` 或 `data-os="win"` 属性，CSS 据此区分：

```js
// App.vue onMounted
import { platform } from '@tauri-apps/plugin-os'
document.documentElement.dataset.os = platform() // 'macos' | 'windows'
```

```css
/* ---- 基础层：半透明实色（全平台，零开销） ---- */
/* 这一层在 Windows 上就是最终效果 */
.sidebar,
.el-tabs--left > .el-tabs__header {
  background: rgba(241, 232, 220, 0.92);
}

.el-dialog,
.el-message-box,
.el-popper {
  background: rgba(255, 249, 240, 0.96);
}

/* ---- 增强层：毛玻璃（仅 macOS，backdrop-filter 硬件加速） ---- */
[data-os="macos"] .sidebar,
[data-os="macos"] .el-tabs--left > .el-tabs__header {
  background: rgba(241, 232, 220, 0.7);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
}

[data-os="macos"] .el-dialog,
[data-os="macos"] .el-message-box,
[data-os="macos"] .el-popper {
  background: rgba(255, 249, 240, 0.82);
  backdrop-filter: blur(24px) saturate(180%);
  -webkit-backdrop-filter: blur(24px) saturate(180%);
}

/* ---- 降级层：用户偏好减少透明度 ---- */
@media (prefers-reduced-transparency: reduce) {
  .sidebar,
  .el-tabs--left > .el-tabs__header,
  .el-dialog,
  .el-message-box,
  .el-popper {
    background: var(--docsy-surface-elevated);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}
```

**设计说明**：

| 平台 | 侧边栏 | 对话框 | 开销 |
|------|--------|--------|------|
| macOS | `rgba(0.7)` + `blur(20px)` | `rgba(0.82)` + `blur(24px)` | 低（硬件加速） |
| Windows | `rgba(0.92)` 实色 | `rgba(0.96)` 实色 | 零 |
| 减少透明度 | 实色变量 | 实色变量 | 零 |

Windows 上的 `rgba(0.92)` 比纯实色略带透明感（能看到一点点底层内容），但不做模糊，开销与实色几乎相同。macOS 上则享受原生毛玻璃质感。

**可选**：如果未来想在 Windows 上也加毛玻璃，可以限制只在小面积元素上使用（如弹出层 `el-popper`），避免大面积元素（侧边栏、整个对话框）导致滚动卡顿。

**工作量**：0.5 天

---

### P1-3：spinner 两种实现并存

**问题**：
- `DocletWorkingPet.vue` — sprite 帧动画（`animation: doclet-look-around 5.6s steps(1, end) infinite`），全局操作动画
- `EvidencePdfWorkbench.vue:3023` — CSS keyframes spinner（`animation: docsy-spin 0.8s linear infinite`），模块本地

两种 spinner 视觉风格完全不同，用户在不同模块看到不同的加载反馈。

emil-design-eng 原则："旋转更快的 spinner 让应用感觉加载更快，即使加载时间相同。"

**修复建议**：
- 保留 `DocletWorkingPet` 作为全局操作级动画（它有品牌特色）
- 模块内的 `docsy-spin` spinner 统一为 Element Plus 的 `el-icon` Loading 或直接复用 `DocletWorkingPet`
- 如果保留 `docsy-spin`，将 `0.8s` 改为 `0.6s`（更快旋转 = 感觉更快）

```css
/* 统一本地 spinner */
.processing-spinner {
  animation: docsy-spin 0.6s linear infinite;
}
```

**工作量**：0.3 天

---

### P1-4：硬编码颜色绕过变量体系

**问题**：4 个文件共 6 处硬编码十六进制色：

| 文件 | 行号 | 硬编码值 | 应使用 |
|------|------|---------|--------|
| PdfToolsView.vue | :1147 | `#f0fdf4` | `--docsy-success-soft`（需新增） |
| PdfToolsView.vue | :1148 | `#16a34a` | `--docsy-success` |
| PdfToolsView.vue | :1151 | `#fef2f2` | `--docsy-danger-soft`（需新增） |
| PdfToolsView.vue | :1152 | `#dc2626` | `--docsy-danger` |
| VideoExtractView.vue | :575 | `#67c23a` | `--docsy-success` |
| VideoExtractView.vue | :579 | `#e6a23c` | `--docsy-warning` |
| DocletWorkingPet.vue | :65 | `#7a8a9a` | `--docsy-text-muted` |

**修复**：在 `:root` 中新增 soft 变量，替换硬编码：

```css
:root {
  /* 新增 soft 变量 */
  --docsy-success-soft: #eaf3de;
  --docsy-danger-soft: #fcebeb;
  --docsy-warning-soft: #faeeda;
}
```

**工作量**：0.3 天

---

### P2-1：全部 transition 用默认 ease，缺少自定义缓动曲线

**问题**：全项目 10 处 `transition` 全部用默认 `ease` 或不指定缓动，没有使用自定义 `cubic-bezier`。

emil-design-eng 原则："内置 CSS 缓动太弱，缺乏让动画感觉有意图的冲击力。使用自定义缓动曲线。"

```css
/* 推荐的全局缓动变量 */
:root {
  --ease-out: cubic-bezier(0.23, 1, 0.32, 1);
  --ease-in-out: cubic-bezier(0.77, 0, 0.175, 1);
  --ease-drawer: cubic-bezier(0.32, 0.72, 0, 1);
}
```

**修复**：将所有 `transition: ... ease` 替换为 `transition: ... var(--ease-out)`。

| 当前 | 修复后 |
|------|--------|
| `transition: background 0.15s` | `transition: background 150ms var(--ease-out)` |
| `transition: opacity 0.15s ease` | `transition: opacity 150ms var(--ease-out)` |
| `transition: color 0.15s, border-color 0.15s` | `transition: color 150ms var(--ease-out), border-color 150ms var(--ease-out)` |

**工作量**：0.2 天

---

### P2-2：弹出层未设置 transform-origin

**问题**：`el-dialog`、`el-popper`、`el-dropdown` 等弹出层没有设置 `transform-origin`，默认 `center`。

emil-design-eng 原则："弹出层应从触发器缩放进入，而非从中心。默认的 `transform-origin: center` 对几乎每个弹出层都是错的。例外：模态框。"

Element Plus 的 popper 组件支持 `--el-popper-transform-origin`，可以利用。

**修复建议**：
```css
/* Popper 类弹出层 — 从触发器缩放 */
.el-popper,
.el-select__popper,
.el-dropdown__popper {
  transform-origin: var(--el-popper-transform-origin, center);
  transition: opacity 150ms var(--ease-out), transform 150ms var(--ease-out);
}

/* 模态框 — 保持居中（这是例外） */
.el-dialog {
  transform-origin: center;
  transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out);
}

/* 入场状态 */
.el-popper[aria-hidden="true"] {
  opacity: 0;
  transform: scale(0.95);
}
```

**工作量**：0.3 天

---

### P2-3：无 `@media (hover: hover)` 触摸设备保护

**问题**：全项目没有任何 `@media (hover: hover)` 保护。触摸设备上点击会触发 `:hover`，导致误报——按钮卡在悬停状态。

emil-design-eng 原则："触摸设备在点击时触发悬停，导致误报。在 `@media (hover: hover) and (pointer: fine)` 后设置悬停动画。"

**修复**：所有 `:hover` 样式应该包裹在媒体查询中。对于 Element Plus 的内置 hover，可以通过覆盖实现：

```css
/* 触摸设备安全悬停 */
@media (hover: hover) and (pointer: fine) {
  .el-button:hover {
    /* hover 样式 */
  }
  .header-group-item:hover {
    background: var(--docsy-surface-hover);
  }
  .home-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 12px 36px rgba(54, 45, 36, 0.12);
  }
}
```

**工作量**：0.5 天

---

### P2-4：`prefers-reduced-motion` 覆盖不全

**问题**：只有 `DocletWorkingPet.vue:135` 有 `prefers-reduced-motion`，其他所有动画/过渡都没有。

apple-design 原则："减弱动画不等于无反馈——它意味着更温和、非前庭的等效替代。保留 opacity 和颜色过渡，移除移动和位置动画。"

**修复**：在 `styles.css` 末尾添加全局降级：

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

**工作量**：0.1 天

---

### P2-5：缺少入场动画

**问题**：文件列表添加/删除、Tab 切换、检测结果出现时，元素直接出现/消失，没有任何过渡。

emil-design-eng 原则："元素无过渡地出现或消失会感觉坏掉了。" "当多个元素一起进入时，交错它们的出现。"

**修复建议**：

```css
/* 文件列表项入场 — stagger */
.queue-item {
  animation: queue-enter 200ms var(--ease-out) backwards;
}

.queue-item:nth-child(1) { animation-delay: 0ms; }
.queue-item:nth-child(2) { animation-delay: 30ms; }
.queue-item:nth-child(3) { animation-delay: 60ms; }
.queue-item:nth-child(4) { animation-delay: 90ms; }
.queue-item:nth-child(5) { animation-delay: 120ms; }
/* 超过 5 项不再 stagger */

@keyframes queue-enter {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 状态标签颜色过渡 */
.status-tag {
  transition: background-color 200ms var(--ease-out), color 200ms var(--ease-out);
}

/* Tab 内容切换过渡 */
.el-tab-pane {
  animation: tab-enter 180ms var(--ease-out);
}

@keyframes tab-enter {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

**注意**：Tab 切换动画仅在鼠标点击时添加，键盘快捷键切换不加动画（emil-design-eng 原则："永远不要给键盘发起的操作加动画"）。

**工作量**：0.5 天

---

### P3-1：排版缺少光学尺寸层次

**问题**：全项目字体大小统一用固定 px，大标题和正文之间缺少光学尺寸调优。

apple-design 原则："字距是特定于大小的——大文本需要负字距；小文本需要微正字距。行高与大小成反比。"

**修复建议**：

```css
:root {
  /* 大标题 — 负字距 + 紧行高 */
  --docsy-heading-ls: -0.02em;
  --docsy-heading-lh: 1.15;
  
  /* 正文 — 正常字距 + 松行高 */
  --docsy-body-ls: 0;
  --docsy-body-lh: 1.6;
}

h1, .text-xl {
  letter-spacing: var(--docsy-heading-ls);
  line-height: var(--docsy-heading-lh);
}

body, .text-base {
  letter-spacing: var(--docsy-body-ls);
  line-height: var(--docsy-body-lh);
}
```

**工作量**：0.2 天

---

### P3-2：首页卡片无悬停动效

**问题**：`HomeView.vue` 的功能卡片用 `el-card shadow="hover"`，但只有阴影变化，没有位移/缩放反馈。

**修复建议**：

```css
.home-card {
  transition: transform 200ms var(--ease-out), box-shadow 200ms var(--ease-out);
  cursor: pointer;
}

@media (hover: hover) and (pointer: fine) {
  .home-card:hover {
    transform: translateY(-3px);
    box-shadow: 0 12px 36px rgba(54, 45, 36, 0.12);
  }
}

.home-card:active {
  transform: scale(0.98);
}
```

**工作量**：0.1 天

---

## 3. 汇总

| 优先级 | 问题 | 修复 | 工作量 |
|--------|------|------|--------|
| **P1** | 按钮无 `:active` 反馈 | 全局 `.el-button:active { transform: scale(0.97) }` | 0.2 天 |
| **P1** | macOS 应用无毛玻璃材质 | `backdrop-filter` 用于侧边栏/对话框/弹出层 | 0.5 天 |
| **P1** | spinner 两种实现并存 | 统一或加速旋转 | 0.3 天 |
| **P1** | 硬编码颜色 6 处 | 新增 soft 变量 + 替换 | 0.3 天 |
| **P2** | 无自定义缓动曲线 | 全局 `--ease-out` 变量 + 替换 | 0.2 天 |
| **P2** | 弹出层 transform-origin 错误 | 设置 popper origin | 0.3 天 |
| **P2** | 无触摸设备 hover 保护 | `@media (hover: hover)` | 0.5 天 |
| **P2** | prefers-reduced-motion 覆盖不全 | 全局降级规则 | 0.1 天 |
| **P2** | 缺少入场动画 | stagger + tab 过渡 | 0.5 天 |
| **P3** | 排版无光学尺寸 | 字距/行高变量 | 0.2 天 |
| **P3** | 首页卡片无悬停动效 | translateY + shadow | 0.1 天 |

**总计**：11 项，约 3.2 天

---

## 4. 合格的方面（无需修改）

- ✅ 没有 `transition: all`
- ✅ 没有 `scale(0)` 入场动画
- ✅ 没有 `ease-in` 用于 UI 动画
- ✅ 所有过渡持续时间 0.15s（< 300ms）
- ✅ CSS 变量体系基础扎实（20 个 `--docsy-*` 变量 + Element Plus 桥接）
- ✅ `DocletWorkingPet` 有 `prefers-reduced-motion` 降级
- ✅ spinner 用 `linear` 缓动（恒定运动正确使用 linear）
- ✅ 字体栈使用系统字体（`-apple-system` 优先）

基础 CSS 质量不错，没有犯常见低级错误。主要缺失的是**交互反馈层**（按钮按下、悬停动效、入场动画）和**材质质感层**（毛玻璃、光学排版），这些是把 UI 从"能用"提升到"好用"的关键。

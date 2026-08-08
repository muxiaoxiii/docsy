# MDG-021: MDG-020 验证问题修复

## 状态：📋 待实施
## 优先级：P2
## 来源：MDG-020 代码审查发现的 3 个问题

---

## 一、问题清单

### Bug-1: `.home-card` 全局与 scoped 样式冲突

**问题：** styles.css（global）定义了 `.home-card:hover` 的 `translateY(-3px)` + `box-shadow`，但 HomeView.vue（scoped）的 `.home-card:hover` 特异性更高（`0-3-0` vs `0-2-0`），全局的 box-shadow 被覆盖，用户看不到阴影效果。

**修复：**
- 删除 styles.css:301-309（`.home-card` + `.home-card:hover`，在 `@media (hover: hover)` 块内）
- 删除 styles.css:320-322（`.home-card:active`，在 `@media (hover: hover)` 块外）
- 在 HomeView.vue 的 scoped `.home-card:hover` 中加入 `box-shadow` + `translateY(-3px)`
- 在 HomeView.vue 新增 `.home-card:active` scoped 样式

### Bug-2: 两个 CSS 变量定义了但 0 处引用

**问题：** `--ease-in-out`（:103）和 `--ease-drawer`（:104）定义了但从未使用。项目无 drawer 组件。

**修复：** 删除 styles.css:103-104 两行。

### Bug-3: `navigator.platform` 缺 fallback

**问题：** `navigator.platform` 已弃用，Tauri 桌面当前可用但未来可能移除。

**修复：** 加 `|| navigator.userAgent` fallback。

## 二、影响分析

- `.home-card` 仅 HomeView.vue 使用，删除全局样式无副作用
- `--ease-in-out` / `--ease-drawer` 0 处引用，删除无副作用
- `navigator.platform` fallback 加在已有行，无行为变化

## 三、验证

- [ ] npm test 82/82
- [ ] cargo test 160/160
- [ ] Vite build 成功

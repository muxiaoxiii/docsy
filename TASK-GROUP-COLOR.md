在模板填写页面（TemplateRenderTab.vue）的字段卡片中，给同名字段组加分组颜色。

## 需求
同一字段名的多个位置（fillAllPositions）应该有相同的颜色标记，不同字段名组用不同颜色。
用左侧彩色边条（border-left）实现，颜色循环使用一组预设色。

## 实现

### 1. TemplateView.vue — 给 renderableTemplateFields 加 colorGroupIndex

在 renderableTemplateFields computed 中，标记完 _isDuplicate 后，给每个字段分配 colorGroupIndex：

```js
// Assign color group index — same name gets same color
const colorMap = new Map()
let colorIdx = 0
for (const field of fields) {
  if (!colorMap.has(field.name)) {
    colorMap.set(field.name, colorIdx++)
  }
  field._colorGroupIndex = colorMap.get(field.name)
}
```

### 2. TemplateRenderTab.vue — 用 colorGroupIndex 设置左边条颜色

在 fill-field-card 的 class 或 style 中：

```html
<div class="fill-field-card"
  :class="{ 'duplicate-field': field._isDuplicate, ... }"
  :style="{ borderLeftColor: GROUP_COLORS[field._colorGroupIndex % GROUP_COLORS.length] }"
>
```

颜色数组（6-8 个低饱和度色）：
```js
const GROUP_COLORS = ['#67c23a', '#409eff', '#e6a23c', '#f56c6c', '#909399', '#b37feb', '#36cfc9', '#ff85c0']
```

### 3. CSS — fill-field-card 加 border-left 样式

```css
.fill-field-card {
  border-left: 3px solid transparent;
}
```

border-left-color 通过 inline style 覆盖。

## 验证
npm test + npm run build 全过。

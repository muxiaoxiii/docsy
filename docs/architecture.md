# Docsy v0.5 架构文档

> 版本：0.5.1 | 更新：2026-06-12

## 设计原则

1. **模块自包含**：每个功能模块在 `src/modules/<name>/` 下自包含 index.js、views、components、composables
2. **统一注册**：模块通过 `index.js` 声明路由、菜单、首页卡片、设置项，注册中心自动收集
3. **分层架构**：后端 `commands/`（薄壳）→ `services/`（业务逻辑）→ 底层模块（docx/pdf/ffmpeg/external）
4. **共享基础设施**：字典、外部工具、日志、设置全局共享，不各模块自建
5. **Rust 优先**：核心逻辑用 Rust 实现，减少外部依赖，控制包体积

## 模块注册系统

### 前端模块注册

每个模块目录下的 `index.js` 导出标准接口：

```javascript
export default {
  id: 'doc-gen',
  name: '文档生成',
  icon: 'Document',
  description: '基于模板生成 Word/PDF 文档',
  category: 'document',
  defaultVisible: true,
  routes: [/* vue-router 路由定义 */],
  menuItems: [/* 侧边栏菜单项 */],
  homeCards: [/* 首页快捷卡片 */],
  settings: null, // 模块级设置 schema
};
```

注册中心 (`core/moduleRegistry.js`) 通过 `import.meta.glob` 自动收集所有模块。

### 后端模块注册

```rust
pub struct ModuleDescriptor {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub category: ModuleCategory,
    pub default_visible: bool,
    pub settings_schema: Option<serde_json::Value>,
    pub sub_modules: Vec<SubModuleDescriptor>,
}
```

## 字典系统

三层叠加架构：

```
global_dictionaries (共享字典，跨模板)
  ↓ 叠加
template_dictionaries (模板级覆盖)
  ↓ 叠加
field_history (动态推荐，用过即记)
```

SQLite 表：
- `global_dictionaries` — courts, causes, firms, lawyers, stages, parties
- `template_dictionaries` — 模板私有覆盖
- `field_history` — 每次生成后新值自动入库
- `parties` — 当事人主档（跨模板共享）

## 文档生成流程

```
选择模板 → 加载 fields.json → 渲染表单 → 用户填写
  ↓
实时预览（mammoth HTML）
  ↓
invoke("generate_document") → Rust 加载 template.docx → 占位符替换 → 输出 .docx
  ↓
可选：docx → PDF（Word/WPS / LibreOffice / 仅docx）
  ↓
写入 generation_records + 更新 field_history
```

## 外部工具抽象

所有外部 CLI 工具通过 `ExternalTool` trait 统一管理：

```rust
pub trait ExternalTool: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> ToolStatus;
    fn try_install(&self) -> Result<String>;
    fn binary_path(&self) -> Result<PathBuf>;
}
```

已实现：`QpdfTool`, `FfmpegTool`, `LibreOfficeTool`

## 占位符语法

| 语法 | 行为 |
|------|------|
| `{{key}}` | 简单文本替换 |
| `{{?key:text}}` | 条件前缀：值非空输出 text |
| `{{*key}}` | 行重复：表格行按列表长度克隆 |
| `{{#row}}` | 行号自动编号 |

## .docsytpl 模板包格式

```
my_template.docsytpl (zip)
├── manifest.json        # id, name, type, version
├── template.docx        # 含占位符的 Word 文件
├── fields.json          # 字段定义
├── dictionaries.json    # 模板字典（可选）
└── builder_state.json   # 编辑器状态（可选）
```

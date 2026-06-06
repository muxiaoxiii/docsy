# Docsy 日志与问题定位规范

更新时间：2026-06-06

本文档用于约束 Docsy 的日志系统。目标是让“加载失败、保存失败、生成失败、外部工具失败、页面异常”都能被定位到具体模块、命令、上下文和错误原因。

## 1. 目标

Docsy 是本地单机软件，问题定位不能依赖服务端监控。因此必须保证本地日志足够完整：

1. 用户看到错误提示时，日志里能找到同一时间的失败记录。
2. 前端调用、后端命令、文件处理、外部工具调用必须能串成时间线。
3. 日志不能记录完整文书正文、完整 docx/base64、完整 HTML/XML。
4. 日志系统本身失败不能影响主流程。

## 2. 文件位置

日志目录位于 Docsy 用户数据目录：

```text
macOS:
~/Library/Application Support/Docsy/logs/

Windows:
%APPDATA%\Docsy\logs\
```

当前日志文件按日期命名：

```text
docsy-YYYYMMDD.log
```

设置页的“问题定位”区提供：

- 当前日志文件路径。
- 日志目录路径。
- 打开日志文件。
- 打开日志目录。
- 基础诊断摘要：系统、架构、是否调试构建。

## 3. 日志格式

日志使用 JSON Lines，一行一条记录：

```json
{"ts":"2026-06-05T10:12:30+08:00","level":"info","target":"template.save","message":"save_user_template.start","context":{"id":"letter","fieldsCount":12,"marksCount":9}}
```

字段含义：

| 字段 | 含义 |
|---|---|
| `ts` | 本地时间，RFC3339 |
| `level` | `debug` / `info` / `warn` / `error` |
| `target` | 模块或功能域 |
| `message` | 具体事件名 |
| `context` | 结构化上下文 |

## 4. 日志等级

| level | 使用场景 |
|---|---|
| `debug` | 高频或细节信息，例如 invoke 成功耗时、docx 文本提取长度 |
| `info` | 用户动作、命令开始、命令成功、应用启动 |
| `warn` | 自动降级、非阻塞失败、可恢复异常 |
| `error` | 用户操作失败、后端命令失败、未捕获异常、panic |

## 5. 已覆盖链路

### 5.1 应用生命周期

- 后端启动：`app.lifecycle/backend.start`
- 前端启动：`app.lifecycle/frontend.start`
- 后端 panic：`app.panic/backend.panic`
- Vue 全局错误：`frontend.vue/vue.error`
- 浏览器脚本错误：`frontend.window/window.error`
- 未处理 Promise：`frontend.promise/unhandled_rejection`

### 5.2 主应用状态

- 菜单/模板列表/设置刷新失败。
- 主导航切换。
- 用户模板重命名、删除。

### 5.3 模板编辑器

- `read_file_bytes`
- `extract_docx_text`
- `extract_docx_text_from_base64`
- `read_template_for_edit`
- `edit_docx_text_range`
- `save_user_template`
- 预览刷新失败。

后端额外记录：

- 模板 id。
- `marksCount`。
- `fieldsCount`。
- 是否存在 `builder_state`。
- docx/base64 长度。
- 固有字替换的 `start/end` 和替换文字长度。
- 失败错误字符串。

### 5.4 文档生成

- `generate_letter.start`
- `generate_letter.success`
- `generate_letter.failed`

记录模板 id、字段数量、字段选项数量、输出路径、输出字节数或错误。

### 5.5 图片排版

- 文件夹分析开始、成功、失败。
- 批量生成开始、成功、失败。

记录输入目录、输出目录、分组模式、输出数量、跳过数量或错误。

### 5.6 视频抽帧

- 视频探测开始、成功、失败。
- 抽帧开始、成功、失败。

记录视频路径、时长、尺寸、fps、编码、输出目录、抽帧数量或错误。

### 5.7 PDF 工具

- PDF 解锁开始、成功、失败。

## 6. 隐私与安全边界

允许记录：

- 命令名。
- 模板 id。
- 文件路径。
- 数量、长度、偏移。
- 错误字符串。
- 输出文件路径。
- 外部工具状态。

禁止记录：

- 完整 docx/base64。
- 完整 HTML/XML。
- 完整用户文书正文。
- 批量生成的完整字段值。
- 身份证号、手机号、银行卡号等敏感字段的原值。

前端 `appLogger` 会自动对 key 中包含 `base64`、`docx`、`bytes`、`content`、`html`、`xml`、`text` 的长字符串做长度摘要处理。

## 7. 排障流程

出现 bug 后按这个顺序定位：

1. 记下用户操作的大致时间。
2. 打开设置页 -> 问题定位 -> 打开日志文件。
3. 搜索 `error`，找到最近的失败记录。
4. 如果是模板问题，继续搜索同一 `templateId` 或同一命令名，例如 `save_user_template`。
5. 沿时间向前看对应 `.start` 记录，确认输入规模、模板 id、marks 数量、fields 数量。
6. 如果前端日志有 `.failed`，但后端没有对应 `.start`，说明命令没有到达后端，优先查前端参数、invoke 名称、序列化问题。
7. 如果后端有 `.start` 和 `.failed`，优先查后端错误和文件状态。
8. 如果只有 `frontend.promise/unhandled_rejection` 或 `frontend.vue/vue.error`，优先查组件生命周期、空值、异步状态竞争。

## 8. 新功能埋点规则

任何新增 Tauri 命令必须至少记录：

```text
<target>.<command>.start
<target>.<command>.success
<target>.<command>.failed
```

最低上下文要求：

- 输入文件/输出文件路径。
- 业务 id，例如 `templateId`、`recordId`。
- 数量和长度，不记录完整内容。
- 失败时记录 `error`。

前端新增复杂流程时必须记录：

- 用户动作入口。
- 后端调用失败。
- 静默降级。
- 不阻断流程但会影响结果的异常。

## 9. 保留策略

当前自动保留最近 14 天日志。后续如果增加“导出诊断包”，应只导出最近 3 天日志和诊断摘要。

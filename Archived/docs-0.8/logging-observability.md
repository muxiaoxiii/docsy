# Docsy 日志与问题定位

更新时间：2026-06-23

## 目标

Docsy 是本地单机工具，日志用于定位外部工具、文件处理、页面异常和后端命令失败。

日志不能记录完整文档正文、完整 base64、完整 HTML/XML 或敏感字段原值。

## 文件位置

```text
macOS:   ~/Library/Application Support/Docsy/logs/
Windows: %APPDATA%\Docsy\logs\
```

当前日志文件按日期命名：

```text
docsy-YYYYMMDD.log
```

设置页提供打开日志目录、打开当前日志和基础诊断信息。

## 日志格式

日志使用 JSON Lines：

```json
{"ts":"2026-06-23T10:12:30+08:00","level":"info","target":"pdf","message":"unlock_pdf.start","context":{"input":"/path/a.pdf"}}
```

## 当前覆盖链路

- 应用启动、后端 panic、前端全局错误
- PDF 解锁、证据整理、页眉页脚叠加
- 图片排版文件夹分析和批量生成
- 视频探测和抽帧
- 外部工具检测
- 设置读取、保存和诊断入口

## 排障流程

1. 记下用户操作时间。
2. 在设置页打开当前日志。
3. 搜索 `error`，找到最近失败记录。
4. 按 `target` 和 `message` 向前追同一流程的 `.start` 记录。
5. 如果前端有失败但后端没有 `.start`，优先查 invoke 名称、参数序列化和组件状态。
6. 如果后端有 `.start` 和 `.failed`，优先查后端错误、文件状态和外部工具状态。

## 新功能埋点规则

新增 Tauri 命令至少记录：

```text
<target>.<command>.start
<target>.<command>.success
<target>.<command>.failed
```

上下文只记录路径、数量、开关、耗时、错误字符串等定位信息，不记录完整内容。

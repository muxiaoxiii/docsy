# Docsy 1.0.6 发布检查记录

日期：2026-09-17。本记录对应 1.0.6 发布源码（基于 origin 1.0.5 重做后的审查修复）。

## 审查结论摘要

完整审查见 `REVIEW_1.0.6.md`。发布前已处理：

1. **P0 页尺寸契约**：`--json-key=pages` 在真实 qpdf 上不含 CropBox/MediaBox；已改为 pages + objects（`--json-stream-data=none`）并解析 `/Parent` 继承，补充契约测试。
2. **P1 版本面脱节**：`index.html` / `README.md` / 站点 `changelog.json` 对齐 1.0.6；`check-release-version` 覆盖上述路径。
3. **P1 keep-insert 丢失**：从备份线移植「原件保留 vs 新插入」冲突确认（检测、suppress/overlap/delete、弹窗、测试）。

## 自动化验证（本轮）

- 后端 `cargo test --lib pdf::`：**177 passed**（含页尺寸 modern objects / Parent 继承 / 真实 pages 无框契约）。
- 前端 `npm test`：**34 套件 / 259 passed**（含 5 项 keep-insert）。
- `node scripts/check-release-version.mjs`：通过。

## 仍需人工/真机验收

- 从当前源码重新打包，在桌面端完成：大文件 A4/压缩/批注分段、拆分普通文本页眉清除、签章确认、keep-insert 冲突弹窗。
- macOS 与 Windows 分别验证；本机结果不能代表 Windows x64/x86。
- 发布后打开官网核对静态版本文案与下载链接是否为 1.0.6（CI 会重生成 releases/changelog 数据）。
- 700+ 页扫描件端到端仍未自动化覆盖。

## 保留说明

- 备份分支 `backup/pre-rebase-1.0.6-20260917-184734` 在验收完成前保留。
- main.js 下载回退数据在真实 Release 发布前仍指向最近已发布版本，由部署脚本在发布后同步。

结论：审查阻断项已修复，自动化基线通过；大文件与桌面端真机验收完成前，不宣称全功能已验收。

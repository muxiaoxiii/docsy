# Docsy v0.9.6 综合审阅报告

> 审阅日期：2026-08-07
> 项目：`/Users/only/Documents/PythonProgram/Docsy`
> 技术栈：Tauri 2 + Rust + Vue 3 + Element Plus
> 代码规模：前端 ~20,400 行 + Rust 后端 ~20,600 行

---

## 审阅文档清单

| 文档 | 行数 | 内容 | 深度 |
|------|------|------|------|
| **deep-review-template-core.md** | 878 | template 模块 + core 层逐文件分析（41 文件） | ⭐⭐⭐ 逐函数级 |
| **deep-review-pdf.md** | 1,037 | pdf-tools + evidence-pdf 逐文件分析（38 文件） | ⭐⭐⭐ 逐函数级 |
| **deep-review-other.md** | 726 | video/image/settings/shared + Rust 外部工具（38 文件） | ⭐⭐⭐ 逐函数级 |
| **codebase-audit.md** | 641 | 全模块概述 + 架构图 + 依赖关系 | ⭐⭐ 模块级 |
| **bug-report.md** | 430 | Bug 清单（P0×3, P1×7, P2×8, P3×11） | ⭐⭐ 分类汇总 |
| **ui-review.md** | 720 | UI/UX 逐组件审查 + 改进路线图 | ⭐⭐⭐ 逐组件级 |
| **design-vs-implementation.md** | 303 | 设计文档 vs 代码差异（47 项） | ⭐⭐ 逐文档级 |
| **review-summary.md** | 本文件 | 综合报告（你正在看的） | ⭐ 摘要 |

---

## 关键数据

### Bug 汇总（深度审阅新增发现合并）

| 严重 | 数量 | 代表问题 |
|------|------|----------|
| 🔴 P0 Critical | **6** | `cancel_operation` 杀所有进程；`Mutex::unwrap()` panic；忙等待轮询；书签页码偏移错位；书签写入非原子可能丢文件；输出路径碰撞覆盖数据 |
| 🟠 P1 High | **12** | `same_path` 3处重复定义；`eprintln!` 生产代码；`TextOverlay` 方法命名误导；`margin_mm` 前后端默认值不一致；`managed.rs` macOS/Linux sha256 永远为空 |
| 🟡 P2 Medium | **19** | 正则解析 XML 脆弱性；篡改映射空间仅4096值可逆向；`temp_named_path` 7处重复；`fnv1a_hash` 重复定义 |
| 🟢 P3 Low | **17** | 大文件（EvidencePdfWorkbench 3634行）；死代码；魔法数字 |
| ✅ 安全验证 | **8** | ZIP bomb 防护✓ SQL注入防护✓ 路径遍历防护✓ 命令注入防护✓ HTTPS下载✓ SHA256校验✓ Zip Slip防护✓ URL白名单✓ |

### 设计文档差异

| 类别 | 数量 |
|------|------|
| 设计有但代码未实现 | **12**（含 OCR 模块整个是空的） |
| 代码有但设计未记录 | **8**（含 anti-copy 452行模块） |
| 架构差异 | **7**（命令数 58→75+） |
| 文档过时 | **14** |
| 决策变更未更新 | **6** |

### UI/UX 核心问题

- **2 个大组件**（已拆分，当前结构合理）：EvidencePdfWorkbench (3,634行，4子组件+5composables) / TemplateView (2,900行，4Tab组件+4composables)，剩余行数为协调层，无需进一步拆分
- **WCAG 对比度不通过**：`--docsy-text-muted` #7a817d
- **无暗色模式**、无全局搜索、无自动保存
- **loading/error 状态**有 4+ 种不同实现方式

---

## 建议优先级

### 立即修复（P0）
1. `cancel_operation` 支持按 ID 取消
2. `Mutex::unwrap()` → 安全模式
3. 忙等待 → `Condvar`/`Notify`
4. 书签写入改为原子操作（先备份）
5. 输出路径碰撞检查
6. 书签页码偏移修复

### 本周（P1）
1. 统一 `same_path` 为公共函数
2. 清理 `eprintln!`
3. 修正 `TextOverlay` 命名
4. 前后端默认值对齐（`margin_mm` 等）
5. 修复 `managed.rs` macOS/Linux sha256
6. 拆分 EvidencePdfWorkbench 和 TemplateView

### 本月（P2）
1. 更新设计文档至 v0.9.6
2. 补写 anti-copy、bookmarks 文档
3. 统一 loading/error 状态模式
4. WCAG 对比度修复
5. 消除 `temp_named_path` / `fnv1a_hash` 重复

### 下季度（P3）
1. 暗色模式
2. 全局搜索 + 键盘快捷键
3. 自动保存
4. 完善公共规则库

---

*审阅方式：Hermes Agent 调度 Claude Code，分 3 路并行逐文件审阅（共 117 个文件），结合 delegate_task 子 agent 产出的概述文档交叉验证。*

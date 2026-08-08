# MDG-018 Phase 3: 设计文档

## 一、硬编码颜色迁移（37 处）

### ① 定位
| 文件 | 行号 | 硬编码值 | 用途 |
|------|------|----------|------|
| EvidencePdfWorkbench.vue | 1761-1763 | `#000000` | header/footer/pageNumber 默认颜色（JS 配置值，保留） |
| EvidencePdfWorkbench.vue | 3089 | `#e5bd92` | 警告边框色 |
| EvidencePdfWorkbench.vue | 3094,3493,3541 | `#b42318` | 错误/危险文字色 |
| EvidencePdfWorkbench.vue | 3469 | `#111827` | 主文字色 |
| EvidencePdfWorkbench.vue | 3525 | `#1d4ed8` | 链接/主色 |
| TemplateBuildTab.vue | 1338 | `#67c23a` | 成功色 |
| TemplateBuildTab.vue | 1368 | `#c0c4cc` | 禁用色 |
| TemplateBuildTab.vue | 1388 | `#c45656` | 错误色 |
| TemplateBuildTab.vue | 1434,1442,1450 | `#67c23a/#8e5cf7/#e6a23c` | 边框色（绿/紫/琥珀） |
| DocletWorkingPet.vue | 36,64 | `#36526f/#7a8a9a` | 宠物组件文字色 |
| ImagePaddlerView.vue | 945 | `#fff` | 白色边框 |
| styles.css | 83,134 | `#f1ede7/#f8efe5` | CSS 变量定义（保留） |

### ② 影响分析
- styles.css 的 2 处是 CSS 变量定义，不改
- EvidencePdfWorkbench.vue 的 `#000000` 是 JS 配置默认值（传给后端），不改
- 其余 CSS 中的硬编码颜色替换为 Element Plus 变量或自定义 CSS 变量

### ③ 替换规则
| 硬编码值 | 替换为 | 语义 |
|----------|--------|------|
| `#b42318` | `var(--el-color-danger)` | 危险/错误 |
| `#111827` | `var(--el-text-color-primary)` | 主文字 |
| `#1d4ed8` | `var(--el-color-primary)` | 主色/链接 |
| `#67c23a` | `var(--el-color-success)` | 成功 |
| `#c0c4cc` | `var(--el-text-color-disabled)` | 禁用 |
| `#c45656` | `var(--el-color-danger)` | 错误 |
| `#8e5cf7` | `var(--el-color-primary-light-3)` | 紫色辅助 |
| `#e6a23c` | `var(--el-color-warning)` | 警告/琥珀 |
| `#e5bd92` | `var(--el-color-warning-light-3)` | 警告边框 |
| `#36526f` | `var(--el-text-color-regular)` | 常规文字 |
| `#7a8a9a` | `var(--el-text-color-secondary)` | 次要文字 |
| `#fff` | `var(--el-bg-color)` 或 `white` | 白色 |

### ④ 验证
- npm test 通过
- 视觉检查：确认替换后颜色看起来一致（Element Plus 变量值可能略有不同）

---

## 二、裸 invoke 修复

### ① 定位
- 文件：`src/services/appLogger.js:49`
- 代码：`await invoke('write_frontend_log', { ... })`
- 问题：绕过 tauriBridge，无错误处理

### ② 影响分析
- `appLogger.js` 只有这一个 invoke 调用
- `write_frontend_log` 是 system.rs 中的命令，不需要返回值
- 改用 `tauriCallSafe` 后，错误会被捕获但不影响功能（日志写入失败静默处理）

### ③ 修改
```js
// 修改前
await invoke('write_frontend_log', { level, target, message, context })
// 修改后
await tauriCallSafe('write_frontend_log', { level, target, message, context })
```

---

## 三、GLM-M3+M4: video-extract 进度 + 取消

### ① 定位
- 文件：`src-tauri/src/ffmpeg/extract.rs` — `extract()` 函数（第 8 行）
- 当前：`_token` 参数完全未使用，`command_output_with_idle_timeout` 阻塞等到完成

### ② 架构分析
FFmpeg 抽帧进度信息在 stderr 中，格式为：
```
frame=  120 fps= 60 q=2.0 size=   1024kB time=00:00:04.00 bitrate= 2097.2kbits/s speed=2.0x
```

已有基础设施：
- `crate::external::command_output_with_idle_timeout` — 等待命令完成
- `OperationManager` — 管理操作生命周期
- Tauri `app.emit()` — 前端事件通知

**没有** `progress.rs` 文件。需要自行实现 FFmpeg 进度解析。

### ③ 设计方案

**方案：替换阻塞等待为逐行读取 + 进度发射**

```rust
pub fn extract(args: &serde_json::Value, token: &CancellationToken) -> Result<serde_json::Value> {
    // ... 前置逻辑不变 ...
    
    cmd.stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());
    
    let mut child = cmd.spawn()?;
    let stderr = child.stderr.take().unwrap();
    
    // 逐行读取 stderr，解析进度
    let reader = std::io::BufReader::new(stderr);
    let mut last_progress = 0u64;
    
    // 在单独线程读取 stderr
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = tx.send(line);
            }
        }
    });
    
    // 主循环：检查进度 + 取消
    loop {
        if token.is_cancelled() {
            child.kill()?;
            anyhow::bail!("操作已取消");
        }
        
        match rx.try_recv() {
            Ok(line) => {
                if let Some(time) = parse_ffmpeg_time(&line) {
                    let progress_ms = (time * 1000.0) as u64;
                    if progress_ms > last_progress {
                        last_progress = progress_ms;
                        // 可选：发射 Tauri 事件给前端
                        // app.emit("video-extract-progress", progress)?;
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }
    }
    
    let status = child.wait()?;
    // ... 后续逻辑不变 ...
}

fn parse_ffmpeg_time(line: &str) -> Option<f64> {
    // 解析 "time=00:00:04.00" 格式
    let time_str = line.split("time=").nth(1)?;
    let time_part = time_str.split_whitespace().next()?;
    parse_time_text(time_part) // 复用已有函数
}
```

**取消机制（GLM-M4）：**
- `token.is_cancelled()` 在进度循环中检查
- 取消时 `child.kill()` 终止 FFmpeg 进程
- 返回错误（或部分结果）

### ④ 影响分析
- `extract()` 签名不变（已有 `token` 参数）
- `commands/video.rs` 中 `extract_frames` 通过 `run_managed` 调用，已传入 token
- 前端无需改动（进度事件是增量功能）

### ⑤ 验证
- cargo test 通过
- 手动测试：大视频文件抽帧，验证进度事件发射
- 手动测试：抽帧过程中取消，验证进程终止

---

## 四、GLM-M1: ImagePaddlerView 拆分

### ① 定位
- 文件：`src/modules/image-paddler/views/ImagePaddlerView.vue`（1060 行）
- 结构：265-480 行是状态声明和逻辑函数，480-1060 行是模板和样式

### ② 拆分方案

提取 `useImagePaddlerState.js` composable：

```js
// src/modules/image-paddler/composables/useImagePaddlerState.js
export function useImagePaddlerState() {
  // 状态声明（265-350）
  const folder = ref('')
  const folders = ref([])
  const analyzing = ref(false)
  const generating = ref(false)
  const analysis = ref(null)
  const generatedResult = ref(null)
  const settings = reactive({ ... })
  
  // 计算属性（315-375）
  const layoutGrid = computed(...)
  const orderedImages = computed(...)
  // ...
  
  // 方法（377-480）
  async function selectFolder() { ... }
  async function analyze() { ... }
  async function run() { ... }
  function reorderLayoutImages({ from, to }) { ... }
  
  return { folder, folders, analyzing, ..., selectFolder, analyze, run }
}
```

ImagePaddlerView.vue 变为薄壳：
```vue
<script setup>
import { useImagePaddlerState } from '../composables/useImagePaddlerState.js'
const state = useImagePaddlerState()
</script>
<template>
  <!-- 使用 state.xxx -->
</template>
<style>
/* 保留样式 */
</style>
```

### ③ 影响分析
- `useImagePaddlerState` 是新文件，无已有调用方
- ImagePaddlerView.vue 的模板需要把所有变量/函数改为 `state.xxx`
- 或者用解构 `const { folder, analyzing, ... } = useImagePaddlerState()` 避免模板改动

### ④ 验证
- npm test 通过
- 手动验证图片铺放功能正常

---

## 五、GLM-A4: 取消机制统一

### ① 定位
- `SubprocessRegistry`（lib.rs:22）— PID-based 子进程管理
- `OperationManager`（operations.rs:66）— CancellationToken-based 异步任务管理
- `cancel_operation`（system.rs:172）— 两者都尝试

### ② 分析
- `SubprocessRegistry` 的唯一外部使用方是 `qpdf.rs:9`（`command_output_with_registry`）
- `cancel_operation` 先试 OperationManager，再试 SubprocessRegistry
- `OperationManager` 的注释说"替代 SubprocessRegistry"

### ③ 统一方案
1. 让 `command_output_with_registry` 也通过 OperationManager 管理
2. 或者保留 SubprocessRegistry 作为底层，但统一取消入口为 OperationManager

**推荐：保留双层，但统一取消入口**
- OperationManager 是面向命令层的（run_managed 自动注册）
- SubprocessRegistry 是面向底层子进程的（qpdf 等外部命令）
- 取消时：OperationManager.cancel() 取消异步任务，同时 SubprocessRegistry.cancel_all() 杀子进程

实际改动很小：`cancel_operation` 已经是先试 OM 再试 SR。只需要清理注释和文档。

### ④ 验证
- cargo test 通过
- 手动测试：取消 PDF 合并操作

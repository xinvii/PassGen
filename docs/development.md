# PassGen 开发与验证

更新日期：2026-10-30。开发约束见 [AGENTS.md](../AGENTS.md)，代码和存储格式见 [实现说明](implementation.md)。

## 构建环境

- Rust Stable，项目声明最低 1.92；当前 Windows 验证使用 rustc／Cargo 1.97.1。
- MSVC C++ 构建工具及 Windows SDK，包含资源编译器 `rc.exe`。构建脚本自动定位 SDK 并嵌入应用图标。
- Slint 固定为 1.18.1。提交 `Cargo.lock`，使用 `--locked` 重现锁定的依赖选择。

```powershell
cargo build --locked --release -j 1
```

默认程序为 `target/release/PassGen.exe`，运行时不需要外部字体、图标或网络。依赖已缓存时可加 `--offline`；开发期下载依赖不属于应用运行时联网。

## 代码检查

```powershell
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features -j 1
cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings
```

纯文档修改检查链接、事实与差异即可，无需增加应用测试。涉及生成、设置或交互时，运行对应检查；设置文件测试仅使用临时目录。

## 示例界面检查

```powershell
cargo build --locked --features ui-qa -j 1
& .\target\debug\PassGen.exe --qa-snapshots
```

该检查只使用非安全示例数据，窗口标题为 `PassGen [UI preview]`。它验证界面事件和真实系统复制动作，并把十四个场景截图写入被忽略的 `work/qa/`；复制检查会覆盖系统剪贴板。

场景包括中英文 Random／PIN／About、长密码、扩大后的窗口、菜单无底色、最小化／关闭悬停，以及 Random／PIN 结果框的 `#EAF2FF` 悬停。过程还检查动画中途的顶部位置、高度、白色背景、圆角和投影。

只查看示例窗口可运行：

```powershell
& .\target\debug\PassGen.exe --qa-preview
```

常规预览和截图检查不读写真实用户设置。不要用示例预览生成实际密码；正式 release 不包含示例源或检查入口。

## 隔离的设置重启检查

先按上一节构建带 `ui-qa` 的调试程序，然后使用项目内独立测试文件：

```powershell
$qaRoot = Join-Path (Get-Location).Path ('work/settings-qa-' + [Guid]::NewGuid().ToString('N'))
$quickFile = Join-Path $qaRoot 'quick/settings.json'
$delayedFile = Join-Path $qaRoot 'delayed/settings.json'

& .\target\debug\PassGen.exe --qa-preview --qa-settings-write --qa-settings-file $quickFile
if ($LASTEXITCODE -ne 0) { throw '快速关闭保存检查失败' }

& .\target\debug\PassGen.exe --qa-preview --qa-settings-read --qa-settings-file $quickFile
if ($LASTEXITCODE -ne 0) { throw 'PIN 恢复检查失败' }

$randomOptions = Get-Content -LiteralPath $quickFile -Raw | ConvertFrom-Json
$randomOptions.mode = 'random'
$randomFile = Join-Path $qaRoot 'random-settings.json'
[System.IO.File]::WriteAllText($randomFile, ($randomOptions | ConvertTo-Json), [System.Text.UTF8Encoding]::new($false))
& .\target\debug\PassGen.exe --qa-preview --qa-settings-read --qa-settings-file $randomFile
if ($LASTEXITCODE -ne 0) { throw 'Random 恢复检查失败' }

& .\target\debug\PassGen.exe --qa-preview --qa-settings-write --qa-settings-wait --qa-settings-file $delayedFile
if ($LASTEXITCODE -ne 0) { throw '延迟保存检查失败' }
```

写入场景设置英文、PIN、Random 长度 37、PIN 长度 9 和部分字符类型，再留下无效输入及瞬时界面状态。读取场景验证设置恢复、独立长度、初始窗口高度和瞬时状态重置；延迟场景确认定时保存发生在退出之前。检查只写入明确指定的文件，不访问真实 AppData 配置。

## 已验证版本

2026-10-30，在 Windows 上完成：

| 检查 | 结果 |
| --- | --- |
| 单元测试 | 18 项通过：核心 7、状态 3、动画 2、外观 1、设置 5 |
| 示例界面 | 14 个场景通过 |
| 设置保存与重启 | 快速关闭补写、延迟保存、恢复 PIN、恢复长密码 Random，4 个场景通过 |
| 格式、Clippy、正式构建 | 通过 |
| DPI 外观 | 测试覆盖 100%／125%／150%／200% 的纹理、透明外缘和投影 |

本地正式程序为 `target/release/PassGen.exe`，保留的独立副本为 `target/release/PassGen-Remember.exe`。该次构建大小 10,766,336 字节，SHA-256：

```text
AF2E8FCF76DCAF2AC214AC38486124FDB0CC568DB63CCB7513CCEAF7D2646DB7
```

校验值记录该次本地文件，不保证不同环境重新构建产生相同字节。Linux／macOS、安装包及 Figma 原型动态播放未纳入本次验证；Figma 静态截图不能证明动画播放正确。

## 版本控制范围

提交源码、Slint 界面、构建脚本、`Cargo.toml`／`Cargo.lock`、资源与许可、项目文档及 `AGENTS.md`。

`target/`、`work/` 和日志已被忽略，正式程序、缓存、示例截图和隔离设置文件不进入 Git。真实用户设置位于 AppData，生成的密码不落盘。

# PassGen 开发与验证

更新日期：2026-10-31。开发约束见 [AGENTS.md](../AGENTS.md)，代码和存储格式见 [实现说明](implementation.md)。

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

场景包括中英文 Random／PIN／About、长密码、扩大后的窗口、菜单无底色、最小化／关闭悬停，以及 Random／PIN 结果框的 `#EAF2FF` 悬停。过程还检查动画中途的顶部位置、高度、白色背景、圆角和投影。滚轮事件检查覆盖两种语言与模式的增减、密码结果更新、上下限、横向滚动、左侧留白、独立长度及About打开时禁用。

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

## 隔离的窗口位置检查

同样使用`--qa-preview`和明确的独立配置文件，构建方式与上一节相同：

```powershell
$positionRoot = Join-Path (Get-Location).Path ('work/position-qa-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $positionRoot -Force | Out-Null
$positionFile = Join-Path $positionRoot 'normal.json'
& .\target\debug\PassGen.exe --qa-preview --qa-position-write --qa-settings-file $positionFile
if ($LASTEXITCODE -ne 0) { throw '首次居中或移动保存检查失败' }
& .\target\debug\PassGen.exe --qa-preview --qa-position-read --qa-settings-file $positionFile
if ($LASTEXITCODE -ne 0) { throw '关闭位置恢复检查失败' }

$minimizedFile = Join-Path $positionRoot 'minimized.json'
& .\target\debug\PassGen.exe --qa-preview --qa-position-minimized --qa-settings-file $minimizedFile
if ($LASTEXITCODE -ne 0) { throw '最小化退出检查失败' }
& .\target\debug\PassGen.exe --qa-preview --qa-position-read --qa-settings-file $minimizedFile
if ($LASTEXITCODE -ne 0) { throw '最小化后正常位置恢复检查失败' }

$offscreenFile = Join-Path $positionRoot 'offscreen.json'
[System.IO.File]::WriteAllText($offscreenFile, '{"window_position":{"x":999999,"y":999999}}', [System.Text.UTF8Encoding]::new($false))
& .\target\debug\PassGen.exe --qa-preview --qa-position-center --qa-settings-file $offscreenFile
if ($LASTEXITCODE -ne 0) { throw '屏幕外位置恢复居中检查失败' }
```

五次启动分别验证首次居中并移动、恢复关闭位置、移动并最小化退出、恢复最小化前位置，以及完全在屏幕外的旧坐标回到主屏居中。位置按原生窗口物理坐标检查；多显示器负坐标、部分越界及不同尺寸由单元测试覆盖，未执行多显示器实机切换。

## 已验证版本

2026-10-30，在 Windows 上完成：

| 检查 | 结果 |
| --- | --- |
| 单元测试 | 23 项通过：核心 7、状态 3、动画 2、外观 1、设置 6、位置 4 |
| 示例界面 | 14 个场景及中英文Random／PIN滚轮事件检查通过 |
| 设置保存与重启 | 快速关闭补写、延迟保存、恢复 PIN、恢复长密码 Random；4 个场景在位置记忆版本通过 |
| 窗口位置 | 首次居中、移动后恢复、最小化退出及恢复、屏幕外坐标回退；5 次启动在位置记忆版本通过 |
| 格式、Clippy、正式构建 | 通过 |
| DPI 外观 | 测试覆盖 100%／125%／150%／200% 的纹理、透明外缘、投影深度及1px内描边颜色／位置 |

此前投影、边框及位置记忆版本已完成对应检查。本轮同步六页长度行留白、模式选中背景与语言选择器，以及隐藏过渡图层；Figma十二条导航保留。应用新增滑动条滚轮，二十三项测试与十四个示例场景通过，六页截图确认中英文无裁切，页面动画中途窗口顶部仍保持固定。Figma静态核对不代表重新播放原型动画。

本地正式程序为 `target/release/PassGen.exe`，保留的独立副本为 `target/release/PassGen-Wheel.exe`。该次构建大小 10,804,736 字节，SHA-256：

```text
820377217EA154299207B88C7A0DA22AF5992E41AC43D4A177BA7CA2EB8EF0D8
```

校验值记录该次本地文件，不保证不同环境重新构建产生相同字节。Linux／macOS、安装包及 Figma 原型动态播放未纳入本次验证；Figma 静态截图不能证明动画播放正确。

## 便携测试包

2026-10-31生成内部兼容性测试包 `target/test-builds/PassGen-0.1.0-test.20261031-windows-x64.zip`，包内应用版本仍为0.1.0。ZIP大小6,916,643字节；`PassGen.exe`为10,972,672字节。应用代码、配置格式与Figma布局均未改变；只对该构建启用静态CRT，不覆盖上节的Wheel版本。

普通MSVC构建引用`VCRUNTIME140.dll`；此测试包的构建方式为：

```powershell
$previousPortableFlags = [Environment]::GetEnvironmentVariable('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS')
try {
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
    cargo build --locked --release --target x86_64-pc-windows-msvc --target-dir work/shadow-release -j 1
    if ($LASTEXITCODE -ne 0) { throw '便携版构建失败' }
} finally {
    [Environment]::SetEnvironmentVariable('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS', $previousPortableFlags)
}
```

产物位于`work/shadow-release/x86_64-pc-windows-msvc/release/PassGen.exe`。依赖就绪后可加`--offline`。PE导入表确认不引用VCRUNTIME、MSVCP、UCRT及CRT API-set运行库DLL，其余为Windows组件；正式构建不包含示例预览或截图检查入口。

已在断网Windows沙盒中完成两次启动与正常关闭：x64，系统版本10.0.26100，未安装开发工具，PATH中无Cargo、rustc、MSVC cl、Git或VS Code，System32也无`VCRUNTIME140.dll`。从带中文和空格的目录启动，主窗口出现并响应，两次正常关闭均返回0，配置只含七个非敏感字段。关闭主机剪贴板、音视频和打印机共享，输入目录只读，仅专用报告目录可写；正常结束后关闭沙盒。本轮未截图、读取密码文本或读取剪贴板。

此检查只证明该环境的启动、退出和基本配置行为；人工生成／复制、显示缩放、多显示器及Windows10／其他构建号仍按包内清单继续测试。不要将沙盒检查写成所有目标系统均通过。

包内包含[使用说明](../packaging/windows/README.zh-CN.txt)、[人工清单](../packaging/windows/TEST-CHECKLIST.zh-CN.txt)、[反馈模板](../packaging/windows/FEEDBACK.zh-CN.txt)、`TEST-RESULTS.txt`、`CLEAN-WINDOWS-SMOKE.json`、`BUILD-INFO.json`、源输入哈希、全部包内文件校验值，以及Windows依赖图242个组件和内嵌字体／Rust标准库的许可原文。少数缓存包未附许可文件时，按`.cargo_vcs_info.json`记录的提交从对应上游仓库补齐并记录来源。组件清单保守包含proc-macro依赖，不表示全部代码进入运行时。未替项目作者指定公开源码许可证，本包未公开发布且未数字签名；公开发行授权与Slint署名流程仍待确认。

ZIP的SHA-256：

```text
A4C77F600FCC97CA677F71413559302792E544AE632DB479DC53226E5BB84F80
```

EXE的SHA-256（与沙盒检查文件一致）：

```text
DB23F26BFAE48F93F522EDCBCD019240FCE1D40881E19CBD743D0A0A7C3A1FB7
```

已验证ZIP的CRC、全部文件哈希、路径和白名单；包内不包含用户配置、密码、日志、调试符号或测试脚本。构建基于尚未提交的当前工作区，基准提交与构建输入哈希记录于`BUILD-INFO.json`，不将其误写为已标记的正式发行版本。

## v0.1.0 发布

2026-10-31，用户反馈已在没有开发工具的Windows10和Windows11环境测试使用，未发现问题。这是用户实机反馈；未提供系统具体构建号、CPU架构和完整DPI／多显示器矩阵，不补写未执行的测试。

公开仓库为[PassGen](https://github.com/xinvii/PassGen)，默认分支main。源码使用[MIT](../LICENSE)；Slint桌面二进制选择Royalty-free 2.0，仓库及Release下载页展示官方署名徽标，字体和徽标保留独立许可，详见[第三方声明](../THIRD-PARTY-NOTICES.md)。

首版附件命名为`PassGen-0.1.0-windows-x64.zip`，同时提供对应`.zip.sha256`和`PassGen.exe.sha256`。ZIP放在本地`target/releases/`，通过GitHub Release发布，不放入源码Git历史。包内附README、反馈表、验证记录、项目及第三方许可、全部文件校验值。

发行包中的EXE与已验证测试版一致，仍为10,972,672字节，SHA-256为`DB23F26BFAE48F93F522EDCBCD019240FCE1D40881E19CBD743D0A0A7C3A1FB7`。不把此文件声称为从发布提交重新编译：包内`BINARY-BUILD-INFO.json`与`BINARY-BUILD-INPUTS.json`保留原始测试版的构建信息和输入哈希；新的`BUILD-INFO.json`另记发布源提交、标签、测试包来源及EXE字节一致性。发布整理只改变文档、包内说明、许可、徽标及Cargo授权／仓库元数据，不改变应用运行逻辑。

版本说明见[发布说明](../packaging/windows/RELEASE-NOTES.md)和[更新记录](../CHANGELOG.md)。后续代码变更需要重新构建、验证及生成校验值，不能继续沿用本次测试结论。安装包、数字签名、Linux／macOS及完整显示器矩阵不在首版验证范围内。

## 版本控制范围

提交源码、Slint 界面、构建脚本、`Cargo.toml`／`Cargo.lock`、资源与许可、项目文档及 `AGENTS.md`。

`target/`、`work/` 和日志已被忽略，正式程序、缓存、示例截图和隔离设置文件不进入 Git。真实用户设置位于 AppData，生成的密码不落盘。

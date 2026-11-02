# PassGen 开发与验证

更新日期：2026-11-02。开发约束见 [AGENTS.md](../AGENTS.md)，代码和存储格式见 [实现说明](implementation.md)。

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

场景包括中英文 Random／PIN／About、长密码、扩大后的窗口、菜单无底色、最小化／关闭悬停，以及 Random／PIN 结果框的 `#EAF2FF` 悬停。过程还检查动画中途的顶部位置、高度、白色背景、圆角和投影。Random／PIN／About各进行四次原生移动，在返回Slint定时器前立即核对主窗口与投影的位置及尺寸。滚轮事件检查覆盖两种语言与模式的增减、密码结果更新、上下限、横向滚动、左侧留白、独立长度及About打开时禁用。

只查看示例窗口可运行：

```powershell
& .\target\debug\PassGen.exe --qa-preview
```

常规预览和截图检查不读写真实用户设置。检查从原生窗口初始化完成后启动，避免固定延时与首次窗口创建竞争；末尾还核对停止修改界面后不再重复提交画面。About链接检查拦截系统打开动作，只验证两种语言的鼠标／键盘回调，不自动访问网站。不要用示例预览生成实际密码；正式release不包含示例源或检查入口。

跨进程穿透验证可用调试参数`--qa-preview --qa-hit-test`，此时仅示例窗口保持置顶，便于点击其投影下的独立测试窗口；正式程序没有这个参数入口，始终按正常窗口层级运行。

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

以下各节保留对应构建完成时的验证与提交状态。2026-11-02将这些开发修复、标题字体资源和文档统一整理为本地Git提交；应用版本仍为0.1.0，未推送GitHub，不移动已发布标签或替换Release附件。当前最新本地测试程序为下节的`PassGen-Title.exe`；高速鼠标拖动、跨DPI多屏及新版本在无开发工具Windows10／11环境的完整复测仍未完成。

### 2026-11-02 标题品牌加粗（尚未发布）

仅将标题栏PassGen改为12px Bold 700，嵌入同源的6码位、3,800字节标题字体子集；主要竖笔画约1.77px，与关闭图标1.7px协调。正文仍为Regular 400。Figma六页的标题字宽为50px，与菜单／返回图标保持4px间距；应用按实际字宽布局，尺寸、颜色及交互保持原值。

编译、格式、十四个示例界面及文档检查通过，已查看应用与Figma示例截图，标题未裁切、菜单未重叠。本轮未修改Rust逻辑，没有重复运行核心单元测试。Windows x64静态CRT测试版为`target/release/PassGen-Title.exe`，10,976,256字节，较Movement测试版增加4,096字节；随附`PassGen-Title.exe.sha256`。保留此前测试版与已发布v0.1.0，未提交或发布GitHub。

```text
C41480DEB60CE16AA78A0C27A756F224A8175CF7DB665D2D0D5E00751F80C053
```

### 2026-11-02 移动同步与About同行布局（尚未发布）

About的GitHub移至版本号之后，使用` / `分隔；同步Figma六页的可见及隐藏内容，保留原画板、卡片顶部、150px About高度与十二条导航动画配置。中英文截图确认文字无溢出。Windows在窗口位置消息完成后直接移动缓存的逐像素卡片与附属投影，避免原生拖动的嵌套消息循环延迟Slint定时器造成分层；没有新增轮询、依赖或改变阴影参数。

| 检查 | 结果 |
| --- | --- |
| 代码 | 格式、Clippy严格检查和29项单元测试通过；默认隔离环境曾阻止临时配置覆盖，正常Windows会话重跑全部通过 |
| 示例界面 | 十四场景通过，包括中途动画锚点、亮度、两种语言的同行GitHub鼠标／键盘回调及空闲停止呈现 |
| 移动同步 | Random／PIN／About共十二次原生位置修改，均在延迟刷新前确认主窗口与投影矩形一致 |
| 原生移动模式 | 当前Windows桌面150%缩放下，通过系统“移动”模式连续右移96、下移64物理px，观察到两个窗口位置一致、画面完整；结束后菜单可点击、About完整显示 |
| 既有行为 | 四个隔离设置重启和五个位置场景通过；五进程竞争、重复唤起与最小化恢复、退出后再次启动通过 |

高速鼠标拖动、跨DPI多显示器及无开发工具Windows10／11的新一轮完整矩阵仍待实机复测；静态截图不等同于逐帧录像验证。所有截图均为示例数据，未访问真实设置或密码。

新Windows x64便携测试版为`target/release/PassGen-Movement.exe`，10,972,160字节，静态CRT；随附`PassGen-Movement.exe.sha256`。沿用已有发布构建参数，未采用体积优化实验。此前测试版及已发布v0.1.0原文件保持不变，未提交或发布GitHub。

```text
2411C13F6641EA54413673A7251CD83C97FC22E5371950B71E6626F291B183A4
```

### 2026-11-02 全部焦点框移除（尚未发布）

移除通用按钮焦点轮廓、滑块聚焦蓝色加粗以及长度输入聚焦蓝框。原有FocusScope、Tab、空格／回车、方向键、输入编辑与失焦提交保留；错误红框、结果框和窗口的正常描边保留。中英文和三类页面统一生效，不修改Figma或页面尺寸。

十四个示例界面及原有键盘回调、格式、Clippy检查通过。未改生成、设置、单实例或启动逻辑，本轮没有重复运行纯核心单元测试。新Windows x64便携测试版为`target/release/PassGen-NoFocus.exe`，10,969,600字节，静态CRT；校验文件为`PassGen-NoFocus.exe.sha256`。保留此前的Startup、Fixes测试版和已发布版本，未提交或发布GitHub。

```text
89754AE3B3BCFA46F4EF1768F3EBFD209A2BEAD79505ABF9343BC5D9953A7373
```

### 2026-11-02 启动与单实例修复（尚未发布）

| 检查 | 结果 |
| --- | --- |
| 单元测试 | 29项通过；新增互斥量排他、退出后重获及独立命名空间检查 |
| 原有回归 | 十四个示例界面、四个设置重启、五个位置启动场景通过；首帧实际呈现前确认DWM cloak仍生效 |
| 同时启动 | 五个示例进程竞争，只有一个创建主窗口，其余正常退出 |
| 重复启动 | 主窗口最小化后，同时启动的四个重复进程正常退出，原窗口恢复，仍仅一个主窗口，示例结果不变 |
| 再次打开 | 主实例退出后重新启动及重复唤起检查通过，无残留锁文件 |
| 桌面复查 | 当前Windows示例窗口显示完整；再次启动后主窗口ID不变，数量为一。高速启动过程以首帧cloak状态和提交顺序检查为证，不将静态截图称为逐帧视频验证 |

测试入口仅在带`ui-qa`的调试构建中启用：`--qa-preview --qa-instance-check`。检查会在同一个预览命名空间启动重复子进程、最小化／恢复自身窗口，不访问真实设置或密码；与真实用户主实例隔离。并发启动验证使用该检查的五个进程，只有获锁者执行完整检查。没有重新执行新版在无开发工具Windows10／11、多屏跨DPI环境的完整矩阵；本轮不修改Figma。

格式、Clippy严格警告检查及Windows x64静态CRT构建通过。新测试程序为`target/release/PassGen-Startup.exe`，随附`PassGen-Startup.exe.sha256`；10,999,808字节（10.49 MiB），不依赖额外VC++运行库，正式EXE不包含QA命令入口。沿用原发布构建参数，没有启用体积优化实验。上一版`PassGen-Fixes.exe`和已发布v0.1.0均保留，没有提交或发布GitHub。

```text
2E0E3F678E63A55DFF5EFF7ACC8AA894143D78B6BB51A42939DF710F25A32F28
```

跨进程结论在正常Windows会话中验证：第二个独立启动入口正常退出，窗口仍为同一个ID；不使用开发工具之间的隔离启动作为这一结论的依据。旧测试版没有单实例协议，新版测试前应先关闭旧版。

### 2026-11-02 开发修复版（尚未发布）

本轮落实模式按钮焦点框、About链接与更新占位、13个特殊符号、紧凑透明留白及Windows局部穿透。应用包版本仍为0.1.0，已发布EXE与标签不变；这些检查不能替代新版在无开发工具Windows10／11上的实机验证。

| 检查 | 结果 |
| --- | --- |
| 单元测试 | 27项：核心8、状态3、动画2、外观／输入区域2、原生投影1、设置6、位置5 |
| 示例界面 | 14场景；中英文Random／PIN滚轮、About链接鼠标／回车、顶部锚点、白色背景、悬停、边框与投影；末尾空闲绘制停止 |
| 隔离设置／窗口位置 | 4个设置保存／重启与5次位置启动：独立长度、长密码高度、最小化退出及恢复、屏幕外位置回退 |
| Windows桌面 | 当前150%缩放下，模式／About切换完成后无旧内容或系统标题条；三类页面的投影点击均由独立进程接收 |
| DPI像素计算 | 100%／125%／150%／200%的九宫格、圆角输入区域、描边与投影；非上述四种DPI的完整桌面实机矩阵 |
| 格式／Clippy／构建 | 格式检查、Clippy严格警告检查、Windows x64静态CRT正式构建通过 |
| Figma | 中英文About及四个生成器的隐藏匹配内容；画板和卡片尺寸、十二条导航配置保留，只做静态核对 |

新投影留白为左右22、上16、下28逻辑px，默认Random客户区394 × 307px。8位alpha包围盒实测四边纯透明条带：100%／125%各2物理px、150%／200%各3px。顶部卷积保留采样行，避免裁掉偏移阴影。旧位置文件继续使用版本1；同一DPI下，36px规范化坐标经编解码保持可见卡片位置。

Windows完整画面与点击区域需同时检查。Slint内部截图仅验证布局及动效采样，不能证明桌面合成正确；实际检查使用带`[UI preview]`标题的示例窗口。原生呈现保持显示DIB有效，跳过旧尺寸帧，不改用硬窗口区域，不阻断Slint正常重绘；停止动画和交互后停止提交。Windows10新代码、多屏跨DPI切换及Figma原型重播尚未重新验证。

格式检查、Clippy（`-D warnings`）和静态CRT正式构建通过。独立测试程序为`target/release/PassGen-Fixes.exe`，随附`PassGen-Fixes.exe.sha256`；大小10,992,640字节（10.48 MiB）。只使用原有release配置与`-C target-feature=+crt-static`，未采用体积优化实验参数。PE为Windows x64，不依赖额外VC++运行库，且不包含QA命令入口。校验值记录本次产物，不保证不同环境重新编译字节一致。

```text
1E82E8E219A107BD7073B6ACC1134EBB55DF289694DEA2EB56A1DEA7343A008D
```

已发布`target/releases/PassGen.exe`哈希仍为`DB23F26BFAE48F93F522EDCBCD019240FCE1D40881E19CBD743D0A0A7C3A1FB7`。没有移动标签、更新Release或提交Git；应用版本号仍为0.1.0，本文件用于本地修复测试。

### 2026-10-30 历史验证记录

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

公开仓库为[PassGen](https://github.com/xinvii/PassGen)，默认分支main。源码使用[MIT](../LICENSE)；Slint桌面二进制选择Royalty-free 2.0，字体和徽标保留独立许可，详见[第三方声明](../THIRD-PARTY-NOTICES.md)。首页按用户要求使用其提供的Slint品牌图片，Release不展示图片，不新增官方徽标展示页。

首版最初提供`PassGen-0.1.0-windows-x64.zip`及其校验文件，随后改为EXE、哈希及额外源码ZIP。2026-10-31用户要求去除重复源码附件，当前只手动上传`PassGen.exe`和`PassGen.exe.sha256`；源码使用GitHub自动生成的Source code ZIP／tar.gz，对应原始v0.1.0标签。被移除的附件在本地保留，未移动已有标签，也未改变应用EXE。

发行包中的EXE与已验证测试版一致，仍为10,972,672字节，SHA-256为`DB23F26BFAE48F93F522EDCBCD019240FCE1D40881E19CBD743D0A0A7C3A1FB7`。不把此文件声称为从发布提交重新编译：包内`BINARY-BUILD-INFO.json`与`BINARY-BUILD-INPUTS.json`保留原始测试版的构建信息和输入哈希；新的`BUILD-INFO.json`另记发布源提交、标签、测试包来源及EXE字节一致性。发布整理只改变文档、包内说明、许可、徽标及Cargo授权／仓库元数据，不改变应用运行逻辑。

版本说明见[发布说明](../packaging/windows/RELEASE-NOTES.md)和[更新记录](../CHANGELOG.md)。后续代码变更需要重新构建、验证及生成校验值，不能继续沿用本次测试结论。安装包、数字签名、Linux／macOS及完整显示器矩阵不在首版验证范围内。

已移除的额外源码ZIP以`dbbb18f`为快照，运行逻辑与v0.1.0相同，`SOURCE-RELEASE.json`记录快照提交及EXE原始标签提交，`release-metadata/`保留二进制构建及验证记录。完整依赖声明同时保存在仓库[LICENSES/third-party](../LICENSES/third-party/README.md)：逐文件保留许可内容并汇总为文本，组件清单和Rust标准库声明保持原始字节。GitHub自动生成的v0.1.0源码包仍是原标签快照，不包含后来添加的文档和许可汇总，Release中单独链接当前许可目录。当前图片位置见上文，历史展示方案保留在设计记录中；不因图片调整更改第三方许可条件。

## 版本控制范围

提交源码、Slint 界面、构建脚本、`Cargo.toml`／`Cargo.lock`、资源与许可、项目文档及 `AGENTS.md`。

`target/`、`work/` 和日志已被忽略，正式程序、缓存、示例截图和隔离设置文件不进入 Git。真实用户设置位于 AppData，生成的密码不落盘。

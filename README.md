# PassGen

使用 Rust + Slint 构建的轻量桌面随机密码生成器。应用完全离线，不保存、记录或传输生成的密码。

[![Made with Slint](docs/assets/made-with-slint.png)](https://slint.dev)

## 下载

[下载 Windows x64 便携版](https://github.com/xinvii/PassGen/releases/latest) · [全部版本](https://github.com/xinvii/PassGen/releases)

下载 ZIP 后解压，双击 `PassGen.exe` 即可运行，无需安装 Rust、Visual Studio、WebView 或额外 VC++ 运行库。设置保存在当前 Windows 用户目录。

2026-10-31，用户在没有开发工具的 Windows 10 和 Windows 11 环境中测试使用，未发现问题。未记录具体系统构建号及完整显示器测试矩阵；Linux / macOS 尚未验证。

## 功能与使用

- **Random**：长度 4–64，默认 20。可多选小写字母 `a-z`、大写字母 `A-Z`、数字 `0-9`、特殊符号 `!@#$`，至少保留一种。
- **PIN**：长度 4–12，默认 6，仅使用数字。两种模式分别记住长度。
- 拖动滑块或输入长度即可生成；鼠标停在滑动条上时，滚轮向上增加1位、向下减少1位，自动限制在当前模式范围内。点击密码结果框可重新生成，悬停底色为 `#EAF2FF`。结果框也支持 Tab 聚焦及空格／回车操作。
- 点击下方整行复制按钮复制当前密码；长密码的展示换行不会进入剪贴板。
- 点击标题栏省略号进入 About，切换 English／简体中文；品牌名称 PassGen 不翻译。About 返回对应语言的 Random 页面。
- 自动记住语言、模式、两种长度和字符类型；每次启动重新生成密码。
- 首次启动在主显示器居中；之后恢复上次正常关闭的位置。最小化退出保留最小化前的位置，显示器变化时自动调整到可见区域。
- 白色圆角窗口、柔和投影与顶部固定的页面动画。内嵌字体和图标，运行时不下载资源。

About 中的“检查更新”和 GitHub 当前仅为文字，不触发联网操作；仓库和下载入口在本页提供。

## 本地设置

Windows 配置文件位置：

```text
%LOCALAPPDATA%\PassGen\settings.json
```

在资源管理器地址栏输入 `%LOCALAPPDATA%\PassGen` 可打开文件夹。首次修改并保存选项或正常关闭应用后自动创建文件；连续调整会合并写入，正常退出前会保存未写入的变化及窗口位置。

文件只保存使用选项及窗口坐标，不包含密码、复制内容或历史记录。关闭应用后删除该文件，下次启动会恢复默认设置并居中；配置缺失、损坏或内容无效时也会使用安全默认值。

## 构建与运行

需要 Rust Stable、MSVC C++ 构建工具及 Windows SDK（包含 `rc.exe`）。项目声明最低 Rust 1.92，当前验证版本为 1.97.1；Slint 固定为 1.18.1。

```powershell
cargo run --locked --release
cargo build --locked --release
```

生成的程序为 `target/release/PassGen.exe`。首次构建需要获取依赖，依赖准备好后可加 `--offline` 构建；应用运行时完全离线。

对外提供的 Windows ZIP 使用静态 CRT 构建；默认 Cargo 构建不保证消除 VC++ 运行库依赖。原测试包及静态构建方式见[开发与验证](docs/development.md#便携测试包)，版本变化见[更新记录](CHANGELOG.md)。

## 验证

```powershell
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

示例界面和设置重启检查见 [开发与验证](docs/development.md)。这些检查仅用于调试构建，使用非安全的示例数据；界面复制检查会用示例文本覆盖系统剪贴板。

## 文档与设计

- [实现说明](docs/implementation.md)：代码结构、生成规则、设置格式和隐私边界。
- [开发与验证](docs/development.md)：构建、隔离检查、验证记录和版本控制范围。
- [设计历史](docs/design-history.md)：已确认设计、迭代记录及动画约束。
- [资源说明与字体许可](assets/README.md)：Figma 图标、窗口图标及字体来源。
- [开发指南](AGENTS.md)：项目约束和后续修改规则。
- [Figma 设计](https://www.figma.com/design/1JbCkEMSfcGFHRWGb7jU7G) / [原型入口](https://www.figma.com/proto/1JbCkEMSfcGFHRWGb7jU7G?node-id=2-41)。

核心敏感缓冲区会尽力清理；Slint 字符串副本和系统剪贴板无法保证全部擦除。

## 许可

PassGen 源码使用 [MIT License](LICENSE)。Slint、字体和官方徽标保留独立许可，见[第三方声明](THIRD-PARTY-NOTICES.md)。

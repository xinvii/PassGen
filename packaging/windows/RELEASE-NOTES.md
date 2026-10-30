首个 Windows x64 便携版本，使用 Rust + Slint 构建，运行时完全离线。

<img src="https://raw.githubusercontent.com/xinvii/PassGen/main/docs/assets/slint-logo.png" alt="Slint" width="285">

## 下载与运行

下载下方 `PassGen.exe`，双击即可运行，无需安装。无需 Rust、Visual Studio、WebView 或额外 VC++ 运行库；无需管理员权限。

`PassGen.exe.sha256` 为对应的 SHA-256 校验文件。源码使用下方 GitHub 自动生成的 `Source code (zip)` 或 `Source code (tar.gz)` 下载，对应原始 v0.1.0 标签。

## 功能

- Random／PIN 两种模式，分别记住长度。
- 四类字符多选、滑块与数字输入，滚轮逐格调整长度。
- 点击结果框重新生成，点击复制按钮复制。
- 英文／简体中文界面及 About 页面。
- 圆角投影、顶部固定的页面过渡和窗口位置记忆。
- 使用系统安全随机源和无偏采样，不保存或传输生成的密码。

发行EXE与已验证测试版字节一致，SHA-256：

```text
DB23F26BFAE48F93F522EDCBCD019240FCE1D40881E19CBD743D0A0A7C3A1FB7
```

反馈请描述操作及现象，不要上传生成的密码或剪贴板内容。

## 已知限制

- 当前版本未数字签名。
- Linux／macOS未验证；本附件为x64，非32位或ARM64原生程序。
- About中的“检查更新”和GitHub仍是文字，不触发联网。
- 设置位于`%LOCALAPPDATA%\PassGen\settings.json`，不会随便携文件夹移动。

## 许可

PassGen自身源码为MIT；Slint使用Royalty-free 2.0，官方署名徽标展示于[仓库首页](https://github.com/xinvii/PassGen#readme)。字体、徽标及其他依赖保留独立许可。完整依赖许可保留在[仓库许可目录](https://github.com/xinvii/PassGen/tree/main/LICENSES/third-party)；详见[第三方声明](https://github.com/xinvii/PassGen/blob/main/THIRD-PARTY-NOTICES.md)。

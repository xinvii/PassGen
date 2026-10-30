首个 Windows x64 便携版本，使用 Rust + Slint 构建，运行时完全离线。

[![Made with Slint](https://raw.githubusercontent.com/xinvii/PassGen/v0.1.0/docs/assets/made-with-slint.png)](https://slint.dev)

## 下载与运行

下载下方 `PassGen-0.1.0-windows-x64.zip`，解压后双击 `PassGen.exe`。无需安装 Rust、Visual Studio、WebView 或额外 VC++ 运行库；无需管理员权限。

请下载 Release 附件中的程序 ZIP。GitHub 自动生成的 Source code 压缩包是源码，需要自行构建。

## 功能

- Random／PIN 两种模式，分别记住长度。
- 四类字符多选、滑块与数字输入，滚轮逐格调整长度。
- 点击结果框重新生成，点击复制按钮复制。
- 英文／简体中文界面及 About 页面。
- 圆角投影、顶部固定的页面过渡和窗口位置记忆。
- 使用系统安全随机源和无偏采样，不保存或传输生成的密码。

## 验证

用户在没有开发工具的 Windows 10 和 Windows 11 环境测试使用，未发现问题。还完成了无开发工具、无 VCRUNTIME140.dll 的断网 Windows 沙盒启动、关闭及重启检查。

开发验证包括23项单元测试、14个示例界面场景及中英文Random／PIN滚轮事件检查。实机的具体系统构建号和完整缩放／多显示器矩阵尚未记录。

发行EXE与已验证测试版字节一致，SHA-256：

```text
DB23F26BFAE48F93F522EDCBCD019240FCE1D40881E19CBD743D0A0A7C3A1FB7
```

ZIP的校验值见随附`.zip.sha256`。包内含全部文件校验、验证记录、使用说明和反馈模板。反馈请描述操作及现象，不要上传生成的密码或剪贴板内容。

## 已知限制

- 当前版本未数字签名。
- Linux／macOS未验证；本附件为x64，非32位或ARM64原生程序。
- About中的“检查更新”和GitHub仍是文字，不触发联网。
- 设置位于`%LOCALAPPDATA%\PassGen\settings.json`，不会随便携文件夹移动。

## 许可

PassGen自身源码为MIT；Slint使用Royalty-free 2.0，官方署名徽标展示于本下载页。字体、徽标及其他依赖保留独立许可。包内附许可原文；详见[第三方声明](https://github.com/xinvii/PassGen/blob/v0.1.0/THIRD-PARTY-NOTICES.md)。

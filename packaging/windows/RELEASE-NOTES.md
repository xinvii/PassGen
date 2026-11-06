Windows x64 便携更新，运行时完全离线。

## 本次更新

- 更新应用图标：蓝色渐变底板、白色 PassGen 文字和绿色圆点。窗口及 EXE 图标使用透明外缘。
- 修复启动黑框、移动时卡片与投影分层，以及透明外缘点击穿透。
- Windows单实例：重复启动唤起已有窗口，恢复最小化，不改变当前密码。
- Random特殊符号扩展为完整的 `~!@#$%^&*()_+`（13个）；按钮仍缩略显示 `!@#$`。
- About的GitHub可点击后通过默认浏览器打开；移除检查更新占位，精简同行布局。
- 标题PassGen加粗；全部控件移除焦点框并保留键盘操作。

## 下载与运行

下载下方 `PassGen.exe`，双击运行，无需安装或管理员权限，无需Rust、Visual Studio、WebView或额外VC++运行库。`PassGen.exe.sha256`为对应校验文件。

源码使用GitHub自动生成的 `Source code (zip)`／`Source code (tar.gz)`，对应 v0.1.2 标签。

SHA-256：

```text
AF476F5C613D8E65F16F533E3C5C9C24811D60DD19499C479C4E2136CAE30B7D
```

## 验证与限制

此前的功能改动通过29项单元测试、十四个示例界面和九个隔离设置及窗口位置场景。本次图标更新通过构建、十种图标尺寸的透明度检查及 Windows EXE 图标提取检查。此次没有重新完成无开发工具Windows10／11、跨DPI或多屏实机矩阵；Linux／macOS未验证。程序未数字签名，附件为x64版本。

设置位于 `%LOCALAPPDATA%\PassGen\settings.json`，不随程序目录移动。反馈请描述操作及现象，不要上传密码或剪贴板内容。

## 许可

PassGen自身源码为MIT；Slint使用Royalty-free 2.0。字体、徽标及其他依赖保留独立许可。完整依赖许可见[仓库许可目录](https://github.com/xinvii/PassGen/tree/v0.1.2/LICENSES/third-party)和[第三方声明](https://github.com/xinvii/PassGen/blob/v0.1.2/THIRD-PARTY-NOTICES.md)。

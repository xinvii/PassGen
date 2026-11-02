0.1.2候选版，尚未公开发布。v0.1.1因图标灰色外框已撤回，最新公开版本为v0.1.0。

本次重制移除导出工具额外加入的灰色画布背景，按最新64 × 48px原稿生成真正透明的窗口与EXE图标，保留原图比例。十种ICO尺寸的透明补边均已验证alpha为0。

Windows x64 便携更新，运行时完全离线。

## 本次更新

- 更新应用图标：白色圆角底板、两行深灰PassGen文字和蓝点，同步窗口及EXE图标。
- 修复启动黑框、移动时卡片与投影分层，以及透明外缘点击穿透。
- Windows单实例：重复启动唤起已有窗口，恢复最小化，不改变当前密码。
- Random特殊符号扩展为完整的 `~!@#$%^&*()_+`（13个）；按钮仍缩略显示 `!@#$`。
- About的GitHub可点击后通过默认浏览器打开；移除检查更新占位，精简同行布局。
- 标题PassGen加粗；全部控件移除焦点框并保留键盘操作。

## 下载与运行

下载下方 `PassGen.exe`，双击运行，无需安装或管理员权限，无需Rust、Visual Studio、WebView或额外VC++运行库。`PassGen.exe.sha256`为对应校验文件。

源码使用GitHub自动生成的 `Source code (zip)`／`Source code (tar.gz)`，候选版尚未创建公开发布标签。历史v0.1.0标签和附件保持原样。

SHA-256：

```text
EAA7453FB13412762C671F5B8BA3E9061BD75CEDE2946820815F0D34F12711B6
```

## 验证与限制

29项单元测试、格式和严格代码检查、十四个示例界面及九个隔离设置／窗口位置场景通过；十种新图标尺寸已嵌入EXE。此次没有重新完成无开发工具Windows10／11、跨DPI或多屏实机矩阵；Linux／macOS未验证。程序未数字签名，附件为x64版本。

设置位于 `%LOCALAPPDATA%\PassGen\settings.json`，不随程序目录移动。反馈请描述操作及现象，不要上传密码或剪贴板内容。

## 许可

PassGen自身源码为MIT；Slint使用Royalty-free 2.0。字体、徽标及其他依赖保留独立许可。完整依赖许可见[仓库许可目录](https://github.com/xinvii/PassGen/tree/main/LICENSES/third-party)和[第三方声明](https://github.com/xinvii/PassGen/blob/main/THIRD-PARTY-NOTICES.md)。

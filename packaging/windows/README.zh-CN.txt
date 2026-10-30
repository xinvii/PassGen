PassGen 0.1.0 · Windows x64 便携版
发布日期：2026-10-31

使用方法
1. 将整个 ZIP 解压到普通文件夹，例如桌面。不要直接在压缩包内运行。
2. 双击 PassGen.exe。无需安装 Rust、Visual Studio、Python、Node.js 或 WebView。
3. 无需管理员权限，也不需要网络。字体、图标和 VC++ 运行库已编入程序。

面向 Windows 10 / 11 的64位系统。本包不是32位或ARM64原生版本。
用户已在没有开发工具的Windows 10 / 11环境测试使用，未发现问题。
验证详情和未记录的显示器测试项目见TEST-RESULTS.txt。

基本操作
- 随机密码：4–64位；PIN码：4–12位。
- 拖动长度滑动条，或点击右侧数字输入长度。
- 鼠标停在滑动条上，滚轮向上增加1位，向下减少1位。
- 字符按钮可多选，至少保留一类；选中的类型表示允许出现，不保证每类都出现。
- 点击密码结果框重新生成。点击下方复制按钮复制当前密码。
- 标题旁的省略号打开关于页面，可切换 English / 简体中文。
- 关闭后再打开，恢复选项和关闭位置；首次启动居中。

本地设置
位置：%LOCALAPPDATA%\PassGen\settings.json
在资源管理器地址栏输入 %LOCALAPPDATA%\PassGen 即可打开文件夹。
只保存语言、模式、两种长度、字符选择和窗口坐标，不保存密码或复制历史。
这是便携运行包，设置仍保存在当前 Windows 用户目录，不跟随 EXE 移动。
每次启动都会重新生成密码。

测试提示
- 按 TEST-CHECKLIST.zh-CN.txt 检查。可在 FEEDBACK.zh-CN.txt 中填写结果。
- 复制测试请先确认剪贴板中的原有内容可以被替换，再在记事本中粘贴核对。
- 反馈不要包含生成的密码、剪贴板内容或带有真实密码的截图。
- 当前 About 中的“检查更新”和 GitHub 只是文字，不提供联网更新或链接功能。
- 当前版本尚未数字签名；如系统拦截启动，请记录完整提示，不必修改安全设置。

卸载
关闭应用后删除解压文件夹即可。若要同时清除设置，关闭应用后再删除
%LOCALAPPDATA%\PassGen 文件夹。正常关闭前不要删除正在使用的配置文件。

项目及下载：https://github.com/xinvii/PassGen
版本下载：https://github.com/xinvii/PassGen/releases
PassGen自身源码使用MIT，见LICENSE.txt；第三方组件、字体和徽标保留独立许可。
此桌面发行版选择Slint Royalty-free 2.0，仓库和版本下载页展示官方署名徽标。
许可原文及第三方声明位于LICENSES、licenses和THIRD-PARTY-NOTICES.md。
重新分发时请保留适用的版权声明，并按所选择的Slint许可履行署名等义务。
构建及验证信息见BUILD-INFO.json、BINARY-BUILD-INFO.json和TEST-RESULTS.txt；
文件校验值见SHA256SUMS.txt。EXE与此前验证的测试版字节一致。

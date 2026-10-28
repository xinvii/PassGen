# PassGen

轻量、完全离线的跨平台随机密码生成器。

## 已确定的技术方案

- Rust + Slint，使用 Cargo 构建。
- 现代轻量 UI，优先实现 Windows，Linux、macOS 为后续目标。
- 不保存、记录或传输生成的密码。

## UI 设计

[Figma 可编辑设计](https://www.figma.com/design/1JbCkEMSfcGFHRWGb7jU7G)

- Random 与 PIN 两种模式，移除 Memorable。
- 白色圆角卡片、蓝色主按钮与菜单；外层画板背景透明。
- 中英文 Random、PIN、About 共六个界面；PassGen 名称保持不变。
- Random 字符类型采用可多选按钮，替代 switch：
  - 小写字母 (a-z)
  - 大写字母 (A-Z)
  - 数字 (0-9)
  - 特殊符号 (!@#$)
- PIN 使用数字字符，单独设置长度。
- 提供复制密码与重新生成按钮。
- Figma 内密码仅为展示示例；当前尚未实现应用功能。

## 交互约定

- 字符类型按钮再次点击可取消选择；至少保留一种类型。
- 蓝色边框与浅蓝底表示已选择，未选择使用中性底色。
- Random 和 PIN 分别保留各自的长度设置。
- 界面展示密码不代表密码历史记录；应用不持久化密码。

## 设计基线

2026-10-29：基本设计已确认可用。后续实现以此为基线。

详见 [设计历史与交互规范](docs/design-history.md)。

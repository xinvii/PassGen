# Third-party notices

PassGen 自身的源码使用 [MIT License](LICENSE)。第三方组件、字体和徽标保留各自的许可与版权，MIT 不替换这些条款。

## Slint

本项目使用 Slint 1.18.1。此 Windows 桌面发行版选择 **Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0**，全文见 [Slint 许可](LICENSES/Slint-Royalty-free-2.0.md)。

仓库首页及 GitHub Release 下载页展示官方 Made with Slint 徽标，履行该许可的网页署名要求。重新分发二进制文件时，应继续按照所选择的 Slint 许可履行署名或其他义务；不能仅因 PassGen 源码为 MIT 就将 Slint 重新许可为 MIT。

- [Slint 官方许可说明](https://github.com/slint-ui/slint/blob/master/FAQ.md#licensing)
- [Made with Slint 徽标来源与品牌指南](https://github.com/slint-ui/slint/tree/master/logo)

## 字体

内嵌 Noto Sans SC Regular 字体子集使用 SIL Open Font License 1.1，见 [字体许可](LICENSES/NotoSansSC-OFL.txt)。历史 Roboto／Inter 字体及其许可仍保留在 `assets/`，当前应用不导入它们。

## Made with Slint 徽标

`docs/assets/made-with-slint.png` 来自 SixtyFPS GmbH／Slint 官方的 `MadeWithSlint-logo-whitebg.png`，未修改。`docs/assets/slint-logo.png` 为用户提供的 Slint 品牌图片，同样保持原图。Slint 徽标归原作者 SixtyFPS GmbH／Slint 所有，使用 [CC BY-ND 4.0](LICENSES/Slint-logos-CC-BY-ND-4.0.txt)，不受 PassGen 的 MIT 许可替代。

## 其他依赖

依赖版本由 `Cargo.lock` 固定。当前Release直接提供EXE和哈希文件；另附`PassGen-0.1.0-source.zip`，其`third-party-licenses/`包含依赖组件清单、许可原文、字体许可和Rust标准库第三方声明。该清单保守包含构建依赖图中的过程宏依赖，不表示所有依赖代码都进入运行时。原程序ZIP保留在本地历史产物中，不再作为Release附件提供。

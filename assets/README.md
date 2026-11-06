# PassGen 资源与字体

应用在构建时嵌入所需资源，运行时不下载字体或图标。设计来源为 [PassGen Figma](https://www.figma.com/design/1JbCkEMSfcGFHRWGb7jU7G)。

## 当前使用的资源

| 文件 | 设计节点／来源 | 原始尺寸 |
| --- | --- | --- |
| `check.svg` | `I67:720;67:210;83:31139;68:15642` | 12.2667 × 8.93333 |
| `figma/title-menu.svg` | `4:37` | 16 × 16 |
| `figma/title-back.svg` | `88:767` | 16 × 16 |
| `figma/window-close.svg` | `I128:1747;98:282` | 12 × 12，线宽 1.7px |
| `figma/slider-thumb.svg` | `2:59` | 16 × 16 |
| `passgen.svg` | Page icon 的 Frame 4（`192:299`）导出容器 | 64 × 48 |
| `passgen.png` | 从上述矢量图生成，供 Slint 窗口使用 | 256 × 256 |
| `passgen.ico` | 从上述矢量图生成，嵌入 Windows EXE | 16、20、24、32、40、48、64、96、128、256px |
| `window-close-hover.svg` | 应用绘制关闭按钮底色的遮罩 | 44 × 30，内缩 1px、右上内圆角 18px，保留窗口描边 |

Figma图标SVG采用最新原稿，移除资产下载工具额外加入的`#E9E9E9`画布矩形（直接节点导出确认原稿无此背景）。2026-11-03按用户最新修改更新为Frame 4：64 × 48px白色圆角底板、两行深灰PassGen文字和蓝点，保留原稿裁切及文字投影；上下原始SVG为64 × 48px，生成PNG／ICO时等比居中并透明补边为正方形，不拉伸原图。关闭按钮悬停遮罩属于实现资源。

滑块是按设计几何实现的交互控件，不使用整张滑块截图。较早导出的菜单、返回、滑块和关闭 SVG 保留为历史资源，当前调用位置以 `ui/` 中的路径为准。

## 字体与许可

中英文统一使用 `NotoSansSC-UI.ttf`，为 Google Fonts Noto Sans SC 的 Regular 400 子集，当前包含 172 个码位、37,228 字节。字体使用 fontTools 4.68.0 生成，许可见 [NotoSansSC-OFL.txt](NotoSansSC-OFL.txt)。

标题栏的 `PassGen` 单独使用 `NotoSansSC-Title.ttf`，同源字体的 Bold 700 静态子集，仅含品牌所需6个不同码位，3,800字节。从源可变字体先筛选`PassGen`，再通过fontTools的`instantiateVariableFont`固定`wght=700`并启用`updateFontNames=True`，保留版权信息及上述许可。12px字号下P的主要竖笔画约1.77px，与关闭图标的1.7px接近；正文仍为400。字体随EXE嵌入，运行时不读取外部文件。

增加界面或错误提示文案时，重新生成字体子集并检查新字形；字体只在构建时嵌入。

历史字体 `Roboto.ttf` 和 `Inter.ttf` 仍保留，当前 UI 不再导入。对应许可为 [Roboto-OFL.txt](Roboto-OFL.txt) 和 [Inter-OFL.txt](Inter-OFL.txt)。

## 最新图标修订

2026-11-03再次读取用户新修改的Frame 4（`192:299`），现为蓝色渐变底板、白色PassGen文字和绿色圆点（`#00FF4D`），64 × 48px。直接采用节点`exportAsync(SVG_STRING)`原稿，保持裁切、文字投影及透明外缘；窗口PNG和十种ICO同步更新。上文白底灰字蓝点为此前修订记录，已被本版取代。

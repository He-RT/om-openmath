# OpenMath 导出标签字体

`crates/om-kernel/assets/OpenMathPlotLabels-Regular.otf` 是由 Noto Sans SC Regular 的固定上游版本生成的标签字体，用于纯 Rust PNG 输出。覆盖拉丁字母、希腊字母、常用数学符号和字体已有的基本中文；缺字明确拒绝 PNG，SVG 保留原文字，不生成空标签。此字体不是公式排版引擎，也不替代已有 SwiftMath/KaTeX 字体。

上游：[Noto CJK](https://github.com/notofonts/noto-cjk)，commit `f8d157532fbfaeda587e826d4cd5b21a49186f7c`，文件 `Sans/SubsetOTF/SC/NotoSansSC-Regular.otf`。许可为随附 OFL-1.1；原始字体的版权/作者记录保持在 OpenType name 表中，衍生字体家族和 PostScript 名已更改为 OpenMath Plot Labels，以避免把修改版本称作原始字体。

生成工具固定 fontTools 4.62.1；脚本 `scripts/build-export-font.py` 从 `target/font-source/NotoSansSC-Regular.otf` 读取固定源文件，关闭时间戳重写、固定 Unicode 范围并重命名。源码工程已经随附生成字体，正常 Rust/Xcode/WASM 构建不要求安装 Python 字体工具、下载或读系统字体。上游和产物的 SHA256、字节数、范围、字形数在 manifest.json。

字体许可与清单随 Web/桌面、iOS 和 XCFramework 一同分发；单独使用图片或 SVG 文档不要求把生成文档改为字体许可。PNG 光栅化仅用 `Swash = 0.2.10`（禁用默认特性，显式启用std/scale/render），读取固定内嵌字体；它不访问系统字体目录，不是第三方数学运行时或三维框架。

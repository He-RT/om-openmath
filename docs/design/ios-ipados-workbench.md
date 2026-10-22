# iOS / iPadOS 工作台草稿 01

2026-10-22 · [交互原型](prototypes/ios-ipados-workbench.html) · [总览](prototypes/ios-ipados-overview.jpg) · [iPhone](prototypes/ios-workbench-phone.jpg) · [推导面板](prototypes/ios-workbench-steps.jpg) · [iPad 横屏](prototypes/ipados-workbench-landscape.jpg) · [iPad 竖屏](prototypes/ipados-workbench-portrait.jpg) · [编辑态](prototypes/ios-workbench-editing.jpg) · [深色](prototypes/ipados-workbench-dark.jpg)

用户要求参考现有 Web 手机端与 Mac 端画移动草稿。本稿是独立的 HTML/CSS/JS 交互设计，没有修改生产 Web、Mac 或现有 SwiftUI 客户端，也不替代冻结的 Mac/Web 工作台规范。

视觉主张：一张随屏幕展开的数学纸面。iPhone 聚焦阅读和编辑，iPad 为同一文稿展开目录和检查器。

内容布局：文稿列表 → 笔记本 → 求根与图形；检查面板承载步骤、助手和变量。没有额外的首页或数据看板。

交互主张：面板从底部展开；文稿列表用短距离横向进入；横竖屏改变辅助栏位置，主文稿不重建。减少动态效果时取消过渡。

## 参考与保留

- [工作台 v2](workbench-v2.md)：白纸、浅灰源码槽、绿色结果左轨、衬线正文与数学结果、等宽源码，以及助手 / 步骤 / 变量三种检查目的地。
- [Mac 浅色工作台](../acceptance/pre-alpha.5/g1-mac-light.png)：目录—文稿—检查器的空间关系，标题与保存状态、运行、添加和面板开关。
- [Web 手机端](../acceptance/pre-alpha.5/g1-web-phone.png)及 `app/src/styles/responsive.css`：单栏文稿、独立滚动的检查面板与固定输入区。
- [现有 iOS 文档](../ios.md)及 `ios/OpenMath/RootView.swift`：系统文稿选择与分享、自动保存、硬件键盘、宽屏检查器 / 窄屏 sheet；本稿重新整理其视觉和入口。

沿用 `#1D7550` 绿色主动作、`#5551CF` 助手标识；图形用蓝色曲线与橙色零点，避免和结果状态混淆。深色采用 v2 的纸面、工具区与语义色。界面用系统无衬线，正文和结果用系统衬线，源码用系统等宽；无远程字体或图片依赖。

## iPhone

- 顶部是返回「文稿」、当前文稿名、运行和更多。文稿菜单集中分享、导出和设置，主工作区不堆放文件操作。
- 文稿保留阅读排版。输入置于源码槽，结果直接排在纸面，状态文字说明精确解与验证，不能只靠左轨颜色。
- 底部固定「目录 / ＋ 单元格 / 检查面板」。相较 Web 的浮动助手入口，原生草稿把三个高频操作放进统一工具栏，避免遮挡图形。
- 点「步骤」直接打开相应推导；点检查面板则恢复上次目的地。目录、步骤、助手和变量使用设备内 sheet；背景文稿仍保留。
- 点源码进入编辑，显示数学符号辅助行和键盘布局。完成后回到阅读；修改后的旧结果变淡并标为待更新。底部工具栏在编辑时让位于键盘。
- 文稿列表提供最近打开、搜索、从系统文件打开的入口。列表不暗示已经具备 iCloud 同步。

## iPad

| 可用宽度（拟议） | 主布局 | 辅助面板 |
| --- | --- | --- |
| ≥ 1100 pt | 目录 188 + 文稿弹性宽度 + 检查器 310 | 可分别收起目录 / 检查器 |
| 760–1099 pt | 文稿 + 检查器约 300 | 目录按需弹出 |
| < 760 pt | 单栏文稿 | 目录 / 检查器使用 sheet，显示底部工具栏 |

横屏参考画布为 1194 × 834；竖屏为 834 × 1112；窄窗口为 640 × 834。它们用于评审空间分配，不绑定具体硬件型号。断点是本稿的建议；没有修改现有客户端 696 pt 的分栏实现。

- 横屏承接 Mac 的三栏，而非放大 iPhone。文稿独立滚动，检查器独立滚动；助手输入框固定在栏底。
- 竖屏保留文稿与检查器，收起目录，优先保证公式行宽。窄窗口再退到单栏。
- 保留触摸与硬件键盘两套入口。原型支持 ⌘/Ctrl + Return 预览预置运行、Escape 关闭面板 / 退出编辑；原生实施仍需复用现有运行、合成输入与撤销规则。
- 分栏变化不应丢失源码、选区、滚动位置、图形视角或助手草稿。原型不重建文稿 DOM；真正的 SwiftUI 宿主状态保持仍需实施。

## 草稿怎么用

直接打开 HTML 可用；也可启动独立只读静态服务：

```sh
python3 -m http.server 8794 --bind 127.0.0.1 --directory docs/design
```

访问 `http://127.0.0.1:8794/prototypes/ios-ipados-workbench.html`。顶部评审工具不属于产品界面。

- 设备：总览 / iPhone / iPad。
- 场景：阅读与图形、推导步骤、助手、变量、公式编辑、文稿列表。
- iPad：横屏 / 竖屏 / 窄窗口；另有深浅色切换。
- 文稿内可点步骤、目录、检查器、源码、添加、复制与菜单；文稿列表可搜索。
- URL 支持 `view=phone|pad|overview`、`orientation=landscape|portrait|compact`、`scene=reading|steps|assistant|variables|editing|library`、`theme=dark|light`。

例如：

```text
?view=phone&scene=editing
?view=pad&orientation=portrait&scene=assistant
?view=pad&theme=dark
```

## 展示范围

`solve(x^2 - 5*x + 6 = 0, x)` 的 2、3 是预置结果；推导、变量与助手记录均为示例。曲线是该固定函数的 SVG 参考图。任意编辑只保留在内存，不计算新结果；发送不会请求模型。系统键盘、文件选择、分享和附件选择仅展示入口与布局。刷新恢复初始内容。

这是视觉与交互草稿。没有进行 iOS 原生构建、模拟器启动、真实模型请求或发行包验证。后续实施应使用现有 Rust 内核、原生数学 / 文稿 / 步骤能力和系统控件；不因 HTML 能点击就称已完成 iOS/iPadOS 客户端。

# iOS / iPadOS 工作台草稿 02：原生玻璃材质

2026-10-23（补充 iPhone Duo） · [SwiftUI 源稿](prototypes/ios27-glass/GlassWorkbench.swift) · [截图图集](prototypes/ios27-glass/index.html) · [Duo 展开](prototypes/ios27-glass/duo-unfolded.jpg)

用户认为草稿 01 的 Liquid Glass 不足，要求重新写一份，并允许改变呈现形式。本稿改用 **SwiftUI 原生预览程序**。导航和操作组由系统 `glassEffect` 渲染，使用 `GlassEffectContainer` 组织相邻控件；HTML 仅作为截图图集，不模拟玻璃。

**预览载体是 macOS 27.0。iPhone / iPhone Duo / iPad 是布局草稿，尚未在 iOS / iPadOS 设备上运行。** 这份程序独立于生产 OpenMath，没有改动既有 Web、Mac 或 iOS 客户端。

## 本次设计

视觉主张：数学保持清晰的纸面，操作浮在纸面上。玻璃的透色、折射和边界由系统材质提供；不在内容区铺设装饰性玻璃卡片。

内容布局：iPhone 为单栏文稿；Duo 随可用空间在单栏、文稿—检查器双栏及半折布局间切换；iPad 横屏为目录—文稿—检查器，竖屏收起目录，窄窗口保留单栏。保留现有绿色左轨、浅灰源码槽、衬线纸面、等宽源码和助手 / 步骤 / 变量检查器。

交互主张：悬浮工具组随滚动收起文字；玻璃控件使用系统交互反馈；菜单在触发位置展开；检查器选中态通过玻璃容器与标识衔接，手机面板从底部展开。

与草稿 01 相比：

| 区域 | 草稿 02 |
| --- | --- |
| iPhone 顶部 | 移除整条实色底板，返回、运行、更多使用独立圆形玻璃控件，文稿标题留在中间 |
| iPhone 底部 | 目录 / 添加放在共享玻璃胶囊；检查器为独立圆形操作；两组悬浮于文稿之上 |
| 滚动 | 文稿从玻璃操作下方经过；底部「单元格」文字收起，保留图标 |
| 菜单 | 触发位置附近展开圆角玻璃菜单，保留背景内容的透色与扩散 |
| iPad 顶部 | 导航、编辑、运行 / 检查器按功能分组；使用同一套原生玻璃材质 |
| iPad 辅助栏 | 使用标准系统材质承载目录和检查内容；上层选择、运行和发送使用玻璃 |
| 公式与图形 | 保留可读的纸面；求解与曲线示例均为预置，不进行新计算 |

## iPhone Duo

Duo 沿用同一份笔记本和原生玻璃材质。外屏与内屏变化只调整呈现，不建立两份文稿；布局读取容器的实际宽高，不读取 `UIScreen` 或以设备 idiom 决定栏数。

| 姿态 | 文稿与检查器 | 操作与折叠区域 |
| --- | --- | --- |
| 合拢 | 单栏；检查器按需从底部打开，保留原选中项 | 纵向玻璃工具组避开评审用状态 / 相机区域 |
| 展开 | 有足够横向空间时，左侧文稿、右侧检查器 | 纵向工具组位于右边缘，平展无折叠分隔 |
| 书本半折 | 宽屏双栏分别位于折叠区域两侧；窄空间仍保留连续文稿 | 折叠区域示意不承载按钮；连续滚动内容可以经过它 |
| 桌面半折 | 上方阅读，下方检查；文稿仍为同一连续滚动视图 | 通过滚动留白让被下方面板遮住的内容可滚到上方 |
| 旋转 / 调整窗口 | 根据当前可用空间重新排列，空间不足时回到单栏 | 工具组继续留在边缘；不以横竖屏名称硬编码设备类型 |

面板的选中项、输入草稿、数学源码与阅读锚点保存在同一预览状态中。合拢时收起侧栏以继续阅读，再点检查器可打开同一内容；展开时恢复侧栏。文稿的 `ScrollView` 在尺寸变化中保持同一结构位置，不通过重建页面恢复状态。

预览提供四种姿态、旋转和宽度滑杆，默认打开 Duo。画布采用接近内外屏的比例，**其数值和 72 点操作边缘都是评审输入，不是已核验的设备 point、安全区或折叠区域测量值**。半折是手动评审状态，不读取真实铰链；浅阴影只标识保留区，不模拟硬件折痕。

Apple 的 Duo 资料将纵向工具栏 / 标签栏及折叠、相机保留区域能力标为 **iOS 27.1**。后续生产接入应由系统提供 `GeometryProxy.reservedRegions(kind:)` 的 `.division` / `.occlusion`，并评估 `ArrangementView`；当前 macOS 27.0 草稿没有调用这些 iOS 27.1 API，不能据此声称已完成真机适配。

## 查看方式

本地预览程序：

```text
target/ios27-glass-study/OpenMath Glass Study.app
```

重新构建只需要本机 Xcode 27 / macOS 27 / Apple Silicon，无第三方依赖：

```sh
bash docs/design/prototypes/ios27-glass/build-preview.sh
open 'target/ios27-glass-study/OpenMath Glass Study.app'
```

构建脚本将产物放在项目 `target/`，使用临时模块缓存并在脚本退出时删除，保留预览程序。仅 ad hoc 本地签名，不是 iOS 发行包。

预览窗口可切换总览 / iPhone / iPad / Duo、横屏 / 竖屏 / 窄窗口、阅读 / 步骤 / 助手 / 编辑、浅色 / 深色，以及常规 / 通透材质。Duo 另有合拢 / 展开 / 书本半折 / 桌面半折、旋转和可用宽度。材质切换是**草稿评审工具**；它不修改系统 Liquid Glass 设置，也不等同于系统染色滑块。

浏览器图集：

```text
http://127.0.0.1:8794/prototypes/ios27-glass/index.html
```

该图集使用已有 `docs/design` 静态服务，保存实际原生窗口截图。它不承载原生交互。

## Apple 参考

- [Prepare your app for iPhone Duo](https://developer.apple.com/iphone-duo/prepare/)：随实际尺寸变化布局、保持连续性；iOS 27.1 的纵向工具栏 / 标签栏。
- [Design for iPhone Duo（Tech Talk 111463）](https://developer.apple.com/videos/play/tech-talks/111463/)：折叠 `.division`、相机 `.occlusion`，连续内容与交互控件的不同处理方式；参考其文字稿。
- [iPhone Duo 技术规格](https://www.apple.com/iphone-duo/specs/)：内外屏比例参考；像素规格不能直接当作布局 point 或安全区。
- [iOS 27 官方介绍](https://www.apple.com/os/ios/) / [iPadOS 27 官方介绍](https://www.apple.com/os/ipados/)：设计细化、折射与对比度、通透到染色的系统外观调节。
- [WWDC26 Keynote](https://developer.apple.com/videos/play/wwdc2026/101/)：背景复杂内容的扩散、更清楚的层次与可读性。
- [Materials](https://developer.apple.com/design/human-interface-guidelines/materials)：玻璃承载上层导航 / 控件，标准材质承载内容层结构。
- [Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)：按功能组织工具组，保持跨设备关系。
- [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/)：折射、动态反馈与形态连续性。

## 展示边界

求根 2、3、推导、变量与助手对话都是预置数据。任意编辑只修改预览内存，旧结果标记为待更新；不会连接 Rust 内核、模型、用户文稿、Keychain 或网络。键盘是布局示意。点击复制会将示例解集写入剪贴板，其他文件 / 附件 / 分享入口只展示提示。

本次只编译独立设计预览。Duo 已实际查看合拢、展开、两种半折、旋转、340 点评审窄宽度与深色；手动编辑源码后合拢 / 展开，修改与旧结果提示保留；输入草稿、选中的变量检查器和图形阅读位置也保留；关闭检查器后可再次从文稿打开步骤。图集保存选定的真实原生窗口截图，旧 iPhone / iPad 截图保留。

CI 仅将这份独立预览目录和两份 iOS / iPadOS 草稿说明归入契约检查；实际生产设计、schema、提示词和混合产品变更仍触发原矩阵，分类回归覆盖此边界。SwiftUI 预览在本机按上述入口编译，CI 不代替它的材质与交互检查。

不启动 iOS 模拟器，不重建生产客户端或发行包，不调用真实模型。截图代表 macOS 27 上的材质与布局表现，触屏、真实折叠切换、iOS 系统导航、手机键盘和辅助功能完整覆盖仍需后续原生实施。

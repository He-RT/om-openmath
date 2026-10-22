# iOS / iPadOS 工作台草稿 02：原生玻璃材质

2026-10-22 · [SwiftUI 源稿](prototypes/ios27-glass/GlassWorkbench.swift) · [截图图集](prototypes/ios27-glass/index.html) · [总览](prototypes/ios27-glass/overview.jpg)

用户认为草稿 01 的 Liquid Glass 不足，要求重新写一份，并允许改变呈现形式。本稿改用 **SwiftUI 原生预览程序**。导航和操作组由系统 `glassEffect` 渲染，使用 `GlassEffectContainer` 组织相邻控件；HTML 仅作为截图图集，不模拟玻璃。

**预览载体是 macOS 27.0。iPhone / iPad 是布局草稿，尚未在 iOS / iPadOS 设备上运行。** 这份程序独立于生产 OpenMath，没有改动既有 Web、Mac 或 iOS 客户端。

## 本次设计

视觉主张：数学保持清晰的纸面，操作浮在纸面上。玻璃的透色、折射和边界由系统材质提供；不在内容区铺设装饰性玻璃卡片。

内容布局：iPhone 为单栏文稿；iPad 横屏为目录—文稿—检查器，竖屏收起目录，窄窗口保留单栏。保留现有绿色左轨、浅灰源码槽、衬线纸面、等宽源码和助手 / 步骤 / 变量检查器。

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

预览窗口可切换总览 / iPhone / iPad、横屏 / 竖屏 / 窄窗口、阅读 / 步骤 / 助手 / 编辑、浅色 / 深色，以及常规 / 通透材质。材质切换是**草稿评审工具**；它不修改系统 Liquid Glass 设置，也不等同于系统染色滑块。

浏览器图集：

```text
http://127.0.0.1:8794/prototypes/ios27-glass/index.html
```

该图集使用已有 `docs/design` 静态服务，保存实际原生窗口截图。它不承载原生交互。

## Apple 参考

- [iOS 27 官方介绍](https://www.apple.com/os/ios/) / [iPadOS 27 官方介绍](https://www.apple.com/os/ipados/)：设计细化、折射与对比度、通透到染色的系统外观调节。
- [WWDC26 Keynote](https://developer.apple.com/videos/play/wwdc2026/101/)：背景复杂内容的扩散、更清楚的层次与可读性。
- [Materials](https://developer.apple.com/design/human-interface-guidelines/materials)：玻璃承载上层导航 / 控件，标准材质承载内容层结构。
- [Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)：按功能组织工具组，保持跨设备关系。
- [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/)：折射、动态反馈与形态连续性。

## 展示边界

求根 2、3、推导、变量与助手对话都是预置数据。任意编辑只修改预览内存，旧结果标记为待更新；不会连接 Rust 内核、模型、用户文稿、Keychain 或网络。键盘是布局示意。点击复制会将示例解集写入剪贴板，其他文件 / 附件 / 分享入口只展示提示。

本次只编译了独立设计预览，查看了原生材质、菜单、步骤面板、横竖屏与深色等关键状态。不启动 iOS 模拟器，不重建生产客户端或发行包，不调用真实模型。截图代表 macOS 27 上的材质与布局表现，触屏、iOS 系统导航和手机键盘仍需后续原生实施。

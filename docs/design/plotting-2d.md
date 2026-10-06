# 二维采样、几何与跨端展示

[下一版账本](../plan/NEXT_RELEASE.md) · [函数参数](../reference/executable.md) · [验收](../acceptance/r35b/README.md)

本节记录 `.3` dev 的 R3.5b；应用版本仍为 `.2`。`explore`、图形导出与三维是后续批次，未因二维几何已接入而宣称发行完成。

## 接口与兼容

原 `Plot`/`ContourPlot` 曲线、解点和区域 wire 保持。`PlotRequest.options`、`PlotData.geometry/scale` 都是可选追加字段；旧请求仍走旧路径。新增 held 注册入口 `ParametricPlot`、`RegionPlot`、`FieldPlot`、`DataPlot`、`Histogram`、`DensityPlot`；现代接口使用 `parametric_plot` 等 snake_case，`plot(...,view:"density")` 在解析适配阶段选真实密度回调。

原 `implicit_plot` 保持稳定 fn_000119/ContourPlot 解析身份；独立三维扩展 fn_000220 仍预留，未注册占位回调。

数学轴保存在 `options.axes`；相机只保存 `x_range/y_range`。参数曲线的 t 域在平移/缩放后保持，原范围不会被横轴相机替换。直方图仅在纵轴未指定时按真实最大频数自动取窗，重采样不会覆盖手工视窗。数据图保留原输入顺序；改相机不重新拟合、不重排数据。

所有几何来自 Rust 内核。tiles 携真实 bounds/value/color；arrows 携真实 start/value 与共同缩放后的 end；points 是真实未连线坐标。渲染器只变换坐标、裁剪和显示，不能执行源式、统计箱频数或计算密度色值。

## 算法与限制

| 入口 | 当前真实算法 | 限制 |
|---|---|---|
| 参数曲线 | 800 基础点与相邻中点检查，非有限或可识别跳变分段 | 单参数二维；有限网格可能漏掉奇点或细节 |
| 隐式轮廓/标量等高线 | 160×160 marching squares、原式数值残差检查，1..32 levels | 近似轮廓，不是精确解集证书 |
| 区域 | 96×96 中点比较/and/or/not 的布尔网格 | 窄区域/复杂边界可能漏采，不声明认证边界 |
| 密度 | 96×96 有限标量值，内核按实际有限范围配色 | 全部非有限或颜色跨度无法表示时失败 |
| 向量场 | 20×20 箭头，统一最大范数视觉缩放且保留原始向量 | 箭长用于显示，不能充当原时间速度 |
| 流线 | 64 种子、归一方向 RK4 双向每向最多240步，零场/域外终止 | 不保证完整拓扑，步长固定为短边/200 |
| 数据图 | ≤100000 标量的成对坐标/x-y数据表；scatter/line/heatmap | 热图矩形，不执行文本数据；连线保留输入顺序 |
| 直方图 | 默认20、1..200等宽频数箱，最大值进入最后箱 | 常量样本扩展非零宽箱域；计数是整数频数 |

机器实数采样；精确表达式可数值采样，新增路径拒绝高精度数静默降级。传统 Plot/ContourPlot 保持已有机器采样兼容语义，不提供高精度图形保证。源式先在只读局部坐标作用域准备，再编译实际数值程序，保留可识别原极点；轴不捕获主会话同名值，参数仍形成真实依赖。所有遍历、采样和输出受原 Interrupt 预算；中断返回实际错误，不能返回部分图冒充完成，下一请求可恢复。

`scale` 支持 linear/log_x/log_y/log_log。数据仍用原数学坐标保存，宿主只做对数显示和相机变换；函数横轴在 log_x 时用对数间隔采样。对数窗必须为正；无正样本明确失败。非正曲线点拆段，不能跨过剔除域重新连接；剔除的点/图元计入 skipped。网格/向量仍按线性物理域采样，因此对数显示不意味着已做自适应对数网格。

固定 color 支持 green/blue/red/orange/purple/cyan 或 #RRGGBB；密度和热图坚持真实值调色板，覆盖请求明确拒绝；颜色函数留 R3.6。未知选项、重复参数、错误轴/形状/箱数/精度给诊断，不默默忽略。

## 基础界面与生命周期

React 用 SVG，SwiftUI 用 Canvas/Path。两端保留按钮放大/缩小/复位、手势平移/缩放、内核采样数据与原式；Web有键盘方向键/+/−/Home，原 Swift 无障碍缩放/平移操作继续可用。数据面板按需展开，不为初始页面拼接大型文本；有限网格注明采样近似和跳过数量，真实密度颜色来自 producer。

沿用现有请求代次/liveness/串行采样最新尾请求，过期单元格禁用重采样；迟到的回复不覆盖新图或已销毁视图。本批不新增伪动画百分比；静态基础路径独立完整。主笔记本计算、文件保存与 AI 生命周期保持原设施，新的参数隔离探索另在 R3.5c 接入。

UI依据为用户指定本地 `telegram-ui-reference` 的 portable 基础路径：

- `/Users/hert/Documents/ChatGPT/ui-learning/08-media-and-rich-content/media-browsing-and-zoom.md`：相机坐标和锚点、可见操作、键盘/手势共用状态，数学域与媒体尺寸的映射由本项目定义。
- `/Users/hert/Documents/ChatGPT/ui-learning/06-state-and-feedback/progress-and-result-feedback.md`：真实 producer 结果、代次和迟到守卫，取消不能伪造完成。
- `/Users/hert/Documents/ChatGPT/ui-learning/09-adaptation-and-accessibility/system-fonts-and-long-text.md`：原生文字缩放一次、长数据可滚动、窄宽操作保留。

参考算法不是执行证据；不复制 Telegram 类体系或读原源码归档。真实测试、截图、命令和未执行平台项单列验收页。

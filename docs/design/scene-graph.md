# 有限场景图元、样式与变换

[三维采样](scene3d.md) · [可运行案例](../examples/README.md) · [接口目录](../reference/scene.md)

这是 `.3` 的真实实现范围。`scene` 容器组合点、线、箭头、圆、圆盘、简单多边形、球、椭球、盒、圆柱、圆锥、管道和文字；`translate`、`rotate`、`scale`、`style` 可以嵌套或作用于列表。二维使用同一内核的 XY 几何并由 Web/桌面 SVG、iOS Canvas 展示；三维使用独立世界网格和 WebGL2。图元构造器保留源码，生成几何发生在容器输出/采样阶段，不把保留的源码称为已经完成绘图。

```om
scene([point([0,0]),label("原点",[0,0]),
  circle([0,0],1) |> style(color:"green",opacity:0.8),
  translate(rotate(line([[0,0],[1,0]]),pi/2),[2,3])])
scene([sphere([0,0,0],1),translate(ellipsoid([0,0,0],[1,2,1]),[3,0,0])],dimensions:3)
blend(["dark_green","light_green"],0.5)
```

`dimensions` 默认 2，仅接受 2/3；`mesh_points` 默认 32，范围 8..64。三维球/椭球/圆柱/圆锥/管道只在三维容器内生成。`point`/`line` 默认蓝，面默认绿；`color` 支持固定颜色名、`#RRGGBB`、RGB/RGBA 列表及只读 `fn(position)`。固定颜色名为 green、blue、red、orange、purple、cyan、black、white、dark_green、light_green。`style` 从父容器继承样式，子节点显式选项覆盖继承值；`opacity` 与颜色 alpha 相乘。颜色计算发生在变换前的局部坐标中；导出保存最后实际世界坐标与颜色。

`blend` 首版对恰好两个颜色按 0..1 权重逐分量插值，返回四个机器实数。它不进行光学线性化，也不暗中扩展为任意多色/高精度插值。非法颜色、分量、权重和不支持的精度明确失败。

`rotate(node,angle,axis:[0,0,1],center:...)` 用弧度与右手规则；二维轴固定垂直 XY，三维轴必须非零。`scale` 支持标量或逐轴向量；中心默认原点，负缩放实际反转面绕序，法线由世界三角形重新计算。零缩放产生真实退化面、线或点，不能生成假小球或非有限法线。圆盘支持三维 `normal`；管道沿有限路径传输局部框架，拒绝重复段和完全折返。简单多边形通过耳切三角化，拒绝自交、重叠、重复相邻顶点和非共面数据。

`width` 是逻辑像素，仅用于点、线、箭头、圆；文字偏移 `offset` 同样是显示像素，均不改变数学位置。标签为纯 Unicode 文本，最多 1024 UTF-8 字节，无控制字符，不能解释为 HTML 或可执行代码。2D 保留独立的 paths/polygons/markers/labels 可选字段，旧二维回复不必携带它们。三维标签为实际世界位置的文字覆盖；相机操作不修改世界坐标。

深度最多 32、遍历节点最多 1024、路径最多 2048 点、整个场景最多 200000 顶点。组合已有采样网格时，在分配展开三角形前检查总量；遍历、颜色、三角化、变换和导出共享中断预算。错误/取消不给部分场景伪造成功。几何输入/颜色/变换在只读会话中执行，不能推进主随机状态或写定义。空场景或不可表示尺度明确失败。

内部 InputForm 回读新增 `parse_input_form`：保持现代下划线符号身份，Pattern/Blank 仍为显式表达式头。它是表达式传输读取器，不新增源码方言；用户 Wolfram `x_` 模式规则不变。Auto 遇到明确 `let`、lambda、管道、范围先选择 Modern，避免将 `fn(p)=>p[1]` 错判为 Wolfram。源码 `.omnb` v1 不改变，也不自动改写旧文件。

iOS 对 `dimensions:3` 保留原式与明确未适配提示，不自动生成未展示的大网格。二维容器使用真实 Canvas、标签和显示操作。三维 OBJ 导出复用已有实际几何与字节回读契约，不重新执行源式。

实现/独立几何与失败验收：`crates/om-kernel/tests/scene_graph.rs`、`crates/om-format/tests/serialized_symbols.rs`、`crates/om-parse/tests/composition.rs`、`app/e2e/science-scenarios.spec.ts`；Swift 的原生协议/FFI验收在 `ios/OpenMathTests/SceneTests.swift`，实跑交 GitHub CI。

界面依据为本地 [媒体浏览与缩放](/Users/hert/Documents/ChatGPT/ui-learning/08-media-and-rich-content/media-browsing-and-zoom.md) 的真实几何/具名操作/生命周期/归一中心，以及 [窗口尺寸与方向](/Users/hert/Documents/ChatGPT/ui-learning/09-adaptation-and-accessibility/screen-size-and-orientation.md) 的可用宽度与状态保留。映射到已有 SVG/Canvas/WebGL2 宿主；没有复制 Telegram 类体系、构建其源码或把手推示例当作执行证据。

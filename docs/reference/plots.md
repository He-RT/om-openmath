<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# 函数与数据绘图

[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)

所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。

| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |
|---|---|---|---|
| [`array_plot`](#array_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ArrayPlot 能力，进入后续全景目录。 |
| [`bar_chart`](#bar_chart) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 BarChart 能力，进入后续全景目录。 |
| [`box_whisker_chart`](#box_whisker_chart) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 BoxWhiskerChart 能力，进入后续全景目录。 |
| [`bubble_chart`](#bubble_chart) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 BubbleChart 能力，进入后续全景目录。 |
| [`data_plot`](#data_plot) | 已实现 | 当前可用 | 数据散点、连线或热图 |
| [`discrete_plot`](#discrete_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 DiscretePlot 能力，进入后续全景目录。 |
| [`discrete_plot3_d`](#discrete_plot3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 DiscretePlot3D 能力，进入后续全景目录。 |
| [`field_plot`](#field_plot) | 已实现 | 当前可用 | 二维向量场或流线 |
| [`histogram`](#histogram) | 已实现 | 当前可用 | 频数直方图 |
| [`histogram3_d`](#histogram3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 Histogram3D 能力，进入后续全景目录。 |
| [`implicit_plot`](#implicit_plot) | 下一版规划 | 规划接口，当前不可用 | 隐式等值曲线或曲面 |
| [`list_contour_plot`](#list_contour_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ListContourPlot 能力，进入后续全景目录。 |
| [`list_density_plot`](#list_density_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ListDensityPlot 能力，进入后续全景目录。 |
| [`list_plot3_d`](#list_plot3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ListPlot3D 能力，进入后续全景目录。 |
| [`list_point_plot3_d`](#list_point_plot3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ListPointPlot3D 能力，进入后续全景目录。 |
| [`list_stream_plot`](#list_stream_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ListStreamPlot 能力，进入后续全景目录。 |
| [`list_vector_plot`](#list_vector_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ListVectorPlot 能力，进入后续全景目录。 |
| [`matrix_plot`](#matrix_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 MatrixPlot 能力，进入后续全景目录。 |
| [`parametric_plot`](#parametric_plot) | 部分支持 | 当前可用 | 参数曲线或曲面 |
| [`pie_chart`](#pie_chart) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 PieChart 能力，进入后续全景目录。 |
| [`plot`](#plot) | 部分支持 | 现有入口可用，统一接口待实施 | 采样一元实函数图像。；采样二元隐函数等值轮廓。 |
| [`polar_plot`](#polar_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 PolarPlot 能力，进入后续全景目录。 |
| [`probability_plot`](#probability_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 ProbabilityPlot 能力，进入后续全景目录。 |
| [`quantile_plot`](#quantile_plot) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 QuantilePlot 能力，进入后续全景目录。 |
| [`radar_chart`](#radar_chart) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 RadarChart 能力，进入后续全景目录。 |
| [`region_plot`](#region_plot) | 已实现 | 当前可用 | 二维不等式区域 |
| [`region_plot3_d`](#region_plot3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 RegionPlot3D 能力，进入后续全景目录。 |
| [`smooth_histogram`](#smooth_histogram) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 SmoothHistogram 能力，进入后续全景目录。 |
| [`stream_plot3_d`](#stream_plot3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 StreamPlot3D 能力，进入后续全景目录。 |
| [`vector_plot3_d`](#vector_plot3_d) | 后续规划 | 规划接口，当前不可用 | 函数与数据绘图中的 VectorPlot3D 能力，进入后续全景目录。 |

## array_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000520`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ArrayPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ArrayPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
array_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## bar_chart

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000516`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 BarChart 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`BarChart`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
bar_chart(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## box_whisker_chart

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000513`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 BoxWhiskerChart 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`BoxWhiskerChart`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
box_whisker_chart(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## bubble_chart

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000518`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 BubbleChart 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`BubbleChart`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
bubble_chart(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## data_plot

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000223`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

数据散点、连线或热图

- 当前支持：≤100000有限机器标量的[x,y]列表或x/y表格，散点/原输入顺序连线；矩形热图用真实值/内核调色板。热图拒绝固定color覆盖。 输入/样本受Interrupt预算，readonly/取消不修改会话。精确输入可机器采样，新增路径拒绝高精度数静默降级；传统Plot/ContourPlot保留机器采样兼容语义，不宣称高精度图形；有限采样不能认证数学边界。
- 目标范围：二维显示在桌面/Web/iOS；内核采样、保留孔洞、预算和旧请求隔离。
- 返回：plot_or_scene_request
- 精度：真实只读机器采样，资源/非有限/不支持模式有诊断，不是认证数学边界。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：`DataPlot`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
data_plot(data,kind:"scatter")
```

当前 Wolfram 签名：

```text
DataPlot[data]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
data_plot(data, kind: "scatter")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `data` | positional | 必填 | 有序输入数据 | current |
| `kind` | option | 签名规定 | 相应视图/样式/箱数 | current |

当前已登记示例（Wolfram）：

```wolfram
DataPlot[{{0,0},{1,1},{2,4}}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/plot_registry.rs](../../crates/om-eval/src/plot_registry.rs)、[crates/om-kernel/src/plot/extended.rs](../../crates/om-kernel/src/plot/extended.rs)。

当前测试引用：[crates/om-kernel/tests/plot_extensions.rs](../../crates/om-kernel/tests/plot_extensions.rs)、[crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[app/src/components/plot/PlotView.test.tsx](../../app/src/components/plot/PlotView.test.tsx)、[ios/OpenMathTests/PlotExtensionTests.swift](../../ios/OpenMathTests/PlotExtensionTests.swift)。

## discrete_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000505`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 DiscretePlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DiscretePlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
discrete_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## discrete_plot3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000506`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 DiscretePlot3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DiscretePlot3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
discrete_plot3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## field_plot

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000222`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

二维向量场或流线

- 当前支持：20×20真实向量箭头/共同最大范数视觉归一，原始值保留；stream按归一方向RK4双向≤240步/64种子，轨线不保证全部拓扑或原时间速度。 输入/样本受Interrupt预算，readonly/取消不修改会话。精确输入可机器采样，新增路径拒绝高精度数静默降级；传统Plot/ContourPlot保留机器采样兼容语义，不宣称高精度图形；有限采样不能认证数学边界。
- 目标范围：二维显示在桌面/Web/iOS；内核采样、保留孔洞、预算和旧请求隔离。
- 返回：plot_or_scene_request
- 精度：真实只读机器采样，资源/非有限/不支持模式有诊断，不是认证数学边界。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：`FieldPlot`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
field_plot(vector,x:a..b,y:c..d,view:"arrows")
```

当前 Wolfram 签名：

```text
FieldPlot[vector,{x,a,b},{y,c,d}]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
field_plot(vector, ...axes, view: "arrows")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `vector` | positional | 必填 | 二维向量表达式 | current |
| `axes` | axis | 必填 | 命名范围 | current |
| `view` | option | line（当前二维）；三维surface尚未实现 | arrows/stream | current |

当前已登记示例（Wolfram）：

```wolfram
FieldPlot[{-y,x},{x,-2,2},{y,-2,2}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/plot_registry.rs](../../crates/om-eval/src/plot_registry.rs)、[crates/om-kernel/src/plot/extended.rs](../../crates/om-kernel/src/plot/extended.rs)。

当前测试引用：[crates/om-kernel/tests/plot_extensions.rs](../../crates/om-kernel/tests/plot_extensions.rs)、[crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[app/src/components/plot/PlotView.test.tsx](../../app/src/components/plot/PlotView.test.tsx)、[ios/OpenMathTests/PlotExtensionTests.swift](../../ios/OpenMathTests/PlotExtensionTests.swift)。

## histogram

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000224`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

频数直方图

- 当前支持：≤100000有限机器样本，1..200等宽频数箱（默认20），最大值入最后箱；所有计数守恒；常量样本使用显式非零箱域。 输入/样本受Interrupt预算，readonly/取消不修改会话。精确输入可机器采样，新增路径拒绝高精度数静默降级；传统Plot/ContourPlot保留机器采样兼容语义，不宣称高精度图形；有限采样不能认证数学边界。
- 目标范围：二维显示在桌面/Web/iOS；内核采样、保留孔洞、预算和旧请求隔离。
- 返回：plot_or_scene_request
- 精度：真实只读机器采样，资源/非有限/不支持模式有诊断，不是认证数学边界。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：`Histogram`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
histogram(data,bins:20)
```

当前 Wolfram 签名：

```text
Histogram[data]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
histogram(data, bins: 20)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `data` | positional | 必填 | 有限实数列表 | current |
| `bins` | option | 20 | 正整数箱数 | current |

当前已登记示例（Wolfram）：

```wolfram
Histogram[{1,1,2,3,3}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/plot_registry.rs](../../crates/om-eval/src/plot_registry.rs)、[crates/om-kernel/src/plot/extended.rs](../../crates/om-kernel/src/plot/extended.rs)。

当前测试引用：[crates/om-kernel/tests/plot_extensions.rs](../../crates/om-kernel/tests/plot_extensions.rs)、[crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[app/src/components/plot/PlotView.test.tsx](../../app/src/components/plot/PlotView.test.tsx)、[ios/OpenMathTests/PlotExtensionTests.swift](../../ios/OpenMathTests/PlotExtensionTests.swift)。

## histogram3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000511`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 Histogram3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Histogram3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
histogram3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## implicit_plot

**当前实现：下一版规划；目标接口：规划接口，当前不可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000220`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

隐式等值曲线或曲面

- 当前支持：本独立三维扩展身份未实现；当前二维implicit_plot沿用稳定fn_000119/ContourPlot入口，可用levels/log选项，详见plot条目。
- 目标范围：二维显示在桌面/Web/iOS；内核采样、保留孔洞、预算和旧请求隔离。 二维沿用轮廓算法；三维采用确定性网格及marching tetrahedra，奇点不连接。
- 返回：plot_or_scene_request
- 精度：真实只读机器采样，资源/非有限/不支持模式有诊断，不是认证数学边界。
- 当前计算平台：无；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
implicit_plot(equation, ...axes)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `equation` | positional | 必填 | 签名对应的equation参数 | r3 |
| `axes` | axis | 数据图可省略 | 所需命名轴 | r3 |

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
implicit_plot(x^2+y^2+z^2=1,x:-2..2,y:-2..2,z:-2..2)
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## list_contour_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000509`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ListContourPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ListContourPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
list_contour_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## list_density_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000510`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ListDensityPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ListDensityPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
list_density_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## list_plot3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000507`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ListPlot3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ListPlot3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
list_plot3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## list_point_plot3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000508`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ListPointPlot3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ListPointPlot3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
list_point_plot3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## list_stream_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000523`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ListStreamPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ListStreamPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
list_stream_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## list_vector_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000522`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ListVectorPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ListVectorPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
list_vector_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## matrix_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000521`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 MatrixPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`MatrixPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
matrix_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## parametric_plot

**当前实现：部分支持；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000219`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

参数曲线或曲面

- 当前支持：一参数二维两坐标机器曲线，800基础点加中点域检查；参数范围独立于相机；非有限/可识别跳变断开，采样不能发现全部奇点；三维曲线/曲面尚待R3.6。 输入/样本受Interrupt预算，readonly/取消不修改会话。精确输入可机器采样，新增路径拒绝高精度数静默降级；传统Plot/ContourPlot保留机器采样兼容语义，不宣称高精度图形；有限采样不能认证数学边界。
- 目标范围：二维显示在桌面/Web/iOS；内核采样、保留孔洞、预算和旧请求隔离。 一参数二维/三维曲线；两参数三维曲面；三维显示仅桌面/Web。
- 返回：plot_or_scene_request
- 精度：真实只读机器采样，资源/非有限/不支持模式有诊断，不是认证数学边界。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：`ParametricPlot`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
parametric_plot(vector,t:a..b)
```

当前 Wolfram 签名：

```text
ParametricPlot[vector,{t,a,b}]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
parametric_plot(vector, ...axes)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `vector` | positional | 必填 | 二维/三维坐标向量 | current |
| `axes` | axis | 必填 | 一或两个命名范围 | current |
| `color` | option | 主题色 | 固定颜色名或#RRGGBB；密度/热图拒绝覆盖；函数颜色待R3.6 | current |

当前已登记示例（Wolfram）：

```wolfram
ParametricPlot[{Cos[t],Sin[t]},{t,0,2*Pi}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
parametric_plot([cos(t),sin(t),t],t:0..2*pi)
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/plot_registry.rs](../../crates/om-eval/src/plot_registry.rs)、[crates/om-kernel/src/plot/extended.rs](../../crates/om-kernel/src/plot/extended.rs)。

当前测试引用：[crates/om-kernel/tests/plot_extensions.rs](../../crates/om-kernel/tests/plot_extensions.rs)、[crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[app/src/components/plot/PlotView.test.tsx](../../app/src/components/plot/PlotView.test.tsx)、[ios/OpenMathTests/PlotExtensionTests.swift](../../ios/OpenMathTests/PlotExtensionTests.swift)。

## pie_chart

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000517`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 PieChart 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`PieChart`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
pie_chart(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## plot

**当前实现：部分支持；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000119`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

采样一元实函数图像。；采样二元隐函数等值轮廓。

- 当前支持：二维实函数/标量等高线/96×96密度真实机器采样；log_x/log_y/log_log正窗口；固定color仅曲线。三维surface尚待R3.6。 输入/样本受Interrupt预算，readonly/取消不修改会话。精确输入可机器采样，新增路径拒绝高精度数静默降级；传统Plot/ContourPlot保留机器采样兼容语义，不宣称高精度图形；有限采样不能认证数学边界。
- 目标范围：统一二维/曲面视图、对数坐标、采样颜色和样式；三维仅桌面/Web可交互展示。
- 返回：plot_or_scene_request
- 精度：现有绘图经过只读 f64 编译与真实内核采样；符号输入可精确保存，像素/采样不是认证解。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：`Plot`、`ContourPlot`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
plot(expr,x:a..b)
plot(expr,x:a..b,y:c..d,view:"contour")
plot(expr,x:a..b,y:c..d,view:"density")
```

当前 Wolfram 签名：

```text
Plot[expr,{x,min,max}]
ContourPlot[eq,{x,min,max},{y,min,max}]
DensityPlot[expr,{x,a,b},{y,c,d}]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
plot(expr, x: lo..hi, y: lo..hi, view: "line")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 标量或曲线列表 | current |
| `axes` | axis | 必填 | 一个或两个命名轴 | current |
| `view` | option | line（当前二维）；三维surface尚未实现 | line/surface/density/contour | current |
| `scale` | option | linear | linear/log_x/log_y/log_log | current |
| `color` | option | 主题色 | 固定颜色名或#RRGGBB；密度/热图拒绝覆盖；函数颜色待R3.6 | current |
| `precision` | option | machine | 采样精度 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Plot[Sin[x],{x,0,2*Pi}]
ContourPlot[x^2+y^2==1,{x,-2,2},{y,-2,2}]
DensityPlot[x*y,{x,-2,2},{y,-2,2}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
plot(sin(x), x: -pi..pi)
plot(sin(x)*cos(y), x: -pi..pi, y: -pi..pi, view: "surface")
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/plot_registry.rs](../../crates/om-eval/src/plot_registry.rs)。

当前测试引用：[crates/om-kernel/tests/plot_extensions.rs](../../crates/om-kernel/tests/plot_extensions.rs)、[crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[app/src/components/plot/PlotView.test.tsx](../../app/src/components/plot/PlotView.test.tsx)、[ios/OpenMathTests/PlotExtensionTests.swift](../../ios/OpenMathTests/PlotExtensionTests.swift)。

## polar_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000504`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 PolarPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`PolarPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
polar_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## probability_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000515`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 ProbabilityPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ProbabilityPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
probability_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## quantile_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000514`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 QuantilePlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`QuantilePlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
quantile_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## radar_chart

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000519`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 RadarChart 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`RadarChart`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
radar_chart(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## region_plot

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000221`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

二维不等式区域

- 当前支持：二维实数比较和and/or/not；96×96中点布尔网格，非有限样本跳过且计数；近似填充不保证精确边界/窄区域。 输入/样本受Interrupt预算，readonly/取消不修改会话。精确输入可机器采样，新增路径拒绝高精度数静默降级；传统Plot/ContourPlot保留机器采样兼容语义，不宣称高精度图形；有限采样不能认证数学边界。
- 目标范围：二维显示在桌面/Web/iOS；内核采样、保留孔洞、预算和旧请求隔离。
- 返回：plot_or_scene_request
- 精度：真实只读机器采样，资源/非有限/不支持模式有诊断，不是认证数学边界。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：desktop, web, ios。
- 兼容名称：`RegionPlot`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
region_plot(condition,x:a..b,y:c..d)
```

当前 Wolfram 签名：

```text
RegionPlot[condition,{x,a,b},{y,c,d}]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
region_plot(condition, x: a..b, y: c..d)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `condition` | positional | 必填 | 签名对应的condition参数 | current |
| `x` | axis | 必填 | 签名对应的x参数 | current |
| `y` | axis | c..d | 签名对应的y参数 | current |

当前已登记示例（Wolfram）：

```wolfram
RegionPlot[x^2+y^2<1,{x,-2,2},{y,-2,2}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/plot_registry.rs](../../crates/om-eval/src/plot_registry.rs)、[crates/om-kernel/src/plot/extended.rs](../../crates/om-kernel/src/plot/extended.rs)。

当前测试引用：[crates/om-kernel/tests/plot_extensions.rs](../../crates/om-kernel/tests/plot_extensions.rs)、[crates/om-kernel/tests/plot_sampling.rs](../../crates/om-kernel/tests/plot_sampling.rs)、[app/src/components/plot/PlotView.test.tsx](../../app/src/components/plot/PlotView.test.tsx)、[ios/OpenMathTests/PlotExtensionTests.swift](../../ios/OpenMathTests/PlotExtensionTests.swift)。

## region_plot3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000524`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 RegionPlot3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`RegionPlot3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
region_plot3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## smooth_histogram

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000512`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 SmoothHistogram 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SmoothHistogram`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
smooth_histogram(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## stream_plot3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000526`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 StreamPlot3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`StreamPlot3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
stream_plot3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## vector_plot3_d

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000525`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

函数与数据绘图中的 VectorPlot3D 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`VectorPlot3D`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
vector_plot3_d(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# ODE、优化与拟合

[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)

所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。

| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |
|---|---|---|---|
| [`bode_plot`](#bode_plot) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 BodePlot 能力，进入后续全景目录。 |
| [`dirichlet_condition`](#dirichlet_condition) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 DirichletCondition 能力，进入后续全景目录。 |
| [`feedback_connect`](#feedback_connect) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 FeedbackConnect 能力，进入后续全景目录。 |
| [`fit`](#fit) | 下一版规划 | 规划接口，当前不可用 | 参数拟合及真实残差 |
| [`interpolate`](#interpolate) | 已实现 | 当前可用 | 从有序样本构造插值函数 |
| [`neumann_value`](#neumann_value) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 NeumannValue 能力，进入后续全景目录。 |
| [`nyquist_plot`](#nyquist_plot) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 NyquistPlot 能力，进入后续全景目录。 |
| [`ode`](#ode) | 已实现 | 当前可用 | 非刚性常微分方程初值数值解 |
| [`optimize`](#optimize) | 下一版规划 | 规划接口，当前不可用 | 带明确最优性保证的优化 |
| [`output_response`](#output_response) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 OutputResponse 能力，进入后续全景目录。 |
| [`parametric_nd_solve`](#parametric_nd_solve) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 ParametricNDSolve 能力，进入后续全景目录。 |
| [`parametric_nd_solve_value`](#parametric_nd_solve_value) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 ParametricNDSolveValue 能力，进入后续全景目录。 |
| [`root_locus_plot`](#root_locus_plot) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 RootLocusPlot 能力，进入后续全景目录。 |
| [`sample`](#sample) | 已实现 | 当前可用 | 对函数或插值对象取真实样本 |
| [`state_space_model`](#state_space_model) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 StateSpaceModel 能力，进入后续全景目录。 |
| [`system_model`](#system_model) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 SystemModel 能力，进入后续全景目录。 |
| [`system_model_simulate`](#system_model_simulate) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 SystemModelSimulate 能力，进入后续全景目录。 |
| [`transfer_function_model`](#transfer_function_model) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 TransferFunctionModel 能力，进入后续全景目录。 |
| [`when_event`](#when_event) | 后续规划 | 规划接口，当前不可用 | ODE、优化与拟合中的 WhenEvent 能力，进入后续全景目录。 |

## bode_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000378`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 BodePlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`BodePlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
bode_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## dirichlet_condition

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000373`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 DirichletCondition 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DirichletCondition`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
dirichlet_condition(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## feedback_connect

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000382`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 FeedbackConnect 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`FeedbackConnect`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
feedback_connect(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## fit

**当前实现：下一版规划；目标接口：规划接口，当前不可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000180`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

参数拟合及真实残差

- 当前支持：当前无此规范接口的实现。
- 目标范围：线性QR最小二乘、非线性Levenberg–Marquardt，≤16参数；不捏造统计置信区间。
- 返回：model_with_diagnostics
- 精度：机器精度路径优先；不声称任意精度。
- 当前计算平台：无；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Fit`、`FindFit`、`LinearModelFit`、`NonlinearModelFit`。
- 管道位置：第 1 个位置参数（从 1 起）。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
fit(data, model: expression, parameters: starts, method: "linear")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `data` | positional | 必填 | 样本 | r3 |
| `model` | option | 必填 | 表达式 | r3 |
| `parameters` | option | 必填 | 参数及初始值 | r3 |
| `method` | option | linear | linear/nonlinear | r3 |

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
fit([[0,1],[1,3],[2,5]],model: a*x+b,parameters: {a: 1,b: 0},method: "linear")
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## interpolate

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000177`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

从有序样本构造插值函数

- 当前支持：有限域scalar/vector节点[x,value]；Hermite可[x,value,slope]，缺导数为有限差分估计并保存estimated来源。严格单调正反节点、维度≤64、100000存储标量、默认拒绝外推；受保护held InterpolationData仅保存真实节点/系数/方法/返回形状/导数来源，取值重新验证，伪造执行表达式拒绝。Wolfram Interpolation/ListInterpolation未声明语义等价，暂不映射。
- 目标范围：线性与分段三次Hermite；时间坐标严格单调；默认拒绝外推。
- 返回：interpolation
- 精度：仅机器实数算法；精确有限常量显式数值化，高精度输入/precision不静默降级。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
interpolate(points, method: "linear")
```

当前 Wolfram 签名：

```text
Interpolate[points,Method->"linear"]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
interpolate(points, method: "linear")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `points` | positional | 必填 | 有序坐标/值列表 | r3 |
| `method` | option | linear | linear/hermite | r3 |

当前已登记示例（Wolfram）：

```wolfram
interpolate([[0,0],[1,2],[2,4]])(0.25)
interpolate([[0,0,0],[1,1,3]], method:"hermite")(0.37)
Interpolate[{{0,0},{1,2},{2,4}}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/ode_registry.rs](../../crates/om-eval/src/science/ode_registry.rs)、[crates/om-eval/src/science/ode.rs](../../crates/om-eval/src/science/ode.rs)、[crates/om-analysis/src/ode.rs](../../crates/om-analysis/src/ode.rs)、[crates/om-analysis/src/interpolation.rs](../../crates/om-analysis/src/interpolation.rs)。

当前测试引用：[crates/om-eval/tests/ode.rs](../../crates/om-eval/tests/ode.rs)、[crates/om-analysis/tests/ode.rs](../../crates/om-analysis/tests/ode.rs)、[crates/om-analysis/tests/interpolation.rs](../../crates/om-analysis/tests/interpolation.rs)。

## neumann_value

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000374`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 NeumannValue 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`NeumannValue`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
neumann_value(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## nyquist_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000379`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 NyquistPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`NyquistPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
nyquist_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## ode

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000176`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

非刚性常微分方程初值数值解

- 当前支持：非刚性机器实数1..64维初值；DP5(4)自适应正反时间、四次连续输出、端点零或符号变化事件。返回solution/domain/converged/termination/accepted_steps/rejected_steps/evaluations/event_evaluations/abs_tol/rel_tol/method，实际取值保持标量/向量形状。readonly原式编译不约消极点；失败报告最后实际节点/状态/工作，不伪造收敛。初始步默认跨度1%，可initial_step/max_step；最多100000存储标量。触碰零点、步内多交叉、刚性/DAE/PDE不保证；NDSolve/NDSolveValue方程语法未适配，不能作简单别名。
- 目标范围：机器实数状态≤64维；Dormand–Prince5(4)、连续插值、零交叉终止事件；首版不含刚性、DAE、PDE。
- 返回：interpolation_with_diagnostics
- 精度：仅机器实数算法；精确有限常量显式数值化，高精度输入/precision不静默降级。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

当前现代签名：

```text
ode(rhs, initial: values, t: start..end, abs_tol: 1e-10, rel_tol: 1e-8, max_steps: 100000, event: fn(t,y)=>expr)
ode(rhs, [t,start,end], initial: values)
```

当前 Wolfram 签名：

```text
Ode[rhs,{t,start,end},Initial->values]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
ode(fn(t, state) => rhs, initial: values, t: start..end)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `rhs` | positional | 必填 | 只读右端函数 | r3 |
| `initial` | option | 必填 | 初值 | r3 |
| `t` | axis | 必填 | 有方向时间范围 | r3 |
| `abs_tol` | option | 1e-10 | 绝对容差 | r3 |
| `rel_tol` | option | 1e-8 | 相对容差 | r3 |
| `max_steps` | option | 100000 | 步数上限 | r3 |
| `event` | option | none | 零交叉终止函数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
let motion=ode(fn(t,y)=>[y[2],-y[1]], initial:[1,0], t:0..6); motion.solution(1.25)
Ode[Function[{t,y},y],{t,0,1},Initial->1]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/ode_registry.rs](../../crates/om-eval/src/science/ode_registry.rs)、[crates/om-eval/src/science/ode.rs](../../crates/om-eval/src/science/ode.rs)、[crates/om-analysis/src/ode.rs](../../crates/om-analysis/src/ode.rs)、[crates/om-analysis/src/interpolation.rs](../../crates/om-analysis/src/interpolation.rs)。

当前测试引用：[crates/om-eval/tests/ode.rs](../../crates/om-eval/tests/ode.rs)、[crates/om-analysis/tests/ode.rs](../../crates/om-analysis/tests/ode.rs)、[crates/om-analysis/tests/interpolation.rs](../../crates/om-analysis/tests/interpolation.rs)。

## optimize

**当前实现：下一版规划；目标接口：规划接口，当前不可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000179`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

带明确最优性保证的优化

- 当前支持：当前无此规范接口的实现。
- 目标范围：机器一维有界Brent或多变量BFGS/盒约束投影；scope global 仅支持可认证凸二次问题，其他请求明确拒绝。
- 返回：optimum_with_diagnostics
- 精度：机器精度路径优先；不声称任意精度。
- 当前计算平台：无；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`FindMinimum`、`FindMaximum`、`Minimize`、`Maximize`。
- 管道位置：第 1 个位置参数（从 1 起）。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
optimize(expr, variables, goal: "min", scope: "local", initial: values)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 目标表达式 | r3 |
| `variables` | positional | 必填 | 变量 | r3 |
| `goal` | option | min | min/max | r3 |
| `scope` | option | local | local/global | r3 |
| `initial` | option | 局部模式必填 | 局部起点 | r3 |

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
optimize((x-2)^2, x, initial: 0, goal: "min", scope: "local")
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## output_response

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000381`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 OutputResponse 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`OutputResponse`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
output_response(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## parametric_nd_solve

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000371`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 ParametricNDSolve 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ParametricNDSolve`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
parametric_nd_solve(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## parametric_nd_solve_value

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000372`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 ParametricNDSolveValue 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ParametricNDSolveValue`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
parametric_nd_solve_value(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## root_locus_plot

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000380`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 RootLocusPlot 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`RootLocusPlot`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
root_locus_plot(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## sample

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000178`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

对函数或插值对象取真实样本

- 当前支持：真实只读scalar/vector函数或InterpolationData采样，非退化正反有限域、count=2..10000，返回DataTable列x/value；总标量≤100000。机器重复节点/非有限值/高精度/写入拒绝；外推和缺失不补零。专门表格展示留R3.5。
- 目标范围：有限有序区间、count≥2；不把外推或缺失结果补零。
- 返回：table
- 精度：仅机器实数算法；精确有限常量显式数值化，高精度输入/precision不静默降级。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
sample(fn, x: a..b, count: 100)
```

当前 Wolfram 签名：

```text
Sample[fn,{x,a,b},Count->100]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
sample(fn, x: a..b, count: 100)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `fn` | positional | 必填 | 可调用对象 | r3 |
| `x` | axis | 必填 | 样本范围 | r3 |
| `count` | option | 100 | 采样数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
sample(fn(x)=>x^2, x:0..1, count:5)
Sample[Function[x,x^2],{x,0,1},Count->5]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/ode_registry.rs](../../crates/om-eval/src/science/ode_registry.rs)、[crates/om-eval/src/science/ode.rs](../../crates/om-eval/src/science/ode.rs)、[crates/om-analysis/src/ode.rs](../../crates/om-analysis/src/ode.rs)、[crates/om-analysis/src/interpolation.rs](../../crates/om-analysis/src/interpolation.rs)。

当前测试引用：[crates/om-eval/tests/ode.rs](../../crates/om-eval/tests/ode.rs)、[crates/om-analysis/tests/ode.rs](../../crates/om-analysis/tests/ode.rs)、[crates/om-analysis/tests/interpolation.rs](../../crates/om-analysis/tests/interpolation.rs)。

## state_space_model

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000377`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 StateSpaceModel 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`StateSpaceModel`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
state_space_model(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## system_model

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000383`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 SystemModel 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SystemModel`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
system_model(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## system_model_simulate

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000384`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 SystemModelSimulate 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SystemModelSimulate`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
system_model_simulate(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## transfer_function_model

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000376`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 TransferFunctionModel 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TransferFunctionModel`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
transfer_function_model(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## when_event

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000375`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

ODE、优化与拟合中的 WhenEvent 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`WhenEvent`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
when_event(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

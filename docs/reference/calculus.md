<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# 微积分与变换

[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)

所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。

| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |
|---|---|---|---|
| [`asymptotic`](#asymptotic) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 Asymptotic 能力，进入后续全景目录。 |
| [`asymptotic_integrate`](#asymptotic_integrate) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 AsymptoticIntegrate 能力，进入后续全景目录。 |
| [`asymptotic_sum`](#asymptotic_sum) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 AsymptoticSum 能力，进入后续全景目录。 |
| [`curl`](#curl) | 已实现 | 当前可用 | 三维笛卡尔旋度 |
| [`d_solve`](#d_solve) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 DSolve 能力，进入后续全景目录。 |
| [`d_solve_value`](#d_solve_value) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 DSolveValue 能力，进入后续全景目录。 |
| [`diff`](#diff) | 已实现 | 现有入口可用，统一接口待实施 | 按链式法则求重复或混合偏导数。 |
| [`difference_delta`](#difference_delta) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 DifferenceDelta 能力，进入后续全景目录。 |
| [`discrete_limit`](#discrete_limit) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 DiscreteLimit 能力，进入后续全景目录。 |
| [`div`](#div) | 已实现 | 当前可用 | 笛卡尔散度 |
| [`dt`](#dt) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 Dt 能力，进入后续全景目录。 |
| [`fourier_transform`](#fourier_transform) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 FourierTransform 能力，进入后续全景目录。 |
| [`generating_function`](#generating_function) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 GeneratingFunction 能力，进入后续全景目录。 |
| [`grad`](#grad) | 已实现 | 当前可用 | 笛卡尔梯度 |
| [`hessian`](#hessian) | 已实现 | 当前可用 | Hessian 矩阵 |
| [`integrate`](#integrate) | 已实现 | 当前可用 | 符号或显式数值积分 |
| [`inverse_fourier_transform`](#inverse_fourier_transform) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 InverseFourierTransform 能力，进入后续全景目录。 |
| [`inverse_laplace_transform`](#inverse_laplace_transform) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 InverseLaplaceTransform 能力，进入后续全景目录。 |
| [`inverse_z_transform`](#inverse_z_transform) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 InverseZTransform 能力，进入后续全景目录。 |
| [`jacobian`](#jacobian) | 已实现 | 当前可用 | Jacobian 矩阵 |
| [`laplace_transform`](#laplace_transform) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 LaplaceTransform 能力，进入后续全景目录。 |
| [`laplacian`](#laplacian) | 已实现 | 当前可用 | 笛卡尔拉普拉斯 |
| [`limit`](#limit) | 已实现 | 当前可用 | 计算有限点或无穷处极限 |
| [`n_product`](#n_product) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 NProduct 能力，进入后续全景目录。 |
| [`n_sum`](#n_sum) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 NSum 能力，进入后续全景目录。 |
| [`pade_approximant`](#pade_approximant) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 PadeApproximant 能力，进入后续全景目录。 |
| [`r_solve`](#r_solve) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 RSolve 能力，进入后续全景目录。 |
| [`r_solve_value`](#r_solve_value) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 RSolveValue 能力，进入后续全景目录。 |
| [`recurrence_table`](#recurrence_table) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 RecurrenceTable 能力，进入后续全景目录。 |
| [`residue`](#residue) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 Residue 能力，进入后续全景目录。 |
| [`series`](#series) | 已实现 | 当前可用 | 局部 Taylor 级数 |
| [`series_coefficient`](#series_coefficient) | 已实现 | 当前可用 | 取已构造级数的指定系数 |
| [`z_transform`](#z_transform) | 后续规划 | 规划接口，当前不可用 | 微积分与变换中的 ZTransform 能力，进入后续全景目录。 |

## asymptotic

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000340`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 Asymptotic 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Asymptotic`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
asymptotic(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## asymptotic_integrate

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000341`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 AsymptoticIntegrate 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`AsymptoticIntegrate`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
asymptotic_integrate(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## asymptotic_sum

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000342`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 AsymptoticSum 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`AsymptoticSum`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
asymptotic_sum(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## curl

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000146`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

三维笛卡尔旋度

- 当前支持：真实求导器计算笛卡尔微分；1..64个互异用户坐标局部化，readonly展开源码，禁止间接写入。Grad/Hessian/Laplacian为标量表达式，Jacobian向量分量为行/坐标为列，Div维度匹配，Curl仅3D。未知求导保持真实形式导数，错误形状/变量/预算拒绝。
- 目标范围：复用真实 D；变量必须唯一，向量维度匹配；不把形式 Derivative 变成已求出的闭式。
- 返回：expression
- 精度：符号导数复用真实求导，不把形式未求值导数或函数采样宣称为解析解；笛卡尔首版，不自动获得其他坐标或任意单位支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Curl`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
curl(vector,variables)
```

当前 Wolfram 签名：

```text
Curl[vector,variables]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
curl(vector, [x,y,z])
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `vector` | positional | 必填 | 待求导标量或向量源码 | current |
| `variables` | positional | 必填 | 有序笛卡尔局部坐标 | current |

当前已登记示例（Wolfram）：

```wolfram
Curl[{-y,x,0},{x,y,z}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/calculus_registry.rs](../../crates/om-eval/src/science/calculus_registry.rs)、[crates/om-eval/src/science/vector_calculus.rs](../../crates/om-eval/src/science/vector_calculus.rs)、[crates/om-eval/src/algebra_diff.rs](../../crates/om-eval/src/algebra_diff.rs)。

当前测试引用：[crates/om-eval/tests/vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。

## d_solve

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000352`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 DSolve 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DSolve`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
d_solve(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## d_solve_value

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000353`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 DSolveValue 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DSolveValue`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
d_solve_value(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## diff

**当前实现：已实现；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000032`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

按链式法则求重复或混合偏导数。

- 当前支持：重复/混合偏导、列表、初等链式法则、形式 Derivative；最多64规格和4096阶。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：expression
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`D`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
D(expr, x, ...)
```

当前 Wolfram 签名：

```text
D[expr, x, ...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
diff(expr, x, order: 1); diff(expr, ...variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 被求导表达式 | current |
| `variables` | variadic | 至少一项 | 旧变量/重复阶数规格 | current |
| `order` | option | 1 | .3 单变量重复阶数命名选项 | r3 |

当前已登记示例（Wolfram）：

```wolfram
D[Sin[x^2],x]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/algebra_registry.rs](../../crates/om-eval/src/algebra_registry.rs)。

当前测试引用：[crates/om-eval/tests/algebra.rs](../../crates/om-eval/tests/algebra.rs)。

## difference_delta

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000349`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 DifferenceDelta 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DifferenceDelta`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
difference_delta(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## discrete_limit

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000350`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 DiscreteLimit 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`DiscreteLimit`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
discrete_limit(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## div

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000145`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

笛卡尔散度

- 当前支持：真实求导器计算笛卡尔微分；1..64个互异用户坐标局部化，readonly展开源码，禁止间接写入。Grad/Hessian/Laplacian为标量表达式，Jacobian向量分量为行/坐标为列，Div维度匹配，Curl仅3D。未知求导保持真实形式导数，错误形状/变量/预算拒绝。
- 目标范围：复用真实 D；变量必须唯一，向量维度匹配；不把形式 Derivative 变成已求出的闭式。
- 返回：expression
- 精度：符号导数复用真实求导，不把形式未求值导数或函数采样宣称为解析解；笛卡尔首版，不自动获得其他坐标或任意单位支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Divergence`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
div(vector,variables)
```

当前 Wolfram 签名：

```text
Divergence[vector,variables]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
div(vector, variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `vector` | positional | 必填 | 待求导标量或向量源码 | current |
| `variables` | positional | 必填 | 有序笛卡尔局部坐标 | current |

当前已登记示例（Wolfram）：

```wolfram
Divergence[{x,y,z},{x,y,z}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/calculus_registry.rs](../../crates/om-eval/src/science/calculus_registry.rs)、[crates/om-eval/src/science/vector_calculus.rs](../../crates/om-eval/src/science/vector_calculus.rs)、[crates/om-eval/src/algebra_diff.rs](../../crates/om-eval/src/algebra_diff.rs)。

当前测试引用：[crates/om-eval/tests/vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。

## dt

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000335`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 Dt 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Dt`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
dt(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## fourier_transform

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000343`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 FourierTransform 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`FourierTransform`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
fourier_transform(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## generating_function

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000351`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 GeneratingFunction 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`GeneratingFunction`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
generating_function(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## grad

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000142`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

笛卡尔梯度

- 当前支持：真实求导器计算笛卡尔微分；1..64个互异用户坐标局部化，readonly展开源码，禁止间接写入。Grad/Hessian/Laplacian为标量表达式，Jacobian向量分量为行/坐标为列，Div维度匹配，Curl仅3D。未知求导保持真实形式导数，错误形状/变量/预算拒绝。
- 目标范围：复用真实 D；变量必须唯一，向量维度匹配；不把形式 Derivative 变成已求出的闭式。
- 返回：expression
- 精度：符号导数复用真实求导，不把形式未求值导数或函数采样宣称为解析解；笛卡尔首版，不自动获得其他坐标或任意单位支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Grad`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
grad(expr,variables)
```

当前 Wolfram 签名：

```text
Grad[expr,variables]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
grad(expr, variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 待求导标量或向量源码 | current |
| `variables` | positional | 必填 | 有序笛卡尔局部坐标 | current |

当前已登记示例（Wolfram）：

```wolfram
Grad[x^2+y^2,{x,y}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/calculus_registry.rs](../../crates/om-eval/src/science/calculus_registry.rs)、[crates/om-eval/src/science/vector_calculus.rs](../../crates/om-eval/src/science/vector_calculus.rs)、[crates/om-eval/src/algebra_diff.rs](../../crates/om-eval/src/algebra_diff.rs)。

当前测试引用：[crates/om-eval/tests/vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。

## hessian

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000144`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

Hessian 矩阵

- 当前支持：真实求导器计算笛卡尔微分；1..64个互异用户坐标局部化，readonly展开源码，禁止间接写入。Grad/Hessian/Laplacian为标量表达式，Jacobian向量分量为行/坐标为列，Div维度匹配，Curl仅3D。未知求导保持真实形式导数，错误形状/变量/预算拒绝。
- 目标范围：复用真实 D；变量必须唯一，向量维度匹配；不把形式 Derivative 变成已求出的闭式。
- 返回：expression
- 精度：符号导数复用真实求导，不把形式未求值导数或函数采样宣称为解析解；笛卡尔首版，不自动获得其他坐标或任意单位支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Hessian`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
hessian(expr,variables)
```

当前 Wolfram 签名：

```text
Hessian[expr,variables]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
hessian(expr, variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 待求导标量或向量源码 | current |
| `variables` | positional | 必填 | 有序笛卡尔局部坐标 | current |

当前已登记示例（Wolfram）：

```wolfram
Hessian[x^2*y,{x,y}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/calculus_registry.rs](../../crates/om-eval/src/science/calculus_registry.rs)、[crates/om-eval/src/science/vector_calculus.rs](../../crates/om-eval/src/science/vector_calculus.rs)、[crates/om-eval/src/algebra_diff.rs](../../crates/om-eval/src/algebra_diff.rs)。

当前测试引用：[crates/om-eval/tests/vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。

## integrate

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000148`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

符号或显式数值积分

- 当前支持：默认exact：有界多项式/仿射初等形式/可识别链式/有限分部积分、Q系数有理式一次或二次因子（重复≤32）、Gaussian→Erf，exact原函数验证限精确系数，近似系数须用户显式有理化；所有候选实际求导并精确化简验证；保留非零/实轴/主值分支条件。定积分验证真实端点和区间奇点，可去孔洞与实际极点区分；无法证明参数收敛/分支时原子拒绝。Gaussian完整或半无限区间限明确正实二次系数。numeric保持实际GK15/7机器路径，误差非证书，高精度/非收敛/预算真实失败。
- 目标范围：符号：多项式、线性初等、有限分部积分、一次/二次因子有理式、Gaussian→Erf，保留条件；数值：一维自适应Gauss–Kronrod15/7、有限/无限区间和显式断点，报告误差估计。
- 返回：exact_expression_or_numeric_diagnostics
- 精度：exact仅所列规则并实际求导验证，不自动近似；符号常数/条件保持。numeric仅机器精度和估计误差，不把误差当包围证书或宣称通用Risch。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Integrate`、`NIntegrate`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
integrate(expr,variable_or_range,mode:"exact")
n_integrate(expr,axis)
```

当前 Wolfram 签名：

```text
Integrate[expr,variable_or_range]
NIntegrate[expr,axis]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
integrate(expr, x); integrate(expr, x: a..b, mode: "exact")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 保持原始结构的被积表达式 | r3 |
| `x` | positional | 必填 | 签名对应的x参数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Integrate[x^2,x]
NIntegrate[x^2,{x,0,1}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-analysis/src/integration.rs](../../crates/om-analysis/src/integration.rs)、[crates/om-eval/src/science/numeric_integration.rs](../../crates/om-eval/src/science/numeric_integration.rs)、[crates/om-eval/src/science/integration_registry.rs](../../crates/om-eval/src/science/integration_registry.rs)、[crates/om-eval/src/science/symbolic_integration.rs](../../crates/om-eval/src/science/symbolic_integration.rs)、[crates/om-eval/src/science/integral_rules.rs](../../crates/om-eval/src/science/integral_rules.rs)、[crates/om-eval/src/science/integral_rational.rs](../../crates/om-eval/src/science/integral_rational.rs)。

当前测试引用：[crates/om-analysis/tests/integration.rs](../../crates/om-analysis/tests/integration.rs)、[crates/om-eval/tests/numeric_integration.rs](../../crates/om-eval/tests/numeric_integration.rs)、[crates/om-eval/tests/symbolic_integration.rs](../../crates/om-eval/tests/symbolic_integration.rs)。

## inverse_fourier_transform

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000344`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 InverseFourierTransform 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`InverseFourierTransform`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
inverse_fourier_transform(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## inverse_laplace_transform

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000346`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 InverseLaplaceTransform 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`InverseLaplaceTransform`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
inverse_laplace_transform(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## inverse_z_transform

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000348`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 InverseZTransform 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`InverseZTransform`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
inverse_z_transform(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## jacobian

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000143`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

Jacobian 矩阵

- 当前支持：真实求导器计算笛卡尔微分；1..64个互异用户坐标局部化，readonly展开源码，禁止间接写入。Grad/Hessian/Laplacian为标量表达式，Jacobian向量分量为行/坐标为列，Div维度匹配，Curl仅3D。未知求导保持真实形式导数，错误形状/变量/预算拒绝。
- 目标范围：复用真实 D；变量必须唯一，向量维度匹配；不把形式 Derivative 变成已求出的闭式。
- 返回：expression
- 精度：符号导数复用真实求导，不把形式未求值导数或函数采样宣称为解析解；笛卡尔首版，不自动获得其他坐标或任意单位支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Jacobian`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
jacobian(expressions,variables)
```

当前 Wolfram 签名：

```text
Jacobian[expressions,variables]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
jacobian(expressions, variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expressions` | positional | 必填 | 待求导标量或向量源码 | current |
| `variables` | positional | 必填 | 有序笛卡尔局部坐标 | current |

当前已登记示例（Wolfram）：

```wolfram
Jacobian[{x*y,Sin[x]},{x,y}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/calculus_registry.rs](../../crates/om-eval/src/science/calculus_registry.rs)、[crates/om-eval/src/science/vector_calculus.rs](../../crates/om-eval/src/science/vector_calculus.rs)、[crates/om-eval/src/algebra_diff.rs](../../crates/om-eval/src/algebra_diff.rs)。

当前测试引用：[crates/om-eval/tests/vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。

## laplace_transform

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000345`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 LaplaceTransform 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`LaplaceTransform`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
laplace_transform(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## laplacian

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000147`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

笛卡尔拉普拉斯

- 当前支持：真实求导器计算笛卡尔微分；1..64个互异用户坐标局部化，readonly展开源码，禁止间接写入。Grad/Hessian/Laplacian为标量表达式，Jacobian向量分量为行/坐标为列，Div维度匹配，Curl仅3D。未知求导保持真实形式导数，错误形状/变量/预算拒绝。
- 目标范围：复用真实 D；变量必须唯一，向量维度匹配；不把形式 Derivative 变成已求出的闭式。
- 返回：expression
- 精度：符号导数复用真实求导，不把形式未求值导数或函数采样宣称为解析解；笛卡尔首版，不自动获得其他坐标或任意单位支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Laplacian`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
laplacian(expr,variables)
```

当前 Wolfram 签名：

```text
Laplacian[expr,variables]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
laplacian(expr, variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 待求导标量或向量源码 | current |
| `variables` | positional | 必填 | 有序笛卡尔局部坐标 | current |

当前已登记示例（Wolfram）：

```wolfram
Laplacian[x^2+y^2+z^2,{x,y,z}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/calculus_registry.rs](../../crates/om-eval/src/science/calculus_registry.rs)、[crates/om-eval/src/science/vector_calculus.rs](../../crates/om-eval/src/science/vector_calculus.rs)、[crates/om-eval/src/algebra_diff.rs](../../crates/om-eval/src/algebra_diff.rs)。

当前测试引用：[crates/om-eval/tests/vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。

## limit

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000149`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

计算有限点或无穷处极限

- 当前支持：精确实单侧/双侧/±inf；有理式≤256次按原消失阶与首项系数，解析0/0通过真实导数消失阶（≤64）与局部解析检查，支持明确初等组合、有界实函数夹逼、正底幂/无限倒数坐标的定理转换。左右不同/振荡/未证明参数或超限保留原式，绝不靠附近采样猜极限。
- 目标范围：有理式与明确可识别的初等形式；左右极限不同则双侧不存在；不能仅靠附近采样宣称符号极限。
- 返回：expression
- 精度：首版精确系数与精确实点；不把机器/大浮点输入舍入成符号证书。Taylor不是Laurent/渐近误差界，未计算或截断外不声称零。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Limit`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
limit(expr,x,at:point,direction:"both")
```

当前 Wolfram 签名：

```text
Limit[expr,x->point]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
limit(expr, x, at: point, direction: "both")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 表达式 | current |
| `x` | positional | 必填 | 变量 | current |
| `at` | option | 必填 | 有限点或无穷 | current |
| `direction` | option | both | both/left/right | current |

当前已登记示例（Wolfram）：

```wolfram
Limit[Sin[x]/x,x->0]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/series_registry.rs](../../crates/om-eval/src/science/series_registry.rs)、[crates/om-eval/src/science/limits.rs](../../crates/om-eval/src/science/limits.rs)、[crates/om-eval/src/science/calculus_source.rs](../../crates/om-eval/src/science/calculus_source.rs)。

当前测试引用：[crates/om-eval/tests/limits.rs](../../crates/om-eval/tests/limits.rs)。

## n_product

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000339`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 NProduct 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`NProduct`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
n_product(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## n_sum

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000338`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 NSum 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`NSum`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
n_sum(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## pade_approximant

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000337`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 PadeApproximant 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`PadeApproximant`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
pade_approximant(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## r_solve

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000354`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 RSolve 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`RSolve`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
r_solve(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## r_solve_value

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000355`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 RSolveValue 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`RSolveValue`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
r_solve_value(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## recurrence_table

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000356`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 RecurrenceTable 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`RecurrenceTable`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
recurrence_table(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## residue

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000336`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 Residue 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Residue`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
residue(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## series

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000150`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

局部 Taylor 级数

- 当前支持：普通Taylor 0..64阶，真实求导获得系数，SeriesData携带0起始/截断后首个未知阶/步长1；展开点为精确有限实数，坐标局部化，raw孔洞/极点/分支点/不可证明解析/未求值导数拒绝。SeriesCoefficient仅已知范围，越界不伪造0；Normal显式去截断返回已知多项式，其余输入原行为保持。
- 目标范围：在可证解析的有限点，阶数0..64；系数来自真实求导；明确截断阶；极点/分支点不伪装Taylor。
- 返回：expression
- 精度：首版精确系数与精确实点；不把机器/大浮点输入舍入成符号证书。Taylor不是Laurent/渐近误差界，未计算或截断外不声称零。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Series`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
series(expr,x,at:0,order:6)
```

当前 Wolfram 签名：

```text
Series[expr,{x,point,order}]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
series(expr, x, at: 0, order: 6)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 表达式 | current |
| `x` | positional | 必填 | 变量 | current |
| `at` | option | 0 | 展开点 | current |
| `order` | option | 6 | 非负整数0..64 | current |

当前已登记示例（Wolfram）：

```wolfram
Series[Sin[x],{x,0,7}]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/series_registry.rs](../../crates/om-eval/src/science/series_registry.rs)、[crates/om-eval/src/science/taylor.rs](../../crates/om-eval/src/science/taylor.rs)、[crates/om-eval/src/science/calculus_source.rs](../../crates/om-eval/src/science/calculus_source.rs)。

当前测试引用：[crates/om-eval/tests/series.rs](../../crates/om-eval/tests/series.rs)。

## series_coefficient

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000151`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

取已构造级数的指定系数

- 当前支持：普通Taylor 0..64阶，真实求导获得系数，SeriesData携带0起始/截断后首个未知阶/步长1；展开点为精确有限实数，坐标局部化，raw孔洞/极点/分支点/不可证明解析/未求值导数拒绝。SeriesCoefficient仅已知范围，越界不伪造0；Normal显式去截断返回已知多项式，其余输入原行为保持。
- 目标范围：首版常规整数阶Taylor；超出截断阶报错，不假定缺失高阶项为零。
- 返回：expression
- 精度：首版精确系数与精确实点；不把机器/大浮点输入舍入成符号证书。Taylor不是Laurent/渐近误差界，未计算或截断外不声称零。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SeriesCoefficient`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
series_coefficient(series,order)
```

当前 Wolfram 签名：

```text
SeriesCoefficient[series,order]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
series_coefficient(series, order)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `series` | positional | 必填 | Series 对象 | current |
| `order` | positional | 必填 | 整数阶 | current |

当前已登记示例（Wolfram）：

```wolfram
SeriesCoefficient[Series[Exp[x],{x,0,4}],3]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/series_registry.rs](../../crates/om-eval/src/science/series_registry.rs)、[crates/om-eval/src/science/taylor.rs](../../crates/om-eval/src/science/taylor.rs)、[crates/om-eval/src/science/calculus_source.rs](../../crates/om-eval/src/science/calculus_source.rs)。

当前测试引用：[crates/om-eval/tests/series.rs](../../crates/om-eval/tests/series.rs)。

## z_transform

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000347`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

微积分与变换中的 ZTransform 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ZTransform`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
z_transform(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

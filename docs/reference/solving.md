<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# 求解与条件

[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)

所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。

| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |
|---|---|---|---|
| [`algebraic_root`](#algebraic_root) | 部分支持 | 现有入口可用，统一接口待实施 | 表示具有精确编号的代数根。 |
| [`asymptotic_solve`](#asymptotic_solve) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 AsymptoticSolve 能力，进入后续全景目录。 |
| [`boolean_convert`](#boolean_convert) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 BooleanConvert 能力，进入后续全景目录。 |
| [`boolean_minimize`](#boolean_minimize) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 BooleanMinimize 能力，进入后续全景目录。 |
| [`conditional`](#conditional) | 已实现 | 现有入口可用，统一接口待实施 | 仅在条件成立时求值；假条件返回 Undefined。 |
| [`cylindrical_decomposition`](#cylindrical_decomposition) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 CylindricalDecomposition 能力，进入后续全景目录。 |
| [`eliminate`](#eliminate) | 部分支持 | 当前可用 | 通过多项式消元理想消去变量。 |
| [`exists`](#exists) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 Exists 能力，进入后续全景目录。 |
| [`find_instance`](#find_instance) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 FindInstance 能力，进入后续全景目录。 |
| [`find_root`](#find_root) | 部分支持 | 现有入口可用，统一接口待实施 | 从局部起点或实数括区间寻找数值根。 |
| [`for_all`](#for_all) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 ForAll 能力，进入后续全景目录。 |
| [`reduce`](#reduce) | 部分支持 | 现有入口可用，统一接口待实施 | 把方程或一元有理不等式化为布尔条件。 |
| [`resolve`](#resolve) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 Resolve 能力，进入后续全景目录。 |
| [`roots`](#roots) | 已实现 | 现有入口可用，统一接口待实施 | 把完整解集合转换为布尔析取。 |
| [`satisfiability_instances`](#satisfiability_instances) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 SatisfiabilityInstances 能力，进入后续全景目录。 |
| [`satisfiable_q`](#satisfiable_q) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 SatisfiableQ 能力，进入后续全景目录。 |
| [`solve`](#solve) | 部分支持 | 现有入口可用，统一接口待实施 | 求解原始方程并保留定义域限制。；认证有限代数解的数值近似。；按请求变量顺序输出精确解值。；按请求变量顺序输出数值解值。 |
| [`solve_always`](#solve_always) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 SolveAlways 能力，进入后续全景目录。 |
| [`tautology_q`](#tautology_q) | 后续规划 | 规划接口，当前不可用 | 求解与条件中的 TautologyQ 能力，进入后续全景目录。 |

## algebraic_root

**当前实现：部分支持；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000090`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

表示具有精确编号的代数根。

- 当前支持：有理闭合多项式根认证≤64次、从1起编号；符号参数根保留；不是普通 nth_root。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：expression
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Root`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
Root(function, k)
```

当前 Wolfram 签名：

```text
Root[function, k]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
algebraic_root(fn(x) => polynomial, index)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `fn` | positional | 必填 | 单变量多项式函数 | current |
| `index` | positional | 必填 | 从 1 起的认证根编号 | current |

当前已登记示例（Wolfram）：

```wolfram
Root[#^5-# +1&,1]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## asymptotic_solve

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000334`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 AsymptoticSolve 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`AsymptoticSolve`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
asymptotic_solve(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## boolean_convert

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000328`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 BooleanConvert 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`BooleanConvert`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
boolean_convert(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## boolean_minimize

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000329`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 BooleanMinimize 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`BooleanMinimize`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
boolean_minimize(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## conditional

**当前实现：已实现；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000024`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

仅在条件成立时求值；假条件返回 Undefined。

- 当前支持：按真实注册签名与参数个数工作；未支持的表达式保持符号形式并给出已有诊断。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：expression
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ConditionalExpression`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
ConditionalExpression(expr, condition)
```

当前 Wolfram 签名：

```text
ConditionalExpression[expr, condition]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
conditional(expr, condition)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `expr` | positional | 必填 | 条件值 | current |
| `condition` | positional | 必填 | True 去包装、False 返回 Undefined | current |

当前已登记示例（Wolfram）：

```wolfram
ConditionalExpression[2+2,True]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## cylindrical_decomposition

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000333`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 CylindricalDecomposition 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`CylindricalDecomposition`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
cylindrical_decomposition(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## eliminate

**当前实现：部分支持；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000037`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

通过多项式消元理想消去变量。

- 当前支持：按真实注册签名与参数个数工作；未支持的表达式保持符号形式并给出已有诊断。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：boolean_or_condition
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Eliminate`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
Eliminate(eqs, vars)
```

当前 Wolfram 签名：

```text
Eliminate[eqs, vars]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
eliminate(equations, variables)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `equations` | positional | 必填 | 多项式等式系统 | current |
| `variables` | positional | 必填 | 被消元变量 | current |

当前已登记示例（Wolfram）：

```wolfram
Eliminate[{x==y+1,y==2z},y]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## exists

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000327`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 Exists 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Exists`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
exists(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## find_instance

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000324`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 FindInstance 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`FindInstance`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
find_instance(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## find_root

**当前实现：部分支持；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000045`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

从局部起点或实数括区间寻找数值根。

- 当前支持：旧 {x,start} / {x,a,b} / 多变量起点形式，Newton/Brent匹配，5..2466十进制位；验证原残差及极点；不承诺完整根集。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：rules_or_values
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`FindRoot`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
FindRoot(eqs, starts, options)
```

当前 Wolfram 签名：

```text
FindRoot[eqs, starts, options]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
find_root(equations, variables, initial: values); find_root(equation, x, bracket: a..b)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `equations` | positional | 必填 | 原始残差或等式 | current |
| `variables` | positional | 必填 | 变量 | current |
| `initial` | option | 无 | 与 bracket 二选一 | r3 |
| `bracket` | option | 无 | 实数括区间 | r3 |
| `precision` | option | machine | 保留现有 5..2466 十进制位上限 | current |
| `method` | option | auto | 区间 Brent、起点 Newton | current |
| `max_iterations` | option | 100 | 最大迭代次数 | current |
| `output` | option | rules | .3 支持单变量 value 投影 | r3 |

当前已登记示例（Wolfram）：

```wolfram
FindRoot[x^2==2,{x,1}]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## for_all

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000326`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 ForAll 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`ForAll`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
for_all(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## reduce

**当前实现：部分支持；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000085`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

把方程或一元有理不等式化为布尔条件。

- 当前支持：已有求解范围及一元有理不等式，保留条件；不支持一般多元 CAD。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：boolean_or_condition
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Reduce`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
Reduce(expr, vars, domain, options)
```

当前 Wolfram 签名：

```text
Reduce[expr, vars, domain, options]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
reduce(conditions, variables, domain: complexes)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `conditions` | positional | 必填 | 等式或不等式 | current |
| `variables` | positional | 可推断 | 有序变量 | current |
| `domain` | option | complexes | 现代命名 domain 当前要求显式变量 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Reduce[x^2<4,x]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## resolve

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000325`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 Resolve 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Resolve`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
resolve(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## roots

**当前实现：已实现；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000092`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

把完整解集合转换为布尔析取。

- 当前支持：按真实注册签名与参数个数工作；未支持的表达式保持符号形式并给出已有诊断。
- 目标范围：保留当前数学行为，补齐规范名称、结构化参数说明和统一元数据。
- 返回：boolean_or_condition
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Roots`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
Roots(eqs, vars, domain, options)
```

当前 Wolfram 签名：

```text
Roots[eqs, vars, domain, options]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
roots(equations, variables, domain: complexes)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `equations` | positional | 必填 | 多项式等式 | current |
| `variables` | positional | 可推断 | 变量 | current |
| `domain` | option | complexes | 域 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Roots[x^2==1,x]
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## satisfiability_instances

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000331`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 SatisfiabilityInstances 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SatisfiabilityInstances`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
satisfiability_instances(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## satisfiable_q

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000330`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 SatisfiableQ 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SatisfiableQ`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
satisfiable_q(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## solve

**当前实现：部分支持；目标接口：现有入口可用，统一接口待实施。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000117`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

求解原始方程并保留定义域限制。；认证有限代数解的数值近似。；按请求变量顺序输出精确解值。；按请求变量顺序输出数值解值。

- 当前支持：复用已有完整有限代数解、特殊超越族及整数/参数条件子集；当前四个入口分立。新 mode/output 尚未接入。
- 目标范围：用严格的 mode/output 适配到旧求解器；默认形状、重数、变量顺序、原始极点与条件不变；无法支持时原式+诊断，绝不自动转局部根。
- 返回：rules_or_values
- 精度：精确计算与已支持的数值近似子集；不把符号保留或格式位数当作新算法/任意精度支持。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Solve`、`NSolve`、`SolveValues`、`NSolveValues`。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
Solve(eqs, vars, domain, options)
NSolve(eqs, vars, options)
SolveValues(eqs, vars, domain, options)
NSolveValues(eqs, vars, options)
```

当前 Wolfram 签名：

```text
Solve[eqs, vars, domain, options]
NSolve[eqs, vars, options]
SolveValues[eqs, vars, domain, options]
NSolveValues[eqs, vars, options]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
solve(equations, variables, mode: "exact", output: "rules", domain: complexes)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `equations` | positional | 必填 | 原始等式/系统 | current |
| `variables` | positional | 可推断 | 有序变量 | current |
| `mode` | option | exact | exact 或 numeric | r3 |
| `output` | option | rules | rules 或 values | r3 |
| `domain` | option | complexes | complexes/reals/integers/rationals | current |
| `precision` | option | machine | numeric 模式5..2466十进制位或机器精度 | r3 |
| `steps` | option | 笔记本设置 | 是否记录真实推导 | r3 |
| `assumptions` | option | 无 | 约束与参数条件 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Solve[x^2==2,x]
NSolve[x^2==2,x,WorkingPrecision->50]
SolveValues[x^2==2,x]
NSolveValues[x^2==2,x]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
solve(x^2 = 2, x, mode: "numeric", precision: 50, output: "values")
```

验收：现有行为、参数拒绝、解析/格式往返、中断和独立数学期望保持不变；新签名另写正反例。

当前源码：[crates/om-eval/src/solver_registry.rs](../../crates/om-eval/src/solver_registry.rs)。

当前测试引用：[crates/om-eval/tests/solver.rs](../../crates/om-eval/tests/solver.rs)、[crates/om-eval/tests/solver_provenance.rs](../../crates/om-eval/tests/solver_provenance.rs)。

## solve_always

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000323`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 SolveAlways 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SolveAlways`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
solve_always(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## tautology_q

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000332`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

求解与条件中的 TautologyQ 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TautologyQ`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
tautology_q(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

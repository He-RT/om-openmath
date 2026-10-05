<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# 向量、矩阵与张量

[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)

所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。

| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |
|---|---|---|---|
| [`adjoint`](#adjoint) | 已实现 | 当前可用 | 共轭转置 |
| [`angle`](#angle) | 下一版规划 | 规划接口，当前不可用 | 向量夹角 |
| [`band`](#band) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 Band 能力，进入后续全景目录。 |
| [`cholesky`](#cholesky) | 已实现 | 当前可用 | 正定矩阵 Cholesky |
| [`cross`](#cross) | 已实现 | 当前可用 | 三维叉积 |
| [`det`](#det) | 已实现 | 当前可用 | 行列式 |
| [`diag`](#diag) | 已实现 | 当前可用 | 对角矩阵或提取对角 |
| [`dot`](#dot) | 已实现 | 当前可用 | 向量/矩阵收缩 |
| [`eigensystem`](#eigensystem) | 已实现 | 当前可用 | 特征值与特征向量 |
| [`eigenvalues`](#eigenvalues) | 已实现 | 当前可用 | 特征值 |
| [`identity`](#identity) | 已实现 | 当前可用 | 单位矩阵 |
| [`inverse`](#inverse) | 已实现 | 当前可用 | 矩阵逆 |
| [`least_squares`](#least_squares) | 已实现 | 当前可用 | 最小二乘解 |
| [`linear_solve`](#linear_solve) | 部分支持 | 当前可用 | 线性系统解 |
| [`lu`](#lu) | 已实现 | 当前可用 | 带置换的 LU |
| [`matrix_exp`](#matrix_exp) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 MatrixExp 能力，进入后续全景目录。 |
| [`matrix_log`](#matrix_log) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 MatrixLog 能力，进入后续全景目录。 |
| [`matrix_power`](#matrix_power) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 MatrixPower 能力，进入后续全景目录。 |
| [`norm`](#norm) | 已实现 | 当前可用 | 向量范数 |
| [`normalize`](#normalize) | 已实现 | 当前可用 | 按范数归一化 |
| [`null_space`](#null_space) | 部分支持 | 当前可用 | 零空间基 |
| [`orthogonalize`](#orthogonalize) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 Orthogonalize 能力，进入后续全景目录。 |
| [`projection`](#projection) | 下一版规划 | 规划接口，当前不可用 | 向量投影 |
| [`pseudo_inverse`](#pseudo_inverse) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 PseudoInverse 能力，进入后续全景目录。 |
| [`qr`](#qr) | 已实现 | 当前可用 | QR 分解 |
| [`rank`](#rank) | 部分支持 | 当前可用 | 矩阵秩 |
| [`sparse_array`](#sparse_array) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 SparseArray 能力，进入后续全景目录。 |
| [`svd`](#svd) | 已实现 | 当前可用 | 奇异值分解 |
| [`tensor_contract`](#tensor_contract) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 TensorContract 能力，进入后续全景目录。 |
| [`tensor_dimensions`](#tensor_dimensions) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 TensorDimensions 能力，进入后续全景目录。 |
| [`tensor_product`](#tensor_product) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 TensorProduct 能力，进入后续全景目录。 |
| [`tensor_rank`](#tensor_rank) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 TensorRank 能力，进入后续全景目录。 |
| [`tensor_transpose`](#tensor_transpose) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 TensorTranspose 能力，进入后续全景目录。 |
| [`trace`](#trace) | 已实现 | 当前可用 | 矩阵迹 |
| [`transpose`](#transpose) | 已实现 | 当前可用 | 转置 |
| [`unit_vector`](#unit_vector) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 UnitVector 能力，进入后续全景目录。 |
| [`vector_angle`](#vector_angle) | 后续规划 | 规划接口，当前不可用 | 向量、矩阵与张量中的 VectorAngle 能力，进入后续全景目录。 |

## adjoint

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000162`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

共轭转置

- 当前支持：矩形矩阵≤64×64，转置并真实共轭；未知符号共轭保留原式。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。 复数运算使用正确的共轭语义；机器夹角仅将舍入越界夹到[-1,1]。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
adjoint(...)
```

当前 Wolfram 签名：

```text
ConjugateTranspose[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
adjoint(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
ConjugateTranspose[{{1,I},{0,2}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
adjoint([[1,i],[0,1]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## angle

**当前实现：下一版规划；目标接口：规划接口，当前不可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000157`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

向量夹角

- 当前支持：当前无此规范接口的实现。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。 复数运算使用正确的共轭语义；机器夹角仅将舍入越界夹到[-1,1]。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：无；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
angle(a, b)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `a` | positional | 必填 | 签名对应的a参数 | r3 |
| `b` | positional | 必填 | 签名对应的b参数 | r3 |

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
angle([1,0],[0,1])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## band

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000358`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 Band 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Band`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
band(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## cholesky

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000171`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

正定矩阵 Cholesky

- 当前支持：实对称正定机器矩阵≤64×64，返回下三角因子；按32机器epsilon检验对称/正定。非正定、近数值退化、非有限与中断拒绝。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
cholesky(matrix)
```

当前 Wolfram 签名：

```text
Cholesky[matrix]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
cholesky(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Cholesky[{{4.,2.},{2.,3.}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
cholesky([[2.0,1.0],[1.0,3.0]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix/symmetric.rs](../../crates/om-analysis/src/matrix/symmetric.rs)。

当前测试引用：[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)、[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)。

## cross

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000154`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

三维叉积

- 当前支持：三维向量的双线性交叉乘积；检查长度，保留数值或符号表达式。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
cross(...)
```

当前 Wolfram 签名：

```text
Cross[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
cross(a, b)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `a` | positional | 必填 | 签名对应的a参数 | r3 |
| `b` | positional | 必填 | 签名对应的b参数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Cross[{1,0,0},{0,1,0}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
cross([1,0,0],[0,1,0])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## det

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000164`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

行列式

- 当前支持：精确有理数方阵复用Bareiss；机器实数方阵使用部分选主元LU对角乘积，数值溢出明确诊断。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
det(...)
```

当前 Wolfram 签名：

```text
Det[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
det(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Det[{{1,2},{3,4}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
det([[2,1],[1,3]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## diag

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000160`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

对角矩阵或提取对角

- 当前支持：最多64项的对角矩阵；保留表达式元素。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
diag(...)
```

当前 Wolfram 签名：

```text
DiagonalMatrix[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
diag(values)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `values` | positional | 必填 | 签名对应的values参数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
DiagonalMatrix[{2,3}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
diag([1,2,3])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## dot

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000153`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

向量/矩阵收缩

- 当前支持：非空向量和最多64×64矩阵；矩阵乘积双线性，两个向量的内积共轭第一向量；检查维度并保留精确或机器运算。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
dot(...)
```

当前 Wolfram 签名：

```text
Dot[a,b]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
dot(a, b)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `a` | positional | 必填 | 签名对应的a参数 | r3 |
| `b` | positional | 必填 | 签名对应的b参数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Dot[{{1,2},{3,4}},{5,6}]
Dot[{1,2},{3,4}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
[1,2] @ [3,4]
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/composition.rs](../../crates/om-eval/src/composition.rs)。

当前测试引用：[crates/om-eval/tests/composition.rs](../../crates/om-eval/tests/composition.rs)。

## eigensystem

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000174`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

特征值与特征向量

- 当前支持：仅实对称机器矩阵≤64×64，返回values/vectors（匹配列向量）/实际residual/method/rotations，向量正交与重根确定性；不将残差称作严格误差界。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
eigensystem(matrix)
```

当前 Wolfram 签名：

```text
Eigensystem[matrix]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
eigensystem(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Eigensystem[{{2.,1.},{1.,2.}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
eigensystem([[2.0,1.0],[1.0,2.0]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix/symmetric.rs](../../crates/om-analysis/src/matrix/symmetric.rs)。

当前测试引用：[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)、[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)。

## eigenvalues

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000173`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

特征值

- 当前支持：仅实对称机器矩阵≤64×64，最大枢轴Jacobi；值升序。非对称/复矩阵明确不支持，有限旋转预算内未收敛失败。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
eigenvalues(matrix)
```

当前 Wolfram 签名：

```text
Eigenvalues[matrix]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
eigenvalues(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Eigenvalues[{{2.,1.},{1.,2.}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
eigenvalues([[2.0,1.0],[1.0,2.0]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix/symmetric.rs](../../crates/om-analysis/src/matrix/symmetric.rs)。

当前测试引用：[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)、[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)。

## identity

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000159`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

单位矩阵

- 当前支持：构造0..64维单位矩阵。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
identity(...)
```

当前 Wolfram 签名：

```text
IdentityMatrix[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
identity(size)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `size` | positional | 必填 | 签名对应的size参数 | r3 |

当前已登记示例（Wolfram）：

```wolfram
IdentityMatrix[2]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
identity(3)
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## inverse

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000165`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

矩阵逆

- 当前支持：精确有理方阵复用Bareiss；机器方阵逐列LU求解，奇异/病态拒绝，相对枢轴阈值1e-12。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
inverse(...)
```

当前 Wolfram 签名：

```text
Inverse[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
inverse(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Inverse[{{1,2},{3,4}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
inverse([[2,1],[1,3]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## least_squares

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000175`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

最小二乘解

- 当前支持：机器实数满列秩m≥n≤64，Householder QR最小二乘；返回solution/residual_norm/method/converged。欠定、秩不足及病态明确失败；相对枢轴阈值1e-12。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
least_squares(matrix,rhs)
```

当前 Wolfram 签名：

```text
LeastSquares[matrix,rhs]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
least_squares(matrix, rhs)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |
| `rhs` | positional | 必填 | 只读右端函数或右端项 | r3 |

当前已登记示例（Wolfram）：

```wolfram
LeastSquares[{{1.,0.},{1.,1.},{1.,2.}},{1.,2.,2.}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
least_squares([[1.0,0.0],[1.0,1.0],[1.0,2.0]],[1.0,2.0,3.0])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix.rs](../../crates/om-analysis/src/matrix.rs)。

当前测试引用：[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)、[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)。

## linear_solve

**当前实现：部分支持；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000168`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

线性系统解

- 当前支持：默认mode exact，精确有理矩阵复用Bareiss；mode numeric首版方阵部分选主元LU，奇异/病态/不相容维度拒绝。矩形机器系统及自由轴返回仍待接入。
- 目标范围：精确有理矩阵复用Bareiss；机器实数部分选主元；欠定结果保留自由轴，不强行给唯一解。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
linear_solve(...)
```

当前 Wolfram 签名：

```text
LinearSolve[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
linear_solve(matrix, rhs, mode: "exact")
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |
| `rhs` | positional | 必填 | 只读右端函数或右端项 | r3 |
| `mode` | option | exact | exact/numeric | r3 |

当前已登记示例（Wolfram）：

```wolfram
LinearSolve[{{2,1},{1,3}},{1,2}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
linear_solve([[2,1],[1,3]],[1,2])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## lu

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000169`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

带置换的 LU

- 当前支持：机器实数矩形矩阵≤64×64，部分选主元LU，L(m×m)/U(m×n)满足P*A=L*U；返回1起始行置换及真实重构残差。奇异矩阵可以分解，求解时明确拒绝奇异/病态。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
lu(matrix)
```

当前 Wolfram 签名：

```text
Lu[matrix]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
lu(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Lu[{{0.,2.},{1.,3.}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
lu([[2.0,1.0],[1.0,3.0]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix.rs](../../crates/om-analysis/src/matrix.rs)。

当前测试引用：[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)、[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)。

## matrix_exp

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000363`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 MatrixExp 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`MatrixExp`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
matrix_exp(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## matrix_log

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000364`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 MatrixLog 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`MatrixLog`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
matrix_log(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## matrix_power

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000362`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 MatrixPower 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`MatrixPower`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
matrix_power(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## norm

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000155`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

向量范数

- 当前支持：非空数值向量≤64维，使用共轭内积的Euclidean范数，保留精确根式。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。 复数运算使用正确的共轭语义；机器夹角仅将舍入越界夹到[-1,1]。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
norm(...)
```

当前 Wolfram 签名：

```text
Norm[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
norm(vector, p: 2)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `vector` | positional | 必填 | 向量 | r3 |
| `p` | option | 2 | 正数或无穷 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Norm[{3,4}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
norm([3,4])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## normalize

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000156`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

按范数归一化

- 当前支持：非空数值向量≤64维，使用真实范数；拒绝零向量。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
normalize(...)
```

当前 Wolfram 签名：

```text
Normalize[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
normalize(vector)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `vector` | positional | 必填 | 向量 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Normalize[{3,4}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
normalize([3,4])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## null_space

**当前实现：部分支持；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000167`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

零空间基

- 当前支持：当前精确有理数矩阵≤64×64，复用Bareiss证书；奇异/欠定/不相容及维度明确诊断。机器矩阵路径仍在实施，不静默将机器系数当成精确保证。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
null_space(...)
```

当前 Wolfram 签名：

```text
NullSpace[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
null_space(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
NullSpace[{{1,2},{2,4}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
null_space([[1,2],[2,4]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## orthogonalize

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000361`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 Orthogonalize 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`Orthogonalize`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
orthogonalize(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## projection

**当前实现：下一版规划；目标接口：规划接口，当前不可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000158`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`documentation_only`，不构成工具授权。

向量投影

- 当前支持：当前无此规范接口的实现。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。 复数运算使用正确的共轭语义；机器夹角仅将舍入越界夹到[-1,1]。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：无；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
projection(a, onto)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `a` | positional | 必填 | 签名对应的a参数 | r3 |
| `onto` | positional | 必填 | 签名对应的onto参数 | r3 |

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
projection([1,2],[1,0])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## pseudo_inverse

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000365`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 PseudoInverse 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`PseudoInverse`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
pseudo_inverse(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## qr

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000170`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

QR 分解

- 当前支持：机器实数矩形矩阵≤64×64，完整Householder Q(m×m)/R(m×n)，返回真实重构残差。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
qr(matrix)
```

当前 Wolfram 签名：

```text
Qr[matrix]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
qr(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Qr[{{1.,2.},{3.,4.},{5.,6.}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
qr([[1.0,2.0],[3.0,4.0],[5.0,6.0]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix.rs](../../crates/om-analysis/src/matrix.rs)。

当前测试引用：[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)、[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)。

## rank

**当前实现：部分支持；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000166`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

矩阵秩

- 当前支持：当前精确有理数矩阵≤64×64，复用Bareiss证书；奇异/欠定/不相容及维度明确诊断。机器矩阵路径仍在实施，不静默将机器系数当成精确保证。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
rank(...)
```

当前 Wolfram 签名：

```text
MatrixRank[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
rank(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
MatrixRank[{{1,2},{2,4}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
rank([[1,2],[2,4]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## sparse_array

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000357`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 SparseArray 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`SparseArray`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
sparse_array(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## svd

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000172`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

奇异值分解

- 当前支持：有限实数矩形矩阵≤64×64，一侧Jacobi，完整u(m×m)/s(m×n)/v(n×n)，A=u@s@transpose(v)，values降序；实际重构residual与sweeps；不可表示旋转下的微小相关残列可视作数值零，独立微小奇异值保留，未收敛/非有限/不可表示极端尺度拒绝；不把残差作为严格认证。
- 目标范围：机器实数稠密矩阵≤64×64；LU 部分选主元、QR Householder、SVD 一侧Jacobi；实对称特征系统Jacobi，非对称/复矩阵特征系统本版不承诺。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
svd(matrix)
```

当前 Wolfram 签名：

```text
Svd[matrix]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
svd(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Svd[{{1.,2.},{3.,4.},{5.,6.}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
svd([[1.0,2.0],[3.0,4.0]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/matrix_numeric.rs](../../crates/om-eval/src/science/matrix_numeric.rs)、[crates/om-analysis/src/matrix/svd.rs](../../crates/om-analysis/src/matrix/svd.rs)。

当前测试引用：[crates/om-analysis/tests/matrix.rs](../../crates/om-analysis/tests/matrix.rs)、[crates/om-eval/tests/numeric_matrix.rs](../../crates/om-eval/tests/numeric_matrix.rs)。

## tensor_contract

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000367`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 TensorContract 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TensorContract`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
tensor_contract(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## tensor_dimensions

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000369`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 TensorDimensions 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TensorDimensions`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
tensor_dimensions(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## tensor_product

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000366`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 TensorProduct 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TensorProduct`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
tensor_product(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## tensor_rank

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000370`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 TensorRank 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TensorRank`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
tensor_rank(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## tensor_transpose

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000368`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 TensorTranspose 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`TensorTranspose`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
tensor_transpose(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## trace

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000163`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

矩阵迹

- 当前支持：方阵≤64×64，真实对角元素求和。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
trace(...)
```

当前 Wolfram 签名：

```text
Tr[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
trace(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Tr[{{1,2},{3,4}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
trace([[1,2],[3,4]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## transpose

**当前实现：已实现；目标接口：当前可用。** 目标版本：`0.1.0-pre-alpha.3`。

- 稳定身份：`fn_000161`；条目类型：`function`。
- 副作用分类（设计预留）：`pure`；参数验证阶段：`runtime_verified`，不构成工具授权。

转置

- 当前支持：矩形矩阵≤64×64，原元素不改变；不把一般张量当作已适配。
- 目标范围：维度检查；精确有理数或机器实数的小型稠密矩阵，继承预算；零向量/奇异矩阵明确诊断。
- 返回：scalar_vector_matrix_or_decomposition
- 精度：精确有理基础路径与明确的机器分解；分解不承诺任意精度。
- 当前计算平台：cli, desktop, web, ios；目标计算平台：cli, desktop, web, ios。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：第 1 个位置参数（从 1 起）。

当前现代签名：

```text
transpose(...)
```

当前 Wolfram 签名：

```text
Transpose[...]
```

目标现代签名（按目标接口状态判断是否已可执行）：

```text
transpose(matrix)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `matrix` | positional | 必填 | 矩阵/向量或签名的主要输入 | r3 |

当前已登记示例（Wolfram）：

```wolfram
Transpose[{{1,2},{3,4}}]
```

规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：

```text
transpose([[1,2],[3,4]])
```

验收：独立数学期望、有效/无效参数、边界、预算、中断及声明的平台/精度测试；范围外不伪造成功。

当前源码：[crates/om-eval/src/science/registry.rs](../../crates/om-eval/src/science/registry.rs)。

当前测试引用：[crates/om-eval/tests/science_basics.rs](../../crates/om-eval/tests/science_basics.rs)。

## unit_vector

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000359`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 UnitVector 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`UnitVector`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
unit_vector(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## vector_angle

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000360`；条目类型：`function`。
- 副作用分类（设计预留）：`unclassified`；参数验证阶段：`documentation_only`，不构成工具授权。

向量、矩阵与张量中的 VectorAngle 能力，进入后续全景目录。

- 当前支持：当前无此规范接口的实现。
- 目标范围：下一版不承诺。实施前独立锁定算法、完整参数与默认值、结果类型、精度和平台；当前不会注册为可执行函数。
- 返回：后续阶段规格确定
- 精度：当前无实现，不声称精确、机器或任意精度能力。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：`VectorAngle`。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
vector_angle(...)  # 后续接口尚未锁定
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| 无 | — | — | 无参数 | — |

验收：后续阶段须新增独立参考期望、错误/边界/中断用例，再更新状态与接口；不得只添加名字或示例。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

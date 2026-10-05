<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# 当前可执行接口与参数 schema

[全景目录](README.md) · [下一版账本](../plan/NEXT_RELEASE.md)

描述版本 11；下列均有真实回调。现代组合语法及已有回调的 mode/output 已接通，后续数学能力仍须按 R3.3–R3.6 交付。参数类型约束用于字面输入；符号与表达式在真实回调求值后检查。默认表达式仅描述省略行为，不自动插入参数；上下文默认值不伪装成字面值。副作用标签只描述入口，不能授权嵌套函数或替代只读隔离。

## abs

稳定身份 `fn_000001`；回调 `Abs`；归属 `abs`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#abs)。

## And

稳定身份 `fn_000002`；回调 `And`；归属 `and`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#and)。

## apart

稳定身份 `fn_000003`；回调 `Apart`；归属 `apart`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](algebra.md#apart)。

## append

稳定身份 `fn_000004`；回调 `Append`；归属 `append`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#append)。

## apply

稳定身份 `fn_000005`；回调 `Apply`；归属 `apply`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：2；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `fn` | 位置 | expression | 是 | 必填 |  |
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#apply)。

## acos

稳定身份 `fn_000006`；回调 `ArcCos`；归属 `acos`。兼容拼写：`arccos`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#acos)。

## acosh

稳定身份 `fn_000007`；回调 `ArcCosh`；归属 `acosh`。兼容拼写：`arccosh`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#acosh)。

## acot

稳定身份 `fn_000008`；回调 `ArcCot`；归属 `acot`。兼容拼写：`arccot`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#acot)。

## acsc

稳定身份 `fn_000009`；回调 `ArcCsc`；归属 `acsc`。兼容拼写：`arccsc`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#acsc)。

## asec

稳定身份 `fn_000010`；回调 `ArcSec`；归属 `asec`。兼容拼写：`arcsec`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#asec)。

## asin

稳定身份 `fn_000011`；回调 `ArcSin`；归属 `asin`。兼容拼写：`arcsin`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#asin)。

## asinh

稳定身份 `fn_000012`；回调 `ArcSinh`；归属 `asinh`。兼容拼写：`arcsinh`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#asinh)。

## atan

稳定身份 `fn_000013`；回调 `ArcTan`；归属 `atan`。兼容拼写：`arctan`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |
| `y` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#atan)。

## atanh

稳定身份 `fn_000014`；回调 `ArcTanh`；归属 `atanh`。兼容拼写：`arctanh`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#atanh)。

## arg

稳定身份 `fn_000015`；回调 `Arg`；归属 `arg`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#arg)。

## choose

稳定身份 `fn_000016`；回调 `Binomial`；归属 `choose`。兼容拼写：`binom`, `binomial`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `n` | 位置 | expression | 是 | 必填 |  |
| `k` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#choose)。

## cancel

稳定身份 `fn_000017`；回调 `Cancel`；归属 `cancel`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#cancel)。

## ceil

稳定身份 `fn_000018`；回调 `Ceiling`；归属 `ceil`。兼容拼写：`ceiling`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |
| `step` | 位置 | expression | 否 | 1 |  |
| `step` | 命名 | expression | 否 | 1 |  |

数学边界与精度：[所属条目](basics.md#ceil)。

## clear

稳定身份 `fn_000019`；回调 `Clear`；归属 `clear`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `symbols` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#clear)。

## coefficient

稳定身份 `fn_000020`；回调 `Coefficient`；归属 `coefficient`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |
| `order` | 位置 | expression | 否 | 1 |  |
| `order` | 命名 | integer | 否 | 1 | ; ≥0 |

数学边界与精度：[所属条目](algebra.md#coefficient)。

## coefficient_list

稳定身份 `fn_000021`；回调 `CoefficientList`；归属 `coefficient_list`。兼容拼写：`coefficientlist`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#coefficient_list)。

## collect

稳定身份 `fn_000022`；回调 `Collect`；归属 `collect`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 是 | 必填 |  |
| `coefficient_fn` | 位置 | expression | 否 | Identity |  |
| `coefficient_fn` | 命名 | expression | 否 | 省略时不转换系数 |  |

数学边界与精度：[所属条目](algebra.md#collect)。

## sequence

稳定身份 `fn_000023`；回调 `CompoundExpression`；归属 `sequence`。兼容拼写：`compoundexpression`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expressions` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#sequence)。

## conditional

稳定身份 `fn_000024`；回调 `ConditionalExpression`；归属 `conditional`。兼容拼写：`conditionalexpression`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `condition` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](solving.md#conditional)。

## conj

稳定身份 `fn_000025`；回调 `Conjugate`；归属 `conj`。兼容拼写：`conjugate`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#conj)。

## implicit_plot

稳定身份 `fn_000119`；回调 `ContourPlot`；归属 `plot`。兼容拼写：`contourplot`, `implicitplot`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equation` | 位置 | expression | 是 | 必填 |  |
| `x_axis` | 位置 | expression | 是 | 必填 |  |
| `y_axis` | 位置 | expression | 是 | 必填 |  |
| `plot_range` | 命名 | expression | 否 | Automatic |  |

数学边界与精度：[所属条目](plots.md#plot)。

## cos

稳定身份 `fn_000026`；回调 `Cos`；归属 `cos`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#cos)。

## cosh

稳定身份 `fn_000027`；回调 `Cosh`；归属 `cosh`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#cosh)。

## cot

稳定身份 `fn_000028`；回调 `Cot`；归属 `cot`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#cot)。

## coth

稳定身份 `fn_000029`；回调 `Coth`；归属 `coth`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#coth)。

## csc

稳定身份 `fn_000030`；回调 `Csc`；归属 `csc`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#csc)。

## csch

稳定身份 `fn_000031`；回调 `Csch`；归属 `csch`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#csch)。

## diff

稳定身份 `fn_000032`；回调 `D`；归属 `diff`。兼容拼写：`derivative`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `order` | 命名 | integer | 否 | 1 | ; ≥0 |

数学边界与精度：[所属条目](calculus.md#diff)。

## denominator

稳定身份 `fn_000033`；回调 `Denominator`；归属 `denominator`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#denominator)。

## discriminant

稳定身份 `fn_000034`；回调 `Discriminant`；归属 `discriminant`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `f` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#discriminant)。

## divide

稳定身份 `fn_000035`；回调 `Divide`；归属 `divide`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#divide)。

## in_domain

稳定身份 `fn_000036`；回调 `Element`；归属 `in_domain`。兼容拼写：`element`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `domain` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#in_domain)。

## eliminate

稳定身份 `fn_000037`；回调 `Eliminate`；归属 `eliminate`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](solving.md#eliminate)。

## equal

稳定身份 `fn_000038`；回调 `Equal`；归属 `equal`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#equal)。

## exp

稳定身份 `fn_000039`；回调 `Exp`；归属 `exp`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#exp)。

## expand

稳定身份 `fn_000040`；回调 `Expand`；归属 `expand`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#expand)。

## exponent

稳定身份 `fn_000041`；回调 `Exponent`；归属 `exponent`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#exponent)。

## factor

稳定身份 `fn_000042`；回调 `Factor`；归属 `factor`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#factor)。

## factor_integer

稳定身份 `fn_000043`；回调 `FactorInteger`；归属 `factor_integer`。兼容拼写：`factorinteger`。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#factor_integer)。

## factorial

稳定身份 `fn_000044`；回调 `Factorial`；归属 `factorial`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#factorial)。

## find_root

稳定身份 `fn_000045`；回调 `FindRoot`；归属 `find_root`。兼容拼写：`findroot`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `starts` | 位置 | expression | 是 | 必填 |  |
| `initial` | 命名 | expression | 否 | 与bracket互斥；配合裸变量 |  |
| `bracket` | 命名 | expression | 否 | 与initial互斥；配合裸变量和两端点区间 |  |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `precision` | 命名 | integer | 否 | MachinePrecision | MachinePrecision, machine; ≥5; ≤2466 |
| `method` | 命名 | enum | 否 | Automatic | Automatic, Newton, Brent |
| `max_iterations` | 命名 | integer | 否 | 100 | ; ≥1 |

数学边界与精度：[所属条目](solving.md#find_root)。

## first

稳定身份 `fn_000046`；回调 `First`；归属 `first`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#first)。

## floor

稳定身份 `fn_000047`；回调 `Floor`；归属 `floor`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |
| `step` | 位置 | expression | 否 | 1 |  |
| `step` | 命名 | expression | 否 | 1 |  |

数学边界与精度：[所属条目](basics.md#floor)。

## full_simplify

稳定身份 `fn_000118`；回调 `FullSimplify`；归属 `simplify`。兼容拼写：`fullsimplify`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `arg2` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `assumptions` | 命名 | expression | 否 | True |  |

数学边界与精度：[所属条目](algebra.md#simplify)。

## function

稳定身份 `fn_000048`；回调 `Function`；归属 `function`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：0；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `parameters_or_body` | 位置 | expression | 是 | 必填 |  |
| `body` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](language.md#function)。

## gcd

稳定身份 `fn_000049`；回调 `GCD`；归属 `gcd`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `integers` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#gcd)。

## greater

稳定身份 `fn_000050`；回调 `Greater`；归属 `greater`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `b` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#greater)。

## greater_equal

稳定身份 `fn_000051`；回调 `GreaterEqual`；归属 `greater_equal`。兼容拼写：`greaterequal`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `b` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#greater_equal)。

## hold

稳定身份 `fn_000052`；回调 `Hold`；归属 `hold`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](language.md#hold)。

## hold_form

稳定身份 `fn_000053`；回调 `HoldForm`；归属 `hold_form`。兼容拼写：`holdform`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#hold_form)。

## im

稳定身份 `fn_000054`；回调 `Im`；归属 `im`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#im)。

## inequality

稳定身份 `fn_000055`；回调 `Inequality`；归属 `inequality`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置 | expression | 是 | 必填 |  |
| `arg2` | 位置 | expression | 是 | 必填 |  |
| `arg3` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#inequality)。

## lcm

稳定身份 `fn_000056`；回调 `LCM`；归属 `lcm`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `integers` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#lcm)。

## last

稳定身份 `fn_000057`；回调 `Last`；归属 `last`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#last)。

## len

稳定身份 `fn_000058`；回调 `Length`；归属 `len`。兼容拼写：`length`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#len)。

## less

稳定身份 `fn_000059`；回调 `Less`；归属 `less`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `b` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#less)。

## less_equal

稳定身份 `fn_000060`；回调 `LessEqual`；归属 `less_equal`。兼容拼写：`lessequal`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `b` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#less_equal)。

## list

稳定身份 `fn_000061`；回调 `List`；归属 `list`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `items` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](data.md#list)。

## log

稳定身份 `fn_000062`；回调 `Log`；归属 `log`。兼容拼写：`ln`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `base_or_value` | 位置 | expression | 是 | 必填 |  |
| `value` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#log)。

## map

稳定身份 `fn_000063`；回调 `Map`；归属 `map`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：2；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `fn` | 位置 | expression | 是 | 必填 |  |
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#map)。

## negate

稳定身份 `fn_000064`；回调 `Minus`；归属 `negate`。兼容拼写：`minus`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#negate)。

## mod

稳定身份 `fn_000065`；回调 `Mod`；归属 `mod`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `n` | 位置 | expression | 是 | 必填 |  |
| `divisor` | 位置 | expression | 是 | 必填 |  |
| `offset` | 位置 | expression | 否 | 0 |  |

数学边界与精度：[所属条目](basics.md#mod)。

## numeric

稳定身份 `fn_000066`；回调 `N`；归属 `numeric`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `precision` | 位置 | expression | 否 | MachinePrecision |  |
| `precision` | 命名 | integer | 否 | MachinePrecision | machine; ≥1; ≤4932 |

数学边界与精度：[所属条目](basics.md#numeric)。

## nsolve

稳定身份 `fn_000117`；回调 `NSolve`；归属 `solve`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `domain` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `domain` | 命名 | enum | 否 | Complexes | Complexes, Reals, Integers, Rationals, positives |
| `cubics` | 命名 | boolean | 否 | False |  |
| `quartics` | 命名 | boolean | 否 | False |  |
| `verify_solutions` | 命名 | enum | 否 | Automatic | True, False, Automatic |
| `max_extra_conditions` | 命名 | integer | 否 | 0 | All; ≥0 |
| `generated_parameters` | 命名 | symbol | 否 | C |  |
| `inverse_functions` | 命名 | boolean | 否 | True |  |
| `precision` | 命名 | integer | 否 | MachinePrecision | MachinePrecision, machine; ≥5; ≤2466 |

历史仅接受语法的选项：`method`, `max_iterations`；不作为当前可用选项推荐，回调继续给出不支持诊断。

数学边界与精度：[所属条目](solving.md#solve)。

## nsolve_values

稳定身份 `fn_000117`；回调 `NSolveValues`；归属 `solve`。兼容拼写：`nsolvevalues`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `domain` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `domain` | 命名 | enum | 否 | Complexes | Complexes, Reals, Integers, Rationals, positives |
| `cubics` | 命名 | boolean | 否 | False |  |
| `quartics` | 命名 | boolean | 否 | False |  |
| `verify_solutions` | 命名 | enum | 否 | Automatic | True, False, Automatic |
| `max_extra_conditions` | 命名 | integer | 否 | 0 | All; ≥0 |
| `generated_parameters` | 命名 | symbol | 否 | C |  |
| `inverse_functions` | 命名 | boolean | 否 | True |  |
| `precision` | 命名 | integer | 否 | MachinePrecision | MachinePrecision, machine; ≥5; ≤2466 |

历史仅接受语法的选项：`method`, `max_iterations`；不作为当前可用选项推荐，回调继续给出不支持诊断。

数学边界与精度：[所属条目](solving.md#solve)。

## not

稳定身份 `fn_000067`；回调 `Not`；归属 `not`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#not)。

## numerator

稳定身份 `fn_000068`；回调 `Numerator`；归属 `numerator`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#numerator)。

## Or

稳定身份 `fn_000069`；回调 `Or`；归属 `or`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#or)。

## out

稳定身份 `fn_000070`；回调 `Out`；归属 `out`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`read_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `index` | 位置 | expression | 否 | -1 |  |

数学边界与精度：[所属条目](language.md#out)。

## at

稳定身份 `fn_000071`；回调 `Part`；归属 `at`。兼容拼写：`part`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `indices` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#at)。

## plot

稳定身份 `fn_000119`；回调 `Plot`；归属 `plot`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `axis` | 位置 | expression | 是 | 必填 |  |
| `view` | 命名 | enum | 否 | "line" | line, contour |
| `plot_range` | 命名 | expression | 否 | Automatic |  |

数学边界与精度：[所属条目](plots.md#plot)。

## add

稳定身份 `fn_000072`；回调 `Plus`；归属 `add`。兼容拼写：`plus`。

保持属性：listable, protected, numeric_function, flat, orderless, one_identity；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#add)。

## polynomial_gcd

稳定身份 `fn_000073`；回调 `PolynomialGCD`；归属 `polynomial_gcd`。兼容拼写：`polynomialgcd`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `polynomials` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#polynomial_gcd)。

## polynomial_lcm

稳定身份 `fn_000074`；回调 `PolynomialLCM`；归属 `polynomial_lcm`。兼容拼写：`polynomiallcm`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `polynomials` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#polynomial_lcm)。

## is_polynomial

稳定身份 `fn_000075`；回调 `PolynomialQ`；归属 `is_polynomial`。兼容拼写：`polynomialq`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#is_polynomial)。

## polynomial_quotient

稳定身份 `fn_000076`；回调 `PolynomialQuotient`；归属 `polynomial_quotient`。兼容拼写：`polynomialquotient`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `f` | 位置 | expression | 是 | 必填 |  |
| `g` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#polynomial_quotient)。

## polynomial_remainder

稳定身份 `fn_000077`；回调 `PolynomialRemainder`；归属 `polynomial_remainder`。兼容拼写：`polynomialremainder`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `f` | 位置 | expression | 是 | 必填 |  |
| `g` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#polynomial_remainder)。

## power

稳定身份 `fn_000078`；回调 `Power`；归属 `power`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `base` | 位置 | expression | 是 | 必填 |  |
| `exponent` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#power)。

## is_prime

稳定身份 `fn_000079`；回调 `PrimeQ`；归属 `is_prime`。兼容拼写：`primeq`。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#is_prime)。

## product

稳定身份 `fn_000080`；回调 `Product`；归属 `product`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `iterators` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#product)。

## lambert_w

稳定身份 `fn_000081`；回调 `ProductLog`；归属 `lambert_w`。兼容拼写：`lambertw`, `productlog`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#lambert_w)。

## quotient

稳定身份 `fn_000082`；回调 `Quotient`；归属 `quotient`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |
| `divisor` | 位置 | expression | 是 | 必填 |  |
| `arg3` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#quotient)。

## range

稳定身份 `fn_000083`；回调 `Range`；归属 `range`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `start_or_end` | 位置 | expression | 是 | 必填 |  |
| `end` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `step` | 位置 | expression | 否 | 1 |  |
| `step` | 命名 | expression | 否 | 1 |  |

数学边界与精度：[所属条目](data.md#range)。

## re

稳定身份 `fn_000084`；回调 `Re`；归属 `re`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#re)。

## reduce

稳定身份 `fn_000085`；回调 `Reduce`；归属 `reduce`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `domain` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `domain` | 命名 | enum | 否 | Complexes | Complexes, Reals, Integers, Rationals, positives |
| `cubics` | 命名 | boolean | 否 | False |  |
| `quartics` | 命名 | boolean | 否 | False |  |
| `verify_solutions` | 命名 | enum | 否 | Automatic | True, False, Automatic |
| `max_extra_conditions` | 命名 | integer | 否 | 0 | All; ≥0 |
| `generated_parameters` | 命名 | symbol | 否 | C |  |
| `inverse_functions` | 命名 | boolean | 否 | True |  |

历史仅接受语法的选项：`method`, `max_iterations`；不作为当前可用选项推荐，回调继续给出不支持诊断。

数学边界与精度：[所属条目](solving.md#reduce)。

## substitute

稳定身份 `fn_000086`；回调 `ReplaceAll`；归属 `substitute`。兼容拼写：`replaceall`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `rules` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#substitute)。

## rewrite

稳定身份 `fn_000087`；回调 `ReplaceRepeated`；归属 `rewrite`。兼容拼写：`replacerepeated`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `rules` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#rewrite)。

## rest

稳定身份 `fn_000088`；回调 `Rest`；归属 `rest`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#rest)。

## resultant

稳定身份 `fn_000089`；回调 `Resultant`；归属 `resultant`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `f` | 位置 | expression | 是 | 必填 |  |
| `g` | 位置 | expression | 是 | 必填 |  |
| `variable` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#resultant)。

## algebraic_root

稳定身份 `fn_000090`；回调 `Root`；归属 `algebraic_root`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `fn` | 位置 | expression | 是 | 必填 |  |
| `index` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](solving.md#algebraic_root)。

## root_reduce

稳定身份 `fn_000091`；回调 `RootReduce`；归属 `root_reduce`。兼容拼写：`rootreduce`。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#root_reduce)。

## roots

稳定身份 `fn_000092`；回调 `Roots`；归属 `roots`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `domain` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `domain` | 命名 | enum | 否 | Complexes | Complexes, Reals, Integers, Rationals, positives |
| `cubics` | 命名 | boolean | 否 | False |  |
| `quartics` | 命名 | boolean | 否 | False |  |
| `verify_solutions` | 命名 | enum | 否 | Automatic | True, False, Automatic |
| `max_extra_conditions` | 命名 | integer | 否 | 0 | All; ≥0 |
| `generated_parameters` | 命名 | symbol | 否 | C |  |
| `inverse_functions` | 命名 | boolean | 否 | True |  |

历史仅接受语法的选项：`method`, `max_iterations`；不作为当前可用选项推荐，回调继续给出不支持诊断。

数学边界与精度：[所属条目](solving.md#roots)。

## round

稳定身份 `fn_000093`；回调 `Round`；归属 `round`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |
| `step` | 位置 | expression | 否 | 1 |  |
| `step` | 命名 | expression | 否 | 1 |  |

数学边界与精度：[所属条目](basics.md#round)。

## rule

稳定身份 `fn_000094`；回调 `Rule`；归属 `rule`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `lhs` | 位置 | expression | 是 | 必填 |  |
| `rhs` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#rule)。

## delayed_rule

稳定身份 `fn_000095`；回调 `RuleDelayed`；归属 `delayed_rule`。兼容拼写：`ruledelayed`。

保持属性：hold_rest, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `lhs` | 位置 | expression | 是 | 必填 |  |
| `rhs` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#delayed_rule)。

## same

稳定身份 `fn_000096`；回调 `SameQ`；归属 `same`。兼容拼写：`sameq`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `b` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#same)。

## sec

稳定身份 `fn_000097`；回调 `Sec`；归属 `sec`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#sec)。

## sech

稳定身份 `fn_000098`；回调 `Sech`；归属 `sech`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#sech)。

## assign

稳定身份 `fn_000099`；回调 `Set`；归属 `assign`。兼容拼写：`set`。

保持属性：hold_first, protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `name` | 位置 | expression | 是 | 必填 |  |
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#assign)。

## define

稳定身份 `fn_000100`；回调 `SetDelayed`；归属 `define`。兼容拼写：`setdelayed`。

保持属性：hold_all, protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `target` | 位置 | expression | 是 | 必填 |  |
| `body` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#define)。

## sign

稳定身份 `fn_000101`；回调 `Sign`；归属 `sign`。兼容拼写：`sgn`。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#sign)。

## simplify

稳定身份 `fn_000118`；回调 `Simplify`；归属 `simplify`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `arg2` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `level` | 命名 | enum | 否 | "basic" | basic, deep |
| `assumptions` | 命名 | expression | 否 | True |  |

数学边界与精度：[所属条目](algebra.md#simplify)。

## sin

稳定身份 `fn_000102`；回调 `Sin`；归属 `sin`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#sin)。

## sinh

稳定身份 `fn_000103`；回调 `Sinh`；归属 `sinh`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#sinh)。

## slot

稳定身份 `fn_000104`；回调 `Slot`；归属 `slot`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：0；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `index` | 位置 | expression | 是 | 1 |  |

数学边界与精度：[所属条目](language.md#slot)。

## solve

稳定身份 `fn_000117`；回调 `Solve`；归属 `solve`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `domain` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `mode` | 命名 | enum | 否 | "exact" | exact, numeric |
| `output` | 命名 | enum | 否 | "rules" | rules, values |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `domain` | 命名 | enum | 否 | Complexes | Complexes, Reals, Integers, Rationals, positives |
| `cubics` | 命名 | boolean | 否 | False |  |
| `quartics` | 命名 | boolean | 否 | False |  |
| `verify_solutions` | 命名 | enum | 否 | Automatic | True, False, Automatic |
| `max_extra_conditions` | 命名 | integer | 否 | 0 | All; ≥0 |
| `generated_parameters` | 命名 | symbol | 否 | C |  |
| `inverse_functions` | 命名 | boolean | 否 | True |  |

历史仅接受语法的选项：`method`, `max_iterations`；不作为当前可用选项推荐，回调继续给出不支持诊断。

数学边界与精度：[所属条目](solving.md#solve)。

## solve_values

稳定身份 `fn_000117`；回调 `SolveValues`；归属 `solve`。兼容拼写：`solvevalues`。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `equations` | 位置 | expression | 是 | 必填 |  |
| `variables` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `domain` | 位置 | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |
| `steps` | 命名 | boolean | 否 | 继承当前 EvalSettings.record_steps |  |
| `domain` | 命名 | enum | 否 | Complexes | Complexes, Reals, Integers, Rationals, positives |
| `cubics` | 命名 | boolean | 否 | False |  |
| `quartics` | 命名 | boolean | 否 | False |  |
| `verify_solutions` | 命名 | enum | 否 | Automatic | True, False, Automatic |
| `max_extra_conditions` | 命名 | integer | 否 | 0 | All; ≥0 |
| `generated_parameters` | 命名 | symbol | 否 | C |  |
| `inverse_functions` | 命名 | boolean | 否 | True |  |

历史仅接受语法的选项：`method`, `max_iterations`；不作为当前可用选项推荐，回调继续给出不支持诊断。

数学边界与精度：[所属条目](solving.md#solve)。

## sqrt

稳定身份 `fn_000105`；回调 `Sqrt`；归属 `sqrt`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#sqrt)。

## subtract

稳定身份 `fn_000106`；回调 `Subtract`；归属 `subtract`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#subtract)。

## sum

稳定身份 `fn_000107`；回调 `Sum`；归属 `sum`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `iterators` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#sum)。

## table

稳定身份 `fn_000108`；回调 `Table`；归属 `table`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `iterators` | 位置（重复） | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#table)。

## tan

稳定身份 `fn_000109`；回调 `Tan`；归属 `tan`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#tan)。

## tanh

稳定身份 `fn_000110`；回调 `Tanh`；归属 `tanh`。兼容拼写：无其他现代拼写。

保持属性：listable, protected, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#tanh)。

## multiply

稳定身份 `fn_000111`；回调 `Times`；归属 `multiply`。兼容拼写：`times`。

保持属性：listable, protected, numeric_function, flat, orderless, one_identity；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#multiply)。

## to_radicals

稳定身份 `fn_000112`；回调 `ToRadicals`；归属 `to_radicals`。兼容拼写：`toradicals`。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#to_radicals)。

## together

稳定身份 `fn_000113`；回调 `Together`；归属 `together`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#together)。

## unequal

稳定身份 `fn_000114`；回调 `Unequal`；归属 `unequal`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 由真实回调按输入推断或省略；不填造字面默认值 |  |

数学边界与精度：[所属条目](basics.md#unequal)。

## unset

稳定身份 `fn_000115`；回调 `Unset`；归属 `unset`。兼容拼写：无其他现代拼写。

保持属性：hold_all, protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `target` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](language.md#unset)。

## variables

稳定身份 `fn_000116`；回调 `Variables`；归属 `variables`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](algebra.md#variables)。

## record

稳定身份 `fn_000210`；回调 `Record`；归属 `record`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `entries` | 位置（重复） | expression | 否 | 允许空记录 |  |

数学边界与精度：[所属条目](data.md#record)。

## dot

稳定身份 `fn_000153`；回调 `Dot`；归属 `dot`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#dot)。

## min

稳定身份 `fn_000120`；回调 `Min`；归属 `min`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 按位置接收列表或重复数据参数 |  |

数学边界与精度：[所属条目](basics.md#min)。

## max

稳定身份 `fn_000121`；回调 `Max`；归属 `max`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置（重复） | expression | 否 | 按位置接收列表或重复数据参数 |  |

数学边界与精度：[所属条目](basics.md#max)。

## minmax

稳定身份 `fn_000122`；回调 `MinMax`；归属 `minmax`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#minmax)。

## integer_part

稳定身份 `fn_000123`；回调 `IntegerPart`；归属 `integer_part`。兼容拼写：`integerpart`。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#integer_part)。

## fractional_part

稳定身份 `fn_000124`；回调 `FractionalPart`；归属 `fractional_part`。兼容拼写：`fractionalpart`。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#fractional_part)。

## precision

稳定身份 `fn_000125`；回调 `Precision`；归属 `precision`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#precision)。

## accuracy

稳定身份 `fn_000126`；回调 `Accuracy`；归属 `accuracy`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#accuracy)。

## chop

稳定身份 `fn_000127`；回调 `Chop`；归属 `chop`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `expr` | 位置 | expression | 是 | 必填 |  |
| `tolerance` | 命名 | real | 否 | 1e-10 | ; ≥0 |

数学边界与精度：[所属条目](basics.md#chop)。

## rationalize

稳定身份 `fn_000128`；回调 `Rationalize`；归属 `rationalize`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |
| `tolerance` | 命名 | real | 否 | 0 | ; ≥0 |

数学边界与精度：[所属条目](basics.md#rationalize)。

## clip

稳定身份 `fn_000139`；回调 `Clip`；归属 `clip`。兼容拼写：无其他现代拼写。

保持属性：listable, protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |
| `bounds` | 命名 | expression | 否 | {-1,1} |  |

数学边界与精度：[所属条目](basics.md#clip)。

## mean

稳定身份 `fn_000181`；回调 `Mean`；归属 `mean`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#mean)。

## median

稳定身份 `fn_000182`；回调 `Median`；归属 `median`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#median)。

## variance

稳定身份 `fn_000183`；回调 `Variance`；归属 `variance`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `sample` | 命名 | boolean | 否 | True |  |

数学边界与精度：[所属条目](statistics.md#variance)。

## std

稳定身份 `fn_000184`；回调 `StandardDeviation`；归属 `std`。兼容拼写：`standarddeviation`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `sample` | 命名 | boolean | 否 | True |  |

数学边界与精度：[所属条目](statistics.md#std)。

## covariance

稳定身份 `fn_000186`；回调 `Covariance`；归属 `covariance`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |
| `sample` | 命名 | boolean | 否 | True |  |

数学边界与精度：[所属条目](statistics.md#covariance)。

## correlation

稳定身份 `fn_000187`；回调 `Correlation`；归属 `correlation`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#correlation)。

## quantile

稳定身份 `fn_000189`；回调 `Quantile`；归属 `quantile`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data_or_distribution` | 位置 | expression | 是 | 必填 |  |
| `probability` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#quantile)。

## percentile

稳定身份 `fn_000185`；回调 `Percentile`；归属 `percentile`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `percent` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#percentile)。

## filter

稳定身份 `fn_000198`；回调 `Filter`；归属 `filter`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `predicate` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#filter)。

## sort

稳定身份 `fn_000199`；回调 `Sort`；归属 `sort`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#sort)。

## sort_by

稳定身份 `fn_000200`；回调 `SortBy`；归属 `sort_by`。兼容拼写：`sortby`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `key_fn` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#sort_by)。

## unique

稳定身份 `fn_000201`；回调 `Unique`；归属 `unique`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#unique)。

## take

稳定身份 `fn_000202`；回调 `Take`；归属 `take`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `count` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#take)。

## drop

稳定身份 `fn_000203`；回调 `Drop`；归属 `drop`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `count` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#drop)。

## slice

稳定身份 `fn_000204`；回调 `Slice`；归属 `slice`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `range` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#slice)。

## flatten

稳定身份 `fn_000205`；回调 `Flatten`；归属 `flatten`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `depth` | 命名 | integer | 否 | All | All; ≥0 |

数学边界与精度：[所属条目](data.md#flatten)。

## reshape

稳定身份 `fn_000206`；回调 `Reshape`；归属 `reshape`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `dimensions` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#reshape)。

## zip

稳定身份 `fn_000207`；回调 `Zip`；归属 `zip`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `lists` | 位置（重复） | expression | 是 | 按位置接收列表或重复数据参数 |  |

数学边界与精度：[所属条目](data.md#zip)。

## fold

稳定身份 `fn_000208`；回调 `Fold`；归属 `fold`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：3；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `fn` | 位置 | expression | 是 | 必填 |  |
| `initial` | 位置 | expression | 是 | 必填 |  |
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#fold)。

## group_by

稳定身份 `fn_000209`；回调 `GroupBy`；归属 `group_by`。兼容拼写：`groupby`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `key_fn` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#group_by)。

## counts

稳定身份 `fn_000188`；回调 `Counts`；归属 `counts`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#counts)。

## identity

稳定身份 `fn_000159`；回调 `IdentityMatrix`；归属 `identity`。兼容拼写：`identitymatrix`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `size` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#identity)。

## diag

稳定身份 `fn_000160`；回调 `DiagonalMatrix`；归属 `diag`。兼容拼写：`diagonalmatrix`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `values` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#diag)。

## transpose

稳定身份 `fn_000161`；回调 `Transpose`；归属 `transpose`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#transpose)。

## adjoint

稳定身份 `fn_000162`；回调 `ConjugateTranspose`；归属 `adjoint`。兼容拼写：`conjugatetranspose`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#adjoint)。

## trace

稳定身份 `fn_000163`；回调 `Tr`；归属 `trace`。兼容拼写：`tr`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#trace)。

## det

稳定身份 `fn_000164`；回调 `Det`；归属 `det`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#det)。

## inverse

稳定身份 `fn_000165`；回调 `Inverse`；归属 `inverse`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#inverse)。

## rank

稳定身份 `fn_000166`；回调 `MatrixRank`；归属 `rank`。兼容拼写：`matrixrank`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#rank)。

## null_space

稳定身份 `fn_000167`；回调 `NullSpace`；归属 `null_space`。兼容拼写：`nullspace`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#null_space)。

## linear_solve

稳定身份 `fn_000168`；回调 `LinearSolve`；归属 `linear_solve`。兼容拼写：`linearsolve`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |
| `rhs` | 位置 | expression | 是 | 必填 |  |
| `mode` | 命名 | enum | 否 | "exact" | exact, numeric |

数学边界与精度：[所属条目](linear.md#linear_solve)。

## cross

稳定身份 `fn_000154`；回调 `Cross`；归属 `cross`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#cross)。

## norm

稳定身份 `fn_000155`；回调 `Norm`；归属 `norm`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `vector` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#norm)。

## normalize

稳定身份 `fn_000156`；回调 `Normalize`；归属 `normalize`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `vector` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#normalize)。

## decimal

稳定身份 `fn_000141`；回调 `Decimal`；归属 `decimal`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `text` | 位置 | expression | 是 | 必填 |  |
| `precision` | 命名 | integer | 否 | 50 | ; ≥1; ≤4931 |

数学边界与精度：[所属条目](basics.md#decimal)。

## rescale

稳定身份 `fn_000140`；回调 `Rescale`；归属 `rescale`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |
| `from` | 命名 | expression | 否 | 从真实数据最小/最大值推断；零宽区间拒绝 |  |
| `to` | 命名 | expression | 否 | {0,1} |  |

数学边界与精度：[所属条目](basics.md#rescale)。

## lu

稳定身份 `fn_000169`；回调 `Lu`；归属 `lu`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#lu)。

## qr

稳定身份 `fn_000170`；回调 `Qr`；归属 `qr`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#qr)。

## least_squares

稳定身份 `fn_000175`；回调 `LeastSquares`；归属 `least_squares`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |
| `rhs` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#least_squares)。

## cholesky

稳定身份 `fn_000171`；回调 `Cholesky`；归属 `cholesky`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#cholesky)。

## eigenvalues

稳定身份 `fn_000173`；回调 `Eigenvalues`；归属 `eigenvalues`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#eigenvalues)。

## eigensystem

稳定身份 `fn_000174`；回调 `Eigensystem`；归属 `eigensystem`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#eigensystem)。

## svd

稳定身份 `fn_000172`；回调 `Svd`；归属 `svd`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `matrix` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#svd)。

## angle

稳定身份 `fn_000157`；回调 `VectorAngle`；归属 `angle`。兼容拼写：`vectorangle`。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#angle)。

## projection

稳定身份 `fn_000158`；回调 `Projection`；归属 `projection`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `onto` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](linear.md#projection)。

## erf

稳定身份 `fn_000132`；回调 `Erf`；归属 `erf`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#erf)。

## erfc

稳定身份 `fn_000133`；回调 `Erfc`；归属 `erfc`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#erfc)。

## gamma

稳定身份 `fn_000134`；回调 `Gamma`；归属 `gamma`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#gamma)。

## log_gamma

稳定身份 `fn_000135`；回调 `LogGamma`；归属 `log_gamma`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#log_gamma)。

## beta

稳定身份 `fn_000136`；回调 `Beta`；归属 `beta`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `a` | 位置 | expression | 是 | 必填 |  |
| `b` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#beta)。

## acoth

稳定身份 `fn_000129`；回调 `ArcCoth`；归属 `acoth`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#acoth)。

## asech

稳定身份 `fn_000130`；回调 `ArcSech`；归属 `asech`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#asech)。

## acsch

稳定身份 `fn_000131`；回调 `ArcCsch`；归属 `acsch`。兼容拼写：无其他现代拼写。

保持属性：protected, listable, numeric_function；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `x` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](basics.md#acsch)。

## normal_distribution

稳定身份 `fn_000190`；回调 `NormalDistribution`；归属 `normal_distribution`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `mean` | 位置 | expression | 否 | 0 |  |
| `std` | 位置 | expression | 否 | 1 |  |
| `mean` | 命名 | expression | 否 | 0 |  |
| `std` | 命名 | expression | 否 | 1 |  |

数学边界与精度：[所属条目](statistics.md#normal_distribution)。

## uniform_distribution

稳定身份 `fn_000191`；回调 `UniformDistribution`；归属 `uniform_distribution`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `bounds` | 位置 | expression | 否 | [0,1] |  |
| `bounds` | 命名 | expression | 否 | [0,1] |  |

数学边界与精度：[所属条目](statistics.md#uniform_distribution)。

## pdf

稳定身份 `fn_000192`；回调 `PDF`；归属 `pdf`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `distribution` | 位置 | expression | 是 | 必填 |  |
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#pdf)。

## cdf

稳定身份 `fn_000193`；回调 `CDF`；归属 `cdf`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `distribution` | 位置 | expression | 是 | 必填 |  |
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](statistics.md#cdf)。

## random_uniform

稳定身份 `fn_000194`；回调 `RandomUniform`；归属 `random_uniform`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `count` | 命名 | integer | 否 | 1 | ; ≥0; ≤100000 |
| `bounds` | 命名 | expression | 否 | [0,1] |  |
| `seed` | 命名 | integer | 否 | 独立会话流；省略不重置种子 | ; ≥0 |

数学边界与精度：[所属条目](statistics.md#random_uniform)。

## random_normal

稳定身份 `fn_000195`；回调 `RandomNormal`；归属 `random_normal`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `count` | 命名 | integer | 否 | 1 | ; ≥0; ≤100000 |
| `mean` | 命名 | expression | 否 | 0 |  |
| `std` | 命名 | expression | 否 | 1 |  |
| `seed` | 命名 | integer | 否 | 独立会话流；省略不重置种子 | ; ≥0 |

数学边界与精度：[所属条目](statistics.md#random_normal)。

## random_choice

稳定身份 `fn_000196`；回调 `RandomChoice`；归属 `random_choice`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `data` | 位置 | expression | 是 | 必填 |  |
| `count` | 命名 | integer | 否 | 1 | ; ≥0; ≤100000 |
| `seed` | 命名 | integer | 否 | 独立会话流；省略不重置种子 | ; ≥0 |

数学边界与精度：[所属条目](statistics.md#random_choice)。

## seed_random

稳定身份 `fn_000197`；回调 `SeedRandom`；归属 `seed_random`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`write_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `seed` | 位置 | integer | 是 | 必填 | ; ≥0 |

数学边界与精度：[所属条目](statistics.md#seed_random)。

## parse_json

稳定身份 `fn_000212`；回调 `ParseJSON`；归属 `parse_json`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `text` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#parse_json)。

## parse_csv

稳定身份 `fn_000211`；回调 `ParseCSV`；归属 `parse_csv`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `text` | 位置 | expression | 是 | 必填 |  |
| `header` | 命名 | boolean | 否 | true |  |

数学边界与精度：[所属条目](data.md#parse_csv)。

## to_json

稳定身份 `fn_000214`；回调 `ToJSON`；归属 `to_json`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#to_json)。

## to_csv

稳定身份 `fn_000213`；回调 `ToCSV`；归属 `to_csv`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `table` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](data.md#to_csv)。

## quantity

稳定身份 `fn_000215`；回调 `Quantity`；归属 `quantity`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `value` | 位置 | expression | 是 | 必填 |  |
| `unit` | 位置 | expression | 否 | 单位必填；此位置与命名unit二选一 |  |
| `unit` | 命名 | expression | 否 | 单位必填：与第二个位置参数二选一，不提供省略默认值 |  |

数学边界与精度：[所属条目](units.md#quantity)。

## convert_units

稳定身份 `fn_000216`；回调 `UnitConvert`；归属 `convert_units`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `quantity` | 位置 | expression | 是 | 必填 |  |
| `target_unit` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](units.md#convert_units)。

## magnitude

稳定身份 `fn_000217`；回调 `QuantityMagnitude`；归属 `magnitude`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `quantity` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](units.md#magnitude)。

## unit

稳定身份 `fn_000218`；回调 `QuantityUnit`；归属 `unit`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：1；入口副作用：`pure`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `quantity` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](units.md#unit)。

## help

稳定身份 `fn_000245`；回调 `Help`；归属 `help`。兼容拼写：无其他现代拼写。

保持属性：protected, hold_first；管道输入位置：0；入口副作用：`read_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `name` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](runtime.md#help)。

## options

稳定身份 `fn_000247`；回调 `Options`；归属 `options`。兼容拼写：无其他现代拼写。

保持属性：protected, hold_first；管道输入位置：0；入口副作用：`read_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `name` | 位置 | expression | 是 | 必填 |  |

数学边界与精度：[所属条目](runtime.md#options)。

## functions

稳定身份 `fn_000246`；回调 `Functions`；归属 `functions`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`read_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|
| `category` | 命名 | expression | 否 | 所有分类 |  |
| `stage` | 命名 | enum | 否 | "current" | current, planned, deferred |

数学边界与精度：[所属条目](runtime.md#functions)。

## capabilities

稳定身份 `fn_000248`；回调 `Capabilities`；归属 `capabilities`。兼容拼写：无其他现代拼写。

保持属性：protected；管道输入位置：0；入口副作用：`read_session`。

| 参数 | 角色 | 类型 | 必填 | 默认值或上下文 | 枚举 / 范围 |
|---|---|---|---|---|---|

数学边界与精度：[所属条目](runtime.md#capabilities)。


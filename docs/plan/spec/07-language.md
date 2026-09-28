<!-- Extracted from docs/plan/PLAN.md sections [7]. PLAN.md is authoritative; keep in sync. -->

## 7. 输入语言规格

### 7.1 共同的内部语义
两种方言产生**完全相同**的 `Expr`。例如现代方言 `solve(x^2 + 2x = 3, x)` 和 Wolfram 方言 `Solve[x^2 + 2 x == 3, x]` 都得到 `Solve[Equal[Plus[Power[x,2], Times[2,x]], 3], x]`（解析器输出的是**未规范化**树；求值器第一步会 `canonicalize`）。

### 7.2 现代方言（默认）

| 构造 | 语法 | 内部表示 |
|---|---|---|
| 数字 | `42`、`3.14`、`1e-3`、`1.5e10`、`0x1F`、`1_000_000` | Integer；十进制小数 → Real::Machine；有效数字 > 16 位 → Real::Big(精度=ceil(digits·log2(10)))|
| 精确分数 | `1/3`（求值时变成 Rational） | Times[1, Power[3,-1]] → 规范化后是 Rational |
| 标识符 | 字母/`_`/Unicode 字母开头，后接字母数字；`xy` 是**一个**符号 | Symbol |
| 希腊字母 | `α β θ π` 等直接可用；`pi`、`π` 均为 Pi | |
| 常量 | `pi` `π` → Pi；`e` → E；`i` → I；`inf` `∞` `infinity` → Infinity | 笔记本设置 `constants = "strict"` 时关闭 `e`、`i` 的映射 |
| 函数调用 | `f(x, y)`：标识符与 `(` **之间无空格** | 函数名按 7.4 映射表转内置名，否则保留原名 |
| 隐式乘法 | `2x`、`2 x`、`x y`、`2(x+1)`、`(x+1)(x-1)`、`x (y+1)`（有空格）、`2sin(x)`、`3π` | Times |
| 幂 | `x^2`、`x**2`、`x²`（Unicode 上标数字 ⁰¹²³⁴⁵⁶⁷⁸⁹ 与 ⁻）、右结合 | Power |
| 根号 | `sqrt(x)`、`√x`、`√(x+1)`、`cbrt(x)`（→ Power[x,1/3]，实数分支见 7.5）、`root(x, n)` | |
| 方程 | `a = b` 与 `a == b` **都**是 Equal（`=` 只在 `let` 语句中是赋值） | Equal |
| 不等 | `!=` `≠` `<` `<=` `≤` `>` `>=` `≥`；链式 `0 < x <= 1` | Inequality 规范化为 And[Less[0,x], LessEqual[x,1]] |
| 逻辑 | `and` `&&` `∧`；`or` `\|\|` `∨`；`not` `!` `¬` | And/Or/Not |
| 列表 | `[1, 2, 3]` 或 `{1, 2, 3}` | List |
| 下标取值 | `v[1]`（标识符后紧跟 `[` 且无空格） | Part[v, 1]（现代方言 1 起始） |
| 规则 | `x -> 1`、`x → 1` | Rule |
| 替换 | `expr where x = 2, y = 3` 或 `expr /. x -> 2` | ReplaceAll |
| 赋值 | `let a = 5`；`let f(x) = x^2`（→ SetDelayed[f[x_], x^2]） | Set / SetDelayed |
| 关键字参数 | `solve(x^2 < 4, x, domain: reals)`、`nsolve(..., precision: 30)` | 见 7.4 “关键字参数映射” |
| 注释 | `# 到行尾` | 丢弃 |
| 语句 | 换行或 `;` 分隔；以 `;` 结尾的语句不显示输出 | |
| 上次输出 | `%`、`%3`、`out(3)` | Out[] / Out[3] |
| 阶乘 | `5!`（后缀）；前缀 `!` 仍是 Not | Factorial |
| 绝对值 | `abs(x)` 或 `|x|`（`|` 成对出现且不在 `||` 中） | Abs |

**优先级（从低到高）：** `;` < `where` < `->` < `or` < `and` < `not` < 比较（`= == != < <= > >=`，可链式）< `+ -` < `* /` 与隐式乘法 < 一元负号 < `^`（右结合；`-x^2` = `-(x^2)`；`2^-1` 合法）< 后缀 `!` < 调用/下标 `f(x)` `v[1]` < 原子。

**关键歧义的裁决（写进测试）：**
- `2x^2` = `2*(x^2)`；`2^x y` = `(2^x)*y`；`x^2y` = `(x^2)*y`（隐式乘法优先级低于 `^`）。
- `f (x)`（有空格）= `f*x`，同时给出 Hint 诊断 `W001`：“`f (x)` 被当作乘法；如果想调用函数请去掉空格”。
- `sin x` → 诊断 Error `E010`：“函数需要括号：sin(x)”，附带 Fix。
- `a(b+c)`：`a` 未知且紧跟 `(` → **函数调用** `a[b+c]`；诊断 Hint `W002`：“`a` 不是已知函数；如果想表示乘法，写成 `a*(b+c)`”，附带 Fix。已知函数 = 内置函数表 ∪ 本会话用 `let` 定义过的函数（kernel 在解析时传入 `known_functions` 集合，见 `parse_with(src, dialect, &ParseEnv)`）。
- `x = 3` 单独成句：解析为 Equal；kernel 在 UI 上提供动作按钮 “求解 x” 与 “赋值 let x = 3”（见 12.3）。
- `|x| + |y|`：用栈式配对；`|` 后面是“表达式起始”时视为开，否则视为闭。

### 7.3 Wolfram 方言
支持 Mathematica 语法的子集，并**保证第 14 节语料中所有 Wolfram 语法都能解析**：
- 调用 `f[x]`，Part `v[[1]]`，列表 `{}`，`==` `!=` `===`（SameQ）`<` `<=` `>` `>=`，`&&` `||` `!`，`->` `:>` `/.` `//.`，`=` `:=`（Set/SetDelayed）`=.`（Unset），`;`（CompoundExpression），`/;`（Condition），`_` `x_` `x_Integer` `x__` `x___`，`#` `#1` `&`（纯函数），`@`（前缀调用）、`//`（后缀调用）、`f@@x`（Apply）、`f/@x`（Map），`'`（导数 `f'[x]` → `Derivative[1][f][x]`），`(* 注释 *)`（可嵌套），字符串 `"..."`，数字 `1.5`、`1.5*^-3`、`1.5`30`（精度标记）、`2^^1011`（基数，可选实现），`\[Pi]` 等具名字符（至少支持 Pi, Alpha…Omega 希腊字母, Infinity, Element, Equal, LessEqual, GreaterEqual, NotEqual, Rule）。
- 隐式乘法：**只有**空白或并置（`2 x`、`2x`、`x y`、`(a)(b)`），与 Mathematica 一致。
- 官方优先级按 Wolfram 文档 “Operator Input Forms” 表格实现；v1 只需要上面列出的运算符。
- **方言自动检测 `detect_dialect`：** 按顺序：(1) 首行是 `%wl` 或 `%modern` 注释标记 → 相应方言（该行从源码中去掉）；(2) 正则 `[A-Za-z][A-Za-z0-9]*\[` 且方括号**不是**紧跟已知小写函数名的下标用法，或出现 `==` `->` `:=` `/.` `(*` 中任意两种 → Wolfram；(3) 否则 → Modern。检测结果在 UI 的 cell 角标上显示，用户可以点击切换并固定。

### 7.4 名称映射表（现代 → 内置）
大小写不敏感，另外所有 Wolfram 原名（如 `Solve`、`Sin`）在现代方言中也可以直接用。

```
solve→Solve  nsolve→NSolve  findroot/find_root→FindRoot  reduce→Reduce  eliminate→Eliminate  solvevalues→SolveValues
sin cos tan cot sec csc → Sin…; asin/arcsin→ArcSin, acos/arccos→ArcCos, atan/arctan→ArcTan (2 参数 atan2(y,x)→ArcTan[x,y]，注意参数顺序)
sinh cosh tanh asinh acosh atanh → 对应; exp→Exp; ln→Log; log(x)→Log[x]; log(b, x)→Log[b,x]; log10→Log[10,·]; log2→Log[2,·]
sqrt→Sqrt  cbrt→CubeRoot(实分支)  abs→Abs  sign/sgn→Sign  re→Re  im→Im  conj→Conjugate  arg→Arg
floor ceil/ceiling round mod gcd lcm → 对应; factorial→Factorial; binom/binomial/choose→Binomial
expand factor simplify together cancel apart collect → 对应; diff/d/derivative→D; n/numeric→N
plot→Plot  implicitplot→ContourPlot(方程)  table→Table  range→Range  length/len→Length  map→Map
lambertw/productlog→ProductLog  root→(2 参数) Power[x,1/n]
```

**关键字参数映射：** `domain: reals|integers|complexes|rationals|positives` → 作为 Solve/Reduce 的第三个位置参数（`positives` → 追加条件 `v > 0` 并使用 Reals）；`precision: n` → `WorkingPrecision -> n`；`steps: false` → 关闭步骤记录；`method: "..."` → `Method -> "..."`；其余 `key: v` → `Rule[Key 的 Wolfram 驼峰名, v]`（例如 `max_iterations: 100` → `MaxIterations -> 100`）。

### 7.5 两种方言都成立的约定
- `cbrt(x)`/`CubeRoot[x]` 取**实数立方根**（`CubeRoot[-8] = -2`），而 `(-8)^(1/3)` 取主值（复数），与 Mathematica 相同。
- 求解变量未给出时（`solve(x^2 = 1)`）：使用方程中全部自由符号，按字母序排列；如果超过方程个数，发出 `Solve::svars` 警告并对前 n 个变量求解（这与 Mathematica 不同；Mathematica 会报错或给出参数解。差异写进 `docs/solve.md`）。

# 输入语言与交互

[English](language.en.md) · [文档导航](README.md)

现代语法与 Wolfram 语法解析到同一棵表达式树。现代函数用圆括号、列表用方括号，`let x=value` 赋值，`x=value` 表示等式；Wolfram 函数用方括号、列表用花括号，`x=value` 赋值，`x==value` 表示等式。完整规格见[计划 §7](plan/PLAN.md#7-输入语言规格现代方言--wolfram-方言)。

本页描述当前 `.2`。新管道、`fn`、区间、记录、矩阵 `@` 及统一 `mode/output` 尚未实现，设计见[现代语言](design/modern-language.md)。[全景目录](reference/README.md)分别列当前可用签名与目标接口，不能把规范名称或已被解析的名字当作已实现函数。

## 常用表达式

| 操作 | 现代语法 | Wolfram | 结果 |
|---|---|---|---|
| 展开 | `Expand((x+1)^3)` | `Expand[(x+1)^3]` | `x^3+3x^2+3x+1` |
| 因式分解 | `Factor(x^2-y^2)` | `Factor[x^2-y^2]` | `(x-y)(x+y)` |
| 约分 | `Cancel((x^2-1)/(x-1))` | `Cancel[(x^2-1)/(x-1)]` | `x+1` |
| 系数 | `CoefficientList(a*x^2+b*x+c,x)` | `CoefficientList[a x^2+b x+c,x]` | `[c,b,a]` / `{c,b,a}` |
| 求导 | `D(sin(x^2),x)` | `D[Sin[x^2],x]` | `2x Cos[x^2]` |
| 部分分式 | `Apart(1/(x*(x+1)),x)` | `Apart[1/(x(x+1)),x]` | `1/x-1/(x+1)` |
| 求解 | `solve(x^2=2,x)` | `Solve[x^2==2,x]` | 两个精确根 |

还支持 `Together`、`Simplify`、`FullSimplify`、`Collect`、`Coefficient`、`Exponent`、`PolynomialQ`、`PolynomialGCD`、`PolynomialLCM`、`PolynomialQuotient`、`PolynomialRemainder`、`Resultant`、`Discriminant`、`Variables`、`RootReduce` 和 `ToRadicals`。内置帮助提供中英文参数说明。

Expand/Factor/Together/Cancel/RootReduce/ToRadicals 可逐项作用于列表。CoefficientList 接受变量列表，以升幂返回矩形系数张量；Collect 的可选第三参数处理每个系数。

多项式查询允许与指定变量无关的符号系数和分母。次数上限 4096，系数张量最多 100 万项、16 个轴。精确 GCD/LCM 使用有理数多项式并保留数值内容；商、余式和结式支持有理参数系数。Apart 仅支持单变量有理数系数有理函数；只有一个自由符号时可推断变量。不支持的输入保持符号形式并给出诊断。

D 支持重复阶数 `D[x^4,{x,2}]`、混合变量 `D[x^2 y^3,x,y]`、列表和初等函数链式法则。未知函数保留形式导数 `Derivative[...]`，最多 64 个规格、4096 阶。`Simplify[Sqrt[x^2],x>0]` 使用正值假设；没有假设时保留主值分支。FullSimplify 比较已认证的 Root/根式形式并选择复杂度较低的表达式；显式要求根式用 ToRadicals。

RootReduce 为支持的精确代数值生成认证最小多项式及从 1 开始的根编号。ToRadicals 遍历表达式头和参数，使用已启用的三/四次公式及特殊降阶，按认证根编号选择分支；不支持的参数根保留 Root。

## 求解函数和选项

支持 Solve、NSolve、FindRoot、Reduce、Eliminate、SolveValues、NSolveValues、Roots、Root、ConditionalExpression。默认定义域 Complexes，可显式选 Reals、Integers、Rationals。省略变量按名称推断；方程少于变量时警告并选择前几个轴。显式空变量列表仍为空。

Solve/Reduce/Roots 和两个 Values 家族接受 Cubics、Quartics、VerifySolutions、MaxExtraConditions、GeneratedParameters（符号头）、InverseFunctions。NSolve/NSolveValues 另接受 WorkingPrecision（5..2466 十进制位或 MachinePrecision）。无效选项保留原调用并诊断。`SolveValues[x^2==2,x]` 返回平坦值列表，`SolveValues[x^2==2,{x}]` 返回单坐标行；保留轴顺序、重数、自由轴和条件。Roots 将完整解转换为带全部条件和生成参数域的等式析取。

FindRoot 接受 `{x,start}`、`{{x,start},{y,start}}`、分别提供的起点规格，或一个实数区间 `{x,a,b}`。选项是 WorkingPrecision、MaxIterations、Method（Automatic/Newton/Brent，符号或字符串）。Automatic 对区间用 Brent，否则用 Newton；显式方法须匹配起点形式。起点变量对会话值局部化，成功返回平坦规则列表，如 `{x->1.4142135623730951}`，舍入后的结果对原始残差和极点再验证。失败保留调用和消息。

求解时保留直接算术源码、解析变量值/延迟定义/纯函数，因此 `Solve[x/x==1,x]` 排除零，`f[t_]:=t/t; Solve[f[x]==1,x]` 也保留孔洞。立即赋值 `eq=x/x==1` 或显式 Cancel/Expand/替换使用已求值含义，已丢失的限制无法恢复。依赖原始算术的字面/类型模式需显式求值后使用。嵌套 Root 参数先解析；Divide/Subtract/Minus 转为原始算术，Floor 等依赖变量的数值函数保留源码树。闭合数值系数仅在确认原定义域后求值。

Root 接受单参数多项式纯函数和有效正索引。有理数系数闭合根使用不超过 64 次的认证，有理根化为精确数；符号参数根保持一般次数/索引。非多项式或索引无效保留调用和诊断。ConditionalExpression 先求条件：True 去除包装，False 返回 Undefined，其他保留。关闭 record_steps 不创建步骤；Eliminate 的现有 API 只返回消元关系。

## 笔记本与终端

数学单元格执行源码，文本单元格渲染 Markdown，“问 AI”单元格提供待确认的建议。

| 快捷键 | 操作 |
|---|---|
| Shift-Enter / Ctrl/Cmd-Enter | 运行当前数学单元格 |
| Ctrl/Cmd-K | 命令面板 |
| Ctrl/Cmd-S | 保存 |
| Ctrl/Cmd-句点 | 中断 |
| Tab | 接受可见补全，插入源码但不运行 |

本地补全和希腊字母快捷输入优先于 AI。分别运行 `let a=2` 和 `a+1`，修改并运行 `let a=5` 后依赖结果变为 6。终端按顺序执行并更新状态。

`.omnb` 仅保存版本、标题和单元格源码/类型/方言。Markdown/LaTeX 导出包含当前结果，排除过期输出；Unicode 源码可能需要支持 Unicode 的 TeX 环境。

终端 `--dialect modern|wolfram|auto` 选择方言，`--json` 输出真实 CellOutput。`-e` 成功退出 0，求值错误 1，解析/用法错误 2。`--no-config` 隔离运行，`--config PATH` 指定配置；桌面支持 `OPENMATH_CONFIG_PATH`。

## .3 开发版的组合语法（R3.2）

公开 `.2` 不支持本节新增语法。当前 dev 实现可使用：

```text
[1, 2, 3] |> map(fn(x) => x^2)  # [1,4,9]
(fn(x) => fn(y) => x+y)(2)(3)   # 5，内层参数不会捕获外层值
solve(x^2=4, x, output: "values")
solve(x^2=4, x, mode: "numeric", precision: 20)
simplify(sin(x)^2+cos(x)^2, level: "deep")
diff(x^4, x, order: 2)
find_root(x^2=2, x, bracket: 1..2)
let config = {color: "green", count: 2}
config.count
let v = [1,2,3,4]
v[2..3]
v[4..2]
[[1,2],[3,4]] @ [5,6]
plot(sin(x), x: -pi..pi)
plot(x^2+y^2=1, x: -2..2, y: -2..2, view: "contour")
```

管道根据元数据选择主参数；map的数据在第二位置，不重复执行左侧。fn按后续箭头识别，原fn(x)调用仍保留；内层参数可遮蔽外层，插入值不会被同名参数捕获。范围为有方向闭区间，切片端点必须存在且不能为0；旧单项索引0取头仍保留。记录的键按文字保存，重复键拒绝，空{}仍为列表，空记录用record()。矩阵乘积双线性；两个向量的内积共轭第一向量，尺寸上限64。Wolfram @原义不变。

模式只选择已有真实算法；精确求解不能接受数值精度选项，局部find_root不承诺完整解集。初值与括区间互斥。line/零轮廓contour已支持，精确规则与显式机器积分已接通；surface/density、ODE及explore按后续批次交付，当前不会被当作成功计算。笔记本继续仅保存原源码，使用本节源码时需要.3；不自动改写旧笔记本。

## .3 开发版的概率与特殊函数（R3.3d）

当前 dev 已接入 `acoth`、`asech`、`acsch`、`erf`、`erfc`、`gamma`、`log_gamma`、`beta`，以及正态/均匀分布的 `pdf`、`cdf`、`quantile`。特殊函数只有所列实数机器路径；Gamma 正整数及少量识别值保持精确，其余精确输入保留原式，使用 `numeric(erf(1))` 等显式请求近似。高精度与复杂分支尚未支持。

```text
gamma(5)                                        # 精确 24
erfc(8.0)                                       # 直接计算微小尾部
quantile(normal_distribution(), 0.025)
quantile(uniform_distribution(bounds: [2,6]), 1/4) # 精确 3
random_normal(count: 100, seed: 42)               # 可重复的局部流
random_uniform()                                # 主会话流中的一个值
seed_random(42)                                 # 重置主会话流
```

随机 count 省略时为标量，显式提供时为列表（0..100000 项）。显式 seed 不改变主会话流；失败或取消不推进主流，只读计算使用独立快照。数学范围、精度和边界详见[特殊函数与概率实现](design/special-probability.md)，这些接口尚未随公开 `.2` 发布。

## .3 开发版的根与矩阵对角

`cbrt` 是实数立方根，`cbrt(-8)` 为 -2；完美立方保持精确，其他精确输入保留实根式，机器与已有高精度数使用实际数值路径。`nth_root(value,degree,branch:"principal")` 限 1..4096 正整数阶及主值分支，负数奇次根也可能是复数；例如 `nth_root(-8,3)` 与 -2 不同。认证代数 `Root` / `algebraic_root` 与这两者分开，旧小写 `root(value,degree)` 的主值 Power 语义不改。

`diag([2,3])` 构造对角矩阵；`diag([[1,2,3],[4,5,6]])` 提取最短轴的对角，返回 [1,5]。矩阵需矩形且不超过 64×64，元素精度和表达式保留，不隐式构造块对角或多层张量。

## .3 开发版的数值积分

显式 `integrate(expr,x:a..b,mode:"numeric")` 或 `n_integrate(expr,[x,a,b])` 使用自适应GK15/7机器路径；结果含 value/error_estimate/converged/evaluations/intervals 和 certified=false，可用 `.value` 参与运算。支持有限、反向、无限范围和 breakpoints；高精度numeric/不支持的exact不静默降级，原极点、失败和取消保留真实诊断。详细边界见[微积分实现](design/calculus.md)。公开 `.2` 不包含这些新增入口。

## .3 开发版的精确规则积分

`integrate(expr,x)` 默认exact：多项式、常用仿射初等形式、识别链式代换、有限分部积分、一次/二次因子的精确有理式及Gaussian→Erf。所有候选经过真实求导/精确残差验证；充分成立条件保留在ConditionalExpression，无法证明区间奇点/参数可积或分支时保留输入。有限定积分支持已验证区间，`integrate(x^2,x:0..1)` 为精确1/3，整个实轴Gaussian可得sqrt(pi)。精确请求不会静默改成近似，详细范围见[微积分实现](design/calculus.md)。

## .3 开发版的极限与 Taylor

`limit(expr,x,at:point,direction:"both")` 计算可证明的实单侧/双侧/无限极限；左右不一致、振荡和未知方法保留原式。`series(expr,x,at:0,order:6)` 生成真实导数的普通Taylor（0..64阶），携带首个未知阶，原式孔洞/分支点和形式导数拒绝。`series_coefficient(s,n)` 只读已知系数，截断外不伪造0；`normal(s)` 明确舍弃O项得到多项式。原坐标不代入全局赋值，其他参数/边界仍响应真实依赖；标准Wolfram位置语法兼容。完整边界与例见[微积分实现](design/calculus.md)。公开`.2`不含新增入口。

## .3 开发版的 ODE、插值与采样

公开 `.2` 不含本节入口；完整约定见 [ODE 与插值设计](design/ode-interpolation.md)。

```text
let motion = ode(fn(t,y)=>[y[2],-y[1]], initial:[1,0], t:0..6)
motion.solution(1.25)
sample(motion.solution, x:0..6, count:100)
interpolate([[0,0,0],[1,1,3]], method:"hermite")(0.37)
```

ODE 为非刚性机器实数 DP5(4)，支持连续解与简单零交叉终止事件，记录实际步骤和调用数。插值默认拒绝外推，高精度/写入/非有限/未收敛明确诊断；参数与返回形状可通过 `help("ode")` 和 `options("ode")` 查询。专门表格/数值展示留 R3.5。

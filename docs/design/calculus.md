# 微积分与数值分析实现

本页随 `.3` 开发批次记录真实算法，不是完整阶段已完成的声明。当前已接通笛卡尔微分、精确规则积分和数值积分；极限/级数、ODE、优化与拟合继续按 NEXT_RELEASE 实施。

## 笛卡尔微分

`grad`/`hessian`/`laplacian` 接受标量表达式和 1..64 个互异用户坐标；`jacobian` 的表达式是非空向量，结果按分量为行、坐标为列；`div` 要求分量与坐标数相等，`curl` 首版仅三维。当前不支持其他坐标系、张量或自动单位微分。

```text
grad(x^2*y, [x,y])                  # [2*x*y,x^2]
jacobian([x*y,sin(x)], [x,y])        # [[y,x],[cos(x),0]]
hessian(x^2*y+y^3, [x,y])
div([x^2,y^2,z^2], [x,y,z])
curl([-y,x,0], [x,y,z])              # [0,0,2]
laplacian(x^2+y^2+z^2, [x,y,z])     # 6
```

变量列表作为局部坐标，源码在只读 fork 中展开；已有 `let x=7` 不会把 `grad(x^2,[x])` 误变为常数求导，返回的坐标也不被调用方再次替换。全局定义保持，名称表达式中的间接写入拒绝。结果若随后被外层表达式显式使用，则遵循外层现有求值规则。

计算复用实际堆式求导器，未知函数保留形式导数，不把未知视为零；旧 D/diff 行为与数学期望不改。新增 Erf/Erfc、实主值反双曲及实立方根链式规则，定义域/可微点约束沿用对应函数，不宣称复杂分支或 Gamma 的通用闭式导数。

解析期望、`curl(grad)=0`、`div(curl)=0`、全局变量保持、独立数值差分及形状/预算失败验证见 [vector_calculus.rs](../../crates/om-eval/tests/vector_calculus.rs)。所有遍历与求导使用现有 Interrupt，不加载外部数学运行时或 Agent 框架。

## 数值积分

```text
integrate(x^2, x: 0..1, mode: "numeric").value
integrate(exp(-x^2), x: -inf..inf, mode: "numeric")
integrate(1/sqrt(x), x: 0..1, mode: "numeric")
integrate((x^2-1)/(x-1), x: 0..2, mode: "numeric", breakpoints: [1])
n_integrate(sin(x), [x,0,pi])
```

`Integrate` 默认 exact 使用下述精确规则，显式 numeric 或 `NIntegrate` 才使用数值路径；不能处理的精确请求保留原式，不自动近似。数值模式是机器实数的一维自适应 Gauss–Kronrod 15/7，默认 abs_tol=1e-10/rel_tol=1e-8、max_intervals=10000（限1..100000）、method=gauss_kronrod、precision=machine。高精度数值积分请求明确拒绝，不能靠补足显示位数宣称精度。

返回 Record：value 是实际近似值，error_estimate 是绝对误差估计，converged 是成功收敛状态，evaluations/intervals 是实际工作统计，precision/method 记录真实路径，certified=false。参与后续数值运算时显式使用 `.value`。估计误差不是严格区间或证明，宿主专门的数值诊断展示在 R3.5 接续。

内核按最大误差优先拆分，保留 resabs/resasc 和舍入底限。有限区间可反向；单侧无限端点映射到(0,1)，双侧无限区间分别计算两侧，不能把发散部分相消成伪造主值。数学方法参考 [GSL 积分说明](https://www.gnu.org/software/gsl/doc/html/integration.html)，代码为本项目编写，不复制或引入 GSL/QUADPACK 运行时。

原始源码在 readonly fork 中展开，坐标局部化；全局 x 的赋值不替换被积函数中的 x，边界/选项在只读调用环境计算。真实编译数值程序采样原式，不约消源极点。单个未定义点可能需要显式 breakpoints 分段；分段点必须有限且严格位于内部，最多4096个。数值采样无法认证所有函数的行为，超出实际容差/范围时不得把估计命名为保证。

非有限样本、输入/方法/精度错误、限额耗尽、误差停滞/机器分辨率及真实取消分别失败；已算出的部分估计、误差和工作量仅作未收敛诊断，保留调用源码。零长度有限范围返回0而不采样，相同无限端点拒绝。每轮规则、堆统计、回调与编译均使用真实 Interrupt。

独立解析参考、无限尾部/反向/端点奇点/断点、发散拒绝、极端尺度、原始孔洞、只读与预算验证见 [纯算法](../../crates/om-analysis/tests/integration.rs)、[实际API](../../crates/om-eval/tests/numeric_integration.rs)。原53数学期望不改；R3.4 其他模块继续实施。

## 精确规则积分

```text
integrate(3*x^4+2*x-7, x)
integrate(sin(3*x+2), x)
integrate(2*x*cos(x^2), x)
integrate(x^2*exp(2*x), x)
integrate(x*log(x), x)
integrate(1/(x^2+1)^2, x)
integrate(exp(-2*x^2+4*x), x)
integrate(exp(-x^2), x: -inf..inf)       # sqrt(pi)
integrate(x^2, x: 0..1)                 # 精确 1/3
```

公式依据包括 [NIST 微积分](https://dlmf.nist.gov/1.4) 与 [NIST Erf 定义](https://dlmf.nist.gov/7.2)；本项目自行实现规则与递推，不复制外部运行时代码。

真实规则覆盖最多256次多项式、仿射代换的常见初等/反三角/反双曲及实立方根形式、识别出的导数因子链式代换、最多16次多项式的有限分部积分；有理式复用 Q 系数精确部分分式，处理一次/二次因子及重复二次递推（重复≤32）。Gaussian 二次完成平方转 Erf；Gaussian完整或半无限区间定积分要求明确正实衰减系数。没有通用 Risch 或任意复路径积分。

exact原函数验证首版要求精确系数，不能用浮点舍入成零伪造证书。含近似系数的被积函数须显式有理化，或对定积分选择 numeric；不把机器小数默默填充成假精度。端点值仍保留其已有数值精度。

每个候选原函数实际经过求导，并以等价三角比值变换及精确化简验证残差为零；失败或预算耗尽保留输入，附近采样不成为符号证书。符号系数的非零假设、原源码孔洞、Log/分数幂的充分主值分支条件进入 ConditionalExpression。分支条件可能比最大数学定义域更严格，表示已交付的局部原函数范围；不宣称通过省略条件涵盖全部复平面。输出不包括任意积分常数。

有限定积分使用已证明的原函数，先验证局部条件在整个实区间成立，再消去哑变量条件，额外验证实际极点不跨区间；当前检查可识别的低次有理极点与有理 Pi 三角边界。可去孔洞可按不当积分计算，真正极点拒绝；无法证明符号幂在端点可积、参数区域或复杂分支时保持原式，不用端点相减伪造成功。尚未支持一般代数奇点端点和任意参数收敛条件。数值选项不适用于 exact 分支。

真求导、独立多项式系数向量、重复因子、分部积分、Gaussian、区间极点/分支、参数拒绝、只读与预算验收见 [symbolic_integration.rs](../../crates/om-eval/tests/symbolic_integration.rs)。独立算法的有理积分递推与基本链式/分部积分规则由本项目编写，无新依赖、外部运行时或 unsafe。

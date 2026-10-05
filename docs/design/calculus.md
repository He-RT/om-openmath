# 微积分与数值分析实现

本页随 `.3` 开发批次记录真实算法，不是完整阶段已完成的声明。当前首先接通六个笛卡尔微分工具；积分、极限/级数、ODE、优化与拟合继续按 NEXT_RELEASE 实施。

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

当前 `Integrate` 的数学路径是显式 numeric，`NIntegrate` 默认为 numeric；符号/精确分支尚未交付，默认 exact 继续保留原式并说明，不自动变为近似。数值模式是机器实数的一维自适应 Gauss–Kronrod 15/7，默认 abs_tol=1e-10/rel_tol=1e-8、max_intervals=10000（限1..100000）、method=gauss_kronrod、precision=machine。高精度请求明确拒绝，不能靠补足显示位数宣称精度。

返回 Record：value 是实际近似值，error_estimate 是绝对误差估计，converged 是成功收敛状态，evaluations/intervals 是实际工作统计，precision/method 记录真实路径，certified=false。参与后续数值运算时显式使用 `.value`。估计误差不是严格区间或证明，宿主专门的数值诊断展示在 R3.5 接续。

内核按最大误差优先拆分，保留 resabs/resasc 和舍入底限。有限区间可反向；单侧无限端点映射到(0,1)，双侧无限区间分别计算两侧，不能把发散部分相消成伪造主值。数学方法参考 [GSL 积分说明](https://www.gnu.org/software/gsl/doc/html/integration.html)，代码为本项目编写，不复制或引入 GSL/QUADPACK 运行时。

原始源码在 readonly fork 中展开，坐标局部化；全局 x 的赋值不替换被积函数中的 x，边界/选项在只读调用环境计算。真实编译数值程序采样原式，不约消源极点。单个未定义点可能需要显式 breakpoints 分段；分段点必须有限且严格位于内部，最多4096个。数值采样无法认证所有函数的行为，超出实际容差/范围时不得把估计命名为保证。

非有限样本、输入/方法/精度错误、限额耗尽、误差停滞/机器分辨率及真实取消分别失败；已算出的部分估计、误差和工作量仅作未收敛诊断，保留调用源码。零长度有限范围返回0而不采样，相同无限端点拒绝。每轮规则、堆统计、回调与编译均使用真实 Interrupt。

独立解析参考、无限尾部/反向/端点奇点/断点、发散拒绝、极端尺度、原始孔洞、只读与预算验证见 [纯算法](../../crates/om-analysis/tests/integration.rs)、[实际API](../../crates/om-eval/tests/numeric_integration.rs)。原53数学期望不改；完整符号积分及 R3.4 其他模块仍须继续实施。

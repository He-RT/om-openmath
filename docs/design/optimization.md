# 局部优化与全局保证边界

[下一版账本](../plan/NEXT_RELEASE.md) · [科研目录](../reference/analysis.md)

当前 `dev` 已接入真实 `optimize`，复用纯数值层并单独实现精确有理凸二次认证。以下示例需要 `.3` 开发源码；公开运行时版本仍为 `.2`，完整 `.3` 发行门禁尚未完成。

## 已实现的纯数值层

`bounded` 接受真实一元目标回调、有限严格递增边界及 Interrupt。使用归一坐标上的黄金分割和受保护的逆抛物线步，并实际计算两个端点；所有返回点和值来自真实回调。按绝对/相对坐标容差缩小区间，保存实际剩余区间宽度和调用次数；多峰函数不保证全局最优，区间宽度不是目标值误差证书。超过迭代限额、非有限值、坐标分辨率不足或取消明确失败并保留实际工作。

`minimize` 接受真实值/梯度回调、1..64 维初值及可选有限盒约束，使用逆 Hessian BFGS 更新与 Armijo 回溯。初值必须在盒内；允许固定维度。投影梯度区分下界的向内/向外方向、上界方向和固定维度，使用真正的无穷范数作为停止条件，不因为步长变小或目标值巨大而宣称收敛。曲率不足时重置更新矩阵，非有限试探可减小步长，实际回调次数包含拒绝试探。梯度停止只代表数值驻点/盒约束驻点候选，不能排除鞍点，也不是已认证局部或全局最优。线搜索失败、仍有大梯度的机器步长停滞、未收敛与中断不返回伪造成功。

| 数值控制 | 默认 | 真实含义 |
|---|---|---|
| abs_tol | 1e-10 | Brent 的正的绝对坐标容差 |
| rel_tol | 1e-8 | Brent 的非负相对坐标容差 |
| gradient_tol | 1e-8 | BFGS 的正的投影梯度无穷范数阈值，与坐标容差分开 |
| max_iterations | 1000 | 1..100000；BFGS 每次另限最多64回溯试探 |

`Answer` 保存 point/value/iterations/evaluations/gradient_norm/bracket_width/method；BFGS 没有虚构区间误差，Brent 没有虚构梯度。`Failure` 保存 Error 与实际部分结果。纯数值层不解析源码、不访问宿主 IO、不启动线程，禁止 unsafe，WASM 可构建，没有引入新依赖。

## 语言接口与真实保证

```text
optimize((x-2)^2, x, initial:0)
optimize((x-4)^2, x, bounds:-2..1)
optimize((x-2)^2+(y+3)^2, [x,y], initial:[0,0], bounds:{x:-1..1,y:-2..2})
optimize(3*x^2+2*x*y+2*y^2-4*x+6*y+9, [x,y], scope:"global")
optimize((x+y-1)^2, [x,y], scope:"global")
```

目标在只读局部坐标中准备，保留原始分母/孔洞；局部梯度来自实际求导器再编译，未支持的导数给诊断。`goal:"max"` 翻转目标/梯度后恢复真实值。坐标1..64个互异用户符号，初值与边界先读取会话快照，目标中坐标不捕获全局绑定。函数不会改变量或随机主会话。

`method:"auto"` 默认：一维有有限 bounds 且无 initial 时使用 Brent，其余局部使用 BFGS。Brent 需要严格递增范围，拒绝 initial 和 gradient_tol；BFGS 必须提供等维 initial，拒绝 abs_tol/rel_tol，使用 gradient_tol。bounds 可为一维 `a..b`、等长范围列表或恰好匹配坐标的记录，BFGS 初值必须在盒内，允许固定维度。支持显式 brent/bfgs，模式不匹配或未知参数不忽略。

结果包含 `variables`（有序文字坐标名）、`point`（等维列表）、`bindings`（文字键记录）、`value`、`goal`、`scope`、`converged`、`guarantee`、`method`、`iterations`、`evaluations`、`projected_gradient_norm`、`bracket_width`、`null_space`、`certificate`。局部 guarantee 为 numerical_bounded_candidate 或 numerical_stationary_candidate；converged 只说明数值停止条件满足，不排除鞍点或其他更优盆地，certificate 为 Null。失败保留调用和真实最后候选/工作，不返回伪造最优点。局部仅机器路径，高精度输入不降级。

## 精确凸二次全局认证

`scope:"global"` 接受原始精确有理总次数不超过2的多项式，最多64变量；`method` 只能 auto/exact_ldlt，precision 只能省略或 "exact"，不接受局部初值和数值容差。近似系数、非有理常量、高次、变量分母、原始孔洞和未知函数拒绝，不靠近似梯度或多起点搜索制造 global 标签。Wolfram Minimize/FindMinimum 等不同约束语法及返回形状没有实现适配，暂未添加误导性别名；本项目 Wolfram 入口为 Optimize。

从原式提取 `c+lᵀx+½xᵀHx`，对 min 使用原目标，max 使用负目标。精确有理 LDLᵀ 只允许非负 D，零主元必须有零余行；返回前独立重构原 Hessian。无约束时解 Hx=−l：正定给唯一点，半正定保留全部 Hessian null_space 自由方向，`optimal_set:"affine"` 表示 `point + null_space` 中方向的任意实线性组合；与驻点条件不相容则目标无界，明确失败。零函数/常函数仍保留全部自由方向，不假定唯一点。

有限有理盒支持最多8个非固定维度，固定坐标可包含在64维内。枚举自由/下界/上界面，解面上的精确驻点并检查可行性、乘子非负、互补与零 KKT 残差；最多3^8面，`max_iterations`（默认1000，上限100000）在此控制实际面数预算，超限不宣称认证成功。盒结果给一个真实认证点，optimal_set 为 one_certified_box_optimum，null_space 为 Null，不假称已经描述全部边界最优集合。

证书包含 objective_sign、constant、linear、hessian、ldlt_l、ldlt_d、gradient、lower_multipliers、upper_multipliers、kkt_residual 和实际 bounds。对变换后的凸目标，`H=L D Lᵀ`、D≥0，以及可行点/非负乘子/互补/零残差保证全局最优；固定坐标允许任意法向梯度，以非负上下界乘子共同表示。原目标值仍正确恢复。所有证书是精确有理数据，分子分母各最多20000位，真实 Interrupt 贯穿提取、分解、枚举和验证；数值精度不填成假证书。global guarantee 为 certified_global，iterations/evaluations 为0（没有数值优化回调），另返回真实 faces_examined。

理论依据是 [Boyd–Vandenberghe《Convex Optimization》§5.5.3](https://web.stanford.edu/~boyd/cvxbook/bv_cvxbook.pdf)：凸问题的可行 KKT 点可作为全局最优证明。这里仅实现有理凸二次与有限盒子集，不宣称通用非凸全局或任意约束能力。

## 已完成验证

[optimization.rs 测试](../../crates/om-analysis/tests/optimization.rs) 用独立二次最优点、Rosenbrock 的 (1,1)、盒边界/固定维度、四次函数、24 组 I+vvᵀ 耦合凸二次解析点检查实际值与残差。覆盖真实调用计数、大常数目标偏移、无效范围/维数/梯度、非有限值、迭代限额与回调中取消。另有真实语言测试 `crates/om-eval/tests/optimization.rs`，独立解析有理最优点、Hessian/LDLT重构、半正定自由方向、盒KKT、无界/原孔洞/非凸/近似伪证拒绝和只读/预算已接入；全回归按进度账本记录。

数学方法说明参考 [SciPy 一元局部最小化](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.minimize_scalar.html) 与 [BFGS 控制项](https://docs.scipy.org/doc/scipy/reference/optimize.minimize-bfgs.html)。本项目独立实现，参考不构成第三方运行时或本项目能力声明。

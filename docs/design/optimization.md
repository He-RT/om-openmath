# 局部优化与全局保证边界

[下一版账本](../plan/NEXT_RELEASE.md) · [科研目录](../reference/analysis.md)

当前 `dev` 已有 `om-analysis::optimization` 的纯数值实现；语言层 `optimize`、参数适配、全局凸二次认证和结构化输出仍在实施，不能把本文示例当作已可运行功能。可执行目录目前保持 `optimize` 为 planned；运行时仍为 `.2`，完整 `.3` 发行门禁尚未完成。

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

## 语言与认证接续（尚未实现）

统一 `optimize(expr, variables, initial: ..., bounds: ..., goal:"min", scope:"local")`，目标在只读局部坐标中准备并复用真正求导/编译器；max 翻转目标及梯度后恢复真实值。Brent 用一维范围，BFGS 用初值及可选有限盒；不得把未知参数、精度或约束忽略掉。真实诊断需明确 numerical_candidate/gradient_stopping 与 certified_global 的区别，不能把所有 success 布尔值映射成已证明最优。

`scope:"global"` 单独证明精确有理凸二次结构、凸性与驻点/约束条件，证明范围外拒绝；不能仅运行 BFGS 或多次起点后添加 global 标签。Minimize/Maximize 的全局兼容名也需要走认证路径；未适配的 Wolfram 多种约束与返回形式不得直接映射成本项目记录。其具体支持子集、结构证书及实际回归需在接口接入时落实。

## 已完成验证

[optimization.rs 测试](../../crates/om-analysis/tests/optimization.rs) 用独立二次最优点、Rosenbrock 的 (1,1)、盒边界/固定维度、四次函数、24 组 I+vvᵀ 耦合凸二次解析点检查实际值与残差。覆盖真实调用计数、大常数目标偏移、无效范围/维数/梯度、非有限值、迭代限额与回调中取消。纯数值33项、定向Clippy/fmt、WASM构建已通过；未据此宣称语言或全局认证完成。

数学方法说明参考 [SciPy 一元局部最小化](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.minimize_scalar.html) 与 [BFGS 控制项](https://docs.scipy.org/doc/scipy/reference/optimize.minimize-bfgs.html)。本项目独立实现，参考不构成第三方运行时或本项目能力声明。

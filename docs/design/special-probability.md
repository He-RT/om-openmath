# 特殊函数与概率随机实现

本页记录 `.3` 开发分支的真实算法与边界；公开应用版本保持 `.2`，完整 `.3` 尚未发行。

`erf`/`erfc` 使用不完全 Gamma 的正项级数与连续分式。小参数直接求 erf，较大参数单独求 erfc，避免 `1-erf` 的尾部抵消；在机器范围外的极小尾部发生实际下溢。`gamma`/`log_gamma` 通过正实递推到 16，再求 Stirling 的 Bernoulli 修正；在 1 和 2 附近单独使用 LogGamma 的局部级数，保留接近零的微小结果。`beta` 使用对数递推和稳定比值，避免直接相减三项巨大 LogGamma。每轮迭代检查既有中断预算，没有平台时钟、网络或新的数学运行时。

公式依据：[NIST Gamma 渐近展开](https://dlmf.nist.gov/5.11)、[NIST Gamma 级数](https://dlmf.nist.gov/5.7)、[NIST 不完全 Gamma 级数](https://dlmf.nist.gov/8.7)、[NIST 不完全 Gamma 连续分式](https://dlmf.nist.gov/8.9)、[NIST 误差函数](https://dlmf.nist.gov/7.2)。实现为本项目编写；不复制外部库代码。

上述数值算法仅机器实数；Gamma/LogGamma/Beta 限正实参数。Gamma 正整数 1..1000001 复用现有受预算限制的精确阶乘。Erf/Erfc 的零值与 asech(1) 保持精确。其他精确非特殊值保留原式，可通过 `numeric` 显式请求机器近似。复杂分支和高精度算法尚未支持，高精度输入或请求明确诊断，不把 f64 结果填充成大浮点。编译数值表达式也调用同一纯算法并传递实际中断；未提供包围区间或严格认证。

`acoth` 的实数范围为 |x|>1；`asech` 为 0<x≤1；`acsch` 为非零有限实数。后两者使用稳定对数形式处理很小的输入，避免倒数溢出。

```text
gamma(5)                         # 精确 24
numeric(erf(1))                   # 机器近似
erfc(8.0)                        # 直接尾部，约 1.1224297173e-29
normal_distribution(mean: 2, std: 3)
cdf(normal_distribution(), 0)     # 机器 0.5
quantile(normal_distribution(), 0.025)
quantile(uniform_distribution(bounds: [2,6]), 1/4) # 精确 3
random_normal(count: 100, seed: 42)
```

正态分布保存真实均值与正标准差；机器 PDF、CDF 和逆 CDF 不声称任意精度。逆 CDF 在下半区直接反解尾部，不相减微小概率，p=0/1 返回数学无穷。精确概率若在舍入后撞到端点则明确拒绝。均匀分布端点必须有限且严格有序，PDF 在端点取区间内密度，CDF 在范围外夹取为 0/1；精确有理输入保留精确计算。样本 `quantile` 的 type7 语义不变。

随机流为 SplitMix64，会话初始 seed=0，种子范围 0..2^64-1；均匀值取 53 位且范围半开，选择索引使用无偏拒绝采样。正态值用 Box–Muller，每项消耗两个随机字。不是密码学随机。

省略 count 返回一个值；显式 count 返回列表，0 为空列表，上限 100000。显式 seed 建立局部流，不修改会话流；`seed_random` 重置会话并返回 Null，只读工具拒绝重置。普通随机请求先复制流，只在整个请求成功后提交；失败、超限或取消不推进主流。只读 fork 保存独立随机快照，无法推进主会话。`.omnb` 仍只保存源码，不序列化随机状态。

独立验收入口：[纯算法参考值](../../crates/om-analysis/tests/special.rs)、[求值器与随机契约](../../crates/om-eval/tests/probability.rs)。参考值、解析密度/分位值、固定种子首字、均值/方差与范围、取消和只读隔离均实际检查；这些测试不是全平台发行已完成的声明。

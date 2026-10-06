# 真实拟合、可调用模型与数值诊断

[下一版账本](../plan/NEXT_RELEASE.md) · [科研函数](../reference/analysis.md)

本页描述当前 `dev` 的 `.3` 实现；公开 `.2` 不包含此接口。`fit` 复用共享纯 Rust 数值层，线性采用列均衡和列主元 Householder QR，非线性采用增广 QR Levenberg–Marquardt（LM）。不引入外部数学运行时、IO、线程或 unsafe。

## 用法与数据边界

```text
let line = fit([[0,1],[1,3],[2,5]], model:a*x+b, parameters:{a:1,b:0})
line.parameters
line.model(3)
sample(line.model, x:0..2, count:5)

let curve = fit(
  [[0,2],[1,numeric(2*exp(0.3))],[2,numeric(2*exp(0.6))],[3,numeric(2*exp(0.9))]],
  model:a*exp(b*x), parameters:{a:1,b:0}, method:"nonlinear"
)
curve.model(0.5)
curve.residual_norm
```

数据为1..10000行有限实数，最后一列是观测值，前面为1..16输入变量。`variables` 默认 x，显式可为一个用户符号或有序列表，如 `[x,t]`；它不创建全局保留字或赋值。参数声明1..16个，保持传入顺序：linear 可用符号列表 `[a,b]` 或有限起点记录；nonlinear 必须用起点记录 `{a:1,b:0}`。线性解不依赖起点，记录仍需通过数值/精度校验。变量/参数互异且不能重名，样本数至少等于参数数，设计矩阵与输入各最多100000标量。

DataTable 按变量名字选择输入列，target 默认 `"y"`，可指定 `target:"score"`；target 不能同时是输入列。普通样本行没有列名，显式 target 拒绝。CSV字段为字符串，拟合不自动把字符串当数学源码；JSON/表格的数据路径继续不执行文本中的代码。值和初值在只读会话快照中读取，随后目标变量与拟合参数局部化，不捕获全局绑定；其他已有函数/常量展开成快照。写入定义、非有限值、高精度输入、维度和列名错误明确诊断。

## 线性 QR

默认 method 为 `"linear"`，先识别对参数仿射的原始结构。系数可为输入变量的可编译初等表达式，因此多项式回归、正弦基函数等可线性拟合；参数乘积/幂/函数/分母不隐式切换 LM。保留原模型并在样本/零参数基点检查定义域；原除零孔洞不因约分或乘零而消失。最终实际模型与仿射设计求值需要在机器舍入范围内一致，严重消去或缩放失真不伪造成功。

QR 不生成 m×m 完整 Q，只保存反射工作矩阵/R 与 Qᵀb；列均衡、列主元和相对1e-12数值秩检查，不把不可识别参数当唯一解。列单位很小时仍保留独立方向；若缩放让非零输入不可表示，则明确失败。恢复参数使用二进制尺度处理，避免可表示结果因中间乘除溢出失效。只保证声明的机器路径；数值秩不是精确证书。欠定/列秩不足首版拒绝，最小范数参数族不在本接口声明范围内。

## 非线性 LM

method 可为 `"nonlinear"` 或 `"levenberg_marquardt"`。实际求导器生成每个参数的导数，然后把原模型/Jacobian 编译成只读回调；未支持的导数不改成虚构值。每步以真实列范数和残差范数归一，求解增广系统 `[J/D; sqrt(lambda) I] z = [-r/||r||; 0]`，再恢复真实参数步。通过真实下降与线性化预测计算 gain ratio 调整阻尼，计入拒绝试探；不构造 JᵀJ 正规方程，不自动换成线性解。

| 选项 | 默认 | 真正含义 |
|---|---|---|
| abs_tol | 1e-10 | 正残差范数绝对阈值 |
| rel_tol | 1e-8 | 非负阈值乘初始残差范数 |
| gradient_tol | 1e-8 | (0,1) 内最大 Jacobian列/残差夹角余弦阈值 |
| max_iterations | 200 | 1..100000，包含拒绝的LM试探 |
| initial_damping | 1e-3 | 正、有限、无量纲初始阻尼 |
| precision | machine | 只机器算法，高精度不降级 |

这些容差/阻尼/迭代项只适用 nonlinear，linear 显式传入时拒绝，不静默忽略。停止依据为真实残差范数容差或列/残差夹角容差；机器步长不再改变参数但容差未满足、未收敛、不可识别 Jacobian、溢出或中断均失败。当前点的 Jacobian 必须数值满列秩，退化初值可能需要选择其他起点。LM 是局部数值拟合，不能保证全局最优或捏造统计置信区间。

## 模型与报告

返回 record 包含 model、parameters（文字键记录）、point（声明顺序）、parameter_names、variables、converged、method、termination、guarantee、precision、residuals、residual_norm、sum_squares、rms、sample_count、parameter_count、degrees_of_freedom、numerical_rank、rank_tolerance、iterations、accepted_steps、rejected_steps、evaluations、model_evaluations、jacobian_evaluations、damping、gradient_cosine 与真实 data 表。

residual 定义为预测减观测；RMS分母为样本数，不是自由度。SSE/RMS若超出或低于机器可表示范围，为 Null 并附 sum_squares_status/rms_status，保留实际残差向量和范数，不填成虚假的零。没有计算的协方差/置信区间/显著性不出现在成功报告中。linear 的迭代/完整回调次数为0，另有真实模型与系数调用计数；nonlinear 记录真实LM工作。失败保留原调用和实际部分状态，不显示 converged=true。

`FittedModelData[variables,raw_expression,"machine"]` 是保持内容的受保护纯数据头，不绑定活会话。数值调用沿拟合时的真实编译路径，输入高精度/容器/错误维数拒绝；符号调用以预算内、避免捕获的同时替换还原源式，供 diff/sample/plot 的既有数值准备路径使用。调用重新验证表达式与坐标，保存内容不当作任意代码执行；InputForm 可以往返。参数值和其他常量已冻结，输入坐标仍属于模型，不受外部同名全局变量影响。此数据头不是新增占位回调。`.omnb` 继续只存源码，不持久化拟合缓存或会话。

本项目 Wolfram 入口为 `Fit[data,Model->expr,Parameters->starts]`；Wolfram传统 basis-list Fit、FindFit/模型统计的其他参数与返回形式尚未适配，不能简单声明同名语义兼容。专门数据/数值诊断界面与图形联动按 R3.5验收。

## 验证与方法依据

纯算法测试在 [fitting.rs](../../crates/om-analysis/tests/fitting.rs)，语言/数据/模型测试在 [求值测试](../../crates/om-eval/tests/fitting.rs)。独立验证1001行线性解、噪声回归 slope=1/2/intercept=7/6 与正规方程残差、指数振幅2/率0.3、实际非零残差、可调用模型/同时换坐标/InputForm往返、伪造数据不执行、数值秩/尺度、未收敛/取消。原53数学期望不改。

算法由本项目编写，方法参考 [LAPACK列主元QR](https://www.netlib.org/lapack/lug/node42.html)、[DTU《Methods for Non-Linear Least Squares Problems》](https://www2.imm.dtu.dk/pubdb/edoc/imm3215.pdf) 与 [SciPy非线性最小二乘说明](https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.least_squares.html)。这些资料不作为运行时依赖或本项目已支持任意优化方法的声明。

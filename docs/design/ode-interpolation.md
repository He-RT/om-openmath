# ODE、连续插值与真实采样

[下一版账本](../plan/NEXT_RELEASE.md) · [科研接口](../reference/analysis.md) · [语言指南](../language.md)

本页描述 `.3` 开发版 `dev` 的真实实现；公开 `.2` 不含这些入口。`om-analysis` 不依赖表达式、网络、文件、线程或数学运行时，使用现有 Interrupt，禁止 unsafe，WASM 与 iOS 共用同一实现。

## 接口与返回

```text
let motion = ode(fn(t,y)=>[y[2],-y[1]], initial:[1,0], t:0..6)
motion.solution(1.25)
sample(motion.solution, x:0..6, count:100)

let curve = interpolate([[0,0,0],[1,1,3]], method:"hermite")
curve(0.37)

let stopped = ode(fn(t,y)=>y, initial:1, t:0..2, event:fn(t,y)=>y-2)
stopped.domain
```

`ode` 输入为右端可调用对象与时间范围，`initial` 必填。标量初值要求标量右端，列表初值要求等长列表右端，包括一维列表；输出取值保持这一区别。状态维数 1..64，时间可正向、反向或零跨度，数值必须有限。命名时间轴只描述范围；右端与事件采用自己的函数形参，不读取轴的全局绑定。其他参数来自会话快照，整个右端与事件在只读作用域中展开并编译一次，原始除零孔洞不会在采样前被约消，定义写入明确拒绝。

| 控制项 | 默认 | 实际约定 |
|---|---|---|
| `abs_tol` | `1e-10` | 正的有限分量绝对误差容差 |
| `rel_tol` | `1e-8` | 非负有限分量相对误差容差 |
| `max_steps` | `100000` | 1..1000000，接受和拒绝的尝试均计数 |
| `initial_step` | 时间跨度的 1% | 正的有限步长幅度，方向由范围决定 |
| `max_step` | 不额外限制 | 提供时要求正的有限幅度 |
| `event` | 不检测 | 可调用的连续实标量零交叉函数，参数为时间/状态 |
| `method` | `"dormand_prince"` | 也接受 `"dormand_prince_5_4"`，无隐式刚性切换 |
| `precision` | `"machine"` | 精确有限常量可显式数值化，高精度输入或其他精度请求拒绝 |

结果是记录，包含 `solution`、有方向 `domain`、`converged`、`termination`（`"end"`/`"event"`）、`accepted_steps`、`rejected_steps`、`evaluations`（真实右端调用数）、`event_evaluations`、`abs_tol`、`rel_tol` 和 `method`。容差和收敛状态描述局部误差控制与到达终点/事件，不是全局误差证书。初值事件为零时立即终止，右端调用数为零；零跨度不执行右端。失败保留原调用和明确诊断，并报告最后实际时间/状态及已完成工作，不伪造成功解；数值层 Failure 保留完整已完成部分轨迹。真正预算耗尽或取消沿 Abort 返回。

## 算法与界限

Dormand–Prince 5(4) 使用七阶段嵌入对，第五阶公式推进，第四/第五阶差作为局部误差估计。使用每个分量的 `abs_tol + rel_tol * max(abs(y),abs(candidate))` 归一，最大归一误差决定接受与步长调整。接受步后复用末阶段导数；节点间使用四次连续扩展，事件在该连续曲线上二分定位。所有节点、系数和中间数检查有限值，保存数据最多 100000 标量。

事件只保证初始点/接受步端点为零，或接受步两端符号变化的检测；要求事件函数连续。触碰零点、同一步内多次交叉或高频振荡不保证，用户可提供 `max_step` 控制检测分辨率。首版不包含刚性、DAE、PDE、复状态或任意精度；步长停滞、非有限右端、形状变化、工作/存储限额和未收敛均明确失败。无需靠附近样本制造数学证明。

公式依据：[Dormand–Prince 论文](https://doi.org/10.1016/0771-050X(80)90013-3)、[Shampine 连续扩展论文](https://doi.org/10.1090/S0025-5718-1986-0815836-3) 与 [SciPy RK45 方法说明](https://docs.scipy.org/doc/scipy/reference/generated/scipy.integrate.RK45.html)。本项目独立编写 Rust 算法，数学系数不构成第三方运行时依赖，没有复制其程序代码。

## 插值对象与采样

`interpolate(points, method:"linear")` 要求严格单调的有限节点，支持递增/递减坐标、标量或 1..64 维值。线性节点为 `[x,value]`；Hermite 节点可以为 `[x,value,slope]`，整表必须使用相同形状。Hermite 未提供导数时，端点使用相邻割线、内部使用邻点割线估计；对象中的来源为 `"estimated"`，实际导数则标 `"supplied"`，不能把估计写成真实导数。线性方法拒绝导数输入。默认不外推；重复节点、维度混用、非法方法与高精度输入明确拒绝。

`InterpolationData[times,values,coefficients,method,scalar,slope_source]` 是受保护、保持内容的数据头，按节点保存值，每段每个分量保存归一坐标 `u` 的 `u..u^4` 系数。普通公式的次数和方法必须对应，端点连续性按浮点舍入容差验证。对象不绑定活会话；InputForm 可往返，调用时重新验证形状、有限值、域和资源界限，数据内部的表达式不会当作代码执行。只有数据头，不把它登记为规划占位回调。可调用属性让编辑器识别保存的插值对象；字段结果后接圆括号是显式调用，例如 `motion.solution(t)`，乘法使用 `motion.value * (x)`。

`sample(fn,x:a..b,count:100)` 支持只读标量/向量函数或插值对象，包含端点，count 为 2..10000，范围非退化且允许反向。返回真正的 `DataTable`，列为 `x` 和 `value`；向量值保存在 value 列，不补零。所有节点必须在机器精度下保持严格有向顺序，总标量最多 100000；外推、重复采样点、非有限结果、写入与高精度拒绝。CSV 只处理标量单元格，向量表可用 JSON；专门表格/数值诊断展示及图形联动留 R3.5。

Wolfram 模式当前提供 `Ode`、`Interpolate`、`Sample` 的本项目接口。`NDSolve/NDSolveValue` 的方程/初始条件形式及 `Interpolation/ListInterpolation` 的不同参数语义没有实现兼容适配，因此没有直接添加误导性别名。

## 验收

实际测试在 [纯 ODE](../../crates/om-analysis/tests/ode.rs)、[纯插值](../../crates/om-analysis/tests/interpolation.rs)、[语言接口](../../crates/om-eval/tests/ode.rs) 与 [笔记本依赖](../../crates/om-kernel/tests/reactive_edges.rs)、[真实内核协议](../../crates/om-kernel/tests/composition.rs) 与 [实际 WASM 调用](../../app/src/kernel/scienceWasm.test.ts)。使用指数/简谐解析解检查节点间真实值，固定步长减半检查第五阶收敛，检查反向时间、事件 ln(2)、零跨度与取消后的完整节点。另覆盖真实导数三次 Hermite、极小线性区间、64 状态加时间、InputForm 往返、局部坐标、记录字段调用、伪造系数/执行表达式、非有限、外推、高精度和限额失败。原 53 条数学语料和门槛不改，本机不启动模拟器。

# 现代语言与统一科研接口设计

[文档导航](../README.md) · [当前语言](../language.md) · [全景功能目录](../reference/README.md) · [下一版账本](../plan/NEXT_RELEASE.md)

本文是 `0.1.0-pre-alpha.3` 的设计规格，不是当前 `.2` 的功能声明。当前已有调用以语言指南、真实注册表及目录的「当前签名」为准。组合语法、现有求解/化简模式和命名绘图轴已在R3.2接通；尚未交付的科研函数及三维示例仍是规划。

## 决策与兼容基线

用户选择渐进演进、科研计算与绘图优先、共享入口与显式模式、常用符号加稳健数值、桌面/Web 优先三维。当前的现代/Wolfram方言继续解析到共享表达式树，既有 Wolfram 内置符号 ID 只追加，不重排。函数大小写名称和旧别名继续接受；规范文档、补全和未来 AI 提示使用小写 snake_case。

`let` 赋值、裸 `=`/`==` 等式、`^`/`**` 幂、隐式乘法、函数调用空格规则、`#` 行注释、`;` 输出抑制、`where` 与规则、现有小数精度全部保留。列表用 `[]`，旧 `{a,b}` 仍是列表。现代索引从1起，负索引从末尾；当前 `v[0]` 取表达式头的兼容行为保留，不能改成零起始索引。

旧源码不自动格式化、重写或迁移；`.omnb` 继续保存 v1 源码。新语法源式要求 `.3`，旧客户端能保存原始文字不意味着可以执行新语法。既有 Wolfram InputForm 与数学期望不因现代显示名称调整而变化。

## 统一入口

| 功能族 | 规范接口 | 默认值与保证 | 兼容入口 |
|---|---|---|---|
| 完整求解 | `solve(eqs, vars, mode: "exact", output: "rules")` | 默认精确与现有规则列表；numeric沿用完整有限代数解的验证 | Solve/NSolve/SolveValues/NSolveValues |
| 局部求根 | `find_root(eq, vars, initial: values)` 或 `bracket: a..b` | 初值/括区间二选一；不宣称完整根集；默认规则，单变量可请求value | FindRoot/findroot |
| 条件与消元 | `reduce`、`eliminate` | 各自保持布尔解集/消元关系语义 | Reduce/Eliminate |
| 化简 | `simplify(expr, level: "basic", assumptions: true)` | deep调用真实深层路径，仍保护主值与定义域 | Simplify/FullSimplify |
| 微分 | `diff(expr, x, order: 1)` | 具名阶数或旧混合规格；grad/jacobian/hessian独立 | D/diff/derivative |
| 积分 | `integrate(expr, x, mode: "exact")` | 默认符号；numeric要求边界，不静默转换 | Integrate/NIntegrate（目标） |
| 绘图 | `plot(expr, ...axes, view: ...)` | 一轴line、二轴surface；contour/density明确选择 | Plot/ContourPlot及后续绘图族 |
| 参数图 | `parametric_plot(vector, ...axes)` | 一参数曲线、两参数三维曲面；坐标长度决定空间维度 | ParametricPlot/ParametricPlot3D（目标） |
| 优化 | `optimize(expr, vars, goal: "min", scope: "local")` | 局部为默认；global仅限已认证支持的凸二次子集 | FindMinimum/FindMaximum（目标） |
| 拟合 | `fit(data, model: ..., parameters: ..., method: "linear")` | 给真实残差/状态；不制造置信区间 | Fit/FindFit及模型族（目标） |
| 数值化 | `numeric(expr, precision: "machine")` | 指定精度不恢复已经损失的信息 | N/n/numeric |
| 根表示 | `algebraic_root`、`nth_root`、`cbrt` | 认证代数根、主值n次根、实数立方根分开 | Root、root、cbrt既有语义保留 |

`Minimize/Maximize` 等全局语义不能被映射成未声明的局部搜索。无法支持的精确计算保留原式及诊断；只能在用户明确请求 numeric 时进入数值路径。条件、自由参数、重数、原始极点、变量顺序继续保留。

## 组合语法与降级到表达式树

以下示例需要 `.3` 开发版本；`integrate`、深层科研能力与 `explore` 按后续批次交付：

```text
let f(x) = sin(x) + x^2
f(x) |> expand() |> simplify(level: "deep")
[1, 2, 3] |> map(fn(x) => x^2)
plot(sin(x), x: -pi..pi)
integrate(exp(-x^2), x: -inf..inf, mode: "numeric")
let config = {color: "green", opacity: 0.8}
config.color
A @ b
explore(plot(sin(a*x), x: 0..2*pi), controls: {a: 0.1..5})
```

| 新构造 | 明确规则 |
|---|---|
| `expr |> call(...)` | 将expr插入元数据指定的主要位置参数；不重复求值；`map(fn,data)`位置2、`fold(fn,initial,data)`位置3，其余按目录定义；没有管道位置时诊断 |
| `fn(x,y) => body` | 降为既有Function语义；形参遮蔽全局名字；自由全局符号使用当前会话并参与依赖分析；外层形参替换后的值保留；不引入系统调用 |
| `lo..hi` | 有方向、闭端点范围；`1..3`不能被词法器误读为小数；`1.23`原义不变；步长用显式step；图轴要求有限有序范围，积分/ODE允许反向区间 |
| `v[2..4]` | 1起始闭区间切片；切片中不使用0取头；越界和无效步长明确诊断，不默默截断 |
| `{key: value}` | 有序记录；键为标识符或字符串且不求键的会话值；重复键报错；空记录用`record()`，不改变旧`{}`为空列表 |
| `record.key` | 不求值的文字键查找；缺失键诊断；小数点仍由数值词法规则处理；不添加任意对象方法副作用 |
| `A @ b` | 降为Dot，检查向量/矩阵维度，`*`的既有逐项行为不改 |
| 命名轴 | 微积分/绘图签名内的`x:lo..hi`把x局部化；函数正式选项名优先；冲突时可用旧位置轴`[x,lo,hi]` |

匿名函数的 `fn` 按上下文识别，已有 `fn(x)` 用户函数调用不能被当作声明。新增别名先检查既有用户绑定及局部形参；原有受保护内置名称的规则不因新别名失效。不新增裸单字母保留名称。

### 优先级

从低到高：语句分隔 → 管道 → where → 规则 → or → and → not → 比较 → 加减 → 乘除、隐式乘法、矩阵@ → 一元负号 → 右结合幂 → 后缀阶乘 → 调用/索引/字段 → 原子。

管道左结合；`-x^2`仍为`-(x^2)`，`2^x y`仍为`(2^x)*y`。`fn` 的body延伸到当前组结束，必要时用括号限定后续管道。新增优先级只作用于新增构造；原优先级测试逐条保留。

### 参数与精度

每个函数拥有自己的参数表，不把所有选项附到每个函数。识别别名后规范化键，拒绝重复键、未知键、互斥模式及不适用参数；给出原源码位置和可执行修复。一般使用`domain`、`precision`、`assumptions`、`method`、`max_iterations`、`abs_tol`、`rel_tol`、`steps`；额外选项必须在条目里登记。

`mode: "exact"`是符号/精确数学请求；`mode: "numeric"`是用户明确接受的近似请求。`precision`为十进制有效位或`"machine"`，不等于显示小数位。现有N支持的精度与求根精度边界分开声明。新增积分/ODE/优化/拟合首版仅机器路径，拒绝冒充任意精度。

```text
decimal("0.1", precision: 50)  # .3规划：直接十进制字符串到大浮点
numeric(1/10, precision: 50)  # .3命名写法；当前可用 N(1/10,50)
```

## 元数据、结果与安全

函数目录定义规范名、兼容名、签名、命名参数及默认值、管道位置、保持/求值角色、返回类型、精度模式、平台、当前证据和计划验收。R3.1 已接入实际回调元数据：当前规范拼写、参数字面校验、保持角色、管道位置、补全、Hover 与 AI 可用名称由 functions.toml 的 runtime 区域生成。R3.2 已接通组合语法及现有回调的显式模式；未实现的数学算法继续按后续批次实施。当前回调详见[可执行接口](../reference/executable.md)。

当前API返回形状保持不变；数值诊断、表格、插值、单位/记录和场景是新增对象。规则值投影要保留变量顺序和条件。矩阵、统计、ODE、拟合的结果不得把误差估计显示成严格认证。插值默认不外推。

表达式、lambda、颜色函数、参数探索都经过同一解析器及受预算约束的只读计算；不执行宿主JS/GLSL/shell，不开放任意文件或网络。参数探索不修改主会话定义；变参、切文档、取消、后台与关闭都过滤旧代次。

## Notebook Agent 的框架无关预留

本节于2026-10-05按用户要求补充，属于后续接入契约，尚未实现。当前现代语法和科研功能设计为其提供基础，不提前引入Agent框架或写入工具。

- 规范函数ID保持稳定，别名和显示名称不作为唯一身份。参数的文字说明与实际类型/必填/枚举/默认值分开，后续可生成模型无关的工具描述；能力查询区分实现、精度、展示平台与任务权限，规划条目不能被推荐为当前可执行工具。

本轮目录先分配独立 `fn_000001` 等稳定身份，[身份账本](../reference/function-identities.json)锁定已有注册归属；规范名称可变但ID不自动重分配。目录 `metadata_version=12`，runtime 区域已提供可验证类型、必填、枚举、数值范围、字面或上下文默认值。GetFunctionCatalog 仅返回实际回调；GetCapabilities 区分内核身份、调用方指定的展示平台与未来任务权限（当前为 null）。副作用标签不授权任意嵌套表达式，`unclassified` 不授予执行权。目标参数文字与六项 Notebook 工具仍是预留，不作为可执行 schema。

- 可执行入口声明 `pure` / `read_session` / `write_session` / `write_document` / `host_io` 副作用类别。标签只用于描述和筛选，只读求值仍由实际执行环境保护；间接用户函数、随机状态与宿主操作不能绕过约束。
- 解析、求值、数据结果和公共Notebook协议不携带Pi/Rig类型。框架适配器可以转换消息与工具格式，不能改变精确/近似、局部/全局、条件或版本冲突的业务含义。
- 后续Notebook工具通过应用接口读取、批量修改和执行；保持手工编辑与Agent编辑共用版本及写入入口。原子源码修改与计算分开，结果绑定源码版本，IME、切文档和迟到回复继续遵守现有保护。
- 会话、提示词版本与工具日志独立于 `.omnb` 源码存储；凭据不进入模型上下文、Notebook和数学导出。修改/执行/持久化的成功状态分别取实际回执。

事务、幂等、撤销和验收细节见[下一版计划的Agent接入预留](../plan/NEXT_RELEASE.md#后续-notebook-agent-的接入预留)。

## 兼容与迁移验证

- 原现代/Wolfram测试、原53语料和旧.omnb往返保留数学期望。
- 明确测试新名字与既有用户函数冲突、局部遮蔽、1起始索引/0取头、函数空格、`root`/`Root`区分。
- 验证pipe只求值一次、Map位置2、Fold位置3，求解保持原始孔洞和极点；显式Cancel/替换的原语义不回滚。
- 新语法每个正例配错误/歧义/修复/UTF-8位置向量；不能通过修改旧数学期望来接受新语法。
- 当前AI提示、当前可执行补全不引入规划名称；实现与回归通过后再按能力元数据启用。

设计参考：[Wolfram规则与模式](https://reference.wolfram.com/language/guide/RulesAndPatterns.html)、[方程求解](https://reference.wolfram.com/language/guide/EquationSolving)、[数值精度](https://reference.wolfram.com/language/guide/NumericalEvaluationAndPrecision.html)。这些资料用于语义核对，不代表OpenMath已实现相同范围。

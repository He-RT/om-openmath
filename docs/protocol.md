# 内核 JSON 协议

[English / 历史详细参考](protocol.en.md) · [文档导航](README.md)

CLI、WASM Worker、Tauri 原生宿主共享 `om_kernel::Session` 及 `om_kernel::protocol`。当前实现覆盖求值、响应式笔记本、编辑辅助、绘图、配置持久化和完整 LLM 任务。协议的准确字段以 [Rust 定义](../crates/om-kernel/src/protocol.rs)及[生成的 TypeScript](../app/src/kernel/generated/)为准；以下按当前实现整理，不按历史里程碑推断功能状态。

## 消息与表示

请求、响应使用 `Envelope<T> {id,body}`；响应 ID 与请求一致，事件 ID 为零。浏览器相关数字 ID 和毫秒超时不得超过 `Number.MAX_SAFE_INTEGER`。LLM 另用字符串 `request_id`，客户端不要复用已结束的 ID。

```json
{"id":42,"body":{"type":"evaluate","cell_id":"a","source":"solve(x^2=2,x)","dialect":"Modern"}}
```

操作标签是 snake_case；解析方言为 `Modern` / `Wolfram` / `Auto`，配置方言为 `modern` / `wolfram` / `auto`。单元格类型/状态、token 类别和消息等级保留 Rust 大小写。解集 kind 为 `finite`、`all`、`none`、`region`；验证为 `"Exact"`、`"ByConstruction"`、`{"Numeric":{"digits":200}}` 或 `"Unverified"`。

可选值通常序列化为 null；为保持旧 JSON 兼容，部分新增可选字段在不存在时省略。元组/坐标/范围为数组，参数映射键为字符串。多数输入字段必需，Option 可省略，配置节按文档默认；未声明的工具/JSON 能力为 false。

Preview 扁平化为标签对象：

```json
{"type":"preview","latex":"x^2","diagnostics":[],"tokens":[],"dialect":"Modern","actions":[]}
```

## 请求职责

| 请求组 | 契约 |
|---|---|
| Evaluate / RunAll / Interrupt | 真实求值、批量运行和中断；Text/Ask 不走数学 Evaluate |
| UpsertCell / DeleteCell / MoveCell | 源码同步、删除及最终零基索引移动 |
| LoadNotebook / SaveNotebook / RenameNotebook | 源码文件载入、保存和不重置定义的标题修改 |
| GetNotebookState / RestoreDefinitions | 源码、静态依赖、状态、实际定义顺序和循环；恢复只运行定义单元格 |
| GetConfig / SetConfig / SetSystemLanguage | 配置读写及临时宿主语言；Auto 不被改为固定语言 |
| GetVariables / Complete / Hover / Preview | 实际定义、无求值编辑服务 |
| InspectExpression / SamplePlot | 有界只读表达式检查和内核采样 |
| LlmTranslate / Explain / Complete / Chat / FixError / TestProfile | 共享模型任务和草稿连接测试 |
| LlmHttpChunk / End / Cancel | 浏览器传输反馈、终止及取消 |

具体 snake_case 标签、必需字段和返回联合类型参见生成类型，不能从展示名称自行构造协议。

## 源码文件与求值

NotebookFile 只含 `version`（当前为 1）、`title`、有序 `cells`；每个单元格仅 `id`、`kind`、`source`、`dialect`。配置、密钥、对话、定义和输出缓存不写入文件。

LoadNotebook 原子校验版本和非空唯一 ID，再重置定义/历史/缓存，保留配置绑定并取消模型任务。Math/Ask 恢复为 Stale，Text 为 Done。RenameNotebook 不重置定义。

每个数学单元格先完整解析，解析错误不执行任何语句。求值失败停止后续语句，保留此前真实副作用；隐藏输出仍进入历史并保留诊断。`exec_count` 是最后成功语句索引；`Cell::input/steps(index)` 暴露实际输入与推导。

输出来自实际求解元数据，保留变量顺序、条件、重数、证据和步骤。普通列表/缓存值/无关包装/不支持结果保持 Expr，不冒充解卡片。SolveValues、NSolveValues、FindRoot、Reduce、Roots 同时保留实际结果形式与原始绑定/区域。关闭步骤不消除验证证据。

## 响应式依赖与恢复

源码分析不求值；defines/uses 包含潜在赋值目标和自由函数头，并排除内置及词法绑定。实际定义归属独立跟踪，失败前的真实写入也保留。

响应式开启时，重复定义所有权在执行前报 `err.multiple_definitions`。成功启动清除本单元格拥有的定义，按拓扑层和文档顺序传递依赖。循环成员报 `err.cycle`，被阻塞的下游保持 Stale；未就绪的闭包外前置节点也阻止自动运行。自动尝试发 Queued、Running、最终状态、CellOutput；`Evaluated.reran` 只列实际尝试的单元格。

关闭 auto_run_dependents 仅标依赖过期；关闭 reactive 允许顺序重定义。UpsertCell 不执行数学，重复相同源码保留状态；改为 Text/Ask 释放实际定义及 CAS 输出。DeleteCell 删除自己的实际定义再传播；MoveCell 只改顺序。RunAll 按真实 planner 路径处理，Text/Ask 不执行。

UI 通过源码/运行代次检查隔离旧输出。运行前同步编辑，中断后合并还在前端队列中的最新源码，保存期间有新编辑仍标未保存。原生中断保持现有 evaluator；浏览器替换 Worker、恢复源码/内存配置并只重建定义，其他结果过期。

## 中断与资源

宿主注入 `Arc<dyn Clock>` 用于期限和计时；没有时钟时 timing_ms 为零，墙钟期限关闭，步数和取消仍有效。单元格所有语句共享 Interrupt；一次发起 Evaluate/SamplePlot/RunAll/自动 Delete 级联只重置一次取消标志，配置/保存/源码编辑不重置。

原生宿主可独立设置 interrupt_handle。只读模型工具要求真实时钟、独立取消和最多 5 秒期限，无时钟则拒绝。表达式检查最多 5 秒与 1,048,576 步；源文本/模型帧等有确定的 1 MiB 边界。

## 输出、步骤与绘图

StepsView.root 存 StepView：稳定 ID、规则 ID、级别、`step.<rule_id>` 标题键、LaTeX 参数及 before/after、子节点。内部 Expr/StepKind 不跨协议。BindingView 的可选 var_latex/root_index/radicals 来自实际格式化和认证转换；精确值数值提示来自只读 N，格式化不改变历史。拷贝保留原始 C[n]，展示别名不改变语义。

LlmExplain 的可选 out_index 指定实际语句记录，省略时用最新记录。未知索引/步骤、过期/失败单元格拒绝。选中步骤仅发送它及后代；相同 S1 在不同输出树中不重新编号。

SamplePlot 校验有限有序范围、不同用户轴、实参数和源码。函数从 400 点开始，边界/曲率最多细化 6 层；隐式用 160×160 网格及确定拼接。极点/跳跃不当作零，非实/未定义样本省略，无限几何不序列化。默认 y 使用 2%–98% 有限分位和边距，显式范围优先。

```text
Plot[Sin[x],{x,0,2*Pi}]
Plot[{x,x^2},{x,-2,2},PlotRange->{-1,5}]
ContourPlot[x^2+y^2==1,{x,-2,2},{y,-2,2}]
plot(sin(x),[x,0,2pi])
implicitplot(x^2+y^2=1,[x,-2,2],[y,-2,2])
```

Plot/ContourPlot 是受保护 HoldAll，轴局部绑定、边界读取外值。明确绘图和自动求解绘图都走内核 sampler；渲染失败保留真实语句记录并报告 Error。单独采样无历史。自动图像基于原始等式/域/完整解，unsupported/自由轴/无限族不构造假几何。

PlotRequest 可选 solve 携带 InputForm 原式及域，PlotData 可选 highlights 携带最新点/区域；缺失字段保持旧 JSON。存在 highlights 时使用响应，即使空数组也清除旧点。参数以有限 binary64 的精确有理值代入，并按原始极点/域重新 NSolve/Reduce；不修改原卡片证据/步骤。普通没有 solve 的请求保留显式旧高亮。

前端只渲染采样数据，单图一次在途、最新队尾、150 ms 视窗去抖和 33 ms 滑块节流。代次防止旧回复安装；过期/重启/unmount 隔离旧几何，失败真实显示可重试。

## 编辑辅助

Complete/Hover/Preview 不执行源码、不写历史/定义、不重置中断。游标和替换范围是 UTF-8 字节，非边界位置报 `err.cursor`。客户端负责 UTF-16/字节映射；方言/常量遵循配置和 `%wl` / `%modern` 标记。

Complete 替换整个标识符，按前缀、驼峰/下划线首字母、子序列排名；真实内置、符号、关键字、snippet 和内层调用选项稳定排序，最多 50 个。注释/字符串/数字/结束调用不泄漏选项。Hover 给实际本地化文档或不求值的已保存定义，最多 200 Unicode 字符。Preview 真实诊断/修复/token，Error 时无 LaTeX/actions；多个语句按游标选中，生成 Solve action 保留原极点和注释。

## 配置与凭据

`Session::new` 保持纯；`Session::new_native` 显式绑定 ConfigStore。默认路径由 `ProjectDirs::from("org","openmath","OpenMath")` 给出；不存在使用默认，部分 TOML 补默认，无效文件不覆盖。

api_key 序列化/Debug 为 `***`。SetConfig 调用 merge_redacted_keys 按配置名称保留旧 key；null/缺失清除，新名称上的掩码无 key，空字符串是明确值。原生新 key 默认入 service=openmath/user=profile 的系统库；显式 Plaintext 回退保留权限，不把失败写库静默降为明文。

解析顺序环境变量→凭据库→回退字段；真实 SecretKey 只供传输。配置只显示存在掩码，不复制解析值；移除配置释放管理的密钥，外部环境变量不删除。实际文件使用邻接独占临时文件、flush/sync、原子替换及私有 Unix 权限；失败回滚库，回滚失败明确报错。错误不带密钥或源码片段。

requires_api_key 默认 true，默认省略；false 表达无密钥服务意图但不绕过验证。extra_body 默认空，不能覆盖协议拥有的字段；有界 JSON 不 Debug 打印值。浏览器非敏感设置独立保存，密钥/请求头/不透明值仅逐配置明确选择后持久化。`KernelConfig.cli` 可省略，默认 ai_hints=false，默认省略整个节。

## LLM 任务与传输

om-llm 是 sans-IO：构造经过校验的 HTTP、SSE/NDJSON 解码、任务状态；核心不访问网络或执行 CAS。SSE 支持拆分 UTF-8、BOM、CR/LF/CRLF、多行 data，帧/行 1 MiB；不完整 SSE 结束丢弃，编码错误拒绝。NDJSON 完整末记录可在 EOF 返回。只消费 OpenAI choice 0，私有 reasoning/未知元数据忽略，工具片段保留实际 ID/索引/名称；等真实 HTTP 成功结束才执行工具或完成。

单个 Session 持有唯一 Job，最多 16 活跃 ID、256 终态 tombstone。Translate/Fix 只发已解析 Suggestion 和 Done，Complete 最多一个过滤后 Delta，Chat/Explain 流真实文本，失败 Error。探测额外给真实 reply/latency_ms/first_byte_ms，没有时钟或字节为 null。

SuggestionParser 严格读取 wolfram/explanation 字符串，要求一条非空未隐藏的 Wolfram 表达式，真实 parser 生成现代/Wolfram/LaTeX，无求值。失败最多两次真实诊断修正。补全保留插入空格、截首逻辑行、按真实 lexer 过滤并避免重复本地补全；上下文至多前三个数学源码，不含 Text/Ask、配置或输出。

只读 evaluate/solve 用真实 CAS、严格参数、独立预算；直接/间接修改定义均拒绝。propose_cell 只解析并发卡片。最多六轮真实工具结果回放，第七轮在执行前失败。配置密钥反射到工具参数时拒绝；公开工具/回放清除凭据，真实 Authorization 仅存在内部 HTTP。

LlmStarted 和后续 LlmHttp 触发传输。浏览器 fetch 禁止重定向，按真实状态/有界 UTF-8 body/期限/abort 反馈。可选 chunk status 保持旧 JSON，非 2xx 不变模型文本，冲突状态失败；解码错误立即结束，迟到数据不能复活。

原生宿主截获所有内部 HTTP DTO，不向前端发送 URL/header/body。公开 Started 的 http=null；实际字节进入唯一 Session Job，不重构 SSE。native_client 保持 TLS/代理、禁止重定向。KernelHost 独立 owner 线程、64 队列、2 异步 IO worker、16 有界 blocking byte worker；取消/期限覆盖排队与确认，销毁取消并 join。

TauriClient 使用 kernel_request、kernel_subscribe、kernel_interrupt，secret_set/delete 只写绑定凭据库。WASM Kernel 接收 JSON Envelope，返回 response/events，注入 Date clock；错误 JSON/零或不安全 JS ID 不执行。Worker 加载实际 bindgen binary，恢复不运行所有计算。

## UI 与兼容验证

Ask/修复卡片、解释、ghost 和对话使用同一任务。建议不自动执行；对话仅 controller 内存，显式 @cellN 展开当前源码/结果，过期结果省略。每个 client/目标/上下文范围分别确认；取消在迟到 start 后再次发 cancel，避免遗留传输。

LlmTestProfile 可选 config 测试未保存草稿；省略保留按名称探测。测试不 SetConfig、不写 TOML/库/定义/历史，只发送 ping。设置草稿保存前不改变运行配置。

生成绑定：

```sh
cargo test -p om-kernel export_bindings --locked
git diff --exit-code app/src/kernel/generated
```

所有可达 DTO 提交到 generated，禁止手改。CI 同时检查差异与新增未追踪文件；前端检查类型收窄和代表 JSON。协议 fixture 保留旧 shape，扩展字段独立测试。真实 Windows 安装窗口、生产 WASM、原生宿主测试补充真实执行证据，不能以 MockKernel 替代。

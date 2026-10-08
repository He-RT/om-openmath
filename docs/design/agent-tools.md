# Mac Agent 工具契约设计

[Agent 架构](notebook-agent.md) · [上下文组装](agent-context.md) · [操作指引](prompts/notebook-operation.md) · [完整 JSON Schema](agent-tools.schema.json)

模型输入必须同时包含主提示词、当前运行环境和全部实际暴露工具的声明。主提示词说明目标与使用顺序；工具声明说明何时调用、参数、返回结果、错误和副作用。当前设计为 12 个模型工具与独立宿主控制，尚未注册到 `.3`；机器契约逐项标为 planned，结构校验不等于实际执行或授权验收。

## 模型实际收到的信息

每轮请求使用同一份已验证工具目录生成以下内容：

1. 主提示词包含软件说明、方言、工作流程、模式、版本及短工具总览。
2. 原生工具字段携带所有本轮可调用工具的 name、完整 description 和 parameters，不只提供名称列表。description 明确用途、前置条件、关键返回字段、错误恢复、副作用和是否可重复调用。
3. 工具结果的模型可见 content 含必要的版本、稳定身份、引用、成功/失败和后续动作；完整数据按需读取。
4. 详细数学函数通过 search_functions/get_function_docs 查询。CAS 函数与笔记本操作工具是不同接口，全部科研函数不各注册成一项模型工具。

不是把 schema 隐藏在程序里，也不在主提示词重复粘贴所有 schema。它们作为完整模型上下文的不同部分发送，纳入 ContextSnapshot 和预算。工具声明的实际 wire 内容、主提示词的名称与顺序以及宿主 handler 必须对应；未实现入口不得出现在原生工具集合或可调用总览中。

## 完整工具目录

JSON Schema 是本设计的参数与返回结构主来源，包含输入联合类型、约束、必填项、示例和结果封装。此表给出整体用途：

| 工具 | 模式与效果 | 主要用途和结果 |
|---|---|---|
| read_notebook | 讨论/执行，读取 | 提供 outline/cells/symbols，返回 snapshot_ref、源码/版本、稳定 cell_id、分页和近期事务引用 |
| search_functions | 讨论/执行，查询 | 按目的/名称搜索真实接口，返回 function_id、短签名、支持边界和目录版本 |
| get_function_docs | 讨论/执行，查询 | 返回准确参数/默认值、返回语义、精度、平台与验证过的例子 |
| preview_source | 讨论/执行，临时检查 | 检查 source 或整个 patch，返回诊断、影响范围和冻结的预览计划 |
| apply_notebook_patch | 执行，修改源码 | 仅提交有效 patch preview_ref，返回新快照、事务和失效单元格；不运行或保存 |
| run_cells | 执行，改变计算状态 | 按目标/依赖范围运行，返回实际执行列表、结果引用和完整/部分/失败/取消状态 |
| inspect_result | 讨论/执行，读取结果 | 读取值、步骤、诊断、几何摘要或真实图形预览，保留生产版本和过期状态 |
| evaluate_scratch | 讨论/执行，隔离计算 | 临时 CAS 可定义局部变量，主笔记本/随机状态不变；返回 scratch result_ref |
| read_attachment | 讨论/执行，读取媒体 | 读取主动附加媒体的元信息、文字、图像或帧，返回真实来源与覆盖范围 |
| prepare_attachment | 讨论/执行，宿主处理 | 在已配置处理能力及任务预算内规范化/OCR/转写/抽帧，返回准备后的媒体引用 |
| get_operation_status | 讨论/执行，核对 | 查询实际操作账本，恢复超时/断线的未知结果，不重放动作 |
| undo_transaction | 执行，修改源码 | 在当前版本上撤销获准事务，整体校验反向操作，冲突不覆盖用户编辑 |

首版只声明实际已接通的子集；表中的模式不自行授予权限。讨论模式的计算使用 evaluate_scratch，不能运行主笔记本、改源码或推进主随机状态。媒体工具同样受当前附件与服务范围约束。

cancel_task 是 UI/宿主直接控制，不需要等待模型调用；focus_cell 由用户点击定位触发。模型选择、提示词设置、系统文件选择器和保存也是宿主界面操作。通用终端、任意文件路径、远程 URL 获取、外部插件和自我修改设置不在首版模型工具中。

## 绑定上下文与引用

宿主注入 agent_session_id、agent_turn_id、tool_call_id、task_id、document_id、document_generation、mode、grants、model_config_revision 和预算。这些字段不是让模型自行填写的参数。

模型使用宿主返回的 snapshot_ref、preview_ref、result_ref、media_ref、operation_ref 和 transaction_ref。引用解析检查来源类型、归属、代次、版本及有效期；字符串格式合法不表示引用有效，也不表示拥有权限。分页 cursor 同样绑定原快照，不能跨文档复用。

cell_id 来自读取结果，不使用显示顺序或文件路径。新单元格用 client_key 标识计划内身份，预览时宿主分配最终 cell_id；插入位置及移动可引用先前已声明的 client_key。修改/删除/文本替换只针对已有 cell_id；若需改变计划中的新格，改其 insert 源码后重新预览。模型不手算 UTF-8 字节偏移。

所有读取结果有界且可分页。源码不完整时优先使用唯一片段 replace_text；整格 update_cell 要求完整原源码可用，防止把一个截取片段当整格替换。全文散列按实际 UTF-8 原文计算，不能因格式化或 Unicode 归一化悄悄改变内容；同一计划中多项操作的 expected_source_hash 对照原快照，片段匹配按操作顺序作用于临时文档。

## 读取和接口查询

read_notebook 首次默认 outline，不把大笔记本完整源码/所有输出默认送入模型。cells 按稳定身份选择；symbols 返回实际执行定义及生产/过期状态。响应区分历史快照与当前快照，修改/运行必须使用当前有效版本。

search_functions 只搜索实际回调过滤后的目录，名称、别名和支持说明都可检索。get_function_docs 接受 function_id 或准确 name 二选一；metadata_version 不匹配时明确返回当前资料，缓存只能按同版本复用。二者不执行源码，也不把 deferred 规划条目作为当前能力返回。

函数文档结果复用真实 FunctionDescriptor，并补充来自已通过测试/发行用例的代码示例。用户函数遮蔽内置别名时，额外说明当前会话的实际解析绑定，不能仅凭目录名字覆盖用户定义。

## 检查与提交修改

preview_source 的 input 分为两种：

- source 检查一个指定方言的片段，不形成可提交事务。
- patch 在实际快照的临时文档中应用全部操作、解析所有受影响源码、处理新定义并形成不可变计划。只有满足校验的 patch 才产生可提交 preview_ref。

patch 支持 insert_cell、update_cell、replace_text、delete_cell、move_cell 和 rename_notebook。新增只创建 Math/Text，保留已有 Ask；移动用 after 身份定位，null 表示开始位置；replace_text 必须匹配唯一完整片段。相互冲突、重复 client_key、越界来源或错误类型使预览失败，主文档不受影响。

apply_notebook_patch 的模型参数只有 preview_ref。宿主从冻结计划填入业务层需要的文档身份、预期版本、operation_id 和 operations；不要求模型重新发送源码，也不重新按最新草稿渲染旧计划。它与此前预留的应用层 Patch 接口是两层关系，并未去掉文档版本或幂等契约。

示例为契约演示，引用与散列是合成值，不是已执行回执：

```json
{
  "tool": "preview_source",
  "arguments": {
    "snapshot_ref": "snap-demo",
    "input": {
      "kind": "patch",
      "operations": [{
        "type": "replace_text",
        "target": {"cell_id": "cell-a"},
        "expected_source_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "match": "let a = 2",
        "replacement": "let a = 5"
      }]
    }
  }
}
```

返回有效 preview_ref 后提交：

```json
{"tool":"apply_notebook_patch","arguments":{"preview_ref":"preview-demo"}}
```

一次有效预览对应稳定提交身份，相同 preview_ref 重复提交返回原回执；检查幂等回执先于判断旧预期版本，避免把自己的成功提交误报成冲突。新修改必须重新预览。取消或文档变化发生在提交前时不执行；提交后取消不伪称源码已回滚。

## 运行和试算

run_cells 输入 snapshot_ref、cell_ids、scope 和 continue_on_error。scope 的默认 required 仅补完成目标所需的前置；selected 不扩展目标，前置未就绪时返回 DEPENDENCY_NOT_READY；affected 进一步重算目标改变影响的下游。所有扩展仍受任务允许的单元格范围检查，实际列表明确返回；用户禁止运行其他单元格时不得借依赖扩展越界。

计算等待/进行期间通过 onUpdate 返回真实进度与操作身份；只在实际完成/失败/取消后给最终结果。默认遇到错误停止，部分已经完成的结果和已提交源码保留。每个输出绑定其生产源码散列，过期结果不作为当前结果交给模型。

evaluate_scratch 使用独立数学会话，允许临时定义但禁用宿主 IO、模型调用及主会话写入，随机状态也隔离。use_notebook_definitions=true 要求用到的执行定义与快照源码相符，否则报告 CONTEXT_NOT_READY；false 从空上下文开始。返回 origin=scratch，不能当作左侧已经执行或保存。

inspect_result 读取 result_ref 指定的原始结果，复杂值通过现有有界协议视图/分页提供，精确数不强制转成机器数。steps 必须来自记录，geometry 默认摘要；preview 只返回对应结果实际渲染/采样的媒体，检查模型与宿主能力，显示 data_only/unavailable 时不能声称已经看到图形。

## 媒体处理

media_ref 只解析当前任务主动附加或处理后的资源，不接受任意路径/URL。read_attachment 先用 metadata 核对类型/状态，再读取 text/image/frames。未处理资料返回 PREPARATION_REQUIRED，而不是空内容成功。

prepare_attachment 的 method 与 selection 在完整 schema 中定义；页数、时间和帧数都受实际资源边界检查。page_end/time_end 必须不早于起点；所有选择范围须位于真实文件内。路由取宿主已配置服务和用户模型，参数没有 api_key、base_url 或任意 profile，模型不能替换认证/目标端点。

预处理保留原媒体和实际页码/时间范围，并说明抽帧/OCR/转写的限制。分段准备、转换失败、模型不兼容和取消都保留准确状态；不因只有部分内容就宣称已经理解整个附件。

## 回执恢复与撤销

每次有副作用的调用在发送宿主前获得稳定操作身份。传输重试使用同一个会话/回合/tool_call_id 派生身份；模型主动发起新的 run_cells 调用是新计算，不能把内容相同就一律当重试。出现未知结果时，适配器在错误 content 中保留 operation_ref，模型先 get_operation_status，不盲目重复计算或编辑。

get_operation_status 只查询账本，返回 operation_state、outcome_known、effect_committed 及可用回执引用。查询完成不表示被查询操作完成；账本未完成恢复时保持 unknown，不能用「未找到」推断操作从未发生。

undo_transaction 可用于用户要求撤销及本任务获准回退，UI 也调用同一业务契约。transaction_refs 来自真实回执/可读取历史，宿主决定允许范围；所有反向操作在当前版本验证后原子提交。手工修改冲突返回 UNDO_CONFLICT，新修订保留用户后续内容，不自动运行或保存。

## 返回结构与 Pi 映射

每个工具的 result_schema 都包含以下公共封装，data 另有该工具的明确结构：

```json
{
  "ok": true,
  "status": "completed",
  "request_ref": "request-demo",
  "operation_ref": null,
  "document": {"id":"doc-demo","generation":4,"revision":12},
  "data": {},
  "error": null,
  "pagination": {"truncated":false,"next_cursor":null}
}
```

此处 data={} 仅表示插入工具专用 payload 的位置，不是任意工具的有效成功响应。ok/status 表示本次工具请求结果，数学目标、预览 valid、执行 run_outcome、被查询 operation_state 和 render_state 另行判断。例如检查完成但 valid=false 不能提交；查询完成而 operation_state=running 不能报任务结束。

Pi 1.0.4 的 structuredContent 是程序调用者结果，不能假设自动成为模型输入。适配器必须将必要封装及引用序列化到 content 的 text 中；UI 的完整详情留在 details/structuredContent，真实图像在模型支持时另加 image 内容。[Pi 工具结果类型](https://github.com/badlogic/pi-mono/blob/503c605528f9af993c0e37ede468cf884fb0ff5b/packages/agent/src/types.ts#L392)

失败以 isError=true 及结构化 error 表示，不能只在成功文字中写「失败」。Preview valid=false、计算失败/取消等也提供正确模型错误信号；合法形式表达式/未求值状态则如实保留，不能仅按字符串未变化判断失败。摘要裁剪不能破坏 JSON、精度、错误、引用或必要条件；不足时分页而不是伪造完整结果。

## 错误恢复

| 错误组 | 必须处理的动作 |
|---|---|
| INVALID_ARGUMENT / INVALID_PATCH / INVALID_SOURCE | 核对对应 schema、源码及诊断，修正后重新预览 |
| STALE_SNAPSHOT / EDITING_BUSY / PREVIEW_MISMATCH | 保留用户编辑，读取最新状态或等待合成结束，再检查 |
| NOT_AVAILABLE / NOT_SUPPORTED / MODEL_INCOMPATIBLE | 使用实际目录/模型能力选择可支持的路径，不能补造接口 |
| DEPENDENCY_NOT_READY / CONTEXT_NOT_READY / STALE_RESULT | 明确缺少或过期的生产状态，在获准范围重算/重新读取 |
| TIMEOUT / LIMIT_EXCEEDED / CANCELLED | 查看实际预算和已完成结果；不无限重试、不抹去提交 |
| UNKNOWN_OUTCOME | 先查询操作回执；未核对前不发新副作用操作 |
| UNDO_CONFLICT / PERMISSION_DENIED / INVALID_REFERENCE | 不绕过范围或覆盖现状，说明具体冲突/限制 |

恢复建议只允许本轮实际工具。数值不收敛与系统超时分别保留，不能把 unsupported 当作数学无解。

## 注册与 schema 兼容

实际声明集合为：已实现且验收通过的 handler ∩ 当前任务范围 ∩ 当前模式 ∩ 当前模型/适配器可表示的 schema。描述性 effect_class 不授予权限，预处理/隔离计算中的间接调用也需宿主检查。

完整机器契约使用 Draft 2020-12 的引用与联合类型；发送供应商前展开引用、剔除本地元数据，并按已验证的 strict/optional/nullable 约束转换。不支持 oneOf 的适配器可用明确的平坦判别字段表达，再由同一业务校验器验证条件；不能删除关键参数约束后称为兼容。只接通部分媒体方法或结果视图时，按实际能力缩窄对应 enum，并记录该声明散列；不把整个计划枚举作为现有支持范围。能力不明时先做合成提供商/实际端点的兼容验收。

JSON Schema 的 default 是描述值，只有明确的宿主规范化规则才填入；默认值及规范化后的调用也记录到快照。所有计划内数组保留顺序，schema/名称有版本与散列，不用动态排序改变同一请求的工具含义。

## 实施门禁

参数/输出 schema 与工具 description 从这份目录统一生成，绑定真实 handler 后才进入 Pi AgentTool。现有 GetNotebookState/GetVariables、目录、Preview 和结果视图可复用；原子事务、冻结计划存储、局部调度、隔离会话、媒体处理、幂等回执及撤销都需实际补齐，不能把现有 UpsertCell/RunAll 简单循环包装成完成。

先校验每个契约的正例、缺字段/未知字段、联合类型和模式；运行时再验证引用归属、版本、IME、唯一片段、源码覆盖完整性、预算、partial/取消、未知回执、撤销冲突以及真实多模态载荷。工具说明必须与实际回执一致；合成验证通过不意味着真实模型已完成整个任务。

当前只交付完整工具与上下文契约设计，不安装 Pi、不增加伪 handler、不修改已安装 `.3`。

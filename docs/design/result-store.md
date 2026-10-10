# 不可变结果库与只读投影

R4.1.10的结果基础与真实NativeHost/14号CABI/Swift ResultClient已接通，原值/步骤/几何/solution presentation及完整source分块、history scope和late reply隔离可实际调用。本文仅描述结果读取任务；完整原生工作台、文件/启动恢复、Agent及`.4`发行仍按对应后续任务完成。

## 真实结果与身份

`Session.retain_result`只从真实cell/StatementRecord拷贝immutable原input/value/steps/solver/science/Explore/原已采样OutputItem与readonly evaluator。每个successful statement保留actual out_index/view_id/suppression，actual cell error/message另保留；零成功history的error不伪装没有发生定义副作用。metadata以实际ExprKind和callback记录给出Exact/Machine/HighPrecision/Text与scientific origin，不从`verified`/`converged`等record字段推导保证。

`StoredResult.capture`只从匹配实际accepted checkpoint/receipt建立，通过OMKS data-only decode构造临时owned对象并复制readonly records；没有运行producer source，不把readonly提升为main writer。结果独立保留原receipt/kernel bytes，因此下一次main编辑或执行不能改原value/history/context。构造代码本身不授权checkpoint接纳；真实receipt与Blob确认仍依赖R4.1.09已有physical port。

每whole-cell root有actual durable result_id，每发生项绑定cell/out/view。ResultStore签HMAC runtime-local result_ref，最大TTL10min/128refs，与read grants、runtime/document/generation/task/permission/metadata严格绑定。结果可以跨source/execution/definition revision以history读取，只有Result-purpose允许这个readonly历史解法，preview/snapshot的严格scope不放宽；错owner/view、过期、revoke、不同task/代次或失去读权限拒绝。

freshness基于current真实accepted occurrence/status索引及epoch/config：另一无关格执行增长global history不使旧current结果自动过期，同格重执行的旧view标history_only，依赖失效/源码改变标stale，实际error前prefix标partial。不会只凭当前ordinal相同认为同一输出。补正main单格路径：实际definition重执行的受影响依赖标stale并退还其owned旧值，不自动cascade或重播source；测试确认旧b结果仍能history读取，但新main已没有旧b定义。

## 投影与资源

- values按原value graph route分页，复用已有1..100行/1..32列/32层path；真实整数/有理/高精度表示保留，稳定row/column occurrence跨页一致。
- complete source是显式原value/input/source，不以省略显示串代替；formatter前按实际展开次数检查200000节点预算，避免共享DAG指数展开。Host source请求按明确UTF8 byte offset/limit返回完整边界片段、原total/SHA256/complete；最多8MiB显式source、单片不超过64KiB。Swift默认16KiB片段逐项核对total/hash、完整assembled bytes才返回，不把部分/stale/expired数据标完整。
- steps按原hierarchy/rule/node/child route分页，子列表另读，没steps明确recorded=false。没有调用求解器重造步骤，clone选中节点时不会clone整棵children树。
- geometry默认原采样counts/bounds/availability，点、tile、arrow、path/polygon、mesh position/normal/color/global triangle indices按原buffer分页，至1024项；不前端重采样/三角化。Explore原initial geometry也沿retained item读取，未采样automatic plot request及iOS unavailable明确data_only。
- numeric直接原value在fresh readonly eval上执行N，精度上限1000digits，真实message拒绝不宣称成功；readonly expression检查实际写/seed/compound并保留实际诊断，延迟user-function中的write由原readonly evaluator拒绝。random draw是独立fork本地流，不推进原流/history；Out读取原retained history。
- scratch显式恢复原accepted bytes成owned未接纳临时state，独立flag/clock，reactive=false避免对旧producer格cascade；可临时let/调用，但无main acceptance入口，默认deadline不超过5000ms，返回真实CAS Response。

缓存至32个/64MiB预留载荷预算/4096创建，保留encoded state与readonly投影的预留计数；这是cache reservation而非进程RSS上限，实际renderer/内存门禁仍待平台测试。revoke lookup不提前释放caller pin，真正release后reap，容量不足明确Budget；不会静默执行原source重新创建一个结果或驱逐active main checkpoint。结果cache/短期capability与actual持久化Blob/SQLite事实分开，重启重新签发属于后续恢复端口。

## 当前验证范围

Kernel5例覆盖精确分页/80位decimal/Unicode、步骤、原N投影、Set/SeedRandom/隐藏write、Out/random隔离、原2D/3D/Explore采样逐值比对及held诊断。Host4例覆盖原accepted capture、current→history、generation/grants/TTL/view拒绝、scratch不会污染original bytes、主定义重执行退还依赖以及revoked pin容量。unit中的synthetic receipt只证明算法/契约，不能当新的actual physical接纳证据；R4.1.09真实CABI/SQLite链继续回归。

NativeHost按独立result worker接收actual accepted snapshot，普通数学event只带durable result ID/接纳摘要和pending_projection，不带source/full value/mesh。Manifest独立分页small statement/type/provenance descriptors及HMAC result refs；inspection回实际binding+payload，ordinary payload限制256KiB，超过则明确失败，可按更小页或source片段取值。真正Result UI/Metal/文件与跨重启重绑定继续后续任务。

## 原生读取通道与交付边界

`native-result-store.schema.json`闭合manifest/status/revoke/inspect及10种实际query，全部字段由Rust/Swift生成（总263DTO）。14号CABI复制bounded JSON；SourceEndpoint只检查/冻结当前scope并admit，不在source/UI lock上decode/格式化/求值。4个waiting消息、32pending与4096原request facts有硬上限，thread使用独立cancel/clock与5s/4M ticks；32terminal详情之外明确expired，不复用原ID重执行。status仍检查原runtime/document/generation/task/grants，不能只知道ID就跨scope读回。

原accepted数据decode/readonly capture在worker完成，KernelState.detached保留shared原bytes但由ResultStore另计预算，不保留main pool旧envelope；避免查看历史耗尽main的32-state容量。新result超过cache阈值只驱逐older unpinned cached结果，真实Blob/SQLite记录不删；旧ref明确expired/unavailable，不静默重新执行原source。跨重启/缓存未保留数据的重新加载按后续storage/recovery路径，当前主体renderer内存门禁仍以实际平台测量为准。

presentation沿actual OutputItem/SolutionSetView分页，保留conditions/verification/Root/intervals，没有再次solve。zero成功statement error按actual CellOutput读取message及原span，view/out为null/0，不伪造成功expression。Core_Explore初始几何/control数据来自原记录，新的滑块采样由后续exploration任务负责，当前读取不会变参数。

Source/main/read请求ID共用取消路由，admission检查原namespace，readonly request不能shadow一个正在运行的main或source操作。ResultRuntime direct cancel/state读回不排在inspection后面；close先停止admission和cancel，再提取thread由后台join。权限/能力变更不授予新write；ResultQuery scratch只使用未接纳隔离副本。

Swift ResultTransport在独立控制队列/actor完成request/readback/metadata解码，ResultClient MainActor只管view selection generation。reply在selection/cell/out/view/document或最新confirmed source/kernel revision改变后丢弃；explicit full source可以复制原history内容，但仍核对doc lifetime/selection/total/hash，partial没有返回值。fixture用actual decoded reply pause确定性验证selection变化及main状态变化后的旧reply拒绝，没有在test中把伪数据注入数学结果。

实际验收包含真实C ABI→main receipt→worker→ResultClient的exact/80digits、长Unicode/NFD source fragments、原solution verification/steps、2D/3D buffer、readonly Set/Seed/Random/Out与scratch隔离、current/history/partial、namespace冲突、decoded late reply、summary-only事件、revocation及zero-success parse error。最终完整相关Rust/Swift/主运行/存储/Release链接和同SHA CI另外记录；这些开发证据不填最终candidate gate。

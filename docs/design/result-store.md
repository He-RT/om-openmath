# 不可变结果库与只读投影

R4.1.10正在实施。当前已有纯Kernel实际结果快照、Host缓存/带权限的引用/只读和scratch基础；真实NativeHost结果读取通道、Swift ResultClient、可分页source传输与工作台仍待接通。以下完成项仅指本批实际代码与测试，不把基础API等同完整任务或`.4`发行。

## 真实结果与身份

`Session.retain_result`只从真实cell/StatementRecord拷贝immutable原input/value/steps/solver/science/Explore/原已采样OutputItem与readonly evaluator。每个successful statement保留actual out_index/view_id/suppression，actual cell error/message另保留；零成功history的error不伪装没有发生定义副作用。metadata以实际ExprKind和callback记录给出Exact/Machine/HighPrecision/Text与scientific origin，不从`verified`/`converged`等record字段推导保证。

`StoredResult.capture`只从匹配实际accepted checkpoint/receipt建立，通过OMKS data-only decode构造临时owned对象并复制readonly records；没有运行producer source，不把readonly提升为main writer。结果独立保留原receipt/kernel bytes，因此下一次main编辑或执行不能改原value/history/context。构造代码本身不授权checkpoint接纳；真实receipt与Blob确认仍依赖R4.1.09已有physical port。

每whole-cell root有actual durable result_id，每发生项绑定cell/out/view。ResultStore签HMAC runtime-local result_ref，最大TTL10min/128refs，与read grants、runtime/document/generation/task/permission/metadata严格绑定。结果可以跨source/execution/definition revision以history读取，只有Result-purpose允许这个readonly历史解法，preview/snapshot的严格scope不放宽；错owner/view、过期、revoke、不同task/代次或失去读权限拒绝。

freshness基于current真实accepted occurrence/status索引及epoch/config：另一无关格执行增长global history不使旧current结果自动过期，同格重执行的旧view标history_only，依赖失效/源码改变标stale，实际error前prefix标partial。不会只凭当前ordinal相同认为同一输出。补正main单格路径：实际definition重执行的受影响依赖标stale并退还其owned旧值，不自动cascade或重播source；测试确认旧b结果仍能history读取，但新main已没有旧b定义。

## 投影与资源

- values按原value graph route分页，复用已有1..100行/1..32列/32层path；真实整数/有理/高精度表示保留，稳定row/column occurrence跨页一致。
- complete source是显式原value/input/source，不以省略显示串代替；formatter前按实际展开次数检查200000节点预算，避免共享DAG指数展开。Host传输分块仍待接通，不把一次可能超frame的完整源码作为ordinary event发送。
- steps按原hierarchy/rule/node/child route分页，子列表另读，没steps明确recorded=false。没有调用求解器重造步骤，clone选中节点时不会clone整棵children树。
- geometry默认原采样counts/bounds/availability，点、tile、arrow、path/polygon、mesh position/normal/color/global triangle indices按原buffer分页，至1024项；不前端重采样/三角化。Explore原initial geometry也沿retained item读取，未采样automatic plot request及iOS unavailable明确data_only。
- numeric直接原value在fresh readonly eval上执行N，精度上限1000digits，真实message拒绝不宣称成功；readonly expression检查实际写/seed/compound并保留实际诊断，延迟user-function中的write由原readonly evaluator拒绝。random draw是独立fork本地流，不推进原流/history；Out读取原retained history。
- scratch显式恢复原accepted bytes成owned未接纳临时state，独立flag/clock，reactive=false避免对旧producer格cascade；可临时let/调用，但无main acceptance入口，默认deadline不超过5000ms，返回真实CAS Response。

缓存至32个/64MiB预留载荷预算/4096创建，保留encoded state与readonly投影的预留计数；这是cache reservation而非进程RSS上限，实际renderer/内存门禁仍待平台测试。revoke lookup不提前释放caller pin，真正release后reap，容量不足明确Budget；不会静默执行原source重新创建一个结果或驱逐active main checkpoint。结果cache/短期capability与actual持久化Blob/SQLite事实分开，重启重新签发属于后续恢复端口。

## 当前验证范围

Kernel5例覆盖精确分页/80位decimal/Unicode、步骤、原N投影、Set/SeedRandom/隐藏write、Out/random隔离、原2D/3D/Explore采样逐值比对及held诊断。Host4例覆盖原accepted capture、current→history、generation/grants/TTL/view拒绝、scratch不会污染original bytes、主定义重执行退还依赖以及revoked pin容量。unit中的synthetic receipt只证明算法/契约，不能当新的actual physical接纳证据；R4.1.09真实CABI/SQLite链继续回归。

尚待R4.1.10：NativeHost按后台通道接收actual accepted结果、普通事件仅summary/ref、按引用读取绑定payload/完整源码分块与late reply隔离、Swift ResultClient，以及跨语言actual main输出验收。后续Result UI/Metal/文件/Agent依照对应任务，task目前保持unchecked/in_progress。

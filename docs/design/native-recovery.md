# 原生 Mac 启动恢复实现

2026-10-12。R4.1.12 正在实施，文档投影、原生草稿、本地启动恢复及规范会话/工具/请求恢复账本已验证。长会话恢复索引和原生会话恢复入口已验证；真实Pi/provider绑定由后续Agent任务接入。运行版本继续 `.3`，不代表 `.4` 已就绪。

## 原文档事实与 Library 投影

源码、计算接纳和文件保存仍由原文档数据库的 `outbox`、`kernel_outbox`、`save_outbox` 和原回执决定。`DocumentOutbox` 逐条通过原 owner 查询及结构、散列、操作/文档/事件身份验证，再生成私有 `DocumentEffect`。不能拿一个自称成功的 JSON 写进 Library；投影入口在 Library 写入之前再次读取原文档事实。

`LibraryStore` 的不可变 `document_effect_events` 使用 `(source_store_id, event_id)` 去重。相同事件返回原序列；身份相同而内容不同报错。来源 generation 保留作证据，稳定散列不含 generation，完整代次升级之后仍可保持事件身份；实际代次升级验收属于 R4.1.13，尚未实现。

先提交 Library、执行真实同步并从独立读连接确认，之后才能将原文档 `projected` 标记为 1。两库没有共同事务；中断后重复投影仍按原事件身份核对。恢复不只扫描 `projected=0`，完整重新核对可以补齐丢失的投影。该标记不是数学成功或用户文件保存的替代证明。

保存投影只有操作/文档、源码修订/散列、binding 修订、文件散列、恢复标志与完成时间。原 `SaveReceipt` 的路径和 bookmark 留在文档私有库，不进入 Library 事件或未来模型历史。

`RecoveryService` 仅持有存储服务，没有 evaluator、FFI host、HTTP 客户端、凭据或工具执行器。每页最多16条、原载荷总量最多4 MiB；每批默认256条，最高4096条。批次未完成返回绑定文档/store/generation 的私有游标，接着读取而不是一直重复第一页。Library 历史每页校验完整记录，不把坏记录当作可裁掉的流式尾部。

## 格式与故障隔离

新 Library 格式为2，store/minimum_reader/codec/user_version 一致；初始化完成投影表后才发布 selector。已选择的格式2缺表时要求恢复，不重建空表。已有 Native Library1继续可读，不在普通打开时自动升级；升级与备份由 R4.1.13 提供。Root仍1，新文档为5（增加草稿表，见下节），已有文档1–4不自动升级；`.omnb` v1、旧 Tauri 数据和 iOS ABI 不变。

## 原生草稿耐久备份

新文档5在selector发布前建立草稿 owner、batch和head表，读取已选择的5若缺表则拒绝。每个运行实例取得持久化递增lease；新实例出现后旧owner不能写入或重新取得原lease。snapshot是不可变原始UTF8字节、基线、native draft sequence、UTF16选区/marked range和来源状态；保存不解析、不计算、不改变source revision。选区拒绝拆开UTF16代理对，NFC/NFD单元格仍按字节区分。

MainActor只冻结实际TextKit输入，序列化/散列和SQLite提交在后台。500ms去抖、连续编辑最多2s发起一次备份；真实IO完成才显示已本地备份。旧回执仅确认捕获的persistence revision，新输入仍未备份；新编辑/确认源码/选区变化会更新这个计数。手工保存和关闭前主动flush，发生失败保留输入，unknown按原owner/sequence和冻结散列核对。只有确定原记录不存在时才用原ID补写该同一snapshot；没有新ID重放。

最近两份完整快照保留，较早载荷裁减但回执/请求散列保留，不把旧ID当新写入。历史裁减保留活跃草稿引用的source baseline。每份原始文本/基线合计最多4MiB，编码最多8MiB；实际大于4MiB的编码已往返，SQL默认读取上限没有被全局放大，只有草稿显式使用8MiB。

恢复使用新运行实例和editor generation。已被确认源码包含的旧草稿不重做；其余原始文本需要用户确认，基线失配保留冲突，删除格的草稿作为独立记录保留。恢复不调用setMarkedText，因此没有伪造IME session。用户确认保留/使用已确认内容后才允许同步。DraftStore与真实TextKit恢复、应用启动入口和已接纳计算prefix接入均已验证。

关闭保护先备份，再允许保留本地修改；关闭批准后的新输入仍会撤销批准。备份失败不把文档关掉，原生UI将本地备份状态与用户文件保存状态分别显示。正常关闭最终flush最新快照之后才停止producer，失败不提前停掉备份owner。

严格 `StorageService.open` 保留原拒绝行为。原生工作区的 `openWorkspace` 在取得正确通道的根锁之后单独打开 Library；Library 损坏/不支持时保留原件，返回明确不可用状态，健康文档与文件保存仍可使用。根格式、路径和锁错误不走此降级。UI 显示独立警告，保存成功不会把它清掉，也不声称助手已经就绪。

启动先通过只读连接检查格式、selector 和有界 Library 记录前缀，随后才启用 writer。损坏的完整记录拒绝，不能用空会话覆盖原库。实际测试证明 Library 完整记录损坏后字节保留，原源码和已接纳检查点可读取，新独立文档也能另存为。

## 没有 WAL 文件时的只读检查

本机 SDK27 的实际 VFS 以 readonly/no-follow 打开已被清理的 `-wal`，返回 ENOENT，导致只读 `PRAGMA user_version` 失败。日志保留了真实 open flags、文件名及系统错误，不修改原数据库来假装重启成功。

SQLite 的 `immutable` URI 读取会跳过锁与变更检测，要求文件在读取期间不变。[SQLite URI 文档](https://www.sqlite.org/uri.html) 因此本实现只在根租约/所属串行队列保证短期不写入、WAL及回滚日志都不存在时使用此视图；每次查询前后核对 main 文件的 device/inode/长度/mtime/ctime及日志仍不存在。任一变化要求恢复，不能发布该查询结果。存在 WAL 或 journal 时仍使用正常 SQLite 读取，不忽略任何日志。该选择依据官方行为和本机实际失败推导，保证来自我们的所有权与文件守卫。

上述无日志只读视图不创建侧文件，不修改未来格式的原件。守卫的独立物理测试改动文件时间后必须拒绝；测试中的独立 header 副本仅用于该守卫，不能作为备份或恢复输入。正式备份仍必须使用计划中的 SQLite Backup API。

## 已运行与剩余验收

- 实际 Rust/Swift/TextKit/SQLite 生成 `2、3、5、6`，保留初始化检查点及四个计算检查点；源码1次、kernel5次、保存1次共7个事实。
- Library 回滚、丢失 ACK、真实 F_FULLFSYNC 未知、按原身份核对、有界分页及同名事件跨文档隔离。
- 伪造保存投影在进入 Library 前拒绝；保存授权/路径不进入投影。
- 两库提交/确认之间五个真实 SIGKILL 点，重启和再次核对不增加原事件，源码、检查点和文件散列不变。
- Library 记录损坏时只读拒绝和工作区降级，健康文档读取与保存保持；原库不被清空。
- 原存储、保存、206事务/200+pin历史、检查点和编辑屏障回归；SDK27 Release 链接。详见本批机器证据。

实际TextKit标记文本、500ms/2s生产者、晚回执、新lease、真实同步unknown、大快照、跨重启及三个SIGKILL点已验证。规范Agent会话/admission/发送未知、备份/迁移/GC及最终同候选门禁继续后续任务，不能由这些本地恢复测试代替。

## 启动扫描与用户恢复

`StartupDocuments` 只读取已锁定的当前通道根下有效UUID目录和active selector；不扫描用户文件、解析bookmark、猜未选代次或执行源码。每页最多32文档，目录上限10000；校验header/source/draft状态，损坏与尚未升级的数据分别保留不可恢复提示。格式1–4不在普通恢复中自动升级，格式5才接入当前完整端口。

启动显示原生恢复列表，用户选择之后才建立独立新runtime。`NativeDocumentSession.recover` 读取原source head、settings、草稿和文件binding，保留document/store身份，分配新draft lease。恢复的非合成文本/IME文本均先呈现待确认；基线相同且已经被源码接纳的草稿不重复提交。已删除格的草稿可查看复制，不能无声消失。

系统关闭选择“保留本地修改”后，实际退出/重启/恢复已验证。恢复期间原源码0、零计算检查点不变；明确“使用恢复草稿”并保存以后才产生源码1和实际文件回执。尚未确认草稿不会被自动保存提交；恢复提示在确认全部草稿后清除。

## 已接纳计算状态的导入

新增可信host内部`kernel_restore_start/write/finish`，现有2MiB控制请求上限和旧iOS ABI保持。分块每次至多65536字节，检查连续offset与原ID重复块，拒绝不同块/空洞/越界。整体包上限为16MiB metadata＋64MiB Blob＋9字节头，不是模型工具或新的计算授权。格式OMKR1包括原plan/receipt和原OMKS字节，plan/receipt从原KernelStore完整核对得到，原Blob再次验证散列。

Rust导入线程在控制锁外验证整体散列、metadata、原回执、document/store、源码修订关系、精确kernel build、producer数学配置和数据codec。不会执行let或单元格。成功后才把原accepted checkpoint接入当前worker，并更新只读结果/源码函数上下文；kernel state revision保持原值，数据库没有新接纳操作。配置修订不符时definitions_current保持false，不能把旧定义标当前。

恢复期间共享文档逻辑门保护状态接入。直接取消触达导入worker；上传取消/后台失败释放逻辑门，Swift在返回编辑之前核对原终止状态。晚停止不覆写已经完成的原导入。原UI/runtime/preview/取消令牌不会作为新权限恢复。

五个实际中断根均证明：导入不增加source/accepted checkpoint/outbox；随后用户明确只运行b，会使用a=5得到6，并只增加该次计算。损坏原Blob拒绝，原源码和历史接纳回执保留，不创建“恢复成功”的空状态。当前计算状态不可恢复时可另存为、打开为独立新文档重新运行；在原文档显式重置不支持的计算head仍需后续工作台/重置接口，当前不宣称已支持。

## 保存意图与历史核对

恢复读取至多32个原prepared save intent。已存在原receipt直接核对；无receipt则由已有文件协调读取核对实际字节/身份和被交换原件，补真实recovered receipt，或确认未写入；没有再次replace。权限、离线、冲突或未知保持原operation ID待核对，阻止依赖的新保存，不伪造失败/成功。

六个保存SIGKILL点改由实际NativeDocumentSession恢复工厂验收，源码修订1保持，用户文件只读核对后保存状态与真实回执一致，零计算。Library就绪时恢复工厂每批投影256个原事件，未完保留cursor并提供继续核对；Library失败不毁原文档。Agent工具admission/网络发送的规范账本已见下一节；长会话与应用恢复入口见末节；R4.1.12底层恢复Done已满足，完整Agent仍待后续任务。

## 规范会话、工具接纳与发送恢复

新freshLibrary3增加不可变`session_events`、会话身份、来源序号和全库operation claim。既有Library1/2不在普通打开时升级，原document5/.omnb v1不变。NativeSessionEvent严格保留原设计的稳定session/event/turn/task ID、source instance/sequence、required-null、载荷散列及原operation/store；实际60条跨中断事件逐条通过原闭合schema校验，没有为了通过测试重写schema。

同一事件ID/载荷返回原回执，身份相同内容不同拒绝；source序号缺口拒绝，来源序号与事件顺序独立。用户/流式文字、完整assistant结束、tool admitted/started/result和request phase都先在同一Library事务提交，实际同步/独立回读以后才ACK；unknown只按原ID核对，新事件不能抢占未确认的序号。流批次最多32KiB且不含工具参数，工具只来自完整assistant结束。

`SessionToolPort`取得实际document/store身份、耐久admission和唯一execution claim后才调用宿主执行器。丢ACK或已有claim不调用第二次；跨会话引用同一个未核对operation也被挡住。已有原文档效果时只读核对/补结果，不能重做。`RecoveryService.recoverSession`没有执行器/网络callback：未admit显示tool_not_started；admit无真实终止事实保持tool_outcome_unknown；有原source/kernel/save效果则按store/op/frozen hash验证、投影真实recovered result。

source使用原冻结patch request hash，save使用目标文件散列。Kernel candidate request hash包含计算产物，执行前尚不存在，因此kernel admission绑定当时的source snapshot hash；恢复仍读原operation receipt及真实accepted state。实际Wolfram复合表达式报错但前面partial=7保留的例子，经只读内核检查和真实接受回执确认后标partial，不能宣称全部成功或回滚。

请求body作为原Blob保存，凭据/header由后续SecretStore/URLSession适配器另行提供。plan和资源引用同库提交，然后dispatch_started耐久确认，才调用真实传输。response字节作为另一个原Blob保存并记录response_started；终止记录不等于供应商已读完全部输入。恢复保持not_dispatched/dispatch_outcome_unknown/response_observed/finished/failed，不自动发送。HTTP503返回实际错误字节并标failed；丢响应或确认保持unknown，不猜测没有收费。

13个实际SIGKILL点覆盖admit前/后、execution claim、真实source COMMIT、tool result、request plan、dispatch、HTTP回复及终止ACK。回环服务器实际POST数在两次重启核对后不增加，source操作数也不增加；未开始和unknown状态保持。记录损坏不截尾吞掉，原Library字节和健康文档保留；伪造结果/错误hash/不完整工具参数/序号缺口均被拒绝。真实F_FULLFSYNC失败直到原ID确认才解除unknown。

上述Library3的4096事件/16MiB保护属于先前版本。当前freshLibrary4使用同库持久索引和续页恢复，已解除该接纳上限，原事件不截断。应用同时提供本地文档与会话核对入口；完整prompt/context revisions、压缩和provider/Pi适配仍待对应任务，不能把恢复完成当作完整Agent已可使用。

Canonical Timestamp使用现有date-time contract。本地无第三方日期/regex依赖的结构校验遵循[RFC3339日期时间结构](https://www.rfc-editor.org/rfc/rfc3339#section-5.6)，实际UTC时间、闰年/错误日历/偏移有独立正负例；不引入时区数据库或改写原schema。最终同候选门禁仍须另跑。


## 长会话索引与原生会话入口

freshLibrary4增加event links、tool/request索引，索引与原事件在同一事务写入并耐久确认。旧Library1–3不在普通打开中升级。所有查找重新核对原事件hash、身份/sequence/父子关系与resource；事件/索引数量、proposal-call数量和request-plan数量一致性检查防止缺指针伪装“没有操作”。实际删除索引行的测试在只读启动阶段拒绝，原件和健康文档保持。

新admission只取相关proposal/父子事件，不能因历史长而卡在旧4096条全量扫描。恢复的tool和request列表分别分页（最多32项），游标绑定session/store/generation及创建序号上界；已确认结果仍来自原事件，跨页不重复/遗漏，新加入的请求留待下一次刷新。4,374条真实事件、34,897,624字节原文，以及37工具/37请求计划，均在每页5项下完整恢复；首批原始记录与重启后新接纳仍可读取，没有截尾。

NativeSessionRecoveryList使用SwiftUI List/HSplitView，显示实际未接纳/unknown/completed/partial与planned/dispatch unknown/response observed/finished/failed。查看与核对只查原操作，未打开的managed文档仅在active selector存在时临时打开，不创建猜测目标；核对之后保持新请求未发送。历史不自动变成新任务授权，面板没有执行/重发按钮。

独立UI fixture复用该生产组件和NativeFileWorkspace/StorageService，仅把已生成的自有测试根注入fixture实例；正常产品不能通过启动参数切通道/数据根。实际点击显示call-edit“尚未接纳，未执行”和context-crash“尚未发送”，两条原记录及零源码效果保持。既有Preview/public数据不参与fixture。

R4.1.12的恢复账本、草稿/source/checkpoint/save核对和原生查询入口Done已满足；Pi循环/模型适配器/完整上下文、备份/升级/GC和发行门禁均继续各自后续任务。没有把历史页当作已恢复执行授权，也没有将基于真实状态的说明升级为“新请求已发”。

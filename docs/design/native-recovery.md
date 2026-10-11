# 原生 Mac 启动恢复实现

2026-10-11。R4.1.12 正在实施，已完成文档事实投影、Library 故障隔离和原生草稿耐久备份。完整启动扫描/工作区恢复、Agent admission/请求恢复仍未完成；运行版本继续 `.3`，不代表 `.4` 已就绪。

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

恢复使用新运行实例和editor generation。已被确认源码包含的旧草稿不重做；其余原始文本需要用户确认，基线失配保留冲突，删除格的草稿作为独立记录保留。恢复不调用setMarkedText，因此没有伪造IME session。用户确认保留/使用已确认内容后才允许同步。当前已验证DraftStore与真实TextKit恢复，完整应用启动入口和计算prefix接入仍待完成。

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

实际TextKit标记文本、500ms/2s生产者、晚回执、新lease、真实同步unknown、大快照、跨重启及三个SIGKILL点已验证。完整启动文档扫描与用户选择、已接纳计算prefix/未接纳工作状态/原save intent核对、规范Agent会话/admission/发送未知、备份/迁移/GC及最终同候选门禁继续后续任务，不能由这些底层测试代替。

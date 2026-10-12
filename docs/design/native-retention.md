# 原生存储维护实现

R4.1.13 的物理实现；规格仍以 [pre-alpha.4 计划](../plan/PRE_ALPHA_4.md#task-r4.1.13) 为准。原生工作台的数据管理界面属于 R4.2.18，最终运行及发行门禁须在同一候选上另验。

## 维护屏障和引用

`StorageService` 对所有公开异步存储操作计数。备份、升级、恢复、备份保留及 GC 关闭新接纳，等待已接纳操作结束，再在后台队列扫描。关闭同样等待在途操作，先直接取消 Blob IO；缓存、SQLite、Blob 及根租约的所有者随后释放。维护期间返回 `transactionUnavailable`，调用者不得将其解释为已经执行或换一个 operation ID 重放。

`StoreCatalog` 验证 Library 和全部文档的 selector、store ID、代次、版本、外键、必需表及引用。维护检查完整会话事件与原回执的散列、序号、索引和资源关系，逐条检查源码快照及已有载荷散列。GC 还扫描所有保留的数字代次；出现未解决的迁移临时代次、损坏/未来格式或缺引用时停止。它不将打不开的库解释成空库。

模型可用的 result/snapshot/preview token 仍是当前运行实例的 HMAC 能力。TTL、任务、权限、文档及运行代次必须核对，撤销立即释放重载荷。重启或过期后，以原已接纳结果 ID 和原检查点重新核对实际结果并签发新 token，不恢复旧 token，也不分配新耐久 operation ID。专项 Rust 测试使用真实 CAS 检查点验证期限边界、重新签发、拒绝旧 issuer、无读权限及撤销；源码幂等 tombstone 仍由原文档库保留。

## 自包含备份

`SQLiteDatabase.backup` 通过 [SQLite Backup API](https://sqlite.org/backup.html) 读取包含 WAL 的已提交视图。两个连接留在同一后台队列；每批 128 页，任何非 OK/DONE 状态失败，不无限重试。目标完整性及外键检查成功后转为无 WAL 的独立数据库，实际 F_FULLFSYNC、关闭目标连接并同步目录。

`BackupService` 先把所有当前选中的库备份到 `Backups/.pending-<UUID>/Stores/<store_id>/`，核对原 header、表计数、引用及独立文件 SHA-256。Blob 队列持有已校验文件和临时 pin，分块复制原字节到备份自己的 `Blobs/`，再次核对全部 SHA/长度。完成 manifest 和目录同步后，才原子发布 `Backups/<UUID>`。备份不是对正在写的 `.sqlite` 的普通文件复制，也不依赖原 Blob 目录作为唯一副本。

`manifest.json` 使用已冻结 Schema 的完整 `BackupManifest` 字段：record_type、backup_id、format_version、created_at、stores、blob_hashes、manifest_hash、verified。它的 manifest_hash 绑定排序编码的 `inventory.json` 实际字节；后者记录 installation/channel、稳定 store identity、每个数据库 SHA/长度/表计数、引用清单、Blob SHA/长度及 manual/beforeUpgrade 用途。规范记录与物理清单都必须核对，未验证标记、未知字段和关联散列不一致被拒绝。实际发布的规范记录还由原 Rust Schema 校验器独立检查。发布后丢失确认仍保留实际发布的健康备份；此前的中断目录是未发布临时资源。备份不添加运行 token 或任何凭据。

保留维护先校验全部已发布备份，保留最近七份普通备份、最近两份升级前备份及至少一份未隔离的完整备份。1 GiB 是软预算，不能以配额为由牺牲上述保留项；超过预算返回实际状态供后续管理界面显示。待删除备份若有未知新增文件则保留并停止删除；Apple VFS 留下的已知 SHM 缓存不作为独立权威或恢复输入，WAL/journal 不在可删除的备份文件集合中。日末触发及 UI 属于原生应用的数据管理接线，不以基础存储测试冒充该界面已交付。

## 升级和备份回退

普通打开仍只选择已验证 selector，不偷偷迁移。显式 `upgradeStore` 在维护屏障内先取得升级前全库备份，再从 SQLite 快照构建新的临时代次。Library 1–3 升至 4；文档 1–4 升至 5。文档 1 的历史摘要从原 forward/inverse 计划建立；Library 3 的恢复索引从原完整事件建立。每个原表的完整 typed-row 散列和行数必须相同，原 receipt、operation ID、source/kernel/file/事件载荷不改写。

新库经外键、完整性、原表/引用/业务检查后关闭所有目标连接，同步和发布 generation 目录，再原子切换 `active.json`。原代次保留且不双写。旧会话分页游标随代次变化失效；文档投影的稳定身份不包含 generation，所以迁移不产生第二份工具效果。

`restoreBackup` 重新验证自包含备份，恢复所需原 Blob 字节，然后作为同一 store ID 的新 generation 发布。每次回退都持久标记 `rollback_quarantine`。普通打开与升级均先拒绝隔离库；`quarantinedSource` 独立核对当前文档的 selector/header，只读原源码，即使 Library 损坏也可以查看，供后续工作台查看/另存为新的文档，不取得写入连接、执行器或网络回调。保留旧代次不能被用来声称已恢复的旧备份含有其后的完整幂等历史。

## 配额、缓存和 GC

实际可用空间由本地 statfs 取得。SQLite 事务及元数据发布前保留 256 MiB；大附件/大资源写入前保留 1 GiB，写入仍逐步处理实际 I/O 错误。软总预算为 5 GiB，容量查询在后台返回实际字节和可用空间，不通过删除原附件解除预算。

派生资源单独位于 `DerivedCache`，不进入原 Blob 的缓存清理路径。完整性检查、原子索引发布、LRU 和默认 512 MiB 上限同属后台串行队列；运行 producer/reader 的 cache pin 优先于配额，不能压入预算时返回 overBudget。重启不复活临时 pin。缓存 ID 的不同字节被拒绝，清缓存不能删除原附件、检查点或用户导出文件。

GC 先获得全库/保留代次引用及健康备份 pin，核对所有原引用的实际文件，再在 Blob 队列核对 publication/reader pin。未引用资源必须超过 24 h，先移入隔离区并同步，最终再次确认引用/pin后才删除。强杀后隔离中的孤儿按原散列继续核对；新引用所需的隔离文件先恢复至 canonical 路径。已排空上传的旧 staging 仅按应用自己的 blob-UUID 命名和 24 h 期限处理，不扫描用户目录或其他应用。库/索引/格式/引用/原字节异常时停止原资源回收。

## 验证入口

- `bash macos/Scripts/test-maintenance.sh`：真实 WAL 备份、未打开文档引用、原字节、pin/reader、24 h、维护与上传并发、SQLite 写入 ENOSPC、确认丢失、LRU、备份保留、Library 3 和文档 1 迁移、回退后缺失新操作的隔离、跨库未来格式停止 GC。
- 同脚本的 Python 驱动：17 个真实 SIGKILL 点，覆盖备份 API/发布、generation/selector 切换、GC 隔离/删除及重启续查；保存命令、强杀返回值、selector 状态和原文档 SHA。
- 存储、源码、Blob、历史、计算接纳、保存、投影、草稿、规范会话和原生 ABI 的既有测试继续独立运行。开发证据不能替代 final candidate 门禁。

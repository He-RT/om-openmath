# 原生源码历史、裁减回执与文本撤销分组

R4.1.07 的实际接口与验收；完整版本目标仍见 [`.4`主计划](../plan/PRE_ALPHA_4.md)。本节不表示完整笔记本工作台或 Agent 已接通。

## 物理版本和兼容性

Root/selector 格式和 Library 库仍为1；新建文档库为 `store_version=minimum_reader_version=codec_version=2`、SQLite `user_version=2`。新读者同时支持已有 NativeMacPreview 文档1的完整记录；文档1没有原地裁减或自动升级，保留完整计划。旧读者在只读检查Header时拒绝文档2，不能猜为空库或写入未知载荷。`.omnb`v1、内核source snapshot/commit协议及旧Tauri目录不变。

文档2新增 `source_history`：同笔源码提交存真实事务身份、修订、不可变 `NativeSourceTransactionTombstone` 和其实际字节SHA256。元数据保留 DocumentCommit、可选输入/撤销group、计算设置变化、原forward/inverse载荷散列；它没有源码文件，不能被解析成可执行的反向计划。

## 保留和裁减

`sourceHistory`读取最近200项，`pinSourceTransaction`由可信宿主固定 user/task/native_undo 记录；固定项超出窗口仍保留。最新真实计算设置另有宿主系统固定项，防止旧设置被裁减后恢复为默认值。固定已裁减项失败，不伪造已消失的源码。

`compactSourceHistory`在所属数据库writer执行，最多裁减256项。先核验每个原回执与完整不可变图，以及原forward/inverse字节散列；全部成立后，一笔SQLite事务清除旧完整计划、inverse snapshot和已完成admission中的重复源码，保留其同源元数据。只有不被当前head或保留事务before/after引用的历史revision才清除源码载荷；身份/散列/FK行仍保留。COMMIT后仍等待真实F_FULLFSYNC。回滚、确认未知、损坏分别报告。

原 `operations/operation_transitions/outbox` 及原回执字节不修改。裁减后的查询同时核对元数据SHA、原请求hash、事务与修订、source/inverse hash、epoch、outbox payload和receipt hash；破损join或元数据不被当作“从未执行”。`sourceTransactions`要求完整反向载荷，裁减后明确 `transactionUnavailable`；`sourceReceipt/sourceUndoMetadata`仍可核对原ID和撤销组。同ID重试不会产生新源码提交。

维护由宿主明确安排在后台writer，不在MainActor等待，也不与单次source COMMIT混为一个成功状态。物理空载荷可由SQLite重用页面；不承诺文件长度在每次裁减后立刻缩小。完整quota/备份/GC调度仍按R4.1.13实施。

## 原生文本分组

NSTextView真实输入命令在其本地UndoManager中登记未确认草稿。路由器依据当前草稿/合成状态选择本地撤销或已确认文档UndoManager，输入命令和本地redo登记使用实际原生响应链，不手写Unicode文本引擎。确认回显只在无更晚编辑/合成时清除已确认的本地动作；它不登记新的编辑。

连续输入用实际caret和单调时钟分组（600ms闲置或caret变化开启新组），IME合成跨更新保持同一组。`EditorCommitBinding.flush`在实际IO前冻结组ID，取得真实source receipt后才登记文档UndoManager。可选 `input_group_id`纳入Rust/Swift request hash，同笔计划和裁减元数据中持久化；模型无权提供该宿主参数。读取、运行、保存与修改入口应使用该绑定的草稿同步屏障。长文虚拟化、完整工作台和全部快捷键继续在R4.2接入。

DocCommitPort跟踪在途调用和准备中的已知ID；停止在原事务读取期间到达也会在实际写入前生效。关闭先取消并等待这些调用，再消费host/storage，不以“还未进入SQL COMMIT”认定没有资源等待。

## 真实验证

- `bash macos/Scripts/test-history.sh`：206个实际Rust计划写入SQLite；200近期+user/task固定项，裁减回滚/失联，实际空载荷和稳定事实计数、重启、取消固定、原ID无重放，以及破损元数据拒绝。
- `bash macos/Scripts/test-undo.sh`：真实C ABI/SQLite/UndoManager、事务组、原ID/错组、later input冲突、unknown核对、emoji本地与耐久undo/redo、IME同组、输入group持久化及早准备停止/关闭。
- `bash macos/Scripts/test-document-store.sh`与`test-storage.sh`：原源码/设置/失联恢复、真实系统刷新、ENOSPC、根锁及初始化故障继续验收。

测试使用仓库内自有临时数据库和合成输入，不启动iOS模拟器，不读旧Tauri或供应商秘密。开发证据与最终同候选门禁仍独立。

# 原生 Mac 文件保存实现

2026-10-11，R4.1.11 实施中。本文记录已运行的文件端口与剩余界面验收；不代表 `.4` 可发布。

## 所有权与格式

- `NativeDocument` 管理文件面板、编辑标记和冻结字节；`DraftStore` 管理实际 NSTextView 的草稿与 IME。Rust source endpoint 保持唯一源码权威。
- `NativeDocumentSession` 从真实 `.omnb` 文件建立独立文档、host 和提交端口。打开不执行源码、不恢复定义或输出。普通文件仍为 `.omnb` v1，仅含 version/title/cells。
- `NativeAppStorage` 在初始化前保留任务，多个文档会话共享一份根目录锁和物理存储服务；关闭文档先完成文件和源码端口，再释放 host 与该文档 writer。隔离试算不另抢持久化根锁。
- 新建 Native 文档库格式为4（store/minimum_reader/codec/user_version）；Root 和 Library 仍为1。新读者支持已有 Native 文档1–3，不自动升级，也不读取旧 Tauri 配置或用户凭据。格式3继续提供原检查点端口；文件保存表只在格式4启用。

## 保存算法

1. 将可确认草稿提交到原 source endpoint。marked text 保留；语法未完成或错误的数学源码仍是可保存的笔记本数据。真实原生输入组在持久化后登记文档 UndoManager。
2. 从实际源码权威读取 revision/hash，冻结 `SaveSnapshot` 的 `.omnb` 字节及 SHA256。后续 writer 不再读可变编辑器。
3. 发布不可变 Blob，再将 `SaveIntent`、原目标指纹、binding revision、源码引用和 Blob 引用提交到同一文档 SQLite 库并确认同步。
4. 后台 NSFileCoordinator 协调目标，检查普通文件、可写性、原 inode/时间/长度/hash。写同目录的 O_EXCL 临时文件，完整写入并执行 F_FULLFSYNC，再检查原目标。
5. 新文件使用 RENAME_EXCL；已有文件使用 RENAME_SWAP，保留被交换的原件并核对其原 hash/inode。原件不匹配时保留文件并报告冲突，不清理外部原件。
6. 同步目录，读取目标全部字节并验证，再同步实际文件和目录。只有这一步产生实际 `SaveReceipt`。
7. 同库提交 receipt、save outbox、绑定指纹和 saved revision，独立读取原回执并确认提交字节。只有当前 binding revision 对应的回执推进当前文件标记。

用户文件与 SQLite 是两个资源，没有宣称跨文件系统原子事务。保存 revision N 期间出现 N+1，仍保留未保存状态；原目标的迟到回执不确认另存为的新目标。源码历史裁减保留当前 saved revision 与尚未完成的保存意图；完成的历史保存不额外永久固定源码载荷，原修订身份和回执保留。

## 未知结果与失败

- 替换前明确的权限、非普通文件、目标变更等错误保留原文件和编辑内容。
- 替换之后的同步或回执确认失败可能已经产生文件效果，按原 operation ID 标记未知，禁止另起 ID 重写。
- 核对优先读取原 SQLite receipt；没有回执时仅协调和读取原授权目标，核对实际字节、遗留的被交换原件并执行同步，再记录恢复回执。没有重新执行数学或再次替换文件。
- 原目标仍是旧指纹时返回“尚未写入”；第三方字节、离线、权限、损坏或无法确认同步均不会当成保存成功。
- 保存目标授权使用独立后台队列，避免原目标 writer 暂停时阻塞另一目标的选择；真正文件写入仍串行。
- 路径和 security-scoped bookmark 只保存在原生私有存储，不进 `.omnb` 或模型工具参数。

## 自动保存和关闭

已绑定文件空闲2秒保存，持续编辑最多10秒触发；保存中合并后续编辑，错误/冲突/未知暂停。只剩 IME 合成时不重复保存同一已确认源码。未命名文件不自动选择用户路径；其完整草稿恢复仍由 R4.1.12 完成。

AppKit 默认的 `scheduleAutosaving` 不再启动另一个 opaque autosave-elsewhere writer。宿主文件动作复用同一端口；后台原生关闭先检查真实 dirty/未知状态。关闭保护不强制结束 IME；确认关闭之后的新输入会撤销该决定，继续保留编辑器。

当前原生工作台为文件编辑基础路径；完整结果展示和交互属于 R4.2。实际测试已验证新建、NSTextView 输入和关闭保护，但系统保存面板的 Save/New Folder 按钮异常禁用尚未解决。增加文档类型声明、前台激活和菜单可用性实验均不足以证明解决；菜单实验已撤回。不能以底层文件测试通过代替面板验收，R4.1.11 仍未勾选。

## 实际验收

`bash macos/Scripts/test-saves.sh` 直接运行 Release C ABI、Swift ports、系统 SQLite、实际文件和 NSTextView；测试同步探针只注入独立 helper，不链接到 App。

- 非法数学源码保存，Unicode/emoji/NFD 原字节，打开后 kernel 未执行。
- 保存 N 期间 N+1、另存为旧 binding 回执、外部修改、只读拒绝。
- 替换后丢失 ACK、真实 F_FULLFSYNC 失败、原 ID 核对及重启回读。
- 实际 TextKit 输入、2秒真实文件自动保存、UndoManager、后续输入撤销关闭决定。
- 六个真实 SIGKILL 位置：intent committed、临时文件已同步、文件已替换、目录已同步、receipt COMMIT 前、receipt COMMIT 后。
- 额外的交换原件冲突：重启保留不匹配的外部原件，不写假 receipt。

SIGKILL 是进程中断验证，不冒充断电实测。尚待系统打开/保存/另存为面板完整路径、移动/离线目标交互和完整工作台生命周期验收；最终同候选32项发行门禁独立执行。

# 原生 Mac 开发

原生客户端正在按 [`.4`完整计划](../docs/plan/PRE_ALPHA_4.md)实施，当前运行源码版本仍`.3`。本目录不是已发行Mac包；公开安装和旧数据保持独立。

## 当前可用

- 可复现的Xcode27/macOS27 ARM64工程和SwiftUI/AppKit开发窗口。
- 独立`om-apple-ffi`静态库，实际隔离计算、内核元数据、后台事件泵与原 ID 状态读回；原生笔记本工作台和完整计算 checkpoint 继续按计划接入。
- 固定Pi/Node/Swift依赖及原许可审计；此阶段未将Pi helper接入或嵌入应用。

## 构建

在仓库根目录执行：

```bash
bash macos/Scripts/verify-env.sh
python3 macos/Scripts/generate-project.py
python3 macos/Scripts/generate-project.py --check
bash macos/Scripts/build-native.sh
```

产物为`target/macos-native/Build/Products/Release/OpenMath Preview.app`。Preview独立Bundle ID为`org.openmath.OpenMath.NativeMacPreview`，只有本地ad-hoc签名；当前不注册`.omnb`关联、不初始化旧Tauri数据或主动连接模型。Xcode工程可直接打开，不需要第三方工程生成器。

工程将库定位到仓库`target/release/libom_apple_ffi.a`，脚本先构建真实Rust库再构建Swift；没有装系统Node或依赖本机Homebrew dylib。最终用户包的随包Node、签名公证、文档/模型/媒体功能以实施任务和真实包验收为准。

关于原生框架、主Actor、文档/计算所有权和后续存储边界，见[宿主契约](../docs/design/macos-host-state.md)。窗口采用实际可用宽高/系统滚动、缺业务能力明确显示，不用动画或mock文字制造成功；参考telegram-ui-reference的屏幕适配和进度/回执基础路径。

原生宿主调度和 C ABI 已接通真实隔离计算、Editor Preview 与 Desktop 能力查询；文档持久化/写入/checkpoint 与完整工作台仍按 `.4` 计划接入。底层 `get_function_catalog` 支持 `offset`/`limit` 分页（默认 0/32），回复携带 `total`/`next_offset`，全部条目来自实际内核回调。`native_renderers_ready=false` 表示当前链接预览还没有完整原生渲染器。

运行跨语言验收：

```sh
bash macos/Scripts/test-native-contracts.sh
```

该脚本直接由 Swift 调用新的 C ABI，验证精确 `2+2`、真实数学错误、Desktop 能力、直接取消和解码后释放。`create` 失败必须读取并释放错误 buffer；`submit` 回复只是入队回执。`close_begin` 只撤销入口和发出取消，`close_finish` 必须后台执行并等待活动调用/owner 停止。调用者不读/释放 opaque handle，不再使用已消费的旧 handle。为避免旧地址重新指向新会话，小 token tombstone 保留到进程结束，上限 4096 次创建/128 同时宿主；完整服务和结果内存均在 finish 释放。返回 buffer 恰好释放一次，地址可由后续 buffer 重用。

新增 `macos-native.yml` 在固定 Xcode 27/SDK27 镜像构建和执行这些验收。当前产物仍是独立 ad-hoc 开发预览，不能据此宣称发行签名、公证或完整 `.4` 已通过。

原生窗口现已接通“试算”，支持运行/停止与⌘↩；结果、错误和取消均来自实际CAS及宿主回执。编辑后保留旧结果并标记源码已改，隔离试算的定义不进入下一操作。这个阶段只实现宿主确认投影和本地草稿，完整笔记本/文件仍未开放。

`om_host_read_snapshot`直接读取带序列的真实状态，不经操作/事件队列；普通事件批携带`last_rust_event_sequence`。消费者检测缺口后取得足够新的快照，再恢复增量。Swift脚本新增reducer与真实session验收，将事件预算设为128字节以强制所有终止帧丢弃，确认结果通过原ID读回。旧/重复回执不能二次应用；UI订阅缓冲保存最新完整投影，操作终止交付独立。

SQLite首代存储已实现，可运行 `bash macos/Scripts/test-storage.sh`。应用支持目录由系统API取得，Preview写入独立NativeMacPreview通道；生产NativeMac通道未来随最终App启用。每库只有串行writer，root由OS锁保护；已有库但selector无效时要求恢复，不重建空库。该阶段只初始化物理库与header，完整笔记本事务/恢复与`.omnb`保存按后续任务接通。

实际SQLite3.54.0/unix VFS3使用WAL/FULL/foreign_keys/fullfsync。系统VFS的实际barrier刷新与宿主COMMIT之后等待的F_FULLFSYNC分别验收，后者失败表示提交确认未知。测试dyld/VFS探针仅在独立fixture helper，未链接进App。SIGKILL用例保存原残留，不以进程中断冒充断电实测。

运行 `bash macos/Scripts/test-document-store.sh` 可验证Rust权威source计划→Swift SQLite原子事务→Rust实际回执的完整数据路径。同库保存源码、反向快照、操作回执与outbox，重复原ID不重复写入；不同内容/旧修订/非法计划拒绝。测试包含中途回滚、提交后失联、真实全刷新失败及原ID恢复，确认UTF8/Unicode cell IDs不被归一化。

这一阶段已有真正source存储端口，以及可信原生内部使用的DocCommitPort与编辑屏障；完整笔记本UI/Agent尚在后续任务接入，不能直接调用物理writer绕过权限。模型可执行body暂未开放写入。较大source需要后续Blob协议，当前超限会明确拒绝。

不可变资源基础可运行 `bash macos/Scripts/test-blobs.sh` 验证。128MiB单件按64KiB复制/散列，完整同步后不覆盖发布；读取每页至多1MiB，并校验真实hash/length。数据库引用、内存publication pin和reader保留是不同事实；重启可从已存引用读回原字节，但不恢复旧pin授权。错误hash、损坏目标、输入变化和缺文件不会当ready，完整GC/媒体解码/工具scope仍按后续任务接入。

源码操作基础已支持在临时文档一次验证 insert/update/delete/move/rename，真实依赖/循环/定义冲突分析不求值。计算worker接纳后可一次应用source并清旧owner，保留历史标stale；title/Text和prose移位不清数学值。actual settings转移与source同库确认，locale不混为execution_epoch。冻结preview与原生fence已接通可信内部端口，UI/Agent修改入口继续按后续任务注册。

冻结预览基础可运行 `bash macos/Scripts/test-preview.sh`。source只检查，patch用原snapshot完整校验并分配新cell ID；invalid/source-only不会有可提交ref。真ref绑定runtime/source/permission/editor-state和期限，返回原不可变计划；actual SQLite端口回读由原Rust文档核验。普通parse成功不表示执行或文件保存。

运行 `bash macos/Scripts/test-commit-port.sh` 验证实际C ABI、MainActor上的NSTextView与SQLite提交链路。Swift负责原生草稿/合成状态和物理存储，Rust负责当前源、冻结计划、原ID和最终提交屏障；`.source_*`是封闭的宿主内部命令，不是模型可调用工具。手工草稿可以保留未完成语法，marked text不会自动提交。最终屏障只做短状态检查：解析owner忙时拒绝而不等待，SQLite及F_FULLFSYNC期间主线程仍可处理新输入和停止。

确认丢失时通过`unresolvedOperation()`取得原ID并`reconcile`读取实际存储。耐久admission、source COMMIT与文件保存是不同事实；同ID重复或已取消/失败不能恢复为新写入，COMMIT后的停止不伪称撤销。确认只推进对应草稿序列，较新的输入/IME继续保留；关闭须先等待DocCommitPort的在途IO，再关闭host和storage。已结束操作只保留有界小回执，完整原计划在SQLite中，旧临时scope资源及时释放；单host生命周期最多4096项操作，达到上限明确拒绝。完整撤销、重启恢复、计算接纳、文件与原生工作台仍待对应任务完成。

`bash macos/Scripts/test-undo.sh`使用实际C ABI与SQLite验证逆事务及原生UndoManager。`DocCommitPort.undo`先读并校验原事务，再在临时比较视图按最新到最旧合并1..32条逆操作，整体产生一个新修订；不倒退权威计数。相关内容/顺序被后来修改时整笔拒绝，无关后来源码保留。`undo_group`是原生内部source计划的可选扩展，绑定group_id与原事务集合到request hash；原无该字段的记录与`.omnb` v1保持兼容。

重复的undo operation ID先读回原回执，错误组或不同内容拒绝；重开runtime从实际head恢复后可核对旧undo，不重新执行。UndoCoordinator只有真实提交成功才消费原生undo命令并注册实际逆事务用于redo；同组回执合并、相同回显不重复登记，失败保留命令，未知结果锁住命令并按原ID核对。EditorCommitBinding已接通实际NSTextView草稿/IME/原生分组和确认后文档撤销；完整工作台与全部编辑器交互继续R4.2接入，没有注册Pi undo工具。

`bash macos/Scripts/test-history.sh`验证实际206笔源码事务、最近200+固定项、物理裁减、重启和原ID回读。新文档物理格式2，Root/Library仍1；已有Native文档1保持完整而不原地升级。裁减后反向源码不可用时明确拒绝，原receipt/outbox/操作事实仍可核对；旧格式和future版本只读核验，不重建空库。后台维护、quota和备份调度继续R4.1.13。具体数据和接口见[源码历史与文本撤销](../docs/design/source-history-retention.md)。

事件恢复固定发起snapshot时已接纳的ID集合，避免读回在接纳前生成、却在ACK后返回时被误判为unknown。真实ABI fixture将该时序确定性复现；修复保留原ID再读实际结果，不重放计算。普通生产调用不配置fixture barriers。

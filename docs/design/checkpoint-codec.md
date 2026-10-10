# 无损数学 checkpoint 编码基础

R4.1.08的无损基础包括数值原子、表达式图、Evaluator数学和整个Session/owners/实际输出记录codec。解码结果是owned候选；R4.1.09已有冻结接纳计划和真实Blob/SQLite同库接纳；实际NativeHost运行入口、共享逻辑门和活动指针恢复仍继续，不能以codec往返宣称已接通公开主会话恢复。

## 数值原子 OMNU v1

`om-num::checkpoint::{encode_number,decode_number}`只读写数据，不执行输入式。头为 `OMNU` 和版本byte1，tag分别为整数0、有理数1、机器实数2、二进制高精度3、复数4。

- 整数为长度界定的canonical有符号小端二进制；零是空payload。有理数保存该numerator及canonical无符号denominator，拒绝零分母、denominator-one和非约分输入。
- 机器数保存原`u64`位型；NaN/∞拒绝，`+0.0/-0.0`、subnormal及有限极值保留。
- 高精度保存原canonical二进制significand、`i64` exponent和`u32` bit precision；目标平台无法无损表示exponent时拒绝，不截断。零只接受原正/负零的exponent0/-1；特殊指数sentinel和非canonical偶数mantissa提前拒绝，避免库归一化overflow或产生非有限值。不用InputForm/十进制/机器数填充高精度。
- 复数只含两个有限scalar，类别/精度一致；不接受nested complex或可以折叠为普通实数的伪类别。

默认每atom4MiB、precision至2^20 bits。调用者可提供更严格预算，长度/版本/末尾字节/cancel均检查；超限给真实错误。数值往返包含4096个固定seed机器位型和真实高精度/精确复杂数。

## 表达式图 OMEX v1

`om-core::checkpoint::{encode_expressions,decode_expressions}`编码postorder节点图：`OMEX`+版本byte1、root count、node count、节点及最后的root引用，整数长度/引用为小端u32。节点tag分别为Number0、builtin name1、user symbol2、UTF8 string3、normal(head+ordered arguments)4。

encoder用存活immutable Arc的进程内身份识别共享节点，仅发出图内ordinal引用；身份/指针不进payload。不能用Expr数学相等去重，因为有符号零等语义相等对象可能有不同底层位型。normal保留原head（可以是任意表达式）、顺序和嵌套，不进行canonicalize或evaluate。

decoder先验证全部长度/节点、UTF8、builtin与user分类、backward references/无cycle、depth/edges、全部root及可达性，再intern名字和构造真实Expr。未知tag/版本、forward/cycle、不被root使用的孤立节点、坏引用或trailing data均拒绝。恢复过程中不读取文件、凭据、HTTP或闭包。

默认64MiB、200000 nodes、100000 roots、1000000 edges、depth1024、8192独立symbols、单text4MiB。多个root共享同一图，100层每层重复两次的实际共享子树保持线性大小，不展开为2^100树；budget/cancel由宿主Interrupt驱动，32bit长度加法也检查overflow。

## Evaluator 持续数学 OMES v1

`Evaluator::encode_persistent/decode_persistent`使用`OMES`+版本byte1、明确metadata/graph小端u32长度、封闭JSON元数据及一个共享OMEX图。ownvalues、原downvalue顺序和delayed标志、全部9位user attributes、changed symbols、successful history input/output、EvalSettings及SplitMix64原stream均保留；恢复不执行Set、延迟RHS、随机请求或历史源码。messages/当前statement solver/science证据属于暂态，真实已产出记录由Session层保存。

元数据绑定调用者精确build、crate版本、目录metadata_version和实际BuiltinTable的稳定ID/name/attribute/arity闭包。恢复只连接当前已注册的真实callbacks，不读回函数指针或闭包；build/目录/registry不符明确拒绝。各表按原UTF8 symbol name排序；未知字段/重复身份/坏索引、未使用root、未知attribute bits、packet trailing/length和已取消状态拒绝。readonly或in-flight lexical/frame状态不可捕获为writable owner。

默认packet64MiB/metadata4MiB、每表8192项、32768规则、10000历史对，数学图另满足OMEX预算。metadata先校验，再解码图并构造独立可写Evaluator；没有改动active Session/文档，也没有生成耐久receipt。已验证Root、高精度、插值、拟合/ODE实际存储值与后续可调用行为；重编码保持原bytes。OneIdentity bit8是既有合法属性，旧from_bits漏接纳的mask已修为9位，bit9仍拒绝。

## Session OMKS v1 与原始求解证据

`Session::encode_checkpoint/decode_checkpoint`保存真实Notebook source/status/defines/uses/exec_count、原CellOutput、StatementRecord的input/value/steps/solver/scientific/readonly Explore snapshot、Evaluator数学、owners、GeneralConfig、output serial/平台/系统语言。`OMKS`+版本byte1及4个小端u32长度界定metadata/evaluator/statement graph/readonly contexts；整个packet默认64MiB、metadata16MiB、source2MiB、10000 cells/records，context单件4MiB，所有子codec另满足各自预算。

CheckpointBinding来自可信producer及已核验的记录：document ID、原producer generation、source revision/epoch、kernel-state revision、source snapshot hash和精确build。CheckpointRestore输入该原记录和当前核验source/general，以及新clock/cancel；byte身份和实际当前source/config分别检查。原producer generation是历史生产事实，新的文档lifetime/允许主状态接纳由Host当前scope另行检查。token/文件store/LLM job/API key/模型连接不序列化。

步骤以扁平postorder节点保存27种原StepKind及所有真实表达式字段，保留rule ID、id、Level、children顺序；坏rule/kind组合、forward/cycle、复用/孤立step或层级ID不符拒绝。SolutionSet保留finite/all/region/unevaluated、条件、生成参数域、重数、原Verification、间隔端点及数值projection的原u64位型；不从LaTeX/截图或字段名推测精确验证。codec数据不创建数学证明，接纳依据真实原producer与实际byte/hash。

恢复后record与对应Evaluator历史对进行有界raw表示核对，solver/science尾值也核对；跨图比较memoize allocation-pairs，不能展开shared DAG，也不把±0或precision差异误为相同。unknown嵌套输入/配置/输出字段、坏owner/record/view/out reference、active Running状态、来源或预算/cancel不符均失败，不返回partial writable session。

只读Explore使用独立`OMRS1` role header及metadata readonly角色，单改magic不能提升为OMES；解码后仍readonly，不能fork working owner。表达式/controls/ranges/当前locale输出保留，后续slider按原只读context重新采样，不污染主定义/random。

Host KernelStatePool临时保留actual encoded bytes、SHA256、原binding/source/general。默认最多32/64MiB，计入revoked但被调用者Arc pin的reservation；撤销lookup不提前释放在途reader/候选内存预算。ID由host熵生成，猜测/错scope/错source/config/hash不授权恢复。临时pool不是已发布Blob/accepted checkpoint，future main owner须等真实IOAck才接纳。

## 后续接入和验收

暂态解释器frame不可捕获，只读fork不能提升，候选写入和真实耐久接纳继续有独立回执。`Session::fork_working_session`克隆实际文档/结果/owners及read-only Explore snapshot，新取消token与parent独立，LLM/config-store/credential IO不随candidate复制。codec本身不授予主文档权限；接下来主worker/candidate/Blob/accepted record与active state引用按R4.1.09–13接通。

新增 `om-num/tests/checkpoint.rs`、`om-core/tests/checkpoint.rs`、`om-eval/tests/state_codec.rs`、`om-solve/tests/evidence_codec.rs`、`om-kernel/tests/checkpoint.rs/checkpoint_corpus.rs`与Host pool测试。原53通过actual kernel计算→Session bytes→数据恢复→原结果/证据核对→byte recapture；原数学/200-digit参考残差不变。实际Root/80位decimal、fit/ODE/interpolation后续可调用、steps/readonly Explore、random/Out和延迟RHS不重跑均覆盖。默认开发用例不测原Release200ms/1s门禁，不将它们写成final candidate pass；本机不启动iOS模拟器。

## 主计算工作通道（R4.1.09，部分已实施）

`om-host-service::kernel::worker::KernelWorker`现在有独立CAS线程和2个等待job。每job显式pin指定parent，不保存自己的active指针；恢复用新的operation token/clock，实际source/settings非执行对齐后调用owned stage的单Math格入口，不隐式调用旧Evaluate cascade。注册表只在真实freeze后短锁插入immutable bytes，读/直接cancel不排在CAS后面。

跨越多个math epoch及同源码的修改/回退无法证明旧定义仍有效，因此保守清除全部原owned definitions；不重跑source或随机/history。实际错误与取消前的副作用保留在未接纳candidate中，CellBoundary的successful_statements不是definition effect count：Wolfram CompoundExpression可以0成功history但已有赋值。非reactive模式保持原重赋值行为，reactive依赖未就绪明确拒绝。

每candidate保留原runtime/generation/operation/selected parent/source/config/hash及真实terminal facts，取消也能到达已freeze且待接纳的token；池满只失败，不覆盖parent。close停止admission/token，join返回后台caller处理。后续实际NativeHost/Swift端口、document逻辑门、Blob+accepted SQLite同库持久化、active指针与接受事件仍待实施；此worker API仅产生临时candidate，不证明耐久接纳或完整工作台可用。

## 冻结接纳与同库记录（R4.1.09，部分已实施）

`FrozenKernelPlan`仅从真实空数学bootstrap或真实worker candidate构造；candidate明确关联已接纳parent类型，保留原producer source/cell revisions，当前acceptance_source另冻结。相同epoch下标题/非Math变化可以在生产后接受，math epoch/config/runtime/generation/parent不符拒绝且撤销临时lookup。不会从当前source重建历史cell revisions；迟到title也不改原producer packet。

新 `native-kernel-store.schema.json`生成Rust/Swift5 DTO（共205），有长度界定request hash、producer/Blob/byte length/source/acceptance revisions/parent/state revision/实际terminal status与history counts。无原request改变或receipt/hash/provenance不符能提升为accepted parent。独立KernelLifecycle让worker direct cancel和最终enter使用同一短锁；enter前cancel拒绝，enter后stop不伪造SQLite回滚，unknown保留原plan/state，matching trusted receipt才有AcceptedKernelState。普通unit里的合成receipt仅证明契约/lifecycle，不是physical durable证据。

新document store3/min reader3/codec3/user_version3支持accepted_checkpoints/kernel_head/accepted_results/kernel_operations/kernel_transitions/kernel_outbox；root/Library仍1、source/history records仍原codec。旧native v1/v2不自动upgrade，继续原source-only/full或history能力，kernel API明确unsupported；旧v2 reader拒绝3。Blob真实hash/length校验且transfer pin存活后，checkpoint blob_refs、原plan/sourceFK、result producer、head/state revision、receipt/transition/outbox在同库单transaction提交，实际WAL F_FULLFSYNC与独立原ID回读后才返成功。旧source revision裁减必须保留kernel producer/acceptance引用。

`StorageService.commitKernel`后台执行IO，Rust只在最终短barrier校验scope/lifecycle；当前physical fixture通过真实Rust bootstrap→Swift Blob/SQLite→原receipt/OMKS恢复→真实a/b/error的连续链。source Math变化可使document_head的当前active ref失效，而kernel_head仍保留最后已接纳基线，下一job必须非执行reconcile后使用；标题变化保留当前active ref。accepted_results目前仅保存checkpoint内真实cell输出的来源索引，完整ResultStore/分页/检查通道继续R4.1.10。

实际验证包含original-ID单效果、旧parent拒绝、rollback/owner barrier拒绝、postcommit lost ACK、真实F_FULLFSYNC失败后unknown及重新确认、实际Blob/outbox损坏拒绝、重开后exact bytes。5个真实SIGKILL点在同库事务中断时仅留下旧parent或完整accepted candidate，未发布的candidate不会成为head；unit的64次accept/cancel race仅证明短lifecycle线性化。NativeHost/CABI main计算注册、真实同文档共享逻辑门、UI/Agent结果事件和完整启动恢复仍待实施，不能将这些独立physical fixtures算整个R4.1.09 Done或final gate。

## 实际宿主主运行端口

NativeKernelEndpoint已进入SourceEndpoint同一owner和逻辑提交门；Math CAS线程独立，draft编辑/已确认source读取/direct stop可继续，frozen candidate与source commit不能绕过彼此的未知结果。KernelCommitPort/13号Mac ABI读回真实chunk bytes并发布Blob，只在matching durable receipt后推进AcceptedKernelState，完整IO由StorageService原writer处理。原producer title/source revision与接受时head分别保留，actual defs/owners上下文给source preview，不从source重建数学。

每个kernel job使用唯一operation/token，重复原请求返回原事实；副作用/Out/random被拒绝时连同未接纳state释放。多格执行每格耐久接纳，首个error/stop/stale保留actual accepted prefix。no-COMMIT确认仅在actual writer已settled且原receipt健康缺失/源与parent未变/稳定sync成功后出具，unknown不通过新ID计算。已关闭lifetime不再新admit或enter，线程提取后在后台join；新的runtime不会使用旧token。

build.rs读取实际Rust源码/manifest/lock/toolchain/Schema/catalog/ABI及target/profile生成编译指纹，作为可信producer build；测试源码变化也保守变化，不读取Git或用户配置，跨未知build不能恢复。这是host build时已锁sha2构建依赖，纯kernel WASM没有新增runtime依赖。旧iOS ABI保持，Mac追加kernel command additive symbol。

实际main runtime fixture经真实C ABI、MainActor DraftStore/原DocCommitPort、Blob和SQLite完成数学/来源/取消/未知/close验收；这里只证明R4.1.09任务范围，完整工作台/ResultStore/文件恢复/Agent与最终发行仍按后续任务。

# 无损数学 checkpoint 编码基础

R4.1.08正在实施。当前已接通数值原子、表达式图及Evaluator持续数学codec；整个Session/owners/输出及耐久checkpoint尚未完整编码，不作为可恢复主会话发布。

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

## 后续接入和验收

下一步与source/settings/build/catalog/owners/实际record evidence绑定完整Session checkpoint；暂态解释器frame不可捕获，只读fork不能提升，候选写入和真实耐久接纳继续有独立回执。`Session::fork_working_session`已克隆真实文档/结果/owners及read-only Explore snapshot，新取消token与parent必须独立，LLM/config-store/credential IO不随candidate复制；这只是owned work基础，不是完整byte checkpoint。codec本身不授予主文档权限。

本批新增 `om-num/tests/checkpoint.rs`、`om-core/tests/checkpoint.rs`；原num/core/working-stage与纯WASM/Clippy回归保留。数学53期望和原时限不变，本机不启动iOS模拟器，也不把这层往返当完整checkpoint完成。

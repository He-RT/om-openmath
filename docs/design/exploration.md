# 隔离参数探索与可取消数学任务

[下一版账本](../plan/NEXT_RELEASE.md) · [规范接口](../reference/executable.md#explore) · [验收](../acceptance/r35c/README.md)

本节为 `.3` dev 的 R3.5c 设计与实际接口；运行版本仍是 `.2`，图形/数据导出和三维继续。`explore` 的稳定身份为 fn_000225，现代命名参数来自元数据版本22；独立 `Explore` held 回调检查 controls/initial。传统 Manipulate 的迭代器语法尚未适配，不能靠别名冒充相同接口。

```om
# .3 dev 的实际接口；范围记录默认取中点，initial允许部分指定。
let rate=2
let a=99
explore(plot(a*sin(x),x:-pi..pi),controls:{a:0..4},initial:{a:2})
# 主笔记本 a 仍为99；滑块只改变局部a。
```

## 数学状态与身份

Explore本身保留原始表达式，只在当前范围/初值验证与输出生成时用只读作用域。controls 为1..16字段的名字→有限正宽度机器范围，初值默认中点；不允许内建函数/常量名、重复名字、未知初值键、越界值或高精度控制数静默降级。表达式可以返回真实精确/符号、机器数、矩阵、解卡片和二维图；计算保证仍属于实际内核算法，不因为提供滑块而变成认证结果。

静态依赖屏蔽 expression 内的滑块名，保留范围/初值的主会话依赖和其余表达式依赖。输出保存独立的只读 evaluator、原表达式和已验证范围；后续用户定义变化不会偷偷改变已有快照的数学含义。`Out[]` 在表达式内看到创建本次包装输出之前的真实历史，不递归引用自身 Explore 包装。随机流在快照中捕获，任务只读副本不推进主会话状态。

GetExploreContext和SampleExplore先核对 cell_id/out_index/view_id、Done状态和真正的保留探索对象。错误owner、旧ID、编辑/替换/失效输出明确拒绝；query是结果身份，不是文档写入权限。输出 `Explore` 携初始真实 `ExploreResult`、范围、初值和源式；每次结果回显producer与客户端revision，UI对照实例liveness/代次/所选参数，不把旧计算当当前结果。

## 独立宿主任务

只读上下文只含数学定义、下值规则、属性、历史、随机状态与求值限制，不含 KernelConfig、LLM profiles、Keychain、API凭据、文件路径或Agent会话。格式版本1是本项目内部任务DTO，与.omnb格式/Agent事务无关。上下文作为不透明JSON字符串在客户端传输，不把64位随机状态/高精度数据转换成JavaScript Number。

表达式按有序平坦节点表编码：Number沿用真实数值serde以保留精度，Symbol以名字跨进程驻留，Normal保存head/args的既有节点索引。直接构造还原原始树、ownvalue/downvalue、历史及属性，不重放赋值源码，也不通过格式化文本重新解释精度或原域。拒绝向前/循环/越界引用、未知版本、异常限制；最多512KiB/100000节点/4096定义和历史，所有遍历共享Interrupt预算。大快照明确失败，不省略定义假装同一个结果。

Web和桌面WebView使用同一纯WASM计算层创建独立Worker：每次新任务先终止上个Worker，旧回调不能进入当前结果；不用SharedArrayBuffer/跨源隔离，也不重启主笔记本Worker。新Worker没有网络提供商或配置密钥；结束/失败/10秒宿主上限/取消/卸载均销毁。变参33ms合并，参数计算与二维平移/缩放都在同一冻结数学上下文中运行；绘图器仍只显示内核真实几何。上下文重建也计入任务预算，宿主失败不能冒充计算成功。

iOS每个探索视图使用独立KernelClient/串行FFI会话，Swift Task取消传到该会话独立interrupt标志；新参数替换前一任务。主笔记本会话、保存和AI不会被滑块的取消重启。进入后台/消失时取消，离开视图销毁独立会话；恢复不自动重算全部单元格或所有滑块。Swift Canvas/数学排版仍为原生，二维重采样和数值查看都使用冻结上下文，不跳回主会话查询同名变量。

## 基础交互与持久化

滑块、可见重新计算/复位/取消按钮与键盘/原生辅助功能操作共同连接真实任务。计算中保留上次真实成功结果并标为过期；失败/取消保留所选参数，明确下方结果属于先前参数，不能把未算出的新值当成功。重新计算等待真正回执；复位使用初始化参数，主变量不变化。数值/矩阵沿用原生公式排版，二维沿用SVG/Canvas及采样数据；三维尚待R3.6，未宣称适配。

临时结果没有写入笔记本历史，因此不附假装绑定主记录的分页链接；现阶段矩阵/复杂值以完整数学表达式显示，复制/插入使用实际结果源式。初始成功结果的求值/采样耗时由实际宿主时钟测量；UI不显示捏造的任务百分比。选中的滑块值只是宿主视图状态，`.omnb` v1继续只保存用户原源码/initial，不保存内部上下文、任务ID或临时计算。

参考用户指定本地telegram-ui-reference的portable基础路径：`06-state-and-feedback/progress-and-result-feedback.md`（真实producer/代次/取消与迟到回调），`09-adaptation-and-accessibility/system-fonts-and-long-text.md`（窄宽换行、系统字体一次），`08-media-and-rich-content/media-browsing-and-zoom.md`（可见相机操作与数学域独立）。未复制Telegram类体系；算法推演不是运行证据，真实验收另列。

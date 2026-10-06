# 结构化结果、科学诊断与只读分页

[下一版账本](../plan/NEXT_RELEASE.md) · [当前进度](../plan/PROGRESS.md) · [验收](../acceptance/r35a/README.md)

本节记录 `.3` dev 的数据展示基础路径；完整二维扩展、explore、图形导出与三维仍在后续批次。桌面/Web与iOS使用同一个内核投影，前端不解析或重新运行数学源码生成表格。

## 真实值与数学保证

原 `OutputItem::Expr` 的 out_index/input_form/modern_form/latex 保持；仅结构化值追加可选 presentation。标量旧wire没有新字段。List/Matrix/Record/DataTable 根据真实表达式投影，空表保持表头，字符串仍是文本。Scalar nature 区分精确数、机器数、高精度数、符号、文本、布尔及Null；这是表示类型，不能把某个整数的精确性当作整个结果的数学保证。

ScientificOrigin只来自本次真实注册回调的尾返回，稳定function_id与名字共用当前目录。仅包装内容的List、未求值调用、任意用户记录、读取缓存的record均不获得本次计算来源。UI只有根页且origin存在时翻译科学状态/字段，普通记录按原值展示，不把converged/guarantee字段伪装成执行证据。精确global、数值驻点/有界候选、线性最小二乘/数值局部拟合、误差估计而非证书等语义分别保留。已有解卡片/步骤仍使用原独立求解证据。

模型/插值/有限级数/量和单位可展开完整原式；本批不宣称这些类型已经有专用交互图表。大型集合先给预览与展开动作，不把截断的源式当完整数学值；完整源码通过明确操作读取。

## 页与身份

新增 `InspectValue{query:ValueQuery}` → `ValuePage{page}`，Query携cell_id/out_index/view_id、零起始内部数据path、行/列offset与limit、include_source。内部path属于宿主契约，不改变语言1起始索引。每次输出由会话分配不重用的十进制serial，保存在StatementRecord；view_id作为不透明字符串，JavaScript不转为浮点编号。

分页只读当前Done单元格保留的不可变结果，核对输出索引/serial、路径、形状和范围后返回，不求值、不执行保存的源式、不改变定义/随机/输出历史。过期、替换、错误owner/ID/路径/限制均明确失败。32层路径、最多100行×32列，基础页32×8；行ID/值ID由producer的快照与不可变出现路径组成，行/列分页不会改变对象身份。UI不以当前位置索引替代producer ID，新快照作为新实例重建，文档切换继续沿已有客户端代次守卫。

Table对真实列/记录关系、重复字段/不等宽等作校验；无效形状保留原Expr显示，不虚构表格。投影的真实Abort/预算不被fallback吞掉。请求具有当前预算/时钟，源码读取保持明确操作，不产生隐藏计算。

## 三端基础交互

- Web/桌面：原生HTML表格、行/列标题、横/纵滚动、冻结列标题、前后页、嵌套展开/返回。根原式与选中标量都支持原有LaTeX/Wolfram/现代复制和插入，复制反馈等待实际clipboard完成。窄窗口控制换行，长文本保留，无悬停专属入口。
- SwiftUI：原生Grid/ScrollView，动态字体/安全区沿现有设施只作用一次；ViewThatFits在窄宽度堆叠标题/操作，关键文本不省略。值菜单给语义化复制/插入，producer ID用于ForEach。状态更新和Task回调核对实例liveness/generation/新旧view_id，过期结果禁用读取动作。
- 没有动画、虚拟化或定制测量时，静态分页与原生换行完整工作。本批分页不用捏造百分比；请求失败/未取得数据是明确状态，不用动画结束替代操作成功。完整表格/数学数据仍可通过源式访问，原Markdown/LaTeX输出保留。

代码实际连接为kernel `output::values`/`Session::inspect_value`、React `ValueView` 与 SwiftUI `ValueOutputView`。CLI保持原文本/JSON和原式，不声明交互分页面板。capabilities仅桌面/Web/iOS新增table/record/numerical_diagnostics，不授予任务权限或声明三维。

## UI依据与验收边界

按用户指定本地 `telegram-ui-reference` 的portable基础路径实施，未复制Telegram类体系。读取的Markdown：

- `/Users/hert/Documents/ChatGPT/ui-learning/04-pages-and-flows/information-lists.md`：producer稳定ID、旧对象不能接受当前动作。
- `/Users/hert/Documents/ChatGPT/ui-learning/06-state-and-feedback/progress-and-result-feedback.md`：真实producer/版本状态、取消或迟到回调不能覆盖新状态。
- `/Users/hert/Documents/ChatGPT/ui-learning/09-adaptation-and-accessibility/system-fonts-and-long-text.md`：关键文字换行、狭窄时堆叠、字体缩放一次。

上述算法是参考规格；真实测试与截图另列，不能把手推worked-example当执行验收。Web已通过实际WASM40行中文/emoji翻页、390宽度、复制反馈和精确global报告/嵌套参数点；截图见验收页。SwiftUI已generic iOS27 ARM64编译，新增原生协议/XCUITest随CI验证；本机没有启动模拟器，不把编译成功当真机/模拟器UI通过。

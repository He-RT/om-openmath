# 原生结果、条件与数值投影

R4.2.09 的 `ResultPresenter` 读取原 `ResultClient` 通道的真实 summary、presentation 和 Binding，按结果引用保留一个文档拥有的展示 owner。主笔记本继续由原 KernelCommitPort 控制；隐藏检查器或滚动离开内容不创建第二个计算会话，也不重新求解。展示第一次随已接纳输出一起准备，避免目的地切换时延迟类型变化重置读点。

解展示只消费内核保存的 `SolutionSetView`：finite、none、all、region 分别显示，未知或未求值表达式保留原式和真实消息。变量绑定、条件、区域、区间开闭性、Root 的多项式/根序及可用根式均来自原记录。条件不同、验证不同或原UTF-8不同的记录不因相同画面合并。精确验证、按构造成立、数值验证位数、尚未验证使用各自真实标签；数值近似和误差证书不混同。

核心的固定 presentation page 会从整个原解集的完整 typed UTF-8 字节身份取得重数和首次出现序号，返回当前页对应的 `solution_multiplicities`、`solution_first_indices`。包含条件、verification 和全部绑定信息，不能只比较 LaTeX。派生索引最多20000条、累计序列化8MiB，并检查实际 Interrupt；超预算原结果/源式仍保留。原数学求值、公共 SolutionSetView 和求解期望不变。Swift只在一页内安排同一实际解的展示，重数跨页仍来自完整记录，20重根不会在16+4页中变成16重或4重。

源码修改保留旧输出并提示需要核对，正在重算保留上次结果；运行单元格由原 operationCell 跟随实际操作，焦点导航不能改变它。接纳新输出后，本会话最近32次历史只保留小的原回执/Binding，完整历史值由原引用按需回读，不复制全部大值。更早记录离开内存列表时明确说明，原耐久回执不删除；引用过期或数据不可用时显示真实失败，不能自动运行旧源码恢复画面。全库历史/数据管理界面继续由 R4.2.18 提供。

ResultClient 对每次实际回复核对选择代次、runtime/document generation、当前 source/epoch/kernel scope 和原 result/out/view。期间提交新主结果或改变文档，迟到的 summary/类型/数值不更新当前展示。历史复制允许真实原值，不将历史 source hash 伪装成当前来源。零成功语句的错误、带已接纳前缀的错误、输出被实际抑制的语句分别呈现；不凭一次 Done 宣称数学目标完成。

整值数值操作用原 `InspectResultNumeric` 和显式20/50/80位；单个解用该原记录的完整 InputForm 与 `InspectExpression` 的 readonly numeric 分支（固定20位）。客户端验证页、原序号、绑定位置和完整原字节，投影缓存按原 result_ref、原记录位置和源式隔离。请求在保留的只读定义上下文中执行，不推进主定义、Out、随机或源码；原精确式仍可访问。高精度原数和投影源码保留精度标记，不转 Double 来排序或复制。

完整原结果、原始 LaTeX、Modern/Wolfram、原输入均有独立读取/复制出口。屏幕大文本只显示明确预览，完整内容经过原分块/总长度/SHA核对后复制。空的新结果、错误类型或不可读取内容不生成一个看似成功的空公式。

实际证据见 [R4.2.09开发记录](../acceptance/pre-alpha.4/development/r42/native-results.json)：12个实际原生/C ABI用例、两页真实20重根、80位投影、实际原生源编辑、真实迟到回复、历史原值和主Out/random/source不变；另包括可见窗口检查及原53公式/工作台/存储编辑回归。结构化表格、步骤树、Markdown、绘图、千格性能和最终candidate门禁仍由各自任务实现，32个final gates保持 not_run。

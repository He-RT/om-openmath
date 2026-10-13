# 原生公式与原式

R4.2.07 接通真实结果的 SwiftMath 排版。公式只消费原已接纳结果的只读 LaTeX、Modern 与完整 InputForm，不求值、不提交源码、不生成近似数。`NotebookValueResult` 保留原 `ResultBinding` 和原始三种源式；LaTeX 暂不可读时保留实际计算回执和可复制的 InputForm。结构化解与数值精度标签另由 R4.2.09 实现。

Mac 工程和独立 fixture 同用 SwiftMath 1.7.3 的固定 revision `fa8244ed032f4a1ade4cb0571bf87d2f1a9fd2d7`。SPM 的原始字体/metrics 资源包随 App 安装，默认 latinmodern-math。完整资源及四份代码/字体许可的字节数和 SHA-256 见 `macos/OpenMathNative/Resources/fonts/swiftmath-fonts.json`；`verify-math-resources.py` 核对已安装 App 内的全部文件。字体发现只查当前可执行文件/App 自有 bundle；默认字体与 metrics 散列不符或缺失时显示原式，不借开发 checkout 或用户字体修补安装包。

`MathRenderAdapter` 保留原始 UTF-8 和 `displayLatex`。唯一显示适配是词法识别、flat ASCII 名称的 `\operatorname{...}` 到 `\mathrm{...}`；记录转换数量并提示名称使用直立字体。转义反斜线、星号形式、嵌套或非 ASCII 名称不改写。源式菜单和键盘复制始终使用原始 LaTeX，不复制适配后的字符串。NFC/NFD 以原字节区别缓存身份。

自动排版上限为原 LaTeX 64 KiB、逻辑深度 128、实际 math atoms 20000。前置扫描覆盖有括号与无括号参数、script、font/accent、根式、分式、left/right、环境表格及 infix 分式，先阻止固定 parser 的递归超限；解析后再按实际 AST 核对所有子路与原子数，之后才用原生字体排版。未知命令、缺参数、预算超限、字体缺失和非法字号都明确显示短原因及可选原式；较长原式按4096字符分段访问，完整复制保留所有字节。没有删命令或括号来制造“成功”。

每个展示位置持有独立 `MTMathUILabel`，解析和原生字体/排版操作均归 MainActor。缓存身份包括原 LaTeX 散列、转换版本、inline/display、字体和实际散列、字号及主题；NSView 不共享或跨线程。`NativeMathContainer` 从公开实际 display 的 ascent/descent 获取基线。行内附件使用 text style、真实 descent 定位 `NSTextAttachmentViewProvider`，`NativeMathTextView` 将 TextKit2 实际 fragment 的 provider view 安装在该 fragment 给出的附件 frame。重新排版时旧附件视图移除；不使用假 attachment 标志或盒高的一半冒充基线。Markdown 的源码映射、语义单元选择和复制规则由 R4.2.08 接线。

独立展示使用 display style 与横向滚动，超宽保持原字号，不自动缩小或拆分式。用户可显式调整8–96 pt，查看只读原式，并复制原始 LaTeX、Modern 或 Wolfram。点击展示公式取得整式焦点，Command-C 复制完整原 LaTeX；行内公式由其 TextKit 文本宿主管理选择。辅助标签使用实际可读源式，不增加数学证明。

实际验证见 [开发证据](../acceptance/pre-alpha.4/development/r42/native-math.json)：53个原始算例及4个科研算例经原 C ABI/SQLite 物理回执获取 LaTeX，再进入生产 renderer；分式、根式、矩阵、cases、区间在12/22/42 pt核对实际 metrics，另验超宽式、内联 provider、原式 fallback、预算、缺失/损坏字体、完整复制和源版本不变。原生可见 fixture 的操作记录只贡献开发覆盖；Markdown、千格性能和最终同candidate门禁仍需后续任务，32项 final gates 保持 not_run。

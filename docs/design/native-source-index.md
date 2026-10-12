# 原生源码位置与原始字节

R4.2.02 已实现。`NativeSourceEditor` 仅呈现文档原有的 `DraftTextViewAdapter`，使用实际 AppKit/TextKit2 和响应链，不复制草稿 owner。数学源关闭 smart quote/dash/replacement/spelling/completion、smart insert-delete 和自动链接/数据识别。纯文本粘贴进入当前单元格，不拆格；Enter 使用实际原文检测到的首个换行风格。Marked text 仍交原生输入引擎处理。

`SourceIndexMap` 绑定原 UTF-8 SHA256 和 draft_sequence，逐 Unicode scalar 记录 UTF-8/UTF-16 边界和逻辑行首。CRLF 算一次逻辑换行，原两个码元保留；Tab、组合字符、ZWJ 与旗帜不按视觉宽度或 Character 数重写。原生范围/内核范围都须准确映射；越界、溢出、半代理对、UTF-8 内部位置、非法 UTF-8、旧 sequence/hash 全部拒绝，不能 clamp 或默认跳零。字素簇显示扩展与实际编辑范围分开，提供独立 composed-range 校验供后续 Fix 使用。

索引随实际输入/回执更新；只改变选区或 draft_sequence 时复用原边界表与散列。中文、A🙂B、NFC/NFD、家庭 ZWJ、区域旗帜、Tab、混合 CRLF/CR/LF、空文/EOF 的42个 scalar 端点及全部合法范围往返通过。真实 TextKit2 窗口保留原始 NFD/CRLF 字节、Enter 插入 CRLF、粘贴引号/破折号/多行仍是原单元格，已确认源码版本不由此测试自动提交。

原提交/撤销、3草稿/6保存/5恢复中断与 Release 回归通过。完整真实输入法候选窗口、语法高亮、Complete/Hover/Fix 和长笔记本布局仍分别由 R4.2.03–06 完成，不把 scalar 映射测试声称为这些能力的完整验收。

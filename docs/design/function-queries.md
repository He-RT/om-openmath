# 帮助与真实能力查询

本页记录 `.3` 开发分支的四个实际查询入口。描述与文档索引均从 `functions.toml` 生成，保持稳定身份，不从函数名推断数学能力。

```text
help("integrate")                 # 可看规划；executable=false
help("map").runtime.pipe_arg      # 真实 2
options("quantity")               # 单位必填二选一的实际规则
functions()                       # 默认仅当前回调身份，不含规划
functions(category:"calculus",stage:"planned")
functions(category:"agent",stage:"deferred")
capabilities()
```

`help(name)` 返回身份、分类、状态、是否可执行、文档与实际 runtime 描述。同一语义的调用变体共享 ID，`help("NSolve")` 保留该真实变体的参数；完整目标说明不能替代它的当前范围。`help` 和 `options` 保持名称参数，不执行其中的任意表达式。可使用文字、未遮蔽的符号，或安全读取文字 ownvalue；被用户绑定遮蔽的别名明确诊断，内置查询可以使用兼容名称。

`options(name)` 的 options 只来自实际 ParameterSchema。规划入口返回空的可执行选项集；文档参数另放在 documentation_parameters，并明确标记不是可执行 schema。文字「必填」「none」不是可以提交给内核的默认值。依赖上下文的必填条件保留真实 default_context，不伪造固定默认值。

`functions` 的 stage 可为 current/planned/deferred；current 默认只列有真实回调的稳定身份，包含 partial 的实际范围。规划与后续条目均带 executable=false，不加入默认执行补全。category 使用全景目录中的分类 ID，未知分类和错误参数明确诊断。

`capabilities` 提供内核版本、描述版本、schema 版本、真实回调数、函数身份与计算平台。没有宿主上下文时 host_presentation 和 task_permissions 为 Null；不能借此宣称某个平台已有三维展示或某个任务已获写入权限。宿主层的 GetCapabilities 保持原协议。函数副作用标签也不能授权任意嵌套表达式，只读环境继续负责实际隔离。

查询仅访问随源码生成的只读数据，不读取用户文件、网络或凭据；遍历和结果构造使用原 Interrupt。`.omnb` 不增加 Agent、查询历史或权限字段。Agent 工具仍是后续预留，浏览其设计不表示可以执行。

验收入口：[reflection.rs](../../crates/om-eval/tests/reflection.rs)、[目录契约](../../tests/test_function_docs.py)。实际回调、规划隔离、保持参数、绑定遮蔽、上下文默认值、未知权限与预算均实际检查。

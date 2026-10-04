<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->

# Notebook Agent 与事务预留

[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)

所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。

| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |
|---|---|---|---|
| [`apply_notebook_patch`](#apply_notebook_patch) | 后续规划 | 规划接口，当前不可用 | 预期版本加唯一operation_id；临时文档验证所有操作后原子提交，失败全部不提交。 |
| [`cancel_task`](#cancel_task) | 后续规划 | 规划接口，当前不可用 | 取消信号传真实计算/模型任务，返回结构化终止状态；不是编辑回滚。 |
| [`get_function_docs`](#get_function_docs) | 后续规划 | 规划接口，当前不可用 | 按稳定函数ID及描述版本查说明，明确当前/规划、精度、条件、返回与能力。 |
| [`read_notebook`](#read_notebook) | 后续规划 | 规划接口，当前不可用 | 读取真实快照：稳定cell ID、document_revision、结果对应source_revision与过期状态。 |
| [`run_cells`](#run_cells) | 后续规划 | 规划接口，当前不可用 | 独立计算请求，绑定源版本与任务ID；不隐式作为patch的一部分。 |
| [`undo_transaction`](#undo_transaction) | 后续规划 | 规划接口，当前不可用 | 使用反向操作且检查被改内容仍匹配，不覆盖后续手工编辑；结果标待重算。 |

## apply_notebook_patch

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000825`；条目类型：`function`。
- 副作用分类（设计预留）：`write_document`；参数验证阶段：`documentation_only`，不构成工具授权。

预期版本加唯一operation_id；临时文档验证所有操作后原子提交，失败全部不提交。

- 当前支持：预留，尚未实现；当前无此Notebook工具。
- 目标范围：后续应用层契约，与Pi/Rig框架类型隔离；Agent会话/提示词版本/权限/操作日志/凭据不写.omnb。
- 返回：transaction_receipt
- 精度：结构化业务回执，不生成数学结论；编辑、执行、保存成功分别等待真实确认。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
apply_notebook_patch(document_id, generation, expected_revision, operation_id, operations)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `document_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `generation` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `expected_revision` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `operation_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `operations` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |

验收：版本冲突/非法操作/IME保护；相同ID相同内容重试返回原回执，不同内容拒绝。批量删除不触发中间计算。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## cancel_task

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000829`；条目类型：`function`。
- 副作用分类（设计预留）：`write_session`；参数验证阶段：`documentation_only`，不构成工具授权。

取消信号传真实计算/模型任务，返回结构化终止状态；不是编辑回滚。

- 当前支持：预留，尚未实现；当前无此Notebook工具。
- 目标范围：后续应用层契约，与Pi/Rig框架类型隔离；Agent会话/提示词版本/权限/操作日志/凭据不写.omnb。
- 返回：task_status
- 精度：结构化业务回执，不生成数学结论；编辑、执行、保存成功分别等待真实确认。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
cancel_task(document_id, generation, task_id)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `document_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `generation` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `task_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |

验收：文档切换、重复取消、迟到回复、不存在任务及已结束任务分别处理。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## get_function_docs

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000827`；条目类型：`function`。
- 副作用分类（设计预留）：`read_session`；参数验证阶段：`documentation_only`，不构成工具授权。

按稳定函数ID及描述版本查说明，明确当前/规划、精度、条件、返回与能力。

- 当前支持：预留，尚未实现；当前无此Notebook工具。
- 目标范围：后续应用层契约，与Pi/Rig框架类型隔离；Agent会话/提示词版本/权限/操作日志/凭据不写.omnb。
- 返回：function_descriptor
- 精度：结构化业务回执，不生成数学结论；编辑、执行、保存成功分别等待真实确认。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
get_function_docs(function_id, metadata_version)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `function_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `metadata_version` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |

验收：规划条目可浏览但不能进入可执行工具集合；名称变化不改变身份。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## read_notebook

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000824`；条目类型：`function`。
- 副作用分类（设计预留）：`read_session`；参数验证阶段：`documentation_only`，不构成工具授权。

读取真实快照：稳定cell ID、document_revision、结果对应source_revision与过期状态。

- 当前支持：预留，尚未实现；当前无此Notebook工具。
- 目标范围：后续应用层契约，与Pi/Rig框架类型隔离；Agent会话/提示词版本/权限/操作日志/凭据不写.omnb。
- 返回：notebook_snapshot
- 精度：结构化业务回执，不生成数学结论；编辑、执行、保存成功分别等待真实确认。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
read_notebook(document_id, generation)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `document_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `generation` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |

验收：文档代次、范围与权限检查；快照不得混入提示词或凭据。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## run_cells

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000826`；条目类型：`function`。
- 副作用分类（设计预留）：`write_session`；参数验证阶段：`documentation_only`，不构成工具授权。

独立计算请求，绑定源版本与任务ID；不隐式作为patch的一部分。

- 当前支持：预留，尚未实现；当前无此Notebook工具。
- 目标范围：后续应用层契约，与Pi/Rig框架类型隔离；Agent会话/提示词版本/权限/操作日志/凭据不写.omnb。
- 返回：task_events
- 精度：结构化业务回执，不生成数学结论；编辑、执行、保存成功分别等待真实确认。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
run_cells(document_id, generation, source_revision, cell_ids, task_id)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `document_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `generation` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `source_revision` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `cell_ids` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `task_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |

验收：失败/取消保留已提交编辑；旧回复不能覆盖新编辑；只读/写会话权限由真实执行环境校验。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

## undo_transaction

**当前实现：后续规划；目标接口：规划接口，当前不可用。** 目标版本：`后续未定`。

- 稳定身份：`fn_000828`；条目类型：`function`。
- 副作用分类（设计预留）：`write_document`；参数验证阶段：`documentation_only`，不构成工具授权。

使用反向操作且检查被改内容仍匹配，不覆盖后续手工编辑；结果标待重算。

- 当前支持：预留，尚未实现；当前无此Notebook工具。
- 目标范围：后续应用层契约，与Pi/Rig框架类型隔离；Agent会话/提示词版本/权限/操作日志/凭据不写.omnb。
- 返回：transaction_receipt
- 精度：结构化业务回执，不生成数学结论；编辑、执行、保存成功分别等待真实确认。
- 当前计算平台：无；目标计算平台：未承诺。
- 目标图形/交互展示平台：不适用或后续未定。
- 兼容名称：无既有兼容入口。
- 管道位置：不接受自动管道输入。

目标现代签名（按目标接口状态判断是否已可执行）：

```text
undo_transaction(document_id, generation, transaction_id, expected_revision)
```

| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |
|---|---|---|---|---|
| `document_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `generation` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `transaction_id` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |
| `expected_revision` | positional | 必填（预留） | 后续需锁定可验证类型/权限，不把文字默认值当schema。 | later |

验收：部分冲突不得以整个旧快照覆盖新文档；幂等、回执和原子性独立验收。

当前源码：暂无当前实现证据。

当前测试引用：暂无当前实现证据。

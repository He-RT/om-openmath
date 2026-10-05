# 纯数据解析与导出

本页记录 `.3` 开发分支已实现的 CSV/JSON 契约。公开版本仍为 `.2`；专门的跨端表格展示与文件导出界面在 R3.5 接续。

JSON 依照 [RFC8259](https://www.rfc-editor.org/rfc/rfc8259) 扫描数据，CSV 依照 [RFC4180](https://www.rfc-editor.org/rfc/rfc4180) 处理引号和字段，并接受 LF 行尾、UTF-8 及开头的 CSV BOM。本项目编写扫描器，复用已锁定的 serde_json 处理字符串转义。解析器不调用数学源码解析器、求值器、文件或网络接口。

| 接口 | 返回与规则 |
|---|---|
| `parse_json(text)` | 对象是有序 Record，数组是 List；空 `{}` 为 Record，空 `[]` 为 List，布尔和 null 对应 True/False/Null。整数和有限十进制直接构造精确整数/有理数。解码后的重复键拒绝。 |
| `to_json(value)` | 仅接受纯数据。数值保持存储点值，允许精确终止十进制与有限机器/大浮点的实际二进制点值；必要时使用十进制科学记数。非终止十进制有理数（如 1/3）、非有限数、复数及未求值数学式拒绝，不能静默近似。 |
| `parse_csv(text, header: true)` | 字段保持字符串。默认返回 `DataTable[columns, records]`，保留列顺序及零数据行的表头。`header: false` 返回矩形行列表。重复表头、不等宽行和不完整引号拒绝。 |
| `to_csv(table)` | 接受 DataTable、同字段记录行或等宽标量行列表；表格/记录行带表头，普通行列表不补表头。字段可为字符串、终止十进制数、布尔或 Null（空字段）。输出 CRLF，引号双写，字段内的换行和中文原样保留。 |

CSV 没有类型和精度信息，因此读回的数值字段仍是字符串；需要数值时显式转换。JSON 保存数学数值点，不保存 Number 的原机器/大浮点精度类别、负零标志或来源误差。精确有理构造不是宣称新增任意精度数值算法。`numeric(1/3)` 可以显式选择近似后再导出，原精确 1/3 不会自动变为小数。

```text
let table = parse_csv("名字,质量\n西瓜,2.5\n")
table[1]                 # 列名
table[2]                 # 记录行；可接已有 map/filter 等列表操作
table[2][1].名字
to_csv(table)
parse_json("{\"a\":0.1,\"b\":[]}").a  # 精确 1/10
to_json({a: 1/10, b: true})
```

输入/输出各限 8MiB；JSON 数据节点限 100000、嵌套限 64；CSV/表格字段限 100000。十进制文字长度和指数分别限 20000，输出有效位/指数也受检查；预算不足、取消和超限明确失败。每个扫描、构造及输出循环使用现有 Interrupt，包括长字符串和精确十进制转换。

DataTable 是只追加的受保护核心数据头，不是一个占位可执行函数，也不进入函数补全。`.omnb` 仍仅保存单元格源码，不增加缓存数据字段。当前通用表达式展示保留完整源式；R3.5 将提供专门表格结果、宿主文件保存与跨端操作。

真实契约测试见 [data_io.rs](../../crates/om-eval/tests/data_io.rs)，覆盖 Unicode/代理对、重复键、完整引号、零行表头、极端十进制往返、原机器点值与中断。所有原 53 条数学期望保持不变。

## 表格计算与访问

`table.columns` 取列名，`table.rows` 取按表头整理的记录行，`table.列名` 返回该列的值列表。列名若为 `columns` 或 `rows`，通过 `table.rows` 中的记录访问对应列。此前开发版的数字下标 1/2 仍访问原始构造字段；行区间使用 `slice(table, 1..3)`。

`length` 返回行数，`first`/`last` 返回记录。`filter`、`sort`、`sort_by`、`unique`、`take`、`drop`、`slice`、`rest` 和 `append` 保留表头，包括结果零行的情况；追加记录必须与列一致。`map` 返回映射结果的列表，`fold` 对记录行归约，`group_by` 的 values 为保留表头的子表格；`zip`/`flatten`/`reshape` 对记录行执行既有列表语义。

表格同一行按表头归一字段顺序，重复行不会因字段书写顺序而被误分。普通同字段记录也可 `sort`，按首行字段顺序稳定比较，标量类型次序为数、文本、布尔、Null；数按实际有理点值比较，文本按字典序，false 在 true 前。复杂字段使用 `sort_by` 提供标量键。函数键每行执行一次，比较过程逐字段检查中断。

```text
let t = parse_csv("类别,值\nA,2\nB,1\nA,3\n")
filter(t, fn(r) => r.类别 == "A")
t.值 |> map(fn(x) => numeric(decimal(x))) |> mean()
to_csv(sort_by(t, fn(r) => numeric(decimal(r.值))))
group_by(t, fn(r) => r.类别)
```

表格语义、表头保留、字段归一、键仅执行一次及失败/取消验收见 [table_data.rs](../../crates/om-eval/tests/table_data.rs)。专门的 UI 表格仍待 R3.5。

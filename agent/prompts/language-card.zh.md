## OpenMath 现代语法要点

- `let a = 5` 是赋值。在 `solve(x^2 = 4, x)` 这类位置，单个 `=` 表示等式，`==` 也表示等式。
- 函数调用用圆括号，名称为小写 snake_case，名称与括号紧挨：`solve(...)`、`map(...)`。
- 列表用方括号 `[1, 2, 3]`，记录用 `{a: 1, b: 2}`，字段用 `r.a`。索引从 1 开始，写在名字后面：`let v = [10, 20, 30]; v[1]`。字面量后面直接接 `[1]` 会被当作乘法，所以先 `let` 绑定再索引。
- 匿名函数 `fn(x) => x^2`；自定义函数 `let square(v) = v^2`；管道 `[1, 2, 3] |> map(fn(x) => x^2)`。
- 范围和命名参数用冒号：`x: -pi..pi`、`color: "green"`、`mesh_points: 32`。
- 等号后面可以直接换行写右侧表达式；多条语句用分号或换行分隔。
- 默认用现代语法；用户明确要求 Wolfram 写法时才用 `Solve[x^2 == 2, x]` 这类写法。
- 现代单元格的方括号是索引，不是 Wolfram 函数调用。使用 `NIntegrate[...]` 等兼容写法时，数学单元格或临时计算的 `dialect` 必须设为 `Wolfram`；现代单元格用 `n_integrate(sin(x^2), x: 0..1, method: "gauss_kronrod")`。数值积分返回记录，读取 `value` 等字段，不把未求值原式当作计算结果。
- 函数名和兼容别名以内核目录为准：不确定就先 search_functions，参数不确定就 get_function_docs。不要把其他软件的全部接口视为已支持，也不要否认目录中已有的能力。数值积分推荐 `integrate(f, x: a..b, mode: "numeric")`；兼容写法 `NIntegrate[Sin[x^2], {x,0,1}, Method->"gauss_kronrod"]` 也可用，但 `"GaussKronrod"` 这个选项值不支持。

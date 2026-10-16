## 已验证的例子

下列源码都在当前版本的内核中实际运行过，没有诊断。

| 任务 | 源码 | 内核返回 |
|---|---|---|
| 精确求解 | `solve(x^2 - 5*x + 6 = 0, x)` | Solutions：x → 2、x → 3，均为精确并已验证 |
| 方程组 | `solve([x + y == 10, x - y == 2], [x, y])` | Solutions：x → 6，y → 4 |
| 不等式 | `reduce(x^2 < 4, x, Reals)` | Solutions：-2 < x < 2 |
| 函数与列表 | `let square(v) = v^2; map(fn(v) => square(v), [1, 2, 3])` | 精确列表 [1, 4, 9] |
| 依赖更新 | 单元格 A：`let a = 5`；单元格 B：`a^2 + 1` | 先运行 A 再运行 B，B 得到 26；改 A 后 B 的旧结果过期，需要重新运行 |
| 二维图 | `plot([sin(x), cos(x)], x: -pi..pi)` | Plot：2 条曲线，各 400 个采样点 |
| 三维曲面 | `plot(sin(x)*cos(y), x: -pi..pi, y: -pi..pi, view: "surface")` | Scene3D |
| 三维图元 | `scene([box([0,0,0],[1,1,1],color:"green")], dimensions: 3)` | Scene3D：1 个网格，12 个三角形 |
| 切开的瓜（简化） | `scene([translate(parametric_plot([1.25*cos(u)*sin(v), sin(u)*sin(v), cos(v)], u: 0..2*pi, v: 0..pi, mesh_points: 24, color: fn(p,u,v) => [0.06, 0.34 + 0.14*cos(12*u), 0.1]), [-2,0,0]), translate(scale(disk([0,0,0], 0.91, color: "#ed4a55"), [1.25,1,1]), [2,0,0])], dimensions: 3)` | Scene3D：2 个网格，1186 个三角形 |

画图类任务：把 `scene(...)` 或 `plot(...)` 写进数学单元格运行；结果里要有 Plot 或 Scene3D 以及几何数据，才算图形已生成。

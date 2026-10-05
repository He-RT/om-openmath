# 纯数值分析层与机器矩阵

本节记录当前 `dev` 实现；公开 `.2` 不包含这些新增入口。完整 `.3` 的 ODE/优化/拟合/单位/绘图等门禁仍在实施。

`om-analysis` 只依赖已有 `om-num`/`thiserror`，接受有限实数样本与宿主注入的 Interrupt。它不解析数学源码、不求表达式值、不访问文件/网络、不创建线程或时钟，禁止 unsafe，并保持 WASM 可编译。源码、表达式及平台展示转换属于 `om-eval`/Kernel/宿主。

当前机器矩阵尺寸1..64；所有结果和中间存储检查有限值，预算与取消沿原机制传播。原有精确有理矩阵继续复用 Bareiss。

| 入口 | 算法与结果 | 限制 |
|---|---|---|
| lu | 部分选主元，P*A=L*U，L(m×m)/U(m×n)，1起始行置换、真实重构残差 | 矩形可分解；求解/行列式需要方阵 |
| qr | Householder，A=Q*R，Q(m×m)/R(m×n)，真实重构残差 | 实数稠密矩阵 |
| cholesky | 先缩放，A=L*transpose(L)，返回下三角因子 | 实对称正定；相对32机器epsilon检查，非正定/数值退化拒绝 |
| svd | 缩放的一侧Jacobi，A=U*S*transpose(V)，完整正交因子及降序values/residual/sweeps | 100轮；未收敛或不可表示极端尺度失败；旋转不可表示的微小相关残列可视作数值零，独立微小列保留 |
| eigenvalues/eigensystem | 缩放实对称最大枢轴Jacobi，值升序，匹配列向量与真实残差/旋转数 | 仅实对称，100*n*n旋转上限；非对称/复特征系统不支持 |
| least_squares | 满列秩m≥n Householder QR；solution/residual_norm/method/converged | 相对阈值1e-12；满列秩QR，欠定/秩亏SVD最小范数，同时提供实际残差/数值秩/零空间方向 |
| linear_solve(mode:"numeric") | 方阵LU真实求解，默认mode:"exact"保留 | 精确/机器矩形与欠定均保留自由参数；唯一解保留列表，非唯一返回solution/particular/null_space/parameters等字段。不相容拒绝，机器rank_kind明确numerical |
| det/inverse | 按实际输入选择精确Bareiss或机器LU | 数值溢出/病态显式诊断 |

残差是对当前存储机器矩阵进行重构/代入的计算结果，不能代替数学误差证书；名义precision/accuracy也不是物理准确度声明。机器rank/null_space基于SVD，1e-12相对阈值，只声明数值秩。数值一致性使用实际残差和浮点后向容差，不声称精确证书。精确欠定由Bareiss生成完整特解、零空间和自由列，使用C[1]等参数；有唯一解的返回格式保持兼容。

矩阵实现为独立手写 Rust，未复制或链接外部数学运行时。算法和结果约定参考 [LAPACK QR 文档](https://www.netlib.org/lapack/lug/node69.html)、[一侧Jacobi SVD接口](https://www.netlib.org/lapack/explore-html/d9/deb/group__gesvj_ga7aec05d2a1523bbeee77ece21b12187c.html)、[实对称正定Cholesky说明](https://netlib.org/lapack/explore-html/de/db9/group__potf2_gac3e7c9b72833e5467d91259ce0d0d4d4.html)。这些参考不是依赖或本项目能力声明。

验收见 `crates/om-analysis/tests/matrix.rs` 和 `crates/om-eval/tests/numeric_matrix.rs`：解析解/正交性/重构残差、固定种子48组不同矩形与秩、重复特征值、较大尺度、极端尺度失败、取消及预算；不修改原53数学语料。

## 向量夹角与投影

angle/VectorAngle定义为acos(dot(a,b)/(norm(a)*norm(b)))，共轭第一向量的内积约定，弧度；复数输入允许真实复值或精确保留的ArcCos式。机器实数仅修正64机器epsilon内的舍入越界，超过则诊断，不伪造可用角度。projection/Projection定义为(dot(onto,a)/dot(onto,onto))*onto，拒绝零目标。数组1..64维且两向量等长，接受数值实/复元素。

投影的共轭约定参照[Wolfram Projection](https://reference.wolfram.com/language/ref/Projection.html)；复数夹角可以非实，参照[VectorAngle说明](https://reference.wolfram.com/language/ref/VectorAngle.html)。OpenMath的dot向量约定独立列明，不把Wolfram Dot的双线性复数行为冒充为同一保证；此前公开.2无Dot实现。

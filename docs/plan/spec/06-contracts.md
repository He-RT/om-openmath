<!-- Extracted from docs/plan/PLAN.md sections [6]. PLAN.md is authoritative; keep in sync. -->

## 6. 核心数据类型与接口契约

> 本节代码是**契约**：名字、字段、签名必须一致。函数体由各任务实现。

### 6.1 om-num

```rust
// crates/om-num/src/lib.rs
pub use dashu::integer::IBig;           // 重导出，其他 crate 不直接依赖 dashu
pub use dashu::rational::RBig;

pub type Integer = IBig;
pub type Rational = RBig;               // 不变量：已约分，分母 > 0（dashu 保证）

/// 实数：机器精度或任意精度（二进制，精度以 bit 计）
#[derive(Clone, Debug)]
pub enum Real {
    Machine(f64),
    Big(BigFloat),                      // BigFloat = dashu::float::FBig<HalfEven, 2>
}

/// 精确或近似的数；Complex 的实部虚部类型相同类别（都精确或都近似）
#[derive(Clone, Debug)]
pub enum Number {
    Integer(Integer),
    Rational(Rational),                 // 不变量：分母 != 1（否则归一为 Integer）
    Real(Real),
    Complex(Box<Complex>),              // 不变量：im != 0（否则归一为实部）
}
#[derive(Clone, Debug)]
pub struct Complex { pub re: Number, pub im: Number }   // re/im 不能是 Complex

impl Number {
    pub fn normalize(self) -> Number;               // 执行上述不变量
    pub fn is_exact(&self) -> bool;
    pub fn is_zero(&self) -> bool; pub fn is_one(&self) -> bool; pub fn is_negative(&self) -> bool; // 仅对实数有意义，Complex 返回 false
    pub fn add(&self, o: &Number) -> Number; pub fn mul(&self, o: &Number) -> Number;
    pub fn neg(&self) -> Number; pub fn recip(&self) -> Result<Number, NumError>;  // 0 的倒数 -> Err(DivByZero)
    pub fn pow_int(&self, e: &Integer) -> Result<Number, NumError>;
    pub fn to_f64(&self) -> Option<f64>;            // Complex -> None
    pub fn to_complex_f64(&self) -> (f64, f64);
    pub fn precision(&self) -> Precision;           // Exact | Machine | Bits(u32)
    pub fn cmp_real(&self, o: &Number) -> Option<std::cmp::Ordering>; // 任一为 Complex -> None
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision { Exact, Machine, Bits(u32) }

// 数论（crates/om-num/src/ntheory.rs）
pub fn gcd(a: &Integer, b: &Integer) -> Integer;
pub fn ext_gcd(a: &Integer, b: &Integer) -> (Integer, Integer, Integer); // (g, s, t)，s*a+t*b=g
pub fn isqrt(n: &Integer) -> Integer;                        // floor(sqrt(n)), n>=0
pub fn exact_root(n: &Integer, k: u32) -> Option<Integer>;   // n 是完全 k 次幂时返回根（负 n 且 k 奇数返回负根）
pub fn perfect_power(n: &Integer) -> Option<(Integer, u32)>; // 最大的 k
pub fn is_probable_prime(n: &Integer) -> bool;               // <2^64 用确定性 Miller-Rabin 底 {2,3,5,7,11,13,17,19,23,29,31,37}；更大用 BPSW
pub fn factor_integer(n: &Integer, ctx_budget: &mut u64) -> Vec<(Integer, u32)>; // 试除到 10^4 + Pollard-Brent rho；预算耗尽返回部分分解，最后一项可能是合数（标 is_probable_prime=false）
pub fn extract_root_factor(n: &Integer, k: u32) -> (Integer, Integer); // n = a^k * b，b 不含 k 次幂因子，返回 (a,b)，用于 Sqrt[12] -> 2 Sqrt[3]

// crates/om-num/src/modp.rs —— u64 素数域
pub struct Fp { pub p: u64 }  // 方法：add, sub, mul, inv, pow（mul 用 u128 中间值）

// crates/om-num/src/rng.rs
pub struct SplitMix64(u64);   // new(seed), next_u64(), next_range(lo, hi)

// crates/om-num/src/ball.rs —— 严格的球算术（is_zero 与验证要用“可证明”的包围）
#[derive(Clone, Debug)]
pub struct Ball { pub mid: BigFloat, pub rad: BigFloat /*低精度，向上舍入*/, pub prec: u32 }
#[derive(Clone, Debug)]
pub struct CBall { pub re: Ball, pub im: Ball }
impl Ball {
    pub fn exact(q: &Rational, prec: u32) -> Ball;
    pub fn add/sub/mul/div(&self, o: &Ball) -> Ball;   // div 在分母包含 0 时返回 rad = +inf 的“全实数球”
    pub fn sqrt/exp/ln/sin/cos/atan(&self) -> Ball;     // 参数约简 + Taylor，截断误差显式加入 rad
    pub fn pi(prec: u32) -> Ball;                       // Machin 公式，按精度缓存
    pub fn contains_zero(&self) -> bool;
    pub fn excludes_zero(&self) -> bool;                // |mid| > rad —— 可证明非零
    pub fn to_f64(&self) -> f64;
}
// CBall 同样提供 add/sub/mul/div/pow_int/exp/ln/sqrt(主值)/sin/cos，由实数 Ball 组合
```
> **执行前核对：** `dashu-float 0.6.1` 已经提供哪些初等函数（exp/ln/sqrt/powf）？已提供的直接用于 `mid` 的计算，但误差半径 `rad` 仍须自己按“结果误差 ≤ 1 ulp(prec)”加上。sin/cos/atan 必须自己实现。

### 6.2 om-core：表达式

```rust
// crates/om-core/src/expr.rs
#[derive(Clone)]
pub struct Expr(std::sync::Arc<ExprNode>);
pub struct ExprNode { pub kind: ExprKind, pub hash: u64 }   // hash 构造时计算（FxHasher），结构相等则 hash 相等

pub enum ExprKind {
    Number(Number),
    Symbol(Symbol),
    String(Box<str>),
    Normal(Normal),
}
pub struct Normal { pub head: Expr, pub args: smallvec::SmallVec<[Expr; 3]> }

impl PartialEq for Expr  // 先比 Arc 指针，再比 hash，最后结构比较；数值比较是“结构相等”：1 与 1.0 不相等，1.0 与 1.0 相等
impl Eq, Hash (用 hash 字段), Debug (输出 FullForm)

impl Expr {
    pub fn int(i: i64) -> Expr; pub fn integer(i: Integer) -> Expr;
    pub fn rational(n: i64, d: i64) -> Expr;      // 会约分，d==0 panic（仅测试辅助）
    pub fn number(n: Number) -> Expr;             // 先 normalize
    pub fn real(f: f64) -> Expr;
    pub fn sym(s: Symbol) -> Expr;
    pub fn symbol(name: &str) -> Expr;            // 驻留
    pub fn string(s: &str) -> Expr;
    pub fn normal(head: Expr, args: impl IntoIterator<Item=Expr>) -> Expr; // **不做**任何化简（原始构造）
    pub fn call(head: Symbol, args: impl IntoIterator<Item=Expr>) -> Expr; // 同上，head 是符号
    // 访问器
    pub fn kind(&self) -> &ExprKind;
    pub fn as_number(&self) -> Option<&Number>; pub fn as_symbol(&self) -> Option<Symbol>;
    pub fn head(&self) -> Expr;                   // Number->Integer/Rational/Real/Complex 符号；Symbol->Symbol；String->String
    pub fn head_symbol(&self) -> Option<Symbol>;  // Normal 且 head 是符号时
    pub fn args(&self) -> &[Expr];                // 非 Normal 返回 &[]
    pub fn is_head(&self, s: Symbol) -> bool;
    pub fn is_zero(&self) -> bool; pub fn is_one(&self) -> bool; // 仅字面数
    pub fn free_of(&self, x: &Expr) -> bool;      // 子树中不含 x
    pub fn free_symbols(&self) -> std::collections::BTreeSet<Symbol>; // 排除内置常量（Pi, E, I, Infinity…）
    pub fn replace_all(&self, rules: &[(Expr, Expr)]) -> Expr; // 结构替换（非模式），结果**经过规范化**
    pub fn map_args(&self, f: impl FnMut(&Expr) -> Expr) -> Expr; // 重建时用规范构造器
    pub fn leaf_count(&self) -> usize;            // 复杂度度量
}
```

**Symbol 驻留（`crates/om-core/src/symbol.rs`）：** `#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)] pub struct Symbol(u32);`（`Ord` 按 id，**只**用于 BTreeMap 等容器；**规范排序与输出排序一律按名字比较**，因为驻留顺序在不同会话、线程中可能不同。哈希用固定种子的 `rustc_hash::FxHasher`，禁止 `RandomState`。） 全局驻留表 `static INTERNER: std::sync::OnceLock<std::sync::RwLock<Interner>>`（wasm 下 RwLock 可用，单线程无竞争）。`Symbol::intern(&str)`、`symbol.name() -> &'static str`（驻留字符串用 `Box::leak`，可以接受）。**内置符号**在 `crates/om-core/src/builtins.rs` 用宏一次性定义为常量，保证 id 固定：

```rust
define_builtins! {
    LIST = "List", PLUS = "Plus", TIMES = "Times", POWER = "Power", RULE = "Rule", RULE_DELAYED = "RuleDelayed",
    EQUAL = "Equal", UNEQUAL = "Unequal", LESS = "Less", LESS_EQUAL = "LessEqual", GREATER = "Greater",
    GREATER_EQUAL = "GreaterEqual", INEQUALITY = "Inequality", AND = "And", OR = "Or", NOT = "Not",
    TRUE = "True", FALSE = "False", NULL = "Null", PI = "Pi", E = "E", I = "I", INFINITY = "Infinity",
    COMPLEX_INFINITY = "ComplexInfinity", DIRECTED_INFINITY = "DirectedInfinity", INDETERMINATE = "Indeterminate",
    SIN = "Sin", COS = "Cos", TAN = "Tan", COT = "Cot", SEC = "Sec", CSC = "Csc",
    ARCSIN = "ArcSin", ARCCOS = "ArcCos", ARCTAN = "ArcTan", ARCCOT = "ArcCot", ARCSEC = "ArcSec", ARCCSC = "ArcCsc",
    SINH = "Sinh", COSH = "Cosh", TANH = "Tanh", COTH = "Coth", SECH = "Sech", CSCH = "Csch",
    ARCSINH = "ArcSinh", ARCCOSH = "ArcCosh", ARCTANH = "ArcTanh",
    EXP = "Exp", LOG = "Log", SQRT = "Sqrt", ABS = "Abs", SIGN = "Sign", RE = "Re", IM = "Im", ARG = "Arg",
    CONJUGATE = "Conjugate", FLOOR = "Floor", CEILING = "Ceiling", ROUND = "Round", PRODUCT_LOG = "ProductLog",
    FACTORIAL = "Factorial", BINOMIAL = "Binomial", MOD = "Mod", GCD = "GCD", LCM = "LCM",
    ROOT = "Root", FUNCTION = "Function", SLOT = "Slot", C = "C", CONDITIONAL_EXPRESSION = "ConditionalExpression",
    ELEMENT = "Element", NOT_ELEMENT = "NotElement", REALS = "Reals", INTEGERS = "Integers", COMPLEXES = "Complexes",
    RATIONALS = "Rationals", ALGEBRAICS = "Algebraics", PRIMES = "Primes", BOOLEANS = "Booleans",
    INTEGER = "Integer", RATIONAL = "Rational", REAL = "Real", COMPLEX = "Complex", SYMBOL = "Symbol", STRING = "String",
    SOLVE = "Solve", NSOLVE = "NSolve", FIND_ROOT = "FindRoot", REDUCE = "Reduce", ELIMINATE = "Eliminate",
    SOLVE_VALUES = "SolveValues", N = "N", SET = "Set", SET_DELAYED = "SetDelayed", CLEAR = "Clear",
    REPLACE_ALL = "ReplaceAll", COMPOUND_EXPRESSION = "CompoundExpression", PART = "Part", SPAN = "Span",
    PATTERN = "Pattern", BLANK = "Blank", BLANK_SEQUENCE = "BlankSequence", BLANK_NULL_SEQUENCE = "BlankNullSequence",
    CONDITION = "Condition", HOLD = "Hold", HOLD_FORM = "HoldForm", SEQUENCE = "Sequence", MESSAGE_NAME = "MessageName",
    EXPAND = "Expand", FACTOR = "Factor", TOGETHER = "Together", CANCEL = "Cancel", APART = "Apart", SIMPLIFY = "Simplify",
    FULL_SIMPLIFY = "FullSimplify", COLLECT = "Collect", D = "D", PLOT = "Plot", CONTOUR_PLOT = "ContourPlot",
    GRAPHICS = "Graphics", OUT = "Out", ASSUMPTIONS = "Assumptions", AUTOMATIC = "Automatic", ALL = "All", NONE = "None",
    METHOD = "Method", CUBICS = "Cubics", QUARTICS = "Quartics", VERIFY_SOLUTIONS = "VerifySolutions",
    MAX_EXTRA_CONDITIONS = "MaxExtraConditions", GENERATED_PARAMETERS = "GeneratedParameters",
    WORKING_PRECISION = "WorkingPrecision", MAX_ITERATIONS = "MaxIterations", ACCURACY_GOAL = "AccuracyGoal",
    PRECISION_GOAL = "PrecisionGoal", TABLE = "Table", RANGE = "Range", MAP = "Map", APPLY = "Apply", LENGTH = "Length",
    // …执行者可以追加，但已列出的**顺序不可改变**（id 由顺序决定）
}
```

### 6.3 om-core：规范构造器（Canonical constructors）

```rust
// crates/om-core/src/canon.rs  —— 详细规则见 8.1
pub fn add(terms: impl IntoIterator<Item = Expr>) -> Expr;   // Plus
pub fn mul(factors: impl IntoIterator<Item = Expr>) -> Expr; // Times
pub fn pow(base: Expr, exp: Expr) -> Expr;                   // Power
pub fn neg(x: Expr) -> Expr;                  // mul([-1, x])
pub fn sub(a: Expr, b: Expr) -> Expr;         // add([a, neg(b)])
pub fn div(a: Expr, b: Expr) -> Expr;         // mul([a, pow(b, -1)])
pub fn sqrt(x: Expr) -> Expr;                 // pow(x, 1/2)
pub fn exp(x: Expr) -> Expr;                  // pow(E, x)
pub fn func(head: Symbol, args: Vec<Expr>) -> Expr; // 对已知初等函数做**最小**自动化简（见 8.1.6），其余等同 Expr::call
pub fn canonicalize(e: &Expr) -> Expr;        // 自底向上用上述构造器重建整棵树
// 排序
pub fn canonical_cmp(a: &Expr, b: &Expr) -> std::cmp::Ordering;  // 见 8.1.1
```

**铁律：** 除解析器输出和 `Expr::normal` 以外，所有代码构造 Plus/Times/Power 时**必须**使用上述构造器，以保证所有 `Expr` 都是规范的，结构相等 ⟺ 规范相等。

### 6.4 om-core：求值上下文、中断与消息

```rust
// crates/om-core/src/ctx.rs
pub trait Clock: Send + Sync { fn now_ms(&self) -> f64; }   // native: Instant；wasm: js Date.now()（由 om-wasm 注入）
pub struct Interrupt {
    pub flag: std::sync::Arc<std::sync::atomic::AtomicBool>, // 外部置 true 即中止
    pub deadline_ms: Option<f64>, pub clock: Option<std::sync::Arc<dyn Clock>>,
    pub steps_left: std::cell::Cell<u64>,                      // 计算预算（默认 5e8 次 tick）
}
#[derive(Debug, Clone, thiserror::Error)]
pub enum Abort { #[error("interrupted")] Interrupted, #[error("time limit exceeded")] Timeout, #[error("step budget exceeded")] Budget }
impl Interrupt {
    /// 所有可能耗时的循环（多项式乘法外层循环、Gröbner 每对 S-多项式、Hensel 每步、求值器每次重写）都要调用
    #[inline] pub fn tick(&self) -> Result<(), Abort>;          // steps_left -=1；每 4096 次检查一次 flag 与时钟
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Message { pub symbol: String, pub tag: String, pub text: String, pub level: MsgLevel } // 例：Solve::svars
#[derive(Clone, Copy, Debug, serde::Serialize)] pub enum MsgLevel { Info, Warning, Error }
pub struct Messages(Vec<Message>);  // push(), take()
```

所有可能长时间运行的公共函数签名最后一个参数是 `ctx: &Interrupt`，返回 `Result<T, Abort>` 或 `Result<T, XxxError>`（其中 `XxxError: From<Abort>`）。

### 6.5 om-parse 与 om-format（接口）

```rust
// crates/om-parse/src/lib.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Dialect { Modern, Wolfram, Auto }
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)] pub struct Span { pub start: u32, pub end: u32 } // UTF-8 字节偏移
#[derive(Clone, Debug, serde::Serialize)]
pub struct Diagnostic { pub span: Span, pub severity: Severity, pub code: &'static str, pub message: String, pub fix: Option<Fix> }
#[derive(Clone, Debug, serde::Serialize)] pub struct Fix { pub span: Span, pub replacement: String, pub label: String }
#[derive(Clone, Copy, Debug, serde::Serialize)] pub enum Severity { Error, Warning, Hint }
pub struct ParseOutput {
    pub statements: Vec<Stmt>,           // 一个 cell 可以有多条语句（换行或 ; 分隔）
    pub diagnostics: Vec<Diagnostic>,
    pub dialect: Dialect,                // Auto 时给出检测结果
    pub tokens: Vec<(Span, TokenClass)>, // 供语法高亮
}
pub struct Stmt { pub expr: Expr, pub span: Span, pub suppress_output: bool } // 以 ; 结尾则 suppress
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum TokenClass { Number, Identifier, Builtin, Operator, Bracket, String, Comment, Keyword, Error }
pub fn parse(src: &str, dialect: Dialect) -> ParseOutput;       // 永不 panic；有错误时 statements 中含 `$Failed`/部分结果
pub fn parse_expr(src: &str, dialect: Dialect) -> Result<Expr, Vec<Diagnostic>>; // 便捷：必须恰好一条语句
pub fn detect_dialect(src: &str) -> Dialect;                    // 规则见 7.3

// crates/om-format/src/lib.rs
pub fn full_form(e: &Expr) -> String;          // Plus[1, Times[2, x]]
pub fn input_form(e: &Expr) -> String;         // Wolfram InputForm: 1 + 2*x
pub fn modern_form(e: &Expr) -> String;        // 现代方言：1 + 2x，sin(x)，[1, 2]
pub fn latex(e: &Expr) -> String;              // KaTeX 可渲染
pub fn unicode_form(e: &Expr) -> String;       // 终端：x² + 2x − 3，√2，π
pub struct FormatOptions { pub display_order: bool /*默认 true：降幂显示*/ }
```

### 6.6 om-solve 输出（接口；步骤数据模型详见 8.10）

```rust
// crates/om-solve/src/types.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Domain { Complexes, Reals, Integers, Rationals }
#[derive(Clone, Debug)]
pub struct SolveOptions {
    pub domain: Domain /*默认 Complexes*/,
    pub cubics: bool   /*默认 false：不可约且非二项/非回文的三次 → Root 对象；true → Cardano 根式*/,
    pub quartics: bool /*默认 false：同上，true → Ferrari*/,
    pub verify: VerifyMode /*默认 Auto*/, pub max_extra_conditions: MaxExtra /*默认 Zero*/,
    pub generated_parameter: Symbol /*默认 C*/, pub inverse_functions: bool /*默认 true*/,
    pub record_steps: bool /*kernel 默认 true*/, pub seed: u64 /*默认 0x0A5E_ED00_0000_0001，确定性*/,
}
#[derive(Clone, Debug)] pub enum VerifyMode { Auto, Always, Never }
#[derive(Clone, Debug)] pub enum MaxExtra { Zero, All, Count(u32) }
/// 一个解 = 一组变量赋值 + 可选条件 + 生成参数
#[derive(Clone, Debug)]
pub struct Solution {
    pub rules: Vec<(Expr /*变量*/, Expr /*值*/)>,   // 自由变量不出现
    pub condition: Option<Expr>,                    // ConditionalExpression 的条件（不含 C[k] ∈ Integers）
    pub constants: Vec<(Expr /*C[k]*/, Domain)>,    // 生成参数及其所属集合
    pub multiplicity: u32,                          // Wolfram 输出时同一规则重复 m 次
    pub verification: Verification,
    pub numeric: Option<Vec<(f64, f64)>>,           // 每个变量的复数近似值（排序、可视化用）
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum Verification { Exact, ByConstruction, Numeric { digits: u32 }, Unverified }
#[derive(Clone, Debug)]
pub enum SolutionSet {
    Finite(Vec<Solution>),     // {} 即 Finite(vec![])
    All,                       // {{}}：对任意值成立
    Region { cond: Expr, intervals: Vec<Interval> }, // Reduce/不等式：布尔条件（如 -2 < x < 2）+ 供 UI 的区间
    Unevaluated,               // 无法求解：Solve 内置函数原样返回输入
}
pub struct Interval { pub lo: Bound, pub hi: Bound }
pub enum Bound { NegInf, PosInf, Closed(Expr), Open(Expr) }
pub struct SolveOutcome { pub set: SolutionSet, pub steps: Option<Steps>, pub messages: Vec<Message> }
pub fn solve(eqs: &Expr, vars: &[Expr], opts: &SolveOptions, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn nsolve(eqs: &Expr, vars: &[Expr], precision: Precision, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn find_root(eqs: &Expr, starts: &[(Expr, Number)], opts: &FindRootOptions, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn reduce(expr: &Expr, vars: &[Expr], domain: Domain, ctx: &Interrupt) -> Result<SolveOutcome, SolveError>;
pub fn eliminate(eqs: &Expr, elim: &[Expr], ctx: &Interrupt) -> Result<Expr, SolveError>;
impl SolutionSet { pub fn to_expr(&self) -> Expr; }   // Wolfram 形状：{{x -> 1}, {x -> 2}}
#[derive(Debug, thiserror::Error)]
pub enum SolveError { #[error(transparent)] Abort(#[from] Abort), #[error("unsupported: {0}")] Unsupported(String), #[error("invalid input: {0}")] Invalid(String) }
```

`Unsupported` 不是崩溃：`Solve` 内置函数捕获它，发出消息 `Solve::nsmet`（"This system cannot be solved with the methods available to Solve."），并**原样返回未求值的 `Solve[...]`**，与 Mathematica 行为一致。

<!-- Extracted from docs/plan/PLAN.md sections [9]. PLAN.md is authoritative; keep in sync. -->

## 9. 求值器规格（om-eval）

### 9.1 核心结构

```rust
// crates/om-eval/src/lib.rs
pub struct Evaluator {
    pub defs: Definitions,                 // 用户定义：OwnValues / DownValues
    builtins: &'static BuiltinTable,       // 进程级只读表（OnceLock 初始化）
    pub messages: Messages,
    pub history: Vec<(Expr /*In*/, Expr /*Out*/)>,
    pub last_steps: Option<om_solve::Steps>, // 最近一次 Solve 类调用的步骤（kernel 取走）
    pub settings: EvalSettings,            // iteration_limit=4096, recursion_limit=1024, record_steps=true
    depth: u32,
}
pub struct Definitions {
    own: BTreeMap<Symbol, Expr>,                         // x = 5
    down: BTreeMap<Symbol, Vec<Rule>>,                   // f[x_] := x^2（按插入顺序；同 LHS 替换旧规则）
    attrs: BTreeMap<Symbol, Attributes>,                 // 用户设置的属性（v1 仅 Protected 检查用）
}
pub struct Rule { pub lhs: Expr, pub rhs: Expr, pub delayed: bool }
bitflags-like: pub struct Attributes(u16) // HOLD_ALL, HOLD_FIRST, HOLD_REST, LISTABLE, NUMERIC_FUNCTION, PROTECTED, ORDERLESS, FLAT, ONE_IDENTITY（后三个由 canon 实现，这里仅作标记）
// 不引入 bitflags crate：手写 const 与 contains()

pub type BuiltinFn = fn(&mut Evaluator, &[Expr], &Interrupt) -> Result<Option<Expr>, EvalError>;
pub struct BuiltinSpec { pub symbol: Symbol, pub f: BuiltinFn, pub attrs: Attributes, pub arity: Arity, pub doc: DocEntry }
pub enum Arity { Exactly(u8), Range(u8, u8), AtLeast(u8), Any }
#[derive(serde::Serialize, Clone)]
pub struct DocEntry { pub name: &'static str, pub modern: &'static str /*"solve(eqs, vars, domain: …)"*/,
    pub wolfram: &'static str /*"Solve[eqs, vars, dom]"*/, pub summary_zh: &'static str, pub summary_en: &'static str,
    pub examples: &'static [&'static str], pub category: &'static str /*"Solving" | "Algebra" | …*/ }

impl Evaluator {
    pub fn new() -> Self;
    pub fn evaluate(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError>;
    pub fn evaluate_statement(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError>; // 记录 history、Out[n]
    pub fn doc(sym: Symbol) -> Option<&'static DocEntry>; pub fn all_docs() -> impl Iterator<Item=&'static DocEntry>;
    pub fn fork_readonly(&self) -> Evaluator;   // 复制 defs，供 LLM 工具调用（禁止 Set）
}
#[derive(Debug, thiserror::Error)]
pub enum EvalError { #[error(transparent)] Abort(#[from] Abort), #[error("recursion limit {0} exceeded")] Recursion(u32), #[error("{0}")] Other(String) }
```

### 9.2 求值算法（每一步都要有对应测试）
`evaluate(e)`：
1. `ctx.tick()?`；`depth += 1`，超过 `recursion_limit` 返回 `EvalError::Recursion`（kernel 显示为消息 `$RecursionLimit::reclim`）。
2. **原子**：Number/String 原样返回；Symbol：若 `defs.own` 有值则返回 `evaluate(值)`，否则原样返回。
3. **Normal**：
   a. `h = evaluate(head)`。
   b. 取 `h` 的属性（内置表 ∪ 用户属性）。按 HoldAll/HoldFirst/HoldRest 决定哪些参数**不**求值；其余参数依次 `evaluate`。
   c. 展开参数中的 `Sequence[...]`（除非 HoldAll）。
   d. Listable 且某参数是 List：按 Mathematica 规则逐元素线程化（长度不一致 → 消息 `Thread::tdlen`，原样返回）。
   e. 用 `canon::rebuild(h, args)` 重建（对 Plus/Times/Power 调用规范构造器，其余直接构造）。
   f. 若 `h` 是符号且 `defs.down[h]` 非空：按顺序对每条规则调用 `pattern::match_rule(rule.lhs, expr)`，首个匹配 → `rhs` 代入绑定（`pattern::substitute`）→ `evaluate` 该结果并返回。
   g. 若 `h` 是内置符号：调用 `f(self, args, ctx)`。`Ok(Some(r))` 且 `r != expr` → 返回 `evaluate(r)`（循环计数 +1，超过 `iteration_limit` 发出 `$IterationLimit::itlim` 并返回 `Hold[r]`）；`Ok(None)` → 返回当前 expr。
   h. 否则原样返回。
4. **参数个数检查**：内置函数调用前检查 `arity`，不符合 → 消息 `F::argx`/`F::argrx`（文案与 Mathematica 相同，如 `Solve::argrx: Solve called with 4 arguments; between 1 and 3 arguments are expected.`），原样返回。

**模式匹配（`crates/om-eval/src/pattern.rs`）：** 支持 `Blank[]`、`Blank[h]`、`Pattern[name, p]`（同名绑定必须一致）、`Condition[p, test]`（test 代入绑定后求值为 `True`）、`BlankSequence`/`BlankNullSequence`（用回溯尝试所有长度）、字面量结构匹配。**不支持** Orderless 交换匹配、`Optional`、`Alternatives`、`PatternTest`（v1 遇到时发出 `Pattern::unsup` 警告并视为不匹配）。

### 9.3 内置函数清单（分里程碑实现；每个函数都要登记 DocEntry）
- **M4（基础）：** `Plus Times Power Subtract Divide Minus Sqrt Exp Log Abs Sign Re Im Conjugate Arg Floor Ceiling Round Mod Quotient GCD LCM Factorial Binomial FactorInteger PrimeQ Numerator Denominator N`、三角/反三角/双曲/反双曲（精确特殊值表见 8.1.6）、`List Part Length First Last Rest Append Table Range Map Apply Sum(有限) Product(有限)`、`Equal Unequal Less LessEqual Greater GreaterEqual And Or Not SameQ`、`Rule RuleDelayed ReplaceAll ReplaceRepeated Set SetDelayed Unset Clear CompoundExpression Hold HoldForm Out`、`Element`（仅保持）、`Function Slot`（纯函数应用）。
- **M10.1（代数）：** `Expand Factor Together Cancel Apart(仅 Q 上一元) Simplify FullSimplify(=Simplify 加更多变换) Collect Coefficient CoefficientList Exponent PolynomialQ PolynomialGCD PolynomialLCM PolynomialQuotient PolynomialRemainder Resultant Discriminant Variables D RootReduce ToRadicals`。
- **M10.2（求解）：** `Solve NSolve FindRoot Reduce Eliminate SolveValues NSolveValues Roots(=Solve 后转 Or 形式) Root ConditionalExpression`。
- **M11.5（绘图，kernel 侧）：** `Plot ContourPlot` 仅返回 HoldAll 的原样表达式，kernel 识别后采样（见 10.5）。

### 9.4 数值求值 `N`
- `N[e]` → 机器精度：递归把精确数转 f64（复数用 `(f64, f64)`），已知函数调用 `om_num::elem` 的 f64 实现；溢出或非有限值 → 自动切换到 BigFloat（精度 64 bit）重算。
- `N[e, d]`（d 为十进制位数）：工作精度 `bits = ceil(d·3.3219) + 32`；计算两次（`bits` 和 `2·bits`），若两次结果在 d 位上一致则返回，否则把精度加倍重试，最多 4 轮；最终结果四舍五入到 `ceil(d·3.3219)` bit。
- `om_num::elem`（任意精度初等函数，自研，放 `crates/om-num/src/elem/`）：`pi(bits)`（Machin 公式 + 缓存）、`exp`（参数约简 + Taylor）、`ln`（AGM 或牛顿迭代 exp）、`sin/cos`（约简到 [−π/4, π/4] + Taylor）、`atan`（约简 + Taylor）、其余由这些组合（`tan = sin/cos`，`asin(x) = atan(x/√(1−x²))`，双曲由 exp 组合）；复数版本由实数版本组合；`lambert_w0/w_m1`（Halley 迭代）。**测试：** 与 f64 标准库对比 1e-15；`pi(3400 bits)` 的前 1000 位十进制与常量字符串比对（常量放测试文件中）。
  > 如果 `dashu-float` 已提供 `exp`/`ln`/`sqrt`，直接调用并用上述测试验证。

# Input language

OpenMath parses modern and Wolfram syntax into the same expression tree. Modern calls use parentheses and lists use square brackets; Wolfram calls use square brackets and lists use braces. Modern `let x = value` assigns a value, while `x = value` expresses equality. Wolfram uses `x = value` for assignment and `x == value` for equality. Full syntax is specified in [PLAN §7](plan/PLAN.md#7-输入语言规格).

M10.1 exposes the following algebra functions through the evaluator. These examples are executable in the indicated dialect.

| Operation | Modern | Wolfram | Result |
|---|---|---|---|
| Expansion | `Expand((x+1)^3)` | `Expand[(x+1)^3]` | `x^3+3x^2+3x+1` |
| Factoring | `Factor(x^2-y^2)` | `Factor[x^2-y^2]` | `(x-y)(x+y)` |
| Cancellation | `Cancel((x^2-1)/(x-1))` | `Cancel[(x^2-1)/(x-1)]` | `x+1` |
| Coefficients | `CoefficientList(a*x^2+b*x+c,x)` | `CoefficientList[a x^2+b x+c,x]` | `[c,b,a]` / `{c,b,a}` |
| Differentiation | `D(sin(x^2),x)` | `D[Sin[x^2],x]` | `2x Cos[x^2]` |
| Partial fractions | `Apart(1/(x*(x+1)),x)` | `Apart[1/(x(x+1)),x]` | `1/x-1/(x+1)` |

`Together`, `Simplify`, `FullSimplify`, `Collect`, `Coefficient`, `Exponent`, `PolynomialQ`, `PolynomialGCD`, `PolynomialLCM`, `PolynomialQuotient`, `PolynomialRemainder`, `Resultant`, `Discriminant`, `Variables`, `RootReduce` and `ToRadicals` are also registered, with bilingual help and checked argument counts. Expand/Factor/Together/Cancel/RootReduce/ToRadicals thread over lists. CoefficientList accepts a variable list and returns a rectangular coefficient tensor in ascending powers. Collect optionally applies a third-argument function to each coefficient.

Polynomial queries accept symbolic coefficients independent of the requested axes, including denominators independent of those axes. Requested polynomial degrees are bounded to 4096; coefficient tensors have at most one million entries and sixteen axes. Exact GCD/LCM use Q polynomials and retain numeric content. Quotient/remainder and resultants support rational parameter coefficients. Apart accepts only a Q rational function in one variable, supplied explicitly or inferred when there is exactly one free symbol. Unsupported inputs stay symbolic with an algebra diagnostic; cancellation and execution budgets propagate.

D supports repeated orders (`D[x^4,{x,2}]`), mixed variables (`D[x^2 y^3,x,y]`), list outputs and elementary chain rules. Unknown function derivatives remain formal `Derivative[...]` expressions. It accepts at most 64 differentiation specifications and 4096 total orders. Simplify supports explicit positive-symbol assumptions, such as `Simplify[Sqrt[x^2],x>0]`, and preserves principal branches without those assumptions. FullSimplify also considers certified Root/radical conversions and keeps the expression with lower weighted complexity; a Root can remain cheaper than a radical. Use ToRadicals to request the radical form explicitly.

RootReduce produces the certified minimal polynomial and correct one-based root index for supported exact algebraic values. ToRadicals traverses expression heads and arguments, uses enabled cubic/quartic formulas and existing special polynomial reductions, and maps the selected branch through the solver’s certified minimal-polynomial root numbering. Unsupported parametric or nonradical Root objects remain intact. Solve-class Rust APIs are implemented, while their evaluator registration is the next M10.2 task.

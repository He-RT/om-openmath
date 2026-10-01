//! Solver callbacks hold source syntax until raw normalization has captured poles.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {
        ($f:ident,$name:literal,$arity:expr,$sig:literal,$zh:literal,$en:literal,$example:literal) => {{
            fn $f(
                ev: &mut Evaluator,
                args: &[Expr],
                ctx: &Interrupt,
            ) -> Result<Option<Expr>, EvalError> {
                crate::solver::dispatch(ev, $name, args, ctx)
            }
            specs.insert(
                $name,
                BuiltinSpec {
                    symbol: Symbol::intern($name),
                    f: $f,
                    attrs: A::HOLD_ALL | A::PROTECTED,
                    arity: $arity,
                    doc: DocEntry {
                        name: $name,
                        modern: concat!($name, "(", $sig, ")"),
                        wolfram: concat!($name, "[", $sig, "]"),
                        summary_zh: $zh,
                        summary_en: $en,
                        examples: &[$example],
                        category: "Solving",
                    },
                },
            );
        }};
    }
    entry!(
        solve,
        "Solve",
        Arity::AtLeast(1),
        "eqs, vars, domain, options",
        "求解原始方程并保留定义域限制。",
        "Solve equations while preserving source restrictions.",
        "Solve[x^2==2,x]"
    );
    entry!(
        nsolve,
        "NSolve",
        Arity::AtLeast(1),
        "eqs, vars, options",
        "认证有限代数解的数值近似。",
        "Approximate complete certified algebraic solutions.",
        "NSolve[x^2==2,x,WorkingPrecision->50]"
    );
    entry!(
        find_root,
        "FindRoot",
        Arity::AtLeast(2),
        "eqs, starts, options",
        "从局部起点或实数括区间寻找数值根。",
        "Find a verified local root from starts or a real bracket.",
        "FindRoot[x^2==2,{x,1}]"
    );
    entry!(
        reduce,
        "Reduce",
        Arity::AtLeast(1),
        "expr, vars, domain, options",
        "把方程或一元有理不等式化为布尔条件。",
        "Reduce equations or univariate rational inequalities to Boolean conditions.",
        "Reduce[x^2<4,x]"
    );
    entry!(
        eliminate,
        "Eliminate",
        Arity::Exactly(2),
        "eqs, vars",
        "通过多项式消元理想消去变量。",
        "Eliminate variables through a polynomial elimination ideal.",
        "Eliminate[{x==y+1,y==2z},y]"
    );
    entry!(
        solve_values,
        "SolveValues",
        Arity::AtLeast(1),
        "eqs, vars, domain, options",
        "按请求变量顺序输出精确解值。",
        "Return exact solution values in requested axis order.",
        "SolveValues[x^2==2,x]"
    );
    entry!(
        nsolve_values,
        "NSolveValues",
        Arity::AtLeast(1),
        "eqs, vars, options",
        "按请求变量顺序输出数值解值。",
        "Return numerical solution values in requested axis order.",
        "NSolveValues[x^2==2,x]"
    );
    entry!(
        roots,
        "Roots",
        Arity::AtLeast(1),
        "eqs, vars, domain, options",
        "把完整解集合转换为布尔析取。",
        "Convert the complete solution set into Boolean alternatives.",
        "Roots[x^2==1,x]"
    );
    entry!(
        root,
        "Root",
        Arity::Exactly(2),
        "function, k",
        "表示具有精确编号的代数根。",
        "Represent an algebraic root with an exact certified index.",
        "Root[#^5-# +1&,1]"
    );
    entry!(
        conditional,
        "ConditionalExpression",
        Arity::Exactly(2),
        "expr, condition",
        "仅在条件成立时求值；假条件返回 Undefined。",
        "Evaluate under a condition; a false condition returns Undefined.",
        "ConditionalExpression[2+2,True]"
    );
}

//! Algebra callbacks and searchable bilingual help share one immutable registry.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {
        ($f:ident,$name:literal,$arity:expr,$attrs:expr,$sig:literal,$zh:literal,$en:literal,$example:literal) => {{
            fn $f(
                ev: &mut Evaluator,
                args: &[Expr],
                ctx: &Interrupt,
            ) -> Result<Option<Expr>, EvalError> {
                crate::algebra::dispatch(ev, $name, args, ctx)
            }
            specs.insert(
                $name,
                BuiltinSpec {
                    symbol: Symbol::intern($name),
                    f: $f,
                    attrs: $attrs | A::PROTECTED,
                    arity: $arity,
                    doc: DocEntry {
                        name: $name,
                        modern: concat!($name, "(", $sig, ")"),
                        wolfram: concat!($name, "[", $sig, "]"),
                        summary_zh: $zh,
                        summary_en: $en,
                        examples: &[$example],
                        category: "Algebra",
                    },
                },
            );
        }};
    }
    entry!(
        expand,
        "Expand",
        Arity::Exactly(1),
        A::LISTABLE,
        "expr",
        "展开乘积与非负整数幂。",
        "Expand products and nonnegative integer powers.",
        "Expand[(x+1)^3]"
    );
    entry!(
        factor,
        "Factor",
        Arity::Exactly(1),
        A::LISTABLE,
        "expr",
        "在有理数域分解多项式。",
        "Factor polynomials over the rationals.",
        "Factor[x^2-y^2]"
    );
    entry!(
        together,
        "Together",
        Arity::Exactly(1),
        A::LISTABLE,
        "expr",
        "合并为公共分母。",
        "Combine terms over a common denominator.",
        "Together[1/x+1/y]"
    );
    entry!(
        cancel,
        "Cancel",
        Arity::Exactly(1),
        A::LISTABLE,
        "expr",
        "消去公共多项式因子。",
        "Cancel common polynomial factors.",
        "Cancel[(x^2-1)/(x-1)]"
    );
    entry!(
        apart,
        "Apart",
        Arity::Range(1, 2),
        A::default(),
        "expr, x",
        "精确分解一元有理函数的部分分式。",
        "Decompose a univariate rational function into partial fractions.",
        "Apart[1/(x(x+1)),x]"
    );
    entry!(
        simplify,
        "Simplify",
        Arity::Range(1, 2),
        A::default(),
        "expr, assumptions",
        "使用保持主值分支的变换化简。",
        "Simplify with branch-preserving transformations.",
        "Simplify[Sin[x]^2+Cos[x]^2]"
    );
    entry!(
        full_simplify,
        "FullSimplify",
        Arity::Range(1, 2),
        A::default(),
        "expr, assumptions",
        "增加认证代数数与根式候选以化简。",
        "Add certified algebraic and radical simplification candidates.",
        "FullSimplify[Sqrt[2]^2]"
    );
    entry!(
        collect,
        "Collect",
        Arity::Range(2, 3),
        A::default(),
        "expr, vars, h",
        "按变量幂合并系数，可对系数应用函数。",
        "Collect powers, optionally applying a function to coefficients.",
        "Collect[a x^2+b x^2+x,x]"
    );
    entry!(
        coefficient,
        "Coefficient",
        Arity::Range(2, 3),
        A::default(),
        "expr, x, n",
        "取指定变量幂的系数。",
        "Extract the coefficient of a specified power.",
        "Coefficient[(x+y)^3,x,2]"
    );
    entry!(
        coefficient_list,
        "CoefficientList",
        Arity::Exactly(2),
        A::default(),
        "expr, vars",
        "按升幂输出系数数组。",
        "Return coefficient arrays in ascending power order.",
        "CoefficientList[a x^2+b x+c,x]"
    );
    entry!(
        exponent,
        "Exponent",
        Arity::Exactly(2),
        A::default(),
        "expr, x",
        "取多项式变量的最高次数。",
        "Return the highest polynomial exponent.",
        "Exponent[(x+1)^5,x]"
    );
    entry!(
        polynomial_q,
        "PolynomialQ",
        Arity::Exactly(2),
        A::default(),
        "expr, vars",
        "检查是否为指定变量的多项式。",
        "Test polynomial dependence on the requested variables.",
        "PolynomialQ[x^2+Sin[a]x,x]"
    );
    entry!(
        polynomial_gcd,
        "PolynomialGCD",
        Arity::AtLeast(1),
        A::default(),
        "poly, ...",
        "计算有理多项式的精确最大公因式。",
        "Compute the exact GCD of rational polynomials.",
        "PolynomialGCD[x^2-1,(x+1)^2]"
    );
    entry!(
        polynomial_lcm,
        "PolynomialLCM",
        Arity::AtLeast(1),
        A::default(),
        "poly, ...",
        "计算有理多项式的精确最小公倍式。",
        "Compute the exact LCM of rational polynomials.",
        "PolynomialLCM[x^2-1,(x+1)^2]"
    );
    entry!(
        polynomial_quotient,
        "PolynomialQuotient",
        Arity::Exactly(3),
        A::default(),
        "f, g, x",
        "计算指定变量的多项式商。",
        "Compute the polynomial quotient in a variable.",
        "PolynomialQuotient[x^3-2x^2-4,x-3,x]"
    );
    entry!(
        polynomial_remainder,
        "PolynomialRemainder",
        Arity::Exactly(3),
        A::default(),
        "f, g, x",
        "计算指定变量的多项式余式。",
        "Compute the polynomial remainder in a variable.",
        "PolynomialRemainder[x^3-2x^2-4,x-3,x]"
    );
    entry!(
        resultant,
        "Resultant",
        Arity::Exactly(3),
        A::default(),
        "f, g, x",
        "消元得到精确多项式结式。",
        "Eliminate a variable with an exact polynomial resultant.",
        "Resultant[x^2-a,x-b,x]"
    );
    entry!(
        discriminant,
        "Discriminant",
        Arity::Exactly(2),
        A::default(),
        "f, x",
        "计算多项式判别式。",
        "Compute the polynomial discriminant.",
        "Discriminant[a x^2+b x+c,x]"
    );
    entry!(
        variables,
        "Variables",
        Arity::Exactly(1),
        A::default(),
        "expr",
        "列出规范排序的代数生成元。",
        "List algebraic generators in canonical order.",
        "Variables[x^2+x y+y^2]"
    );
    entry!(
        differentiate,
        "D",
        Arity::AtLeast(2),
        A::default(),
        "expr, x, ...",
        "按链式法则求重复或混合偏导数。",
        "Compute repeated and mixed partial derivatives by the chain rule.",
        "D[Sin[x^2],x]"
    );
    entry!(
        root_reduce,
        "RootReduce",
        Arity::Exactly(1),
        A::LISTABLE,
        "expr",
        "把精确代数数转换为最小多项式根。",
        "Convert exact algebraic numbers to minimal-polynomial roots.",
        "RootReduce[Sqrt[2]+Sqrt[3]]"
    );
    entry!(
        to_radicals,
        "ToRadicals",
        Arity::Exactly(1),
        A::LISTABLE,
        "expr",
        "认证地把可处理的 Root 子式转换为根式。",
        "Convert supported Root subexpressions to certified radicals.",
        "ToRadicals[Root[#^2-2&,1]]"
    );
}

//! Only implemented scalar functions are registered with executable examples.

use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;

pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    macro_rules! entry {
        ($f:ident,$name:literal,$arity:expr,$attrs:expr,$zh:literal,$en:literal,$examples:expr) => {{
            fn $f(
                ev: &mut Evaluator,
                args: &[Expr],
                ctx: &Interrupt,
            ) -> Result<Option<Expr>, EvalError> {
                crate::scalar::dispatch(ev, Symbol::intern($name), args, ctx)
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
                        modern: concat!($name, "(…)"),
                        wolfram: concat!($name, "[…]"),
                        summary_zh: $zh,
                        summary_en: $en,
                        examples: $examples,
                        category: "Mathematics",
                    },
                },
            );
        }};
    }
    let numeric = A::LISTABLE | A::NUMERIC_FUNCTION;
    entry!(
        plus,
        "Plus",
        Arity::Any,
        numeric | A::ORDERLESS | A::FLAT | A::ONE_IDENTITY,
        "相加并合并同类项。",
        "Add arguments and combine like terms.",
        &["1 + 2", "x + x"]
    );
    entry!(
        times,
        "Times",
        Arity::Any,
        numeric | A::ORDERLESS | A::FLAT | A::ONE_IDENTITY,
        "相乘并合并幂。",
        "Multiply arguments and combine powers.",
        &["2 * 3", "x * x"]
    );
    entry!(
        power,
        "Power",
        Arity::Exactly(2),
        numeric,
        "计算主值幂。",
        "Compute a principal power.",
        &["2^3", "x^2"]
    );
    entry!(
        subtract,
        "Subtract",
        Arity::Exactly(2),
        numeric,
        "用规范和式相减。",
        "Subtract using a canonical sum.",
        &["Subtract[5, 2]", "Subtract[x, 1]"]
    );
    entry!(
        divide,
        "Divide",
        Arity::Exactly(2),
        numeric,
        "精确相除或构造倒数。",
        "Divide exactly or construct an inverse.",
        &["Divide[6, 4]", "Divide[x, 2]"]
    );
    entry!(
        minus,
        "Minus",
        Arity::Exactly(1),
        numeric,
        "取负。",
        "Negate an expression.",
        &["Minus[3]", "Minus[x]"]
    );
    entry!(
        sqrt,
        "Sqrt",
        Arity::Exactly(1),
        numeric,
        "取主值平方根。",
        "Take a principal square root.",
        &["Sqrt[12]", "Sqrt[4]"]
    );
    entry!(
        exp,
        "Exp",
        Arity::Exactly(1),
        numeric,
        "以 E 为底取幂。",
        "Raise E to a power.",
        &["Exp[0]", "Exp[x]"]
    );
    entry!(
        log,
        "Log",
        Arity::Range(1, 2),
        numeric,
        "主值自然对数或指定底数的对数。",
        "Compute a principal natural or base logarithm.",
        &["Log[1]", "Log[2, 8]"]
    );
    entry!(
        abs,
        "Abs",
        Arity::Exactly(1),
        numeric,
        "取数值模或简化符号绝对值。",
        "Take a numeric magnitude or simplify a symbolic absolute value.",
        &["Abs[-3]", "Abs[3 + 4I]"]
    );
    entry!(
        sign,
        "Sign",
        Arity::Exactly(1),
        numeric,
        "取实数符号或复数单位方向。",
        "Return a real sign or complex unit direction.",
        &["Sign[-3]", "Sign[0]"]
    );
    entry!(
        re,
        "Re",
        Arity::Exactly(1),
        numeric,
        "取实部。",
        "Return the real component.",
        &["Re[3 + 4I]", "Re[2]"]
    );
    entry!(
        im,
        "Im",
        Arity::Exactly(1),
        numeric,
        "取虚部。",
        "Return the imaginary component.",
        &["Im[3 + 4I]", "Im[2]"]
    );
    entry!(
        conj,
        "Conjugate",
        Arity::Exactly(1),
        numeric,
        "取复共轭。",
        "Return the complex conjugate.",
        &["Conjugate[3 + 4I]", "Conjugate[2]"]
    );
    entry!(
        arg,
        "Arg",
        Arity::Exactly(1),
        numeric,
        "取主值幅角。",
        "Return the principal argument.",
        &["Arg[-3]", "Arg[I]"]
    );
    entry!(
        floor,
        "Floor",
        Arity::Range(1, 2),
        numeric,
        "向下取整或取指定单位的下界。",
        "Round down to an integer or multiple.",
        &["Floor[-3/2]", "Floor[2.9]"]
    );
    entry!(
        ceil,
        "Ceiling",
        Arity::Range(1, 2),
        numeric,
        "向上取整或取指定单位的上界。",
        "Round up to an integer or multiple.",
        &["Ceiling[-3/2]", "Ceiling[2.1]"]
    );
    entry!(
        round,
        "Round",
        Arity::Range(1, 2),
        numeric,
        "取最近整数或单位，中点取偶数。",
        "Round to the nearest integer or multiple, with ties to even.",
        &["Round[5/2]", "Round[7/2]"]
    );
    entry!(
        modulo,
        "Mod",
        Arity::Range(2, 3),
        numeric,
        "按除数方向取模，支持偏移。",
        "Take the floor-based remainder with an optional offset.",
        &["Mod[-7, 3]", "Mod[7, -3]"]
    );
    entry!(
        quotient,
        "Quotient",
        Arity::Range(2, 3),
        numeric,
        "返回相除的整数商，支持偏移。",
        "Return a floor-based quotient with an optional offset.",
        &["Quotient[-7, 3]", "Quotient[7, 3]"]
    );
    entry!(
        gcd,
        "GCD",
        Arity::Any,
        numeric,
        "计算非负整数或精确有理数最大公约数。",
        "Compute a nonnegative integer or exact rational greatest common divisor.",
        &["GCD[12, 18]", "GCD[]"]
    );
    entry!(
        lcm,
        "LCM",
        Arity::Any,
        numeric,
        "计算整数或精确有理数最小公倍数。",
        "Compute an integer or exact rational least common multiple.",
        &["LCM[4, 6]", "LCM[]"]
    );
    entry!(
        factorial,
        "Factorial",
        Arity::Exactly(1),
        numeric,
        "计算非负整数阶乘。",
        "Compute a nonnegative integer factorial.",
        &["Factorial[5]", "Factorial[0]"]
    );
    entry!(
        binomial,
        "Binomial",
        Arity::Exactly(2),
        numeric,
        "计算整数二项式系数。",
        "Compute an integer binomial coefficient.",
        &["Binomial[5, 2]", "Binomial[-3, 2]"]
    );
    entry!(
        factor_integer,
        "FactorInteger",
        Arity::Exactly(1),
        A::LISTABLE,
        "返回整数的素因子和重数。",
        "Return prime integer factors and multiplicities.",
        &["FactorInteger[-12]", "FactorInteger[1]"]
    );
    entry!(
        prime,
        "PrimeQ",
        Arity::Exactly(1),
        A::LISTABLE,
        "检测显式整数是否为素数。",
        "Test an explicit integer for primality.",
        &["PrimeQ[97]", "PrimeQ[91]"]
    );
    entry!(
        numerator,
        "Numerator",
        Arity::Exactly(1),
        A::LISTABLE,
        "提取数或乘积的分子。",
        "Extract the numerator of a number or product.",
        &["Numerator[2/3]", "Numerator[4]"]
    );
    entry!(
        denominator,
        "Denominator",
        Arity::Exactly(1),
        A::LISTABLE,
        "提取数或乘积的分母。",
        "Extract the denominator of a number or product.",
        &["Denominator[2/3]", "Denominator[4]"]
    );
    entry!(
        equal,
        "Equal",
        Arity::Any,
        A::default(),
        "判断相等，保留未知符号关系。",
        "Test equality and retain unknown symbolic relations.",
        &["1 == 1.", "1 == 2"]
    );
    entry!(
        unequal,
        "Unequal",
        Arity::Any,
        A::default(),
        "判断所有参数两两不等。",
        "Test whether every pair of arguments differs.",
        &["Unequal[1, 2, 3]", "Unequal[1, 2, 1]"]
    );
    entry!(
        less,
        "Less",
        Arity::Any,
        A::default(),
        "判断严格递增关系。",
        "Test strictly increasing values.",
        &["1 < 2 < 3", "3 < 2"]
    );
    entry!(
        less_equal,
        "LessEqual",
        Arity::Any,
        A::default(),
        "判断非递减关系。",
        "Test nondecreasing values.",
        &["1 <= 1", "2 <= 1"]
    );
    entry!(
        greater,
        "Greater",
        Arity::Any,
        A::default(),
        "判断严格递减关系。",
        "Test strictly decreasing values.",
        &["3 > 2 > 1", "1 > 2"]
    );
    entry!(
        greater_equal,
        "GreaterEqual",
        Arity::Any,
        A::default(),
        "判断非递增关系。",
        "Test nonincreasing values.",
        &["1 >= 1", "1 >= 2"]
    );
    entry!(
        inequality,
        "Inequality",
        Arity::AtLeast(3),
        A::default(),
        "连接混合比较链。",
        "Combine a chain of mixed comparisons.",
        &["0 < 1 <= 2", "0 < x <= 2"]
    );
    entry!(
        and,
        "And",
        Arity::Any,
        A::HOLD_ALL,
        "短路逻辑与。",
        "Compute a short-circuit logical conjunction.",
        &["True && True", "False && x"]
    );
    entry!(
        or,
        "Or",
        Arity::Any,
        A::HOLD_ALL,
        "短路逻辑或。",
        "Compute a short-circuit logical disjunction.",
        &["False || False", "True || x"]
    );
    entry!(
        not,
        "Not",
        Arity::Exactly(1),
        A::default(),
        "逻辑非。",
        "Negate a Boolean value.",
        &["Not[True]", "Not[False]"]
    );
    entry!(
        same,
        "SameQ",
        Arity::Any,
        A::default(),
        "判断结构和数值类别相同。",
        "Test structural equality including numeric categories.",
        &["SameQ[1, 1]", "SameQ[1, 1.]"]
    );
    entry!(
        sin,
        "Sin",
        Arity::Exactly(1),
        numeric,
        "精确角度正弦。",
        "Evaluate exact-angle sine values.",
        &["Sin[Pi/6]", "Sin[-Pi/6]"]
    );
    entry!(
        cos,
        "Cos",
        Arity::Exactly(1),
        numeric,
        "精确角度余弦。",
        "Evaluate exact-angle cosine values.",
        &["Cos[Pi/3]", "Cos[Pi]"]
    );
    entry!(
        tan,
        "Tan",
        Arity::Exactly(1),
        numeric,
        "精确角度正切。",
        "Evaluate exact-angle tangent values.",
        &["Tan[Pi/4]", "Tan[Pi/3]"]
    );
    entry!(
        asin,
        "ArcSin",
        Arity::Exactly(1),
        numeric,
        "精确反正弦主值。",
        "Evaluate exact principal inverse sine values.",
        &["ArcSin[1/2]", "ArcSin[-1/2]"]
    );
    entry!(
        acos,
        "ArcCos",
        Arity::Exactly(1),
        numeric,
        "精确反余弦主值。",
        "Evaluate exact principal inverse cosine values.",
        &["ArcCos[1/2]", "ArcCos[-1/2]"]
    );
    entry!(
        atan,
        "ArcTan",
        Arity::Range(1, 2),
        numeric,
        "精确反正切主值或平面方向角。",
        "Evaluate principal inverse tangent values or a planar direction.",
        &["ArcTan[1]", "ArcTan[-1]"]
    );
    entry!(
        product_log,
        "ProductLog",
        Arity::Exactly(1),
        numeric,
        "Lambert W 主分支的精确特殊值。",
        "Evaluate exact special values of the principal Lambert W branch.",
        &["ProductLog[0]", "ProductLog[E]"]
    );
}

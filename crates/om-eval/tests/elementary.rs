//! M4's complete elementary registry, exact values, numerical modes and docs.
use om_core::{Expr, Interrupt, Symbol, canonicalize};
use om_eval::Evaluator;
use om_num::{Number, Precision, Real};
use om_parse::{Dialect, parse_expr};
fn src(s: &str) -> Expr {
    parse_expr(s, Dialect::Wolfram).unwrap()
}
fn run(s: &str) -> Expr {
    Evaluator::new()
        .evaluate(&src(s), &Interrupt::default())
        .unwrap()
}

fn reference(name: &str, x: f64) -> f64 {
    match name {
        "Sin" => x.sin(),
        "Cos" => x.cos(),
        "Tan" => x.tan(),
        "Cot" => 1.0 / x.tan(),
        "Sec" => 1.0 / x.cos(),
        "Csc" => 1.0 / x.sin(),
        "ArcSin" => x.asin(),
        "ArcCos" => x.acos(),
        "ArcTan" => x.atan(),
        "ArcCot" => (1.0 / x).atan(),
        "ArcSec" => (1.0 / x).acos(),
        "ArcCsc" => (1.0 / x).asin(),
        "Sinh" => x.sinh(),
        "Cosh" => x.cosh(),
        "Tanh" => x.tanh(),
        "Coth" => 1.0 / x.tanh(),
        "Sech" => 1.0 / x.cosh(),
        "Csch" => 1.0 / x.sinh(),
        "ArcSinh" => x.asinh(),
        "ArcCosh" => x.acosh(),
        "ArcTanh" => x.atanh(),
        _ => panic!("fixture name"),
    }
}
#[test]
fn every_elementary_function_has_two_numeric_tests_at_machine_and_big_precision() {
    for name in [
        "Sin", "Cos", "Tan", "Cot", "Sec", "Csc", "ArcSin", "ArcCos", "ArcTan", "ArcCot", "ArcSec",
        "ArcCsc", "Sinh", "Cosh", "Tanh", "Coth", "Sech", "Csch", "ArcSinh", "ArcCosh", "ArcTanh",
    ] {
        let inputs = match name {
            "ArcSec" | "ArcCsc" => [2.0, -2.0],
            "ArcCosh" => [2.0, 3.0],
            _ => [0.5, -0.5],
        };
        for x in inputs {
            for (suffix, precision) in [("", Precision::Machine), ("`30", Precision::Bits(100))] {
                let s = format!("{name}[{x:.1}{suffix}]");
                let e = run(&s);
                let n = e.as_number().expect(&s);
                assert_eq!(n.precision(), precision, "{s}");
                assert!(
                    (n.to_f64().unwrap() - reference(name, x)).abs() < 1e-15,
                    "{s}: {n:?}"
                );
            }
        }
    }
}
#[test]
fn reciprocal_angles_and_hyperbolic_special_values_stay_exact() {
    for (s, want) in [
        ("Cot[Pi/4]", "1"),
        ("Cot[Pi/3]", "Sqrt[3]/3"),
        ("Sec[Pi/3]", "2"),
        ("Sec[Pi]", "-1"),
        ("Csc[Pi/6]", "2"),
        ("Csc[Pi/2]", "1"),
        ("ArcCot[1]", "Pi/4"),
        ("ArcCot[-1]", "-Pi/4"),
        ("ArcSec[2]", "Pi/3"),
        ("ArcSec[-1]", "Pi"),
        ("ArcCsc[2]", "Pi/6"),
        ("ArcCsc[-1]", "-Pi/2"),
        ("Sinh[0]", "0"),
        ("Cosh[0]", "1"),
        ("Tanh[0]", "0"),
        ("Coth[0]", "ComplexInfinity"),
        ("Sech[0]", "1"),
        ("Csch[0]", "ComplexInfinity"),
        ("ArcSinh[0]", "0"),
        ("ArcCosh[1]", "0"),
        ("ArcCosh[0]", "I Pi/2"),
        ("ArcTanh[0]", "0"),
    ] {
        assert_eq!(run(s), canonicalize(&src(want)), "{s}");
    }
    for s in ["Sin[1/3]", "Sinh[x]", "ArcCosh[x]", "Cot[x]", "ArcSec[x]"] {
        assert_eq!(run(s), canonicalize(&src(s)), "{s}");
    }
}
#[test]
fn approximation_threads_and_uses_the_complex_principal_branch() {
    for (s, re, im) in [
        (
            "ArcSin[2.]",
            std::f64::consts::FRAC_PI_2,
            -(2.0f64 + 3.0f64.sqrt()).ln(),
        ),
        (
            "ArcCosh[-2.]",
            (2.0f64 + 3.0f64.sqrt()).ln(),
            std::f64::consts::PI,
        ),
        (
            "ArcTanh[2.]",
            3.0f64.ln() / 2.0,
            -std::f64::consts::FRAC_PI_2,
        ),
        ("Log[-2.]", 2.0f64.ln(), std::f64::consts::PI),
        ("Log[2.,8.]", 3.0, 0.0),
        ("ArcTan[-1.,1.]", 3.0 * std::f64::consts::FRAC_PI_4, 0.0),
        ("Sin[Pi+0.5]", -0.5f64.sin(), 0.0),
    ] {
        let n = run(s);
        let (r, i) = n.as_number().expect(s).to_complex_f64();
        assert!(
            (r - re).abs() < 1e-14 && (i - im).abs() < 1e-14,
            "{s}: {n:?}"
        );
    }
    let out = run("Sinh[{0.5,-0.5}]");
    assert!(out.is_head(om_core::BUILTIN::LIST));
    assert!(
        out.args()
            .iter()
            .all(|e| e.as_number().unwrap().precision() == Precision::Machine)
    );
    assert!(matches!(
        run("Exp[1000.]").as_number(),
        Some(Number::Real(Real::Big(_)))
    ));
    assert_eq!(run("ArcTan[1,1]"), canonicalize(&src("Pi/4")));
    assert_eq!(run("Im[0.5]"), Expr::int(0));
    assert_eq!(run("Sign[0.5]"), Expr::int(1));
    let s = "Plus[10000000000000000.,1.,-10000000000000000.]";
    assert_eq!(run(s), canonicalize(&src(s)));
}
#[test]
fn all_docs_matches_implemented_milestones_and_elementary_functions_check_arity() {
    let names = "Abs And Append Apply ArcCos ArcCosh ArcCot ArcCsc ArcSec ArcSin ArcSinh ArcTan ArcTanh Arg Binomial Ceiling Clear CompoundExpression Conjugate Cos Cosh Cot Coth Csc Csch Denominator Divide Dot Element Equal Exp FactorInteger Factorial First Floor Function GCD Greater GreaterEqual Hold HoldForm Im Inequality LCM Last Length Less LessEqual List Log Map Minus Mod N Not Numerator Or Out Part Plus Power PrimeQ Product ProductLog Quotient Range Re Record ReplaceAll ReplaceRepeated Rest Round Rule RuleDelayed SameQ Sec Sech Set SetDelayed Sign Sin Sinh Slot Sqrt Subtract Sum Table Tan Tanh Times Unequal Unset";
    let algebra = "Expand Factor Together Cancel Apart Simplify FullSimplify Collect Coefficient CoefficientList Exponent PolynomialQ PolynomialGCD PolynomialLCM PolynomialQuotient PolynomialRemainder Resultant Discriminant Variables D RootReduce ToRadicals";
    let solving = "Solve NSolve FindRoot Reduce Eliminate SolveValues NSolveValues Roots Root ConditionalExpression";
    let mut expected: Vec<_> = names
        .split_whitespace()
        .chain(algebra.split_whitespace())
        .chain(solving.split_whitespace())
        .chain(["Plot", "ContourPlot"])
        .collect();
    expected.sort();
    expected.extend([
        "Min",
        "Max",
        "MinMax",
        "IntegerPart",
        "FractionalPart",
        "Precision",
        "Accuracy",
        "Chop",
        "Rationalize",
        "Clip",
        "Mean",
        "Median",
        "Variance",
        "StandardDeviation",
        "Covariance",
        "Correlation",
        "Quantile",
        "Percentile",
        "Filter",
        "Sort",
        "SortBy",
        "Unique",
        "Take",
        "Drop",
        "Slice",
        "Flatten",
        "Reshape",
        "Zip",
        "Fold",
        "GroupBy",
        "Counts",
    ]);
    expected.sort();
    expected.extend([
        "IdentityMatrix",
        "DiagonalMatrix",
        "Transpose",
        "ConjugateTranspose",
        "Tr",
        "Det",
        "Inverse",
        "MatrixRank",
        "NullSpace",
        "LinearSolve",
        "Cross",
        "Norm",
        "Normalize",
        "Decimal",
        "Rescale",
    ]);
    expected.sort();
    expected.extend([
        "Lu",
        "Qr",
        "LeastSquares",
        "Cholesky",
        "Eigenvalues",
        "Eigensystem",
        "Svd",
    ]);
    expected.sort();
    expected.extend(["VectorAngle", "Projection"]);
    expected.extend([
        "Erf",
        "Erfc",
        "Gamma",
        "LogGamma",
        "Beta",
        "ArcCoth",
        "ArcSech",
        "ArcCsch",
        "NormalDistribution",
        "UniformDistribution",
        "PDF",
        "CDF",
        "RandomUniform",
        "RandomNormal",
        "RandomChoice",
        "SeedRandom",
    ]);
    expected.sort();
    expected.extend(["ParseCSV", "ParseJSON", "ToCSV", "ToJSON"]);
    expected.sort();
    expected.extend([
        "Quantity",
        "UnitConvert",
        "QuantityMagnitude",
        "QuantityUnit",
    ]);
    expected.sort();
    expected.extend(["Help", "Options", "Functions", "Capabilities"]);
    expected.sort();
    expected.extend(["CubeRoot", "NthRoot"]);
    expected.sort();
    expected.extend([
        "Grad",
        "Jacobian",
        "Hessian",
        "Divergence",
        "Curl",
        "Laplacian",
    ]);
    expected.sort();
    expected.extend(["Integrate", "NIntegrate"]);
    expected.sort();
    expected.extend(["Limit", "Series", "SeriesCoefficient", "Normal"]);
    expected.extend(["Ode", "Interpolate", "Sample", "Optimize", "Fit"]);
    expected.sort();
    let actual: Vec<_> = Evaluator::all_docs().map(|d| d.name).collect();
    assert_eq!(actual, expected);
    for name in [
        "Cot", "Sec", "Csc", "ArcCot", "ArcSec", "ArcCsc", "Sinh", "Cosh", "Tanh", "Coth", "Sech",
        "Csch", "ArcSinh", "ArcCosh", "ArcTanh",
    ] {
        let doc = Evaluator::doc(Symbol::intern(name)).unwrap();
        assert!(
            !doc.summary_zh.is_empty() && !doc.summary_en.is_empty() && doc.examples.len() >= 2
        );
        let mut ev = Evaluator::new();
        let raw = src(&format!("{name}[]"));
        assert_eq!(ev.evaluate(&raw, &Interrupt::default()).unwrap(), raw);
        assert!(
            ev.messages
                .take()
                .iter()
                .any(|m| m.symbol == name && m.tag == "argx")
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)] // Preserve the independent fixture's 17-digit exports.
fn every_complex_family_matches_independent_cmath_fixtures_off_the_cuts() {
    // Generated with Python's cmath at z=1/2+I/3; not the evaluator's formulas.
    for (name, re, im) in [
        ("Sin", 0.50630782403911545, 0.29797487210253704),
        ("Cos", 0.92679025531054937, -0.16278441454050099),
        ("Tan", 0.47517166005754674, 0.40497341274426596),
        ("Cot", 1.2190403660246045, -1.0389486133121135),
        ("Sec", 1.0467014836919395, 0.18384600749214053),
        ("Csc", 1.4669779143452459, -0.86335335076809072),
        ("ArcSin", 0.48683705132529498, 0.3687388509321739),
        ("ArcCos", 1.0839592754696015, -0.3687388509321739),
        ("ArcTan", 0.50113588953192834, 0.2678959040700476),
        ("ArcCot", 1.0696604372629683, -0.2678959040700476),
        ("ArcSec", 0.67849084365402068, 1.1784337975384371),
        ("ArcCsc", 0.89230548314087599, -1.1784337975384371),
        ("Sinh", 0.49241262861831692, 0.3689532357851702),
        ("Cosh", 1.0655579886666302, 0.170499620482925),
        ("Tanh", 0.5046017147200651, 0.26551237749565582),
        ("Coth", 1.5520489665356796, -0.81666034631526285),
        ("Sech", 0.91504738273404862, -0.14641646267912309),
        ("Csch", 1.3006258255857708, -0.97452843206340578),
        ("ArcSinh", 0.5019934403321612, 0.29980472264280833),
        ("ArcCosh", 0.3687388509321739, 1.0839592754696015),
        ("ArcTanh", 0.46942547475719493, 0.40333577471075477),
    ] {
        for s in [format!("{name}[0.5+I/3]"), format!("N[{name}[1/2+I/3],30]")] {
            let out = run(&s);
            let (r, i) = out.as_number().expect(&s).to_complex_f64();
            assert!(
                (r - re).abs() < 1e-14 && (i - im).abs() < 1e-14,
                "{s}: {out:?}"
            );
        }
    }
}

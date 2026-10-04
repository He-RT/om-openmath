//! Independent results for actual composition, lexical closures, records and matrix products.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
use om_parse::{Dialect, ParseEnv};
fn execute(ev: &mut Evaluator, source: &str) -> Expr {
    let parsed = om_parse::parse_with(
        source,
        Dialect::Modern,
        &ParseEnv {
            known_functions: ev.defs.known_functions(),
            ..Default::default()
        },
    );
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == om_parse::Severity::Error),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut result = Expr::sym(B::NULL);
    for s in parsed.statements {
        result = ev.evaluate(&s.expr, &Interrupt::default()).unwrap();
    }
    result
}
fn equal(ev: &mut Evaluator, src: &str, expected: &str) {
    let a = execute(ev, src);
    let b = om_parse::parse_expr(expected, Dialect::Wolfram).unwrap();
    assert_eq!(
        om_core::canonicalize(&a),
        om_core::canonicalize(&b),
        "{src}"
    );
}
#[test]
fn pipelines_evaluate_input_once_and_use_map_second_argument() {
    let mut ev = Evaluator::new();
    execute(&mut ev, "let n=0");
    equal(&mut ev, "assign(n,n+1) |> power(2)", "1");
    equal(&mut ev, "n", "1");
    equal(&mut ev, "[1,2,3] |> map(fn(x)=>x^2)", "{1,4,9}");
    equal(&mut ev, "(x+1)^2 |> expand()", "x^2+2x+1");
}
#[test]
fn closures_substitute_outer_parameters_preserve_shadowing_and_avoid_capture() {
    let mut ev = Evaluator::new();
    equal(&mut ev, "(fn(x)=>fn(y)=>x+y)(2)(3)", "5");
    equal(&mut ev, "(fn(x)=>fn(x)=>x+1)(10)(2)", "3");
    equal(&mut ev, "(fn(x)=>fn(y)=>x+y)(y)(3)", "y+3");
    equal(&mut ev, "let a=2\nlet f=fn(x)=>x+a\nlet a=5\nf(1)", "6");
    equal(&mut ev, "let fn(x)=x+2\nfn(3)", "5");
}
#[test]
fn records_fields_and_closed_slices_use_actual_values_and_report_missing_data() {
    let mut ev = Evaluator::new();
    equal(
        &mut ev,
        "let color=99\nlet config={color:2, α:3}\nconfig.color",
        "2",
    );
    equal(&mut ev, "config.α", "3");
    equal(&mut ev, "record()", "Record[]");
    equal(&mut ev, "let v=[1,2,3,4]\nv[2..3]", "{2,3}");
    equal(&mut ev, "v[4..2]", "{4,3,2}");
    ev.messages.take();
    execute(&mut ev, "v[0..2]");
    assert!(!ev.messages.take().is_empty());
    ev.messages.take();
    execute(&mut ev, "config.missing");
    assert!(!ev.messages.take().is_empty());
    ev.messages.take();
    execute(&mut ev, "v[2..7]");
    assert!(!ev.messages.take().is_empty());
}
#[test]
fn matrix_products_and_hermitian_vector_inner_products_check_dimensions() {
    let mut ev = Evaluator::new();
    equal(&mut ev, "[[1,2],[3,4]] @ [5,6]", "{17,39}");
    equal(&mut ev, "[[1,2],[3,4]] @ [[1,0],[0,1]]", "{{1,2},{3,4}}");
    equal(&mut ev, "[1+i,2] @ [1+i,2]", "6");
    ev.messages.take();
    execute(&mut ev, "[1,2] @ [1]");
    assert!(!ev.messages.take().is_empty());
}

#[test]
fn unified_modes_preserve_actual_solution_shapes_and_local_root_semantics() {
    let mut ev = Evaluator::new();
    equal(&mut ev, "solve(x^2=4,x,output:\"values\")", "{-2,2}");
    let numerical = execute(
        &mut ev,
        "solve(x^2=4,x,precision:20,mode:\"numeric\",output:\"values\")",
    );
    assert!(numerical.is_head(B::LIST));
    assert_eq!(numerical.args().len(), 2);
    let values: Vec<_> = numerical
        .args()
        .iter()
        .map(|e| e.as_number().unwrap().to_f64().unwrap())
        .collect();
    assert_eq!(values, vec![-2.0, 2.0]);
    equal(&mut ev, "simplify(sin(x)^2+cos(x)^2,level:\"deep\")", "1");
    equal(&mut ev, "diff(x^4,x,order:2)", "12x^2");
    equal(&mut ev, "coefficient(x^3+2x,x,order:3)", "1");
    equal(&mut ev, "floor(7/3,step:1/2)", "2");
    let root = execute(&mut ev, "find_root(x^2=2,x,bracket:1..2)");
    assert!(root.is_head(B::LIST));
    let number = root.args()[0].args()[1]
        .as_number()
        .unwrap()
        .to_f64()
        .unwrap();
    assert!((number - 2.0f64.sqrt()).abs() < 1e-10);
}

//! Real parser/evaluator tests use independent RFC data and verify that data never executes code.
use om_core::{BUILTIN as B, Expr, Interrupt};
use om_eval::Evaluator;
fn call(ev: &mut Evaluator, name: &str, args: impl IntoIterator<Item = Expr>) -> Expr {
    ev.evaluate(
        &Expr::call(om_core::Symbol::intern(name), args),
        &Interrupt::default(),
    )
    .unwrap()
}
fn parse(ev: &mut Evaluator, name: &str, text: &str) -> Expr {
    call(ev, name, [Expr::string(text)])
}
fn string(e: &Expr) -> &str {
    if let om_core::ExprKind::String(s) = e.kind() {
        s
    } else {
        panic!("{e:?}")
    }
}
#[test]
fn json_maps_lossless_data_and_does_not_execute_or_confuse_empty_records() {
    let mut ev = Evaluator::new();
    let data = parse(
        &mut ev,
        "ParseJSON",
        r#"{"中文":"瓜🍉","number":9007199254740993,"decimal":0.1,"null":null,"flag":true,"list":[],"object":{},"code":"Set[secret,99]"}"#,
    );
    assert!(data.is_head(B::RECORD));
    let fields: Vec<_> = data
        .args()
        .iter()
        .map(|e| (&e.args()[0], &e.args()[1]))
        .collect();
    assert_eq!(
        *fields[1].1,
        Expr::integer("9007199254740993".parse().unwrap())
    );
    assert_eq!(*fields[2].1, Expr::rational(1, 10));
    assert!(fields[5].1.is_head(B::LIST));
    assert!(fields[6].1.is_head(B::RECORD));
    assert!(
        ev.defs
            .own_value(om_core::Symbol::intern("secret"))
            .is_none()
    );
    let encoded = call(&mut ev, "ToJSON", [data.clone()]);
    let round = parse(&mut ev, "ParseJSON", string(&encoded));
    assert_eq!(data, round);
    let escaped = parse(&mut ev, "ParseJSON", r#""\uD83C\uDF49\n\t\"\\""#);
    assert_eq!(string(&escaped), "🍉\n\t\"\\");
}
#[test]
fn malformed_json_duplicate_keys_and_non_data_outputs_are_rejected() {
    let mut ev = Evaluator::new();
    for text in [
        r#"{"a":1,"\u0061":2}"#,
        "[1,]",
        "01",
        "NaN",
        "1e",
        "1e20001",
        "true false",
        r#""\uD800""#,
        "{a:1}",
    ] {
        ev.messages.take();
        assert!(
            parse(&mut ev, "ParseJSON", text).is_head(om_core::Symbol::intern("ParseJSON")),
            "{text}"
        );
        assert!(!ev.messages.take().is_empty(), "{text}");
    }
    for value in [
        Expr::rational(1, 3),
        Expr::sym(om_core::Symbol::intern("unresolved")),
    ] {
        ev.messages.take();
        call(&mut ev, "ToJSON", [value]);
        assert!(!ev.messages.take().is_empty());
    }
}
#[test]
fn csv_quotes_unicode_embedded_newlines_and_headers_roundtrip() {
    let mut ev = Evaluator::new();
    let csv = "名字,说明,空\r\n\"西瓜🍉\",\"一行\n另一行,\"\"甜\"\"\",\r\n";
    let rows = parse(&mut ev, "ParseCSV", csv);
    assert!(rows.is_head(B::DATA_TABLE));
    assert_eq!(rows.args()[1].args().len(), 1);
    let row = &rows.args()[1].args()[0];
    assert!(row.is_head(B::RECORD));
    assert_eq!(string(&row.args()[1].args()[1]), "一行\n另一行,\"甜\"");
    let encoded = call(&mut ev, "ToCSV", [rows.clone()]);
    assert_eq!(rows, parse(&mut ev, "ParseCSV", string(&encoded)));
    let raw = call(
        &mut ev,
        "ParseCSV",
        [
            Expr::string("1,2\n3,4\n"),
            Expr::call(
                B::RULE,
                [
                    Expr::sym(om_core::Symbol::intern("Header")),
                    Expr::sym(B::FALSE),
                ],
            ),
        ],
    );
    assert!(raw.args()[0].is_head(B::LIST));
    assert_eq!(string(&raw.args()[0].args()[0]), "1");
    assert_eq!(
        parse(&mut ev, "ParseCSV", ""),
        Expr::call(
            B::DATA_TABLE,
            [Expr::call(B::LIST, []), Expr::call(B::LIST, [])]
        )
    );
    let schema = parse(&mut ev, "ParseCSV", "a,b\r\n");
    assert_eq!(string(&call(&mut ev, "ToCSV", [schema])), "a,b\r\n");
    for text in [
        "a,a\n1,2",
        "a,b\n1",
        "a\n\"unterminated",
        "a\nb\"c",
        "a\n\"b\"x",
    ] {
        ev.messages.take();
        parse(&mut ev, "ParseCSV", text);
        assert!(!ev.messages.take().is_empty(), "{text}");
    }
}
#[test]
fn pure_data_parsing_and_export_share_the_actual_budget() {
    let mut ev = Evaluator::new();
    let ctx = Interrupt::default();
    ctx.steps_left.set(30);
    assert!(
        ev.evaluate(
            &Expr::call(
                om_core::Symbol::intern("ParseJSON"),
                [Expr::string(&format!("\"{}\"", "a".repeat(1000)))]
            ),
            &ctx
        )
        .is_err()
    );
    let deep = format!("{}0{}", "[".repeat(70), "]".repeat(70));
    ev.messages.take();
    parse(&mut ev, "ParseJSON", &deep);
    assert!(!ev.messages.take().is_empty());
    let expr = om_parse::parse_expr(
        "parse_json(to_json({x:1/10,list:[true,false]}))",
        om_parse::Dialect::Modern,
    )
    .unwrap();
    assert!(
        ev.evaluate(&expr, &Interrupt::default())
            .unwrap()
            .is_head(B::RECORD)
    );
}
#[test]
fn extreme_decimal_data_and_machine_values_preserve_their_actual_point() {
    let mut ev = Evaluator::new();
    for text in ["1e20000", "1e-20000", "-123456.789e-200", "-0", "1.2500"] {
        let value = parse(&mut ev, "ParseJSON", text);
        let encoded = call(&mut ev, "ToJSON", [value.clone()]);
        assert_eq!(
            value,
            parse(&mut ev, "ParseJSON", string(&encoded)),
            "{text}"
        );
    }
    let value = Expr::number(om_num::Number::Real(om_num::Real::Machine(0.1)));
    let encoded = call(&mut ev, "ToJSON", [value]);
    let decoded = parse(&mut ev, "ParseJSON", string(&encoded));
    assert_eq!(decoded.as_number().unwrap().to_f64(), Some(0.1));
    let cancelled = Interrupt::default();
    cancelled.steps_left.set(40);
    assert!(
        ev.evaluate(
            &Expr::call(
                om_core::Symbol::intern("ToJSON"),
                [Expr::string(&"🍉".repeat(1000))]
            ),
            &cancelled
        )
        .is_err()
    );
}

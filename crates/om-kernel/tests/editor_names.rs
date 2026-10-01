//! Every registered completion name parses to its real builtin rather than a lookalike.
use om_core::Symbol;
#[test]
fn every_registered_modern_callable_uses_the_real_parser_mapping() {
    for doc in om_eval::Evaluator::all_docs() {
        let symbol = Symbol::intern(doc.name);
        let name = om_parse::modern_name(symbol);
        assert_eq!(
            om_parse::identifier_symbol(
                &name,
                om_parse::Dialect::Modern,
                om_parse::ConstantMode::Math,
                true
            ),
            Some(symbol),
            "{name}"
        );
        let parsed = om_parse::parse(&format!("{name}(editor_axis)"), om_parse::Dialect::Modern);
        assert!(
            !parsed.diagnostics.iter().any(|d| d.code == "W002"),
            "{name}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(
            parsed.statements[0].expr.head_symbol(),
            Some(symbol),
            "{name}"
        );
    }
}

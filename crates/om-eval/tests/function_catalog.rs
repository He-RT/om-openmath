//! Documentation claims are checked against the actual callable registry.
use om_eval::Evaluator;
use std::collections::BTreeSet;

#[test]
fn catalog_covers_registered_builtins_without_promoting_plans() {
    let docs: Vec<_> = Evaluator::all_docs().collect();
    if let Ok(path) = std::env::var("OPENMATH_CATALOG_AUDIT_PATH") {
        let mut audit = String::new();
        for doc in &docs {
            audit.push_str("[[functions]]\n");
            audit.push_str(&toml::to_string(doc).unwrap());
            audit.push('\n');
        }
        std::fs::write(path, audit).unwrap();
    }
    let catalog: toml::Value =
        toml::from_str(include_str!("../../../docs/reference/functions.toml")).unwrap();
    let entries = catalog["functions"].as_array().unwrap();
    let mut names = BTreeSet::new();
    for entry in entries {
        let status = entry["status"].as_str().unwrap();
        let runtime = entry["runtime_names"].as_array().unwrap();
        if matches!(status, "planned" | "deferred") {
            assert!(runtime.is_empty(), "a plan cannot claim a callback");
        }
        for name in runtime {
            let name = name.as_str().unwrap();
            assert!(names.insert(name.to_owned()), "duplicate callback: {name}");
            assert!(
                docs.iter().any(|doc| doc.name == name),
                "missing callback: {name}"
            );
        }
    }
    assert_eq!(names, docs.iter().map(|doc| doc.name.to_owned()).collect());
}

#[test]
fn catalog_current_examples_are_registered_examples_and_parse() {
    let catalog: toml::Value =
        toml::from_str(include_str!("../../../docs/reference/functions.toml")).unwrap();
    for entry in catalog["functions"].as_array().unwrap() {
        for name in entry["runtime_names"].as_array().unwrap() {
            let doc = Evaluator::all_docs()
                .find(|doc| Some(doc.name) == name.as_str())
                .unwrap();
            let examples = entry["current_examples"].as_array().unwrap();
            for source in doc.examples {
                assert!(examples.iter().any(|value| value.as_str() == Some(source)));
                om_parse::parse_expr(source, om_parse::Dialect::Wolfram).unwrap();
            }
            assert!(
                entry["current_wolfram_signatures"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(doc.wolfram))
            );
        }
    }
}

#[test]
fn export_registry_audit() {
    use om_eval::{Arity, Attributes as A};
    let Ok(path) = std::env::var("OPENMATH_RUNTIME_AUDIT_PATH") else {
        return;
    };
    let mut text = String::new();
    for spec in Evaluator::all_specs() {
        let (min, max) = match spec.arity {
            Arity::Exactly(n) => (n, Some(n)),
            Arity::Range(a, b) => (a, Some(b)),
            Arity::AtLeast(n) => (n, None),
            Arity::Any => (0, None),
        };
        text.push_str(&format!(
            "[[runtime]]\nname = {:?}\nmin = {min}\n",
            spec.symbol.name()
        ));
        if let Some(max) = max {
            text.push_str(&format!("max = {max}\n"));
        }
        let attributes: Vec<_> = [
            ("hold_all", A::HOLD_ALL),
            ("hold_first", A::HOLD_FIRST),
            ("hold_rest", A::HOLD_REST),
            ("listable", A::LISTABLE),
            ("protected", A::PROTECTED),
            ("numeric_function", A::NUMERIC_FUNCTION),
            ("flat", A::FLAT),
            ("orderless", A::ORDERLESS),
            ("one_identity", A::ONE_IDENTITY),
        ]
        .into_iter()
        .filter_map(|(name, attr)| spec.attrs.contains(attr).then_some(name))
        .collect();
        text.push_str(&format!("attributes = {attributes:?}\n"));
    }
    std::fs::write(path, text).unwrap();
}

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

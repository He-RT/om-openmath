//! Original 53 authority rows through the actual mobile session bridge.
#[path = "../../../tests/corpus/solve_support.rs"]
mod support;
use om_ios_ffi::Host;
use serde_json::{Value, json};
#[test]
fn mobile_bridge_matches_every_original_authority_row() {
    let cases = support::corpus();
    assert_eq!(cases.len(), 53);
    for case in cases {
        let mut config = om_kernel::KernelConfig::default();
        config.general.eval_timeout_ms = 120_000;
        let host = Host::new(Some(&serde_json::to_string(&config).unwrap())).unwrap();
        let packet: Value = serde_json::from_str(&host.request(&json!({"id":1,"body":{"type":"evaluate","cell_id":"test","source":case.input,"dialect":"Wolfram"}}).to_string()).unwrap()).unwrap();
        let output = &packet["response"]["body"]["output"];
        let source = output["items"]
            .as_array()
            .and_then(|x| x.last())
            .and_then(|x| x["input_form"].as_str())
            .unwrap_or_else(|| panic!("#{}: {packet}", case.id));
        let actual = support::e(source);
        let context = format!("{} {}", case.authority, case.notes);
        if case.check == "=" {
            assert_eq!(
                actual,
                support::e(case.expected.as_ref().unwrap()),
                "#{}: {}",
                case.id,
                context
            );
        } else {
            let original = om_parse::parse_expr(&case.input, om_parse::Dialect::Wolfram).unwrap();
            support::numeric_expr(&case, &actual, &original.args()[0])
                .unwrap_or_else(|reason| panic!("#{}: {reason}", case.id));
        }
    }
}

//! The same 53 immutable authority rows execute through the real evaluator.
use om_core::canonicalize;
use om_eval::Evaluator;
use om_num::ctx::Interrupt;
use om_parse::{Dialect, parse_expr};
#[path = "../../../tests/corpus/solve_support.rs"]
mod support;
#[test]
fn all_fifty_three_authority_rows_execute_through_evaluator() {
    let cases = support::corpus();
    assert_eq!(cases.len(), 53);
    let mut errors = vec![];
    for case in cases {
        let result = (|| {
            let call =
                parse_expr(&case.input, Dialect::Wolfram).map_err(|e| format!("parse {e:?}"))?;
            let mut ev = Evaluator::new();
            let actual = ev
                .evaluate_statement(&call, &Interrupt::default())
                .map_err(|e| e.to_string())?;
            if case.check == "=" {
                let expected = support::e(
                    case.expected
                        .as_ref()
                        .ok_or("missing exact authority form")?,
                );
                let expected = om_format::input_form(&expected);
                let actual = om_format::input_form(&canonicalize(&actual));
                if actual != expected {
                    return Err(format!("exact form {actual}; expected {expected}"));
                }
            } else {
                support::numeric_expr(&case, &actual, &call.args()[0])?;
            }
            Ok(())
        })();
        if let Err(reason) = result {
            errors.push(format!(
                "#{} {}: {reason}; authority {} {}",
                case.id, case.input, case.authority, case.notes
            ));
        }
    }
    assert!(
        errors.is_empty(),
        "{} corpus failures:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

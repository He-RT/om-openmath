//! All authority mathematics is validated after real om -e process execution.
#[path = "../../../tests/corpus/solve_support.rs"]
mod support;
#[test]
fn all_fifty_three_authority_rows_execute_through_actual_cli() {
    use om_parse::{Dialect, parse_expr};
    use std::process::Command;
    let cases = support::corpus();
    assert_eq!(cases.len(), 53);
    // Debug builds retain correctness/overflow checks but need a larger wall-clock budget.
    let config =
        std::env::temp_dir().join(format!("openmath-cli-corpus-{}.toml", std::process::id()));
    std::fs::write(
        &config,
        "[general]\neval_timeout_ms = 120000\n[llm]\nenabled = false\n",
    )
    .unwrap();
    let mut errors = vec![];
    for case in cases {
        let result = (|| {
            let output = Command::new(env!("CARGO_BIN_EXE_om"))
                .args([
                    "--config",
                    config.to_str().unwrap(),
                    "--language",
                    "en",
                    "--dialect",
                    "wolfram",
                    "--json",
                    "-e",
                    &case.input,
                ])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "exit {:?}: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let value: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
            let item = value["items"]
                .as_array()
                .and_then(|items| items.last())
                .ok_or("No actual CLI item")?;
            let source = item["input_form"].as_str().ok_or("Missing InputForm")?;
            let actual = support::e(source);
            if case.check == "=" {
                let expected = support::e(case.expected.as_ref().ok_or("No authority form")?);
                if actual != expected {
                    return Err(format!("actual {source}; expected {:?}", case.expected));
                }
            } else {
                let original =
                    parse_expr(&case.input, Dialect::Wolfram).map_err(|e| format!("{e:?}"))?;
                support::numeric_expr(&case, &actual, &original.args()[0])?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!(
                "#{} {}: {error}; authority {}; {}",
                case.id, case.input, case.authority, case.notes
            ));
        }
    }
    std::fs::remove_file(config).unwrap();
    assert!(
        errors.is_empty(),
        "{} CLI authority failures:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

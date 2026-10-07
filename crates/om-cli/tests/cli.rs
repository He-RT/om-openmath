//! Actual process-level CLI behavior; isolated config paths never use live credentials.
use std::process::Command;
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_om"))
        .args(["--no-config", "--language", "en"])
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn actual_kernel_eval_dialects_json_and_error_codes() {
    for source in ["solve(x^2-5x+6=0,x)", "Solve[x^2-5x+6==0,x]"] {
        let o = run(&["--json", "-e", source]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["items"][0]["type"], "solutions");
        assert_eq!(
            v["items"][0]["view"]["solutions"].as_array().unwrap().len(),
            2
        );
    }
    let parse = run(&["--json", "-e", "solve("]);
    assert_eq!(parse.status.code(), Some(2));
    assert!(serde_json::from_slice::<serde_json::Value>(&parse.stdout).is_ok());
    let eval = run(&["--json", "-e", "Solve[x==1,2]"]);
    assert_eq!(eval.status.code(), Some(1));
    let text = run(&["-e", "solve(x^2==4,x)"]);
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("x = −2") && text.contains("x = 2"));
}
#[test]
fn piped_repl_is_sequential_and_meta_commands_use_real_records() {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_om"))
        .args(["--no-config", "--language", "en"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"let a=2\nlet a=5\na+1\nsolve(x^2==4,x)\n:steps\n:latex\n:clear\na\n:quit\n")
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("Out[3]= 6"), "{text}");
    assert!(text.contains("S1"));
    assert!(text.contains("x"));
}

#[test]
fn real_source_and_notebook_files_run_in_order_with_source_only_data() {
    let root = std::env::temp_dir().join(format!("openmath-cli-files-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("test.om");
    std::fs::write(&script, "let a=2\na+1").unwrap();
    let o = run(&["--json", "run", script.to_str().unwrap()]);
    assert!(o.status.success());
    let value: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(value["items"][1]["input_form"], "3");
    let file = root.join("test.omnb");
    std::fs::write(&file,r##"{"version":1,"title":"Test","cells":[{"id":"a","kind":"Math","dialect":"Modern","source":"let a=5"},{"id":"t","kind":"Text","dialect":"Auto","source":"# notes"},{"id":"b","kind":"Math","dialect":"Modern","source":"a+1"}]}"##).unwrap();
    let o = run(&["--json", "run", file.to_str().unwrap()]);
    assert!(o.status.success());
    let lines = String::from_utf8(o.stdout).unwrap();
    let outputs = lines
        .lines()
        .map(|s| serde_json::from_str::<serde_json::Value>(s).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outputs.last().unwrap()["items"][0]["input_form"], "6");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn notebook_cells_preserve_their_declared_dialect() {
    let file =
        std::env::temp_dir().join(format!("openmath-cli-dialect-{}.omnb", std::process::id()));
    std::fs::write(&file, r#"{"version":1,"title":"Dialect","cells":[{"id":"w","kind":"Math","dialect":"Wolfram","source":"Sin[Pi/2]"}]}"#).unwrap();
    let output = run(&[
        "--dialect",
        "modern",
        "--json",
        "run",
        file.to_str().unwrap(),
    ]);
    std::fs::remove_file(file).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["items"][0]["input_form"], "1");
}

#[test]
fn executable_help_and_capabilities_are_query_only_and_never_include_plans() {
    use std::{io::Write, process::Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_om"))
        .args(["--no-config", "--language", "zh-CN", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":functions\n:options find_root\n:help fn_000001\n:capabilities\n1+1\n:quit\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 5);
    assert!(
        rows[0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["modern_name"] == "polynomial_gcd")
    );
    assert!(
        rows[0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["modern_name"] == "explore" && f["id"] == "fn_000225")
    );
    assert!(
        rows[0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["modern_name"] == "scene" && f["id"] == "fn_000226")
    );
    assert!(
        !rows[0]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["modern_name"] == "polar_plot")
    );
    assert!(
        rows[1]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "max_iterations" && p["default_source"] == "100")
    );
    assert_eq!(rows[2]["descriptor"]["name"], "Abs");
    assert!(
        !rows[2]["documentation"]["summary"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert_eq!(rows[3]["capabilities"]["scene_3d"], false);
    assert!(rows[3]["capabilities"]["task_permissions"].is_null());
    assert_eq!(rows[4]["items"][0]["input_form"], "2");
    assert_eq!(rows[4]["items"][0]["out_index"], 1);
}

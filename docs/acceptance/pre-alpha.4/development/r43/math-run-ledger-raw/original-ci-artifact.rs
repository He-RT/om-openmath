//! Real CLI exports, persisted-byte readback and overwrite boundaries.
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn folder() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "om-artifact-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&p).unwrap();
    p
}
#[test]
fn noninteractive_svg_png_csv_json_exports_are_real_and_invalid_source_never_writes() {
    let dir = folder();
    for (format, source) in [
        ("svg", "plot(x^2,x:-2..2)"),
        ("png", "plot(x^2,x:-2..2)"),
        ("csv", "[[1,2],[3,4]]"),
        ("json", "[[1,2],[3,4]]"),
    ] {
        let target = dir.join(format!("result.{format}"));
        let out = Command::new(env!("CARGO_BIN_EXE_om"))
            .args([
                "--no-config",
                "--json",
                "--dialect",
                "modern",
                "export",
                "-e",
                source,
                "--format",
                format,
                "--output",
            ])
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let bytes = fs::read(&target).unwrap();
        let receipt: serde_json::Value = serde_json::from_slice(
            String::from_utf8(out.stdout)
                .unwrap()
                .lines()
                .last()
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(receipt["persisted"], true);
        assert_eq!(receipt["artifact"]["byte_len"], bytes.len());
        match format {
            "svg" => assert!(
                String::from_utf8(bytes.clone())
                    .unwrap()
                    .contains("<metadata>")
            ),
            "png" => assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n"),
            "csv" => assert_eq!(bytes, b"1,2\r\n3,4\r\n"),
            _ => assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
                serde_json::json!([[1, 2], [3, 4]])
            ),
        }
        let second = Command::new(env!("CARGO_BIN_EXE_om"))
            .args(["--no-config", "export", "-e", source, "-f", format, "-o"])
            .arg(&target)
            .output()
            .unwrap();
        assert!(!second.status.success());
        assert_eq!(fs::read(&target).unwrap(), bytes);
    }
    let bad = dir.join("bad.png");
    let out = Command::new(env!("CARGO_BIN_EXE_om"))
        .args(["--no-config", "export", "-e", "plot(", "-f", "png", "-o"])
        .arg(&bad)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!bad.exists());
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn source_only_notebook_exports_the_actual_last_table_and_scope_rejects_export_flags_elsewhere() {
    let dir = folder();
    let input = dir.join("source.omnb");
    fs::write(&input,serde_json::json!({"version":1,"title":"数据","cells":[{"id":"a","kind":"Math","source":"let a=3","dialect":"Modern"},{"id":"table","kind":"Math","source":"[[a,a^2]]","dialect":"Modern"}]}).to_string()).unwrap();
    let output = dir.join("table.csv");
    let out = Command::new(env!("CARGO_BIN_EXE_om"))
        .args(["--no-config", "export", "--input"])
        .arg(&input)
        .args(["--format", "csv", "--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(&output).unwrap(), b"3,9\r\n");
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_om"))
            .args(["--no-config", "-e", "1+1", "--width", "1000"])
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_obj_contains_original_3d_helix_geometry_and_real_normals_without_an_interactive_window() {
    let dir = folder();
    let target = dir.join("helix.obj");
    let out = Command::new(env!("CARGO_BIN_EXE_om"))
        .args([
            "--no-config",
            "--json",
            "--dialect",
            "modern",
            "export",
            "-e",
            "parametric_plot([cos(t),sin(t),t],t:0..2*pi)",
            "--format",
            "obj",
            "--output",
        ])
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let data = fs::read_to_string(target).unwrap();
    assert!(data.lines().any(|line| line.starts_with("l ")));
    for vertex in data.lines().filter(|line| line.starts_with("v ")) {
        let p = vertex
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse::<f64>().unwrap())
            .collect::<Vec<_>>();
        assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-12);
    }
    fs::remove_dir_all(dir).unwrap();
}

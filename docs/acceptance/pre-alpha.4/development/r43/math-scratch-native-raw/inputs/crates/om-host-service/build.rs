//! Bind native persistent checkpoints to the actual compiled Rust workspace content, no Git needed.
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries = fs::read_dir(dir)
        .expect("workspace source directory")
        .map(|e| e.expect("source entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for p in entries {
        if p.is_dir() && p.file_name().is_some_and(|n| n != "target" && n != ".git") {
            collect(&p, files);
        } else if p.is_file()
            && (p.extension().is_some_and(|e| e == "rs")
                || p.file_name().is_some_and(|n| n == "Cargo.toml"))
        {
            files.push(p);
        }
    }
}
fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest path"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let mut files = Vec::new();
    collect(&root.join("crates"), &mut files);
    for n in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        files.push(root.join(n));
    }
    for entry in fs::read_dir(root.join("docs/design")).expect("native schemas") {
        let path = entry.expect("schema entry").path();
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with(".schema.json"))
        {
            files.push(path);
        }
    }
    files.push(root.join("docs/reference/functions.toml"));
    files.push(root.join("docs/design/native-host-abi.json"));
    files.sort();
    let mut hash = Sha256::new();
    hash.update(b"openmath-native-compiled-workspace-v1");
    for key in [
        "TARGET",
        "PROFILE",
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_CFG_TARGET_OS",
    ] {
        let value = std::env::var(key).expect("compiler environment");
        hash.update((key.len() as u64).to_be_bytes());
        hash.update(key.as_bytes());
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .strip_prefix(root)
            .expect("workspace member")
            .to_str()
            .expect("source path UTF8");
        let bytes = fs::read(&path).expect("actual source bytes");
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    println!(
        "cargo:rustc-env=OPENMATH_NATIVE_KERNEL_BUILD_ID={:x}",
        hash.finalize()
    );
}

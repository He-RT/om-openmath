"""为移动应用与 XCFramework 复制实际生产 Rust 依赖的上游许可。"""
import json
import shutil
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
tree = subprocess.check_output(["cargo", "tree", "-p", "om-ios-ffi", "--target", "aarch64-apple-ios", "--edges", "normal", "--prefix", "none", "--locked"], cwd=root, text=True)
used = {tuple(line.split()[:2]) for line in tree.splitlines()}
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", "aarch64-apple-ios"], cwd=root, text=True))
out = root / "ios/OpenMath/Resources/RustNotices"
if out.is_dir(): shutil.rmtree(out)
out.mkdir(exist_ok=True)
for package in metadata["packages"]:
    if (package["name"], "v"+package["version"]) not in used or package["name"].startswith("om-"):
        continue
    folder = Path(package["manifest_path"]).parent
    files = {p for p in folder.iterdir() if p.is_file() and p.name.lower().startswith(("license", "copying", "notice"))}
    if package.get("license_file"):
        files.add(folder / package["license_file"])
    target = out / (package["name"]+"-"+package["version"])
    target.mkdir(exist_ok=True)
    for file in files:
        shutil.copyfile(file, target / file.name)
    if not files:
        vendor = root / "ios/Licenses" / ("dashu" if package["name"].startswith("dashu") else "ts-rs")
        if not vendor.is_dir():
            raise SystemExit(f"缺少上游许可文本：{package['name']}")
        for text in vendor.iterdir():
            if text.is_file(): shutil.copyfile(text, target / text.name)
        shutil.copyfile(folder / "Cargo.toml.orig", target / "Cargo.toml.orig")

for name in ["LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"]:
    shutil.copyfile(root / name, out / name)
framework = root / "ios/Frameworks/OpenMathKernel.xcframework"
if framework.is_dir():
    shutil.copytree(out, framework / "Licenses", dirs_exist_ok=True)

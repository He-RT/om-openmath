"""校验同源版本和发行资产；只使用 Python 标准库。"""
import argparse
import hashlib
import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.0-pre-alpha.1"


def verify_version(root: Path, tag: str) -> str:
    """标签必须与 Rust、npm、锁文件和桌面配置一致。"""
    if tag != f"v{VERSION}":
        raise ValueError(f"发行标签必须是 v{VERSION}")
    versions = [tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]]
    for name in ("app/package.json", "app/package-lock.json", "app/src-tauri/tauri.conf.json"):
        data = json.loads((root / name).read_text())
        versions.append(data["version"])
        if "packages" in data:
            versions.append(data["packages"][""]["version"])
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    versions.extend(p["version"] for p in lock["package"] if p["name"].startswith("om-"))
    if any(version != VERSION for version in versions):
        raise ValueError("版本不一致，请同步 Cargo/npm/Tauri 及锁文件")
    return VERSION


def asset_names(version: str) -> set[str]:
    return {
        f"OpenMath_{version}_x64-setup.exe",
        f"OpenMath_{version}_x64_zh-CN.msi",
        f"OpenMath_{version}_aarch64.dmg",
        f"OpenMath_{version}_macos_arm64.app.zip",
        f"om-cli_{version}_windows_x64.zip",
        f"om-cli_{version}_macos_arm64.zip",
        f"OpenMath-web_{version}.zip",
    }


def finalize(root: Path, tag: str, revision: str) -> None:
    """拒绝缺失、额外、空文件或链接；生成大小与 SHA256 清单。"""
    version = verify_version(ROOT, tag)
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("必须提供完整的 Git 提交 SHA")
    expected = asset_names(version)
    actual = {path.name for path in root.iterdir()} - {"release-manifest.json"}
    if actual != expected:
        raise ValueError(f"发行资产不完整：缺失={sorted(expected - actual)}，额外={sorted(actual - expected)}")
    assets = []
    for name in sorted(expected):
        path = root / name
        if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"发行资产必须是非空普通文件：{name}")
        with path.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        assets.append({"name": name, "bytes": path.stat().st_size, "sha256": digest})
    manifest = {"version": version, "tag": tag, "revision": revision, "assets": assets}
    (root / "release-manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["verify", "finalize"])
    parser.add_argument("--tag", required=True)
    parser.add_argument("--root", type=Path, default=Path("release-assets"))
    parser.add_argument("--sha")
    args = parser.parse_args()
    if args.command == "verify":
        print(verify_version(ROOT, args.tag))
    else:
        if not args.sha:
            parser.error("finalize 需要 --sha")
        finalize(args.root, args.tag, args.sha)


if __name__ == "__main__":
    main()

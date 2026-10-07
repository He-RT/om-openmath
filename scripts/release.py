"""校验同源版本和发行资产；只使用 Python 标准库。"""
import argparse
import hashlib
import json
import plistlib
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.1.0-pre-alpha.3"


def verify_version(root: Path, tag: str) -> str:
    """标签必须与 Rust、npm、锁文件和桌面配置一致。"""
    if tag != f"v{VERSION}":
        raise ValueError(f"发行标签必须是 v{VERSION}")
    versions = [tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]]
    for name in ("app/package.json", "app/package-lock.json", "app/src-tauri/tauri.conf.json"):
        data = json.loads((root / name).read_text(encoding="utf-8"))
        versions.append(data["version"])
        if "packages" in data:
            versions.append(data["packages"][""]["version"])
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    versions.extend(p["version"] for p in lock["package"] if p["name"].startswith("om-"))
    if any(version != VERSION for version in versions):
        raise ValueError("版本不一致，请同步 Cargo/npm/Tauri 及锁文件")
    ios = plistlib.loads((root / "ios/OpenMath/Info.plist").read_bytes())
    build = VERSION.rsplit(".", 1)[1]
    base = VERSION.split("-", 1)[0]
    windows = json.loads((root / "app/src-tauri/tauri.windows.conf.json").read_text(encoding="utf-8"))
    if (ios.get("OpenMathReleaseVersion") != VERSION
            or ios.get("CFBundleShortVersionString") != base
            or ios.get("CFBundleVersion") != build
            or windows["bundle"]["windows"]["wix"]["version"] != f"{base}.{build}"):
        raise ValueError("iOS 版本或 Windows 数字产品版本与发行版不一致")
    catalog = tomllib.loads((root / "docs/reference/functions.toml").read_text(encoding="utf-8"))
    if catalog["current_version"] != VERSION:
        raise ValueError("函数目录版本与内核发行版不一致")
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
        f"OpenMath_{version}_ios_simulator_arm64.app.zip",
        f"OpenMathKernel_{version}.xcframework.zip",
    }


def verify_payload_bytes(original: bytes, installed: bytes, kind: str) -> dict:
    """只允许 Tauri 2.10 的唯一安装类型标记被打包器修改，其余逐字节相同。"""
    marker = b"__TAURI_BUNDLE_TYPE_VAR_UNK"
    stamps = {"nsis": b"NSS", "msi": b"MSI"}
    if kind not in stamps or original.count(marker) != 1:
        raise ValueError("安装类型或 Tauri 原始标记无效/不唯一")
    expected = original.replace(marker, marker[:-3] + stamps[kind], 1)
    if installed != expected:
        raise ValueError("安装程序出现 Tauri 类型标记以外的字节差异")
    return {
        "kind": kind,
        "bytes": len(installed),
        "original_sha256": hashlib.sha256(original).hexdigest(),
        "installed_sha256": hashlib.sha256(installed).hexdigest(),
        "allowed_change": f"__TAURI_BUNDLE_TYPE_VAR_UNK -> {stamps[kind].decode()}",
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
    parser.add_argument("command", choices=["verify", "finalize", "verify-payload"])
    parser.add_argument("--tag")
    parser.add_argument("--root", type=Path, default=Path("release-assets"))
    parser.add_argument("--sha")
    parser.add_argument("--original", type=Path)
    parser.add_argument("--installed", type=Path)
    parser.add_argument("--kind", choices=["nsis", "msi"])
    args = parser.parse_args()
    if args.command == "verify-payload":
        if not (args.original and args.installed and args.kind):
            parser.error("verify-payload 需要 --original/--installed/--kind")
        print(json.dumps(verify_payload_bytes(args.original.read_bytes(), args.installed.read_bytes(), args.kind), ensure_ascii=False, indent=2))
        return
    if not args.tag:
        parser.error("需要 --tag")
    if args.command == "verify":
        print(verify_version(ROOT, args.tag))
    else:
        if not args.sha:
            parser.error("finalize 需要 --sha")
        finalize(args.root, args.tag, args.sha)


if __name__ == "__main__":
    main()

"""Package an existing CLI binary, retain its notices, and smoke-test the extracted ZIP."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import zipfile

TARGETS = {"macos_arm64": "aarch64-apple-darwin", "windows_x64": "x86_64-pc-windows-msvc"}


def capture(command, cwd=None):
    return subprocess.run(command, cwd=cwd, check=True, capture_output=True,
                          encoding="utf-8", timeout=120).stdout


def retain_notices(source, stage, target):
    for name in ("LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"):
        shutil.copy2(source / name, stage / name)
    shutil.copytree(source / "licenses", stage / "licenses")
    metadata = json.loads(capture(["cargo", "metadata", "--locked", "--format-version", "1",
                                   "--filter-platform", target], source))
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    pending = [p["id"] for p in packages.values() if p["name"] == "om-cli"]
    selected = set()
    while pending:
        identity = pending.pop()
        if identity in selected:
            continue
        selected.add(identity)
        for dep in nodes[identity]["deps"]:
            if any(kind["kind"] in (None, "build") for kind in dep["dep_kinds"]):
                pending.append(dep["pkg"])
    index = []
    for identity in sorted(selected):
        package = packages[identity]
        if not package["source"]:
            continue
        directory = Path(package["manifest_path"]).parent
        destination = stage / "licenses" / "rust" / f'{package["name"]}-{package["version"]}'
        destination.mkdir(parents=True)
        for entry in directory.iterdir():
            if entry.name.lower().startswith(("license", "licence", "copying", "notice", "unlicense")):
                if entry.is_dir():
                    shutil.copytree(entry, destination / entry.name)
                else:
                    shutil.copy2(entry, destination / entry.name)
        index.append(f'{package["name"]} {package["version"]} | {package["license"]} | {package["source"]}')
    (stage / "licenses" / "INDEX.txt").write_text("\n".join(sorted(index)) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--platform", choices=TARGETS, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = args.source.resolve()
    target = TARGETS[args.platform]
    revision = capture(["git", "rev-parse", "HEAD"], source).strip()
    if revision != args.sha:
        raise ValueError("Source checkout differs from the requested release SHA")
    version = tomllib.loads((source / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    if version != args.version:
        raise ValueError("Source version differs from the package version")
    executable = "om.exe" if args.platform == "windows_x64" else "om"
    binary = source / "target" / target / "release" / executable
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output.resolve() / f"om-cli_{version}_{args.platform}.zip"
    with tempfile.TemporaryDirectory(prefix="openmath-cli-") as temporary:
        stage = Path(temporary) / "package"
        stage.mkdir()
        shutil.copy2(binary, stage / executable)
        retain_notices(source, stage, target)
        (stage / "BUILD.json").write_text(json.dumps({"version": version, "revision": revision,
            "target": target}, indent=2) + "\n", encoding="utf-8")
        (stage / "README.txt").write_text(
            f"OpenMath {version} CLI ({target})\nSource: {revision}\n\n"
            f"Extract this ZIP, then run {executable} in a terminal. No Node or GUI runtime is required.\n"
            "Optionally add this directory to PATH.\n\n"
            'om --version\nom --no-config -e "solve(x^2 - 5*x + 6 = 0, x)"\n'
            'om --no-config --json -e "2+2"\nom run example.omnb\nom --help\n\n'
            "This package ships the existing terminal features; no notebook assistant was added.\n"
            "No Developer ID / Authenticode distribution signature or Apple notarization is provided.\n"
            "Project and dependency notices accompany this executable.\n", encoding="utf-8")
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    bundle.write(path, path.relative_to(stage).as_posix())
        extracted = Path(temporary) / "extracted"
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(extracted)
            if os.name != "nt":
                for item in bundle.infolist():
                    (extracted / item.filename).chmod((item.external_attr >> 16) & 0o777)
        installed = str(extracted / executable)
        reported = capture([installed, "--version"]).strip()
        if reported != f"OpenMath {version}":
            raise ValueError(f"Unexpected CLI version: {reported}")
        result = json.loads(capture([installed, "--no-config", "--language", "en", "--json",
                                     "-e", "solve(x^2 - 5*x + 6 = 0, x)"]))
        solutions = next(item["view"]["solutions"] for item in result["items"] if "view" in item)
        roots = sorted(binding["input_form"] for solution in solutions
                       for binding in solution["bindings"] if binding["var"] == "x")
        if roots != ["2", "3"]:
            raise ValueError(f"Unexpected roots: {roots}")
        print(json.dumps({"asset": archive.name, "revision": revision, "target": target,
                          "version": reported, "extracted_roots": roots}, indent=2))


if __name__ == "__main__":
    main()

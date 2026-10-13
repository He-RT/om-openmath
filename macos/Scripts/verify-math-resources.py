"""Check installed SwiftMath fonts/metrics/notices against the committed package inventory."""
from pathlib import Path
import argparse
import hashlib
import json

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("app", type=Path)
parser.add_argument("--output", type=Path)
args = parser.parse_args()
resources = args.app / "Contents/Resources"
catalog = json.loads((resources / "fonts/swiftmath-fonts.json").read_text())
assert catalog["revision"] == "fa8244ed032f4a1ade4cb0571bf87d2f1a9fd2d7", "package pin changed"
package = resources / catalog["resource_bundle"]
candidates = [package / catalog["font_bundle"], package / "Contents/Resources" / catalog["font_bundle"]]
fonts = [path for path in candidates if path.is_dir()]
assert len(fonts) == 1, "installed math font bundle absent or ambiguous"
expected = {record["name"] for record in catalog["files"]}
actual = {path.relative_to(fonts[0]).as_posix() for path in fonts[0].rglob("*") if path.is_file()}
assert actual == expected, f"font resource inventory changed: {sorted(actual ^ expected)}"
checked = []
for directory, records in [(fonts[0], catalog["files"]), (resources / "Licenses", catalog["license_files"])]:
    for record in records:
        path = directory / record["name"]
        assert not path.is_symlink(), "resource symlink not allowed"
        raw = path.read_bytes()
        assert len(raw) == record["bytes"] and hashlib.sha256(raw).hexdigest() == record["sha256"], f"resource differs: {path.name}"
        checked.append(record)
report = {"schema_version": 1, "revision": catalog["revision"], "default_face": catalog["default_face"], "checked_files": checked}
if args.output:
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
print(f"Installed SwiftMath font/metrics inventory and four license notices verified: {len(checked)} files")

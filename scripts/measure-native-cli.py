"""Record every original authority row from the actual Release CLI, without selecting retries."""
from pathlib import Path
import argparse
import json
import subprocess
import tomllib

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
args = parser.parse_args()
cases = tomllib.loads((root / "tests/corpus/solve.toml").read_text())["case"]
assert len(cases) == 53
args.output.parent.mkdir(parents=True, exist_ok=True)
rows = []
for case in cases:
    output = subprocess.run(
        [str(root / "target/release/om"), "--no-config", "--json", "--dialect", "wolfram", "-e", case["input"]],
        check=True, capture_output=True, text=True,
    )
    value = json.loads(output.stdout)
    rows.append({"id": case["id"], "input": case["input"], "input_form": value["items"][-1]["input_form"], "timing_ms": value["timing_ms"]})
    args.output.write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n")
failures = [(row["id"], row["timing_ms"]) for row in rows if row["timing_ms"] >= 200]
print(json.dumps({"rows": len(rows), "max_ms": max(row["timing_ms"] for row in rows), "failures": failures}))
raise SystemExit(bool(failures))

"""Compare the shared workbench status keys in Swift and TypeScript (§7.9)."""
from pathlib import Path
import json
import re

root = Path(__file__).resolve().parents[1]
swift = (root / "macos/OpenMathNative/Agent/UIStrings.swift").read_text()
web = (root / "app/src/i18n/index.ts").read_text()


def entries(text):
    pairs = re.findall(r'("(?:cell|doc|kernel|agent)\.[^"]+")\s*:\s*("(?:[^"\\]|\\.)*")', text)
    result = {}
    for key, value in pairs:
        key, value = json.loads(key), json.loads(value)
        if key in result:
            raise SystemExit(f"Duplicate status key: {key}")
        result[key] = value
    return result


native = entries(swift.split("static let values", 1)[1].split("static func text", 1)[0])
english_source, chinese_source = web.split("const zh: typeof en =", 1)
english, chinese = entries(english_source), entries(chinese_source)
required = {
    "cell.pending", "cell.running", "cell.verified", "cell.exact", "cell.numeric",
    "cell.conditional", "cell.stale", "cell.error", "cell.stopped",
    "doc.edited", "doc.saved", "kernel.ready", "agent.turn_ended",
    "agent.stopped", "agent.failed", "agent.budget", "agent.next_turn",
}
for platform, values in (("Mac", native), ("Web Chinese", chinese), ("Web English", english)):
    if values.keys() != required or not all(values.values()):
        raise SystemExit(f"{platform}: missing {required - values.keys()}, extra {values.keys() - required}, or empty text")
different = [key for key in sorted(required) if native[key] != chinese[key]]
if different:
    raise SystemExit(f"Mac/Web Chinese status text differs: {', '.join(different)}")
print(f"{len(required)} shared status keys match; English translations are present")

#!/usr/bin/env python3
"""Run the prompt's examples and language-card claims through the real CLI.

  python3 scripts/check_prompt_examples.py [--om target/release/om]

Fails when an example in agent/prompts/examples.zh.md now produces diagnostics or a different
result type, or when a documented language pitfall no longer behaves as described. Run it after
rebuilding the CLI so the prompt cannot teach syntax the kernel no longer accepts.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / 'agent/prompts/examples.zh.md'
# first word of the "内核返回" column -> item type reported by `om --json`
TYPES = {'Solutions': 'solutions', 'Plot': 'plot', 'Scene3D': 'scene3_d', '精确列表': 'expr'}
# (name, source, check on the parsed result)
CLAIMS = [
    ('index-from-1', 'let v = [10, 20, 30]; v[1]', lambda d: d['items'][-1].get('modern_form') == '10' and not d['messages']),
    ('literal-index-is-multiplication', '[10, 20, 30][1]', lambda d: bool(d['messages'])),
    ('newline-after-equals', 'let f(x) =\n  x^2 + 1;\nf(3)', lambda d: d['items'][-1].get('modern_form') == '10' and not d['messages']),
    ('double-equals-is-equation', 'solve(x^2 + 2*x == 3, x)', lambda d: d['items'][0]['type'] == 'solutions' and not d['messages']),
    ('records', 'let r = {a: 1, b: 2}; r.a + r.b', lambda d: d['items'][-1].get('modern_form') == '3' and not d['messages']),
    ('pipe', '[1, 2, 3] |> map(fn(x) => x^2)', lambda d: d['items'][-1].get('modern_form') == '[1, 4, 9]' and not d['messages']),
    ('numeric-integrate', 'integrate(exp(-x^2), x: -inf..inf, mode: "numeric")', lambda d: not d['messages']),
    ('no-nintegrate-method', 'NIntegrate[Sin[x^2],{x,0,1},Method->"GaussKronrod"]', lambda d: bool(d['messages'])),
    ('dependent-cells', 'let a = 5; a^2 + 1', lambda d: d['items'][-1].get('modern_form') == '26' and not d['messages']),
]


def run(om, source):
    proc = subprocess.run([om, '--json', '-e', source], capture_output=True, text=True, timeout=60)
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {'items': [], 'messages': [{'text': (proc.stdout + proc.stderr)[:200]}]}


def table_rows():
    for line in EXAMPLES.read_text(encoding='utf-8').splitlines():
        cells = [c.strip() for c in line.strip().strip('|').split('|')]
        if len(cells) != 3 or cells[0] in ('任务', '---'):
            continue
        match = re.fullmatch(r'`(.+)`', cells[1])
        if match:
            yield cells[0], match.group(1), cells[2]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--om', default=str(ROOT / 'target/release/om'))
    args = parser.parse_args()
    if not Path(args.om).exists():
        print(f'CLI not found: {args.om} (cargo build -p om-cli --release --locked)', file=sys.stderr)
        return 2
    failures = []
    count = 0
    for task, source, expected in table_rows():
        count += 1
        result = run(args.om, source)
        types = [i.get('type') for i in result.get('items', [])]
        want = next((TYPES[k] for k in TYPES if expected.startswith(k)), None)
        problem = None
        if result.get('messages'):
            problem = 'diagnostics: ' + '; '.join(str(m.get('text')) for m in result['messages'])
        elif want and want not in types:
            problem = f'expected {want}, got {types}'
        print(('FAIL ' if problem else 'ok   ') + task + (' -> ' + problem if problem else ''))
        if problem:
            failures.append(task)
    for name, source, predicate in CLAIMS:
        result = run(args.om, source)
        try:
            good = bool(predicate(result))
        except (KeyError, IndexError, TypeError):
            good = False
        print(('ok   ' if good else 'FAIL ') + 'claim: ' + name)
        if not good:
            failures.append(name)
    print(f'{count} examples, {len(CLAIMS)} claims, {len(failures)} failures')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())

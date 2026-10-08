"""只校验 .4 计划结构/覆盖；不执行运行时门禁，也不改变发行状态。"""
from __future__ import annotations

import argparse
import collections
import hashlib
import importlib.util
import json
import re
import tomllib
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / 'docs/plan/PRE_ALPHA_4.md'
PROGRESS = ROOT / 'docs/plan/PRE_ALPHA_4_PROGRESS.md'
GATES = ROOT / 'docs/acceptance/pre-alpha.4/gates.toml'
REPORT = ROOT / 'docs/acceptance/pre-alpha.4/planning-review.json'
EXPECTED_COUNTS = {0: 11, 1: 13, 2: 18, 3: 12, 4: 20, 5: 7, 6: 12, 7: 5}
SOURCE_NAMES = {
    'macos-host-state.md', 'macos-storage-recovery.md', 'macos-editor-rendering.md',
    'model-media.md', 'macos-model-media-ux.md', 'agent-context.md', 'agent-tools.md',
    'notebook-agent.md', 'macos-ux.md', 'macos-installation.md', 'agent-composer.md',
    'macos-native-ui.md', 'prompts/notebook-operation.md',
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def acyclic(dependencies: dict[str, list[str]]) -> None:
    seen: set[str] = set()
    active: set[str] = set()

    def visit(key: str) -> None:
        require(key not in active, f'循环依赖：{key}')
        if key in seen:
            return
        active.add(key)
        for dep in dependencies[key]:
            require(dep in dependencies, f'{key} 前置不存在：{dep}')
            visit(dep)
        active.remove(key)
        seen.add(key)

    for key in dependencies:
        visit(key)


def anchors(path: Path) -> set[str]:
    body = path.read_text(encoding='utf-8')
    ids = set(re.findall(r'<a\s+id="([^"]+)"', body))
    fence = False
    for line in body.splitlines():
        if line.startswith('```'):
            fence = not fence
        if fence:
            continue
        match = re.match(r'^#{1,6}\s+(.+)', line)
        if match:
            text = re.sub(r'\[([^\]]+)\]\([^)]*\)', r'\1', match[1])
            ids.add(re.sub(r'[^\w\-\s]', '', text.lower()).replace(' ', '-'))
    require(not fence, f'围栏未闭合：{path.relative_to(ROOT)}')
    return ids


def local_links(paths: list[Path]) -> tuple[int, int]:
    cache: dict[Path, set[str]] = {}
    count = optional = 0
    for path in paths:
        cache.setdefault(path, anchors(path))
        body = path.read_text(encoding='utf-8')
        for match in re.finditer(r'\[[^\]\n]+\]\(([^)\n]+)\)', body):
            url = match[1]
            if urlsplit(url).scheme:
                continue
            file, _, anchor = url.partition('#')
            if file.startswith('/Users/'):
                # 本地参考skill/recipes不是任何构建前置。
                optional += 1
                continue
            target = (path.parent / unquote(file)).resolve() if file else path
            require(target.exists(), f'链接文件不存在：{path.name} → {url}')
            if anchor and target.suffix == '.md':
                cache.setdefault(target, anchors(target))
                require(unquote(anchor) in cache[target], f'链接锚点不存在：{path.name} → {url}')
            count += 1
    return count, optional


def inspect_plan(plan: str, progress: str, gates: dict, *, initial: bool = False) -> dict:
    matches = list(re.finditer(r'^#### (R4\.\d\.\d{2}) (.+)$', plan, re.M))
    ids = [m[1] for m in matches]
    require(len(ids) == 98 and len(set(ids)) == 98, '必须有98个唯一任务ID')
    counts = collections.Counter(int(task.split('.')[1]) for task in ids)
    require(dict(counts) == EXPECTED_COUNTS, '阶段任务数量漂移')
    gate_ids = {g['id'] for g in gates['gate']}
    require(len(gate_ids) == 32, '必须有32个gate')
    task_states: dict[str, str] = {}
    task_deps: dict[str, list[str]] = {}
    contributions: dict[str, list[str]] = {gate: [] for gate in gate_ids}
    for i, match in enumerate(matches):
        body = plan[match.end():matches[i + 1].start() if i + 1 < len(matches) else plan.index('## 14.')]
        for field in ('Files', 'Depends', 'Interfaces', 'Steps', 'Tests', 'Done', 'Gates'):
            require(len(re.findall(rf'^\*\*{field}：\*\*', body, re.M)) == 1, f'{match[1]} 缺失/重复 {field}')
        state = re.findall(rf'^- \[([ x])\] \*\*{re.escape(match[1])} 完成\*\*', body, re.M)
        require(len(state) == 1, f'{match[1]} 没有唯一任务复选框')
        task_states[match[1]] = state[0]
        if initial:
            require(state[0] == ' ', f'{match[1]} 初始计划不能误标完成')
        dependencies = re.search(r'^\*\*Depends：\*\* (.+)$', body, re.M)
        require(dependencies is not None, '缺少前置')
        task_deps[match[1]] = re.findall(r'\[(R4\.\d\.\d{2})\]', dependencies[1])
        gs = re.search(r'^\*\*Gates：\*\* (.+)$', body, re.M)
        require(gs is not None, '缺少门禁')
        links = re.findall(r'\[(R4G\d{2})\]', gs[1])
        require(bool(links) and len(set(links)) == len(links), f'{match[1]} 没有唯一gate映射')
        for gate in links:
            require(gate in gate_ids, f'{match[1]} 使用未知gate：{gate}')
            contributions[gate].append(match[1])
    acyclic(task_deps)
    require('R4.5.07' in task_deps['R4.5.05'], '汇总器必须先于候选冻结提交')
    require(all(contributions.values()), '有gate没有任何贡献任务')
    ledger = re.findall(r'^\| \[([ x])\] \[(R4\.\d\.\d{2})\]', progress, re.M)
    require([task for _, task in ledger] == ids, '任务进度/主计划不一致')
    require(all(task_states[task] == state for state, task in ledger), '任务复选框状态不一致')
    case_ids = {case for g in gates['gate'] for case in g['required_case_ids']}
    require(len(case_ids) == 133, '必须有133条稳定case')
    mapping = plan[plan.index('### 16.12'):plan.index('## 17.')]
    for gate in gates['gate']:
        if initial:
            require(gate['check_status'] == 'not_run', f'{gate["id"]} 初始计划不能误标运行时状态')
        line = next((line for line in mapping.splitlines() if line.startswith(f'| <a id="gate-{gate["id"].lower()}">')), '')
        require(bool(line), f'{gate["id"]} 没有门禁映射行')
        cells = line.split('|')
        for case in gate['required_case_ids']:
            require(case in re.findall(r'\b[A-Z]+\d*\b', cells[2]), f'{gate["id"]} 缺case：{case}')
        require(re.findall(r'R4\.\d\.\d{2}', cells[3]) == contributions[gate['id']], f'{gate["id"]} 任务映射漂移')
    acyclic({g['id']: g['depends_on'] for g in gates['gate']})
    if initial:
        require(not gates['ready_to_publish'] and not gates['released_verified'], '初始计划不能标发行就绪')
        require(gates['candidate_sha'] == 'UNSET', '初始计划不能分配candidate')
        require(gates['current_runtime_version'] == '0.1.0-pre-alpha.3', '初始计划不切换运行版本')
    require(sum(g['pre_publish_required'] for g in gates['gate']) == 31, '公开前gate数量错误')
    require(sum(g['post_publish_required'] for g in gates['gate']) == 1, '公开后gate数量错误')
    loader = importlib.util.spec_from_file_location('openmath_release_plan_check', ROOT / 'scripts/release.py')
    require(loader is not None and loader.loader is not None, '无法加载资产命名函数')
    release = importlib.util.module_from_spec(loader)
    loader.loader.exec_module(release)
    assets = gates['asset_set']['expected_names']
    require(len(set(assets)) == 9 and set(assets) == release.asset_names(gates['target_version']), '九资产命名漂移')
    final = plan[plan.index('### 18.2'):plan.index('### 18.3')]
    require(all(f'`{asset}`' in final for asset in assets), '最终交付缺资产')
    source_section = plan[plan.index('### 19.1'):plan.index('### 19.2')]
    sources = re.findall(r'\| \[[^\]]+\]\(\.\./design/([^)]*)\) \| ([\d.]+) \| `([a-f0-9]{64})` \|', source_section)
    require({s[0] for s in sources} == SOURCE_NAMES, '13篇设计正文未全部纳入')
    for source, section, sha in sources:
        require(digest(ROOT / 'docs/design' / source) == sha, f'设计来源已漂移：{source}')
        require(f'{section} ' in plan, f'缺少来源章节：{source}')
    return {'task_count': len(ids), 'phase_task_counts': dict(counts), 'gate_count': len(gate_ids),
            'unique_case_count': len(case_ids), 'pre_publish_gate_count': 31, 'post_publish_gate_count': 1,
            'source_design_count': len(sources), 'asset_count': len(assets), 'task_dependency_dag_valid': True,
            'gate_dependency_dag_valid': True, 'all_tasks_and_gates_mapped': True}


def self_test(plan: str, progress: str, gates: dict) -> int:
    bad_plans = [
        plan.replace('#### R4.0.01 ', '#### R4.0.02 ', 1),
        plan.replace('**Depends：** 无；以第0节基线为前置。', '**Depends：** [R4.0.01](#task-r4.0.01)', 1),
        plan.replace('**Depends：** 无；以第0节基线为前置。', '**Depends：** [R4.0.99](#task-r4.0.99)', 1),
        plan.replace('[R4G01](#gate-r4g01)', '[R4G99](#gate-r4g99)', 1),
        plan.replace('**Done：**', '**Missing：**', 1),
        plan.replace('PFMAC |', 'PFVOID |', 1),
        plan.replace('OpenMath-web_0.1.0-pre-alpha.4.zip', 'missing-web.zip'),
    ]
    for i, bad in enumerate(bad_plans):
        try:
            inspect_plan(bad, progress, gates)
        except ValueError:
            continue
        raise ValueError(f'计划checker没有拒绝反例{i}')
    return len(bad_plans)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='校验结构/账本，不执行运行门禁')
    parser.add_argument('--initial', action='store_true', help='额外核对本次初始计划尚无任务/gate执行')
    parser.add_argument('--write-report', action='store_true', help='保存设计检查摘要，不标runtime gate')
    parser.add_argument('--self-test', action='store_true', help='验证结构错误/循环/漏项会被拒绝')
    args = parser.parse_args()
    plan = PLAN.read_text(encoding='utf-8')
    progress = PROGRESS.read_text(encoding='utf-8')
    gates = tomllib.loads(GATES.read_text(encoding='utf-8'))
    result = inspect_plan(plan, progress, gates, initial=args.initial)
    paths = [PLAN, PROGRESS, ROOT / 'docs/plan/NEXT_RELEASE.md', ROOT / 'docs/plan/PLAN.md',
             ROOT / 'docs/README.md', ROOT / 'CODEX_HANDOFF.md', ROOT / 'docs/acceptance/pre-alpha.4/README.md']
    links, optional = local_links(paths)
    result.update({'schema_version': 1, 'check_kind': 'design_plan_structure', 'status': 'passed',
                   'markdown_local_links_checked': links, 'optional_local_reference_links': optional,
                   'runtime_gate_checks_executed': 0, 'ready_to_publish': False, 'released_verified': False,
                   'negative_cases_rejected': self_test(plan, progress, gates) if args.self_test else None,
                   'initial_baseline_checked': args.initial,
                   'inputs': [{'path': str(p.relative_to(ROOT)), 'sha256': digest(p)} for p in paths + [GATES, Path(__file__).resolve()]],
                   'command': 'python3 scripts/check_pre_alpha_4_plan.py --check --initial --self-test' if args.initial else 'python3 scripts/check_pre_alpha_4_plan.py --check --self-test'})
    if args.write_report:
        REPORT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k != 'inputs'}, ensure_ascii=False))


if __name__ == '__main__':
    main()

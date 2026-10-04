"""校验功能目录并确定性生成中文参考文档；不修改运行时注册表。"""
from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "docs/reference/functions.toml"
IDENTITIES = ROOT / "docs/reference/function-identities.json"
STATUSES = {"implemented": "已实现", "partial": "部分支持", "planned": "下一版规划", "deferred": "后续规划"}
INTERFACES = {"current": "当前可用", "partial": "现有入口可用，统一接口待实施", "planned": "规划接口，当前不可用"}
PLATFORMS = {"cli", "desktop", "web", "ios"}
EFFECTS = {"pure", "read_session", "write_session", "write_document", "host_io", "unclassified"}
REQUIRED = {"id", "name", "category", "status", "interface_status", "runtime_names", "compatibility_names", "current_modern_signatures", "current_wolfram_signatures", "signatures", "parameters", "return_type", "semantics", "current_support", "target_support", "precision_modes", "precision_notes", "current_platforms", "target_platforms", "render_platforms", "current_examples", "planned_examples", "sources", "tests", "acceptance", "pipe_arg", "target_version", "kind", "effect_class", "validation_stage"}


def reference_path(value: str) -> Path:
    """引用只能指向仓库内真实文件，可在 # 后列测试名。"""
    path = (ROOT / value.split("#", 1)[0]).resolve()
    if not path.is_relative_to(ROOT) or not path.is_file():
        raise ValueError(f"无效仓库引用：{value}")
    return path


def validate(catalog: dict) -> None:
    if catalog.get("schema_version") != 1:
        raise ValueError("目录 schema_version 必须为 1")
    categories = catalog.get("categories", [])
    category_ids = [c["id"] for c in categories]
    if len(set(category_ids)) != len(category_ids):
        raise ValueError("分类 ID 重复")
    entries = catalog.get("functions", [])
    if not entries:
        raise ValueError("功能目录为空")
    ids, names, runtime_names = set(), set(), set()
    for entry in entries:
        missing = REQUIRED - entry.keys()
        if missing:
            raise ValueError(f"{entry.get('id')} 缺少字段：{sorted(missing)}")
        ident = entry["id"]
        if not re.fullmatch(r"[a-z][a-z0-9_]*", ident) or ident in ids:
            raise ValueError(f"无效或重复 ID：{ident}")
        if not re.fullmatch(r"[a-z][a-z0-9_]*", entry["name"]) or entry["name"] in names:
            raise ValueError(f"无效或重复规范名：{entry['name']}")
        ids.add(ident)
        names.add(entry["name"])
        if entry["category"] not in category_ids:
            raise ValueError(f"{ident} 的分类不存在")
        if entry["status"] not in STATUSES or entry["interface_status"] not in INTERFACES:
            raise ValueError(f"{ident} 的状态无效")
        if entry["kind"] not in {"function", "operator", "constant"}:
            raise ValueError(f"{ident} 的符号类型无效")
        if entry["effect_class"] not in EFFECTS or entry["validation_stage"] != "documentation_only":
            raise ValueError(f"{ident} 的预留副作用或参数验证阶段无效")
        if entry["status"] == "planned" and not entry["planned_examples"]:
            raise ValueError(f"{ident} 缺少下一版示例")
        if not isinstance(entry["pipe_arg"], int) or entry["pipe_arg"] < 0:
            raise ValueError(f"{ident} 的管道参数位置无效")
        for field in ("current_platforms", "target_platforms", "render_platforms"):
            if not set(entry[field]).issubset(PLATFORMS):
                raise ValueError(f"{ident} 的 {field} 无效")
        if entry["status"] in {"implemented", "partial"}:
            if not all(entry[field] for field in ("runtime_names", "current_support", "current_examples", "sources", "tests", "current_wolfram_signatures")):
                raise ValueError(f"{ident} 声称现有实现但缺少注册或证据")
        elif entry["runtime_names"] or entry["current_examples"] or entry["current_platforms"]:
            raise ValueError(f"{ident} 把规划条目标成当前可用")
        if entry["status"] == "planned" and entry["target_version"] != catalog["target_version"]:
            raise ValueError(f"{ident} 的目标版本不一致")
        if entry["interface_status"] == "current" and not entry["runtime_names"]:
            raise ValueError(f"{ident} 的接口无当前注册")
        for name in entry["runtime_names"]:
            if name in runtime_names:
                raise ValueError(f"注册名覆盖重复：{name}")
            runtime_names.add(name)
        if not all(entry[field] for field in ("semantics", "target_support", "signatures", "return_type", "acceptance", "precision_notes")):
            raise ValueError(f"{ident} 缺少行为、签名、边界或验收")
        parameter_names = set()
        for param in entry["parameters"]:
            if set(param) != {"name", "kind", "default", "description", "availability"}:
                raise ValueError(f"{ident} 参数字段不完整")
            if param["name"] in parameter_names or param["kind"] not in {"positional", "option", "axis", "variadic"}:
                raise ValueError(f"{ident} 参数重复或类型错误")
            if param["availability"] not in {"current", "r3", "later"}:
                raise ValueError(f"{ident} 参数可用阶段错误")
            parameter_names.add(param["name"])
        for ref in entry["sources"] + entry["tests"]:
            path = reference_path(ref)
            if "#" in ref:
                symbol = ref.split("#", 1)[1]
                if not re.search(r"\bfn\s+" + re.escape(symbol) + r"\s*\(", path.read_text(encoding="utf-8")):
                    raise ValueError(f"测试名不存在：{ref}")
    for feature in catalog.get("features", []):
        if feature["status"] not in STATUSES:
            raise ValueError(f"功能族状态无效：{feature['id']}")
        for ref in feature["evidence"]:
            reference_path(ref)


def validate_identities(catalog: dict, identities: dict) -> None:
    """稳定 ID 与名称分离；现有注册名不能被无声归到另一身份。"""
    locked = identities["identities"]
    ids = [value["id"] for value in locked]
    if identities.get("schema_version") != 1 or len(ids) != len(set(ids)):
        raise ValueError("身份账本无效")
    if {e["id"] for e in catalog["functions"]} != set(ids):
        raise ValueError("目录身份变更必须显式追加/迁移账本，不得删除或复用稳定 ID")
    owners = {name: e["id"] for e in catalog["functions"] for name in e["runtime_names"]}
    for item in locked:
        for name in item["runtime_names"]:
            if owners.get(name) != item["id"]:
                raise ValueError(f"实际注册名 {name} 的稳定身份发生变化")


def cell(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", "<br>")


def code(value: str) -> str:
    fence = "``" if "`" in value else "`"
    return fence + value + fence


def evidence(values: list[str]) -> str:
    return "、".join(f"[{cell(v)}](../../{v.split('#', 1)[0]})" for v in values) or "暂无当前实现证据"


def render(catalog: dict) -> dict[Path, str]:
    generated = "<!-- 由 scripts/function_docs.py 生成；编辑 functions.toml 后重新生成。 -->\n\n"
    entries = catalog["functions"]
    counts = Counter(e["status"] for e in entries)
    index = [generated, "# 全景功能目录\n\n", f"当前发行：`{catalog['current_version']}`；下一版目标：`{catalog['target_version']}`。\n\n",
             "此目录是设计与真实实现的对照。规范名称不等于当前已接受的调用名；每项分别列当前签名和目标签名。规划条目不会自动进入运行时或可执行补全。别名不作为新增数学能力计数。\n\n",
             "稳定 ID 使用独立的 `fn_000001` 等身份，不随名称/别名变化；[身份账本](function-identities.json)记录初始归属。副作用标签是设计预留，不是运行时授权；`unclassified` 不授予执行能力。当前参数默认值是文字说明，尚非可验证的 Agent 工具 schema，能力查询/Notebook事务仍未实现。\n\n",
             "| 状态 | 条目数 | 含义 |\n|---|---:|---|\n"]
    descriptions = {"implemented": "在所列支持范围内已有实现", "partial": "已有真实入口，但数学范围或目标接口未完整交付", "planned": "锁定下一版，当前不可用", "deferred": "进入全景目录，下一版不承诺实现"}
    for status, label in STATUSES.items():
        index.append(f"| {label} | {counts[status]} | {descriptions[status]} |\n")
    index.append(f"\n当前登记 `{sum(len(e['runtime_names']) for e in entries)}` 个真实注册名；条目按语义归并，与注册名数量不同。\n\n")
    index.append("| 分类 | 条目数 | 已实现 / 部分 / 下一版 / 后续 |\n|---|---:|---|\n")
    outputs = {}
    for category in catalog["categories"]:
        group = sorted((e for e in entries if e["category"] == category["id"]), key=lambda e: e["name"])
        c = Counter(e["status"] for e in group)
        index.append(f"| [{category['title']}]({category['id']}.md) | {len(group)} | " + " / ".join(str(c[s]) for s in STATUSES) + " |\n")
        parts = [generated, f"# {category['title']}\n\n[全景目录](README.md) · [现代语言设计](../design/modern-language.md) · [下一版账本](../plan/NEXT_RELEASE.md)\n\n",
                 "所有示例区分当前 Wolfram/现代入口与规划的现代接口。`precision` 的出现不代表任意精度能力；以每项精度说明为准。\n\n",
                 "| 规范名称 | 当前实现 | 目标接口状态 | 数学含义 |\n|---|---|---|---|\n"]
        for e in group:
            parts.append(f"| [{code(e['name'])}](#{e['name']}) | {STATUSES[e['status']]} | {INTERFACES[e['interface_status']]} | {cell(e['semantics'])} |\n")
        for e in group:
            parts.extend([f"\n## {e['name']}\n\n", f"**当前实现：{STATUSES[e['status']]}；目标接口：{INTERFACES[e['interface_status']]}。** 目标版本：`{e['target_version'] or '后续未定'}`。\n\n",
                          f"- 稳定身份：`{e['id']}`；条目类型：`{e['kind']}`。\n",
                          f"- 副作用分类（设计预留）：`{e['effect_class']}`；参数验证阶段：`{e['validation_stage']}`，不构成工具授权。\n\n",
                          e["semantics"] + "\n\n", "- 当前支持：" + e["current_support"] + "\n", "- 目标范围：" + e["target_support"] + "\n",
                          "- 返回：" + e["return_type"] + "\n", "- 精度：" + e["precision_notes"] + "\n",
                          "- 当前计算平台：" + (", ".join(e["current_platforms"]) or "无") + "；目标计算平台：" + (", ".join(e["target_platforms"]) or "未承诺") + "。\n",
                          "- 目标图形/交互展示平台：" + (", ".join(e["render_platforms"]) or "不适用或后续未定") + "。\n",
                          "- 兼容名称：" + ("、".join(code(n) for n in e["compatibility_names"]) or "无既有兼容入口") + "。\n",
                          "- 管道位置：" + (f"第 {e['pipe_arg']} 个位置参数（从 1 起）" if e["pipe_arg"] else "不接受自动管道输入") + "。\n\n"])
            if e["current_modern_signatures"]:
                parts.append("当前现代签名：\n\n```text\n" + "\n".join(e["current_modern_signatures"]) + "\n```\n\n")
            if e["current_wolfram_signatures"]:
                parts.append("当前 Wolfram 签名：\n\n```text\n" + "\n".join(e["current_wolfram_signatures"]) + "\n```\n\n")
            parts.append("目标现代签名（按目标接口状态判断是否已可执行）：\n\n```text\n" + "\n".join(e["signatures"]) + "\n```\n\n")
            parts.append("| 参数 | 类型 | 默认值 | 含义 | 可用阶段 |\n|---|---|---|---|---|\n")
            for p in e["parameters"]:
                parts.append(f"| {code(p['name'])} | {p['kind']} | {cell(p['default'])} | {cell(p['description'])} | {p['availability']} |\n")
            if not e["parameters"]:
                parts.append("| 无 | — | — | 无参数 | — |\n")
            if e["current_examples"]:
                parts.append("\n当前已登记示例（Wolfram）：\n\n```wolfram\n" + "\n".join(e["current_examples"]) + "\n```\n")
            if e["planned_examples"]:
                parts.append("\n规划示例（尚未执行；需要目标版本，后续条目不承诺 .3）：\n\n```text\n" + "\n".join(e["planned_examples"]) + "\n```\n")
            parts.append("\n验收：" + e["acceptance"] + "\n\n当前源码：" + evidence(e["sources"]) + "。\n\n当前测试引用：" + evidence(e["tests"]) + "。\n")
        outputs[ROOT / "docs/reference" / (category["id"] + ".md")] = "".join(parts)
    index.append("\n## 产品与平台功能\n\n| 功能族 | 当前状态 | 当前/目标范围 | 证据 |\n|---|---|---|---|\n")
    for feature in catalog.get("features", []):
        index.append(f"| {feature['title']} | {STATUSES[feature['status']]} | {cell(feature['scope'])} | {evidence(feature['evidence'])} |\n")
    index.append("\n## 常量、定义域与选项符号\n\n符号识别不等于函数或求解域已实现；下列当前含义独立列明，不计入真实回调数量。\n\n| 名称 | 兼容名 | 含义与边界 |\n|---|---|---|\n")
    for symbol in catalog.get("symbols", []):
        index.append(f"| {code(symbol['name'])} | {cell(', '.join(symbol['aliases']))} | {cell(symbol['meaning'])} |\n")
    index.append("\n## 维护与验证\n\n```sh\npython3 scripts/function_docs.py --check\nCARGO_PROFILE_TEST_OPT_LEVEL=2 cargo test -p om-eval --test function_catalog --locked\n```\n\n目录校验检查状态、稳定身份、副作用预留、必填字段、证据和生成文件一致性；Rust 契约比较真实注册表与当前示例。测试引用表示已有覆盖入口，不代替本轮执行日志，也不意味着规划接口已实现。后续类型/必填/枚举/范围及实际默认值接入时另升级描述版本，不能将文字说明直接传给模型。\n")
    outputs[ROOT / "docs/reference/README.md"] = "".join(index)
    return outputs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="只检查，不改写文件")
    args = parser.parse_args()
    catalog = tomllib.loads(CATALOG.read_text(encoding="utf-8"))
    validate(catalog)
    import json
    validate_identities(catalog, json.loads(IDENTITIES.read_text(encoding="utf-8")))
    outputs = render(catalog)
    stale = [path for path, text in outputs.items() if not path.is_file() or path.read_text(encoding="utf-8") != text]
    if args.check and stale:
        print("参考文档过期：" + ", ".join(str(p.relative_to(ROOT)) for p in stale), file=sys.stderr)
        return 1
    if not args.check:
        for path, text in outputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    print(f"目录有效：{len(catalog['functions'])} 个语义条目，{len(outputs)} 个参考页面")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

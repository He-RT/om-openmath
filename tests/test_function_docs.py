"""目录必须保留真实支持边界、稳定身份和未实现的Agent预留。"""
import copy
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))

ROOT = Path(__file__).resolve().parents[1]


class FunctionDocsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("function_docs", ROOT / "scripts/function_docs.py")
        cls.docs = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.docs)
        cls.catalog = tomllib.loads((ROOT / "docs/reference/functions.toml").read_text(encoding="utf-8"))
        cls.identities = json.loads((ROOT / "docs/reference/function-identities.json").read_text(encoding="utf-8"))

    def test_catalog_validates_and_generated_documents_are_current(self):
        self.docs.validate(self.catalog)
        self.docs.validate_identities(self.catalog, self.identities)
        for path, expected in self.docs.render(self.catalog).items():
            self.assertEqual(path.read_text(encoding="utf-8"), expected, path.name)
        for path, expected in self.docs.render_runtime(self.catalog).items():
            self.assertEqual(path.read_text(encoding="utf-8"), expected, path.name)

    def test_planning_cannot_claim_current_callbacks_or_platforms(self):
        for field, value in [("runtime_names", ["ImaginaryCallback"]), ("current_platforms", ["web"]), ("current_examples", ["ImaginaryCallback[]"])]:
            catalog = copy.deepcopy(self.catalog)
            entry = next(e for e in catalog["functions"] if e["status"] in {"planned", "deferred"})
            entry["status"] = "planned"
            entry[field] = value
            with self.assertRaises(ValueError):
                self.docs.validate(catalog)

    def test_rename_keeps_identity_but_rebinding_a_callback_is_rejected(self):
        catalog = copy.deepcopy(self.catalog)
        entry = next(e for e in catalog["functions"] if "Abs" in e["runtime_names"])
        self.assertNotEqual(entry["id"], entry["name"])
        entry["name"] = "absolute_value_renamed"
        self.docs.validate(catalog)
        self.docs.validate_identities(catalog, self.identities)
        other = next(e for e in catalog["functions"] if "Sin" in e["runtime_names"])
        entry["runtime_names"].remove("Abs")
        other["runtime_names"].append("Abs")
        with self.assertRaises(ValueError):
            self.docs.validate_identities(catalog, self.identities)

    def test_agent_tools_remain_deferred_and_no_session_data_is_a_notebook_field(self):
        entries = {e["name"]: e for e in self.catalog["functions"] if e["category"] == "agent"}
        self.assertEqual(set(entries), {"read_notebook", "apply_notebook_patch", "run_cells", "get_function_docs", "undo_transaction", "cancel_task"})
        for entry in entries.values():
            self.assertEqual(entry["status"], "deferred")
            self.assertEqual(entry["validation_stage"], "documentation_only")
            self.assertEqual(entry["runtime_names"], [])
            self.assertEqual(entry["current_platforms"], [])
        plan = (ROOT / "docs/plan/NEXT_RELEASE.md").read_text(encoding="utf-8")
        for text in ["尚未实现", "document_revision", "operation_id", "不静默覆盖", "不写入笔记本", "不安装 Pi/Rig"]:
            self.assertIn(text, plan)

    def test_runtime_schema_rejects_missing_callback_or_fictitious_defaults(self):
        catalog = copy.deepcopy(self.catalog)
        catalog["runtime"].pop()
        with self.assertRaises(ValueError):
            self.docs.validate(catalog)
        catalog = copy.deepcopy(self.catalog)
        runtime = next(r for r in catalog["runtime"] if r["name"] == "FindRoot")
        runtime["options"][0]["default_source"] = "省略"
        with self.assertRaises(ValueError):
            self.docs.validate(catalog)
        catalog = copy.deepcopy(self.catalog)
        catalog["runtime"][0]["aliases"].append("ode")
        catalog["runtime"][1]["aliases"].append("ode")
        with self.assertRaises(ValueError):
            self.docs.validate(catalog)

    def test_invalid_evidence_or_effect_class_is_rejected(self):
        catalog = copy.deepcopy(self.catalog)
        entry = next(e for e in catalog["functions"] if e["runtime_names"])
        entry["tests"] = ["crates/om-eval/tests/scalars.rs#missing_catalog_test"]
        with self.assertRaises(ValueError):
            self.docs.validate(catalog)

    def test_fixed_positional_arity_cannot_silently_omit_a_parameter(self):
        catalog=copy.deepcopy(self.catalog)
        entry=next(r for r in catalog['runtime'] if r['name']=='Slice')
        entry['parameters'].pop()
        with self.assertRaises(ValueError):
            self.docs.validate(catalog)
        entry["tests"] = self.catalog["functions"][0]["tests"]
        entry["effect_class"] = "allow_everything"
        with self.assertRaises(ValueError):
            self.docs.validate(catalog)


if __name__ == "__main__":
    unittest.main()

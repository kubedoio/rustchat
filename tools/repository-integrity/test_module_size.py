#!/usr/bin/env python3
"""Acceptance tests for the RI-C05 module-growth-control guard.

Validates the P004 acceptance criteria:
  * public behavior is preserved by decomposition (verified separately by Rust);
  * the guard trips on a newly monolithic module and on material growth of a
    large module;
  * the guard has an exception mechanism (RI-C10-style);
  * line movement / modest changes within thresholds create no false debt.
"""

from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

TOOL_DIR = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOL_DIR))

import check_module_size as guard  # noqa: E402


def build_tree(files: dict[str, str], baseline: str = "", exceptions: str = "") -> pathlib.Path:
    root = pathlib.Path(tempfile.mkdtemp(prefix="ri-c05-test-"))
    for rel, content in files.items():
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content, encoding="utf-8")
    base_dir = root / "tools" / "repository-integrity" / "baselines"
    base_dir.mkdir(parents=True, exist_ok=True)
    (base_dir / "large-production-modules.txt").write_text(baseline, encoding="utf-8")
    exc_dir = root / "tools" / "repository-integrity" / "exceptions"
    exc_dir.mkdir(parents=True, exist_ok=True)
    (exc_dir / "module-size.txt").write_text(exceptions, encoding="utf-8")
    return root


def baseline_of(counts: dict[str, int]) -> str:
    return "\n".join(f"{p} {c}" for p, c in sorted(counts.items())) + "\n"


N = "\n"  # short alias


class GuardTests(unittest.TestCase):
    def test_clean_tree_passes(self):
        root = build_tree(
            {"backend/src/a.rs": "fn a(){}\n" * 100},
            baseline=baseline_of({"backend/src/a.rs": 100}),
        )
        self.assertEqual(guard.run_checks(root), [])

    def test_new_monolithic_module_fails(self):
        root = build_tree(
            {"backend/src/big.rs": "fn x(){}\n" * (guard.NEW_THRESHOLD + 1)},
            baseline="",
        )
        issues = guard.run_checks(root)
        self.assertTrue(any("newly added production module" in i for i in issues))

    def test_new_small_module_ok(self):
        root = build_tree(
            {"backend/src/small.rs": "fn x(){}\n" * 100},
            baseline="",
        )
        self.assertEqual(guard.run_checks(root), [])

    def test_material_growth_of_large_module_fails(self):
        # Baseline 1000; now 1100 (> +100 and > +5%) -> should fail.
        root = build_tree(
            {"backend/src/big.rs": "fn x(){}\n" * 1100},
            baseline=baseline_of({"backend/src/big.rs": 1000}),
        )
        issues = guard.run_checks(root)
        self.assertTrue(any("material growth" in i for i in issues))

    def test_modest_change_no_false_debt(self):
        # Baseline 1000; now 1050 (exactly +5%, below +100 threshold) -> ok.
        root = build_tree(
            {"backend/src/big.rs": "fn x(){}\n" * 1050},
            baseline=baseline_of({"backend/src/big.rs": 1000}),
        )
        self.assertEqual(guard.run_checks(root), [])

    def test_small_module_growth_not_tracked(self):
        # Neither baseline nor current crosses the large threshold -> ok.
        root = build_tree(
            {"backend/src/a.rs": "fn x(){}\n" * 400},
            baseline=baseline_of({"backend/src/a.rs": 100}),
        )
        self.assertEqual(guard.run_checks(root), [])

    def test_reduction_ok(self):
        root = build_tree(
            {"backend/src/big.rs": "fn x(){}\n" * 900},
            baseline=baseline_of({"backend/src/big.rs": 1200}),
        )
        self.assertEqual(guard.run_checks(root), [])

    def test_exception_relaxes_growth(self):
        root = build_tree(
            {"backend/src/big.rs": "fn x(){}\n" * 1300},
            baseline=baseline_of({"backend/src/big.rs": 1000}),
            exceptions="backend/src/big.rs  justification  note  big_test.rs\n",
        )
        self.assertEqual(guard.run_checks(root), [])

    def test_inventory_scopes_production_and_skips_target(self):
        root = build_tree(
            {
                "backend/src/code.rs": "fn x(){}\n" * 50,
                "backend/target/debug/build/vendor/lib.rs": "\n" * 5000,
            }
        )
        inv = guard.inventory(root)
        self.assertIn("backend/src/code.rs", inv)
        self.assertNotIn("backend/target/debug/build/vendor/lib.rs", inv)


if __name__ == "__main__":
    unittest.main(verbosity=2)

#!/usr/bin/env python3
"""Acceptance tests for the RI-C04 api-persistence-boundary guard.

Validates the P003 acceptance criteria:
  * baseline generation is deterministic;
  * adding a synthetic new call site makes the guard fail;
  * moving line numbers without changing call sites does not create false debt;
  * existing debt does not require a mass rewrite (baseline == current debt);
  * a registered exception relaxes the tripwire for a justified path.
"""

from __future__ import annotations

import pathlib
import shutil
import sys
import tempfile
import unittest

# Make the guard module importable without a package.
TOOL_DIR = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOL_DIR))

import check_api_direct_sql as guard  # noqa: E402


def build_tree(files: dict[str, str], baseline: str = "", exceptions: str = "") -> pathlib.Path:
    """Create a temp tree with backend/src/api files, baseline and exceptions."""
    root = pathlib.Path(tempfile.mkdtemp(prefix="ri-c04-test-"))
    for rel, content in files.items():
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content, encoding="utf-8")
    base_dir = root / "tools" / "repository-integrity" / "baselines"
    base_dir.mkdir(parents=True, exist_ok=True)
    (base_dir / "api-direct-sql.txt").write_text(baseline, encoding="utf-8")
    exc_dir = root / "tools" / "repository-integrity" / "exceptions"
    exc_dir.mkdir(parents=True, exist_ok=True)
    (exc_dir / "api-direct-sql.txt").write_text(exceptions, encoding="utf-8")
    return root


def root_with_baseline(files, counts):
    base = "\n".join(f"{p} {c}" for p, c in sorted(counts.items())) + "\n"
    return build_tree(files, baseline=base)


class CountTests(unittest.TestCase):
    def test_counts_tokens_not_lines_and_ignores_comments(self):
        text = (
            "sqlx::query_as(\"a\").await?;\n"
            "sqlx::query_as(\"b\").await?;\n"
            "sqlx::query(\"c\").await?;\n"
            "// sqlx::query comment-only mention\n"
            "let flag = true;\n"
            "sqlx::QueryBuilder::new(\"d\")\n"
        )
        # Two query_as + one query + one QueryBuilder = 4 (comment not counted).
        self.assertEqual(guard.count_direct_sql(text), 4)

    def test_multiple_tokens_on_one_line(self):
        text = "let a = sqlx::query_as(\"x\"); let b = sqlx::query_scalar(\"y\");"
        self.assertEqual(guard.count_direct_sql(text), 2)


class DeterminismTests(unittest.TestCase):
    def test_regen_is_stable_across_runs(self):
        files = {"backend/src/api/users.rs": "sqlx::query(\"a\")\nsqlx::query_as(\"b\")\n"}
        root = build_tree(files)
        a = guard.regenerate_lines(root)
        b = guard.regenerate_lines(root)
        self.assertEqual(a, b)

    def test_line_move_does_not_change_count(self):
        original = "fn f() {\n  sqlx::query_as(\"a\").await?;\n  sqlx::query(\"b\").await?;\n}\n"
        reflowed = (
            "// added a leading comment to shift line numbers\n"
            "fn f() {\n"
            "\n"
            "\n"
            "  sqlx::query_as(\"a\")\n"
            "    .await?;\n"
            "  // interleaved comment that is not SQL\n"
            "  sqlx::query(\"b\")\n"
            "    .await?;\n"
            "}\n"
        )
        self.assertEqual(
            guard.count_direct_sql(original), guard.count_direct_sql(reflowed)
        )


class GuardTests(unittest.TestCase):
    def test_clean_tree_passes(self):
        files = {"backend/src/api/users.rs": "sqlx::query(\"a\")\n"}
        root = root_with_baseline(files, {"backend/src/api/users.rs": 1})
        issues, _ = guard.run_checks(root)
        self.assertEqual(issues, [])

    def test_new_file_with_sql_fails(self):
        files = {
            "backend/src/api/users.rs": "sqlx::query(\"a\")\n",
            "backend/src/api/brand_new.rs": "sqlx::query_as(\"b\")\n",
        }
        root = root_with_baseline(files, {"backend/src/api/users.rs": 1})
        issues, _ = guard.run_checks(root)
        self.assertTrue(any("new production API file" in i and "brand_new.rs" in i for i in issues))

    def test_increased_count_fails(self):
        files = {"backend/src/api/users.rs": "sqlx::query(\"a\")\n\nsqlx::query(\"c\")\n"}
        root = root_with_baseline(files, {"backend/src/api/users.rs": 1})
        issues, _ = guard.run_checks(root)
        self.assertTrue(any("increased" in i and "users.rs" in i for i in issues))

    def test_decreased_count_is_not_a_tripwire(self):
        # Guard logic only rejects growth; a reduction must not be flagged.
        files = {"backend/src/api/users.rs": "sqlx::query(\"a\")\n"}
        root = root_with_baseline(files, {"backend/src/api/users.rs": 3})
        issues, _ = guard.run_checks(root)
        self.assertEqual(issues, [])

    def test_exception_allows_increased_count(self):
        files = {"backend/src/api/users.rs": "sqlx::query(\"a\")\n\nsqlx::query(\"c\")\n"}
        base = "backend/src/api/users.rs 1\n"
        exc = ("backend/src/api/users.rs  justification  ADR-006 RI-C04 note  user_test.rs\n")
        root = build_tree(files, baseline=base, exceptions=exc)
        issues, _ = guard.run_checks(root)
        self.assertEqual(issues, [])


class FreshnessTests(unittest.TestCase):
    def test_stale_baseline_is_detected(self):
        files = {"backend/src/api/users.rs": "sqlx::query(\"a\")\n"}
        # Baseline says 0 (users.rs absent) while the tree has a call -> stale.
        root = build_tree(files, baseline="backend/src/api/other.rs 2\n")
        issues, fresh = guard.run_checks(root)
        self.assertFalse(fresh)

    def test_existing_debt_does_not_require_rewrite(self):
        # A tree whose baseline equals current debt passes with no changes.
        files = {"backend/src/api/a.rs": "sqlx::query(\"1\")\nsqlx::query(\"2\")\n"}
        root = root_with_baseline(files, {"backend/src/api/a.rs": 2})
        issues, fresh = guard.run_checks(root)
        self.assertEqual(issues, [])
        self.assertTrue(fresh)


if __name__ == "__main__":
    unittest.main(verbosity=2)

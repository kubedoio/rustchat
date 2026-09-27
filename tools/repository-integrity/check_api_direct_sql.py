#!/usr/bin/env python3
"""RI-C04 api-persistence-boundary guard.

Direct production SQL in API handlers is baseline debt and must not grow
silently (see ADR-006 RI-C04). This guard inventory production direct-SQL
usage under ``backend/src/api/**`` and rejects:

  * direct SQL in a newly created production API file; and
  * an increased normalized direct-SQL count in an existing API file,

unless the affected path is registered in the RI-C04 exception register
(``tools/repository-integrity/exceptions/api-direct-sql.txt``).

The normalized count is the number of direct-SQL invocation tokens
(``sqlx::query``, ``sqlx::query_as``, ``sqlx::query_scalar``,
``sqlx::query_with``, ``sqlx::query_file``, ``sqlx::raw_sql`` and
``sqlx::QueryBuilder``) present in a file after stripping comments.

The baseline is deterministic: it depends only on the set of invocation
tokens, never on absolute line numbers, so moving lines around does not
create false debt.

Usage:
  check_api_direct_sql.py            # enforce tripwires + baseline freshness (CI)
  check_api_direct_sql.py --update   # regenerate baseline from the current tree
  check_api_direct_sql.py --print    # print current per-file normalized counts
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

# Canonical roots relative to the repository root.
REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
API_GLOB = "backend/src/api/**/*.rs"
BASELINE = (
    REPO_ROOT / "tools" / "repository-integrity" / "baselines" / "api-direct-sql.txt"
)
EXCEPTIONS = (
    REPO_ROOT / "tools" / "repository-integrity" / "exceptions" / "api-direct-sql.txt"
)

# Ordered longest-name-first so `query_as` is not counted as `query`.
_DIRECT_SQL_RE = re.compile(
    r"sqlx::(?:query_as_unchecked|query_as|query_scalar|query_with|query_file"
    r"|raw_sql|QueryBuilder|query)\b"
)

_COMMENT_LINE_RE = re.compile(r"//.*")
_BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.DOTALL)


def strip_comments(text: str) -> str:
    """Remove Rust line and block comments so they cannot create false debt.

    Line comments are removed first; block comments are removed with a
    non-greedy matcher. The direct-SQL tokens we search for never appear
    inside string literals in practice, so this is deterministic and stable.
    """
    text = _COMMENT_LINE_RE.sub("", text)
    text = _BLOCK_COMMENT_RE.sub("", text)
    return text


def count_direct_sql(text: str) -> int:
    """Return the normalized number of direct-SQL invocation tokens."""
    return len(_DIRECT_SQL_RE.findall(strip_comments(text)))


def inventory(root: pathlib.Path | None = None) -> dict[str, int]:
    """Map relative API file path -> normalized direct-SQL count.

    Files are included even when their count is zero so the baseline can be
    regenerated authoritatively; callers that only care about debt filter on
    ``count > 0``.
    """
    root = root or REPO_ROOT
    counts: dict[str, int] = {}
    for path in sorted((root / "backend" / "src" / "api").glob("**/*.rs")):
        rel = path.relative_to(root).as_posix()
        try:
            counts[rel] = count_direct_sql(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError):
            # A file we cannot read coherently is not silently debt-free; flag
            # it as a single canonical failure path by counting it as unknown.
            counts[rel] = -1
    return counts


def regenerate_lines(root: pathlib.Path | None = None) -> list[str]:
    """Return the deterministic baseline lines for the current tree."""
    counts = inventory(root)
    lines = [
        f"{path} {count}"
        for path, count in sorted(counts.items())
        if count > 0
    ]
    return lines


def load_baseline(root: pathlib.Path | None = None) -> dict[str, int]:
    path = pathlib.Path(root or REPO_ROOT) / BASELINE.relative_to(REPO_ROOT)
    baseline: dict[str, int] = {}
    if not path.exists():
        return baseline
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) == 2:
            try:
                baseline[parts[0]] = int(parts[1])
            except ValueError:
                continue
    return baseline


def load_exceptions(root: pathlib.Path | None = None) -> set[str]:
    path = pathlib.Path(root or REPO_ROOT) / EXCEPTIONS.relative_to(REPO_ROOT)
    excepted: set[str] = set()
    if not path.exists():
        return excepted
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        # Format: <path> <reason> <decision_note> <tests>
        excepted.add(line.split()[0])
    return excepted


def run_checks(root: pathlib.Path | None = None) -> tuple[list[str], bool]:
    """Return (issues, baseline_fresh).

    A path is exempt from the tripwires only when it is present in the
    exception register. Baseline freshness requires the committed baseline to
    equal the current deterministic generation.
    """
    root = root or REPO_ROOT
    current = inventory(root)
    baseline = load_baseline(root)
    excepted = load_exceptions(root)
    issues: list[str] = []

    baseline_path = root / BASELINE.relative_to(REPO_ROOT)

    for path in sorted(current):
        count = current[path]
        if count < 0:
            issues.append(f"FAIL: could not read/count {path}")
            continue
        if count == 0:
            continue
        base = baseline.get(path)
        if base is None:
            if path in excepted:
                continue
            issues.append(
                f"FAIL: new production API file contains direct SQL: {path} "
                f"(count={count}); move it behind a repository/service boundary "
                f"or register an RI-C04 exception."
            )
        elif count > base:
            if path in excepted:
                continue
            issues.append(
                f"FAIL: direct-SQL count in {path} increased "
                f"({base} -> {count}); move it behind a repository/service "
                f"boundary or register an RI-C04 exception."
            )

    committed_lines = (
        [line.strip() for line in baseline_path.read_text(encoding="utf-8").splitlines()]
        if baseline_path.exists()
        else []
    )
    regenerated_lines = [line.strip() for line in regenerate_lines(root)]
    fresh = committed_lines == regenerated_lines
    return issues, fresh


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="RI-C04 api-persistence-boundary guard"
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--update",
        action="store_true",
        help="regenerate the baseline from the current tree and write it",
    )
    group.add_argument(
        "--print",
        action="store_true",
        help="print current per-file normalized direct-SQL counts",
    )
    args = parser.parse_args(argv)

    if args.print:
        for path, count in sorted(inventory().items()):
            if count > 0:
                print(f"{path} {count}")
        return 0

    if args.update:
        lines = regenerate_lines()
        BASELINE.parent.mkdir(parents=True, exist_ok=True)
        BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"Updated {BASELINE.relative_to(REPO_ROOT)} "
              f"({len(lines)} inventoried files)")
        return 0

    issues, fresh = run_checks()
    for issue in issues:
        print(issue)
    if not fresh:
        print(
            "FAIL: committed baseline is stale (drifted from the current tree); "
            "run --update after intentional, reviewed changes."
        )
    if issues or not fresh:
        print("RI-C04 tripwire: FAIL")
        return 1
    print("RI-C04 tripwire: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())

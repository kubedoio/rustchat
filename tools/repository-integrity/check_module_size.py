#!/usr/bin/env python3
"""RI-C05 module-growth-control guard.

Large production modules are baseline debt and must not grow materially without
explicit responsibility justification; newly monolithic production modules
require review (ADR-006 RI-C05).

The baseline records the current line count of every production Rust module
under ``backend/src`` and ``push-proxy/src`` in
``tools/repository-integrity/baselines/large-production-modules.txt``. The
"large" subset is the set of modules above the review threshold.

Thresholds are REVIEW TRIPWIRES, not architecture targets (RI-C05 semantics):

  * new module line count      : 1500  (a newly added module this large needs review)
  * existing large module line : 1000  (a module at/above this is "large")
  * baseline growth percent    : 5%    (material growth trigger for large modules)
  * baseline growth absolute   : 100   (absolute line growth trigger for large modules)

The guard does NOT require baseline freshness: normal edits and line movement
within the thresholds create no false debt. A path may be exempted only via the
RI-C10/RI-C05 exception register (decision note + focused tests + reviewer).

Usage:
  check_module_size.py            # enforce tripwires (CI)
  check_module_size.py --update   # regenerate the baseline from the current tree
  check_module_size.py --print    # print current per-module line counts (debug)
"""

from __future__ import annotations

import argparse
import pathlib
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
BASELINE = REPO_ROOT / "tools" / "repository-integrity" / "baselines" / "large-production-modules.txt"
EXCEPTIONS = REPO_ROOT / "tools" / "repository-integrity" / "exceptions" / "module-size.txt"

LARGE_THRESHOLD = 1000      # existing_large_module_line_count
NEW_THRESHOLD = 1500        # new_module_line_count
GROWTH_PERCENT = 5          # baseline_growth_percent
GROWTH_ABSOLUTE = 100       # baseline_growth_absolute_lines

PROD_ROOTS = ("backend/src", "push-proxy/src")


def module_paths(root: pathlib.Path | None = None) -> list[pathlib.Path]:
    root = root or REPO_ROOT
    paths: list[pathlib.Path] = []
    for base in PROD_ROOTS:
        for p in (root / base).glob("**/*.rs"):
            # Skip the target/ build dir and any vendored/cached sources.
            if "target" in p.parts:
                continue
            paths.append(p)
    return sorted(paths, key=lambda p: p.as_posix())


def line_count(path: pathlib.Path) -> int:
    try:
        with path.open("r", encoding="utf-8") as fh:
            return sum(1 for _ in fh)
    except (OSError, UnicodeDecodeError):
        return -1


def inventory(root: pathlib.Path | None = None) -> dict[str, int]:
    root = root or REPO_ROOT
    counts: dict[str, int] = {}
    for path in module_paths(root):
        counts[path.relative_to(root).as_posix()] = line_count(path)
    return counts


def load_baseline(root: pathlib.Path | None = None) -> dict[str, int]:
    path = (root or REPO_ROOT) / BASELINE.relative_to(REPO_ROOT)
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
    path = (root or REPO_ROOT) / EXCEPTIONS.relative_to(REPO_ROOT)
    excepted: set[str] = set()
    if not path.exists():
        return excepted
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        excepted.add(line.split()[0])
    return excepted


def run_checks(root: pathlib.Path | None = None) -> list[str]:
    root = root or REPO_ROOT
    current = inventory(root)
    baseline = load_baseline(root)
    excepted = load_exceptions(root)
    issues: list[str] = []

    for path in sorted(current):
        cur = current[path]
        if cur < 0:
            issues.append(f"FAIL: could not count lines for {path}")
            continue
        base = baseline.get(path)
        if base is None:
            if cur > NEW_THRESHOLD:
                if path in excepted:
                    continue
                issues.append(
                    f"FAIL: newly added production module is monolithic "
                    f"({path} = {cur} lines > {NEW_THRESHOLD}); split it into "
                    f"cohesive submodules or register an RI-C05 exception."
                )
            continue
        if base <= LARGE_THRESHOLD and cur <= LARGE_THRESHOLD:
            continue  # not a large module
        growth = cur - base
        if growth <= 0:
            continue
        pct = (growth * 100.0) / base if base else 0.0
        if pct > GROWTH_PERCENT or growth >= GROWTH_ABSOLUTE:
            if path in excepted:
                continue
            issues.append(
                f"FAIL: material growth in large module {path} "
                f"({base} -> {cur}, +{growth}); justify the responsibility or "
                f"decompose it, or register an RI-C05 exception."
            )
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="RI-C05 module-growth-control guard")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--update", action="store_true", help="regenerate baseline")
    group.add_argument("--print", action="store_true", help="print per-module counts")
    args = parser.parse_args(argv)

    if args.print:
        for path, count in sorted(inventory().items()):
            mark = " *" if count > LARGE_THRESHOLD else ""
            print(f"{path} {count}{mark}")
        return 0

    if args.update:
        counts = inventory()
        lines = [f"{p} {c}" for p, c in sorted(counts.items()) if c >= 0]
        BASELINE.parent.mkdir(parents=True, exist_ok=True)
        BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")
        nlarge = sum(1 for c in counts.values() if c > LARGE_THRESHOLD)
        print(f"Updated {BASELINE.relative_to(REPO_ROOT)} "
              f"({len(lines)} modules, {nlarge} large)")
        return 0

    issues = run_checks()
    for issue in issues:
        print(issue)
    if issues:
        print("RI-C05 tripwire: FAIL")
        return 1
    print("RI-C05 tripwire: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())

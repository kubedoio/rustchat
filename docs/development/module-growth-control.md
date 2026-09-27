# Module Growth Control (RI-C05)

Large production modules are baseline debt and must not grow materially
without explicit responsibility justification; newly monolithic production
modules require review. This is ADR-006 RI-C05.

This page is the source of truth for the RI-C05 guard.

## The guard

`tools/repository-integrity/check_module_size.py` inventories the line count of
every production Rust module under `backend/src` and `push-proxy/src` and, in
`--check` mode (the default, used by CI), trips on:

* a **newly added** production module above the new-module threshold
  (1500 lines) — such a module needs to be split into cohesive submodules; and
* **material growth** of a large module (at/above 1000 lines) beyond the
  review triggers (>&nbsp;5% or &ge;&nbsp;100 lines),

unless the path is registered in the RI-C05 exception register.

Thresholds are **review triggers, not architecture targets**. The guard does
not demand a mass rewrite of the baseline debt, and normal edits or line
movement within the thresholds create no false debt (the guard does not enforce
baseline freshness).

## Files

| Path | Purpose |
|------|---------|
| `tools/repository-integrity/check_module_size.py` | guard + baseline generator |
| `tools/repository-integrity/baselines/large-production-modules.txt` | generated per-module line-count baseline |
| `tools/repository-integrity/exceptions/module-size.txt` | RI-C05 exception register |
| `tools/repository-integrity/test_module_size.py` | guard acceptance tests |

## Commands

```bash
# Enforce the tripwires (matches CI).
python3 tools/repository-integrity/check_module_size.py

# Regenerate the baseline after an intentional, reviewed change.
python3 tools/repository-integrity/check_module_size.py --update

# Show current per-module line counts (large modules marked with *).
python3 tools/repository-integrity/check_module_size.py --print

# Run the guard acceptance tests.
python3 tools/repository-integrity/test_module_size.py
```

## Keeping it green

If you intentionally grow a large module beyond a review trigger, you must:

1. register the path in the exception register with a decision note, focused
   tests, and reviewer justification; and
2. regenerate the baseline with `--update`.

Prefer decomposing the module into cohesive submodules over an exception.

## Pilot (P004)

`backend/src/services/posts.rs` (1177 lines) was decomposed into a directory
module with cohesive submodules and an unchanged public API:

* `services/posts/mod.rs` — facade + `parse_mentions` + re-exports;
* `services/posts/posting.rs` — post creation, fan-out, automation, pushes;
* `services/posts/query.rs` — listing, single fetch, thread fetch, files;
* `services/posts/system.rs` — system messages.

Large-module count dropped from 17 to 16; `cargo test --lib` (251 tests)
passes unchanged.

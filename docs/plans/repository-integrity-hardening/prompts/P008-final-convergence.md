# P008 — Final Repository Integrity Convergence

## Mission

Perform a clean-room re-audit of the completed repository-integrity program.
Do not add product features and do not declare success from implementation
intent alone.

Primary contracts: **RI-C01 through RI-C10**.

## Starting procedure

1. Check out the exact candidate head.
2. Confirm the worktree is clean.
3. Read ADR-006, the implementation spec, contracts YAML, and all accepted
   exceptions.
4. Query current GitHub rulesets/protections and check runs.
5. Inspect the latest published release and candidate release state.
6. Recompute generated baselines instead of trusting committed output blindly.

## Required verification

For every contract produce:

| Contract | Implementation | Automated proof | Live/manual proof | Exceptions | Result |
|---|---|---|---|---|---|
| RI-C01 | ... | ... | ... | ... | PASS/BLOCKED |
| ... | ... | ... | ... | ... | ... |

### Mandatory adversarial checks

Attempt safe negative cases:

- make the aggregate see a simulated required failure;
- add a temporary synthetic handler SQL call and prove the guard catches it;
- grow/add a synthetic oversized module and prove the guard reports it;
- introduce an exact duplicate doc in a test fixture and prove hygiene detection;
- verify migration tests fail on an intentionally invalid fixture/schema;
- run Buzz compatibility tests with a deliberately malformed response/event;
- verify a release/promotion path refuses an unverified commit.

Revert all synthetic changes before final evidence is recorded.

## Full validation

Run all normal validation required by `AGENTS.md` for backend, frontend, and
push-proxy, plus:

- repository integrity guards;
- security/dependency checks;
- integration suite;
- migration/recovery matrix;
- docs/link/hygiene checks;
- Buzz deterministic compatibility suite;
- release-candidate validation.

Use current workflow names and commands; do not copy stale names from the
original audit.

## Final status rules

**PASS** only when every blocker contract is satisfied and all exceptions are
explicit.

**PARTIAL** when non-blocker debt remains but blocker contracts pass; list the
remaining debt precisely.

**BLOCKED** when any blocker contract lacks evidence or live repository settings
cannot be verified/applied.

Do not convert BLOCKED to PASS with prose.

## Final deliverable

Write a dated evidence report under the appropriate audit/history location with:

- exact source SHA;
- current release/tag state;
- contract matrix;
- checks/workflow runs;
- exception register;
- remaining open issues;
- explicit recommendation whether `v0.5.1` can be published.

The report is evidence, not a new roadmap.

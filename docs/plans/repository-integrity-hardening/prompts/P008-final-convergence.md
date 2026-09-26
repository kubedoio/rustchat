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
5. Inspect latest published release and candidate release state.
6. Recompute operational baselines instead of trusting committed output blindly.
7. Inspect open release-blocking issues, including #88, #89, #258, #259 and any
   newer blocker.

## Required verification

For every contract produce:

| Contract | Lifecycle | Merge gate | Promotion gate | Release gate | Evidence | Exceptions | Result |
|---|---|---|---|---|---|---|---|
| RI-C01 | active/planned | ... | ... | ... | ... | ... | PASS/BLOCKED |
| ... | ... | ... | ... | ... | ... | ... | ... |

A `planned` contract expected for v0.5.1 cannot be silently treated as active
or satisfied.

### Mandatory adversarial checks

Attempt safe negative cases:

- make a merge aggregate/set see a simulated required failure;
- make a post-merge promotion gate fail and prove a moving alias does not advance;
- add a temporary synthetic handler SQL call/new API file and prove the tripwire catches it;
- grow/add a synthetic oversized module and prove the tripwire reports it;
- introduce an exact duplicate active doc in a test fixture and prove hygiene detection;
- verify migration tests fail on an intentionally invalid fixture/schema;
- run Buzz compatibility tests with a deliberately malformed response/event;
- verify release publication refuses an unverified/arbitrary candidate commit.

Revert all synthetic changes before final evidence is recorded.

## Full validation

Run all normal validation required by current `AGENTS.md` for backend, frontend,
and push-proxy, plus:

- repository-integrity tripwires;
- required merge/security checks;
- post-merge integration/health checks;
- migration/recovery matrix;
- docs/link/hygiene checks;
- Buzz deterministic compatibility suite;
- release-candidate validation.

Use current workflow names and commands; do not copy stale names from the
original audit.

## Final status rules

**PASS (repository integrity)** only when every repository-integrity blocker
expected for the target release has evidence and all exceptions are explicit.

**PARTIAL** when non-blocker debt remains but the activated blocker contracts
pass; list remaining debt precisely.

**BLOCKED** when a required contract lacks evidence or live repository settings
cannot be verified/applied.

Do not convert BLOCKED to PASS with prose.

### Product release decision boundary

Repository-integrity PASS is **necessary but not sufficient** for v0.5.1.

The final report must separately list unresolved product/release blockers such as
#88/#89. Do not recommend publishing v0.5.1 while a separately defined release
blocker remains unresolved merely because RI-C01..RI-C10 pass.

## Final deliverable

Write a dated evidence report under the appropriate audit/history location with:

- exact source SHA;
- current release/tag state;
- contract lifecycle/gate matrix;
- check/workflow runs;
- exception register;
- remaining repository-integrity blockers;
- remaining product/release blockers;
- explicit recommendation for **repository-integrity readiness**;
- separate statement on whether the complete v0.5.1 release criteria are met.

The report is evidence, not a new roadmap.

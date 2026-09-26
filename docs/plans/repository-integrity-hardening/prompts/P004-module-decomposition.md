# P004 — Control Module Growth and Decompose by Responsibility

## Mission

Prevent RustChat from developing hidden monoliths inside otherwise reasonable
directories, and reduce the highest-value existing module debt without a
repository-wide file-splitting campaign.

Primary contracts: **RI-C05, RI-C09, RI-C10**.

## Revalidate the baseline

Measure current production source files. Do not rely on historical line counts.

At minimum inspect the current responsibilities and callers of the audit
candidates:

- `backend/src/repositories/post_repository.rs`;
- `backend/src/repositories/admin_repository.rs`;
- `backend/src/repositories/channel_repository.rs`;
- `backend/src/repositories/user_repository.rs`;
- `backend/src/api/v4/websocket/connection.rs`;
- `backend/src/services/unreads.rs`.

Rank them by:

1. number of distinct responsibilities/invariants;
2. churn/conflict pressure;
3. testability;
4. coupling;
5. likely review benefit from decomposition.

Do not rank only by line count.

## Required work

1. Generate `.governance/baselines/large-production-modules.txt` with a stable,
   documented algorithm.
2. Add a fast check implementing RI-C05:
   - identify new production modules crossing the review threshold;
   - detect material growth of baseline-large modules;
   - print actionable diagnostics;
   - support explicit RI-C10 exceptions.
3. Choose at most **two** modules for decomposition in this phase.
4. Split by cohesive capability while preserving public interfaces where that
   minimizes caller churn.
5. Keep tests close to the responsibilities they prove.
6. Update module documentation only where ownership changed.

## Good decomposition examples

A large post repository may become domain modules such as:

```text
repositories/posts/
  mod.rs
  create.rs
  history.rs
  threads.rs
  reactions.rs
  unread.rs
```

only if those divisions match actual invariants and query ownership.

A WebSocket connection module may separate:

- handshake/authentication;
- connection lifecycle;
- resumption/replay;
- outbound delivery/backpressure;

only if the existing code demonstrates those responsibilities.

## Forbidden shortcuts

- `part1.rs`, `part2.rs`, `misc.rs`;
- moving tests away only to reduce measured size;
- duplicated private helpers in each new module;
- changing API behavior under the label "cleanup";
- decomposing every candidate in one PR.

## Acceptance evidence

- baseline and guard tests;
- responsibility map before/after for each decomposed module;
- relevant test suite unchanged or improved;
- no public behavior change unless separately specified;
- line counts reported as supporting evidence, never as the sole success metric.

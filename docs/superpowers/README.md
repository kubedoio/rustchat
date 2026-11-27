# Superpowers Specs and Plans

Working area for the plan-first agent/contributor workflow described in
[`docs/development/agent-model.md`](../development/agent-model.md) and
[`AGENTS.md`](https://github.com/kubedoio/rustchat/blob/main/AGENTS.md).

This is not product architecture documentation. Durable design decisions belong
in `docs/adr/`; released/operational documentation lives under its respective
`docs/` section.

## Layout

| Path | Purpose | Naming |
|---|---|---|
| `specs/` | Design specs written before implementation | `YYYY-MM-DD-<topic>-design.md` |
| `plans/` | Task-by-task implementation plans derived from a spec | `YYYY-MM-DD-<topic>.md` |

Both directories are empty apart from `.gitkeep` in this snapshot. The
`.gitkeep` files preserve the layout referenced by
`.governance/agent-contracts.yml`, `.governance/protected-paths.yml`, and
[`AGENTS.md`](https://github.com/kubedoio/rustchat/blob/main/AGENTS.md). The
`compat-agent` contract permits analysis output in these directories, but not
production code.

Historical superpowers material formerly archived under
`docs/archive/superpowers/` was removed from the public tree during the
public-promotion cleanup.

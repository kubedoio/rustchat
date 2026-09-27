# Plans

This directory holds implementation plans and dated planning documents for
RustChat. Per ADR-006/RI-C06, `docs/plans/**` is the canonical home for
maintainer-approved, in-flight plans; completed designs are archived under
`docs/archive/**` when they are superseded.

## Active program

- **[Repository Integrity Hardening](repository-integrity-hardening/README.md)**
  — execution prompts for
  [ADR-006](../adr/ADR-006-repository-integrity-and-maintainability.md) and the
  [implementation spec](2026-09-26-repository-integrity-hardening-spec.md),
  covering P001–P008 (green-main gates, governance, persistence/module guards,
  docs hygiene, migration evidence, Buzz compat record, release verdict).

## Dated plans and designs

- `2026-04-06-composer-enhancement-design.md` / `-implementation.md` — composer
  enhancement design and implementation (Completed).
- `2026-04-08-websocket-disconnection-ux-design.md` / `-implementation.md` —
  WebSocket disconnection UX design and implementation (Completed).
- `2026-05-30-execution-plan-analysis.md` — historical execution-plan analysis.
- `2026-05-30-rustchat-next-steps-handoff.md` — historical handoff document.

## Conventions

- Name in-flight plans `YYYY-MM-DD-<short-description>.md`.
- Move completed/superseded plans to `docs/archive/**` when they no longer
  describe active work, and repair any references that pointed at them.

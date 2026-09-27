# Decision Notes

This directory contains decision notes for elevated-tier changes.

A decision note is a short explanation (3–5 lines) of what was decided and why — required for any PR marked as `elevated` or `architectural` risk tier. Unlike ADRs (which are for architectural-scale decisions), decision notes cover routine elevated changes such as auth tweaks, migration strategies, or compat surface changes.

## When to create a decision note

Required when a PR's risk tier is `elevated` or `architectural` and the change is not covered by an existing ADR.

## Format

File naming: `YYYY-MM-DD-<short-description>.md`

```
# Decision Note: [Short title]

**Date:** YYYY-MM-DD
**PR:** #NNN
**Risk tier:** elevated | architectural

## What was decided

[2–4 sentences: what the decision is]

## Why

[2–4 sentences: context, constraints, alternatives considered]
```

## Index

_(No decision notes available yet — add entries here as notes are created.)_

The directory currently holds one historical artifact retained for continuity:
- `code-quality-audit-2026-05.md` — a dated code-quality audit (evidence, not a
  decision note); dated audits also live under `docs/internal/` and
  `docs/audits/`.

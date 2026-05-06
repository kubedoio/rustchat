# Frontend Dependency Policy

This document defines the approved supply-chain policy for the Vue frontend in `rustchat`.

## Goals

1. keep the direct dependency surface small
2. make installs deterministic
3. stop untrusted install-time code from running automatically in CI
4. make dependency changes visible and reviewable
5. support emergency transitive remediation without waiting on upstream

## Standard Package Manager

The frontend uses **npm** as the only supported package manager.

Rules:

1. `frontend/package-lock.json` is the single lockfile source of truth.
2. `frontend/pnpm-lock.yaml` and `frontend/yarn.lock` are not allowed.
3. Routine local setup uses `npm ci`.
4. CI and release-oriented builds use `npm ci --ignore-scripts`.
5. Use `npm install` or `npm uninstall` only when you are intentionally changing dependencies and plan to review the resulting manifest and lockfile diff.

## Approved Dependency Categories

Every direct dependency must be justified in `frontend/dependency-policy.json` under exactly one of these categories:

1. `framework-critical`
2. `build-tool-critical`
3. `security-critical`
4. `editor-format-tooling-critical`
5. `impossible-to-replace-economically`

A dependency is not approved merely because it is popular or convenient.

## Adding a Direct Dependency

Before adding a new frontend dependency:

1. verify that browser APIs, existing internal utilities, or current packages cannot solve the problem cheaply
2. document the dependency in `frontend/dependency-policy.json`
3. choose one approved category
4. add a short reason that explains why the package exists
5. include the dependency change in PR notes

Frontend dependency PRs must include:

1. the problem being solved
2. why existing code or platform APIs were insufficient
3. the chosen category and rationale
4. whether the package introduces lifecycle scripts, native code, browser binaries, or external network behavior

## Removing a Dependency

Dependency removal is preferred whenever a package becomes:

1. unused
2. replaceable with existing platform APIs
3. duplicated by another dependency already in the repo
4. obsolete after a refactor

When removing a dependency:

1. delete its source usage
2. remove it from `frontend/package.json`
3. update `frontend/package-lock.json`
4. remove its entry from `frontend/dependency-policy.json`

## Install Policy

### Local development

Use:

```bash
cd frontend
npm ci
```

### CI and release-oriented builds

Use:

```bash
cd frontend
npm ci --ignore-scripts
```

Lifecycle scripts are blocked in CI by default.

If a tool requires additional setup in CI, that setup must happen as an explicit trusted step after dependency installation.

Example:

```bash
npx playwright install chromium
```

## Dependency Review and Vulnerability Gates

Frontend dependency changes are enforced by:

1. GitHub dependency review on pull requests
2. `frontend/scripts/check-dependency-policy.mjs`
3. `npm audit --audit-level=high`

These checks must stay green before merge.

## Overrides: First-Line Transitive Remediation

Use `frontend/package.json` `overrides` when a vulnerable transitive dependency can be fixed by forcing a safer version.

Requirements:

1. add or update the override in `frontend/package.json`
2. document it in `frontend/dependency-policy.json`
3. explain why it exists
4. record how to know when it can be removed

Use overrides first because they are the smallest and least surprising mitigation path.

## Patches: Fallback When Overrides Are Not Enough

If an override cannot remediate the issue, use the repo-managed patch workflow.

Artifacts:

1. patch files live in `frontend/patches/`
2. patch metadata lives in `frontend/dependency-patches.json`
3. patches are applied by `frontend/scripts/apply-dependency-patches.mjs`

Each patch entry must include:

1. package name
2. patch file path
3. reason
4. removal condition

Patch policy:

1. patches are emergency or short-lived controls, not permanent forks by default
2. each patch must have a clear retirement path
3. if upstream publishes a safe fix, remove the patch and prefer a normal versioned dependency update

## Install Script Exceptions

Default policy:

1. no dependency lifecycle scripts run in CI
2. no silent exceptions are allowed

If an exception becomes necessary, document all of the following in the PR and policy metadata:

1. why the package is required
2. what risk the script introduces
3. why it cannot be replaced
4. whether it is build-only or runtime-critical
5. how CI invokes it explicitly

## Update Strategy

Frontend dependency updates are PR-based and reviewable.

Rules:

1. Dependabot may open frontend update PRs.
2. Major updates must not auto-merge.
3. Security updates are reviewed separately from routine feature/tooling upgrades.
4. Every dependency update must still pass frontend tests, build, and policy gates.

## Dependency Inventory

Frontend CI publishes a machine-readable dependency inventory artifact using npm's resolved dependency tree.

Reviewers should be able to answer:

1. what changed
2. why the dependency exists
3. whether install-time code ran
4. whether a transitive dependency is pinned or patched centrally

## PR Checklist for Dependency Changes

If your PR changes `frontend/package.json` or `frontend/package-lock.json`, make sure it also includes:

1. updates to `frontend/dependency-policy.json`
2. override metadata if `overrides` changed
3. patch metadata if `frontend/dependency-patches.json` changed
4. a short explanation in the PR body
5. proof that the frontend dependency checks still pass

# Buzz Feasibility Assessment

**Status:** Evidence record — update after each re-verification
**First spike:** 2026-07-23 against `block/buzz` @ `6a56c8b` (shallow clone)
**Method:** direct source inspection of the upstream repository, not documentation claims

This document records whether the pivot plan is feasible against the *actual* Buzz codebase. It exists because ADR-004's central assumption — that RustChat enterprise services can integrate with Buzz without core divergence — must be verified evidence, not optimism. Re-run this assessment at every Phase gate and whenever an assumption here turns out to be wrong.

## Verdict summary

Building an external enterprise control plane around Buzz without core divergence is **feasible**. Every planned pillar has an existing integration seam. The residual needs are a bounded, specific list of upstream PRs, not architectural blockers.

| Pillar | Verdict | Integration seam (verified in source) |
|---|---|---|
| Keycloak identity binding | FEASIBLE | `relay_members` membership gate enforced at every entry point (`crates/buzz-relay/src/api/mod.rs`); provision/deprovision via `buzz-admin` CLI, NIP-43 admin events (kinds 9030–9033), or DB writes; cross-pod revocation over Redis pub/sub |
| Organization attestation (C3) | FEASIBLE | NIP-OA owner attestation (`docs/nips/NIP-OA.md`, enforced at the membership gate): an external key issues a signed capability the relay accepts. This is the upstream-compatible mechanism C3 requires — RustChat must use it and must not invent a private relay-auth protocol |
| Suspension / revocation (C2) | FEASIBLE | `community_bans` table with actor + expiry and disconnect propagation; `archived_identities` + NIP-IA (kinds 9035/9036) for identity retirement with admin/self/owner consent paths |
| Compliance export (C5) | FEASIBLE | `buzz-audit` hash-chained audit log, `audit_log` / `moderation_actions` tables, NIP-98-authenticated `/query` HTTP bridge |
| RustShare bridge (C4) | FEASIBLE | service identity subscribing over WebSocket or polling `/query` with scoped filters; no core change |
| User-owned key custody | FEASIBLE | client-held keys today: OS keyring (desktop), `flutter_secure_storage` (mobile) |
| Organization-managed service identities | FEASIBLE | a bridge service holding org keys in KMS and publishing via `POST /events` + NIP-98 works today; Schnorr signing is location-agnostic |
| Organization-managed end-user keys | FEASIBLE-WITH-UPSTREAM-PR | no NIP-46 (remote signer) support in any client; must be upstreamed before managed custody for end users |
| iOS push | FEASIBLE-WITH-UPSTREAM-PR | `buzz-push-gateway` is real (~4,100 LOC, APNs, App Attest, keyrings, migrations, own Dockerfile); rebranded app needs a new `AppProfile` variant (~5-line change, `model.rs`) and its own `apns_topic` |
| Android push (FCM) | FEASIBLE-WITH-UPSTREAM-PR | FCM is explicitly not yet a conforming profile (NIP-PL); new transport profile required upstream |
| New event kinds | FEASIBLE-WITH-UPSTREAM-PR | the kind registry is closed — the relay rejects unknown kinds by design (`required_scope_for_kind`, with a `unknown_kind_rejected` test). Enterprise features should first ride existing generic kinds (30023, 30078 with d-tag namespacing, NIP-51 lists); genuinely new kinds go upstream |
| Runtime fine-grained policy | FEASIBLE-WITH-UPSTREAM-PR | no runtime policy hook (OPA/plugin/webhook at the authz layer); membership-level gates exist. Only needed if membership gates prove too coarse |
| Web chat client | **GAP** | upstream `web/` is a ~4k LOC companion (invite acceptance, git browser), **not** a chat client. The RustChat web-client story must be replanned: desktop + mobile first, or fund/contribute a web client upstream. Do not assume it exists |

## Upstream PR backlog (planned, not blockers)

These belong in the fork patch ledger as `extension-hook` / `upstream-pending` entries and should be proposed upstream early, because upstream merges are fast (~800 commits/30 days):

1. FCM transport profile for `buzz-push-gateway` (Android push).
2. NIP-46 remote signing in clients (organization-managed end-user keys).
3. `AppProfile` variant for the rebranded RustChat iOS app.
4. Runtime external-policy hook (only if membership gates prove insufficient).
5. Any net-new event kinds not expressible via generic kinds.

## Rebranding cost (verified)

Names and identifiers are hardcoded but concentrated in config files: `desktop/src-tauri/tauri.conf.json` (`productName`, `identifier`), Android `applicationId` / `android:label`, iOS `PRODUCT_NAME`, `web/index.html`, plus the `AppProfile` enum. Expect a thin, mechanical downstream branding overlay — small but never zero-divergence. Apache-2.0 grants no trademark rights to "Buzz", so rebranding is legally *required* for distribution anyway.

## Upstream health (verified 2026-07-23)

- Public since 2026-03-06 (~4.5 months old), Apache-2.0, no CLA/DCO constraints on forking.
- ~800 commits in the last 30 days; 39+ contributors; top contributors are Block staff.
- ~3,500 Rust test functions, 16 e2e files, a dedicated `buzz-conformance` crate, 125 desktop/web test files — runnable unmodified by a downstream fork (satisfies TEST-STRATEGY layer 1).
- Upstream's own `ARCHITECTURE.md` candidly lists known limitations (rate-limit enforcement, half-wired approval gates, stubbed workflow actions).

## Honest risk statement

- **Youth:** the upstream is ~4.5 months old. The enterprise pilot gates (HA, RTO/RPO, pen test) depend partly on upstream maturation. There is no schedule fallback beyond maintaining the fork independently (legally clean, operationally expensive).
- **Velocity:** ~800 commits/month cuts both ways. Upstream PRs land fast, but the no-divergence strategy requires a disciplined, staffed sync cadence. If sync falls behind, the patch budget in SPEC-001 is the tripwire that forces re-evaluation.
- **Web client:** the largest product-surface gap. ADR-004 decision 4 assumed a derivable web client; one does not exist upstream. Phase 4 must explicitly decide: contribute a web client upstream, ship desktop/mobile only, or defer web.
- **Nostr commitment:** adopting Buzz means adopting the Nostr event model, NIP authentication, and `npub` key identity. This is a protocol-level commitment, not an implementation detail; enterprise reviewers must evaluate it as such.

## Re-verification triggers

Re-run this assessment (and update this file) when:

- a Phase gate is reached,
- an upstream sync changes any file named above,
- a planned upstream PR is merged or rejected,
- any "FEASIBLE" claim fails in implementation.

# SPEC-002: RustChat Enterprise Control Plane

**Status:** Proposed
**Related ADR:** ADR-004

## Purpose

Define the functionality RustChat adds around Buzz without making Buzz core aware of RustChat-specific business logic.

## Components

### Identity controller

Responsibilities:

- configure and integrate Keycloak/OIDC
- map immutable OIDC `(issuer, subject)` identities to Buzz public keys
- issue and revoke organization membership attestations
- synchronize account enabled/disabled state
- support device enrollment and recovery policy
- record lifecycle audit events

It must not:

- store user passwords
- impersonate users silently
- mutate Buzz database tables
- bypass Buzz signature verification

### Organization controller

Responsibilities:

- organizations and tenant configuration
- domain and realm association
- group-to-role and group-to-channel policy
- subscription and entitlement state
- organization-level agent and workflow policy

### RustShare bridge

Responsibilities:

- expose RustShare retrieval through MCP and versioned APIs
- enforce the intersection of Buzz channel access and RustShare artifact access
- return evidence references with every retrieval result
- propagate deletion and access-revocation signals
- avoid copying unrestricted document content into Buzz events

### Compliance exporter

Responsibilities:

- request collaboration data through supported Buzz query/export contracts
- correlate enterprise identity, Buzz identity, organization, and artifact references
- produce immutable export manifests with checksums
- enforce retention and authorization policy

It must not become a second writable collaboration database.

### Distribution controller

Responsibilities:

- pin the complete tested version matrix
- publish Compose and Helm distributions
- configure branding and update channels
- manage release metadata, SBOMs, provenance, and artifact digests
- run backup, restore, and upgrade orchestration

## Identity model

The enterprise identity and Buzz identity remain distinct but bound:

```text
EnterpriseIdentity {
  issuer: URL,
  subject: string,
  organization_id: UUID,
  email: optional string,
  state: active | suspended | deleted
}

BuzzIdentity {
  public_key: string,
  custody: user_owned | organization_managed,
  state: active | revoked
}

IdentityBinding {
  enterprise_identity,
  buzz_public_key,
  issued_at,
  expires_at,
  binding_version,
  status
}
```

The OIDC subject, not email, is the stable enterprise identifier.

## Authentication and authorization flow

1. Client authenticates with Keycloak through Authorization Code + PKCE.
2. Identity controller validates issuer, audience, signature, nonce, and token lifetime.
3. Controller resolves or creates the approved identity binding.
4. Client proves possession of the bound Buzz private key or completes an approved managed-key operation.
5. Controller issues/refreshes a short-lived organization membership attestation.
6. Buzz continues to verify signed events and applies relay/channel authorization.
7. Suspension or deletion revokes the enterprise attestation and organization admission without rewriting historical signed events.

## Key custody modes

### User-owned

- Private key remains on user devices.
- Secure device storage is mandatory.
- Recovery uses an explicit export/recovery mechanism.
- Organization controls current admission, not ownership of historical identity.

### Organization-managed

- Keys are generated and protected through KMS/HSM-backed services.
- Signing operations are authenticated and audited.
- Raw private keys are not returned to application services.
- Rotation and offboarding policies are organization controlled.

The first public preview supports **user-owned custody only**. Organization-managed signing for service identities (bridge publishing via `POST /events` + NIP-98 with KMS-held keys) may ship in the preview. Organization-managed custody for end users requires upstream NIP-46 remote-signer support in the clients (see `FEASIBILITY.md`); it is out of scope until that upstream PR lands. The contract must not prevent the managed mode later.

## Policy precedence

Access is allowed only when all relevant layers allow it:

```text
Keycloak account active
AND organization membership active
AND Buzz relay/community membership active
AND Buzz channel authorization active
AND RustShare artifact authorization active, when knowledge is accessed
```

No service may widen access granted by another service.

## Upstream integration seams (verified 2026-07-23)

The control plane integrates through these seams, verified against upstream source in [`FEASIBILITY.md`](./FEASIBILITY.md). Building a private alternative to any of them is a prohibited core divergence:

- **Admission control:** the relay membership gate (`relay_members`), managed through the `buzz-admin` CLI (shipped in the relay image) or NIP-43 admin events; cross-pod revocation propagates over Redis pub/sub. Direct database writes are prohibited (SPEC-001); where only direct access exists for an operation today, that operation waits for an upstream API or becomes an upstream PR — it does not become a private shortcut.
- **Attestation:** NIP-OA owner attestations issued by the identity controller's attester keypair (contract C3).
- **Suspension/revocation:** `community_bans` with expiry and disconnect propagation; NIP-IA identity archive for retirement.
- **Audit and export:** the `buzz-audit` hash-chained log and the NIP-98-authenticated `/query` HTTP bridge.
- **Service publishing:** `POST /events` with NIP-98 auth for bridge/service identities.

## Failure behavior

- Keycloak unavailable: existing short-lived sessions may continue only until their documented expiry; new enterprise sessions fail closed.
- Identity controller unavailable: no new bindings or attestations; Buzz-native operation follows the configured deployment policy.
- RustShare unavailable: chat remains available; knowledge tools return a typed dependency error.
- Revocation propagation delayed: monitoring alerts before the maximum revocation SLA is exceeded.
- Contract version mismatch: startup/readiness fails rather than silently degrading authorization.

## Observability

All cross-boundary operations use correlation IDs and structured audit fields:

- organization ID
- OIDC issuer and hashed subject reference
- Buzz public key
- channel/community ID when relevant
- contract version
- request/result class
- policy decision
- evidence references

Secrets, raw tokens, private keys, document bodies, and system prompts must not be logged.

## Initial delivery slices

1. Keycloak login and identity binding.
2. Suspension/revocation propagation.
3. RustShare MCP retrieval with citations.
4. Organization and group synchronization.
5. Backup/restore and tested distribution matrix.
6. Compliance export and retention after the core lifecycle is stable.

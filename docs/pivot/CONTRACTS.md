# RustChat–Buzz Boundary Contracts

**Status:** Proposed
**Versioning:** Semantic versioning per contract family

These contracts are the only supported communication paths between RustChat enterprise services and the Buzz collaboration core. Implementations must generate machine-readable schemas from the normative definitions below before public preview.

## Contract principles

- Every contract is versioned.
- Unknown required fields fail validation.
- New optional fields are backward compatible.
- Authorization decisions fail closed.
- Requests are idempotent where retries are expected.
- Cross-service messages carry correlation and causation IDs.
- No contract exposes private keys, passwords, raw access tokens, or unrestricted document bodies.
- Direct cross-service database access is not a contract.

## C1 — Identity binding

Purpose: bind an enterprise identity to a Buzz public key.

```json
{
  "contract": "rustchat.identity-binding/v1",
  "binding_id": "uuid",
  "organization_id": "uuid",
  "issuer": "https://id.example.com/realms/acme",
  "subject": "immutable-oidc-subject",
  "buzz_public_key": "hex-or-npub",
  "custody": "user_owned",
  "status": "active",
  "issued_at": "RFC3339",
  "expires_at": "RFC3339",
  "correlation_id": "uuid"
}
```

Required invariants:

- `(issuer, subject, organization_id)` uniquely identifies one active enterprise identity.
- One active binding cannot point to multiple Buzz public keys unless multi-key support is explicitly enabled by a later contract version.
- Binding creation requires proof of OIDC authentication and proof of Buzz-key possession or an approved managed-key operation.
- Email is display metadata and is never the stable binding key.

## C2 — Lifecycle event

Purpose: propagate enterprise account and organization lifecycle changes.

```json
{
  "contract": "rustchat.lifecycle-event/v1",
  "event_id": "uuid",
  "event_type": "identity.suspended",
  "organization_id": "uuid",
  "binding_id": "uuid",
  "buzz_public_key": "hex-or-npub",
  "effective_at": "RFC3339",
  "reason_code": "admin_action",
  "idempotency_key": "string",
  "correlation_id": "uuid",
  "causation_id": "uuid-or-null"
}
```

Initial event types:

- `identity.bound`
- `identity.suspended`
- `identity.reactivated`
- `identity.deleted`
- `identity.key_rotated`
- `organization.membership_granted`
- `organization.membership_revoked`
- `group.membership_changed`

Delivery requirements:

- At-least-once delivery.
- Consumers deduplicate by `event_id` and `idempotency_key`.
- Suspension and revocation have a documented maximum propagation SLA.
- Historical Buzz events remain attributable to their original signing key.

## C3 — Organization membership attestation

Purpose: prove that a Buzz identity is currently admitted to an enterprise organization.

Required claims:

```json
{
  "contract": "rustchat.organization-attestation/v1",
  "issuer": "rustchat-identity-controller",
  "organization_id": "uuid",
  "buzz_public_key": "hex",
  "roles": ["member"],
  "groups": ["engineering"],
  "issued_at": "RFC3339",
  "not_before": "RFC3339",
  "expires_at": "RFC3339",
  "attestation_id": "uuid"
}
```

The transport and signature format may follow a Buzz-supported owner-attestation mechanism or a generic upstream extension. RustChat must not introduce an incompatible private relay-auth protocol when an upstream-compatible mechanism is available.

## C4 — RustShare retrieval request

Purpose: retrieve permission-filtered company knowledge for a human or agent.

```json
{
  "contract": "rustchat.rustshare-retrieval/v1",
  "organization_id": "uuid",
  "requester": {
    "buzz_public_key": "hex",
    "binding_id": "uuid",
    "actor_type": "human"
  },
  "context": {
    "community_id": "string",
    "channel_id": "string-or-null",
    "thread_id": "string-or-null"
  },
  "query": "text",
  "top_k": 5,
  "correlation_id": "uuid"
}
```

Response:

```json
{
  "contract": "rustchat.rustshare-retrieval-result/v1",
  "result_id": "uuid",
  "items": [
    {
      "artifact_id": "uuid",
      "chunk_id": "uuid",
      "title": "string",
      "excerpt": "bounded text",
      "score": 0.91,
      "evidence_uri": "rustshare://artifact/chunk",
      "authorization_basis": ["artifact_acl", "channel_mapping"]
    }
  ],
  "policy_decision_id": "uuid",
  "correlation_id": "uuid"
}
```

Required invariants:

- Returned access is the intersection of requester, organization, Buzz context, and RustShare permissions.
- Empty authorized results are valid and must not be distinguished from forbidden results in a way that leaks artifact existence.
- Every answer derived from retrieval retains evidence references.

## C5 — Compliance export request

```json
{
  "contract": "rustchat.compliance-export/v1",
  "export_id": "uuid",
  "organization_id": "uuid",
  "requested_by_binding_id": "uuid",
  "scope": {
    "communities": ["string"],
    "channels": ["string"],
    "from": "RFC3339",
    "to": "RFC3339"
  },
  "legal_basis": "admin_export",
  "correlation_id": "uuid"
}
```

Export output must contain:

- manifest and contract version
- source component versions and Buzz SHAs
- query scope
- record counts
- checksums
- identity-correlation map authorized for the export
- evidence/artifact references
- errors and omissions

## C6 — Distribution version matrix

```yaml
contract: rustchat.distribution-matrix/v1
rustchat_release: 1.0.0-preview.1
buzz_upstream:
  repository: block/buzz
  commit: <sha>
buzz_downstream:
  repository: kubedoio/buzz
  commit: <sha>
clients:
  desktop: <version>
  android: <version>
  ios: <version>
services:
  identity_controller: <version>
  rustshare_bridge: <version>
  organization_controller: <version>
keycloak: <version>
rustshare_contract: <version>
conformance_suite: <version>
artifacts:
  relay_image: <immutable-digest>
  identity_image: <immutable-digest>
```

A release pipeline must reject missing mutable references such as an unpinned `main`, `latest`, or image tag without a digest.

## C7 — Branding manifest

Branding must be data-driven where practical:

```yaml
contract: rustchat.branding/v1
product_name: RustChat
company_name: Kubedo GmbH
bundle_ids:
  desktop: io.rustchat.desktop
  android: io.rustchat.mobile
  ios: io.rustchat.mobile
urls:
  website: https://rustchat.io
  privacy: https://rustchat.io/privacy
  terms: https://rustchat.io/terms
  updates: https://updates.rustchat.io
```

Branding changes must not alter event kinds, authorization behavior, storage schema, or protocol compatibility.

## Contract change procedure

1. Open an issue describing the compatibility need.
2. Update the contract and its schema.
3. Add producer and consumer contract tests.
4. Document migration and rollback behavior.
5. Use a major version for breaking changes.
6. Keep at least one supported compatibility window during upgrades.
7. Require an ADR for changes affecting identity, authorization, protocol, or data ownership.

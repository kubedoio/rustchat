# Documentation Archive

This directory contains historical documentation for completed work, implementation summaries, and superseded plans. These documents are kept for reference but are no longer actively maintained.

## Archive Structure

### [2026-03-entity-foundation/](./2026-03-entity-foundation/)
Phase 1 implementation: Entity management system, API keys, rate limiting.
- Entity registration system for bots, integrations, webhooks
- API key authentication with Argon2id hashing
- Mobile compatibility audit results (39/41 endpoints)

### [2026-03-voip-push/](./2026-03-voip-push/)
VoIP push notification implementation for mobile calls.
- Push notification architecture
- FCM/APNS integration
- Mobile ringing implementation

### [2026-03-v4-api/](./2026-03-v4-api/)
Mattermost v4 API compatibility implementation.
- API gap analysis
- Endpoint implementation summaries
- Compatibility testing results

### [2026-03-fetch-migration/](./2026-03-fetch-migration/)
Frontend HTTP client migration from Axios to native Fetch.
- Migration completion report
- Contract test results

### [2026-03-sso/](./2026-03-sso/)
Single Sign-On implementation.
- SSO configuration guides
- Implementation summary

### [2026-04-postcard-migration/](./2026-04-postcard-migration/)
Postcard migration plan (dependency security update).
- Plan for replacing `bincode` with `postcard`
- Risk assessment and verification steps

### superpowers/ (removed)
The historical spec and plan files formerly archived here were removed from the
public tree during the public-promotion cleanup. New specs and plans use the
working layout at [`../superpowers/`](../superpowers/).

---

## Why Archive?

These documents are archived because:
- The work they describe is **complete and shipped**
- They provide **historical context** for future decisions
- They document **lessons learned** and trade-offs made
- They may contain **debugging approaches** useful for similar problems

## When to Reference Archive Docs

- Understanding why a particular implementation choice was made
- Debugging issues in completed features
- Planning similar work (learning from past approaches)
- Researching compatibility decisions

---

*Note: Documents in this directory are not actively maintained. For current documentation, see the relevant sections in the parent `docs/` directory.*

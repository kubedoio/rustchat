# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for rustchat.

An ADR documents a significant architectural decision: the context, the options considered, the decision made, and the consequences.

## When to create an ADR

Per `.governance/risk-tiers.yml`, an ADR is required for `architectural` tier changes, and for specific elevated changes involving:
- Auth or permission model changes
- API contract changes
- Storage model changes

## Format

```
# ADR-NNN: Title

**Date:** YYYY-MM-DD
**Status:** Proposed | Accepted | Deprecated | Superseded by ADR-NNN
**Risk tier:** architectural

## Context
[What is the problem and why does it matter?]

## Decision
[What was decided?]

## Consequences
[What are the trade-offs and implications?]
```

## Index

- [ADR-001: AI Agents as Channel Participants](./ADR-001-ai-agents-architecture.md) - Agent identity, configuration, runtime, channel assignment, memory, LLM provider, and admin surface for first-class AI agents.
- [Spec: AI Agents Implementation](./ADR-001-ai-agents-implementation-spec.md) - Implementation plan and endpoint contract for the initial AI agents slice.
- [ADR-002: RAG Knowledge Base with External Sync](./ADR-002-rag-knowledge-base.md) - pgvector-backed knowledge bases, document ingestion, RustShare sync, and grounded agent retrieval.
- [ADR-002 Implementation Spec: RAG Knowledge Base](./ADR-002-rag-knowledge-base-implementation-spec.md) - Implementation details for knowledge base storage, chunking, embedding, sync sources, and agent assignments.
- [ADR-003: Agents Phase 3 - Streaming, Tools, and Feedback](./ADR-003-agents-phase3.md) - Streaming responses, tool calling, feedback, analytics, and hybrid retrieval direction.
- [ADR-003 Implementation Spec: Agents Phase 3](./ADR-003-agents-phase3-implementation-spec.md) - Implementation details for streaming, tools, message feedback, analytics, and frontend behavior.
- [ADR-005: RustChat Core Continuation and Buzz as an Optional Integration Target](./ADR-005-rustchat-core-and-buzz-integration.md) - RustChat remains an independently developed platform; Buzz is an optional, isolated external integration behind a connector boundary. (Supersedes the retired ADR-004, which is preserved in git history.)
- [ADR-006: Repository Integrity and Incremental Maintainability](./ADR-006-repository-integrity-and-maintainability.md) - Makes green-main semantics, repository protection, release integrity, persistence boundaries, module growth control, documentation sources of truth, migration evidence, and external-integration isolation explicit repository contracts.
- [ADR: Frontend Supply-Chain Security Model](./ADR-frontend-supply-chain-security.md) - npm normalization, hardened CI installs, dependency governance, and override/patch policy for the Vue frontend.

# RustShare Integration

RustShare is Kubedo's permission-aware file sharing service. RustChat
integrates with it as a **sync source for AI agent knowledge bases**: a
RustShare folder can be mapped into a RustChat knowledge base, and its
documents are periodically ingested into the RAG pipeline.

This is a read-side integration over RustShare's documented HTTP API
(`GET /api/v1/folders/{id}/files` with bearer authentication). RustChat never
writes to RustShare through this integration, and RustChat does not depend on
RustShare to operate — knowledge bases can also be filled by direct document
upload.

## How it works

```
RustShare folder
    ↓ (bearer-authenticated HTTP listing + downloads)
Sync orchestrator (backend/src/services/sync/rustshare/)
    ↓
RAG pipeline: extract text → chunk → embed → pgvector
    ↓
Knowledge base (available to assigned AI agents)
```

- The sync source configuration (RustShare base URL, auth token, folder ID) is
  stored encrypted at rest using the RustChat encryption key.
- The orchestrator performs full and incremental syncs (`modified_since`
  filtering with pagination), stores document files in S3-compatible storage,
  and records sync state per source.
- Use a dedicated integration credential with the minimum scope required for
  the folder being synced.

## Configuration

RustShare sync sources are configured per knowledge base through the admin
surface (see [Admin Guide — AI Agents](../admin/ai-agents.md)). No global
environment variable is required; each sync source carries its own connection
details and credential.

## Failure behavior

RustShare sync failures are isolated to the affected sync source: sync errors
are reported per source and do not affect messaging, realtime, authentication,
or other knowledge bases. Documents already ingested remain available to
agents.

## Related documentation

- [ADR-002: RAG Knowledge Base with External Sync](../adr/ADR-002-rag-knowledge-base.md)
- [Architecture — RAG pipeline](../architecture/overview.md#rag-pipeline)
- [Admin Guide — AI Agents](../admin/ai-agents.md)

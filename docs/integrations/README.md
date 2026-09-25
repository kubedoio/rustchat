# External Integrations

Documentation for integrating RustChat with external systems.

RustChat is authoritative for RustChat-owned state. Every integration is
optional: with no integration configured, RustChat behaves exactly like a
build without it. See
[ADR-005](../adr/ADR-005-rustchat-core-and-buzz-integration.md) and
[Integrations Architecture](../architecture/integrations.md) for the rules
that govern integration boundaries.

## Available integrations

- [RustShare](./rustshare.md) — permission-aware document sync for AI agent
  knowledge bases (RAG).

## Built-in integration surfaces

Webhooks, slash commands, bots, and API keys are configured per team through
the administration surface and do not require separate integration
documentation here. See:

- [Admin Guide — AI Agents](../admin/ai-agents.md) for agent tool
  configuration (including Tavily web search)
- [User Guide](../user/README.md) for using slash commands and webhooks
- [Compatibility Scope](../compatibility-scope.md) for
  Mattermost-compatible integration endpoints

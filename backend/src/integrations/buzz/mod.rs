//! Optional Buzz integration (outbound channel bridging, phase one).
//!
//! Buzz (github.com/block/buzz) is a self-hosted team communication platform
//! built on the Nostr protocol. This module implements a **one-directional,
//! RustChat → Buzz** bridge: messages posted in a mapped RustChat channel are
//! re-published to the mapped Buzz channel over Buzz's public HTTP bridge
//! (`POST /events`, NIP-98 authentication).
//!
//! * RustChat remains a fully independent product; nothing here runs when
//!   [`crate::config::BuzzIntegrationConfig::enabled`] is false.
//! * No Buzz code is vendored or forked, and no Buzz database is shared. The
//!   only coupling is the documented public HTTP API
//!   (see `docs/integrations/buzz.md`).
//! * Ownership stays RustChat-authoritative: channel mappings live in
//!   RustChat's database, and the Buzz side only needs the bridge pubkey to
//!   be a channel member.

pub mod connector;
pub mod dispatcher;
pub mod events;
pub mod http;
pub mod mock;
pub mod repository;

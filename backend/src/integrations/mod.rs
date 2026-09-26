//! Optional external integrations.
//!
//! This module hosts RustChat's outbound bridges to external systems. It is
//! strictly additive: when no integration is enabled (the default),
//! [`crate::state::AppState`] behavior is indistinguishable from a build
//! without this module — no workers are spawned, no lookups run on request
//! paths, and the admin surface returns an explicit "not enabled" error.
//!
//! Integration rules (ADR-005, "RustChat core and Buzz integration"):
//!
//! * Integrations talk to **external systems over their public APIs**; they
//!   never vendor, fork, or share a database schema with the target system.
//! * Delivery uses the **transactional outbox** ([`outbox`]): the RustChat
//!   state change and the intent to notify are committed atomically, and a
//!   background dispatcher performs the remote call with bounded retries.
//! * The local system stays authoritative for ownership and identity.

pub mod buzz;
pub mod outbox;

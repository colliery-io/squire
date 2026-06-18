//! Squire — shared contract
//!
//! The single seam all sides build against:
//!   * the Keep (computer, Rust): engine + store + admin UI + local API; the only writer
//!   * the Squire (child phone, Kotlin): renders `StateView`, posts claims/requests
//!   * the Knight (parent phone, Kotlin): posts privileged quick-actions under a parent credential
//!
//! Design-first: this file is types + signatures only. No engine logic here.
//!
//! Identity & tenancy (ADR SQUIRE-A-0002 / A-0004):
//!   * A tenant = a Household, fully isolated at the schema level (own Postgres schema /
//!     own SQLite file). Tenancy never appears in these types — a `Snapshot` IS one
//!     household's data, so the domain core stays tenant-agnostic.
//!   * A Household has one-or-more Knights (adults) and Squires (children). All player
//!     activity is attributed to a Squire (`UserId`); every `Event` carries one.
//!
//! Storage split (within a tenant schema):
//!   * IDENTITY (User) + DEFINITIONS (Quest / RedeemableItem / Achievement) = mutable tables.
//!   * ACTIVITY (Event) = append-only log; the only thing that moves a Squire's balance.
//!
//! The placeholder primitives below (Date, Timestamp, *Id) keep this file
//! self-contained — swap them for `chrono` / `uuid` when wiring real deps.

pub mod primitives;
pub use primitives::*;

pub mod identity;
pub use identity::*;

pub mod definitions;
pub use definitions::*;

pub mod events;
pub use events::*;

pub mod commands;
pub use commands::*;

pub mod ports;
pub use ports::*;

pub mod errors;
pub use errors::*;

pub mod api;
pub use api::*;

pub mod config;
pub use config::*;

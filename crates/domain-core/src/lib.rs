//! # Squire Domain Core
//!
//! The pure, portable heart of Squire (spec **SQUIRE-S-0001**): it owns the domain types
//! (re-exported from [`contract`], the in-crate copy of `shared_contract.rs`) and implements
//! the two behavioural traits — [`contract::Engine`] (the one validated door, `handle`) and
//! [`contract::Projections`] (read derivations over the event log).
//!
//! No I/O, storage, transport, UI, or direct clock access (AR-7 / NFR-1.1.4). The core reads a
//! [`contract::Snapshot`] and a `&dyn Clock` and returns [`contract::Change`]s for a single
//! writer to apply. Per-Squire (ADR SQUIRE-A-0005) and tenant-agnostic (ADR SQUIRE-A-0002).
//!
//! Implemented incrementally by the Domain Core tasks:
//! - **T-0001** (this scaffold): types, dispatch skeleton, child-surface guard, test doubles.
//! - T-0002 authoring · T-0003 claim/review · T-0004 due · T-0005 redemption/ledger
//!   · T-0006 streaks/achievements · T-0007 invariant property suite.

pub mod contract;
pub mod engine;
pub mod projections;
pub mod testkit;

// Per-command-family handlers + shared helpers (crate-internal).
mod authoring;
mod claims;
pub(crate) mod common;
mod redemption;

pub use contract::*;
pub use engine::{child_originable, DomainEngine};
pub use projections::{quest_status, reward_view, Proj};

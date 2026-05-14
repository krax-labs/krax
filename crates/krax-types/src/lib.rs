//! krax-types: core domain types and cross-crate traits.
//!
//! This crate is the single point of cross-crate type sharing for the Krax workspace.
//! All other crates depend on the traits defined here; none import concrete types
//! from each other directly. See AGENTS.md Rule 1.

pub mod block;
pub mod journal;
pub mod journal_entry;
pub mod rwset;
pub mod snapshot;
pub mod state;
pub mod tx;

#[cfg(test)]
mod test_helpers;

pub use block::Block;
pub use journal::{Journal, JournalEntry};
pub use rwset::RWSet;
pub use snapshot::Snapshot;
pub use state::{State, StateError};
pub use tx::{MempoolEntry, PendingTx};

//! krax-types: core domain types and cross-crate traits.
//!
//! This crate is the single point of cross-crate type sharing for the Krax workspace.
//! All other crates depend on the traits defined here; none import concrete types
//! from each other directly. See AGENTS.md Rule 1.

pub mod snapshot;
pub mod state;

pub use snapshot::Snapshot;
pub use state::{State, StateError};

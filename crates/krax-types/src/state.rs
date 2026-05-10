//! State trait and its associated error type for the Krax state backend.

use alloy_primitives::B256;
use thiserror::Error;

use crate::snapshot::Snapshot;

/// Errors returned by [`State`] and [`Snapshot`] operations.
///
/// Starts with a single variant. `#[non_exhaustive]` ensures downstream match
/// arms won't break when I/O variants are added in Step 1.3.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum StateError {
    /// The snapshot was already released and can no longer be read.
    #[error("snapshot has been released")]
    Released,
}

/// The V1↔V2 state backend contract.
///
/// All state mutations flow through this trait. Concrete implementations live
/// in `krax-state`; consumers depend only on this abstraction (AGENTS.md Rule 1).
/// Consumed as `&mut dyn State` in the commit phase and as `&dyn State` for
/// read-only paths such as RPC root queries.
pub trait State: Send + Sync {
    /// Returns the current value of `slot`, or `B256::ZERO` if unset.
    fn get(&self, slot: B256) -> Result<B256, StateError>;

    /// Writes `val` to `slot`. Writes are pending until [`commit`][State::commit].
    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError>;

    /// Creates an isolated read-only snapshot at the current commit point.
    ///
    /// Workers read from snapshots so they never observe each other's pending
    /// writes. See AGENTS.md "State Snapshot".
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError>;

    /// Durably applies all pending writes and returns the post-commit state root.
    ///
    /// The returned root will be posted to Ethereum L1 in Phase 14.
    fn commit(&mut self) -> Result<B256, StateError>;

    /// Returns the current state root without committing pending writes.
    ///
    /// Concrete implementations may return a cached value.
    fn root(&self) -> B256;
}

// Compile-time assertion that State is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
const _: Option<&dyn State> = None;

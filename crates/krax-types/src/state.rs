//! State trait and its associated error type for the Krax state backend.

use alloy_primitives::B256;
use thiserror::Error;

use crate::snapshot::Snapshot;

/// Errors returned by [`State`] and [`Snapshot`] operations.
///
/// `#[non_exhaustive]` ensures downstream match arms won't break when
/// additional variants are added.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum StateError {
    /// The snapshot was already released and can no longer be read.
    #[error("snapshot has been released")]
    Released,
    /// Underlying storage I/O failure.
    ///
    /// Source is boxed (`Box<dyn std::error::Error + Send + Sync>`) so
    /// `krax-types` does not depend on any specific storage backend. V1's
    /// MDBX backend (Step 1.3b) wraps `reth_db::DatabaseError` here; V2's
    /// LSM backend will wrap its own error type without touching this enum.
    /// Trade-off: callers cannot statically downcast to a backend-specific
    /// error type (Decision 5 maintainer revision).
    #[error("state I/O error: {0}")]
    Io(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl StateError {
    /// Constructs an [`StateError::Io`] from any `Send + Sync + 'static` error.
    ///
    /// Use this at storage-backend call sites: e.g.
    /// `reth_db_call().map_err(StateError::io)?`.
    pub fn io<E>(source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Io(Box::new(source))
    }
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
    ///
    /// # Panics
    ///
    /// The trait signature is infallible (Step 1.5 Decision 12 (d)).
    /// Implementations MAY `panic!` on unrecoverable internal storage
    /// failure after emitting `tracing::error!`. The V1 MDBX-backed
    /// implementation (`MptState::root` in `krax-state`) panics on cursor
    /// or txn errors during the slot scan — these are unrecoverable for
    /// the surrounding commit pipeline. Callers must NOT invoke `root`
    /// against a state whose backing storage is suspected corrupt.
    ///
    /// (`MptState` is defined in `krax-state/src/mpt/mod.rs`; it is not
    /// importable from `krax-types` to avoid a backend dependency.)
    fn root(&self) -> B256;
}

// Compile-time assertion that State is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
const _: Option<&dyn State> = None;

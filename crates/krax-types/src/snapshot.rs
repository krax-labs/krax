//! Snapshot trait for isolated read-only views of Krax state.

use alloy_primitives::B256;

use crate::state::StateError;

/// A consistent read-only view of state at a single commit point.
///
/// Obtained via [`State::snapshot`][crate::State::snapshot]. Workers read from
/// snapshots so they never observe each other's uncommitted writes (AGENTS.md
/// "State Snapshot").
///
/// `release` takes `self: Box<Self>` — the `Box` is consumed, so any attempt to
/// call `get` after `release` is a compile error ("borrow of moved value"). This
/// is the compile-time guarantee chosen in step-1.1a-decisions.md Decision 1.
pub trait Snapshot: Send + Sync {
    /// Returns the value of `slot` at the snapshot's commit point.
    fn get(&self, slot: B256) -> Result<B256, StateError>;

    /// Returns the MPT root as of the snapshot's commit point.
    ///
    /// The returned root reflects the state visible to this snapshot —
    /// NOT the live state, and NOT any post-snapshot commits made on the
    /// underlying [`State`][crate::State]. Implementations MAY cache the
    /// computed root for the snapshot's lifetime (the V1 MDBX backend's
    /// `MptSnapshot` does — Step 1.5 Decision 3 (b)).
    ///
    /// Infallible by design (Step 1.5 Decisions 12 (d) + 14 (a)): an
    /// internal storage-read failure during root computation is
    /// unrecoverable for the surrounding commit pipeline. Implementations
    /// MAY `panic!` on storage corruption after emitting
    /// `tracing::error!`. V1 callers must NOT call `root` against a
    /// snapshot whose underlying storage is suspected corrupt.
    ///
    /// (`MptSnapshot` is defined in `krax-state/src/mpt/mod.rs`; it is not
    /// importable from `krax-types` to avoid a backend dependency.)
    fn root(&self) -> B256;

    /// Releases this snapshot, consuming it.
    ///
    /// Post-release reads on the same handle are a compile-time error, not a
    /// runtime check. The receiver `self: Box<Self>` is consumed; any subsequent
    /// use of the original `Box<dyn Snapshot>` triggers E0382 ("borrow of moved
    /// value"). Verified by the `compile_fail` doctest below (Step 1.4
    /// Decisions 3 + 4 — `compile_fail` doctest only, hosted on the trait method;
    /// trait-level stub keeps the doctest free of `krax-state` and `tempfile`):
    ///
    /// ```compile_fail
    /// # use alloy_primitives::B256;
    /// # use krax_types::{Snapshot, StateError};
    /// struct S;
    /// impl Snapshot for S {
    ///     fn get(&self, _slot: B256) -> Result<B256, StateError> { Ok(B256::ZERO) }
    ///     fn root(&self) -> B256 { B256::ZERO }
    ///     fn release(self: Box<Self>) {}
    /// }
    /// let s: Box<dyn Snapshot> = Box::new(S);
    /// s.release();
    /// drop(s); // error[E0382]: use of moved value: `s`
    /// ```
    fn release(self: Box<Self>);
}

// Compile-time assertion that Snapshot is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
const _: Option<&dyn Snapshot> = None;

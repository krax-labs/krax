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

    /// Releases this snapshot, consuming it.
    ///
    /// Post-release reads on the same handle are a compile-time error, not a
    /// runtime check. Step 1.4 tests this via `trybuild` or a `compile_fail`
    /// doctest.
    fn release(self: Box<Self>);
}

// Compile-time assertion that Snapshot is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
const _: Option<&dyn Snapshot> = None;

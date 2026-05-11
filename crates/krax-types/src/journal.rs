//! Worker journal — in-memory record of speculative writes.

use alloy_primitives::B256;

use crate::state::{State, StateError};

/// A single write recorded in a worker's speculative journal.
///
/// `old` uses `B256::ZERO` for "slot was unset" — the EVM storage model has no
/// distinct "absent" state; SLOAD on an unset slot returns `B256::ZERO`. This
/// avoids `Option<B256>` and the attendant unwrapping in `discard`. The EVM
/// gas refund model (EIP-2200 "original value") is tracked separately by revm's
/// own journal inside the EVM executor; Krax's `JournalEntry` only needs to
/// know what value to restore if this tx is discarded.
/// See step-1.1b-decisions.md Decision 8.
#[derive(Debug, PartialEq, Eq)]
pub struct JournalEntry {
    /// Storage slot written.
    pub slot: B256,
    /// Value of the slot before this write; `B256::ZERO` if the slot was unset.
    pub old: B256,
    /// Value written to the slot.
    pub new: B256,
}

/// An ordered list of speculative writes produced by a single worker.
///
/// Workers write to a `Journal` instead of the main state, keeping their
/// execution isolated from other workers and from committed state. After the
/// commit phase verifies no conflicts, [`apply`][Journal::apply] flushes the
/// journal to state. On conflict, [`discard`][Journal::discard] drops it.
#[derive(Debug, PartialEq, Eq)]
pub struct Journal {
    /// Writes in the order they occurred during speculative execution.
    ///
    /// The same slot may appear multiple times — the last write wins per EVM
    /// semantics. `apply` iterates in order, so later entries override earlier
    /// ones on the same slot (correct EVM behavior). `Vec` not `BTreeSet` because
    /// this is an ordered log, not a set — see step-1.1b-decisions.md Decision 11.
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    /// Applies all journal entries to `state` in write order.
    ///
    /// Borrows `self` so callers can inspect the journal after applying it —
    /// the Phase 6 `CommitReport` may count written slots or log entries post-apply.
    /// Applying a journal twice is idempotent at the EVM-state level; the
    /// type system does not prevent it, so callers must manage this via logic.
    /// See step-1.1b-decisions.md Decision 9.
    pub fn apply(&self, state: &mut dyn State) -> Result<(), StateError> {
        for entry in &self.entries {
            state.set(entry.slot, entry.new)?;
        }
        Ok(())
    }

    /// Discards this journal without applying it to state.
    ///
    /// Consumes `self` — there is no meaningful use of a journal after discard.
    /// Mirrors `Snapshot::release(self: Box<Self>)` from Step 1.1a.
    /// See step-1.1b-decisions.md Decision 10.
    pub fn discard(self) {}
}

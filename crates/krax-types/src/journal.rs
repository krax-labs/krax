//! Worker journal — in-memory record of speculative writes.
//!
//! `JournalEntry` (the per-write record) lives in the sibling [`journal_entry`]
//! module and is re-exported from here for crate-external callers. The split
//! keeps `journal.rs` focused on `Journal` + `impl Journal` (the logic surface)
//! and isolates the data-only `JournalEntry` so coverage measurement reflects
//! exercise of real logic. See step-1.3.5-decisions.md Decision 2 (revised).

pub use crate::journal_entry::JournalEntry;

use crate::state::{State, StateError};

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

    /// Discards the journal's pending writes without applying them.
    ///
    /// Used on conflict detection (Phase 6): the misspeculating worker's journal
    /// is discarded and the transaction is queued for serial re-execution.
    ///
    /// Consumes `self` — attempting to use the journal after `discard` is a compile error:
    ///
    /// ```compile_fail
    /// # use krax_types::Journal;
    /// let journal = Journal { entries: Vec::new() };
    /// journal.discard();
    /// drop(journal); // error[E0382]: use of moved value: `journal`
    /// ```
    pub fn discard(self) {}
}

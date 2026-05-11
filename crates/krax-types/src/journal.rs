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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use alloy_primitives::B256;
    use pretty_assertions::assert_eq;

    use super::{Journal, JournalEntry};
    use crate::snapshot::Snapshot;
    use crate::state::{State, StateError};
    use crate::test_helpers::slot;

    // StubState: minimal in-test State impl. Scaffolding only — deleted in Step 1.3.
    // See docs/plans/step-1.2-decisions.md Decision 6 + Post-execution directives below.
    struct StubState(BTreeMap<B256, B256>);

    impl StubState {
        fn new() -> Self {
            StubState(BTreeMap::new())
        }
    }

    impl State for StubState {
        fn get(&self, slot: B256) -> Result<B256, StateError> {
            Ok(*self.0.get(&slot).unwrap_or(&B256::ZERO))
        }

        fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> {
            self.0.insert(slot, val);
            Ok(())
        }

        // Never called by apply or discard; non-panicking placeholder avoids
        // unimplemented!/todo! which are deny-listed at workspace level.
        fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
            Err(StateError::Released)
        }

        fn commit(&mut self) -> Result<B256, StateError> {
            Ok(B256::ZERO)
        }

        fn root(&self) -> B256 {
            B256::ZERO
        }
    }

    #[test]
    fn apply_empty_journal_leaves_state_unchanged() {
        let mut state = StubState::new();
        let journal = Journal { entries: vec![] };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.0.len(), 0);
    }

    #[test]
    fn apply_single_entry_writes_slot() {
        let mut state = StubState::new();
        let journal = Journal {
            entries: vec![JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(42) }],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.0.get(&slot(1)), Some(&slot(42)));
    }

    #[test]
    fn apply_last_write_wins_on_same_slot() {
        let mut state = StubState::new();
        let journal = Journal {
            entries: vec![
                JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(10) },
                JournalEntry { slot: slot(1), old: slot(10), new: slot(20) },
            ],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.0.get(&slot(1)), Some(&slot(20)));
    }
}

//! In-memory `MptState` — Step 1.3a backend.
//!
//! Single `BTreeMap<B256, B256>` slot store implementing [`State`] and
//! [`Snapshot`]. No MDBX, no I/O. Step 1.3b replaces the in-memory map with
//! `reth-db`-backed durability; Step 1.5 replaces the placeholder root with
//! real Ethereum-compatible MPT root computation.
//!
//! Decisions: docs/plans/step-1.3a-decisions.md.

use std::collections::BTreeMap;

use alloy_primitives::B256;
use krax_types::{Snapshot, State, StateError};

/// In-memory implementation of the [`State`] trait.
///
/// Backed by a single `BTreeMap<B256, B256>` per Decision 3 — no pending /
/// committed layering in 1.3a; that distinction belongs to 1.3b's MDBX
/// transaction model if it surfaces meaningfully there. Writes are visible to
/// subsequent `get` calls without a prior `commit` (Decision 5).
#[derive(Debug, Default)]
pub struct MptState {
    slots: BTreeMap<B256, B256>,
}

impl MptState {
    /// Constructs an empty `MptState`.
    ///
    /// 1.3a's backing is in-memory; 1.3b will accept a path/handle for the
    /// MDBX-backed variant.
    #[must_use]
    pub fn new() -> Self {
        Self { slots: BTreeMap::new() }
    }
}

/// In-memory implementation of the [`Snapshot`] trait.
///
/// Owns a clone of the source `MptState`'s slot map (Decision 4 — clone, not
/// `Arc`). Subsequent writes to the source state do not affect this snapshot.
#[derive(Debug)]
pub struct MptSnapshot {
    slots: BTreeMap<B256, B256>,
}

impl State for MptState {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        Ok(self.slots.get(&slot).copied().unwrap_or(B256::ZERO))
    }

    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> {
        self.slots.insert(slot, val);
        Ok(())
    }

    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
        Ok(Box::new(MptSnapshot { slots: self.slots.clone() }))
    }

    fn commit(&mut self) -> Result<B256, StateError> {
        // Decision 5 — no-op checkpoint: writes are immediately visible via
        // `get`; `commit` returns the current root for caller bookkeeping.
        Ok(self.root())
    }

    fn root(&self) -> B256 {
        // TODO Step 1.5 — MPT Root Computation:
        // replace placeholder with real Ethereum-compatible MPT root.
        // Decision (alloy-trie vs custom MPT) surfaced in step-1.3a-decisions.md
        // and answered before Step 1.5 dispatch.
        B256::ZERO
    }
}

impl Snapshot for MptSnapshot {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        Ok(self.slots.get(&slot).copied().unwrap_or(B256::ZERO))
    }

    fn release(self: Box<Self>) {
        // Consuming release: the `Box<Self>` is dropped on return, freeing the
        // owned slot map. No additional cleanup required for the in-memory
        // backend (Decision 4).
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use alloy_primitives::B256;
    use pretty_assertions::assert_eq;

    use super::*;

    fn slot(n: u8) -> B256 {
        B256::from([n; 32])
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut state = MptState::new();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_empty_journal_leaves_state_unchanged() {
        use krax_types::{Journal, State};

        let mut state = MptState::new();
        let journal = Journal { entries: vec![] };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), B256::ZERO);
    }

    #[test]
    fn apply_single_entry_writes_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let mut state = MptState::new();
        let journal = Journal {
            entries: vec![JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(42) }],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_last_write_wins_on_same_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let mut state = MptState::new();
        let journal = Journal {
            entries: vec![
                JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(10) },
                JournalEntry { slot: slot(1), old: slot(10), new: slot(20) },
            ],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(20));
    }
}

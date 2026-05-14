//! MDBX-backed `MptState` — Step 1.3b backend.
//!
//! Replaces the Step 1.3a in-memory `BTreeMap` with a `reth-db`-backed
//! MDBX environment storing a single flat `Slots: B256 → Vec<u8>` table
//! (Decision 6; LVP-driven Value-type substitution — see Deviation 3
//! below). `set` auto-flushes per call via a short-lived `RwTxn`
//! (Decision 4); `snapshot` opens an RoTxn-backed [`MptSnapshot`]
//! (Decision 3); `commit` is a sync barrier returning the (placeholder)
//! root (Decisions 4 + 6 + Step 1.5 deferral). Real MPT root computation
//! lands in Step 1.5.
//!
//! Decisions: docs/plans/step-1.3b-decisions.md.
//!
//! ## Library Verification Protocol — confirmed surfaces (per Step 1.3b plan)
//!
//! Verified against reth-db pinned rev `02d1776786abc61721ae8876898ad19a702e0070`
//! (2026-05-06), with on-disk source fallback for items Context7 did not surface.
//!
//! - **Q1 (env open) — DEVIATION:** Actual function is
//!   `reth_db::mdbx::init_db_for::<P, TS: TableSet>(path, DatabaseArguments)
//!   -> eyre::Result<DatabaseEnv>` (source: `crates/storage/db/src/mdbx.rs:107`).
//!   Planner expected `reth_db::create_db_and_tables`. `init_db_for` is the
//!   correct entry point when registering a custom `TableSet` (we do).
//!   Return type is `eyre::Result`, not `Result<_, DatabaseError>` — `eyre::Report`
//!   does not implement `std::error::Error`, so we convert via
//!   `std::io::Error::other(e.to_string())` before boxing into `StateError::Io`.
//! - **Q2 (RoTxn/RwTxn) — confirmed:** `Database::tx() -> Result<Self::TX,
//!   DatabaseError>`, `Database::tx_mut() -> Result<Self::TXMut, DatabaseError>`,
//!   `Self::TX: DbTx + Send + Sync + Debug + 'static`. Storing
//!   `<DatabaseEnv as Database>::TX` in `MptSnapshot` is sound (no lifetime
//!   parameter required — Decision 3 structural shape preserved).
//!   `DbTx::get::<T>(&self, key) -> Result<Option<T::Value>, DatabaseError>`,
//!   `DbTx::commit(self) -> Result<(), DatabaseError>` (consuming),
//!   `DbTxMut::put::<T>(&self, key, value) -> Result<(), DatabaseError>`.
//! - **Q3 (`tables!` macro) — DEVIATION, fallback taken:** The macro IS at
//!   `reth_db::tables!`, but it emits `pub enum Tables { ... }` and
//!   `impl TableSet for Tables`, referencing a sibling `table_names` module
//!   that only exists inside reth-db itself. Cannot be invoked from outside
//!   reth-db. Hand-roll the `Table` + `TableInfo` + `TableSet` impls per
//!   Decision 7 fallback.
//! - **Q5 (`DatabaseError`) — confirmed:** `reth_db::DatabaseError` (re-export of
//!   `reth_storage_errors::db::DatabaseError`) implements
//!   `std::error::Error + Send + Sync + 'static`. Compatible with
//!   `StateError::io()` constructor via `.map_err(StateError::io)?`.
//! - **Q6 (B256 codec) — PARTIAL DEVIATION:** `impl Encode for B256` and
//!   `impl Decode for B256` (source: `db-api/src/models/mod.rs:89-101`) — 32 raw
//!   bytes, no padding — so B256 is directly usable as `type Key = B256;`.
//!   On the Value side, B256 has NO `Compress` impl anywhere in the
//!   reth-db / reth-codecs / reth-primitives-traits tree we can reach;
//!   `Vec<u8>: Compress + Decompress + Serialize` blanket impl is provided
//!   by `reth_codecs::compress::scale::impl_compression_for_scale!`. Using
//!   `Vec<u8>` as the table Value and converting at the `State::get` /
//!   `State::set` boundary keeps the wire format as exactly 32 bytes
//!   (the load-bearing property — matches the planner-expected on-disk shape)
//!   without adding a serde/serialize dep just to wrap B256.
//! - **Q7 (test helpers) — confirmed:** Using `tempfile::TempDir` directly,
//!   per the plan.
//! - **Configuration deviation:** `reth-db` workspace dep is
//!   `default-features = false`; `DatabaseEnv` / `init_db_for` /
//!   `DatabaseArguments` are gated on the `mdbx` feature. Therefore
//!   `crates/krax-state/Cargo.toml` enables `features = ["mdbx"]` on
//!   `reth-db`.

use std::path::Path;
use std::sync::Arc;

use alloy_primitives::B256;
use krax_types::{Snapshot, State, StateError};
use reth_db::{
    Database,
    mdbx::{DatabaseArguments, DatabaseEnv, init_db_for},
    transaction::{DbTx, DbTxMut},
};

mod slots;

use slots::{Slots, SlotsTableSet};

/// Converts any `Display`-able error (notably `eyre::Report`) to a
/// [`StateError::Io`] via an [`std::io::Error`] wrapper.
///
/// `eyre::Report` does not implement [`std::error::Error`]; we render its
/// `Display` and wrap in `std::io::Error::other` which DOES implement
/// `std::error::Error + Send + Sync + 'static`, then box it through
/// [`StateError::io`]. Loses the original error chain but preserves the
/// message. Generic over the input type so callers don't have to name
/// `eyre::Report` (avoids requiring `eyre` as a direct dependency).
fn display_to_state<E: std::fmt::Display>(e: E) -> StateError {
    StateError::io(std::io::Error::other(e.to_string()))
}

/// MDBX-backed implementation of the [`State`] trait.
///
/// Owns a refcounted handle to the underlying MDBX environment
/// (`Arc<DatabaseEnv>`). Cloning the `Arc` is cheap and lets [`MptSnapshot`]
/// hold its own reference for the lifetime of the read transaction —
/// required because [`State::snapshot`] returns a `Box<dyn Snapshot>` with no
/// borrow back to `self` (Decision 3).
#[derive(Debug)]
pub struct MptState {
    env: Arc<DatabaseEnv>,
}

impl MptState {
    /// Opens (or creates) the MDBX environment rooted at `path`.
    ///
    /// Registers the [`Slots`] table on first open via
    /// [`init_db_for`]. Returns [`StateError::Io`] if the environment cannot
    /// be opened or the table cannot be initialized.
    pub fn open(path: &Path) -> Result<Self, StateError> {
        let env = init_db_for::<_, SlotsTableSet>(path, DatabaseArguments::default())
            .map_err(display_to_state)?;
        Ok(Self { env: Arc::new(env) })
    }

    /// Opens an [`MptState`] rooted at a fresh `TempDir`.
    ///
    /// Returns the `TempDir` alongside the state so the caller controls drop
    /// ordering — the directory is removed when the `TempDir` is dropped.
    /// Test-and-integration-only helper (Decision 1).
    #[cfg(any(test, feature = "integration"))]
    pub fn open_temporary() -> Result<(Self, tempfile::TempDir), StateError> {
        let dir = tempfile::TempDir::new()
            .expect("MptState::open_temporary: tempdir creation failed");
        let state = Self::open(dir.path())?;
        Ok((state, dir))
    }
}

/// Decode a `Slots` raw `Vec<u8>` value into a [`B256`].
///
/// All writes go through [`MptState::set`] which encodes exactly 32 bytes; a
/// non-32-byte read surface indicates database corruption and surfaces as
/// [`StateError::Io`].
fn decode_slot_value(bytes: &[u8]) -> Result<B256, StateError> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| {
        StateError::io(std::io::Error::other(format!(
            "Slots value must be 32 bytes (got {})",
            bytes.len()
        )))
    })?;
    Ok(B256::new(arr))
}

impl State for MptState {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        let tx = self.env.tx().map_err(StateError::io)?;
        let raw = tx.get::<Slots>(slot).map_err(StateError::io)?;
        let result = match raw {
            None => B256::ZERO,
            Some(bytes) => decode_slot_value(&bytes)?,
        };
        tx.commit().map_err(StateError::io)?;
        Ok(result)
    }

    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> {
        // Decision 4 (b): auto-flush per set — open, write, commit a
        // short-lived RwTxn. Writes are durable + visible to subsequent
        // `get` calls without an intervening `State::commit`.
        let tx = self.env.tx_mut().map_err(StateError::io)?;
        tx.put::<Slots>(slot, val.0.to_vec()).map_err(StateError::io)?;
        tx.commit().map_err(StateError::io)?;
        Ok(())
    }

    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
        // Decision 3 (a): RoTxn-backed snapshot. Reads through the txn observe
        // a stable view; MDBX MVCC isolates from concurrent writes against the
        // same env.
        let tx = self.env.tx().map_err(StateError::io)?;
        Ok(Box::new(MptSnapshot { tx }))
    }

    fn commit(&mut self) -> Result<B256, StateError> {
        // Decision 4 (b): `set` already committed each individual write —
        // `commit` here is a sync-barrier semantic equivalent to 1.3a's no-op.
        // Returns the current (placeholder) root for caller bookkeeping.
        Ok(self.root())
    }

    fn root(&self) -> B256 {
        // TODO Step 1.5 — MPT Root Computation:
        // replace placeholder with real Ethereum-compatible MPT root
        // (alloy-trie vs custom MPT — decision pre-surfaced in
        // docs/plans/step-1.3a-decisions.md, answered at 1.5 dispatch).
        B256::ZERO
    }
}

/// MDBX read-only snapshot.
///
/// Owns a reth-db `RoTxn` (Decision 3); reads traverse the txn directly. Drop
/// releases the MDBX reader slot via the txn's `Drop` impl (Decision 11).
// Drop: relies on `tx`'s auto-Drop, which releases the MDBX reader slot
// (Step 1.4 Decision 13 — RAII; no explicit Drop impl, no explicit abort()).
#[derive(Debug)]
pub struct MptSnapshot {
    tx: <DatabaseEnv as Database>::TX,
}

impl Snapshot for MptSnapshot {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        let raw = self.tx.get::<Slots>(slot).map_err(StateError::io)?;
        match raw {
            None => Ok(B256::ZERO),
            Some(bytes) => decode_slot_value(&bytes),
        }
    }

    fn release(self: Box<Self>) {
        // Decision 11 (a): drop releases the RoTxn via RAII — the `Box<Self>`
        // is dropped on return, `tx` drops, MDBX releases the reader slot.
        // No explicit `RoTxn::abort()` call (LVP Query 8 conditional).
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
        let (mut state, _tmp) = MptState::open_temporary().unwrap();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_empty_journal_leaves_state_unchanged() {
        use krax_types::{Journal, State};

        let (mut state, _tmp) = MptState::open_temporary().unwrap();
        let journal = Journal { entries: vec![] };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), B256::ZERO);
    }

    #[test]
    fn apply_single_entry_writes_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let (mut state, _tmp) = MptState::open_temporary().unwrap();
        let journal = Journal {
            entries: vec![JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(42) }],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_last_write_wins_on_same_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let (mut state, _tmp) = MptState::open_temporary().unwrap();
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

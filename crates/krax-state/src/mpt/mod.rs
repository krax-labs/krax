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
use std::sync::{Arc, OnceLock};

use alloy_primitives::B256;
use krax_types::{Snapshot, State, StateError};
use reth_db::{
    Database,
    cursor::DbCursorRO,
    mdbx::{DatabaseArguments, DatabaseEnv, init_db_for},
    transaction::{DbTx, DbTxMut},
};

mod slots;
mod trie;

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
    /// Memoized MPT root (Step 1.5 Decision 2 (b)). `OnceLock` (not
    /// `Option`) because `root(&self)` must populate it through a shared
    /// borrow — `Cell` is not `Sync` and `State: Send + Sync` requires it
    /// (symmetry with `MptSnapshot`'s `OnceLock`, Decision 3 (b)).
    /// `set` invalidates by replacing the lock (`&mut self`); `commit`
    /// repopulates it with the post-commit root (Decision 19 (a)).
    cached_root: OnceLock<B256>,
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
        Ok(Self { env: Arc::new(env), cached_root: OnceLock::new() })
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

    /// Computes the live MPT root by walking the `Slots` table on a fresh
    /// RO txn (Step 1.5 Decision 8 (a)).
    ///
    /// Infallible by design (Decision 12 (d)): an MDBX failure here is
    /// unrecoverable for the surrounding commit pipeline, so each fallible
    /// step emits `tracing::error!` then `panic!` rather than swallowing
    /// the error or widening the [`State::root`] signature. Four panic
    /// sites: txn open, cursor open, cursor walk, slot-value decode.
    fn compute_root_from_storage(&self) -> B256 {
        // Per Context7 LVP-Q5 (reth-db @ 02d1776, Step 1.5):
        // `DbTx::cursor_read::<T>() -> Result<Self::Cursor<T>, _>`;
        // `DbCursorRO::walk(None) -> Result<Walker, _>` where
        // `Walker: Iterator<Item = Result<(T::Key, T::Value), _>>` yields
        // rows in B-tree key order.
        let tx = self.env.tx().unwrap_or_else(|e| {
            tracing::error!(error = %e, "MDBX txn open failure in MptState::root");
            panic!("MDBX txn open failure in MptState::root: {e}");
        });
        let mut cursor = tx.cursor_read::<Slots>().unwrap_or_else(|e| {
            tracing::error!(error = %e, "MDBX cursor open failure in MptState::root");
            panic!("MDBX cursor open failure in MptState::root: {e}");
        });
        let walker = cursor.walk(None).unwrap_or_else(|e| {
            tracing::error!(error = %e, "MDBX cursor walk failure in MptState::root");
            panic!("MDBX cursor walk failure in MptState::root: {e}");
        });
        let entries = walker.map(|row| match row {
            Ok((slot, raw)) => (
                slot,
                decode_slot_value(&raw).unwrap_or_else(|e| {
                    tracing::error!(error = %e, "MDBX Slots value decode failure in MptState::root");
                    panic!("MDBX Slots value decode failure in MptState::root: {e}");
                }),
            ),
            Err(e) => {
                tracing::error!(error = %e, "MDBX cursor walk failure in MptState::root");
                panic!("MDBX cursor walk failure in MptState::root: {e}");
            }
        });
        trie::compute_root(entries)
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
        // Step 1.5 Decision 2 (b): invalidate the memoized root before the
        // write — the next `root()` recomputes against the new slot set.
        self.cached_root = OnceLock::new();
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
        Ok(Box::new(MptSnapshot { tx, cached_root: OnceLock::new() }))
    }

    fn commit(&mut self) -> Result<B256, StateError> {
        // Decision 4 (b): `set` already committed each individual write —
        // `commit` here is a sync barrier. Decision 19 (a): compute the
        // post-commit root once and repopulate the memo so subsequent
        // `root()` calls are free. `&mut self` gives exclusive access, so
        // replace the (set-invalidated) lock with a freshly-seeded one.
        let r = self.compute_root_from_storage();
        self.cached_root = OnceLock::new();
        let _ = self.cached_root.set(r);
        Ok(r)
    }

    fn root(&self) -> B256 {
        // Step 1.5 Decisions 2 (b) + 8 (a) + 12 (d) + 14 (a): lazily
        // compute the MPT root on first call (or after a `set`
        // invalidation) and memoize it; recompute walks a fresh RO txn
        // cursor. Infallible — internal MDBX failure panics after a
        // `tracing::error!` (see `compute_root_from_storage`).
        *self.cached_root.get_or_init(|| self.compute_root_from_storage())
    }
}

/// MDBX read-only snapshot.
///
/// Owns a reth-db `RoTxn` (Decision 3); reads traverse the txn directly. Drop
/// releases the MDBX reader slot via the txn's `Drop` impl (Decision 11).
///
/// Caches the computed MPT root lazily in `cached_root` (Step 1.5 Decision
/// 3 (b)) — the first [`Snapshot::root`] call walks the slots via the
/// snapshot's RO cursor and populates the cache; later calls return the
/// cached value. The cache is per-snapshot and does NOT share with
/// [`MptState`]'s memo (different view; Decision 2 (b)).
// Drop: relies on `tx`'s auto-Drop, which releases the MDBX reader slot
// (Step 1.4 Decision 13 — RAII; no explicit Drop impl, no explicit abort()).
#[derive(Debug)]
pub struct MptSnapshot {
    tx: <DatabaseEnv as Database>::TX,
    cached_root: OnceLock<B256>,
}

impl Snapshot for MptSnapshot {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        let raw = self.tx.get::<Slots>(slot).map_err(StateError::io)?;
        match raw {
            None => Ok(B256::ZERO),
            Some(bytes) => decode_slot_value(&bytes),
        }
    }

    fn root(&self) -> B256 {
        // Step 1.5 Decisions 1 (a) + 3 (b) + 8 (a) + 12 (d) + 14 (a):
        // lazy + cache; cursor walk on the snapshot's own RO txn (the
        // frozen view, NOT live state); infallible — panic on MDBX
        // failure after `tracing::error!`. `OnceLock<B256>` gives the
        // `&self` interior mutability with `Send + Sync` (the `Snapshot`
        // supertrait).
        *self.cached_root.get_or_init(|| {
            // Per Context7 LVP-Q5 (reth-db @ 02d1776, Step 1.5): cursor
            // walk on the held RO txn iterates `Slots` in B-tree key
            // order; `Walker: Iterator<Item = Result<(K, V), _>>`.
            let mut cursor = self.tx.cursor_read::<Slots>().unwrap_or_else(|e| {
                tracing::error!(error = %e, "MDBX cursor open failure in MptSnapshot::root");
                panic!("MDBX cursor open failure in MptSnapshot::root: {e}");
            });
            let walker = cursor.walk(None).unwrap_or_else(|e| {
                tracing::error!(error = %e, "MDBX cursor walk failure in MptSnapshot::root");
                panic!("MDBX cursor walk failure in MptSnapshot::root: {e}");
            });
            let entries = walker.map(|row| match row {
                Ok((slot, raw)) => (
                    slot,
                    decode_slot_value(&raw).unwrap_or_else(|e| {
                        tracing::error!(error = %e, "MDBX Slots value decode failure in MptSnapshot::root");
                        panic!("MDBX Slots value decode failure in MptSnapshot::root: {e}");
                    }),
                ),
                Err(e) => {
                    tracing::error!(error = %e, "MDBX cursor walk failure in MptSnapshot::root");
                    panic!("MDBX cursor walk failure in MptSnapshot::root: {e}");
                }
            });
            trie::compute_root(entries)
        })
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

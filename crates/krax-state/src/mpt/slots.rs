//! [`Slots`] table schema + [`SlotsTableSet`] enumeration — reth-db trait glue.
//!
//! Split out of `mod.rs` for two reasons: (1) the reth-db trait glue is
//! mechanically distinct from the State/Snapshot semantics of [`MptState`]
//! (separating them keeps `mod.rs` focused on logic as the MPT layer grows
//! in Step 1.5+); and (2) the data-only/glue surface can be excluded from
//! coverage measurement via path-based `--ignore-filename-regex` without
//! also excluding `MptState`'s logic. See step-1.3.5-decisions.md Decision 2
//! (revised) and step-1.3b-decisions.md (Cross-Step Impact section that
//! pre-flagged this split).

use alloy_primitives::B256;
use reth_db::{
    table::{Table, TableInfo},
    tables::TableSet,
};

/// Flat slot table backing [`MptState`][crate::MptState].
///
/// Key: [`B256`] storage slot identifier (encoded as 32 raw bytes via the
/// `Encode`/`Decode` impls in `reth-db`).
/// Value: `Vec<u8>` carrying exactly 32 bytes (the B256 value). The Value-type
/// choice is an LVP-driven deviation — see the crate-level docs in
/// [`mpt`][crate::mpt].
#[derive(Debug)]
pub struct Slots;

impl Table for Slots {
    const NAME: &'static str = "Slots";
    const DUPSORT: bool = false;
    type Key = B256;
    type Value = Vec<u8>;
}

impl TableInfo for Slots {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn is_dupsort(&self) -> bool {
        Self::DUPSORT
    }
}

/// [`TableSet`] enumeration for [`init_db_for`][reth_db::mdbx::init_db_for].
///
/// Single-table set — registers [`Slots`] with the MDBX environment on
/// first open so subsequent `put`/`get` calls observe a valid sub-database.
#[derive(Debug)]
pub(super) struct SlotsTableSet;

impl TableSet for SlotsTableSet {
    fn tables() -> Box<dyn Iterator<Item = Box<dyn TableInfo>>> {
        Box::new(std::iter::once(Box::new(Slots) as Box<dyn TableInfo>))
    }
}

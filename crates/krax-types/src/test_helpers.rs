//! Shared test helpers for krax-types unit tests.

use alloy_primitives::B256;

use crate::RWSet;

/// Returns a `B256` where every byte equals `n`. Compact slot-key generator for test cases.
pub(crate) fn slot(n: u8) -> B256 {
    B256::from([n; 32])
}

/// Constructs `RWSet::Concrete` from iterables of read-slots and write-slots.
pub(crate) fn concrete(
    r: impl IntoIterator<Item = B256>,
    w: impl IntoIterator<Item = B256>,
) -> RWSet {
    RWSet::Concrete {
        r_set: r.into_iter().collect(),
        w_set: w.into_iter().collect(),
    }
}

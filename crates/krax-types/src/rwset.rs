//! Read/write set for speculative transaction conflict detection.

use std::collections::BTreeSet;

use alloy_primitives::B256;

/// The read and write sets inferred or observed for a transaction.
///
/// Used by the conflict detector (Phase 6) to decide whether a speculatively
/// executed transaction must be re-executed serially against current state.
///
/// `Clone` is deliberately omitted — borrowing semantics on [`union`][RWSet::union]
/// and [`conflicts`][RWSet::conflicts] remove all in-tree clone call sites at this
/// stage. Derive `Clone` when a real call site needs it.
/// See step-1.1b-decisions.md Decision 7.
#[derive(Debug, PartialEq, Eq)]
pub enum RWSet {
    /// Concrete read and write sets inferred or measured for a transaction.
    Concrete {
        /// Storage slots read by the transaction.
        r_set: BTreeSet<B256>,
        /// Storage slots written by the transaction.
        w_set: BTreeSet<B256>,
    },
    /// Conservative sentinel: conflicts with all other RW-sets.
    ///
    /// Returned by the conservative inferer (Phase 4, Step 4.1) when the
    /// transaction's access pattern cannot be statically determined. Modelling
    /// this as an enum variant from day one avoids a public-API breaking change
    /// at Phase 4 when `Everything` becomes load-bearing across workers, the
    /// conflict detector, and tests. See step-1.1b-decisions.md Decision 6.
    Everything,
}

impl RWSet {
    /// Returns `true` if executing `self` and `other` speculatively could
    /// produce incorrect state.
    ///
    /// Two `Concrete` RW-sets conflict when either writes a slot the other reads
    /// or writes. `Everything` conflicts with every RW-set, including itself —
    /// the conservative inferer's guarantee that re-execution is always safe.
    pub fn conflicts(&self, other: &RWSet) -> bool {
        match (self, other) {
            (RWSet::Everything, _) | (_, RWSet::Everything) => true,
            (
                RWSet::Concrete { r_set: r1, w_set: w1 },
                RWSet::Concrete { r_set: r2, w_set: w2 },
            ) => !w1.is_disjoint(r2) || !w1.is_disjoint(w2) || !w2.is_disjoint(r1),
        }
    }

    /// Returns the union of `self` and `other`.
    ///
    /// Used by the Phase 6 commit phase to accumulate a cumulative write-set
    /// across committed transactions in mempool order:
    ///
    /// ```ignore
    /// let mut cumulative = RWSet::Concrete { r_set: BTreeSet::new(), w_set: BTreeSet::new() };
    /// for result in committed.iter() {
    ///     cumulative = cumulative.union(&result.rwset);
    /// }
    /// if later_tx.rwset.conflicts(&cumulative) { /* re-execute */ }
    /// ```
    ///
    /// `Everything` absorbs all: `Everything.union(anything) == Everything`.
    #[must_use]
    pub fn union(&self, other: &RWSet) -> RWSet {
        match (self, other) {
            (RWSet::Everything, _) | (_, RWSet::Everything) => RWSet::Everything,
            (
                RWSet::Concrete { r_set: r1, w_set: w1 },
                RWSet::Concrete { r_set: r2, w_set: w2 },
            ) => RWSet::Concrete {
                r_set: r1.union(r2).copied().collect(),
                w_set: w1.union(w2).copied().collect(),
            },
        }
    }
}

//! [`JournalEntry`] — per-write record in a worker's speculative journal.
//!
//! Split out of `journal.rs` so the data-only struct can be excluded from
//! coverage measurement via path-based `--ignore-filename-regex` without
//! also excluding `Journal::apply` / `Journal::discard` (the logic surface).
//! See step-1.3.5-decisions.md Decision 2 (revised).

use alloy_primitives::B256;

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

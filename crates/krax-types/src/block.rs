//! Block type for sealed, committed Krax blocks.

// Per Context7 (alloy-consensus v1, 2026-05-10): TxEnvelope is a type alias for
// EthereumTxEnvelope<TxEip4844> — the canonical EIP-2718 wire-format signed transaction.
// Import path alloy_consensus::TxEnvelope is confirmed valid. See tx.rs for full alias note.
use alloy_consensus::TxEnvelope;
use alloy_primitives::B256;

/// A sealed, committed Krax block.
///
/// Represents the canonical artifact produced by the commit phase after all
/// transactions have been finalized against state. An in-progress batch is
/// `Vec<MempoolEntry>` in the commit phase; it becomes a `Block` only after
/// `state_root` is known. Requiring `state_root` at construction enforces this
/// invariant via the type system.
///
/// Block hash (`keccak(RLP(header))`) is deferred to Phase 11 — no hash field
/// or hash method exists here. Adding either would require RLP infrastructure
/// not yet planned. See step-1.1b-decisions.md Decision 4.
// Per Context7 (/alloy-rs/alloy, 2026-05-11): TxEnvelope does not derive PartialEq — fallback path (Decision 3).
#[derive(Debug)]
pub struct Block {
    /// Hash of the parent block's header.
    pub parent_hash: B256,
    /// Monotonic block number (0-indexed).
    pub height: u64,
    /// Unix timestamp in seconds at which this block was committed.
    pub timestamp: u64,
    /// Transactions in commit order (mempool gas-price order, then arrival-time
    /// tiebreak). Mempool decoration (`sender`, `arrival_time`) is stripped;
    /// only the wire-format envelope is stored. See step-1.1b-decisions.md
    /// Decision 5.
    pub txs: Vec<TxEnvelope>,
    /// State root after applying all transactions in this block.
    pub state_root: B256,
}

impl Block {
    /// Constructs a new sealed block.
    ///
    /// All fields are required. There is no partial or in-progress `Block`
    /// representation — that state is `Vec<MempoolEntry>` in the commit phase.
    pub fn new(
        parent_hash: B256,
        height: u64,
        timestamp: u64,
        txs: Vec<TxEnvelope>,
        state_root: B256,
    ) -> Self {
        Self { parent_hash, height, timestamp, txs, state_root }
    }
}

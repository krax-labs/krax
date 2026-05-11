//! Transaction types for the Krax mempool and worker pipeline.

// Per Context7 (alloy-consensus v1, 2026-05-10): TxEnvelope is a type alias for
// EthereumTxEnvelope<TxEip4844> — the concrete generic type is EthereumTxEnvelope;
// TxEnvelope is the canonical EIP-2718 signed transaction envelope alias. Import path
// alloy_consensus::TxEnvelope is confirmed valid. See step-1.1b-decisions.md Decision 1.
use alloy_consensus::TxEnvelope;
use alloy_primitives::Address;

/// A signed Ethereum transaction as received on the wire.
///
/// Mirrors the EIP-2718 envelope; carries no mempool or block context.
/// Layers without a mempool (RPC ingress, P2P, fuzz harnesses, the V2 fault
/// prover) hold this type and never need to reason about sender recovery or
/// arrival time.
///
/// This is a newtype wrapper, not a re-export. The wrapper adds a stable
/// Krax-specific attachment point for future methods without modifying alloy
/// types directly.
pub struct PendingTx {
    /// The signed EIP-2718 envelope wrapping the typed transaction.
    pub tx: TxEnvelope,
}

/// A transaction enriched by the mempool with recovered sender and arrival time.
///
/// Constructed only by the mempool's validation step (Phase 3, Step 3.1).
/// Workers and the commit phase consume this type, not [`PendingTx`], because
/// they need the sender address and require stable ordering by arrival time.
///
/// `arrival_time` is `u64` Unix milliseconds. The Phase 3 mempool plan MUST
/// specify a deterministic source for this value — `SystemTime::now()` at the
/// mempool layer violates AGENTS.md Rule 7 because two sequencers stamping
/// independently would produce different blocks from the same transaction
/// stream. See step-1.1b-decisions.md Decision 2.
pub struct MempoolEntry {
    /// The wrapped wire-format transaction.
    pub tx: PendingTx,
    /// Sender address recovered from the transaction signature at mempool insertion.
    pub sender: Address,
    /// Unix milliseconds at which this transaction entered the mempool.
    ///
    /// Must come from a deterministic source. See AGENTS.md Rule 7 and
    /// step-1.1b-decisions.md Decision 2.
    pub arrival_time: u64,
}

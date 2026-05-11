# Step 1.1b — Data Types (`PendingTx`, `Block`, `RWSet`, `Journal`)

> **Plan status:** Ready for execution.
> **Phase:** 1 — Domain Types & State Trait (second sub-step).
> **ARCHITECTURE.md reference:** Phase 1, Step 1.1b.
> **Prerequisites:** Step 1.1a complete and committed (`feat(types): define State, Snapshot,
> StateError traits — Step 1.1a`). `make build`, `make lint`, and `make test` all exit 0.

---

## Purpose

Six new deliverables, all inside `crates/krax-types/`:

1. **`src/tx.rs` (new)** — `PendingTx` struct (wire-format wrapper around
   `alloy_consensus::TxEnvelope`) and `MempoolEntry` struct (enriched with recovered
   `sender: Address` and `arrival_time: u64`). Co-located in a single file.

2. **`src/block.rs` (new)** — `Block` struct (`parent_hash`, `height`, `timestamp`,
   `txs: Vec<TxEnvelope>`, `state_root: B256`). No hash field, no hash method, no `From`
   impl; sealed-block invariant enforced by requiring `state_root` at construction.

3. **`src/rwset.rs` (new)** — `RWSet` enum with `Concrete { r_set, w_set }` and
   `Everything` variants; borrowing `union` and `conflicts` methods; no `#[derive(Clone)]`.

4. **`src/journal.rs` (new)** — `JournalEntry` struct and `Journal` struct; borrowing
   `apply(&self, state: &mut dyn State) -> Result<(), StateError>`; consuming `discard(self)`.

5. **`src/lib.rs` (rewrite)** — all six modules declared alphabetically with flat `pub use`
   re-exports for all eight public types.

6. **`Cargo.toml` (edit)** — `alloy-consensus` added as workspace-inherited dep.

In the same commit, four other files are updated:

7. **Root `Cargo.toml` (edit)** — `alloy-consensus` added to the "Ethereum types" group.

8. **`ARCHITECTURE.md` (edit × 2)** — Step 1.1b heading `✅`, all six checkboxes `[x]`;
   Step 3.1 `lookahead` return type reconciled (`Vec<PendingTx>` → `Vec<MempoolEntry>`).

9. **`AGENTS.md` (edit × 2)** — Current State updated ("What was just completed: Step 1.1b";
   "What to do next: Step 1.2") and Changelog Session 12 appended at the bottom.

This step is **data type definitions only**. No tests, no concrete implementations, no
serialization, no RLP, no signature recovery logic, no Phase 3+ concerns. Tests come in
Step 1.2.

---

## Decisions resolved before this plan was written

All fourteen decisions below are **final**. They were made in a pre-planning session and
recorded in `docs/plans/step-1.1b-decisions.md` (the canonical source). Do not re-derive or
re-litigate any of them; cite the decisions document by number if background is needed.

| # | Topic | Resolution |
|---|---|---|
| 1 | `PendingTx` envelope type | `alloy_consensus::TxEnvelope`. Coder must verify whether it is a standalone enum or a type alias by inspecting source before writing the import. |
| 2 | `arrival_time` representation | `u64` Unix milliseconds, on `MempoolEntry` (not `PendingTx`). Phase 3 mempool must provide a deterministic source — `SystemTime::now()` violates Rule 7. |
| 3 | `PendingTx` / `MempoolEntry` split | Two types: `PendingTx { tx }` (wire) and `MempoolEntry { tx, sender, arrival_time }` (enriched). Co-located in `tx.rs`. |
| 4 | `Block` hash | No hash field, no hash method in 1.1b. Deferred to Phase 11. |
| 5 | `Block::txs` storage | `Vec<alloy_consensus::TxEnvelope>`. `state_root: B256` required at construction. No `From<Vec<MempoolEntry>>` impl. |
| 6 | `RWSet` shape | Enum from day one: `Concrete { r_set: BTreeSet<B256>, w_set: BTreeSet<B256> }` and `Everything`. |
| 7 | `union` / `conflicts` ownership | Borrowing — `fn union(&self, other: &RWSet) -> RWSet` and `fn conflicts(&self, other: &RWSet) -> bool`. No `#[derive(Clone)]` on `RWSet`. |
| 8 | `JournalEntry` shape | Named struct `{ slot: B256, old: B256, new: B256 }`. `old = B256::ZERO` for "slot was unset"; no `Option<B256>`. |
| 9 | `Journal::apply` ownership | Borrowing — `fn apply(&self, state: &mut dyn State) -> Result<(), StateError>`. |
| 10 | `Journal::discard` ownership | Consuming — `fn discard(self)`. Mirrors `Snapshot::release(self: Box<Self>)`. |
| 11 | `BTreeSet` discipline | `BTreeSet<B256>` in `RWSet::Concrete`. `Vec<JournalEntry>` in `Journal` (ordered, not a set — same slot may appear multiple times). No `HashSet`/`HashMap` anywhere in `krax-types`. |
| 12 | Cargo.toml edits | Add `alloy-consensus = { version = "1", default-features = false }` to workspace root (between `alloy-primitives` and `alloy-rpc-types`); add `alloy-consensus = { workspace = true }` to `crates/krax-types/Cargo.toml`. |
| 13 | `lib.rs` re-export structure | Flat namespace, alphabetical `pub mod` and `pub use` blocks. Six modules, eight re-exported types. |
| 14 | Scope exclusions | Comprehensive — see "What this step does NOT do" below. |

---

## Library verification checklist

All Context7 verification was performed in the pre-planning session. Results are captured in
`docs/plans/step-1.1b-decisions.md`. Do **not** re-run Context7 queries. The relevant
findings are reproduced here as a coder reference.

| Library | Version | Relevant API surface | Status |
|---|---|---|---|
| `alloy-consensus` | v1 (NEW — not yet in workspace at plan-writing time) | `alloy_consensus::TxEnvelope` — the EIP-2718 signed transaction envelope; wraps `TxLegacy`, `TxEip1559`, `TxEip2930`, `TxEip4844`, `TxEip7702`. May be a standalone enum or a type alias for `EthereumTxEnvelope<TxEip4844>` — **coder must verify by inspecting source** (see Decision 1 follow-up). Standalone crate, not via umbrella `alloy`. | ✅ Verified API shape via Context7 (`/alloy-rs/alloy` + `/alloy-rs/core`, 2026-05-09) |
| `alloy-primitives` | v1 (already in workspace) | `B256 = FixedBytes<32>` — Copy, Clone, Ord, Hash, `B256::ZERO`. `Address` — 20-byte type, same crate. Both already available via `alloy-primitives = { workspace = true }` in `crates/krax-types/Cargo.toml` from Step 1.1a. | ✅ Verified in Step 1.1a decisions document |
| `alloy-rpc-types` | v1 (already in workspace) | `alloy_rpc_types::Transaction` — definitively wrong for `PendingTx`; it adds block-context fields absent at mempool time. **Not used in 1.1b.** | ✅ Confirmed disqualified — settled in step-1.1b-decisions.md |

**Before writing any code that imports `alloy_consensus`:** query Context7 to confirm the
current import path for `TxEnvelope` (see Decision 1 coder follow-up). Write the verified
import path in a `// Per Context7` comment immediately above the import in `tx.rs` and
`block.rs`. If the actual path differs from `alloy_consensus::TxEnvelope`, surface the
discrepancy — do not silently fix it.

---

## Files to create or modify

### Ordered execution sequence

1. Create `crates/krax-types/src/tx.rs`
2. Create `crates/krax-types/src/block.rs`
3. Create `crates/krax-types/src/rwset.rs`
4. Create `crates/krax-types/src/journal.rs`
5. Rewrite `crates/krax-types/src/lib.rs`
6. Edit `crates/krax-types/Cargo.toml` — add `alloy-consensus` to `[dependencies]`
7. Edit root `Cargo.toml` — add `alloy-consensus` to `[workspace.dependencies]`
8. Edit `ARCHITECTURE.md` — Step 1.1b six checkboxes `[ ]` → `[x]` (str_replace)
9. Edit `ARCHITECTURE.md` — Step 1.1b heading `✅` (str_replace)
10. Edit `ARCHITECTURE.md` — Step 3.1 `lookahead` return type reconciliation (str_replace)
11. Edit `AGENTS.md` — Current State replacement (str_replace)
12. Edit `AGENTS.md` — Changelog Session 12 append

---

### Step 1 (create): `crates/krax-types/src/tx.rs`

New file. LF line endings, trailing newline.

**Exact content:**

```rust
//! Transaction types for the Krax mempool and worker pipeline.

// Per Context7 (alloy-consensus v1, 2026-05-09): TxEnvelope is the signed
// EIP-2718 envelope. Coder: verify whether TxEnvelope is a standalone enum or
// a type alias for EthereumTxEnvelope<TxEip4844> by inspecting alloy-consensus
// source. Both work identically at the use site. Capture the verified import
// path in a comment here — see step-1.1b-decisions.md Decision 1.
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
```

---

### Step 2 (create): `crates/krax-types/src/block.rs`

New file. LF line endings, trailing newline.

**Exact content:**

```rust
//! Block type for sealed, committed Krax blocks.

// Per Context7 (alloy-consensus v1, 2026-05-09): TxEnvelope is the canonical
// EIP-2718 wire-format signed transaction. See tx.rs for alias-vs-concrete note.
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
```

---

### Step 3 (create): `crates/krax-types/src/rwset.rs`

New file. LF line endings, trailing newline.

**Exact content:**

```rust
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
```

---

### Step 4 (create): `crates/krax-types/src/journal.rs`

New file. LF line endings, trailing newline.

**Exact content:**

```rust
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

    /// Discards this journal without applying it to state.
    ///
    /// Consumes `self` — there is no meaningful use of a journal after discard.
    /// Mirrors `Snapshot::release(self: Box<Self>)` from Step 1.1a.
    /// See step-1.1b-decisions.md Decision 10.
    pub fn discard(self) {}
}
```

---

### Step 5 (rewrite): `crates/krax-types/src/lib.rs`

Read the file first to confirm the exact current content (11 lines from Step 1.1a). Then
apply the following str_replace.

**str_replace:**

Old:
```
pub mod snapshot;
pub mod state;

pub use snapshot::Snapshot;
pub use state::{State, StateError};
```

New:
```
pub mod block;
pub mod journal;
pub mod rwset;
pub mod snapshot;
pub mod state;
pub mod tx;

pub use block::Block;
pub use journal::{Journal, JournalEntry};
pub use rwset::RWSet;
pub use snapshot::Snapshot;
pub use state::{State, StateError};
pub use tx::{MempoolEntry, PendingTx};
```

The crate-level `//!` doc comment (lines 1–5) is unchanged. Only the `pub mod` and `pub use`
blocks are replaced. The new `pub mod` ordering is alphabetical: `block`, `journal`, `rwset`,
`snapshot`, `state`, `tx`. The new `pub use` ordering mirrors it. Downstream code writes
`use krax_types::MempoolEntry` — flat namespace, no module path needed.

---

### Step 6 (edit): `crates/krax-types/Cargo.toml` — add `alloy-consensus`

Read the file first to confirm exact current whitespace (21 lines). Then apply the
str_replace below.

**str_replace:**

Old:
```toml
[dependencies]
# B256 (= FixedBytes<32>) is the slot key and value type throughout the State trait.
alloy-primitives = { workspace = true }
# Per-crate typed errors per AGENTS.md Rule 3.
thiserror        = { workspace = true }
```

New:
```toml
[dependencies]
# B256 (= FixedBytes<32>) is the slot key and value type throughout the State trait.
alloy-primitives = { workspace = true }
# TxEnvelope (EIP-2718 wire-format signed transaction) wraps PendingTx.
alloy-consensus  = { workspace = true }
# Per-crate typed errors per AGENTS.md Rule 3.
thiserror        = { workspace = true }
```

No other changes to `crates/krax-types/Cargo.toml`. No version pins in the per-crate file;
workspace inheritance only.

---

### Step 7 (edit): root `Cargo.toml` — add `alloy-consensus` to `[workspace.dependencies]`

Read the "Ethereum types" group first (currently lines 65–69 of the root `Cargo.toml`) to
confirm exact whitespace. Then apply the str_replace below.

Per Decision 12, `alloy-consensus` is inserted between `alloy-primitives` and `alloy-rpc-types`.
The existing single comment line is amended in place to cite the new addition (matches the
existing one-comment-per-group style).

**str_replace:**

Old:
```toml
# --- Ethereum types ---
# ✅ Context7 (/alloy-rs/alloy + /alloy-rs/core, 2026-05-06): version "1" confirmed.
alloy-primitives = { version = "1", default-features = false, features = ["serde"] }
alloy-rpc-types  = { version = "1", default-features = false }
alloy-sol-types  = { version = "1", default-features = false }
```

New:
```toml
# --- Ethereum types ---
# ✅ Context7 (/alloy-rs/alloy + /alloy-rs/core, 2026-05-06; alloy-consensus added 2026-05-09): version "1" confirmed for all four.
alloy-primitives = { version = "1", default-features = false, features = ["serde"] }
alloy-consensus  = { version = "1", default-features = false }
alloy-rpc-types  = { version = "1", default-features = false }
alloy-sol-types  = { version = "1", default-features = false }
```

No other changes to the root `Cargo.toml`.

---

### Step 8 (edit): `ARCHITECTURE.md` — Step 1.1b six checkboxes

**str_replace:**

Old:
```
- [ ] `crates/krax-types/src/tx.rs` — `PendingTx` struct (tx envelope + arrival time + cached sender). Tx envelope type to be confirmed via Context7 (likely `alloy_consensus::TxEnvelope` in 2026). `arrival_time` representation must be deterministic-friendly for replay.
- [ ] `crates/krax-types/src/block.rs` — `Block` struct (parent hash, height, timestamp, txs, state root). Block hash semantics (computed on demand vs stored) settled in the planning round.
- [ ] `crates/krax-types/src/rwset.rs` — `RWSet` (BTreeSet-based). Phase 4 introduces a sentinel "Everything" RWSet that conflicts with all others; the planning round decides whether to model `RWSet` as an enum from day one or defer to Phase 4. Methods: `conflicts(&self, other: &RWSet) -> bool`, `union(self, other: RWSet) -> RWSet`.
- [ ] `crates/krax-types/src/journal.rs` — `Journal` (ordered list of writes). Entry shape (named struct vs tuple, `Option<B256>` vs `B256::ZERO` for "old") settled in the planning round. Methods: `apply(&self, state: &mut dyn State)`, `discard(self)`. Depends on Step 1.1a's `State` trait.
- [ ] Add `alloy-consensus` to `crates/krax-types/Cargo.toml` if `PendingTx::tx` uses it (verify via Context7 first).
- [ ] Use `BTreeSet`/`BTreeMap` (NOT `HashSet`/`HashMap`) anywhere ordering or determinism matters.
```

New:
```
- [x] `crates/krax-types/src/tx.rs` — `PendingTx` struct (tx envelope + arrival time + cached sender). Tx envelope type to be confirmed via Context7 (likely `alloy_consensus::TxEnvelope` in 2026). `arrival_time` representation must be deterministic-friendly for replay.
- [x] `crates/krax-types/src/block.rs` — `Block` struct (parent hash, height, timestamp, txs, state root). Block hash semantics (computed on demand vs stored) settled in the planning round.
- [x] `crates/krax-types/src/rwset.rs` — `RWSet` (BTreeSet-based). Phase 4 introduces a sentinel "Everything" RWSet that conflicts with all others; the planning round decides whether to model `RWSet` as an enum from day one or defer to Phase 4. Methods: `conflicts(&self, other: &RWSet) -> bool`, `union(self, other: RWSet) -> RWSet`.
- [x] `crates/krax-types/src/journal.rs` — `Journal` (ordered list of writes). Entry shape (named struct vs tuple, `Option<B256>` vs `B256::ZERO` for "old") settled in the planning round. Methods: `apply(&self, state: &mut dyn State)`, `discard(self)`. Depends on Step 1.1a's `State` trait.
- [x] Add `alloy-consensus` to `crates/krax-types/Cargo.toml` if `PendingTx::tx` uses it (verify via Context7 first).
- [x] Use `BTreeSet`/`BTreeMap` (NOT `HashSet`/`HashMap`) anywhere ordering or determinism matters.
```

---

### Step 9 (edit): `ARCHITECTURE.md` — Step 1.1b heading `✅`

**str_replace:**

Old:
```
### Step 1.1b — Data Types (`PendingTx`, `Block`, `RWSet`, `Journal`)
```

New:
```
### Step 1.1b — Data Types (`PendingTx`, `Block`, `RWSet`, `Journal`) ✅
```

---

### Step 10 (edit): `ARCHITECTURE.md` — Step 3.1 `lookahead` return type reconciliation

With the two-type split resolved in Decision 3, workers consume `MempoolEntry` (not
`PendingTx`). The `lookahead` method in the Phase 3 spec must reflect this.

This is a **text-only** edit. No Phase 3 code is touched in Step 1.1b.

**str_replace:**

Old:
```
- [ ] `crates/krax-mempool/src/pool.rs` — `Mempool` with `add(tx) -> Result<(), MempoolError>`, `lookahead(n: usize) -> Vec<PendingTx>`, `remove(hashes: &[B256])`
```

New:
```
- [ ] `crates/krax-mempool/src/pool.rs` — `Mempool` with `add(tx: PendingTx) -> Result<(), MempoolError>`, `lookahead(n: usize) -> Vec<MempoolEntry>`, `remove(hashes: &[B256])`
```

---

### Step 11 (edit): `AGENTS.md` — Current State replacement

Replace the full body of the `## Current State` section — from the line beginning
`**Current Phase:**` through the last `**Notes:**` bullet — with the content below.
Leave the section header (`## Current State`) and its `> Rewritten by the agent…`
note line unchanged.

**Replacement content:**

```markdown
**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.1b complete; Step 1.2 next).

**What was just completed (Step 1.1b — Data Types):**
`crates/krax-types/src/tx.rs` created: `PendingTx` struct (wraps
`alloy_consensus::TxEnvelope`) and `MempoolEntry` struct (`PendingTx` + `sender: Address` +
`arrival_time: u64`). Co-located in one module per Decision 3.
`crates/krax-types/src/block.rs` created: `Block` struct (`parent_hash`, `height`,
`timestamp`, `txs: Vec<TxEnvelope>`, `state_root: B256`); `Block::new()` constructor;
no hash field or hash method (deferred to Phase 11).
`crates/krax-types/src/rwset.rs` created: `RWSet` enum (`Concrete { r_set: BTreeSet<B256>,
w_set: BTreeSet<B256> }` and `Everything`); borrowing `conflicts` and `union` methods;
no `#[derive(Clone)]`.
`crates/krax-types/src/journal.rs` created: `JournalEntry` struct (`slot`, `old`, `new:
B256`; `old = B256::ZERO` for unset), `Journal` struct (`entries: Vec<JournalEntry>`);
borrowing `apply(&self, state: &mut dyn State) -> Result<(), StateError>`; consuming
`discard(self)`.
`crates/krax-types/src/lib.rs` rewritten: six modules declared alphabetically (`block`,
`journal`, `rwset`, `snapshot`, `state`, `tx`); flat `pub use` re-exports for all eight
public types.
`crates/krax-types/Cargo.toml` updated: `alloy-consensus` added as workspace-inherited dep.
`Cargo.toml` (workspace root) updated: `alloy-consensus = { version = "1",
default-features = false }` added to "Ethereum types" group between `alloy-primitives` and
`alloy-rpc-types`.
`ARCHITECTURE.md` Step 1.1b heading ✅ and all six checkboxes marked `[x]`; Step 3.1
`lookahead` return type updated `Vec<PendingTx>` → `Vec<MempoolEntry>`.

**What Step 1.1a delivered:**
`crates/krax-types/src/state.rs`: `StateError` enum (`Released` variant,
`#[non_exhaustive]`) and `State` trait (`get`, `set`, `snapshot`, `commit`, `root`) with
`Send + Sync` supertraits and module-scope object-safety assertion.
`crates/krax-types/src/snapshot.rs`: `Snapshot` trait (`get`,
`release(self: Box<Self>)`) with `Send + Sync` supertraits and object-safety assertion.

**What Phase 0 delivered (Steps 0.1–0.9):**
- Cargo workspace with 14 members (3 binaries, 11 library crates), edition 2024, resolver 3, Rust
  toolchain pinned to 1.95.0.
- Full `bin/*` and `crates/*` directory tree with stub entrypoints and empty library stubs;
  all 14 members build cleanly from day one.
- Minimal entrypoints: `kraxd` prints a version banner; `kraxctl` serves `--help` via `clap` derive.
- Makefile with 7 targets: `build`, `test`, `test-integration`, `lint`, `run`, `fmt`, `clean`.
- `.gitignore` audited; `.env.example` with 4 `KRAX_*` variables.
- `docker-compose.yml` placeholder (no active services); `scripts/devnet-up.sh` and
  `devnet-down.sh` as no-op placeholder scripts.
- `contracts/` Foundry project (solc 0.8.24, `forge-std` v1.16.1 as a git submodule, empty
  `src/`, `test/`, `script/` directories with `.gitkeep`).
- `rustfmt.toml` and `clippy.toml`; workspace-level lint policy (`unsafe_code` deny,
  `unwrap_used` deny, pedantic warn at priority -1); all 14 per-crate `Cargo.toml` files opt in.
- `README.md` and `LICENSE` (Apache-2.0); repository and license fields updated to match.

**Known scaffolding placeholders carrying into Phase 1:**
- `kraxctl` `Commands` enum is empty — no real subcommands yet.
- `docker-compose.yml` has no active services — auxiliary services land in Phase 11+.
- `contracts/src/`, `contracts/test/`, `contracts/script/` contain only `.gitkeep` — real
  Solidity lands in Phase 12.
- `integration` feature on every crate is empty — integration tests land in Phase 1+.
- `.env.example` has 4 `KRAX_*` variables but nothing reads them — `krax-config` lands in
  Phase 1+.
- `scripts/devnet-up.sh` and `devnet-down.sh` print a placeholder message and exit 0 — real
  service management in Phase 11+.
- `tracing-subscriber` initialization deferred to a step alongside `krax-config`.

**What to do next:**
1. 🔴 **Step 1.2 — Type Tests.** Write tests for `RWSet::conflicts`, `RWSet::union`,
   `Journal::apply`, and `Journal::discard`. Follow ARCHITECTURE.md Step 1.2 exactly.

**Blockers:**
- Repository URL is a placeholder (`https://github.com/krax-labs/krax`). Replace before V1.0
  branding. Not a blocker for Phase 1 work.
- Project name not finalized. "Krax" is a working name. Search-replace before mainnet branding
  (V1.1 concern).

**Notes:**
- `kraxd` version banner uses `println!` — documented Rule 4 exception with inline comment in
  `main.rs`. All future runtime output uses `tracing`.
- `tracing-subscriber` initialization is deferred to a later step alongside `krax-config`.
- The `Commands` enum in `kraxctl` is empty until a step adds a real subcommand. No clippy warning
  fires on it under 1.95.0 (confirmed at Steps 0.4 and 0.8).
- `forge-std` is a git submodule at `v1.16.1`. New contributors must run
  `git submodule update --init` after cloning.
- Workspace lint policy: `unsafe_code` and `unwrap_used` are denied at workspace level. Call-site
  `#[allow(...)]` with a comment is required for any legitimate exception. For `unwrap_used`,
  tests are exempt via `#[allow(clippy::unwrap_used)]` at the test module or function level.
- Pedantic lints are `warn` in `[workspace.lints.clippy]` but `-D warnings` in `make lint`
  escalates them to errors. Any pedantic lint that fires on Phase 1+ code must be fixed or
  suppressed at the call site with a reason.
- `missing_docs = "warn"` enforced by `make lint`. Every public item in every crate must have a
  `///` doc comment. Binary `main.rs` files require a `//!` crate-level doc comment.
- Do NOT start any sequencer or RW-set work until the relevant Phase 1+ step specifies it.
- Every external library use MUST be Context7-verified per the Library Verification Protocol
  section. No exceptions.
- `reth-*` git rev must be updated periodically as reth main advances. When upgrading, change ALL
  `reth-*` entries to the same new rev in one commit.
- `Snapshot::release` signature is `release(self: Box<Self>)` — consuming. Post-release reads are
  a compile-time error ("borrow of moved value"), not a runtime `StateError::Released`. Step 1.4
  must use `trybuild` or a `compile_fail` doctest for the "after release" test case.
- `MempoolEntry::arrival_time` is `u64` Unix milliseconds. The Phase 3 mempool plan MUST specify
  a deterministic source — `SystemTime::now()` at insertion violates AGENTS.md Rule 7 because
  two sequencers stamping independently would produce different blocks from the same tx stream.
  The type is set here; the policy lands in Phase 3 (settled in step-1.1b-decisions.md Decision 2).
- `RWSet` deliberately does not `#[derive(Clone)]` — all in-tree call sites in 1.1b use borrowing
  `union` and `conflicts`. Derive `Clone` when a real call site needs it.
```

---

### Step 12 (edit): `AGENTS.md` — Changelog append

Append the following entry at the **bottom** of the `## Changelog` section, after the
Session 11 entry. Do not modify any existing entry.

```markdown

### Session 12 — Step 1.1b: Data Types
**Date:** 2026-05-10
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary:** Created `crates/krax-types/src/tx.rs` (`PendingTx` wrapping
`alloy_consensus::TxEnvelope`; `MempoolEntry` with `sender: Address` and `arrival_time:
u64`; co-located per Decision 3). Created `crates/krax-types/src/block.rs` (`Block` with
five fields; `Block::new()` constructor; no hash field per Decision 4). Created
`crates/krax-types/src/rwset.rs` (`RWSet` enum with `Concrete { r_set, w_set }` and
`Everything`; borrowing `conflicts` and `union`; no `Clone` per Decision 7). Created
`crates/krax-types/src/journal.rs` (`JournalEntry { slot, old, new: B256 }`; `Journal
{ entries: Vec<JournalEntry> }`; borrowing `apply`; consuming `discard`). Rewrote
`crates/krax-types/src/lib.rs` with six alphabetical `pub mod` declarations and eight
flat `pub use` re-exports. Added `alloy-consensus = { workspace = true }` to
`crates/krax-types/Cargo.toml`. Added `alloy-consensus = { version = "1",
default-features = false }` to workspace root "Ethereum types" group. Updated
`ARCHITECTURE.md`: Step 1.1b heading ✅, all six checkboxes `[x]`, Step 3.1 `lookahead`
return type `Vec<PendingTx>` → `Vec<MempoolEntry>`. All fourteen decisions settled in
`docs/plans/step-1.1b-decisions.md`.
**Commit suggestion:** `feat(types): define PendingTx, Block, RWSet, Journal — Step 1.1b`
```

---

## Verification steps

Run in order from the project root. Every command must exit 0 (or produce no output for
grep checks) before the step is considered done.

```bash
# 1. Build — confirms alloy-consensus resolves, new types compile, no import errors.
make build
# Expected: exits 0.

# 2. Lint — confirms no pedantic violations, missing_docs, unwrap_used, or
#            HashMap/HashSet in new code.
make lint
# Expected: exits 0 with -D warnings.

# 3. Test — no new test code in this step; must not regress existing (zero) tests.
make test
# Expected: exits 0.

# 4. Docs — confirms every public item in the four new files has a doc comment.
cargo doc --workspace --no-deps
# Expected: exits 0. If missing_docs fires for any new public item, add or fix
#           the doc comment before proceeding.

# 5. Defensive HashMap/HashSet check — krax-types must not use HashMap or HashSet.
grep -E '(HashSet|HashMap)' crates/krax-types/src/*.rs
# Expected: no output (grep exits 1 = no match = pass). Any match is a violation.

# 6. New files exist.
test -f crates/krax-types/src/tx.rs      && echo "OK: tx.rs"
test -f crates/krax-types/src/block.rs   && echo "OK: block.rs"
test -f crates/krax-types/src/rwset.rs   && echo "OK: rwset.rs"
test -f crates/krax-types/src/journal.rs && echo "OK: journal.rs"
# Expected: four "OK:" lines.

# 7. ARCHITECTURE.md Step 1.1b edits verified.
grep "Step 1.1b.*✅"                               ARCHITECTURE.md && echo "OK: Step 1.1b ✅"
grep "\[x\].*tx\.rs"                               ARCHITECTURE.md && echo "OK: tx.rs checkbox"
grep "\[x\].*block\.rs"                            ARCHITECTURE.md && echo "OK: block.rs checkbox"
grep "\[x\].*rwset\.rs"                            ARCHITECTURE.md && echo "OK: rwset.rs checkbox"
grep "\[x\].*journal\.rs"                          ARCHITECTURE.md && echo "OK: journal.rs checkbox"
grep "\[x\].*alloy-consensus.*Cargo"               ARCHITECTURE.md && echo "OK: alloy-consensus checkbox"
grep "\[x\].*BTreeSet"                             ARCHITECTURE.md && echo "OK: BTreeSet checkbox"
# Expected: seven "OK:" lines.

# 8. ARCHITECTURE.md Step 3.1 reconciliation.
grep "lookahead(n: usize) -> Vec<MempoolEntry>"    ARCHITECTURE.md && echo "OK: Step 3.1 updated"
# Expected: one "OK:" line.

# 9. AGENTS.md updated.
grep "Step 1.1b complete"   AGENTS.md && echo "OK: Current State references Step 1.1b"
grep "Step 1.2"             AGENTS.md && echo "OK: Current State names next step"
grep "Session 12"           AGENTS.md && echo "OK: Changelog Session 12 present"
# Expected: three "OK:" lines.

# 10. krax-types/Cargo.toml deps.
grep "alloy-primitives"  crates/krax-types/Cargo.toml && echo "OK: alloy-primitives"
grep "alloy-consensus"   crates/krax-types/Cargo.toml && echo "OK: alloy-consensus"
grep "thiserror"         crates/krax-types/Cargo.toml && echo "OK: thiserror"
# Expected: three "OK:" lines.

# 11. Workspace root Cargo.toml has alloy-consensus in workspace.dependencies.
grep 'alloy-consensus.*version.*"1"'  Cargo.toml && echo "OK: workspace alloy-consensus"
# Expected: one "OK:" line.

# 12. lib.rs pub mods and pub uses present.
grep "pub mod block"     crates/krax-types/src/lib.rs && echo "OK: pub mod block"
grep "pub mod journal"   crates/krax-types/src/lib.rs && echo "OK: pub mod journal"
grep "pub mod rwset"     crates/krax-types/src/lib.rs && echo "OK: pub mod rwset"
grep "pub mod tx"        crates/krax-types/src/lib.rs && echo "OK: pub mod tx"
grep "pub use block"     crates/krax-types/src/lib.rs && echo "OK: pub use block"
grep "pub use journal"   crates/krax-types/src/lib.rs && echo "OK: pub use journal"
grep "pub use rwset"     crates/krax-types/src/lib.rs && echo "OK: pub use rwset"
grep "pub use tx"        crates/krax-types/src/lib.rs && echo "OK: pub use tx"
# Expected: eight "OK:" lines.
```

---

## Definition of "Step 1.1b done"

- ✅ `crates/krax-types/src/tx.rs` exists; contains `PendingTx { tx: TxEnvelope }` and
  `MempoolEntry { tx: PendingTx, sender: Address, arrival_time: u64 }`.
- ✅ `crates/krax-types/src/block.rs` exists; contains `Block` with five fields and `Block::new`
  constructor; no hash field, no hash method, no `From` impl.
- ✅ `crates/krax-types/src/rwset.rs` exists; contains `RWSet` enum with `Concrete { r_set:
  BTreeSet<B256>, w_set: BTreeSet<B256> }` and `Everything` variants; `conflicts(&self, other:
  &RWSet) -> bool`; `union(&self, other: &RWSet) -> RWSet`; no `#[derive(Clone)]`.
- ✅ `crates/krax-types/src/journal.rs` exists; contains `JournalEntry { slot, old, new: B256 }`;
  `Journal { entries: Vec<JournalEntry> }`; `apply(&self, state: &mut dyn State) -> Result<(),
  StateError>`; `discard(self)`.
- ✅ `crates/krax-types/src/lib.rs` contains six alphabetical `pub mod` declarations and eight
  flat `pub use` re-exports.
- ✅ `crates/krax-types/Cargo.toml` `[dependencies]` block contains `alloy-consensus = {
  workspace = true }`.
- ✅ Root `Cargo.toml` `[workspace.dependencies]` "Ethereum types" group contains `alloy-consensus
  = { version = "1", default-features = false }` between `alloy-primitives` and `alloy-rpc-types`.
- ✅ `make build` exits 0.
- ✅ `make lint` exits 0 — no missing docs, no pedantic violations, no `HashMap`/`HashSet`.
- ✅ `make test` exits 0.
- ✅ `cargo doc --workspace --no-deps` exits 0.
- ✅ `grep -E '(HashSet|HashMap)' crates/krax-types/src/*.rs` produces no output.
- ✅ `ARCHITECTURE.md` Step 1.1b heading has `✅`; all six checkboxes `[x]`; Step 3.1 `lookahead`
  signature reads `Vec<MempoolEntry>`.
- ✅ `AGENTS.md` Current State reflects Step 1.1b complete and Step 1.2 as next; Changelog has
  Session 12 as the last entry.

---

## Open questions / coder follow-ups

**Decision 1 follow-up — verify `TxEnvelope` alias-vs-concrete before writing imports:**
Before writing `use alloy_consensus::TxEnvelope;`, query Context7 for `alloy-consensus` and
inspect the source to determine whether `TxEnvelope` is a standalone enum or a type alias
(likely `type TxEnvelope = EthereumTxEnvelope<TxEip4844>`). Both resolve identically at the
call site; the alias form just passes through. Capture the verified import form in a
`// Per Context7` comment immediately above the `use` statement in both `tx.rs` and `block.rs`.
If the actual path differs from `alloy_consensus::TxEnvelope`, stop and surface the discrepancy
to the maintainer — do not silently adjust.

**Decision 2 informational — `arrival_time` deterministic source (Phase 3 concern):**
`MempoolEntry::arrival_time` is `u64` Unix milliseconds. The type is correct and complete;
this is not a 1.1b code concern. However, Phase 3's mempool plan must specify a deterministic
source. Using `SystemTime::now()` at the mempool layer would violate AGENTS.md Rule 7 because
two sequencers processing the same transaction stream independently would produce different
`arrival_time` values, breaking deterministic block production. Acceptable options include:
a monotonic per-block sequence number, a synced clock reading captured from L1 calldata, or
a value received from the transaction submitter. Phase 3's planner is responsible for this
decision. This note is recorded here so it is not lost between sessions.

**If `make lint` reports a pedantic warning on `Journal::discard`:**
A lint such as `clippy::unused_self` will not fire because `self` is consumed (moved in and
dropped). If an unexpected warning fires on the empty body `{}`, investigate before suppressing
— it should not require `todo!()` or any placeholder.

**If `make lint` reports a warning about missing `Debug` impls:**
`clippy::missing_debug_implementations` is in the `clippy::restriction` group, not `pedantic`.
It is not enabled in the workspace lint policy and should not fire. If it does fire, the
workspace policy needs to be audited — do not derive `Debug` speculatively.

**If `clippy::exhaustive_structs` or `clippy::exhaustive_enums` fires:**
These are in `clippy::restriction`, not enabled. If they fire, audit the workspace policy.
Do not add `#[non_exhaustive]` to data types without a plan-level decision.

**If Step 1.2 tests require `Debug` or `PartialEq` on 1.1b types:**
Step 1.2 is a separate plan. That plan should derive `Debug` and `PartialEq` on the types it
needs to test. Do NOT add these derives in 1.1b — that is speculative scope growth.

---

## What this step does NOT do

- ❌ No tests — Step 1.2 writes tests for `RWSet::conflicts`, `RWSet::union`, `Journal::apply`,
  and `Journal::discard`.
- ❌ No `Inferer` trait — Phase 4, Step 4.2.
- ❌ No `Worker` struct — Phase 5.
- ❌ No concrete `MptState` or any `State` implementation — Phase 1, Step 1.3.
- ❌ No `#[derive(Serialize, Deserialize)]` on any Krax type — Phase 11+.
- ❌ No RLP encoding or block hash computation — Phase 11.
- ❌ No `From<Vec<MempoolEntry>> for Block` impl — Phase 5–6 commit phase responsibility.
- ❌ No signature recovery logic — `MempoolEntry` is a data struct; how `sender` is populated
  is a Phase 3 mempool concern. Step 1.2 test code will use stub addresses.
- ❌ No `#[derive(Clone)]` on `RWSet` — per Decision 7.
- ❌ No `alloy_rpc_types::Transaction` usage — definitively wrong for mempool-stage types.
- ❌ No `alloy-eips`, `alloy-sol-types`, or other new alloy sub-crates beyond `alloy-consensus`.
- ❌ No gas accounting types — Phase 2.
- ❌ No changes outside `crates/krax-types/`, workspace `Cargo.toml`, `ARCHITECTURE.md`,
  and `AGENTS.md`.
- ❌ No new `src/*.rs` files except `tx.rs`, `block.rs`, `rwset.rs`, `journal.rs`.

---

## Updates to other files in the same commit

All changes below land in the **same commit** as the four new `.rs` files.

| File | Change |
|---|---|
| `crates/krax-types/src/lib.rs` | Rewrite: six alphabetical `pub mod` + eight flat `pub use` re-exports |
| `crates/krax-types/Cargo.toml` | `[dependencies]`: add `alloy-consensus = { workspace = true }` between `alloy-primitives` and `thiserror` |
| `Cargo.toml` (workspace root) | `[workspace.dependencies]` "Ethereum types" group: add `alloy-consensus = { version = "1", default-features = false }` between `alloy-primitives` and `alloy-rpc-types`; amend comment to cite the addition |
| `ARCHITECTURE.md` | Step 1.1b: all six `[ ]` → `[x]`; heading `✅` |
| `ARCHITECTURE.md` | Step 3.1: `lookahead` return type `Vec<PendingTx>` → `Vec<MempoolEntry>` |
| `AGENTS.md` | Current State: full replacement reflecting Step 1.1b complete, Step 1.2 next |
| `AGENTS.md` | Changelog: Session 12 appended at the bottom |

---

## Commit suggestion

```
feat(types): define PendingTx, Block, RWSet, Journal — Step 1.1b

crates/krax-types/src/tx.rs (new):
- PendingTx: newtype wrapper around alloy_consensus::TxEnvelope (wire format).
- MempoolEntry: PendingTx + sender: Address + arrival_time: u64 (enriched;
  constructed by Phase 3 mempool only).
- Co-located per Decision 3; see step-1.1b-decisions.md.

crates/krax-types/src/block.rs (new):
- Block: parent_hash, height, timestamp, txs: Vec<TxEnvelope>, state_root: B256.
- Block::new() constructor; sealed-block invariant via required state_root.
- No hash field/method (deferred to Phase 11, Decision 4).

crates/krax-types/src/rwset.rs (new):
- RWSet enum: Concrete { r_set, w_set: BTreeSet<B256> } and Everything sentinel.
- conflicts(&self, other: &RWSet) -> bool; union(&self, other: &RWSet) -> RWSet.
- No #[derive(Clone)] — borrowing semantics remove all clone call sites (Decision 7).
- Everything variant shipped now to avoid Phase 4 breaking change (Decision 6).

crates/krax-types/src/journal.rs (new):
- JournalEntry { slot, old, new: B256 }; old = B256::ZERO for unset (Decision 8).
- Journal { entries: Vec<JournalEntry> }.
- apply(&self, state: &mut dyn State) -> Result<(), StateError> (borrowing, Decision 9).
- discard(self) consuming, mirrors Snapshot::release (Decision 10).

crates/krax-types/src/lib.rs (rewrite):
- Six alphabetical pub mod declarations; eight flat pub use re-exports.

crates/krax-types/Cargo.toml:
- alloy-consensus added as workspace-inherited dep.

Cargo.toml (workspace root):
- alloy-consensus = { version = "1", default-features = false } added to Ethereum
  types group; comment amended to cite the addition.

ARCHITECTURE.md:
- Step 1.1b: heading ✅, all six checkboxes [x].
- Step 3.1: lookahead return type Vec<PendingTx> → Vec<MempoolEntry> (cross-step
  reconciliation per step-1.1b-decisions.md cross-step impact note).

AGENTS.md:
- Current State: Step 1.1b complete; Step 1.2 next. arrival_time and RWSet Clone
  notes added.
- Changelog: Session 12 appended at the bottom.

All fourteen decisions settled in docs/plans/step-1.1b-decisions.md.
```

---

## Outcomes

**Execution date:** 2026-05-10
**Agent:** Claude Code (claude-sonnet-4-6)

**File line counts:**
- `tx.rs`: 46 lines
- `block.rs`: 50 lines
- `rwset.rs`: 78 lines (81 after `#[must_use]` fix — see deviation below)
- `journal.rs`: 62 lines
- Total new code: 236 lines

**Decision 1 alias-vs-concrete finding:**
`alloy_consensus::TxEnvelope` is a **type alias** for the generic `EthereumTxEnvelope<TxEip4844>`.
The concrete generic type is `EthereumTxEnvelope`; `TxEnvelope` is the canonical alias exposed
at the `alloy_consensus` module level. Import path `alloy_consensus::TxEnvelope` is confirmed
valid and identical at the use site regardless of alias-vs-concrete distinction. Context7
evidence: the EIP-7594 example in `/websites/alloy_rs` imports `alloy::consensus::EthereumTxEnvelope`
directly for a generic specialization (`EthereumTxEnvelope<TxEip4844WithSidecar<...>>`), while
the standard import remains `alloy_consensus::TxEnvelope`. No discrepancy from plan assumption —
no stop required.

**Deviations from plan:**
1. `#[must_use]` added to `RWSet::union` (line before `pub fn union`). The IDE flagged
   `clippy::return_self_not_must_use` (pedantic group) — this lint fires when a `&self` method
   returns `Self`. The workspace policy suppresses `must_use_candidate` but not
   `return_self_not_must_use`. Since `make lint` escalates pedantic warnings to errors via
   `-D warnings`, the attribute is required. The plan's verbatim content did not include it.
   This is a minimal, non-functional deviation; the `#[must_use]` has no runtime effect.

**Unexpected findings:**
- `alloy-consensus` resolved to version `1.8.3` on crates.io (latest in the `1` range). Build
  succeeded immediately with no version conflicts.
- The `ignore` doctest in `RWSet::union` was correctly treated as ignored (not compiled) and
  appeared in `make test` output as "1 ignored" — expected behavior.

**Verification results:** All 12 verification command groups passed on first attempt after the
`#[must_use]` fix. `make build` exit 0. `make lint` exit 0. `make test` exit 0.
`cargo doc --workspace --no-deps` exit 0. All grep checks produced expected output.

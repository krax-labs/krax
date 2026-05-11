# Step 1.1b — Decision Surface

> Pre-plan decision-surface document produced by the strategic-guide agent (decision-surfacer recommendations + maintainer amendments).
> This is the input to `step-1.1b-data-types.md`. All decisions below are final.
> Archived alongside the plan file when Step 1.1b ships.

---

## Library Verification Results (Context7, 2026-05-09)

### `alloy-consensus` — v1 (NEW — not yet in workspace)

Source: Context7 (alloy crate family).

- `alloy_consensus::TxEnvelope` is the correct signed transaction envelope type for standard Ethereum in alloy 1.x. It wraps the EIP-2718 typed transaction variants: `Signed<TxLegacy>`, `Signed<TxEip1559>`, `Signed<TxEip2930>`, `Signed<TxEip4844>`, `Signed<TxEip7702>`.
- The alloy docs also reference an `EthereumTxEnvelope<T>` generic. `TxEnvelope` appears to be a concrete type alias (likely `type TxEnvelope = EthereumTxEnvelope<TxEip4844>` or a fixed enum). The Context7 query did not return the exact alias definition — **coder must verify by inspecting `alloy-consensus` source before writing imports.** Both shapes work identically at the use site; the alias form just resolves through.
- `alloy_consensus` scope per its own README: *"A type is generally included in this crate if it is committed to within the EL block header. This scope encompasses transactions, blocks, headers, receipts, and EIP-2718 envelopes."* This is exactly the wire-format scope Step 1.1b's `PendingTx` needs.
- Imported as the standalone `alloy-consensus = "1"` crate, or as `alloy::consensus::*` via the umbrella. Krax workspace pattern is per-crate (`alloy-primitives`, `alloy-rpc-types`, `alloy-sol-types`), so the standalone form fits.

**Cargo impact — critical:** `alloy-consensus` is NOT in `[workspace.dependencies]`. Both the workspace root `Cargo.toml` AND `crates/krax-types/Cargo.toml` need edits this step. (See Decision 12.)

### `alloy-primitives` — v1 (already in workspace, verified in 1.1a)

`Address` is `alloy_primitives::Address` (a 20-byte type). Used for `MempoolEntry::sender`. Already inherited via `alloy-primitives = { workspace = true }` in `crates/krax-types/Cargo.toml` from Step 1.1a — no additional dep changes for primitives in 1.1b.

`B256` continues to be the storage-slot key/value type as established in 1.1a. Used by `RWSet` (BTreeSet members) and `JournalEntry` (slot, old, new fields).

### `alloy-rpc-types` — v1 (already in workspace, NOT used in 1.1b)

`alloy_rpc_types::Transaction` is the JSON-RPC response type — it includes block number, transaction index, and other block-context fields that don't exist in the mempool. **Definitively wrong for Step 1.1b's wire-format type.** No new use in 1.1b. Continues to be the right choice in Phase 10 (JSON-RPC gateway).

### `serde` — already in workspace, NOT used in 1.1b

`serde` is in workspace deps and `alloy-consensus` types have serde support. Krax's own structs (`PendingTx`, `MempoolEntry`, `Block`, `RWSet`, `Journal`, `JournalEntry`) do NOT need `#[derive(Serialize, Deserialize)]` in 1.1b. Adding serde derives now is speculative dependency growth (violates AGENTS.md Rule 10). Defer to the phase that actually needs serialization — likely Phase 11 (block storage).

### `thiserror` — v2.0.18 (already in workspace, NOT used in 1.1b)

`thiserror` is inherited by `krax-types` from 1.1a but is not used by any of 1.1b's types. The data types in 1.1b are infallible at construction (no `Result` returns). Kept in `[dependencies]` from 1.1a; no new error types introduced this step.

### Workspace root — what's there, what's needed

Already present in `[workspace.dependencies]`:
- `alloy-primitives = { version = "1", default-features = false, features = ["serde"] }` ✅
- `alloy-rpc-types = { version = "1", default-features = false }` ✅
- `alloy-sol-types = { version = "1", default-features = false }` ✅
- `thiserror = "2"` ✅
- `serde = { version = "1", features = ["derive"] }` ✅

Missing and required:
- `alloy-consensus = { version = "1", default-features = false }` — must be added in this commit.

---

## Decisions (final, after amendments)

### Decision 1 — `PendingTx` envelope type

**Resolved: `alloy_consensus::TxEnvelope`.**

The wire-format signed transaction envelope. Disqualified alternative: `alloy_rpc_types::Transaction` (adds block-context fields that don't exist at mempool time).

**Coder follow-up:** verify whether `TxEnvelope` is a standalone enum or a type alias for `EthereumTxEnvelope<TxEip4844>` by inspecting `alloy-consensus` source. Both shapes work at the use site; the alias form just resolves through. Capture the verified import path in a comment alongside the import.

### Decision 2 — `arrival_time` representation (AMENDED)

**Resolved: `u64` Unix milliseconds.** Lives on `MempoolEntry`, not `PendingTx` (per Decision 3's split).

**Amendment from surfacer's recommendation:** the surfacer correctly identified `u64` as the right type but argued that `SystemTime::now()` at the mempool layer is fine because "the mempool stamping `arrival_time` at insertion is not state-affecting code." This is wrong.

Per `AGENTS.md` and `ARCHITECTURE.md` Step 3.1, the Phase 3 mempool orders by gas price descending, **then by arrival time** as the tiebreaker. That makes `arrival_time` consensus-relevant — two sequencers stamping `SystemTime::now()` independently would produce different blocks from the same transaction stream, violating Rule 7 (determinism).

The type stays `u64`. The cross-step impact note (below) calls out that Phase 3's mempool plan must specify a deterministic source for this value (e.g. monotonic per-block sequence number, or a synced clock reading captured into L1 calldata for replay). `SystemTime::now()` at the mempool layer is NOT a sufficient source.

This is the same anti-deferral pattern as 1.1a's Decision 1: the type lands now in 1.1b; the discipline that makes it correct lands in Phase 3.

### Decision 3 — `PendingTx` and `MempoolEntry` — two types, not one (AMENDED)

**Resolved: split `PendingTx` (wire) and `MempoolEntry` (enriched).**

The surfacer's original recommendation was a single `PendingTx { tx, arrival_time, sender: Option<Address> }`. **Amended at maintainer's direction.**

`PendingTx` should model what's on the wire. The wire doesn't carry a sender — it carries a signature you recover the sender from. Putting `sender` (even as `Option<Address>`) on `PendingTx` itself conflates two different things: the transaction as transmitted, and the mempool's enriched view of it. That conflation makes `PendingTx` mean "post-mempool transaction" everywhere it's used — RPC ingress, P2P propagation, fuzz harnesses, replay tools, and the V2 fault prover all become places where someone has to either re-recover the sender (wasteful) or fabricate one (wrong).

Splitting into two types means the type each layer holds matches what that layer actually knows:

```rust
/// A signed Ethereum transaction as received on the wire. Mirrors the
/// EIP-2718 envelope; carries no mempool or block context.
pub struct PendingTx {
    pub tx: alloy_consensus::TxEnvelope,
}

/// A transaction enriched by the mempool with the recovered sender and
/// arrival time. Constructed only by the mempool's validation step
/// (Phase 3); workers and the commit phase consume this type, not PendingTx.
pub struct MempoolEntry {
    pub tx: PendingTx,
    pub sender: Address,
    pub arrival_time: u64,
}
```

**Performance and correctness benefits:**
- Signature recovery happens exactly once, at mempool insertion. The recovered value travels with the transaction by construction — no `Option` to unwrap, no re-recovery, no "is this populated yet" checks at the hot path.
- V2's prover can consume `PendingTx` directly without dragging mempool-only metadata into the proof circuit.
- Layers without a mempool (RPC ingress, P2P, fuzz harnesses) hold `PendingTx` and never have to reason about `sender` at all.

**Module location (settled, not coder choice):** Both types live in `crates/krax-types/src/tx.rs`. Co-locating is appropriate given the tight coupling (`MempoolEntry` wraps `PendingTx`); they're imported together at consumer sites. Single module file ~50–80 lines is cleaner than two files of 30–40 lines each. No new `mempool_entry.rs` module.

**`PendingTx` is a wrapper struct, not a re-export.** Whether to keep the wrapper or collapse to `pub use alloy_consensus::TxEnvelope as PendingTx` is settled in favor of the wrapper. The wrapper costs nothing today but gives a place to hang Krax-specific methods later without modifying upstream alloy types.

### Decision 4 — `Block` hash semantics

**Resolved: no hash field, no hash method in 1.1b.**

The block hash is `keccak(RLP(header))`. Krax has no RLP encoding infrastructure in Phase 1. Adding a hash field now either (a) requires RLP logic that hasn't been planned yet, or (b) leaves the field always `B256::ZERO` — a misleading invariant. Adding a hash method has the same problem.

Phase 11 (Block Production & Internal Storage) is where block hashing becomes load-bearing. That phase adds the hash computation method and/or a cached hash field. For 1.1b, `Block` is a plain data struct with no hash-related surface. `parent_hash: B256` (per ARCHITECTURE.md Step 1.1b spec) satisfies all near-term uses.

This is genuine scope discipline, not deferral — the hash cannot be implemented without infrastructure that hasn't been planned yet.

### Decision 5 — `Block::txs` storage type

**Resolved: `Vec<alloy_consensus::TxEnvelope>` for the committed `Block` type. `state_root: B256` required (not `Option<B256>`).**

The committed block is the canonical artifact; it strips mempool decoration. `Vec<MempoolEntry>` would carry mempool-only metadata (`sender`, `arrival_time`) into a sealed structure where it's meaningless. `Vec<PendingTx>` would carry the wrapper struct unnecessarily — committed blocks need only the wire-format envelope.

The Phase 5–6 commit phase converts `Vec<MempoolEntry>` → `Vec<TxEnvelope>` by extracting `entry.tx.tx` from each entry. Tx hash is recoverable from the envelope itself.

**`state_root: B256` required at construction:** `Block` represents a sealed, committed block. An in-progress block is the sequencer's local state (a `Vec<MempoolEntry>` being processed), not a `Block`. Requiring `state_root` at construction enforces this invariant via the type system without a runtime check.

**Out-of-scope for 1.1b: no `From<Vec<MempoolEntry>> for Block` impl.** The conversion is the Phase 5–6 commit phase's responsibility. `Block` exposes only `Block::new(parent_hash, height, timestamp, txs, state_root)`. The coder must not add a convenience `From` impl — it would be premature.

### Decision 6 — `RWSet` as enum from day one

**Resolved: enum from day one, with `Concrete { r_set, w_set }` and `Everything` variants.**

```rust
pub enum RWSet {
    /// Concrete read and write sets inferred or measured for a transaction.
    Concrete {
        r_set: BTreeSet<B256>,
        w_set: BTreeSet<B256>,
    },
    /// Conservative sentinel: conflicts with all other RW-sets.
    ///
    /// Returned by the conservative inferer (Phase 4) when the transaction's
    /// access pattern cannot be statically determined.
    Everything,
}
```

**The anti-deferral case is stronger here than in 1.1a's Decisions 6 and 7:**

1. **The refactor IS planned.** ARCHITECTURE.md Step 4.1 says explicitly: *"Define the 'everything' sentinel in `crates/krax-types/src/rwset.rs` (a dedicated `RWSet::Everything` variant or a flag)."* Doing it now vs. in Phase 4 is purely timing.
2. **Blast radius at Phase 4 is real.** By Phase 4, `RWSet` is touched by tests (Step 1.2), the `Inferer` trait (Step 4.2), workers (Phase 5), and the conflict detector (Phase 6). Refactoring a public struct in `krax-types` to an enum at that point breaks every match/destructure across those files.
3. **`conflicts` semantics are fundamentally enum-shaped.** `Everything.conflicts(anything) == true`. With a struct, expressing this requires a hidden `bool` flag or a special sentinel value — an ad-hoc enum without type safety. The correct abstraction is an enum.
4. **YAGNI does not apply.** YAGNI means "don't build what you don't know you'll need." We know we'll need `Everything` — it's in the architecture plan.

Construction-site verbosity is one extra `RWSet::Concrete { ... }` per construction site. Step 1.2 tests will write a constructor helper once and use it everywhere.

### Decision 7 — `RWSet::union` and `conflicts` ownership

**Resolved: borrowing for both — `fn union(&self, other: &RWSet) -> RWSet` and `fn conflicts(&self, other: &RWSet) -> bool`.**

The Phase 6 commit phase accumulates cumulative write-sets:

```rust
let mut cumulative = RWSet::Concrete {
    r_set: BTreeSet::new(),
    w_set: BTreeSet::new(),
};
for result in committed_results.iter() {
    cumulative = cumulative.union(&result.rwset); // borrowing: no clone needed
}
if later_tx.rwset.conflicts(&cumulative) { /* ... */ }
```

With consuming `union`, this pattern requires cloning `cumulative` before each call or restructuring into a fold that moves ownership. Neither is natural.

The `Everything` variant also makes consuming semantics odd: `Everything.union(other)` would consume `other` for no reason — the result is just `Everything`.

`conflicts` is symmetrically borrowing for the same reason: checking intersection doesn't require ownership of either side.

**Sub-decision: do NOT `#[derive(Clone)]` on `RWSet`.** With borrowing semantics there are no clone call sites in 1.1b. Future call sites that genuinely need cloning can derive `Clone` then. Don't add it speculatively.

### Decision 8 — `JournalEntry` shape

**Resolved: named struct with `B256::ZERO` for "old" (not `Option<B256>`).**

```rust
pub struct JournalEntry {
    /// Storage slot written.
    pub slot: B256,
    /// Value of the slot before this write; B256::ZERO if the slot was unset.
    pub old: B256,
    /// Value written to the slot.
    pub new: B256,
}

pub struct Journal {
    pub entries: Vec<JournalEntry>,
}
```

**Named struct vs tuple:** named struct unambiguous at the call site (`entry.slot`, `entry.old`, `entry.new` vs `entry.0`, `entry.1`, `entry.2`). The conflict detector and `apply` logic will pattern-match on these — named fields matter. Not controversial.

**`B256::ZERO` vs `Option<B256>` for `old`:**
- The EVM has no concept of a "nonexistent" storage slot. Every slot defaults to `B256::ZERO`. SLOAD on an unset slot returns `B256::ZERO`.
- The Journal's `discard` operation restores `state.set(slot, old_value)`. If `old_value` is `B256::ZERO`, that means "restore to the EVM's default" — which is exactly `B256::ZERO`. No special-casing needed.
- Using `Option<B256>` means every `apply` and `discard` call would handle the `None` case with `state.set(slot, B256::ZERO)` anyway — just unwrapping to `B256::ZERO`. No semantic benefit.
- The only scenario where `Option<B256>` would add something is if you wanted to DELETE a slot (vs set it to ZERO). The EVM storage model has no "delete" — zero and absent are identical from SLOAD's perspective.

**EIP-2200 gas refund note:** SSTORE refund logic distinguishes "original value" (at tx start) from "current value." `JournalEntry::old` captures the value before this specific write within the tx — not necessarily the tx-start value. For refund purposes, Phase 2's EVM executor tracks this separately via revm's own journal (revm handles SSTORE gas refunds internally). Krax's `JournalEntry` doesn't need to replicate that — it just needs to know what to restore on `discard`.

### Decision 9 — `Journal::apply` ownership — borrowing

**Resolved: borrowing — `fn apply(&self, state: &mut dyn State) -> Result<(), StateError>`.**

**Why borrowing, not consuming:**

1. **Inspect-after-apply use cases are real.** Phase 6's `CommitReport` may need to inspect journal entries after applying them — counting slots written, summing gas, debug logging. Consuming would require extracting that data first.
2. **Testing.** A test can apply a journal to a test state and inspect the journal afterwards without consuming.
3. **The "don't apply twice" bug is logic, not corruption.** Applying a journal twice is **idempotent at the EVM-state level** — `state.set(slot, val)` twice is the same as once. The bug the type system would catch by consuming `apply` is "applied twice when you meant once" — a logic bug, not a state-corruption bug. Cost of catching that with the type system is losing the inspect-after-apply use cases. Trade favors borrowing.

**Returns `Result<(), StateError>`** because `state.set(...)` is fallible (per 1.1a's Decision 7).

### Decision 10 — `Journal::discard` ownership — consuming

**Resolved: consuming — `fn discard(self)`.**

`discard` is semantically "destroy this journal without applying it." Consuming is the correct model — there's no sensible use of a journal after discard (unlike `apply`, where post-application inspection has real use cases). Type system enforcing this is cheap and correct.

Mirrors `Snapshot::release(self: Box<Self>)` from 1.1a — both are one-way destruction operations.

### Decision 11 — `BTreeSet` discipline checkbox

**Resolved: confirmed for 1.1b.**

- `RWSet::Concrete` uses `BTreeSet<B256>` for both `r_set` and `w_set`.
- `Journal::entries` is `Vec<JournalEntry>` — ordered by write order within a tx, not a set, because the same slot can be written multiple times in one tx with the last write winning.
- No `HashSet` or `HashMap` anywhere in `krax-types`.

This closes the open BTreeSet discipline checkbox from ARCHITECTURE.md Step 1.1b.

### Decision 12 — `Cargo.toml` edits (workspace root + per-crate)

**Resolved: two-level edits.**

#### Workspace root `Cargo.toml`

Add `alloy-consensus` to the "Ethereum types" group, alphabetically between `alloy-primitives` and `alloy-rpc-types`. Amend the existing single comment line to cover the addition (matches the existing style of one comment per group, rather than per dep):

```toml
# --- Ethereum types ---
# ✅ Context7 (/alloy-rs/alloy + /alloy-rs/core, 2026-05-06; alloy-consensus added 2026-05-09): version "1" confirmed for all four.
alloy-primitives = { version = "1", default-features = false, features = ["serde"] }
alloy-consensus  = { version = "1", default-features = false }
alloy-rpc-types  = { version = "1", default-features = false }
alloy-sol-types  = { version = "1", default-features = false }
```

The existing comment is amended in place to cite the new addition. This is the cleaner pattern — every other group in the workspace `Cargo.toml` has one comment line for the whole group, not one per dep.

#### `crates/krax-types/Cargo.toml`

Add `alloy-consensus` to the existing `[dependencies]` block (which currently contains `alloy-primitives` and `thiserror` from 1.1a). Workspace inheritance, no version pin:

```toml
[dependencies]
# B256 (= FixedBytes<32>) is the slot key and value type throughout the State trait.
alloy-primitives = { workspace = true }
# TxEnvelope (EIP-2718 wire-format signed transaction) wraps PendingTx.
alloy-consensus  = { workspace = true }
# Per-crate typed errors per AGENTS.md Rule 3.
thiserror        = { workspace = true }
```

Order: alphabetical by crate name (`alloy-consensus` slots between `alloy-primitives` and `thiserror`).

No serde dep in the per-crate `Cargo.toml`. No `alloy-rpc-types` or `alloy-sol-types` either.

### Decision 13 — `lib.rs` re-export structure after 1.1b

**Resolved: flat namespace, alphabetical across both `pub mod` and `pub use` blocks.**

After Step 1.1b, `crates/krax-types/src/lib.rs` will be:

```rust
//! krax-types: core domain types and cross-crate traits.
//!
//! This crate is the single point of cross-crate type sharing for the Krax workspace.
//! All other crates depend on the traits defined here; none import concrete types
//! from each other directly. See AGENTS.md Rule 1.

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

`MempoolEntry` and `PendingTx` are co-located in `tx.rs` (per Decision 3) and re-exported together. Downstream code writes `use krax_types::MempoolEntry;` and `use krax_types::PendingTx;` — flat namespace.

The new modules are inserted into the alphabetical `pub mod` ordering established in 1.1a (`snapshot`, `state` were the only entries; new entries go in their alphabetical positions — `block` and `journal` before, `rwset` between, `tx` after). Same alphabetical ordering for the `pub use` block.

### Decision 14 — Scope discipline — what 1.1b does NOT do

**Resolved: confirmed exclusions.**

- ❌ No `Inferer` trait — that's Phase 4 Step 4.2.
- ❌ No `Worker` struct — Phase 5.
- ❌ No concrete `MptState` or any `State` implementation — Phase 1.3.
- ❌ No tests — Phase 1.2 (tests follow 1.1b in the same phase).
- ❌ No `#[derive(Serialize, Deserialize)]` on Krax types — Phase 11+.
- ❌ No RLP encoding or block hash computation — Phase 11.
- ❌ No `alloy_rpc_types::Transaction` usage anywhere in 1.1b.
- ❌ No `alloy-eips`, `alloy-sol-types`, or other new alloy sub-crates beyond `alloy-consensus`.
- ❌ No gas accounting types — Phase 2.
- ❌ No `From<Vec<MempoolEntry>> for Block` impl — Phase 5–6 conversion logic.
- ❌ No signature recovery logic — that's a Phase 3 mempool concern. `MempoolEntry` is a data struct; how it gets constructed (with what `sender` value) is not 1.1b's problem. Test code in Step 1.2 will use stub addresses.
- ❌ No `#[derive(Clone)]` on `RWSet` (per Decision 7 sub-decision).
- ❌ No changes outside `crates/krax-types/`, workspace `Cargo.toml`, `ARCHITECTURE.md`, and `AGENTS.md`.

---

## Summary table

| # | Topic | Resolution |
|---|---|---|
| 1 | `PendingTx` envelope type | `alloy_consensus::TxEnvelope`. Coder verifies alias-vs-concrete in source. |
| 2 | `arrival_time` representation | `u64` Unix milliseconds, on `MempoolEntry` (AMENDED — was on `PendingTx`). Cross-step impact: Phase 3 mempool needs deterministic source, not `SystemTime::now()`. |
| 3 | `PendingTx` / `MempoolEntry` split | Two types — `PendingTx { tx }` (wire) and `MempoolEntry { tx, sender, arrival_time }` (enriched). Co-located in `tx.rs`. (AMENDED — surfacer recommended single type with `Option<Address>` sender.) |
| 4 | `Block` hash | Defer entirely to Phase 11 — no hash field, no hash method in 1.1b. |
| 5 | `Block::txs` storage | `Vec<alloy_consensus::TxEnvelope>`. `state_root: B256` required at construction. No `From` impl. |
| 6 | `RWSet` shape | Enum from day one — `Concrete { r_set, w_set }` and `Everything` variants. |
| 7 | `union` / `conflicts` ownership | Borrowing — `&self` and `&RWSet`. No `#[derive(Clone)]` on RWSet. |
| 8 | `JournalEntry` shape | Named struct `JournalEntry { slot, old, new }` with `old: B256` (`B256::ZERO` for "unset"). |
| 9 | `Journal::apply` ownership | Borrowing — `&self`. Returns `Result<(), StateError>`. |
| 10 | `Journal::discard` ownership | Consuming — `self`. Mirrors `Snapshot::release`. |
| 11 | `BTreeSet` discipline | Confirmed: `BTreeSet<B256>` in `RWSet::Concrete`; `Vec<JournalEntry>` in `Journal`. No `HashSet`/`HashMap`. |
| 12 | Cargo.toml edits | Add `alloy-consensus = { version = "1", default-features = false }` to workspace root; add `alloy-consensus = { workspace = true }` to per-crate. |
| 13 | `lib.rs` re-exports | Flat namespace, alphabetical. New `pub mod`s: `block`, `journal`, `rwset`, `tx`. New `pub use`s: `Block`, `{Journal, JournalEntry}`, `RWSet`, `{MempoolEntry, PendingTx}`. |
| 14 | Scope exclusions | Comprehensive — see Decision 14 list. |

---

## Cross-step impact (must be reflected in the plan)

- **ARCHITECTURE.md Step 3.1 `lookahead` signature change.** Current text reads: *"`Mempool` with `add(tx) -> Result<(), MempoolError>`, `lookahead(n: usize) -> Vec<PendingTx>`, `remove(hashes: &[B256])`."* With Decision 3's split, workers consume the enriched type — `lookahead` returns `Vec<MempoolEntry>`. The 1.1b plan must specify the exact `str_replace` for the Step 3.1 line. Text-only edit in the 1.1b commit; no Phase 3 code touched. Same shape as 1.1a's Decision 1 cross-step note for Step 1.4.

- **ARCHITECTURE.md Step 3.1 — additional cross-step note (informational, no edit required in 1.1b plan):** Per Decision 2, the Phase 3 mempool plan must specify a deterministic source for `MempoolEntry::arrival_time`. `SystemTime::now()` at the mempool layer violates Rule 7. This decision in 1.1b sets the type; Phase 3 sets the policy that makes it safe. The 1.1b plan does not need to edit Step 3.1's text for this — Phase 3's planner will face the question when they get there. But the 1.1b plan's "What this step does NOT do" or "Open questions / coder follow-ups" section should record the constraint so it isn't lost.

- **Workspace root `Cargo.toml` edit required** (per Decision 12). 1.1a touched only the per-crate `Cargo.toml`; 1.1b touches both. The plan's "Files to create or modify" section must include the workspace root edit explicitly.

- **No new files outside `crates/krax-types/`, workspace `Cargo.toml`, `ARCHITECTURE.md`, and `AGENTS.md`.** Step 1.1b is contained.

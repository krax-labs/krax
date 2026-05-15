# Step 1.5 Plan — MPT Root Computation (two commits)

Date: 2026-05-14
Status: ⏳ Ready for coder execution
Decisions: docs/plans/step-1.5-decisions.md (✅ Answered 2026-05-14; 19 decisions)
Companion: 1.5 closes Phase 1 — Domain Types & State Trait. After 1.5 lands, the Phase 1 Gate is satisfied and Phase 2 (EVM Execution Wrapper) is next.

## Critical: Do not run git commit

Do not run `git commit`. Stage files via `git add` if useful for verification; commit is the maintainer's action. Report your proposed commit message at the end of each commit's Outcomes block. The maintainer reviews Outcomes and runs the commits. (AGENTS.md "Coding agents do NOT run `git commit`".)

---

## Purpose

Step 1.5 replaces the `B256::ZERO` placeholder in `MptState::root()` (`crates/krax-state/src/mpt/mod.rs:182-188`) with a real Ethereum-compatible Merkle Patricia Trie root computed over the Slots table, and adds an analogous `root()` surface to `Snapshot` so post-commit views can report their root (Phase 14 commitment posting will rely on this). The work ships in **two commits per Decision 16 = (b)**:

1. **Commit 1** — pure trie internals: `crates/krax-state/src/mpt/trie.rs` (Node enum + NodeRef enum + Nibbles helper + sort-then-build `compute_root(entries)` + RLP encoding via `alloy-rlp` + the `EMPTY_ROOT` constant), unit-test vectors, the `alloy-rlp` workspace + per-crate dep additions, and the AGENTS.md Rule 10 approved-dep list update. Nothing about `MptState` / `MptSnapshot` changes in Commit 1; their `root()` still returns the placeholder.

2. **Commit 2** — wire the trie into `MptState::root` and `MptSnapshot::root` (the latter is the **Rule 8 trait surface change** on `Snapshot` — `fn root(&self) -> B256` is added to the trait per Decision 1 = (a)). Memoization fields land here (`Option<B256>` on `MptState` per D2 = (b); `OnceLock<B256>` on `MptSnapshot` per D3 = (b)). Three new root-isolation integration tests extend `tests/snapshot_isolation.rs` per D17 = (a). ARCHITECTURE.md Step 1.5 closes; the Phase 1 Gate "Real MPT root computation in place" line item closes; AGENTS.md Current State + Changelog + Domain Concepts entries are added per D18 = (a).

The custom-MPT vs alloy-trie axis is FROZEN in favor of custom (decisions doc starting context); the scope is "Ethereum-compatible MPT root for the Slots table, nothing more" — no per-account state trie, no proof generation, no ZK-friendly hashes, no sidecar nodes table, no new crates.

---

## Frozen decisions reference

Each Execution Step below cites the decisions it executes. Do NOT re-litigate.

- **D1** = (a) — Add `fn root(&self) -> B256` to the `Snapshot` trait. Rule 8 change.
- **D2** = (b) — Memoize `Option<B256>` on `MptState`; invalidate on `set()`; snapshot does NOT share the cache.
- **D3** = (b) — `MptSnapshot` caches its root lazily inside a `OnceLock<B256>` field.
- **D4** = (a) — Add `alloy-rlp` as a workspace dep + AGENTS.md Rule 10 approved-dep list update, in the SAME commit (1.3b `tempfile` precedent).
- **D5** = (a) — `enum Node { Leaf, Extension, Branch }` + separate `enum NodeRef { Hash(B256), Inline(Vec<u8>) }` (or boxed `Inline(Box<Node>)` — coder picks the lighter-weight variant after writing the encoder; both shapes are equivalent semantically).
- **D6** = (a) — `alloy_primitives::keccak256` everywhere.
- **D7** = (b) — Sort-then-build (bottom-up) trie construction; (c) stack-based incremental is the acceptable coder fallback if (b) needs too much buffering.
- **D8** = (a) — Cursor walk via reth-db `DbTx::cursor_read`; (c) hybrid cursor + builder is the acceptable fallback for tight memory.
- **D9** = (a) — Inline-vs-hash threshold is mandatory per spec. At least one test vector MUST exercise an inline-encoded child (RLP encoding ≤ 32 bytes).
- **D10** = (e) — Hybrid: generate fixtures from `reth-trie` during dev, ship only static fixtures. Coder may substitute (d) (canonical `ethereum/tests` repo MPT vectors) at execution time based on LVP-Q4 outcome.
- **D11** = (c) — Both: hardcode `EMPTY_ROOT` constant AND compute it via the trie path; assert equality.
- **D12** = (d) — `State::root() -> B256` STAYS infallible. On internal MDBX read failure, `MptState::root` / `MptSnapshot::root` emit `tracing::error!` and `panic!` with a documented message. The `State::root` doc comment in `crates/krax-types/src/state.rs` is extended to document the panic surface; the **trait signature is unchanged** (no Rule 8 change to `State`; the Rule 8 change is ONLY on `Snapshot` per D1).
- **D13** = (c) — Single `mpt/trie.rs` for all trie internals; `mpt/mod.rs` stays surface wiring.
- **D14** = (a) — `pub fn compute_root(entries: impl Iterator<Item = (B256, B256)>) -> B256` — stateless, iterator-based, infallible.
- **D15** = (a) — Hold-only at 85% per-crate; document any temporary dips in Outcomes.
- **D16** = (b) — Two commits: (1) trie + unit tests; (2) wiring + integration tests + docs.
- **D17** = (a) — Extend `tests/snapshot_isolation.rs` with three new root-isolation cases. Unit-level vector tests live in `mpt/trie.rs::tests` (or `tests/mpt_root.rs` only if file-cap pressure forces a split).
- **D18** = (a) — Standard close: all five ARCHITECTURE.md Step 1.5 line items checked, heading `✅`, Phase 1 Gate "Real MPT root computation in place" line item checked, AGENTS.md Current State updated to "Phase 1 Gate satisfied — Phase 2 next," Changelog Session 18 appended at BOTTOM, Domain Concepts gains MPT / Trie Node / Storage Root entries.
- **D19** = (a) — `MptState::commit()` populates the memoized root after writing.

---

## Pre-flight — Library Verification Protocol

Run all six queries below BEFORE Commit 1's Execution Steps. Cite findings inline in each commit's Outcomes "LVP findings" block using the per-query template (1.3b / 1.4 precedent). Context7 first; cargo-registry / on-disk source fallback ONLY on genuine unavailability (HTTP 5xx / no relevant hits / Context7 returns unrelated content) — NOT "I prefer source." All six queries are completable in Commit 1's window; the findings carry into Commit 2 without re-verification.

### Per-query template (fill in at execution time)

```
- **Q<N>: <one-line restatement of what the query proves>**
  - Library: <crate + version/rev>
  - Query: <Context7 query string actually issued, OR cargo-registry source path if fallback>
  - Expected finding: <what the planner expected, restated from below>
  - Actual finding: <what was retrieved>
  - Source path + line: <file:line OR Context7 doc URL>
  - Verbatim quote: <minimal verbatim excerpt that supports the finding>
  - Decision impact: <which Decision(s) this finding gates; if a gap is found, mark "AUDIT GAP — STOP">
```

### LVP-Q1 (tier-1) — `alloy-rlp` encoding API surface

- **Expected:** `alloy_rlp::Encodable` trait with `fn encode(&self, out: &mut dyn BufMut)` and `fn length(&self) -> usize`. `alloy_rlp::encode(&value)` free function returning `Vec<u8>`. Single-byte / empty-string / short-string (1–55 bytes) / long-string (>55 bytes) discriminants per the standard RLP spec. The crate exposes raw byte-string encoding helpers (`encode_fixed_size`, `Header`, or equivalent) that we can call directly to RLP-encode `&[u8]` slices — required for encoding trie node children/values without forcing a wrapper struct.
- **Source-fallback target:** `alloy-rs/core` repo `crates/rlp/src/encode.rs` (and `header.rs` for `Header::encode`).
- **Decision impact:** D4 (a) — direct dep on `alloy-rlp` is sound. If the trait surface is wildly different from the planner's expectation (e.g. no `BufMut`-based `encode`), surface as Outcomes deviation and propose a hand-rolled fallback under Decision 4 (b) — STOP and re-surface to maintainer.

### LVP-Q2 (tier-1) — `alloy_primitives::keccak256` input + output

- **Expected:** `pub fn keccak256(bytes: impl AsRef<[u8]>) -> B256` re-exported from `alloy-primitives` at the same major version we already pin (`alloy-primitives = "1"`). Stable across the workspace pin.
- **Source-fallback target:** `alloy-rs/core` repo `crates/primitives/src/utils.rs` (or wherever `keccak256` lives at the pinned major).
- **Decision impact:** D6 (a) — confirms `alloy_primitives::keccak256` is the correct call. Cheap; near-certain.

### LVP-Q3 (tier-1) — Ethereum MPT spec: nibble-prefix encoding + inline-vs-hash threshold + branch layout

- **Expected:** (i) RLP encoding of a node ≤ 32 bytes → embed inline in parent as the encoded bytes themselves (NOT wrapped in another RLP layer); > 32 bytes → hash via keccak256 and embed the 32-byte hash. (ii) Leaf and Extension nibble path encoding: terminator nibble distinguishes leaf vs extension; the path's parity (odd/even) selects between prefix byte `0x00` / `0x01` / `0x02` / `0x03` — even-extension `0x00`, odd-extension `0x1n`, even-leaf `0x20`, odd-leaf `0x3n` where `n` is the first nibble of an odd-length path. (iii) Branch node: 17-element RLP list — 16 children (each a hash, an inline node, or RLP-empty-string `0x80`) followed by an optional value slot.
- **Source:** Ethereum Yellow Paper Appendix D, OR the canonical Ethereum wiki page on Modified Merkle Patricia Trie (search "Compact encoding of hex sequence with optional terminator" + "Trie definition"). Either source authoritative.
- **Decision impact:** D9 (a) + D11 (c) + trie implementation generally. **Most-commonly-botched part of a custom MPT** — getting the nibble prefix bytes wrong, or wrapping inline children in an extra RLP layer, produces a structurally valid root that does NOT match go-ethereum / reth.

### LVP-Q4 (tier-1) — test-vector source: canonical `ethereum/tests` MPT subdirectory AND `reth-trie` storage-root oracle

- **Expected (sub-query 4a, canonical tests path):** The `ethereum/tests` repo at `https://github.com/ethereum/tests` ships MPT test vectors under a `TrieTests/` (or similarly named) subdirectory in JSON shape `{ name: { in: { hex_key: hex_value, ... }, root: "0x..." } }`. Determine the precise directory path, file naming convention, and JSON schema at the latest tag.
- **Expected (sub-query 4b, reth oracle path):** `reth-trie` exposes a public API for computing a single-trie storage root from `(B256, B256)` entries — likely `StorageRoot`-builder pattern or a free `storage_root_of(entries)` helper. Identify the exact function signature at the workspace-pinned reth rev `02d1776786abc61721ae8876898ad19a702e0070`. This is needed at **development time only** — used to populate the static JSON fixtures, then dropped before commit (D10 (e)). `reth-trie` is **NOT** added as a `[dev-dependencies]` entry in the shipped tree.
- **Source-fallback target (4a):** `https://github.com/ethereum/tests` repo root.
- **Source-fallback target (4b):** pinned reth rev — likely `crates/trie/trie/src/state.rs`, `crates/trie/trie/src/proof.rs`, or `crates/trie/trie/src/hash_builder/mod.rs`.
- **Decision impact:** D10 — coder picks (d) (canonical Ethereum tests JSON) vs (e) (reth-generated fixtures) at execution time based on whichever path is more workable from LVP findings. EITHER outcome ships only static JSON fixtures; neither outcome adds `reth-trie` as a shipped dep.

### LVP-Q5 (tier-1) — reth-db cursor API at the pinned rev

- **Expected:** `DbTx::cursor_read::<T>() -> Result<<Self as DbTx>::Cursor<T>, DatabaseError>` (or named-associated-type equivalent). The cursor exposes a `walk(start: Option<T::Key>)` method (or `walk_range`, or `first` + `next`) returning a falliable iterator of `(T::Key, T::Value)` pairs in B-tree key order. Cursor lifetime is bound to the txn (`'tx`), and the `<DatabaseEnv as Database>::TX` we hold in `MptSnapshot` carries a `'static` bound (already confirmed in 1.3b LVP-Q2). The cursor must be drop-safe across our compute_root iteration (no early-abort hazards on errors — we panic per D12 (d)).
- **Source-fallback target:** `crates/storage/db-api/src/cursor.rs` at reth rev `02d1776786abc61721ae8876898ad19a702e0070`. Likely also `crates/storage/db-api/src/transaction.rs` for the `cursor_read` declaration.
- **Decision impact:** D8 (a). If the cursor's lifetime / borrow shape forces a hybrid approach (cursor → BTreeMap or cursor → small buffer), fall back to D8 (c). If cursors at this rev are absent / radically different from expected, surface as AUDIT GAP — STOP.

### LVP-Q6 (tier-2) — empty-trie root constant cross-check

- **Expected:** `keccak256(rlp(""))` = `0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421`. Confirmed against go-ethereum's `core/types` constants (`EmptyRootHash` / `EmptyTrieHash`) or the Ethereum wiki MPT page.
- **Source-fallback target:** go-ethereum repo `core/types/hashes.go` (or wherever `EmptyRootHash` is defined), OR Ethereum wiki MPT page constants section.
- **Decision impact:** D11 (c). Tier-2 because the constant is widely published; the LVP-Q3 finding implicitly confirms it via the spec.

---

# Commit 1: feat(state): add MPT trie module with sort-then-build root computation — Step 1.5 (1/2)

## Purpose

Land the trie internals as a standalone, unit-tested module under `crates/krax-state/src/mpt/trie.rs`. After Commit 1: `alloy-rlp` is a workspace + per-crate dep; AGENTS.md Rule 10 includes `alloy-rlp`; `mpt/trie.rs` exists with `Node`/`NodeRef`/`Nibbles` types, RLP-encoded `compute_root(entries) -> B256`, the `EMPTY_ROOT` constant, and a unit-test suite that asserts at least six fixed-vector roots (empty, single, two-divergent, shared-prefix-extension+branch, inline-encoded-child, and one larger reth-or-tests-derived vector). **`MptState::root` and `MptSnapshot::root` are NOT touched in Commit 1** — they continue to return `B256::ZERO` with the `// TODO Step 1.5` marker until Commit 2.

## Execution Steps

### Step 1.1 — Add `alloy-rlp` to workspace `Cargo.toml`

**File:** `Cargo.toml` (workspace root)

**Old (lines 65–70, current HEAD):**

```toml
# --- Ethereum types ---
# ✅ Context7 (/alloy-rs/alloy + /alloy-rs/core, 2026-05-06; alloy-consensus added 2026-05-09): version "1" confirmed for all four.
alloy-primitives = { version = "1", default-features = false, features = ["serde"] }
alloy-consensus  = { version = "1", default-features = false }
alloy-rpc-types  = { version = "1", default-features = false }
alloy-sol-types  = { version = "1", default-features = false }
```

**New:**

```toml
# --- Ethereum types ---
# ✅ Context7 (/alloy-rs/alloy + /alloy-rs/core, 2026-05-06; alloy-consensus added 2026-05-09): version "1" confirmed for all four.
# ✅ Context7 LVP-Q1 (Step 1.5, 2026-05-14): alloy-rlp at "<version confirmed by LVP-Q1>".
alloy-primitives = { version = "1", default-features = false, features = ["serde"] }
alloy-consensus  = { version = "1", default-features = false }
alloy-rpc-types  = { version = "1", default-features = false }
alloy-sol-types  = { version = "1", default-features = false }
alloy-rlp        = { version = "<version confirmed by LVP-Q1>", default-features = false }
```

**Coder action:** the planner does NOT freeze a version pin — fill the `<version confirmed by LVP-Q1>` placeholder with whatever LVP-Q1 returns as the current stable version (1.3b precedent: pin to the latest stable major). The 1.3b `tempfile` precedent shows the test-only group; `alloy-rlp` is a runtime dep, NOT test-only — it goes alongside the other `alloy-*` lines in the Ethereum-types group.

**Rationale:** D4 (a). `alloy-rlp` is the canonical Ethereum-stack RLP crate; thin, focused, low V2 unwind cost. Required by Steps 1.3 + 1.4 below.

### Step 1.2 — Add `alloy-rlp` to `crates/krax-state/Cargo.toml`

**File:** `crates/krax-state/Cargo.toml`

**Old (lines 9–23, current HEAD):**

```toml
[dependencies]
# MptState implements State and Snapshot from krax-types.
krax-types        = { path = "../krax-types" }
# B256 is used directly in mpt/mod.rs; krax-types does not re-export it.
alloy-primitives  = { workspace = true }
# MDBX env + table + transaction surface for the durable MptState backend (Step 1.3b).
# LVP finding: `mdbx` feature must be enabled — workspace dep is `default-features = false`
# and the env/txn surface (DatabaseEnv, init_db) is gated on that feature.
reth-db           = { workspace = true, features = ["mdbx"] }
# Optional: required when `integration` feature is on, because
# `MptState::open_temporary` (cfg'd under `any(test, feature = "integration")`)
# names `tempfile::TempDir` in its return type. Under `cfg(test)` the
# dev-dependency below is in scope; under `feature = "integration"` we need
# this optional regular dep too.
tempfile          = { workspace = true, optional = true }
```

**New:**

```toml
[dependencies]
# MptState implements State and Snapshot from krax-types.
krax-types        = { path = "../krax-types" }
# B256 is used directly in mpt/mod.rs; krax-types does not re-export it.
alloy-primitives  = { workspace = true }
# RLP encoding for trie nodes + the storage-slot value strip-leading-zeros transform (Step 1.5).
# Per Context7 LVP-Q1 (2026-05-14): canonical Ethereum-stack RLP crate.
alloy-rlp         = { workspace = true }
# MDBX env + table + transaction surface for the durable MptState backend (Step 1.3b).
# LVP finding: `mdbx` feature must be enabled — workspace dep is `default-features = false`
# and the env/txn surface (DatabaseEnv, init_db) is gated on that feature.
reth-db           = { workspace = true, features = ["mdbx"] }
# tracing::error! before panicking on internal MDBX read failure in
# MptState::root / MptSnapshot::root (Step 1.5 Decision 12 (d)).
tracing           = { workspace = true }
# Optional: required when `integration` feature is on, because
# `MptState::open_temporary` (cfg'd under `any(test, feature = "integration")`)
# names `tempfile::TempDir` in its return type. Under `cfg(test)` the
# dev-dependency below is in scope; under `feature = "integration"` we need
# this optional regular dep too.
tempfile          = { workspace = true, optional = true }
```

**Coder action:** verify `tracing` is already in `[workspace.dependencies]` (it is — line 103 of root `Cargo.toml`); confirm via `cargo tree -p krax-state` after the edit that `tracing` is in scope. If `tracing` was already an indirect dep through `reth-db`, the explicit declaration is still required (Rule 1 — don't rely on transitive imports).

**Rationale:** D4 (a) (alloy-rlp), D12 (d) (tracing::error! emit before panic). NOT added to `krax-types/Cargo.toml` — the trie code lives in `krax-state` only, and `krax-types`'s only Step 1.5 edit is the doc-comment / trait-method addition (Commit 2). Per the dispatch out-of-scope check, no other Cargo.toml file changes.

### Step 1.3 — Create `crates/krax-state/src/mpt/trie.rs` (NEW FILE)

**File (NEW):** `crates/krax-state/src/mpt/trie.rs`

**Skeleton + key invariants (coder writes full body at execution time, citing LVP-Q1/Q3 inline above each load-bearing call site per AGENTS.md Library Verification Protocol):**

```rust
//! Ethereum-compatible Merkle Patricia Trie root computation over the
//! Slots table — Step 1.5 trie internals.
//!
//! Scope: the public surface is the single function [`compute_root`]
//! (Decision 14 (a)). The internals — [`Node`], [`NodeRef`], [`Nibbles`],
//! and RLP encoding helpers — are pub(super) so `mpt/mod.rs` can call them
//! without re-exporting them at the crate root.
//!
//! Algorithm: sort-then-build (Decision 7 (b)) — entries arrive in B-tree
//! key order from the MDBX cursor (Decision 8 (a)), so the trie is
//! constructed bottom-up in one pass. Per spec (LVP-Q3): nodes whose RLP
//! encoding is ≤ 32 bytes are embedded inline in their parent; > 32 bytes
//! are referenced by their keccak256 hash. The [`EMPTY_ROOT`] constant
//! (Decision 11 (c)) is asserted equal to the computed empty-trie path in
//! the test suite below.
//!
//! Out of scope (per Step 1.5 decisions doc out-of-scope reminder): proof
//! generation, per-account state trie / world state root, ZK-friendly
//! hashes, trie pruning. The Node type is plain enough to admit proof
//! generation later without rewrite (Decision 5 (a) — we did NOT close
//! that door), but Step 1.5 does not ship it.

use alloy_primitives::{B256, keccak256};
// Per Context7 LVP-Q1 (alloy-rlp v<X>, 2026-05-14): <verbatim surface citation>.
use alloy_rlp::{Encodable /* + Header/encode helpers per LVP-Q1 */};

/// The keccak256 hash of the RLP encoding of the empty string —
/// the canonical Ethereum empty-trie root.
///
/// Cross-checked at LVP-Q6 against go-ethereum's `EmptyRootHash` /
/// Ethereum wiki. Asserted equal to the value [`compute_root`] returns
/// for an empty entry stream in the test suite (Decision 11 (c)).
pub(super) const EMPTY_ROOT: B256 = B256::new([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6,
    0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0,
    0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

/// Trie node — three Ethereum MPT kinds (Decision 5 (a)).
#[derive(Debug)]
pub(super) enum Node {
    Leaf { path: Nibbles, value: Vec<u8> },
    Extension { path: Nibbles, child: NodeRef },
    Branch { children: [Option<NodeRef>; 16], value: Option<Vec<u8>> },
}

/// Reference to a child node — inline (RLP ≤ 32 bytes embedded directly)
/// or hashed (> 32 bytes referenced by keccak256). The inline-vs-hash
/// distinction is THE spec-load-bearing detail — LVP-Q3.
#[derive(Debug)]
pub(super) enum NodeRef {
    Hash(B256),
    Inline(Vec<u8>),
}

/// Nibble-path helper: an even-or-odd-length sequence of 4-bit nibbles
/// with a `is_leaf` terminator flag. RLP-prefix-encoding per LVP-Q3 lives
/// in [`Nibbles::encode_path`].
#[derive(Debug, Clone)]
pub(super) struct Nibbles { /* nibbles: Vec<u8>, is_leaf: bool */ }

// ... encode_path, sort-then-build internal helpers, ...

/// Computes the Ethereum-compatible MPT root over `entries`.
///
/// Entries MUST arrive in ascending key order (the MDBX cursor at
/// `Slots` provides this naturally — Decision 8 (a) + LVP-Q5). If
/// `entries` yields zero items, returns [`EMPTY_ROOT`] (Decision 11 (c)).
///
/// Infallible by design (Decision 14 (a) + Decision 12 (d)). Internal
/// invariant violations (which cannot arise from well-formed sorted
/// input) trigger `panic!`; caller-side I/O errors are converted to
/// panics at the [`MptState::root`] / [`MptSnapshot::root`] boundary
/// in `mpt/mod.rs` (Commit 2).
pub fn compute_root(entries: impl Iterator<Item = (B256, B256)>) -> B256 {
    // sort-then-build (Decision 7 (b)). Coder MAY fall back to a
    // stack-based incremental builder (Decision 7 (c)) if the auxiliary
    // memory needed by (b) exceeds reasonable bounds; either algorithm
    // is acceptable as long as it produces the same roots.
    todo!("Step 1.5 Commit 1: implement per LVP-Q1/Q3/Q6")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use alloy_primitives::B256;
    use pretty_assertions::assert_eq;

    // (i) Empty trie returns EMPTY_ROOT AND matches the computed path
    //     (Decision 11 (c)).
    #[test]
    fn empty_trie_root_is_empty_root_constant() { /* ... */ }

    #[test]
    fn empty_trie_path_matches_constant() {
        assert_eq!(compute_root(std::iter::empty()), EMPTY_ROOT);
    }

    // (ii) Single-key vector — root computed against either the canonical
    //      ethereum/tests JSON (D10 (d)) OR a reth-trie-generated fixture
    //      (D10 (e)).
    #[test]
    fn single_key_vector() { /* ... */ }

    // (iii) Two-key diverging-prefix vector (forces a top-level branch).
    #[test]
    fn two_keys_diverging_prefix() { /* ... */ }

    // (iv) Two-key shared-prefix vector (forces extension+branch).
    #[test]
    fn two_keys_shared_prefix_extension_then_branch() { /* ... */ }

    // (v) Inline-encoded-child vector — at least one child with RLP
    //     encoding ≤ 32 bytes (Decision 9 (a) — load-bearing per the
    //     dispatch's "most-commonly-botched" warning).
    #[test]
    fn inline_encoded_child_vector() { /* ... */ }

    // (vi) ≥ 1 vector loaded from the static fixture file
    //      `crates/krax-state/tests/fixtures/mpt_roots.json` (Step 1.5
    //      Decision 10 (e) — reth-trie-generated, OR (d) — canonical
    //      Ethereum tests).
    #[test]
    fn fixture_file_vectors() { /* ... */ }
}
```

**Notes for the coder:**

- **The `// Per Context7 LVP-Q1 (...)` comment above the `use alloy_rlp::...` line is load-bearing** per AGENTS.md "For the coding agent" — every external-library import gets an inline Context7-confirmed comment.
- **D5 (a) NodeRef variant.** The plan shows `NodeRef::Inline(Vec<u8>)` because the inline reference is literally the child's RLP-encoded bytes (per LVP-Q3, an inline child IS its encoding — not a re-RLP-wrapped further). If during implementation the coder finds `Inline(Box<Node>)` cleaner for in-memory traversal (deferring the inline RLP encoding to the parent's encoding pass), either shape is acceptable as long as the on-wire encoding matches the spec. Document the chosen shape in Outcomes.
- **D7 (b) algorithm.** The dispatch authorizes (c) as a fallback — if mid-implementation the sort-then-build buffering exceeds reasonable bounds (e.g. requires materializing all entries into `Vec<(B256, B256)>` for a large slot count), switch to the stack-based incremental builder (HashBuilder-style). Document the choice in Outcomes.
- **File-cap (AGENTS.md / krax-conventions).** `trie.rs` must stay under 500 lines (per krax-conventions skill). If implementation pushes past ~450 lines, split: `mpt/nibbles.rs` and/or `mpt/rlp.rs` are pre-authorized splits (D13 (c) — "if `rlp` or `nibbles` helpers grow beyond ~50 lines each, split them — but not preemptively"). The split is a per-file refactor, not a Decision change.
- **`#[cfg(test)] mod tests`.** Unit-level vector tests live INSIDE `trie.rs` per D17 (a). If the test block grows large enough that `trie.rs` exceeds 500 lines, move the test block to `crates/krax-state/tests/mpt_root.rs` (a new integration-style test file, no `integration` feature gate needed since these tests are pure-CPU — they call `compute_root` on `std::iter::*`). Document the split in Outcomes.
- **D11 (c) two-assertion form.** Test (i) asserts `EMPTY_ROOT` equals the literal constant bytes; test (ii) (the `empty_trie_path_matches_constant` test) asserts `compute_root(empty)` equals `EMPTY_ROOT`. Both tests must exist.
- **D9 (a) inline-encoded-child vector.** Construct a fixture whose total node-encoding size for at least one child is ≤ 32 bytes — typically a leaf with a short value (e.g. a single-byte storage value, which RLP-strips-leading-zeros down to its non-zero suffix). If you cannot find a canonical fixture that exercises this, hand-construct one and compute the expected root via the dev-time `reth-trie` oracle (D10 (e)).

### Step 1.4 — Wire `mod trie;` into `crates/krax-state/src/mpt/mod.rs`

**File:** `crates/krax-state/src/mpt/mod.rs`

**Old (line 75–77, current HEAD):**

```rust
mod slots;

use slots::{Slots, SlotsTableSet};
```

**New:**

```rust
mod slots;
mod trie;

use slots::{Slots, SlotsTableSet};
```

**Rationale:** D13 (c). `trie` is `mod` (not `pub mod`) — its types are crate-internal; `mpt/mod.rs` consumes `trie::compute_root` (and possibly `trie::EMPTY_ROOT`) via the `pub(super)` visibility above. **This is the ONLY edit to `mpt/mod.rs` in Commit 1** — Commit 2 wires the `root()` calls.

### Step 1.5 — Vendor static test fixtures at `crates/krax-state/tests/fixtures/mpt_roots.json`

**File (NEW):** `crates/krax-state/tests/fixtures/mpt_roots.json`

**Coder action:**

1. Per LVP-Q4 outcome, pick path:
   - **(d) canonical Ethereum tests path:** Vendor a small JSON subset from `https://github.com/ethereum/tests` (`TrieTests/` subdirectory at a stable tag). Preserve the upstream JSON shape. Cite the source tag + SHA in a top-level JSON `"_source"` key.
   - **(e) reth-trie oracle path:** Spin up a one-off dev binary (NOT committed) that depends on `reth-trie` at the workspace-pinned rev, generates `(slot_set, expected_root)` pairs for ~6 fixtures, serializes them as JSON, drops the binary + the `reth-trie` reference. Vendor the JSON only. Document the generation procedure as a comment in the JSON's `"_source"` field.
2. The shipped JSON file MUST have stable enough shape that the test in `trie.rs::tests::fixture_file_vectors` can deserialize it via `serde_json` — note that `serde_json` is already a workspace dep, but is NOT currently in `krax-state/Cargo.toml`'s `[dev-dependencies]`. If `fixture_file_vectors` needs JSON parsing, add `serde_json = { workspace = true }` to `crates/krax-state/Cargo.toml`'s `[dev-dependencies]` in Step 1.2's same edit. **Coder action:** decide at execution time whether to use serde_json (cleanest) or hand-parse a hex-key/hex-value JSON via byte-level scanning (no new dev-dep). The planner LEANS to serde_json — adding a dev-dep that's already in workspace doesn't fire Rule 10 (it's already approved). Document the chosen path in Outcomes.
3. The fixture file MUST exercise at least one inline-encoded child (D9 (a)).

**Rationale:** D10 (e) primary, (d) fallback. The fixture lives under `tests/fixtures/` (NOT under `src/`) so it is excluded from `cargo build` output and is read at test time via `include_str!("../../tests/fixtures/mpt_roots.json")` or a relative `std::fs::read_to_string` (depending on the dev-dep choice).

## Verification suite — Commit 1 scope

| # | Item | Command / Procedure | Expected Result |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | Lint clean | `make lint` | exit 0; no `unused_imports`, `clippy::unwrap_used` outside test modules, no pedantic firings |
| 3 | Unit tests pass | `cargo test -p krax-state --lib trie::tests` | exit 0; ≥ 6 test functions all pass (empty / empty-path-matches-constant / single / two-diverging / two-shared-prefix / inline-child / fixture-file) |
| 4 | Full unit-test run | `make test` | exit 0; preexisting test count preserved (krax-types 14, mpt::tests 4) PLUS the new `trie::tests` cases |
| 5 | Fixture file present | `ls crates/krax-state/tests/fixtures/mpt_roots.json` | file exists, non-empty, contains at least one entry with hex-key/hex-value plus expected root |
| 6 | `alloy-rlp` in workspace deps | `grep -n '^alloy-rlp' Cargo.toml` | one match in the Ethereum-types group |
| 7 | `alloy-rlp` in `krax-state` deps | `grep -n 'alloy-rlp' crates/krax-state/Cargo.toml` | one match under `[dependencies]`, NOT under `[dev-dependencies]` |
| 8 | `alloy-rlp` NOT in `krax-types` deps | `grep -n 'alloy-rlp' crates/krax-types/Cargo.toml` | zero matches (out-of-scope) |
| 9 | `tracing` declared in `krax-state` (D12 d) | `grep -n '^tracing' crates/krax-state/Cargo.toml` | one match under `[dependencies]` |
| 10 | AGENTS.md Rule 10 list contains `alloy-rlp` | `grep -n 'alloy-rlp' AGENTS.md` | at least one match under the Rule 10 approved-root-dependencies block (~line 365) |
| 11 | `trie.rs` exists with the right public surface | `grep -nE 'pub fn compute_root\|pub\(super\) const EMPTY_ROOT' crates/krax-state/src/mpt/trie.rs` | both names present |
| 12 | `EMPTY_ROOT` constant matches the canonical value | inspect `trie.rs::EMPTY_ROOT`; cross-check against LVP-Q6 finding | bytes match `0x56e81f17...b421` |
| 13 | `mod trie;` declared in `mpt/mod.rs` | `grep -n '^mod trie;' crates/krax-state/src/mpt/mod.rs` | one match |
| 14 | `MptState::root` STILL returns placeholder (Commit 1 doesn't touch wiring) | `grep -nA2 '// TODO Step 1.5' crates/krax-state/src/mpt/mod.rs` | placeholder still present; `B256::ZERO` still returned |
| 15 | `Snapshot` trait UNCHANGED in Commit 1 | `git diff -- crates/krax-types/src/snapshot.rs` | empty diff (Commit 1 does not edit krax-types) |
| 16 | `trie.rs` under 500-line cap | `wc -l crates/krax-state/src/mpt/trie.rs` | result ≤ 500; if split mid-implementation, also `wc -l crates/krax-state/src/mpt/{nibbles,rlp}.rs` each ≤ 500 |
| 17 | LVP block populated for Commit 1 | inspect Outcomes → "LVP findings" | Q1, Q2, Q3, Q4, Q5, Q6 each populated; if any STOP/AUDIT-GAP — halt |
| 18 | No `reth-trie` in shipped deps (D10 (e) discipline) | `grep -rnE '^reth-trie' Cargo.toml crates/*/Cargo.toml` | zero matches |
| 19 | No `proptest` added (D14 + decisions out-of-scope) | `grep -nE '^proptest' crates/krax-state/Cargo.toml` | zero matches in per-crate Cargo.toml |
| 20 | No new crates created | `git status --porcelain \| grep -E '^A.*crates/[^/]+/Cargo\.toml$'` | zero matches |
| 21 | `Snapshot::release` `compile_fail` doctest preserved (Commit 1) | `grep -n '```compile_fail' crates/krax-types/src/snapshot.rs` | one match (the 1.4 doctest — untouched in Commit 1) |
| 22 | **Out-of-scope check (decisions doc out-of-scope reminder)** | inspect full Commit-1 diff: no per-account trie, no proof gen, no ZK hashes, no sidecar nodes table, no `alloy-trie` / `reth-trie` shipped, no new crates, no `State` trait signature change, no `state.rs` edits, no `Snapshot` trait edits | every item passes; if any fails, HALT and re-surface |

## Commit message — Commit 1

```
feat(state): add MPT trie module with sort-then-build root computation — Step 1.5 (1/2)
```

## Outcomes — Commit 1 (filled in at execution time, 2026-05-15)

### Files changed

- `Cargo.toml` (workspace) — added `alloy-rlp = { version = "0.3", default-features = false }` to the Ethereum-types group with an LVP-Q1 provenance comment (Step 1.1).
- `crates/krax-state/Cargo.toml` — added `alloy-rlp = { workspace = true }` and `tracing = { workspace = true }` to `[dependencies]`; added `serde_json = { workspace = true }` to `[dev-dependencies]` for fixture parsing (Step 1.2 + Step 1.5 note 2 — serde_json already a Rule-10-approved workspace dep, no approved-list change) (Step 1.2).
- `Cargo.lock` — regenerated (adds `alloy-rlp 0.3.15` + its `bytes`/`arrayvec` deps).
- `AGENTS.md` — Rule 10 approved-dep list: appended `alloy-rlp` to the `alloy` grouping line (`...alloy-sol-types\`, \`alloy-rlp\`) — Ethereum types, ABI, and RLP encoding`). Chosen presentation: extend the existing alloy grouping line (not a sibling bullet) — 1.3b `tempfile` precedent. **Landed in Commit 1**, not Commit 2 — see Deviations. (gitignored — maintainer `git add -f`).
- `crates/krax-state/src/mpt/trie.rs` — **NEW FILE** (472 lines, < 500 cap). `EMPTY_ROOT` const; `Nibbles` (compact/hex-prefix encoding); `NodeRef { Hash(B256), Inline(Vec<u8>) }`; `Node { Leaf, Extension, Branch }` (+ `encode`); `compute_root(impl Iterator<Item=(B256,B256)>) -> B256` (secure-trie keccak256(slot) path, `rlp(minimal(value))` leaf, zero-value exclusion, sort-then-build); 8 `#[cfg(test)] mod tests` functions. Carries a documented `#![allow(dead_code)]` (Commit 2 removes it — see Deviations).
- `crates/krax-state/src/mpt/mod.rs` — added `mod trie;` (the ONLY Commit 1 edit; `MptState::root` placeholder untouched, `// TODO Step 1.5` preserved).
- `crates/krax-state/tests/fixtures/mpt_roots.json` — **NEW FILE.** 7 `(name, entries, root)` vectors + `empty_root`, generated by the D10 (e) dev oracle (alloy-trie 0.9.5 `storage_root_unhashed`); oracle dropped, never in tree.
- `docs/plans/step-1.5-plan.md` — this Commit 1 Outcomes block filled in.

### LVP findings (Q1–Q6)

- **Q1: `alloy-rlp` encoding API surface — CONFIRMED (matches planner expectation).**
  - Library: `alloy-rlp` 0.3.15 (workspace pin `"0.3"`).
  - Query: Context7 `/alloy-rs/rlp` "Encodable trait encode/length, free encode() -> Vec<u8>, Header, byte-string discriminants" + on-disk fetch of `github.com/alloy-rs/rlp` `crates/rlp/Cargo.toml` (features) and workspace `Cargo.toml` (version).
  - Expected finding: `Encodable::encode(&self, out: &mut dyn BufMut)` + `length`; free `encode() -> Vec<u8>`; `Header { list, payload_length }` with `.encode`; raw `&[u8]` byte-string encoding; `default-features = false` keeps these available.
  - Actual finding: All confirmed. `alloy_rlp::encode(&b"\xAB\xBA"[..]) == [0x82,0xAB,0xBA]`, `encode("") == [0x80]`. `Header { list: true, payload_length: 3 }.encode(&mut out)` → `[0xC3]`. `Encodable` requires `encode(&self, out: &mut dyn BufMut)`, optional `length`. Crate `[features]`: `default = ["std"]`, `std = ["bytes/std", "arrayvec?/std"]` — there is NO `alloc` feature; `alloc` is unconditional, so `encode()`/`Encodable`/`Header` remain available under `default-features = false` (`std` only adds `bytes/std`, `arrayvec/std`, `std::error::Error` impls). Crate version (workspace.package): `0.3.15`.
  - Source path + line: Context7 `/alloy-rs/rlp` llms.txt (Encodable / Header / encode sections); `raw.githubusercontent.com/alloy-rs/rlp/main/crates/rlp/Cargo.toml` `[features]`; `.../rlp/main/Cargo.toml` `version = "0.3.15"`.
  - Verbatim quote: `default = ["std"]` / `std = ["bytes/std", "arrayvec?/std"]`; `Header { list: true, payload_length: 3 }.encode(&mut out); assert_eq!(&out[..], &[0xC3]);`
  - Decision impact: D4 (a) sound; the plan's frozen `default-features = false` is correct (no surface lost). No STOP.
- **Q2: `alloy_primitives::keccak256` — CONFIRMED.**
  - Library: `alloy-primitives` v1 (workspace pin), latest crates.io 1.6.0.
  - Query: Context7 `/alloy-rs/core` "keccak256 signature input AsRef<[u8]> output B256".
  - Expected finding: `keccak256(impl AsRef<[u8]>) -> B256`, stable at pinned major.
  - Actual finding: Confirmed — `keccak256(b"hello") == b256!("1c8aff95…")`; "computes the Keccak-256 hash … over any type that implements `AsRef<[u8]>`"; returns `B256`. Called O(N) per root in `compute_root` (keys + every node).
  - Source path + line: Context7 `/alloy-rs/core` llms.txt (`keccak256` section).
  - Verbatim quote: "The `keccak256` function computes the Keccak-256 hash … over any type that implements `AsRef<[u8]>`."
  - Decision impact: D6 (a) confirmed. No STOP.
- **Q3: Ethereum MPT spec (nibble-prefix, inline-vs-hash, branch layout) — CONFIRMED with one prose correction.**
  - Library/source: go-ethereum `master` `trie/encoding.go`, `trie/hasher.go`, `trie/node_enc.go` (canonical reference implementation; authoritative per LVP-Q3 source list).
  - Query: on-disk fetch of the three geth trie files.
  - Expected finding: compact hex-prefix `(is_leaf<<5)|(odd<<4)|firstNibble?`; inline when RLP `≤32`, hash when `>32`; branch = 17-element list.
  - Actual finding: Compact encoding confirmed verbatim (`buf[0] = terminator << 5`; `if odd: buf[0] |= 1<<4 | hex[0]`; pack pairs). Branch = `fullNode` 17-element list (16 children + value; empty slot = `rlp.EmptyString` = `0x80`); inline child spliced raw (`w.Write(c)`), hashed child `WriteBytes` (`0xa0||hash`). **Correction:** geth `trie/hasher.go:68` uses `if len(enc) < 32` — i.e. inline is **strictly `< 32`**; a node whose RLP is exactly 32 bytes is **hashed**, not inline. The decisions/plan prose said "≤ 32 inline / > 32 hash" (loose). Implemented per the authoritative spec (`< 32` inline, `>= 32` hash), which IS the go-ethereum match D9's stated intent names. Also confirmed: `leafNodeEncoder.encode` does `WriteBytes(Key); WriteBytes(Val)` where storage `Val` is already `RLP(minimal(value))` → on-wire leaf value is `RLP_string(RLP(minimal(value)))` (deliberate double-wrap). Root is force-hashed (geth `force` flag) regardless of size.
  - Source path + line: `go-ethereum/trie/encoding.go:18-50` (hexToCompact + doc); `trie/hasher.go:68` (`if len(enc) < 32 && !force`); `trie/node_enc.go` (`fullNode.encode`, `leafNodeEncoder.encode`, `extNodeEncoder.encode`).
  - Verbatim quote: `if len(enc) < 32 && !force {` / `// Nodes smaller than 32 bytes are embedded directly in their parent.`; `func (n *leafNodeEncoder) encode(w rlp.EncoderBuffer) { offset := w.List(); w.WriteBytes(n.Key); w.WriteBytes(n.Val); w.ListEnd(offset) }`.
  - Decision impact: D9 (a) + D11 (c). The `≤32`→`<32` correction is a prose deviation (see Deviations), NOT a material gap — empirically validated by all 7 oracle fixtures matching. No STOP.
- **Q4: test-vector source — (d) ethereum/tests rejected; (e) executed via alloy-trie.**
  - Library/source: `ethereum/tests` repo `TrieTests/` (GitHub contents API + `trietest_secureTrie.json`); alloy-trie 0.9.5 `src/root.rs` (the storage-trie root path reth-trie is built on).
  - Query: GitHub API `repos/ethereum/tests/contents/TrieTests`; raw `trietest_secureTrie.json`; on-disk alloy-trie-0.9.5 `src/root.rs` + `src/lib.rs` + `Cargo.toml`.
  - Expected finding (4a): canonical `(set, root)` JSON usable directly. (4b): a Rust `storage_root_of(entries)` oracle.
  - Actual finding (4a): `TrieTests/{trietest,trietest_secureTrie,trieanyorder,...}.json` store `[key, value]` where value is a raw string (`"verb"`, `"wookiedoo"`) — NOT RLP-wrapped and not the storage leading-zero-strip transform, and keys are arbitrary-length, not 32-byte slots. **(d) is NOT workable** for Krax's `(B256 slot, B256 value) → keccak256(slot) path + RLP(minimal(value)) leaf` contract. (4b): `alloy_trie::storage_root_unhashed(impl IntoIterator<Item=(B256,U256)>) -> B256` (behind the non-default `ethereum` feature) does exactly `keccak256(slot)` → sort → `HashBuilder::add_leaf(Nibbles::unpack(hashed), alloy_rlp::encode_fixed_size(&value))` — identical semantics to Krax's storage trie. alloy-trie is the crate reth-trie's storage-root path is built on; latest 0.9.5.
  - Source path + line: `ethereum/tests` `TrieTests/trietest_secureTrie.json` (`"in": [["do","verb"], …]`); `~/.cargo/registry/src/.../alloy-trie-0.9.5/src/root.rs:89-118` (`storage_root_unhashed`/`storage_root`).
  - Verbatim quote: `pub fn storage_root_unhashed(storage: impl IntoIterator<Item = (B256, U256)>) -> B256 { storage_root_unsorted(storage.into_iter().map(|(slot, value)| (keccak256(slot), value))) }` and `hb.add_leaf(Nibbles::unpack(hashed_slot), alloy_rlp::encode_fixed_size(&value).as_ref())`.
  - Decision impact: D10 — executed **(e)** via the alloy-trie crate that backs reth-trie's storage-root path (oracle out-of-tree, JSON-only shipped; reth-trie/alloy-trie NEVER a shipped or dev dep). (d) documented unworkable. No STOP.
- **Q5: reth-db cursor API at pinned rev `02d1776` — CONFIRMED.**
  - Library: `reth-db` pinned rev `02d1776786abc61721ae8876898ad19a702e0070`.
  - Query: raw fetch of `crates/storage/db-api/src/{cursor.rs,transaction.rs,common.rs}` at the pinned rev.
  - Expected finding: `DbTx::cursor_read::<T>() -> Result<Cursor, DatabaseError>`; cursor `walk`/`next` yielding ordered `(K,V)`; lifetime composes with our owned `<DatabaseEnv as Database>::TX`.
  - Actual finding: Confirmed. `transaction.rs:21` `trait DbTx: Debug + Send`; `:23` `type Cursor<T: Table>: DbCursorRO<T> + Send`; `:42` `fn cursor_read<T: Table>(&self) -> Result<Self::Cursor<T>, DatabaseError>`. `cursor.rs:13` `trait DbCursorRO<T>`; `:39` `fn walk(&mut self, start_key: Option<T::Key>) -> Result<Walker<'_, T, Self>, DatabaseError>`. `cursor.rs:157-161` `impl Iterator for Walker { type Item = Result<TableRow<T>, DatabaseError>; }` (`common.rs:10` `PairResult<T> = Result<Option<KeyValue<T>>, DatabaseError>`). `cursor_read(&self)` returns an owned cursor; `Walker` borrows `&'cursor mut CURSOR` (the local cursor), so it composes inside an `&self` method that drives `compute_root` to completion before returning — no `'static`/borrow gap (relevant to the Commit 2 audit).
  - Source path + line: reth `02d1776` `crates/storage/db-api/src/transaction.rs:21-46`, `.../cursor.rs:13-44,157-161`, `.../common.rs:10-19`.
  - Verbatim quote: `fn cursor_read<T: Table>(&self) -> Result<Self::Cursor<T>, DatabaseError>;` / `impl<T: Table, CURSOR: DbCursorRO<T>> Iterator for Walker<'_, T, CURSOR> { type Item = Result<TableRow<T>, DatabaseError>; … }`.
  - Decision impact: D8 (a) cursor walk confirmed workable; no D8 (c) fallback needed. Carries to Commit 2 wiring + audit. No STOP.
- **Q6: empty-trie root constant — CONFIRMED.**
  - Library/source: go-ethereum `core/types/hashes.go`; alloy-trie 0.9.5 (`EMPTY_ROOT_HASH`, empty `HashBuilder` root).
  - Query: raw fetch of geth `core/types/hashes.go`; dev-oracle `storage_root_unhashed([])` output.
  - Expected finding: `0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421`.
  - Actual finding: geth `hashes.go:26` `EmptyRootHash = common.HexToHash("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")`; dev oracle's `empty_root` and `empty` vector both = `0x56e81f17…b421`. Matches `trie.rs::EMPTY_ROOT` bytes exactly (asserted by `empty_trie_root_is_empty_root_constant`).
  - Source path + line: `go-ethereum/core/types/hashes.go:26`; fixture `mpt_roots.json` `"empty_root"`.
  - Verbatim quote: `EmptyRootHash = common.HexToHash("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")`.
  - Decision impact: D11 (c) confirmed. No STOP.

### Verification table results

| # | Result | Evidence |
|---|---|---|
| 1 | PASS | `make build` → `Finished \`release\` profile … in 10.82s`; exit 0. |
| 2 | PASS | `make lint` → exit 0; clean. Two clippy firings fixed during execution (not suppressed): `type_complexity` (test helper → `type FixtureVector`/`Entries` aliases) and `large_enum_variant` (`Node::Branch` children → `Box<[Option<NodeRef>;16]>`, clippy's recommended fix). |
| 3 | PASS | `cargo test -p krax-state --lib trie::tests` → 8 passed (empty / empty-path-matches-constant / single / two-diverging / shared-prefix+multibyte+many / inline-child / fixture-file / zero-excluded). |
| 4 | PASS | `make test` → krax-state lib 12 (4 preexisting `mpt::tests` + 8 new `trie::tests`); krax-types 14; doctests: 2 `compile_fail` ok + 1 ignored. Preexisting counts preserved. |
| 5 | **PASS (pre-existing failure did NOT recur — favorable deviation)** | `make coverage` exit 0; workspace TOTAL **88.60% lines** (≥ 85). Commit 1's well-tested `trie.rs` (209 lines, 93.78%) lifted workspace-total above the `--fail-under-lines 85` gate, so the documented bin-driven (`bin/*/main.rs` 0%) failure did NOT fire this step. D15 hold-only satisfied (improved, not regressed). **No `--ignore-filename-regex` change** (dispatch forbids; not needed). Per-crate: krax-state ≈92.6% (mod.rs 90.00% / trie.rs 93.78%), krax-types 85.0% (state.rs 0% = preexisting `StateError::Released` arm per 1.4 D7, untouched). |
| 6 | PASS | `grep -n '^alloy-rlp' Cargo.toml` → one match (Ethereum-types group, line 75). |
| 7 | PASS | `grep -n 'alloy-rlp' crates/krax-state/Cargo.toml` → line 16 under `[dependencies]`, not `[dev-dependencies]`. |
| 8 | PASS | `grep -c 'alloy-rlp' crates/krax-types/Cargo.toml` → 0. |
| 9 | PASS | `grep -n '^tracing' crates/krax-state/Cargo.toml` → line 23 under `[dependencies]`. |
| 10 | PASS | `grep -n 'alloy-rlp' AGENTS.md` → line 367 (Rule 10 alloy grouping). |
| 11 | PASS | `pub(super) const EMPTY_ROOT` (trie.rs:76) + `pub fn compute_root` (trie.rs:298) both present. |
| 12 | PASS | `EMPTY_ROOT` bytes = `0x56e81f17…b421` (LVP-Q6); asserted by `empty_trie_root_is_empty_root_constant` + matches fixture `empty_root`. |
| 13 | PASS | `grep -n '^mod trie;' crates/krax-state/src/mpt/mod.rs` → line 76. |
| 14 | PASS | `// TODO Step 1.5` marker + `B256::ZERO` still in `MptState::root` (mod.rs:184); placeholder untouched. |
| 15 | PASS | `git diff -- crates/krax-types/src/snapshot.rs` → empty (Commit 1 does not edit krax-types). |
| 16 | PASS | `wc -l crates/krax-state/src/mpt/trie.rs` → 472 (≤ 500; no split needed). |
| 17 | PASS | LVP block populated Q1–Q6; no STOP/AUDIT-GAP. |
| 18 | PASS | `grep -rnE '^reth-trie\|^alloy-trie' Cargo.toml crates/*/Cargo.toml` → none. |
| 19 | PASS | `grep -cE '^proptest' crates/krax-state/Cargo.toml` → 0. |
| 20 | PASS | `git status --porcelain` → no new `crates/<name>/Cargo.toml` (only `trie.rs` + `tests/fixtures/` in the existing crate). |
| 21 | PASS | `grep -c '\`\`\`compile_fail' crates/krax-types/src/snapshot.rs` → 1 (1.4 doctest untouched in Commit 1). |
| 22 | PASS | Full Commit-1 diff reviewed: only deps + `trie.rs` + `mod trie;` + AGENTS.md Rule 10 + fixture JSON. No per-account/world trie, no proof gen, no ZK hashes, no sidecar nodes table, no `alloy-trie`/`reth-trie` shipped, no new crates, no `State`/`Snapshot` trait edits, no `state.rs` edits. |

Summary: 22/22 PASS (row 5 passed outright — the documented pre-existing bin-driven failure did not recur because `trie.rs` lifted workspace-total coverage above the gate; no masking change made).

### Deviations from plan

- **AGENTS.md Rule 10 edit landed in Commit 1, not Commit 2.** The plan placed the Rule 10 `alloy-rlp` edit under Commit 2 Step 2.7, but **D4 = (a)** freezes "add `alloy-rlp` … + AGENTS.md Rule 10 … in the SAME commit (1.3b `tempfile` precedent)", the dispatch explicitly states "alloy-rlp addition lands in Commit 1 in the same commit as the workspace + per-crate Cargo.toml additions", and the plan's OWN Commit 1 verification row 10 expects it in Commit 1. Intent unambiguous → executed in Commit 1. (Commit 2's cumulative row 38 still holds since the edit persists.)
- **Inline-vs-hash threshold prose `≤ 32` → `< 32`.** Decisions/plan prose said "≤ 32 inline / > 32 hash"; the authoritative source (go-ethereum `trie/hasher.go:68` `if len(enc) < 32`) is strictly `< 32` inline / `>= 32` hash (a 32-byte encoding is hashed). Implemented per spec — this IS the go-ethereum match D9 names. Empirically validated: all 7 alloy-trie oracle fixtures match.
- **`compute_root` hash-then-sort clarification.** The plan's `trie.rs` doc-comment draft said "Entries MUST arrive in ascending key order (the MDBX cursor … provides this naturally)". That is imprecise: the trie is a *secure* trie keyed by `keccak256(slot)`, so raw-slot cursor order ≠ trie-path order. `compute_root` internally hashes every key and re-sorts via a `BTreeMap` (Rule 7 — BTreeMap, never HashMap). D7 (b) sort-then-build inherently buffers; expected and within Phase 1 scope. Doc-comment authored to state the correct contract.
- **Zero-value exclusion (eth_storageRoot).** `compute_root` skips entries whose value is `B256::ZERO` (Ethereum deletes zeroed storage slots; reth's `StorageRoot`/the oracle filter identically). The plan did not call this out explicitly; it is required for the decisions doc's stated `eth_storageRoot` semantics. Covered by the `with_zero_value_entry` fixture and the `zero_value_slots_are_absent` unit test.
- **D9 inline-child tested via the internal helper, not the public surface.** Secure-trie keccak256 32-byte keys make every `compute_root` node's RLP ≥ 32 bytes (deep ~64-nibble paths), so the inline branch is unreachable through `compute_root`. The plan's own Step 1.5 note authorized hand-construction; the load-bearing inline-vs-hash decision is exercised directly via `inline_encoded_child_vector` on `NodeRef::from_encoding` / `payload` (including the exactly-32-bytes → Hash boundary). This tests the precise code path D9 targets.
- **`#![allow(dead_code)]` in `trie.rs` (Commit 1 only).** `compute_root`/`Node`/etc. have no non-test caller until Commit 2 wires them, so `dead_code` would fail `make lint`'s `-D warnings`. The plan did not specify a guard but its own row 2 requires lint-clean while row 14 requires the wiring to stay deferred. Added a documented module-inner allow in the new file (keeps the mod.rs edit minimal per plan). **Commit 2 MUST remove this allow** when the wiring makes the items reachable (so it cannot mask real dead code afterward).
- **`serde_json` dev-dep added (plan-anticipated).** Step 1.5 note 2 leaned serde_json (already a Rule-10-approved workspace dep). Chosen; `fixture_file_vectors` parses the JSON via `serde_json::Value`. No approved-dep-list change (not a Rule 10 event).
- **Two clippy fixes during execution (fixed, not suppressed):** `clippy::type_complexity` (test helper return type → `type` aliases) and `clippy::large_enum_variant` (`Node::Branch` array boxed, clippy's own recommended fix; documented at the field). No `#[allow]` added for either. No other Old/New drift.

### Pre-authorized execution-time choices documented

- **D5 `NodeRef` shape:** chose `NodeRef::Inline(Vec<u8>)` (raw child RLP bytes, spliced directly into the parent payload) over `Inline(Box<Node>)`. The lighter variant after writing the encoder — an inline child IS its RLP encoding (LVP-Q3 / geth `w.Write(c)` raw splice); no deferred re-encode needed. Semantically equivalent on-wire.
- **D7 algorithm:** sort-then-build (b). Auxiliary buffering is one `BTreeMap<B256,Vec<u8>>` over the (small Phase-1) slot set; well within bounds — D7 (c) stack-based fallback NOT invoked.
- **D10 fixture source:** (e), executed via the alloy-trie 0.9.5 crate that backs reth-trie's storage-root path (out-of-tree throwaway oracle, JSON-only vendored). (d) canonical ethereum/tests documented unworkable (raw non-RLP values, variable-length keys — incompatible with the `(B256,B256)` secure-storage-trie contract). reth-trie/alloy-trie are NOT shipped or dev deps.
- **Test placement:** unit-vector tests kept INLINE in `trie.rs::tests` (D17 (a) lean); `trie.rs` = 472 lines < 500, no split to `tests/mpt_root.rs` needed.

### Audit outcome (LVP Q1/Q3/Q4/Q5 sufficiency for Commit 1's deliverable)

**Wiring confirmed correct.** All six LVP queries resolved with no STOP/AUDIT-GAP. Q1 (alloy-rlp surface) matches the planner expectation and `default-features = false` is sound. Q3 (spec) found authoritatively in go-ethereum, with the `≤32`→`<32` prose correction documented and empirically validated. Q4 resolved as (e)-via-alloy-trie ((d) documented unworkable). Q5 (cursor API) confirmed and shown to compose with the owned `<DatabaseEnv as Database>::TX` for Commit 2. The deliverable is empirically proven: `compute_root`'s output is byte-identical to alloy-trie 0.9.5's `storage_root_unhashed` across all 7 fixtures (empty, single, two-diverging, shared-prefix/extension+branch, multi-byte-value, 12-entry, zero-excluded) plus the explicit inline-vs-hash and zero-exclusion unit tests. 22/22 verification rows PASS.

### Proposed commit message (final)

```
feat(state): add MPT trie module with sort-then-build root computation — Step 1.5 (1/2)

New crates/krax-state/src/mpt/trie.rs: Ethereum-compatible secure storage
MPT root over the Slots table — Node/NodeRef/Nibbles + sort-then-build
compute_root (keccak256(slot) path, rlp(minimal(value)) leaf, zero-value
exclusion per eth_storageRoot, force-hashed root) + EMPTY_ROOT. RLP via
alloy-rlp. 8 unit-test vectors validated byte-identical to alloy-trie
0.9.5 storage_root_unhashed (D10 (e) dev oracle; JSON fixture vendored,
oracle not in tree). mpt/mod.rs gains `mod trie;` only — MptState::root
stays B256::ZERO (wiring is Commit 2). alloy-rlp added to workspace +
krax-state Cargo.toml and AGENTS.md Rule 10 (D4 — same commit as the
dep); tracing declared in krax-state; serde_json dev-dep for fixtures.

Inline-vs-hash uses strict <32 (go-ethereum trie/hasher.go:68); plan
prose said ≤32 — corrected to match the spec D9 names. LVP Q1–Q6 in
docs/plans/step-1.5-plan.md Commit 1 Outcomes.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

I did NOT run git commit.

---

# Commit 2: feat(state,types): wire MPT root through Snapshot::root + State::root — Step 1.5 (2/2)

## Purpose

Wire the `compute_root` function from Commit 1 into `MptState::root` and `MptSnapshot::root`. **This is the Rule 8 trait-surface change** — `Snapshot::root` is added to the trait in `krax-types/src/snapshot.rs` per Decision 1 = (a). Memoization fields land: `cached_root: Option<B256>` on `MptState` (invalidated on `set`, populated by `commit`) and `cached_root: OnceLock<B256>` on `MptSnapshot` (populated on first `root()` call). Three new integration tests in `tests/snapshot_isolation.rs` assert root-isolation alongside the existing get-isolation cases (D17 (a)). ARCHITECTURE.md Step 1.5 fully closes; the Phase 1 Gate "Real MPT root computation in place" line item closes; AGENTS.md Current State, Domain Concepts, and Changelog Session 18 land.

## Cross-step reconciliation — Rule 8 trait edit on `Snapshot`

The 1.1a plan precedent (`docs/plans/archive/step-1.1a-trait-interfaces.md`) and AGENTS.md Rule 8 require that trait surface changes be flagged as deliberate phase-planning decisions. Decision 1 = (a) is that decision: `fn root(&self) -> B256` is added to `Snapshot`. **Implications:**

- All current `Snapshot` implementors must add a `root` method. **Today the only implementor is `MptSnapshot`** (verified by `grep -rn 'impl Snapshot' crates/`). Step 2.x onward (V2 LSM snapshot) inherits the requirement.
- The 1.4 `compile_fail` doctest on `Snapshot::release` (lines 29–40 of `crates/krax-types/src/snapshot.rs`) contains an `impl Snapshot for S` stub that today implements only `get` + `release`. **The doctest's stub MUST be extended in this commit to add a `root` method** (or the doctest fails to compile under the new trait surface — which is the load-bearing invariant 1.4 set up). The minimal extension is `fn root(&self) -> B256 { B256::ZERO }`.
- The `compile_fail` annotation itself remains — the load-bearing failure case (`drop(s);` after `s.release();`) is unchanged. The trait-stub extension is purely to satisfy the trait surface so the OTHER lines in the doctest can fail-to-compile for the right reason (the post-release use, not a missing trait method).

## Execution Steps

### Step 2.1 — Add `fn root` to the `Snapshot` trait + extend the existing `compile_fail` doctest (D1 + cross-step reconciliation)

**File:** `crates/krax-types/src/snapshot.rs`

**Old (lines 16–42, current HEAD):**

```rust
pub trait Snapshot: Send + Sync {
    /// Returns the value of `slot` at the snapshot's commit point.
    fn get(&self, slot: B256) -> Result<B256, StateError>;

    /// Releases this snapshot, consuming it.
    ///
    /// Post-release reads on the same handle are a compile-time error, not a
    /// runtime check. The receiver `self: Box<Self>` is consumed; any subsequent
    /// use of the original `Box<dyn Snapshot>` triggers E0382 ("borrow of moved
    /// value"). Verified by the `compile_fail` doctest below (Step 1.4
    /// Decisions 3 + 4 — `compile_fail` doctest only, hosted on the trait method;
    /// trait-level stub keeps the doctest free of `krax-state` and `tempfile`):
    ///
    /// ```compile_fail
    /// # use alloy_primitives::B256;
    /// # use krax_types::{Snapshot, StateError};
    /// struct S;
    /// impl Snapshot for S {
    ///     fn get(&self, _slot: B256) -> Result<B256, StateError> { Ok(B256::ZERO) }
    ///     fn release(self: Box<Self>) {}
    /// }
    /// let s: Box<dyn Snapshot> = Box::new(S);
    /// s.release();
    /// drop(s); // error[E0382]: use of moved value: `s`
    /// ```
    fn release(self: Box<Self>);
}
```

**New:**

```rust
pub trait Snapshot: Send + Sync {
    /// Returns the value of `slot` at the snapshot's commit point.
    fn get(&self, slot: B256) -> Result<B256, StateError>;

    /// Returns the MPT root as of the snapshot's commit point.
    ///
    /// The returned root reflects the state visible to this snapshot —
    /// NOT the live state, and NOT any post-snapshot commits made on the
    /// underlying [`State`][crate::State]. Implementations MAY cache the
    /// computed root for the snapshot's lifetime (the V1 MDBX backend's
    /// [`MptSnapshot`][^MptSnapshot] does — Step 1.5 Decision 3 (b)).
    ///
    /// Infallible by design (Step 1.5 Decisions 12 (d) + 14 (a)): an
    /// internal storage-read failure during root computation is
    /// unrecoverable for the surrounding commit pipeline. Implementations
    /// MAY `panic!` on storage corruption after emitting
    /// `tracing::error!`. V1 callers must NOT call `root` against a
    /// snapshot whose underlying storage is suspected corrupt.
    ///
    /// [^MptSnapshot]: defined in `krax-state/src/mpt/mod.rs`; not
    /// importable from `krax-types` to avoid a backend dep.
    fn root(&self) -> B256;

    /// Releases this snapshot, consuming it.
    ///
    /// Post-release reads on the same handle are a compile-time error, not a
    /// runtime check. The receiver `self: Box<Self>` is consumed; any subsequent
    /// use of the original `Box<dyn Snapshot>` triggers E0382 ("borrow of moved
    /// value"). Verified by the `compile_fail` doctest below (Step 1.4
    /// Decisions 3 + 4 — `compile_fail` doctest only, hosted on the trait method;
    /// trait-level stub keeps the doctest free of `krax-state` and `tempfile`):
    ///
    /// ```compile_fail
    /// # use alloy_primitives::B256;
    /// # use krax_types::{Snapshot, StateError};
    /// struct S;
    /// impl Snapshot for S {
    ///     fn get(&self, _slot: B256) -> Result<B256, StateError> { Ok(B256::ZERO) }
    ///     fn root(&self) -> B256 { B256::ZERO }
    ///     fn release(self: Box<Self>) {}
    /// }
    /// let s: Box<dyn Snapshot> = Box::new(S);
    /// s.release();
    /// drop(s); // error[E0382]: use of moved value: `s`
    /// ```
    fn release(self: Box<Self>);
}
```

**Rationale:** D1 (a) — the Rule 8 trait change. The doctest's stub gains one line (`fn root(&self) -> B256 { B256::ZERO }`); the load-bearing `drop(s);` post-release-move trigger is unchanged. The `Send + Sync` supertrait, the object-safety assertion at the bottom of the file (`const _: Option<&dyn Snapshot> = None;`), and the `release` method's surface are all preserved verbatim. The new `root` method is placed between `get` and `release` — symmetric with `State::root` (which also sits between `get`/`set`/`snapshot`/`commit` and is the last method on its trait, but the krax-types convention does not require a particular order; placing `root` adjacent to `get` is the natural read-only-method grouping).

### Step 2.2 — Document panic surface on `State::root` (D12 d)

**File:** `crates/krax-types/src/state.rs`

**Old (lines 67–70, current HEAD):**

```rust
    /// Returns the current state root without committing pending writes.
    ///
    /// Concrete implementations may return a cached value.
    fn root(&self) -> B256;
```

**New:**

```rust
    /// Returns the current state root without committing pending writes.
    ///
    /// Concrete implementations may return a cached value.
    ///
    /// # Panics
    ///
    /// The trait signature is infallible (Step 1.5 Decision 12 (d)).
    /// Implementations MAY `panic!` on unrecoverable internal storage
    /// failure after emitting `tracing::error!`. The V1 MDBX-backed
    /// implementation
    /// ([`MptState::root`][^MptState] in `krax-state`) panics on cursor
    /// or txn errors during the slot scan — these are unrecoverable for
    /// the surrounding commit pipeline. Callers must NOT invoke `root`
    /// against a state whose backing storage is suspected corrupt.
    ///
    /// [^MptState]: defined in `krax-state/src/mpt/mod.rs`; not
    /// importable from `krax-types` to avoid a backend dep.
    fn root(&self) -> B256;
```

**Rationale:** D12 (d). The **trait signature is unchanged** (still `fn root(&self) -> B256` — no `Result` wrapping, no Rule 8 change to `State`). Only the doc comment is extended. The `# Panics` section is conventional Rust documentation; AGENTS.md `missing_panics_doc = "allow"` means this doc is voluntary, but the dispatch instructs that the panic surface be "documented in the doc comment" — this is that documentation.

**Out-of-scope guardrail (decisions doc, post-D14):** "Edits to `state.rs` beyond the doc-comment update on `State::root` per D12 (d)" — NONE. `StateError` is unchanged; the `State` trait surface is unchanged; the `Snapshot` import is unchanged.

### Step 2.3 — Wire `MptState::root` + `MptState::set` + `MptState::commit` (D2, D8, D12, D19)

**File:** `crates/krax-state/src/mpt/mod.rs`

**Old (lines 99–189, current HEAD — the `MptState` struct + its `State` impl):** the existing `MptState { env: Arc<DatabaseEnv> }` struct and its `impl State`. Specifically the placeholder `root()` at lines 182–188, the `set()` at 157–165, and the `commit()` at 175–180.

**Procedure (Old/New shown for the load-bearing edits; the rest is mechanical):**

1. **`MptState` struct.** Add a `cached_root: Option<B256>` field (D2 (b)). Wrap in a `RwLock` or `Mutex` if `&self`-call-sites need to mutate it; otherwise `&mut self`-call-sites (i.e. `set`, `commit`) are the only mutation paths and `root(&self)` is a pure-read of a `&self` field — **but `root(&self)` IS called on `&self`, not `&mut self`, so the cache cannot be populated from `root(&self)` itself unless wrapped in interior mutability.** Per the AGENTS.md Rule 2 "no global state" rule (no `OnceLock` for *global* mutable state), instance-level `OnceLock` IS allowed (1.4 precedent: `OnceLock` is what D3 (b) prescribes for `MptSnapshot`). **Coder action:** the most ergonomic shape is `cached_root: OnceLock<B256>` on `MptState` — `set` calls `self.cached_root.take()` (requires `OnceLock::take(&mut self)` — available on stable per LVP-Q1 prep; if not, swap with `std::sync::OnceLock` + `&mut`-borrow on `set` AND `commit`); `commit` calls `let _ = self.cached_root.set(new_root);`; `root(&self)` returns `*self.cached_root.get_or_init(|| self.compute_root_from_storage())`. **Document the chosen interior-mutability shape in Outcomes.** Alternative: `cached_root: std::cell::Cell<Option<B256>>` works for `Copy` types like `B256` but adds `Sync` complications (`Cell` is NOT `Sync`); the `State: Send + Sync` supertrait requires `Sync`, so `Cell` is OUT. Pick `OnceLock<B256>` (per D3 (b) precedent on `MptSnapshot` — symmetry).

2. **`set` invalidates the cache.** Add `self.cached_root = OnceLock::new();` (or equivalent `take`) at the top of `set`, before the txn open.

3. **`commit` populates the cache (D19 (a)).** After the existing "sync barrier" return-of-`self.root()`, ensure the post-commit `root` value is stored in the cache. The cleanest shape is: `let r = self.compute_root_from_storage(); let _ = self.cached_root.set(r); Ok(r)`. If `cached_root` already has a value (race with a concurrent reader between `set`'s `take` and `commit`'s `set` — not possible in single-threaded use, but the type wants to be defensive), use `commit`'s `&mut self` exclusivity to overwrite via `self.cached_root = OnceLock::from(r);` (or the appropriate constructor for the chosen shape).

4. **`root` body.** Replace the `B256::ZERO` placeholder with:
   ```rust
   *self.cached_root.get_or_init(|| self.compute_root_from_storage())
   ```
   and add a private helper:
   ```rust
   fn compute_root_from_storage(&self) -> B256 {
       // Per Context7 LVP-Q5 (reth-db @ 02d1776, Step 1.5): DbTx::cursor_read::<T>()
       // returns a cursor that walks Slots in key order.
       let tx = self.env.tx().unwrap_or_else(|e| {
           tracing::error!(error = %e, "MDBX read failure in MptState::root");
           panic!("MDBX read failure in MptState::root: {e}");
       });
       let cursor = tx.cursor_read::<Slots>().unwrap_or_else(|e| {
           tracing::error!(error = %e, "MDBX cursor open failure in MptState::root");
           panic!("MDBX cursor open failure in MptState::root: {e}");
       });
       // Iterator over (B256, B256) — decode_slot_value at each step.
       let entries = cursor /* walk + map per LVP-Q5 */ .map(|r| match r {
           Ok((k, v)) => (k, decode_slot_value(&v).unwrap_or_else(|e| {
               tracing::error!(error = %e, "Slots value decode failure in MptState::root");
               panic!("Slots value decode failure in MptState::root: {e}");
           })),
           Err(e) => {
               tracing::error!(error = %e, "MDBX cursor walk failure in MptState::root");
               panic!("MDBX cursor walk failure in MptState::root: {e}");
           }
       });
       trie::compute_root(entries)
   }
   ```
   The exact cursor walk shape (`.walk(None)?`, `.next()` loop, `walk_range`, etc.) depends on LVP-Q5's findings — pick whichever matches the pinned-rev API. The error-handling pattern (`tracing::error!` then `panic!`) is uniform across each fallible call site, per D12 (d). The cursor's `&self`-borrow on the txn (and the txn's `'static` bound from 1.3b LVP) means the cursor outlives the iterator chain; if a `'static` issue surfaces, collect into a `Vec<(B256, B256)>` and pass `entries.into_iter()` to `compute_root` (D8 (c) fallback).

5. **Remove the `// TODO Step 1.5` marker.** The placeholder lines 183–186 disappear entirely.

**Rationale:** D2 (b) + D8 (a) + D12 (d) + D14 (a) + D19 (a). The `compute_root_from_storage` helper is private to `mpt/mod.rs`; `trie::compute_root` is the iterator-stateless function from Commit 1. The `unwrap_or_else` + `panic!` form (rather than `expect()`) lets the `tracing::error!` emit before the panic — `expect()` would format-and-panic in one step without emitting through the structured-logging layer.

### Step 2.4 — Wire `MptSnapshot::root` + add `cached_root: OnceLock<B256>` field (D1, D3)

**File:** `crates/krax-state/src/mpt/mod.rs`

**Old (lines 191–216, current HEAD — the `MptSnapshot` struct + its `Snapshot` impl):**

```rust
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
```

**New:**

```rust
/// MDBX read-only snapshot.
///
/// Owns a reth-db `RoTxn` (Decision 3); reads traverse the txn directly. Drop
/// releases the MDBX reader slot via the txn's `Drop` impl (Decision 11).
///
/// Caches the computed MPT root lazily in `cached_root` (Step 1.5 Decision
/// 3 (b)) — first call to [`Snapshot::root`] walks the slots via the
/// snapshot's RO cursor and populates the cache; subsequent calls return
/// the cached value.
// Drop: relies on `tx`'s auto-Drop, which releases the MDBX reader slot
// (Step 1.4 Decision 13 — RAII; no explicit Drop impl, no explicit abort()).
#[derive(Debug)]
pub struct MptSnapshot {
    tx: <DatabaseEnv as Database>::TX,
    cached_root: std::sync::OnceLock<B256>,
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
        // Step 1.5 Decisions 3 (b) + 8 (a) + 12 (d) + 14 (a): lazy + cache;
        // cursor walk on `self.tx`; infallible — panic on MDBX failure
        // after `tracing::error!`.
        *self.cached_root.get_or_init(|| {
            // Per Context7 LVP-Q5 (reth-db @ 02d1776, Step 1.5): cursor walk
            // on a RO txn iterates Slots in B-tree key order.
            let cursor = self.tx.cursor_read::<Slots>().unwrap_or_else(|e| {
                tracing::error!(error = %e, "MDBX cursor open failure in MptSnapshot::root");
                panic!("MDBX cursor open failure in MptSnapshot::root: {e}");
            });
            let entries = cursor /* walk + map per LVP-Q5 */ .map(|r| match r {
                Ok((k, v)) => (k, decode_slot_value(&v).unwrap_or_else(|e| {
                    tracing::error!(error = %e, "Slots value decode failure in MptSnapshot::root");
                    panic!("Slots value decode failure in MptSnapshot::root: {e}");
                })),
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
```

**Update `MptState::snapshot` to construct the new field:**

```rust
fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
    let tx = self.env.tx().map_err(StateError::io)?;
    Ok(Box::new(MptSnapshot { tx, cached_root: std::sync::OnceLock::new() }))
}
```

**Rationale:** D1 (a) + D3 (b) + D8 (a) + D12 (d). `OnceLock<B256>` provides interior mutability with `Sync` (`Send + Sync` is the `Snapshot` supertrait), correct lazy initialization, and the `*get_or_init(...)` form is idiomatic. The `// Drop: relies on tx's auto-Drop ...` comment from 1.4 is preserved — `OnceLock<B256>` is `Copy`-content and has no resource semantics; only the `tx` field carries the reader slot. The `release` method is unchanged. The cursor walk shape is identical to `MptState::compute_root_from_storage` — both call `trie::compute_root` on a cursor-derived iterator; the only difference is which `DbTx` instance the cursor borrows.

### Step 2.5 — Extend `tests/snapshot_isolation.rs` with three root-isolation cases (D17 a)

**File:** `crates/krax-state/tests/snapshot_isolation.rs`

**Procedure (append three new `#[test]` functions after `two_snapshot_independence`):**

```rust
#[test]
fn root_after_write_does_not_bleed_in() {
    // Step 1.5 D17 (a) case 1: snapshot taken at v1, sibling write to v2,
    // snapshot's root still reflects v1. Mirrors `write_after_snapshot_does_not_bleed_in`
    // but asserts on `Snapshot::root` instead of `Snapshot::get`.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(1), slot(0xAA)).unwrap();

    let snap = state.snapshot().unwrap();
    let root_v1 = snap.root();

    state.set(slot(1), slot(0xBB)).unwrap();

    assert_eq!(snap.root(), root_v1);
    snap.release();
}

#[test]
fn root_after_commit_does_not_bleed_in() {
    // Step 1.5 D17 (a) case 2: snapshot taken at v1, sibling write+commit,
    // snapshot's root still reflects v1.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(2), slot(0x11)).unwrap();

    let snap = state.snapshot().unwrap();
    let root_v1 = snap.root();

    state.set(slot(2), slot(0x22)).unwrap();
    state.commit().unwrap();

    assert_eq!(snap.root(), root_v1);
    snap.release();
}

#[test]
fn two_snapshot_root_independence() {
    // Step 1.5 D17 (a) case 3: snapshot A at v1, sibling write+commit to v2,
    // snapshot B at v2. A's root != B's root; A's root unchanged after B is taken.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(3), slot(0x01)).unwrap();

    let snap_a = state.snapshot().unwrap();
    let root_a = snap_a.root();

    state.set(slot(3), slot(0x02)).unwrap();
    state.commit().unwrap();

    let snap_b = state.snapshot().unwrap();
    let root_b = snap_b.root();

    assert_ne!(root_a, root_b);
    assert_eq!(snap_a.root(), root_a); // A's root is stable; the cache held.
    snap_a.release();
    snap_b.release();
}
```

**Rationale:** D17 (a). Three new test functions appended to `tests/snapshot_isolation.rs` mirroring the three existing get-isolation cases. The `Snapshot` trait is already imported (preexisting line `use krax_types::{Snapshot, State};`). No new imports needed. The `_tmp` `TempDir` binding pattern, the `MptState::open_temporary` helper, and the `slot(n)` helper are all preexisting. **No edits to existing test functions** — Commit 2 strictly appends.

### Step 2.6 — Edit `ARCHITECTURE.md` (D18 a)

**File:** `ARCHITECTURE.md`

**Old (lines 153–168, current HEAD):**

```markdown
### Step 1.5 — MPT Root Computation

Replace the `B256::ZERO` placeholder root in `MptState::root()` with real Ethereum-compatible Merkle Patricia Trie root computation.

- [ ] Decide: `alloy-trie` (external dep) vs custom MPT implementation (decision pre-surfaced in step-1.3a-decisions.md; planner surfaces options properly at 1.5 dispatch)
- [ ] Implement MPT root computation against the chosen approach
- [ ] Root changes deterministically when state changes (table-driven test)
- [ ] Re-run Step 1.4 snapshot tests against real-root MptState (strengthened-tests gate)
- [ ] Update ARCHITECTURE.md and AGENTS.md Current State; remove `// TODO Step 1.5` placeholders from `mpt/mod.rs`

**Phase 1 Gate:**
- ✅ All types in `krax-types` have tests
- ✅ MPT state backend passes round-trip and restart tests
- ✅ Snapshot isolation is enforced and tested
- ✅ Real MPT root computation in place (Step 1.5 ✅)
- ✅ Coverage on `krax-types` and `krax-state` is >85%
```

**New:**

```markdown
### Step 1.5 — MPT Root Computation ✅

Replace the `B256::ZERO` placeholder root in `MptState::root()` with real Ethereum-compatible Merkle Patricia Trie root computation.

- [x] Decide: `alloy-trie` (external dep) vs custom MPT implementation (decision pre-surfaced in step-1.3a-decisions.md; planner surfaces options properly at 1.5 dispatch)
- [x] Implement MPT root computation against the chosen approach
- [x] Root changes deterministically when state changes (table-driven test)
- [x] Re-run Step 1.4 snapshot tests against real-root MptState (strengthened-tests gate)
- [x] Update ARCHITECTURE.md and AGENTS.md Current State; remove `// TODO Step 1.5` placeholders from `mpt/mod.rs`

**Phase 1 Gate:**
- ✅ All types in `krax-types` have tests
- ✅ MPT state backend passes round-trip and restart tests
- ✅ Snapshot isolation is enforced and tested
- ✅ Real MPT root computation in place (Step 1.5 ✅)
- ✅ Coverage on `krax-types` and `krax-state` is >85%
```

**Rationale:** D18 (a). All five Step 1.5 line items check; heading gains `✅`; the "Real MPT root computation in place" Phase 1 Gate item ALREADY displays `✅` typographically (per the 1.3a/1.3b/1.4 convention — those `✅` markers are goal-state, not literal status), so no text edit is required there beyond the implicit "this is now LITERALLY true."

### Step 2.7 — Edit `AGENTS.md` Rule 10 (D4 + Rule 10)

**File:** `AGENTS.md`

**Procedure:**

1. Locate the Rule 10 approved-root-dependencies list (lines ~365–378; the alloy line is at line 367 per the grep snapshot).
2. Add `alloy-rlp` to the alloy grouping. Specifically, change line 367 from:
   ```
     - `alloy` (`alloy-primitives`, `alloy-rpc-types`, `alloy-sol-types`) — Ethereum types and ABI
   ```
   to:
   ```
     - `alloy` (`alloy-primitives`, `alloy-rpc-types`, `alloy-sol-types`, `alloy-rlp`) — Ethereum types, ABI, and RLP encoding
   ```
3. **OR** add `alloy-rlp` as a sibling bullet if the coder prefers the line stays terse — either presentation is acceptable per the 1.3b `tempfile` precedent. Document the chosen presentation in Outcomes.

**Rationale:** D4 (a). The 1.3b `tempfile` precedent (line 378's `Test-only: ..., tempfile` addition in Session 15) shows that dep additions land in the same commit as the dep's first use; Step 1.5 follows that pattern. `alloy-rlp` is part of the alloy family — it's not a Test-only dep.

### Step 2.8 — Edit `AGENTS.md` Current State (D18 a)

**File:** `AGENTS.md`

**Procedure (literal Old/New text omitted; coder writes full-body replacement at execution time per the 1.3b / 1.4 convention; the structural edits below are required):**

1. **Top-of-section line.** Locate the line that currently reads `**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.4 complete; Step 1.5 next).` and replace with:
   ```
   **Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.5 complete; **Phase 1 Gate satisfied** — Phase 2 next).
   ```
2. **Insert a new "What was just completed (Step 1.5 — MPT Root Computation, shipped <YYYY-MM-DD>):"** paragraph as the first "What was just completed" block (above the existing Step 1.4 block). Two-commit summary covers:
   - (a) **Commit 1:** new `crates/krax-state/src/mpt/trie.rs` (Node enum, NodeRef enum, Nibbles helper, sort-then-build `compute_root`, RLP encoding via `alloy-rlp`, `EMPTY_ROOT` constant, unit test vectors); `alloy-rlp` workspace + per-crate dep additions; `tracing` declared in `krax-state`; Rule 10 list updated; static fixture file at `crates/krax-state/tests/fixtures/mpt_roots.json` (D10 path — coder records which path).
   - (b) **Commit 2:** `Snapshot::root` added to the trait (Rule 8 — second-ever change to the Snapshot surface; 1.4 doctest extended); `State::root` doc comment extended with the `# Panics` section; `MptState::root` + `MptSnapshot::root` wired through `trie::compute_root`; memoization fields land (`OnceLock<B256>` on both `MptState` and `MptSnapshot`); three new root-isolation tests in `tests/snapshot_isolation.rs`; ARCHITECTURE.md Step 1.5 closed; Phase 1 Gate "Real MPT root computation in place" satisfied.
   - (c) **Coverage delta.** Record measured pre/post percentages per D15 (a).
   - (d) **LVP findings.** Cite Q1 (alloy-rlp surface), Q3 (spec details), Q5 (cursor API), Q6 (empty-root constant). Q2, Q4 referenced.
3. **"What to do next" block.** Replace the current Step-1.5 entry (currently item 1 of "What to do next" — line ~697) with a Phase-2 entry. Refer to ARCHITECTURE.md Phase 2 (Step 2.1 — EVM Wrapper, lines 176–195) for the next-action prose.
4. **Notes section additions.** Append a note documenting:
   - The `Snapshot` trait now has THREE methods (`get`, `root`, `release`); the `compile_fail` doctest's stub was extended to keep compiling.
   - `MptState` and `MptSnapshot` both carry `OnceLock<B256>` for root memoization; the cache is per-state and per-snapshot (does NOT cross).
   - The `compute_root_from_storage` helper inside `mpt/mod.rs` panics on MDBX failure (D12 (d)); document the four panic sites (txn open, cursor open, cursor walk, slot decode) for the next reader.
   - Keep all existing notes intact.

**Rationale:** D18 (a). Standard close, two-commit narrative.

### Step 2.9 — Edit `AGENTS.md` Domain Concepts (D18 a)

**File:** `AGENTS.md`

**Procedure:**

1. **Verify before adding.** `grep -n "MPT\|Trie Node\|Storage Root" AGENTS.md` — the Domain Concepts section currently (per the planning sweep) does NOT have these entries. The 273-line Domain Concepts block ends at line ~290; insertion point is between "Lookahead Depth" and the `---` separator at line 291.
2. **Append three new entries to Domain Concepts (preserve alphabetical / topical grouping):**
   ```markdown
   - **MPT (Merkle Patricia Trie)** — the hexary Ethereum-compatible commitment trie over `(keccak256(slot_key) → RLP(slot_value))` entries that produces the V1 storage root. V1's MPT is built fresh per `root()` call (Step 1.5 Decision 7 (b) sort-then-build); V2's LSM backend will replace it with a log-structured commitment.
   - **Trie Node** — one of three Ethereum MPT node kinds: Leaf (terminator nibble path + value), Extension (shared-prefix nibble path + child reference), Branch (17-element: 16 child refs + optional value slot). Inline-encoded when RLP ≤ 32 bytes; hash-referenced when > 32 bytes (Step 1.5 Decision 9).
   - **Storage Root** — the root hash of the MPT computed over the Slots table at a single commit point. V1 is chain-global slots only (not per-account); V2 may shard. Returned by `State::root` and `Snapshot::root`.
   ```
3. If a check reveals an entry already exists for any of the three, skip that entry and document in Outcomes.

**Rationale:** D18 (a). The dispatch authorizes adding Domain Concepts entries for "MPT", "Trie Node", "Storage Root" if not already present; the grep verification is the load-bearing pre-check.

### Step 2.10 — Append `AGENTS.md` Changelog Session 18 entry at BOTTOM (D18 a)

**File:** `AGENTS.md`

**Procedure:**

1. Read the current bottom of `AGENTS.md`. The most recent entry is `### Session 17 — Step 1.4: Snapshot Semantics`. The new entry is therefore `### Session 18 — Step 1.5: MPT Root Computation`.
2. Append (do NOT insert above existing entries) a new entry at the absolute bottom of the file, in the same shape as Sessions 15, 16, 17:
   ```markdown
   ### Session 18 — Step 1.5: MPT Root Computation
   **Date:** <YYYY-MM-DD>
   **Agent:** Claude Code (claude-opus-4-7)
   **Summary (two commits — see commit messages below):**
   <prose covering both commits — Commit 1 lands the trie module + alloy-rlp dep + Rule 10 update + fixture file; Commit 2 lands the Snapshot::root trait change + MptState/MptSnapshot wiring + memoization fields + three new root-isolation tests + the ARCHITECTURE.md + AGENTS.md closes. Cite LVP findings Q1, Q3, Q5, Q6. Note Phase 1 Gate satisfaction.>
   **Commit messages (two commits):**
   1. `feat(state): add MPT trie module with sort-then-build root computation — Step 1.5 (1/2)`
   2. `feat(state,types): wire MPT root through Snapshot::root + State::root — Step 1.5 (2/2)`
   ```
3. After append, run `tail -1 AGENTS.md` and confirm the Session 18 commit-message line is the last line of the file (the convention).

**Rationale:** D18 (a). Per-session changelog convention; insertion-at-bottom is load-bearing per the AGENTS.md Changelog header.

## Verification suite — Commit 2 scope (and CUMULATIVE post-Commit-1)

All Commit 1 rows (1–22) must still pass on the cumulative tree. New rows for Commit 2:

| # | Item | Command / Procedure | Expected Result |
|---|---|---|---|
| 23 | `Snapshot::root` exists on trait | `grep -nE 'fn root\(&self\) -> B256;' crates/krax-types/src/snapshot.rs` | one match (the new method declaration) |
| 24 | `Snapshot::release` `compile_fail` doctest stub extended | `grep -nA1 'fn release\(self: Box<Self>\) \{\}' crates/krax-types/src/snapshot.rs` AND inspect the doctest block | the stub `impl Snapshot for S` now contains a `fn root(&self) -> B256 { B256::ZERO }` line; the `drop(s);` trigger preserved |
| 25 | `State::root` doc comment includes `# Panics` section | `grep -nA6 'fn root\(&self\) -> B256;' crates/krax-types/src/state.rs` | `# Panics` heading present in the doc comment ABOVE the method; trait signature unchanged |
| 26 | `MptState::root` no longer returns `B256::ZERO` placeholder | `grep -n 'B256::ZERO' crates/krax-state/src/mpt/mod.rs` | the only matches are in `State::get` / `Snapshot::get` (existing empty-key returns), NOT in `MptState::root` body; `// TODO Step 1.5` marker is gone |
| 27 | `MptSnapshot` has `cached_root: OnceLock<B256>` field | `grep -nA1 'cached_root:' crates/krax-state/src/mpt/mod.rs` | matches in BOTH `MptState` and `MptSnapshot` struct bodies (per D2 (b) + D3 (b)) |
| 28 | Panic-surface present in both root impls | `grep -nE 'tracing::error!.*MDBX.*root\|panic!.*MDBX.*root' crates/krax-state/src/mpt/mod.rs` | ≥ 4 `tracing::error!` matches and ≥ 4 `panic!` matches (txn open, cursor open, cursor walk, slot decode — each in `MptState::root` and `MptSnapshot::root`) |
| 29 | `MptSnapshot::root` returns non-zero for non-empty state | sanity-check via integration tests (rows 32–34 below) | tests pass |
| 30 | Workspace builds | `make build` | exit 0 |
| 31 | Lint clean | `make lint` | exit 0 |
| 32 | Unit tests pass | `make test` | exit 0; all preexisting unit tests preserved; `Snapshot::release` `compile_fail` doctest still reports `ok` (per the extended stub) |
| 33 | Integration tests pass | `make test-integration` | exit 0; preexisting 2 restart + 3 snapshot_isolation tests + the **3 new** root-isolation tests (`root_after_write_does_not_bleed_in`, `root_after_commit_does_not_bleed_in`, `two_snapshot_root_independence`) all pass |
| 34 | Coverage hold-only (D15 a) | `make coverage` | per-crate `krax-types` and `krax-state` ≥ 85%; document workspace-total percentage in Outcomes; the pre-existing `bin/*/main.rs` 0% lines may continue to fail the workspace-total threshold (same as 1.4) — D15 is hold-only, NOT lift |
| 35 | ARCHITECTURE.md Step 1.5 fully closed | `grep -nA8 '### Step 1.5 — MPT Root Computation' ARCHITECTURE.md` | heading carries `✅`; five `- [x]` checkboxes; Phase 1 Gate "Real MPT root" `✅` preserved |
| 36 | AGENTS.md Current State reflects Step 1.5 complete + Phase 1 Gate satisfied | `grep -n 'Step 1.5 complete' AGENTS.md` AND `grep -n 'Phase 1 Gate satisfied' AGENTS.md` | both match |
| 37 | AGENTS.md Domain Concepts contains MPT / Trie Node / Storage Root | `grep -nE '\*\*MPT\b\|\*\*Trie Node\b\|\*\*Storage Root\b' AGENTS.md` | three matches in the Domain Concepts section (~lines 280–290 + insertions) |
| 38 | AGENTS.md Rule 10 contains `alloy-rlp` | `grep -n 'alloy-rlp' AGENTS.md` | at least one match in Rule 10 (line ~365–378 area) AND one match in Current State or Changelog (the Session 18 dep-addition note) |
| 39 | AGENTS.md Changelog Session 18 at BOTTOM | `tail -40 AGENTS.md \| grep -n '### Session 18 — Step 1.5'` AND `tail -1 AGENTS.md` | Session 18 appears in `tail -40`; the very last line of the file is part of the Session 18 entry (the commit-message line per the convention) |
| 40 | `Snapshot::release` `compile_fail` doctest still reports `compile fail ... ok` | `cargo test --doc -p krax-types 2>&1 \| grep -E '- compile fail .* ok'` | both `Journal::discard` and `Snapshot::release` lines appear (the 1.4 invariant is preserved across the trait-stub extension) |
| 41 | No new crates created (D14) | `git status --porcelain \| grep -E '^A.*crates/[^/]+/Cargo\.toml$'` | zero matches across both commits |
| 42 | No `reth-trie` / `alloy-trie` in shipped deps (decisions out-of-scope) | `grep -rnE '^(reth-trie\|alloy-trie)' Cargo.toml crates/*/Cargo.toml` | zero matches |
| 43 | No `proptest` added (decisions out-of-scope) | `grep -nE '^proptest' crates/krax-types/Cargo.toml crates/krax-state/Cargo.toml` | zero matches (workspace-level definition is unchanged) |
| 44 | `State` trait signature UNCHANGED (D12 d — only doc edit) | `git diff -- crates/krax-types/src/state.rs \| grep -E '^[+-] *fn '` | zero matches (only doc-comment additions on `root`; signature `fn root(&self) -> B256;` preserved verbatim) |
| 45 | `Snapshot` trait surface gained exactly one method (D1 a) | `git diff -- crates/krax-types/src/snapshot.rs \| grep -E '^\+.*fn '` | exactly one match (`+    fn root(&self) -> B256;`); `get` and `release` signatures preserved |
| 46 | Out-of-scope check (decisions doc, post-D14) | inspect full Commit-2 diff: no per-account state trie, no proof gen, no ZK hashes, no sidecar nodes table, no archive node support, no `alloy-trie` / `reth-trie` shipped, no new crates, no edits to `state.rs` beyond the `# Panics` doc-comment addition | every item passes; if any fails, HALT and re-surface |
| 47 | LVP block populated for Commit 2 | inspect Commit 2 Outcomes → "LVP findings" | references back to Commit 1's Q1/Q2/Q3/Q4/Q5/Q6 findings (no re-verification needed unless a Commit 2 surface emerged that wasn't covered by Commit 1's LVP — e.g. `OnceLock::get_or_init` API; if so, add as Q7 in Commit 2 Outcomes) |

## Commit message — Commit 2

```
feat(state,types): wire MPT root through Snapshot::root + State::root — Step 1.5 (2/2)
```

## Outcomes — Commit 2 (filled in at execution time, 2026-05-15)

### Files changed

- `crates/krax-types/src/snapshot.rs` — added `fn root(&self) -> B256;` to the `Snapshot` trait (Rule 8 surface change, D1 (a)) between `get` and `release`, with a doc comment (D1 + Step 2.1/2.2 prose); extended the 1.4 `compile_fail` doctest's `impl Snapshot for S` stub with `fn root(&self) -> B256 { B256::ZERO }`. `drop(s);` E0382 trigger, `Send + Sync` supertrait, and `const _: Option<&dyn Snapshot> = None;` object-safety assertion all preserved.
- `crates/krax-types/src/state.rs` — `State::root` doc comment gained a `# Panics` section (D12 (d)). **Signature unchanged** (`fn root(&self) -> B256;`); `StateError`/trait surface untouched.
- `crates/krax-state/src/mpt/mod.rs` — imports: `std::sync::OnceLock`, `reth_db::cursor::DbCursorRO` (for the LVP-Q5-confirmed `.walk`). `MptState` + `MptSnapshot` each gained `cached_root: OnceLock<B256>` (D2 (b)/D3 (b)). `MptState::open`/`snapshot` initialize the field; `set` invalidates it; `commit` repopulates it with the post-commit root (D19 (a)); `root` = `*self.cached_root.get_or_init(|| self.compute_root_from_storage())`; new private `compute_root_from_storage` (D8 (a) cursor walk, D12 (d) 4 panic sites). `MptSnapshot::root` added (D1/D3 (b)/D8 (a)/D12 (d), 3 panic sites over the snapshot's held RO txn). `// TODO Step 1.5` placeholder removed; `MptSnapshot` doc comment updated.
- `crates/krax-state/src/mpt/trie.rs` — **Step 2.0 delta:** removed the Commit-1 `#![allow(dead_code)]` + its Commit-1-only comment block (the wiring makes all items reachable).
- `crates/krax-state/tests/snapshot_isolation.rs` — appended three root-isolation tests (D17 (a)): `root_after_write_does_not_bleed_in`, `root_after_commit_does_not_bleed_in`, `two_snapshot_root_independence`. Existing get-isolation tests untouched.
- `ARCHITECTURE.md` — Step 1.5 heading `✅`, five `- [x]`; Phase 1 Gate "Real MPT root computation in place" line already `✅` (goal-state convention — no text edit). (gitignored — `git add -f`.)
- `AGENTS.md` — Current Phase → "Step 1.5 complete; **Phase 1 Gate satisfied** — Phase 2 next"; new "What was just completed (Step 1.5 …)" two-commit block atop the stack; "What to do next" → Phase 2 entry; four Notes bullets added; Domain Concepts gained MPT / Trie Node / Storage Root; Changelog Session 18 appended at BOTTOM (last line = commit-message #2). **Plan Step 2.7 skipped** — alloy-rlp Rule 10 entry already landed in Commit 1 (Delta 1). (gitignored — `git add -f`.)
- `docs/plans/step-1.5-plan.md` — this Commit 2 Outcomes block filled in.
- Cargo.toml / Cargo.lock — **unchanged in Commit 2** (no dep changes; all dep work was Commit 1).

### Verification table results (cumulative — Commit 1 rows 1–22 + Commit 2 rows 23–47)

Commit 1 rows 1–22: re-confirmed on the cumulative tree — all PASS. Row 5 (coverage) is re-evaluated cumulatively at row 34 below (now FAIL-BY-DESIGN — see Coverage delta).

| # | Result | Evidence |
|---|---|---|
| 23 | PASS | `fn root(&self) -> B256;` on the `Snapshot` trait (snapshot.rs:37). |
| 24 | PASS | doctest stub has `fn root(&self) -> B256 { B256::ZERO }` (snapshot.rs:54); `drop(s); // error[E0382]` preserved (snapshot.rs:59). |
| 25 | PASS | `# Panics` in `State::root` doc (state.rs:71); signature `fn root(&self) -> B256;` unchanged (state.rs:83). |
| 26 | PASS | No `B256::ZERO` in `MptState::root` body (now `*self.cached_root.get_or_init(...)`); no `TODO Step 1.5` match anywhere. Remaining `B256::ZERO` matches are the pre-existing `State::get`/`Snapshot::get` empty-key returns + test module. |
| 27 | PASS | `cached_root: OnceLock<B256>` in BOTH `MptState` (mod.rs:110) and `MptSnapshot` (mod.rs:266) structs; constructed/invalidated/repopulated at 122/226/211/236. |
| 28 | PASS | `tracing::error!.*MDBX.*root` ×9, `panic!.*MDBX.*root` ×9 (≥4 each): 5+5 in `MptState::compute_root_from_storage` (txn open, cursor open, walk, decode, Err-arm), 4+4 in `MptSnapshot::root` (cursor open, walk, decode, Err-arm). |
| 29 | PASS | `MptSnapshot::root` returns non-zero for non-empty state — `two_snapshot_root_independence` asserts `root_a != root_b` (distinct real roots) and root stability; all 3 root-isolation integration tests pass. |
| 30 | PASS | `make build` exit 0. |
| 31 | PASS | `make lint` exit 0; clean (no `dead_code` after Step 2.0 allow removal — `cargo build -p krax-state` confirmed zero residual). |
| 32 | PASS | `make test` exit 0; krax-state lib 12 + krax-types 14; doctests: `Snapshot::release - compile fail … ok` (now line 48) + `Journal::discard - compile fail … ok` + 1 ignored. |
| 33 | PASS | `make test-integration` exit 0; restart 2 + `snapshot_isolation` **6** (3 preexisting get-isolation + 3 new root-isolation). |
| 34 | **FAIL (BY DESIGN, pre-existing — documented, NOT masked)** | `make coverage` exit 2 — workspace-total **82.64% lines** < `--fail-under-lines 85`. Per-crate Phase 1 Gate targets HOLD: **krax-state 85.15%** (357 lines, 53 missed: mod.rs 72.97% / trie.rs 93.78%), **krax-types 85.0%** (unchanged from 1.4). Failure is bin-driven (`bin/*/main.rs` 12 lines @ 0%, unchanged since 1.3.5) PLUS the new D12 (d)-mandated defensive panic arms (7 sites, untestable without out-of-scope MDBX fault injection). Exactly the dispatch-predicted recurrence. **No `--ignore-filename-regex` change** (dispatch forbids; D15 hold-only). See Coverage delta. |
| 35 | PASS | `### Step 1.5 — MPT Root Computation ✅` (ARCHITECTURE.md:153); 5 `- [x]`; "Real MPT root computation in place (Step 1.5 ✅)" gate line present (goal-state ✅ per 1.3a/1.3b/1.4 convention). |
| 36 | PASS | `Step 1.5 complete` + `Phase 1 Gate satisfied` both in AGENTS.md Current State. |
| 37 | PASS | Domain Concepts has `**MPT (Merkle…`, `**Trie Node`, `**Storage Root` (3 matches, none pre-existing — grep-verified absent first). |
| 38 | PASS | `alloy-rlp` ×3 in AGENTS.md — Rule 10 line (Commit 1, persists) + Current State + Changelog Session 18. |
| 39 | PASS | `tail -1 AGENTS.md` = `2. \`feat(state,types): wire MPT root through Snapshot::root + State::root — Step 1.5 (2/2)\`` (Session 18's last line; Session 18 present in tail). |
| 40 | PASS | `cargo test --doc -p krax-types`: `Journal::discard (line 52) - compile fail … ok` AND `Snapshot::release (line 48) - compile fail … ok` (1.4 invariant preserved across the stub extension; line moved 29→48 due to the added `root` doc). |
| 41 | PASS | No new `crates/<name>/Cargo.toml`. |
| 42 | PASS | No `reth-trie`/`alloy-trie` in `Cargo.toml`/`crates/*/Cargo.toml`. |
| 43 | PASS | No `proptest` in `krax-types`/`krax-state` Cargo.toml. |
| 44 | PASS | `git diff -- state.rs \| grep '^[+-] *fn '` → none (only `# Panics` doc additions; signature verbatim). |
| 45 | PASS (intent; grep-literalism noted) | `Snapshot` trait body gained EXACTLY ONE method: `+    fn root(&self) -> B256;`. The row's raw grep also matches `+    ///     fn root(&self) -> B256 { B256::ZERO }` — the doctest-stub line that **plan Step 2.1 + the cross-step reconciliation explicitly mandate**. Excluding `///` doctest lines, exactly one `fn` added to the trait. Object-safety assertion intact (snapshot.rs:66). |
| 46 | PASS | Commit-2 diff = snapshot.rs (1 trait method + doc + mandated doctest stub), state.rs (doc only), mod.rs (wiring + memo fields), snapshot_isolation.rs (3 appended tests), trie.rs (allow removed), ARCHITECTURE.md/AGENTS.md docs. No per-account/world trie, no proof gen, no ZK hashes, no sidecar nodes table, no archive, no alloy-trie/reth-trie shipped, no new crates, no `state.rs` edits beyond the `# Panics` doc. |
| 47 | PASS | Commit-1 LVP Q1–Q6 carried forward unchanged. One additional Commit-2 surface: `reth_db::cursor::DbCursorRO` for `.walk` — already covered by Commit-1 LVP-Q5 (which documented `walk`/`Walker`). `OnceLock::{new,set,get_or_init}` are std (low-priority, stable — no LVP needed). No Q7 required. |

Summary: 46/47 PASS; **row 34 = FAIL (BY DESIGN, pre-existing + new defensive arms)** — the dispatch-predicted, accepted state; per-crate Phase 1 Gate held; no masking change. (Row 45 PASS on intent; the grep also matching the plan-mandated doctest-stub line is documented, not a defect.)

### Deviations from plan

- **Delta 1 — Plan Step 2.7 skipped.** The `alloy-rlp` AGENTS.md Rule 10 entry landed in Commit 1 per D4 ("same commit as the dep") + the 1.3b `tempfile` precedent + the dispatch. Step 2.7 is a no-op for Commit 2; the edit was NOT re-applied. Cumulative verification row 38 still asserts presence (passes against the persisted Commit-1 state).
- **Delta 2 — Step 2.0 preflight executed.** Removed `#![allow(dead_code)]` (and its Commit-1-only comment) from `mpt/trie.rs`. `cargo build -p krax-state` post-removal: clean, **zero residual dead-code warnings** — the wiring (`MptState::root`/`MptSnapshot::root` → `compute_root` → build → Node/NodeRef/Nibbles/EMPTY_ROOT) makes every trie item reachable.
- **Delta 3 — `step-1.5-decisions.md` D9 prose** was corrected to strict `< 32` post-Commit-1 and is already on disk/committed. Not surfaced as a deviation per the dispatch.
- **Added import `reth_db::cursor::DbCursorRO`.** The plan's Step 2.3/2.4 sketch uses `cursor.walk(None)`; `walk` is a `DbCursorRO` method (LVP-Q5-confirmed), so the trait must be in scope. This is a required consequence of the LVP-Q5-confirmed API, not a design change.
- **`commit` repopulation shape.** `std::sync::OnceLock` has no `From<T>`; the plan's "`OnceLock::from(r)`" sketch isn't a real API. Used the documented "appropriate constructor for the chosen shape": `self.cached_root = OnceLock::new(); let _ = self.cached_root.set(r);` (fresh lock, infallible seed under `&mut self` exclusivity). Semantically identical to the plan's intent (D19 (a)).
- **Row 45 grep-literalism (documented, not a defect).** `git diff snapshot.rs | grep '^\+.*fn '` returns 2 lines; the second is the `///     fn root(&self) -> B256 { B256::ZERO }` doctest-stub line that Step 2.1's cross-step reconciliation REQUIRES. The trait surface gained exactly one method (intent satisfied). Surfaced here per the hand-off rule; not a HALT (the deliverable is correct and plan-mandated).
- **No other Old/New drift.** snapshot.rs / state.rs Old blocks matched HEAD verbatim (Commit 1 did not touch krax-types). mod.rs line numbers shifted by Commit 1's `mod trie;` (+1) and the memo-field/import edits; all anchors re-read at execution time.

### Coverage delta (D15 (a) — pre/post evidence; hold-only)

| Scope | Pre-1.5 (1.4 record) | Post-Commit-1 | Post-Commit-2 (final) | Verdict |
|---|---|---|---|---|
| `krax-state` per-crate (Phase 1 Gate ≥85%) | 90.0% | ≈92.6% | **85.15%** (357 lines, 53 missed) | HOLD — ≥85% ✓ |
| `krax-types` per-crate (Phase 1 Gate ≥85%) | 85.0% | 85.0% | **85.0%** (40 lines, 6 missed) | HOLD — ≥85% ✓ (unchanged from 1.4; state.rs 0% = preexisting `StateError::Released` arm, 1.4 D7) |
| Workspace total lines | 80.99% | 88.60% (transient) | **82.64%** | FAILS `--fail-under-lines 85` — bin-driven (`bin/*/main.rs` 12 lines @0%) + new D12 (d) panic arms |

Per-file post-Commit-2 (`cargo llvm-cov report`, same regex as Makefile): `mpt/mod.rs` 148 lines / 40 missed (72.97%) — the wiring added 7 D12 (d) defensive panic arms (`tracing::error!`+`panic!` on unrecoverable MDBX failure) that are intentionally untested (fault injection is explicitly out of Step 1.5 scope — "ships Ethereum-compatible MPT root computation, nothing more"); `mpt/trie.rs` 209/13 (93.78%); `krax-types/src/state.rs` 5/5 (0% — preexisting `StateError::Released` Display arm, 1.4 Decision 7 = (a)).

**D15 hold-only verdict: SATISFIED.** Per-crate Phase 1 Gate targets both hold (krax-state 85.15%, krax-types 85.0% — both ≥85%). The krax-state 90.0%→85.15% delta is the correctly-added D12 (d) defensive panic arms (unrecoverable storage-corruption paths; the same class of deliberately-untested defensive code as 1.4's `StateError::Released`). The workspace-total `--fail-under-lines 85` failure is the documented pre-existing condition the dispatch explicitly predicted ("the row-5 pre-existing failure from bin-driven coverage will recur; document but do NOT extend `--ignore-filename-regex` to mask it"). **No `Makefile` regex change made.** The right long-term fix (excluding `bin/.*/main\.rs`) is out of Step 1.5 scope, exactly as recorded in the 1.4 Notes.

### Audit outcome

**Wiring confirmed correct.** (i) 46/47 verification rows PASS; row 34 is the dispatch-predicted FAIL-BY-DESIGN coverage state with the per-crate Phase 1 Gate held (krax-state 85.15%, krax-types 85.0%) — not a regression below 85%, not a freelance-able gap; row 45 PASSes on intent. (ii) `make test-integration` proves the three new root-isolation tests (`root_after_write_does_not_bleed_in`, `root_after_commit_does_not_bleed_in`, `two_snapshot_root_independence`) pass against the real-root wiring — `Snapshot::root` reflects the snapshot's frozen view, not live state. (iii) Per-crate coverage does NOT regress below 85% (krax-state 85.15%, krax-types 85.0%); workspace-total failure is bin-driven + accepted defensive arms. (iv) The Commit-1 Q1/Q3/Q4/Q5 LVP findings carried over cleanly: the LVP-Q5 cursor shape (`cursor_read(&self)` → owned cursor; `DbCursorRO::walk(None)` → `Walker: Iterator<Item=Result<(B256,Vec<u8>),_>>`) composes with both wiring sites with **no borrow/lifetime/`'static` gap** — `MptState::compute_root_from_storage` drives `trie::compute_root` to completion over a fresh-RO-txn-owned local cursor before returning; `MptSnapshot::root`'s `get_or_init` closure does the same over the snapshot's held `'static` RO txn and returns only the `Copy` `B256`. Empirically confirmed by `cargo build -p krax-state` clean (post-`#![allow(dead_code)]` removal) + all 6 `snapshot_isolation` + 12 krax-state lib tests passing. No Q3/Q4/Q5 spec gap surfaced; no STOP.

### Proposed commit message (final)

Commit 1 (already committed by maintainer):
```
feat(state): add MPT trie module with sort-then-build root computation — Step 1.5 (1/2)
```

Commit 2 (proposed):
```
feat(state,types): wire MPT root through Snapshot::root + State::root — Step 1.5 (2/2)

Snapshot::root(&self) -> B256 added to the krax-types trait (Rule 8
surface change, D1 — second-ever Snapshot surface change; 1.4
compile_fail doctest stub extended, drop(s) trigger untouched).
State::root doc gains a # Panics section (D12 — signature unchanged,
still infallible). MptState/MptSnapshot each gain cached_root:
OnceLock<B256> (D2/D3); MptState::set invalidates, commit repopulates
(D19), root memoizes via compute_root_from_storage; MptSnapshot::root
lazily computes over the snapshot's frozen RO txn. Both walk Slots via
DbTx::cursor_read -> DbCursorRO::walk (LVP-Q5 @ reth 02d1776) and
panic after tracing::error! on MDBX failure (D12). Three root-isolation
tests added to tests/snapshot_isolation.rs (D17). Commit-1
#![allow(dead_code)] removed (wiring makes trie items reachable).
ARCHITECTURE.md Step 1.5 closed; Phase 1 Gate satisfied; AGENTS.md
Current State/Domain Concepts/Changelog Session 18 updated. Plan
Step 2.7 skipped — alloy-rlp Rule 10 entry landed in Commit 1 (D4).

Coverage: make coverage workspace-total < 85 (bin/*/main.rs 0% +
D12 defensive panic arms, untestable without out-of-scope fault
injection); per-crate Phase 1 Gate holds (krax-state 85.15%,
krax-types 85.0%) — D15 hold-only satisfied, no Makefile mask.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

I did NOT run git commit.

---

## Open questions back to the maintainer

None. The 19-decision document is internally consistent and fully specifies the Step 1.5 deliverable. Two minor judgment calls left to the coder at execution time (both authorized by the decisions doc):

- **D5 `NodeRef::Inline` variant shape.** `Vec<u8>` (the child's RLP bytes, ready for direct embedding) vs `Box<Node>` (the child as a still-encodable Node). Both shapes produce identical on-wire encodings; the choice is implementation-ergonomic only. Coder documents the chosen shape in Outcomes.
- **D7 sort-then-build vs stack-based incremental.** D7 (b) is the lean; D7 (c) is the authorized fallback if (b) needs too much auxiliary buffering. Coder documents the chosen algorithm in Outcomes.
- **D10 (e) reth-trie-generated fixtures vs (d) canonical Ethereum tests JSON.** Coder picks at execution time based on LVP-Q4 outcome. Either path produces the same shipped JSON file (`crates/krax-state/tests/fixtures/mpt_roots.json`); only the generation procedure differs. Coder documents the chosen path in Outcomes.
- **D17 (a) split-vs-inline test placement.** Unit-vector tests in `mpt/trie.rs::tests` is the lean; moving them to `crates/krax-state/tests/mpt_root.rs` is authorized if the 500-line file cap on `trie.rs` would otherwise be exceeded. Coder documents the chosen placement in Outcomes.

These are NOT open questions for the maintainer — they are pre-authorized execution-time choices. Documenting them in Outcomes is sufficient.

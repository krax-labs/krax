# Step 1.5 — MPT Root Computation: Open Decisions

_Maintainer answers each decision below. Planner-round-2 turns answers into `step-1.5-plan.md`._

## Starting context (frozen state coming into 1.5)

Step 1.5 replaces the `B256::ZERO` placeholder in `MptState::root()` (`crates/krax-state/src/mpt/mod.rs:182-188`) and, by extension, adds analogous root computation to `MptSnapshot` (which currently has no `root` surface at all — `Snapshot` in `crates/krax-types/src/snapshot.rs` carries only `get` + `release`). The root MUST be the keccak256 root of the Ethereum-style hexary MPT over `(keccak256(slot_key) → RLP(slot_value))` entries — matching `eth_storageRoot` semantics for a single storage trie. This is the storage trie shape, not the world state trie; Krax's Slots table holds chain-global slots (Phase 1 simplification, not per-account storage).

The Slots table is the hand-rolled reth-db `B256 → Vec<u8>` table from Step 1.3b (`crates/krax-state/src/mpt/slots.rs`), with `Vec<u8>` carrying exactly 32 bytes (the LVP-driven `Value` deviation documented in `mpt/mod.rs` lines 46-55). `MptState::set` auto-flushes per call via a short-lived `RwTxn`; `MptState::snapshot` returns an `MptSnapshot { tx: <DatabaseEnv as Database>::TX }` backed by a long-lived `RoTxn` whose MVCC semantics 1.4 proved via the `tests/snapshot_isolation.rs` three-case suite. Step 1.5 must NOT break that suite (the suite was deliberately written not to assert on root values).

**The alloy-trie vs custom-MPT decision is RESOLVED in favor of custom.** Maintainer rationale (both grounds frozen): (1) AGENTS.md "our own MPT layer" is load-bearing — Krax owns its commitment shape, not reth's evolution timeline; (2) V2 unwind cost — V2's LSM commitments will likely share infrastructure with the MPT layer; internal code is structurally easier to refactor than an external dep. Step 1.5's MPT is scoped to **Ethereum-compatible MPT root for the slots table** — NOT a general-purpose Merkle trie framework. **If scope starts growing into a general framework, halt and re-surface the alloy-trie reconsideration.**

The decisions below are about HOW to build the custom MPT, not WHETHER to. Most decisions are about computation strategy, code shape, test methodology, and where to draw the seam between the trie module and the surrounding `MptState`/`MptSnapshot`.

---

## Decision 1: Does the `Snapshot` trait gain a `root()` method, or does `MptSnapshot` expose `root()` as an inherent method only?

**Context.** `Snapshot` (in `krax-types/src/snapshot.rs`) currently has `get` + `release` only — no `root`. `State::root() -> B256` exists on the parent trait. Phase 14 (optimistic commitments) and any RPC `eth_storageRoot`-style call against a frozen view will want to ask a snapshot for its root. Adding `root` to the `Snapshot` trait is a Rule 8 trait-surface change — exactly the kind of change AGENTS.md tells us to handle as a deliberate phase-planning decision, not a freelancing change. The dispatch prompt language ("`MptSnapshot::root()` currently returns `B256::ZERO`") presumes the method exists; it does not. Surface and decide.

**Options.**

- (a) **Add `fn root(&self) -> B256;` to the `Snapshot` trait, mirroring `State::root`.** Symmetry with `State`. Forces all current and future `Snapshot` implementors to provide a root (today there is only `MptSnapshot`; in V2 there will be an LSM snapshot). Rule 8 change — small, well-motivated. Updates the existing `Snapshot` doctest stub (the one added in 1.4 for the compile_fail invariant).
- (b) **Inherent `MptSnapshot::root(&self) -> B256` only.** No `krax-types` change. Phase-14 callers downcast `Box<dyn Snapshot>` (impossible without trait method) or take a concrete `&MptSnapshot`. Defers the trait surface decision; risks later phase needing it on the trait and surfacing the same Rule 8 churn anyway. Honest if Phase 14 isn't yet pinned on dyn-Snapshot.
- (c) **Add a separate `trait SnapshotRoot: Snapshot { fn root(&self) -> B256; }` in `krax-types`.** Extension trait. Lets the base `Snapshot` stay minimal; lets root-aware callers require the extension. Adds a concept (extension traits) Krax hasn't yet used. Probably premature.

**Phase 1 Gate / Phase 2 implications.** Phase 1 Gate item "Real MPT root computation in place" doesn't strictly require the trait method; it requires `MptState::root` to work. Phase 14 commitment posting WILL eventually need root from a snapshot (the post-commit root is posted from a settled view). Decision (a) lands that surface now while we have one impl. Decision (b) defers and re-opens later.

**My lean:** **(a) Add `fn root(&self) -> B256` to the `Snapshot` trait.** Phase 14 WILL need root from a snapshot (post-commit root for commitment posting), and downcasting a `Box<dyn Snapshot>` is not a clean path. Add it now while there's one impl; V2's LSM snapshot will need it too. Small Rule 8 change, well-motivated.

---

## Decision 2: Root computation strategy — recompute vs memoize vs persist intermediate nodes

**Context.** Every call to `MptState::root()` could (a) walk the Slots table and rebuild from scratch, (b) reuse a cached root from the last write, or (c) read pre-persisted intermediate trie nodes from a sidecar MDBX table. The choice trades CPU for memory/disk for invalidation complexity. The slot count in Phase 1 is small (single-chain V1, no per-account state); the slot count in V2 could be much larger.

**Options.**

- (a) **Recompute from the Slots table on every `root()` call.** Simplest, fully deterministic, no invalidation logic. O(N) where N = slot count per call. `MptSnapshot::root()` uses the snapshot's RO txn directly — same code path as `MptState::root()` but bound to the snapshot view. Easiest to reason about; easiest to delete or replace at V2 boundary. Performance is acceptable for Phase 1 (no Phase 1 caller is in a hot loop calling `root()`).
- (b) **Memoize the root inside `MptState`.** Add an `Option<B256>` field; `set()` invalidates it; `root()` recomputes on miss and caches. `MptSnapshot` cannot share the memo (different view); it computes fresh on first call and caches inside itself for the snapshot lifetime. Cheap, low risk. Cache invalidation logic is one `self.cached_root = None` line in `set()`.
- (c) **Persist intermediate trie nodes in a sidecar MDBX table** (`Nodes: B256 → Vec<u8>` keyed by node hash). `set()` updates the path of nodes from leaf to root; `root()` reads the root pointer. Amortizes cost across calls; enables proof generation later for free. BUT — adds a second table to maintain in lockstep with `Slots`; adds invalidation complexity (any `set()` rewrites O(log N) nodes); adds V2 unwind cost (LSM commit will replace this table). 1.5 scope explicitly excludes proof generation; (c) over-builds for a future step.
- (d) **Hybrid: memoize for `MptState`, recompute for `MptSnapshot`.** Most callers are `MptState`; memoization helps them. Snapshots are typically used for a single root query (the post-commit view in Phase 14), so caching inside the snapshot pays for itself only if the snapshot's `root()` is called multiple times.

**Phase 1 Gate / Phase 2 implications.** (c) imports invalidation complexity that V2 will have to unwind; (a) and (b) leave V2 free to design its own commitment storage. Phase 14 callers will not call `root()` in a hot loop in V1.

**My lean:** **(b) Memoize with `Option<B256>` field, invalidate on `set()`.** For Phase 1, (a) is fine — no caller is in a hot loop calling `root()`. But (b) is nearly free (one `self.cached_root.take()` in `set()`) and avoids redundant O(N) walks. The snapshot does not share the cache; it computes fresh per-snapshot lifetime (per Decision 3).

---

## Decision 3: Snapshot root strategy — lazy on first call vs eager at snapshot-creation time

**Context.** Given Decision 2, the snapshot's root needs a separate computation entry — it must reflect the snapshot's RO txn view, not the live state. Three timings:

**Options.**

- (a) **Lazy: compute on first `root()` call, no caching inside `MptSnapshot`.** Each `root()` call walks the Slots table via the snapshot's RO txn. O(N) per call. Cheapest snapshot creation; most expensive per-call.
- (b) **Lazy + cache: compute on first call, cache the result inside `MptSnapshot` via `OnceLock` or interior mutability.** Requires interior mutability inside `MptSnapshot` (or a `Mutex<Option<B256>>` — adds `Sync` careful-thinking). Pays once per snapshot lifetime.
- (c) **Eager: compute at `MptState::snapshot()` time, store the precomputed root in `MptSnapshot`.** Pays the cost at snapshot creation; subsequent `root()` is free. Increases snapshot creation latency for callers that may never ask for the root. `MptSnapshot { tx, root: B256 }` shape change.
- (d) **`State::snapshot` returns `(Box<dyn Snapshot>, B256)`** — root surfaced alongside the snapshot at creation time. Trait surface change. Avoids the question of whether `Snapshot` has `root()`. Awkward shape for callers who only want one or the other.

**Phase 1 Gate / Phase 2 implications.** Interacts with Decision 1: if `Snapshot` gains `root()` (Decision 1 (a)), the implementation strategy is one of (a)/(b)/(c). If not, the inherent `MptSnapshot::root` can be whichever shape.

**My lean:** **(b) Lazy + cache inside `MptSnapshot` via `OnceLock<B256>` or similar.** Eager pays the cost at snapshot creation even for callers who never ask for root. Lazy is cheaper overall. Cache the result inside `MptSnapshot` so repeated calls don't re-walk.

---

## Decision 4: RLP encoding — alloy-rlp dep vs hand-roll vs reth-rlp transitive

**Context.** The Ethereum MPT requires RLP encoding for slot values (a B256 → 32-byte big-endian integer with leading-zero stripping) and for trie nodes (list-of-bytes shapes for leaf/extension/branch). `alloy-rlp` is the canonical Ethereum-stack RLP crate. It is NOT currently in `[workspace.dependencies]` (verified: only `alloy-primitives` is). It is NOT on AGENTS.md Rule 10's approved-dep list — adoption requires an approved-dep list update in the same commit (1.3b `tempfile` precedent).

**Options.**

- (a) **Add `alloy-rlp` to the workspace and to AGENTS.md Rule 10.** Canonical Ethereum-stack RLP encoding, derive-macro convenience, tier-1 LVP query before use. Thin focused crate, low V2 unwind cost. Used as a regular dep in `krax-state` only (not in `krax-types`).
- (b) **Hand-roll the two RLP shapes Step 1.5 needs.** RLP for storage trie has a small surface — encode a `&[u8]` as a string (single-byte vs short-string vs long-string), encode a list-of-encodings (short-list vs long-list), and the strip-leading-zeros transform for the storage-slot value. ~50-100 lines of well-tested code. No new dep. Reinvents a well-tested wheel; correctness burden falls on Krax.
- (c) **Pull RLP encoding via reth-rlp / reth-trie types.** Rules out by the custom-MPT decision (don't reach into reth's trie internals, even for the encoding pieces). NOT a live option; listed only to rule it out explicitly.

**Phase 1 Gate / Phase 2 implications.** If V2's LSM commitment scheme is non-RLP (e.g. SSZ, custom packed encoding), an `alloy-rlp` dep is dead weight at V2 — but easy to remove. A hand-rolled RLP module in `mpt/rlp.rs` is also easy to remove; the unwind cost is roughly equivalent. The relevant axis is correctness confidence vs dep weight.

**My lean:** **(a) Add `alloy-rlp` as a workspace dep + AGENTS.md Rule 10 update.** The correctness burden of hand-rolling RLP is real — the nibble-prefix encoding (0x00/0x01/0x02/0x03, terminator nibbles) is the most commonly botched part of custom MPT implementations. Using `alloy-rlp` eliminates an entire class of subtle bugs. Thin focused crate, V2 unwind cost is trivial (remove one dep line).

---

## Decision 5: Trie node representation — enum vs struct-with-tag vs trait

**Context.** The Ethereum MPT has three node kinds: Leaf (terminator nibble path + value), Extension (shared-prefix nibble path + child reference), Branch (17-element: 16 child references + value slot). Internal type representation affects allocation, pattern-match ergonomics, and Send + Sync compliance.

**Options.**

- (a) **Plain enum: `enum Node { Leaf { path: Nibbles, value: Vec<u8> }, Extension { path: Nibbles, child: NodeRef }, Branch { children: [Option<NodeRef>; 16], value: Option<Vec<u8>> } }`.** Idiomatic Rust, pattern-match friendly. `NodeRef` is `enum NodeRef { Hash(B256), Inline(Box<Node>) }` to model the embedded-vs-hashed distinction. Likely the smallest viable representation.
- (b) **Tagged struct: `struct Node { kind: NodeKind, ... }`.** More C-like; less idiomatic for Rust; saves on enum tag overhead (negligible).
- (c) **Trait-based: `trait Node { fn encode(&self) -> Vec<u8>; fn hash(&self) -> B256; }` + three impl types.** Object dispatch for traversal; lets specialized impls (e.g. compressed branch) drop in later. Over-engineered for the bottom-up batch-build use case Step 1.5 actually needs.
- (d) **No persistent `Node` type at all — the trie-building algorithm streams encodings directly.** A sort-then-build algorithm (Decision 7 (b)) can emit the RLP encoding of each node as it pops the build stack, never materializing a `Node` value. Minimal types; harder to read; harder to test in isolation.

**Phase 1 Gate / Phase 2 implications.** Proof generation (future step, NOT 1.5) wants a persistent node type or a re-traversal API. (d) closes that off; (a) leaves it open. (a) is the spec-textbook shape and the easiest to cross-check against the Ethereum reference.

**My lean:** **(a) Plain enum with `NodeRef` as a separate enum for inline-vs-hash.** The spec-textbook shape. Pattern-match friendly, idiomatic Rust, and it leaves the door open for proof generation later (not 1.5, but not closed either). (d) closes off proof generation; don't do that.

---

## Decision 6: Keccak source — alloy-primitives::keccak256 vs tiny-keccak vs sha3

**Context.** All hashing in the MPT is keccak256. `alloy-primitives` is already a workspace dep and re-exports `keccak256(impl AsRef<[u8]>) -> B256`. Two alternatives exist (`tiny-keccak`, `sha3`) but neither is in the workspace.

**Options.**

- (a) **`alloy_primitives::keccak256`** — already a dep, returns `B256` directly, no boilerplate. Default.
- (b) **`tiny-keccak` direct** — finer-grained API (streaming hasher), no allocation if used carefully. Adds a dep for negligible benefit.
- (c) **`sha3` crate** — sha3-family general crate; adds a dep, slower than tiny-keccak.

**My lean:** **(a) `alloy_primitives::keccak256`** — already a dep, returns `B256` directly, no boilerplate. Trivially confirmed.

---

## Decision 7: Trie building algorithm — insertion-based vs sort-then-build vs stack-based incremental

**Context.** Three textbook approaches to compute an MPT root over a known set of (key, value) entries.

**Options.**

- (a) **Insertion-based.** Start from empty trie, insert each (key, value). Rebalance on each insert (split leaves into branches, etc.). O(N · log N · K) where K is keccak cost per node touched. Simple to read; conceptually closest to "this is what an MPT is." Slow for batch root computation. Each insert mutates the trie structure.
- (b) **Sort-then-build (bottom-up).** Read all entries from the Slots table (free in key order — MDBX is a B-tree), build the trie bottom-up in one pass. O(N · K) keccak calls. Standard reth/parity approach. Stream-friendly; minimal allocation if combined with Decision 5 (d).
- (c) **Stack-based incremental.** Maintain a stack of in-progress nodes; for each new (key, value), pop the stack to the divergence point with the previous key, push new partial nodes. Same complexity as (b), lowest auxiliary memory (O(tree height) instead of O(N)). Most-aligned with how reth's `HashBuilder` works.

**Phase 1 Gate / Phase 2 implications.** (a) is easiest to cross-check against test vectors but slowest. (b) and (c) match the standard fast-path. Proof generation (later) is easier from (a) or (b)'s materialized-tree intermediate states than from (c)'s ephemeral stack.

**My lean:** **(b) Sort-then-build (bottom-up).** Both (b) and (c) are O(N·K) and standard reth/parity. Sort-then-build is easier to cross-check against reference implementations (walk sorted keys, build bottom-up). The MDBX cursor delivers sorted keys for free, so the sort pass is free. Lean (b) for simplicity and traceability; (c) is acceptable as a coder fallback if (b) needs too much auxiliary buffering.

---

## Decision 8: Iteration order over the Slots table — cursor walk vs BTreeMap materialization vs hybrid

**Context.** Per AGENTS.md Rule 7, state-affecting code must be deterministic; HashMap iteration is forbidden. The Slots table is MDBX-backed (B-tree, naturally key-ordered). Sort-then-build (Decision 7 (b)/(c)) wants sorted iteration; insertion-based (7 (a)) does not strictly require it.

**Options.**

- (a) **Cursor walk via reth-db's `DbTx`-cursor API on the snapshot/state RO txn.** O(1) auxiliary memory. Streams entries in key order (B-tree natural order). Requires an LVP query on the cursor API surface in reth-db at the pinned rev.
- (b) **Materialize the full slot set into a `BTreeMap<B256, B256>` first, then iterate.** O(N) memory. Simpler code (no cursor lifetime); decouples iteration from the txn. Fine for Phase 1 slot counts; concerning for V2.
- (c) **Hybrid: cursor walk into a builder, builder buffers as needed.** Best of both for the stack-based algorithm (7 (c)): cursor provides ordered streaming, builder needs only O(tree height) memory.

**Phase 1 Gate / Phase 2 implications.** (b)'s O(N) memory footprint becomes a real concern at V2 scale; (a)/(c) keep the V1 implementation V2-shaped.

**My lean:** **(a) Cursor walk via reth-db's `DbTx` cursor API**, or **(c) hybrid cursor + builder** if the build algorithm needs internal buffering. O(1) auxiliary memory, streams in B-tree key order. Requires LVP-Q5 confirmation of the cursor API surface at the pinned rev.

---

## Decision 9: Inline-vs-hash node encoding — implement the 32-byte threshold correctly

**Context.** The Ethereum MPT spec: nodes whose RLP encoding is **strictly < 32 bytes** are embedded inline in their parent (the parent's reference IS the encoding); nodes whose RLP encoding is **≥ 32 bytes** are referenced by their keccak256 hash. Source: go-ethereum `trie/hasher.go:68` — a 32-byte encoding IS hashed, not inlined. This is THE most common implementation bug in custom MPTs — getting it wrong (e.g. using `≤ 32`) produces a root that's structurally valid but does not match go-ethereum / reth. Surfaced not because there's a real choice but because the planner round must explicitly call it out, and the coder must test it with a vector that has at least one inline-encoded child.

**Prose corrected 2026-05-14 (Commit 1 of Step 1.5):** earlier draft of this decision wrote `≤ 32` in two places; the spec is strict `< 32`. The Commit 1 implementation matches the spec (verified byte-identical against alloy-trie 0.9.5 across 7 fixtures).

**Options.**

- (a) **Implement the inline-vs-hash distinction per spec — strict `< 32` threshold.** Mandatory. The test vector strategy (Decision 10) must include at least one fixture that exercises the inline-encoded child path (RLP encoding strictly < 32 bytes).
- (b) **Always-hash variant.** Wrong. Listed only to be ruled out explicitly.

**My lean:** **(a) by definition.** Mandatory per spec. The planner MUST add at least one test vector that exercises an inline-encoded child (RLP encoding strictly < 32 bytes) — this is the most common MPT implementation bug.

---

## Decision 10: Test vector strategy — fixture file vs runtime cross-check vs property test

**Context.** Step 1.5 MUST cross-check against a reference. Three approaches; they're not mutually exclusive.

**Options.**

- (a) **Static fixture file.** Pre-generate `(slot_set, expected_root)` pairs using go-ethereum or reth (or hand-computed for small cases), commit as JSON/TOML in `crates/krax-state/tests/fixtures/mpt_roots.json`. Cases: empty trie, single slot, two slots with diverging prefixes, two slots with shared prefix (forces extension+branch), many slots forcing inline-vs-hash threshold (Decision 9), the canonical empty-trie root constant (Decision 11). Maintenance burden: fixture regeneration tooling.
- (b) **Runtime cross-check against `reth-trie` as a dev-dep only.** Adds reth-trie (or alloy-trie) as a `[dev-dependencies]` entry. The "custom MPT" decision rules out these as PRODUCTION deps, but a dev-dep for test cross-checking is arguably fine — it's a test oracle, not part of the shipped binary. Surface explicitly because the spirit of the custom-MPT decision is "Krax owns its MPT," and even a dev-dep oracle creates a soft coupling.
- (c) **proptest-based property test against a reference.** Same as (b) but generates random slot sets at test time. Highest signal; same dep coupling concern as (b). proptest is in Rule 10's test-only dep list but not yet a Krax dev-dep.
- (d) **Cross-check against the canonical Ethereum test vectors** (the `tests` repo at `https://github.com/ethereum/tests`, which ships MPT test vectors used by all major clients). Vendored or fetched at test time. No client dep; canonical authority. Vendoring adds repo bytes; fetching breaks hermetic builds.
- (e) **Hybrid: (a) for hermetic CI + (b) for one-time verification during development**, then drop the dev-dep before commit. Coder uses reth-trie to generate the fixtures (a); ships only (a). Best of both.

**Phase 1 Gate / Phase 2 implications.** (a) and (d) are hermetic; (b) and (c) drag a reference impl into the test build. The Phase 1 Gate item "real MPT root computation in place" implies "verified against the Ethereum spec," not "verified against reth specifically."

**My lean:** **(e) Hybrid: generate fixtures from reth-trie during dev, ship only the static fixtures.** Phase 1 Gate requires "verified against the Ethereum spec," not "verified against reth specifically." Vendoring canonical Ethereum `tests` repo MPT vectors is gold standard if the JSON shape is workable (LVP-Q4 confirms). Practical fallback: use reth-trie as a development-time oracle to generate `(slot_set, expected_root)` JSON, vendor the JSON only, drop the reth-trie reference before commit. Coder picks (d) (canonical tests) vs (e) (reth-generated fixtures) at execution time based on LVP-Q4 outcome.

---

## Decision 11: Empty trie root — hardcode constant vs compute via the trie path

**Context.** The Ethereum MPT empty root is `keccak256(rlp(""))` = `0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421`. Every Ethereum client hardcodes this somewhere.

**Options.**

- (a) **Hardcode the constant** as `EMPTY_ROOT: B256` in `mpt/mod.rs` or `mpt/trie.rs`; `root()` returns it directly when the Slots table is empty. Documents the canonical value at the source level; serves as a sanity check.
- (b) **Compute it via the trie code path.** Naturally produces the same value if the trie code is correct. Doubles as a built-in correctness check.
- (c) **Both:** compute via the path AND assert equality with the hardcoded constant in a test. Belt-and-suspenders.

**My lean:** **(c) Both — hardcode the constant AND compute via the trie path, asserting equality.** The hardcoded constant serves as a documented sanity check; the assertion is a built-in correctness proof that the trie path is correct.

---

## Decision 12: Error surface — `root()` infallible vs `Result<B256, StateError>`

**Context.** `State::root() -> B256` is infallible by trait (`krax-types/src/state.rs:70`). But the MDBX-backed implementation needs to open an RO txn and iterate via cursor, both of which can return `DatabaseError`. Today's placeholder side-steps this by returning `B256::ZERO` unconditionally. The real implementation cannot.

**Options.**

- (a) **Keep `State::root() -> B256` infallible; panic on internal I/O error.** Rationale: an MDBX read failure in `root()` is unrecoverable for the surrounding caller anyway (commit pipeline can't proceed without a root). Convert `DatabaseError` to `panic!` via `expect("MDBX read failure in root()")`. Trait stays clean. Loses graceful degradation.
- (b) **Change `State::root` to `Result<B256, StateError>`.** Rule 8 trait change. Symmetric with `get`, `set`, `snapshot`, `commit` (all return `Result`). Forces every caller to handle the error. Cleanest design; biggest surface change.
- (c) **Cache the last-known-good root in `MptState` (Decision 2 (b) memoize); `root()` returns the cache.** Sidesteps the I/O question — `root()` doesn't touch MDBX. But the cache has to be initialized somehow, and `MptState::open` cannot compute it without potentially failing — so the failure is just moved to `open`, where `Result` is already returned.
- (d) **Hybrid: `State::root` stays infallible, `MptState::root` panics on internal error AND emits a `tracing::error!`. Document that `root()` may panic on storage corruption** at the `State::root` doc comment level. Defensive but not type-safe.

**Phase 1 Gate / Phase 2 implications.** Trait change (b) ripples to any future `State` implementor (V2 LSM backend). If V2's root is also fallible, the trait change pays for itself. If V2 amortizes root computation into write-time (so `root()` is always a field read), (a)/(c) holds.

**My lean:** **(d) Keep `State::root() -> B256` infallible; panic with `tracing::error!` on internal I/O error, documented in the doc comment.** An MDBX read failure during root computation is unrecoverable for the commit pipeline anyway. Converting to a `Result` would require a Rule 8 trait change that doesn't pay for itself in Phase 1. Document that `root()` may panic on storage corruption. Note: per Decision 2 (b) + Decision 19 (a), the cached-root path makes panic surface narrow — only initial population (first `root()` call before any `set()`) can fail.

---

## Decision 13: Module layout — single file vs split

**Context.** The trie code will be ~300-500 lines. The .claude/skills/krax-conventions file-cap is 500 lines.

**Options.**

- (a) **All trie code in `crates/krax-state/src/mpt/mod.rs`.** Single file. Risks exceeding the 500-line cap; tests pile in on top. Already at ~275 lines pre-1.5.
- (b) **Split into `mpt/mod.rs` (MptState + MptSnapshot wiring), `mpt/trie.rs` (trie build + node + root entry point), `mpt/nibbles.rs` (nibble-path type if Decision 5 (a) wants one), `mpt/rlp.rs` (RLP helpers if Decision 4 (b) hand-rolls).** Spreads tests across the modules they cover. Cleanest separation.
- (c) **Single `mpt/trie.rs` for all trie internals; `mpt/mod.rs` stays surface-only.** Middle path. `nibbles` and `rlp` inline in `trie.rs` if small enough.
- (d) **Move trie under `mpt/trie/` directory: `mpt/trie/mod.rs`, `mpt/trie/node.rs`, `mpt/trie/build.rs`.** More structure, more files, more directory churn for what may end up being a small surface.

**Phase 1 Gate / Phase 2 implications.** None directly; refactoring layout later is cheap. Pick what scales for 1.5's actual code volume.

**My lean:** **(c) Single `mpt/trie.rs` for trie internals; `mpt/mod.rs` stays surface wiring.** The file-cap is 500 lines; putting all trie code in `trie.rs` (~300-400 lines) keeps `mod.rs` clean and gives tests a clear home. If `rlp` or `nibbles` helpers grow beyond ~50 lines each, split them — but not preemptively.

---

## Decision 14: Trie API surface — `compute_root` function vs `TrieBuilder` struct vs both

**Context.** What does the internal trie module expose to the surrounding `MptState`/`MptSnapshot`?

**Options.**

- (a) **`fn compute_root(entries: impl Iterator<Item = (B256, B256)>) -> B256` (or `Result<B256, _>` per Decision 12).** Stateless, takes an iterator. Maximally simple. Both `MptState::root` and `MptSnapshot::root` pass their txn cursor here.
- (b) **`struct TrieBuilder { ... } impl { fn new(); fn insert(&mut self, k, v); fn finish(self) -> B256; }`.** Stateful, push-based. Useful for the stack-based algorithm (7 (c)). Slightly more boilerplate at call sites.
- (c) **Both** — expose `compute_root` as the primary public surface, `TrieBuilder` as a lower-level escape hatch. YAGNI for 1.5.
- (d) **An `impl Iterator` wrapper that yields RLP-encoded nodes as it walks** — supports streaming write to a sidecar nodes table (Decision 2 (c)). Only relevant if 2 (c) is chosen.

**My lean:** **(a) `fn compute_root(entries: impl Iterator<Item = (B256, B256)>) -> B256`** — stateless, iterator-based. Both `MptState::root` and `MptSnapshot::root` can call it. Simpler than a builder struct; sufficient for Phase 1's batch-root use case. Per Decision 12 (d), signature stays infallible — internal panics on invariant violation; caller-facing I/O errors are converted at the wrapping `MptState::root` / `MptSnapshot::root` boundary.

---

## Decision 15: Coverage target — hold at 85% vs lift

**Context.** Step 1.5 adds substantial production code. Per 1.3.5 / 1.4 Decision 12, the established pattern is "hold; lift later if natural." Phase 1 Gate is `>85%`; current krax-state is somewhere above that.

**Options.**

- (a) **Hold-only at 85%.** Pattern continuity. The trie code will likely be heavily tested by virtue of test vectors; coverage will move naturally upward.
- (b) **Lift to 88% or 90%** explicitly. Forces the test suite to cover the trie's branches (inline-vs-hash threshold, branch-with-value, extension followed by branch, etc.).
- (c) **Accept a temporary dip** — if trie code has hard-to-reach defensive branches (e.g. internal-invariant `unreachable!()` arms), use `--ignore-filename-regex` to exclude them and hold the line, OR document the dip in the Outcomes block.

**My lean:** **(a) Hold-only at 85%.** The trie code will naturally exercise many branches via test vectors. If hard-to-reach defensive branches appear, document them in Outcomes rather than inflating the target.

---

## Decision 16: Commit shape — single vs multi-commit

**Context.** Step 1.5 is bigger than 1.4. Conventional commits + AGENTS.md tolerate either. 1.3b shipped three commits; 1.4 shipped one.

**Options.**

- (a) **Single commit: `feat(state): implement Ethereum-compatible MPT root — Step 1.5`.** All trie code + tests + wiring + ARCHITECTURE.md/AGENTS.md edits in one. Big but cohesive.
- (b) **Two commits: (1) trie data structures + build algorithm + unit tests in isolation; (2) `MptState::root` + `MptSnapshot::root` wiring + integration tests + docs.** Reviewable; bisect-friendly.
- (c) **Three commits split by surface: (1) RLP + nibbles + hashing plumbing; (2) trie structure + build; (3) State/Snapshot wiring + ARCHITECTURE/AGENTS.md close.** Matches 1.3b precedent. Each commit independently reviewable.
- (d) **Two commits split by concern: (1) trie + wiring; (2) ARCHITECTURE.md/AGENTS.md close + Changelog.** Doc commit at the end.

**My lean:** **(b) Two commits — (1) trie data structures + build algorithm + unit tests; (2) `MptState::root` + `MptSnapshot::root` wiring + integration tests + docs.** Reviewable and bisect-friendly for a step of this size. Single commit (a) is acceptable as a coder fallback if the trie code stays tight (<300 lines) and the diff stays cohesive.

---

## Decision 17: Snapshot root tests — extend `tests/snapshot_isolation.rs` vs new file

**Context.** 1.4's `tests/snapshot_isolation.rs` proves `Snapshot::get` isolation. 1.5's analogous property is `Snapshot::root` (or `MptSnapshot::root`, depending on Decision 1) isolation: a snapshot's root, taken before a write, MUST equal the pre-write root, even after the write commits. The three-case 1.4 suite already exercises the underlying isolation; 1.5's tests strengthen the assertion by checking root values rather than slot values.

**Options.**

- (a) **Extend `tests/snapshot_isolation.rs`** with new test functions that assert root isolation alongside the existing get-isolation cases. Keeps the snapshot-isolation property in one place. File grows ~3 test functions.
- (b) **New file `tests/mpt_root.rs`** (or `tests/snapshot_root.rs`). Separation of concerns: 1.4's file proves isolation of slot reads; 1.5's file proves correctness AND isolation of root computation. Slightly more file overhead.
- (c) **Split: root-computation correctness tests as unit tests in `mpt/trie.rs::tests` (test vector cross-check); snapshot-root isolation tests added to `tests/snapshot_isolation.rs` (extending the property).** Tests live where they belong by topic.

**Phase 1 Gate / Phase 2 implications.** None.

**My lean:** **(a) Extend `tests/snapshot_isolation.rs`** with root-isolation cases alongside the existing get-isolation cases. Keeps the snapshot-isolation property in one place. Add ~3 test functions: root-after-write, root-after-commit, two-snapshot-root-independence. Unit-level root-computation correctness tests (test vectors per D10) live in `mpt/trie.rs::tests` or a parallel `tests/mpt_root.rs` — the snapshot-isolation extensions are about isolation, not computation.

---

## Decision 18: ARCHITECTURE.md & AGENTS.md close + Domain Concepts additions

**Context.** Standard step-close. ARCHITECTURE.md Step 1.5 has five unchecked line items + an unmarked heading + the Phase 1 Gate item "Real MPT root computation in place (Step 1.5 ✅)". AGENTS.md Current State, Changelog, and possibly Domain Concepts (does it list "Trie", "Root", "Node" yet? — verify during planner round). The Phase 1 Gate closes at 1.5 — coverage on krax-types and krax-state >85% interacts with Decision 15.

**Options.**

- (a) **Standard close:** check all five Step 1.5 line items; mark heading ✅; check the Phase 1 Gate "Real MPT root" line; update AGENTS.md Current State to point to "Phase 1 Gate satisfied — Phase 2 next"; add Changelog entry; add Domain Concepts entries for "MPT", "Trie Node", "Storage Root" if not already present.
- (b) **Standard close MINUS the Phase 1 Gate text edit** — leave the gate items as separate checkbox flips and do the "Phase 1 complete" prose in a follow-up close step. Defers a moment of celebration; cleaner separation of "Step 1.5 closes" from "Phase 1 closes."
- (c) **Bundle Phase 1 Gate close into 1.5 wholesale** — explicit "Phase 1 complete" section in AGENTS.md Current State, Changelog calls out gate satisfaction, ARCHITECTURE.md Phase 1 Gate items all checked. Largest narrative impact.

**Phase 1 Gate / Phase 2 implications.** Phase 2 starts immediately after; the EVM execution wrapper depends on state-with-real-root.

**My lean:** **(a) Standard close** — check all five line items, mark heading ✅, check Phase 1 Gate "Real MPT root" line, update AGENTS.md Current State to "Phase 1 Gate satisfied — Phase 2 next," add Changelog entry, verify Domain Concepts has "MPT" / "Trie Node" / "Storage Root" entries (add if missing).

---

## Decision 19: Should `MptState::commit()` cache the post-commit root for reuse by subsequent `root()` calls?

**Context.** `MptState::commit() -> Result<B256, StateError>` currently returns `self.root()`. If Decision 2 picks memoization (b)/(d), `commit` is the natural place to populate the cache. If Decision 2 picks recompute-always (a), `commit` recomputes inline.

**Options.**

- (a) **Yes, `commit` populates a memoized root** (assumes Decision 2 (b) or (d)).
- (b) **No, `commit` recomputes every call** (assumes Decision 2 (a) or (c)).
- (c) **`commit` is a sync barrier only** — returns the same `B256` that the next `root()` would return, but does not pre-compute. (Same as (b) in effect.)

**My lean:** **(a) Yes, `commit` populates the memoized root.** Consequence of Decision 2 (b). `commit` writes all pending slots and then populates `self.cached_root` so subsequent `root()` calls are free. Snapshot root is separate (computed lazy per Decision 3).

---

## Reconsider trigger — alloy-trie

_Single-line entry, NOT a live option requiring trade-off analysis. The alloy-trie vs custom-MPT decision is resolved in favor of custom. Re-surface to the maintainer ONLY if implementation scope starts growing beyond "Ethereum-compatible MPT root for the slots table" into a general Merkle trie framework. The trigger is scope growth, not implementation difficulty._

---

## LVP — Library Verification Protocol items

_Planner-round-2 must run these Context7 queries before drafting `Old:` / `New:` blocks. Use 1.3b's LVP format (per-query: library, query terms, expected finding, actual finding, source path). Cargo-registry-source fallback per 1.3b precedent if Context7 unavailable (genuine unavailability — HTTP 5xx, no relevant hits — not "I prefer source")._

- **LVP-Q1 (tier-1): alloy-rlp encoding API surface** — if Decision 4 = (a). Required surfaces: `Encodable` trait, `encode(&self, out: &mut dyn BufMut)`, `length(&self)`, the string-vs-list discriminant rules, the empty-string and single-byte edge cases. Source-fallback target: `crates/rlp/src/encode.rs` in the alloy-rs/core repo. Matters because the RLP encoding of trie nodes is load-bearing for root correctness — getting it wrong is silent.
- **LVP-Q2 (tier-1): `alloy_primitives::keccak256` API** — confirm input type (`impl AsRef<[u8]>`), output type (`B256`), and that the function is stable across the workspace pin. Cheap; near-certain to be confirmed. Matters because it's called O(N) times per root.
- **LVP-Q3 (tier-1): Ethereum MPT spec — inline-vs-hash threshold and nibble-prefix encoding.** Source: Ethereum Yellow Paper Appendix D, or the canonical Ethereum wiki page on MPT. Surfaces: (i) the 32-byte threshold rule for inline encoding, (ii) the leaf/extension nibble-prefix encoding (terminator nibble 0x10/0x20, odd/even path length 0x00/0x01/0x02/0x03 prefix bytes), (iii) branch node 17-slot layout (16 children + value). Matters because every custom-MPT implementation that gets the prefix encoding wrong produces a structurally valid but spec-incorrect root.
- **LVP-Q4 (tier-1, conditional): reth-trie or `tests/` repo test vectors** — if Decision 10 picks (a), (b), (c), (d), or (e). For (a)/(d): the exact path to canonical (slot_set, expected_root) fixtures and the JSON shape. For (b)/(c): the reth-trie public function signature for `storage_root_of(entries)`. For (e): the reth-trie API for development-time fixture generation. Source-fallback target: `https://github.com/ethereum/tests` MPT subdirectory.
- **LVP-Q5 (tier-1, conditional): reth-db cursor API** — if Decision 8 picks (a) or (c). Required: `DbTx::cursor_read::<T>() -> Result<Self::Cursor, _>`, the cursor's `walk` / `next` API, the cursor's lifetime bound relative to the txn. Source-fallback target: `crates/storage/db-api/src/cursor.rs` at the pinned rev `02d1776786abc61721ae8876898ad19a702e0070`. Matters because the iteration shape over the Slots table is load-bearing for the streaming build algorithm.
- **LVP-Q6 (tier-2): empty-trie root constant cross-check** — confirm `0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421` against a canonical source (go-ethereum constants file or Ethereum wiki). Tier-2 because the constant is widely published; a sanity check.

---

## Out-of-scope reminder

Step 1.5 ships **Ethereum-compatible MPT root computation for the Slots table — nothing more.** Explicitly out of scope: (1) per-account state trie / world state root — Phase 1 is chain-global slots only; (2) MPT proof generation (`eth_getProof` semantics) — later phase; (3) ZK-friendly hashes (Poseidon, Rescue) — V2 / Phase 23+; (4) trie pruning, archive node support, historical state queries — V1 is forward-only commitment over the current slot set; (5) persistence of intermediate trie nodes BEYOND what Decision 2 explicitly authorizes (default is no sidecar nodes table); (6) alloy-trie as a PRODUCTION dep — frozen decision; only re-opened by the scope-growth reconsider trigger above; (7) EVM execution, RPC integration, mempool, sequencer, batcher — all later phases; (8) new `krax-types` traits beyond the one possible Rule 8 change in Decision 1; (9) new crates — Step 1.5 touches `krax-state` and possibly `krax-types` only.

The planner round 2 MUST re-read this section before drafting Execution Steps and MUST add an explicit "Out-of-scope check" row to the per-commit Verification table.

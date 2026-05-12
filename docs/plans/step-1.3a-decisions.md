# Step 1.3a Decisions — In-Memory MptState

Status: ✅ Answered 2026-05-12
Date surfaced: 2026-05-12
Date answered: 2026-05-12

---

## Context

Step 1.3a ships the first usable `MptState` implementation in `crates/krax-state/src/mpt/mod.rs`,
backed entirely by an in-memory `BTreeMap`. No MDBX, no `reth-db`, no disk I/O. The round-trip
test (`state.set(k, v); state.commit(); state.get(k) == v`) passes against this in-memory backend.
As part of this step, `StubState` is deleted from `crates/krax-types/src/journal.rs`'s
`#[cfg(test)] mod tests`, and the three `Journal::apply` tests are rewritten to use `MptState`.

The step is split from 1.3b because the original ARCHITECTURE.md Step 1.3 description conflates
two concerns with very different risk profiles: (1) defining the `State` trait impl shape and the
snapshot/commit/root contracts — a design problem resolved against an in-memory backend with no
moving parts — and (2) wiring MDBX via `reth-db`, which requires reth LVP queries, restart-test
infrastructure, and `StateError` I/O variants. Splitting lets the coder verify the design against
a simple backend before introducing the storage layer.

What 1.3a is NOT responsible for: MDBX integration, reth-db dependency, restart tests, the
`StateError` I/O variants that MDBX will require, Step 1.3.5 coverage tooling (separate step),
or Step 1.4 snapshot isolation tests. The `State` and `Snapshot` trait surfaces in `krax-types`
are stable (AGENTS.md Rule 8 — no extensions without explicit phase planning); if any decision
below requires a trait extension, it is flagged as a blocker, not a pre-resolved choice.

---

## Open decisions

### Decision 1 — Module layout: how does `lib.rs` expose `MptState` and does `mpt/mod.rs` need internal structure?

Does 1.3a need an internal backend abstraction inside `krax-state`, or is `MptState` a flat struct
in `mpt/mod.rs` that `lib.rs` simply re-exports?

Options:
- **(a) Flat re-export** — `mpt/mod.rs` contains `MptState` and its `impl State` and `impl Snapshot`
  (for the in-memory snapshot type) directly. `lib.rs` re-exports `MptState` (or keeps it
  `pub(crate)` since consumers use `Box<dyn State>`). No internal backend trait. Simplest possible
  1.3a shape. 1.3b restructures `mpt/mod.rs` to accommodate the MDBX backend, possibly splitting
  into `mpt/mod.rs` + `mpt/mdbx.rs`.
- **(b) Internal backend trait** — `lib.rs` introduces a private `trait MptBackend` with an
  in-memory impl (`mpt/memory.rs`) and a stub MDBX impl (`mpt/mdbx.rs`) to follow. `MptState`
  is generic over or holds a `Box<dyn MptBackend>`. Anticipates 1.3b explicitly.
- **(c) Generic-over-backend** — `MptState<B: Backend>` is a generic struct. 1.3a uses
  `MptState<MemoryBackend>`; 1.3b uses `MptState<MdbxBackend>`. The consumer always holds
  `Box<dyn State>` so the generic is erased at the trait boundary.

**Constraints from prior steps:** AGENTS.md Rule 1: cross-crate consumers interact via traits in
`krax-types` (`Box<dyn State>`) — `MptState`'s internal structure is invisible to them.
AGENTS.md Rule 10 / design principle 6: "don't pre-build V2 abstractions in V1 code" applies
equally to V1 internal abstractions premature to the current step.

**1.3b implications:** Option (a) requires the least 1.3a work but may require restructuring
`mpt/mod.rs` in 1.3b to accommodate MDBX. Options (b) and (c) anticipate 1.3b but add complexity
with no current benefit; AGENTS.md Rule 10 prohibits abstractions beyond what the task requires.

**Answer:** **(a) Flat re-export.** `mpt/mod.rs` contains `MptState` directly; `lib.rs` re-exports `MptState` as `pub`. `MptState::new() -> Self` is a public constructor with a `///` doc comment from day one (per Open Question 3 answer below). No internal backend trait, no generic-over-backend pattern. Rationale: V1 design principle 6 — don't pre-build for 1.3b inside 1.3a. When MDBX lands in 1.3b, the restructuring is a known cost; doing it now would be premature abstraction. Keep 1.3a small, honest, deletable.

---

### Decision 2 — Cargo.toml dependencies: what does `crates/krax-state` need for 1.3a?

`crates/krax-state/Cargo.toml` currently has an empty `[dependencies]` block. What runtime and
dev-dependencies does 1.3a add?

**Runtime dependency candidates:**
- `krax-types = { path = "../krax-types" }` — required: `State`, `Snapshot`, `StateError` all live
  here. This is the only runtime dep clearly required for an in-memory backend.
- `alloy-primitives = { workspace = true }` — `B256` is used directly in `mpt/mod.rs`; however,
  `B256` is already re-exported transitively through `krax-types`'s public API. The question is
  whether `mpt/mod.rs` needs to import `alloy_primitives::B256` directly or can use the krax-types
  re-export.
- `thiserror = { workspace = true }` — only needed if Decision 7 adds new `StateError` variants
  (which live in `krax-types`, not `krax-state`). Likely not needed in 1.3a.
- `tracing = { workspace = true }` — only if 1.3a emits tracing events. For an in-memory stub,
  arguably premature.

**Dev-dependency candidates:**
- `rstest = { workspace = true }` — consistent with 1.2b precedent (1.2 Decision 2).
- `pretty_assertions = { workspace = true }` — consistent with 1.2b precedent (1.2 Decision 7).
- `krax-types` is already a runtime dep, so its public types are available in tests without a
  separate dev-dep entry.

**1.3b implications:** `reth-db` and other `reth-*` crates must NOT be added in 1.3a — they pull
in the full reth dependency graph and require LVP queries. They land in 1.3b.

**Answer:** **Runtime:** `krax-types = { path = "../krax-types" }` ONLY. No direct `alloy-primitives` dep — `B256` is accessed via `krax_types::` re-exports (note: planner-flagged concern that `krax-types` doesn't currently re-export `B256` directly. Coder verifies during execution: if `B256` is not transitively visible, add `alloy-primitives = { workspace = true }` to runtime deps as a minimal fix, surface as a deviation in Outcomes). No `thiserror`, no `tracing` in 1.3a. **Dev-dependencies:** `rstest = { workspace = true }` and `pretty_assertions = { workspace = true }`, consistent with 1.2b precedent. Rationale: minimal dep surface, additions earned by code that needs them.

---

### Decision 3 — In-memory backing structure: flat map vs. committed/pending two-layer model

What data structure holds slot state inside `MptState`, and should 1.3a model a "pending writes"
layer separate from "committed state"?

Options:
- **(a) Single `BTreeMap<B256, B256>`** — all writes go directly into the map. `set()` mutates the
  map; `get()` reads from it; `commit()` returns the root (or no-op if root is a placeholder);
  `snapshot()` clones the map. No distinction between committed and pending writes.
- **(b) Two-layer model: `committed: BTreeMap<B256, B256>` + `pending: Vec<(B256, B256)>` (or
  pending BTreeMap)** — `set()` appends to `pending`; `commit()` merges `pending` into `committed`
  (last write wins per EVM semantics) and clears `pending`; `get()` checks `pending` first, then
  `committed` (or checks the merged view); `snapshot()` clones only `committed`. Makes the
  commit/pending distinction explicit.
- **(c) Two `BTreeMap` fields (`committed` + `pending`)** — same as (b) but both layers are maps
  for O(log n) lookup within pending. `get()` queries `pending`, falls back to `committed`.

**Constraints from prior steps:** AGENTS.md Rule 7 (Determinism): "No `HashMap` iteration in
commit-path code." `BTreeMap` iteration is inherently ordered and satisfies Rule 7 directly.
`HashMap` is not an option here. 1.1b Decision 11: `BTreeSet`/`BTreeMap` confirmed for all
commit-path data structures.

**1.3b implications:** If option (a) is chosen, 1.3b's MDBX backend replaces the BTreeMap with
MDBX read/write semantics — the two-layer distinction (pending writes vs committed state) is
naturally expressed by MDBX's transaction model. If option (b) or (c) is chosen in 1.3a, the
1.3b MDBX backend's transaction model maps cleanly onto the pending/committed layer concept.
Either way, the choice here is an implementation detail of `MptState` — it is not visible through
the `State` trait interface.

**Answer:** **(a) Single `BTreeMap<B256, B256>`.** All writes go directly into the map; no pending/committed distinction in 1.3a. The two-layer model is 1.3b's call when MDBX transaction semantics make the distinction meaningful. Rationale: the simpler structure is the right call when there's no I/O to justify the complexity. Pre-building a pending layer now means dead code that 1.3b may discard if MDBX's transaction model expresses the distinction differently.

---

### Decision 4 — Snapshot struct: how does `MptState::snapshot()` produce a `Box<dyn Snapshot>`?

The `snapshot()` method returns `Result<Box<dyn Snapshot>, StateError>`. The `Box<dyn Snapshot>`
is `'static` — the snapshot cannot borrow from `&self`. What type implements `Snapshot` for the
in-memory backend, and how does it obtain its data?

Options:
- **(a) Clone the map** — `MptSnapshot { data: BTreeMap<B256, B256> }` (a fully-owned clone of the
  current map state). `MptSnapshot::get()` reads from `data`. O(n) clone on `snapshot()`. Fully
  isolated: subsequent writes to `MptState` do not affect the snapshot.
- **(b) Arc-of-map** — `MptState` wraps its map in `Arc<BTreeMap<B256, B256>>`. On `snapshot()`,
  clone the `Arc` (O(1)) into `MptSnapshot { data: Arc<BTreeMap<B256, B256>> }`. On `set()`,
  `MptState` calls `Arc::make_mut` to get exclusive ownership before mutating (copy-on-write
  triggered by any write that occurs while a snapshot exists). The snapshot holds its Arc safely.
- **(c) Snapshot is a no-op struct that always reads from MptState** — not viable; the `Box<dyn
  Snapshot>` is `'static` and cannot borrow from `MptState`. Included for completeness; can be
  dismissed.

**Constraints from prior steps:** 1.1a Decision 6: `Snapshot: Send + Sync`. The snapshot struct
must satisfy these bounds. `BTreeMap<B256, B256>` is `Send + Sync`; `Arc<BTreeMap<B256, B256>>`
is also `Send + Sync`. 1.1a Decision 1: `release(self: Box<Self>)` is consuming — the snapshot
implementation of `release()` is a no-op (the `Box` is dropped, and with it the owned map clone
or Arc reference).

**1.3b implications:** In 1.3b, `MptState::snapshot()` will return an MDBX read-only transaction
wrapped in a struct. The in-memory `MptSnapshot` struct from 1.3a may be deleted entirely, or
kept alongside the MDBX snapshot for a test-backend path. If option (a), deletion is clean. If
option (b), the Arc pattern may be worth retaining for a lightweight test-only path.

**Answer:** **(a) Clone the map.** `MptSnapshot { data: BTreeMap<B256, B256> }` is an owned clone of the current state. O(n) cost on `snapshot()` is acceptable for 1.3a; correctness and simplicity matter more than snapshot performance at this stage. Rationale: Arc-of-map (option b) is clever and might survive 1.3b, but "might" is the operative word — 1.3b's MDBX snapshot is a different shape (read-only transaction) that probably doesn't reuse the Arc pattern. Cloning is deletable scaffolding; Arc-of-map is scaffolding that thinks it's not scaffolding.

---

### Decision 5 — `commit()` semantics: what does committing mean for an in-memory backend?

`commit(&mut self) -> Result<B256, StateError>` is supposed to "durably apply all pending writes
and return the post-commit state root." For an in-memory backend, there is no I/O. What should
`commit()` do?

Options:
- **(a) No-op checkpoint** — `commit()` returns `Ok(self.root())`. All writes are immediately
  "durable" (in memory); there is no pending/committed distinction. `commit()` is simply a signal
  that callers can use to checkpoint the root value.
- **(b) Flush-pending-to-committed** — if Decision 3 chose a two-layer model, `commit()` merges
  the pending layer into the committed layer, then computes and returns the root. This mirrors the
  eventual MDBX semantics (flush to disk) with an in-memory analogue.
- **(c) Literal no-op returning `Ok(B256::ZERO)`** — commit does nothing and returns a zero root.
  Honest for 1.3a but creates a mismatch: the round-trip test calls `commit()` and presumably
  expects something meaningful back if root() is non-zero (Decision 6).

**Sub-question:** Does `get()` return values written by `set()` before `commit()` is called?
EVM semantics within a transaction mean yes — a worker's own writes are visible to its own
subsequent reads within the same speculative context. For the round-trip test
(`state.set(k, v); state.commit(); state.get(k) == v`), `commit()` is called before `get()`, so
both behaviors (writes-visible-before-commit and writes-visible-only-after-commit) pass the test.
The EVM model, and the sequencer architecture, are consistent with "writes always visible
immediately" for the in-memory backend.

**Constraints from prior steps:** 1.1a Decision 7: `commit()` returns `Result<B256, StateError>`
(post-commit state root). This decision is already settled at the trait level; the question is
what the concrete implementation puts in that `B256`.

**1.3b implications:** In 1.3b, `commit()` must flush to MDBX. The two-layer model (if chosen in
Decision 3 option b or c) maps cleanly onto the MDBX transaction commit semantics. Option (a)
here does not prevent 1.3b from implementing real flush semantics.

**Answer:** **(a) No-op checkpoint returning `Ok(self.root())`.** All writes are immediately visible via `get()` without requiring a prior `commit()`. `commit()` is a no-op that returns the current root value (which is `B256::ZERO` per Decision 6). Consistent with Decision 3's single-map model: there's no pending layer to flush. EVM-semantic-correct: writes within a transaction are visible to their own subsequent reads. Sub-question (writes visible before commit?): YES.

---

### Decision 6 — `root()` semantics: placeholder, deterministic hash, or real MPT root?

`root(&self) -> B256` returns the current state root without committing pending writes. The doc
comment says "concrete implementations may return a cached value." The returned root will be
posted to Ethereum L1 in Phase 14 — it must eventually be a real Ethereum-compatible state root.

Options:
- **(a) Placeholder `B256::ZERO`** — always returns `B256::ZERO` in 1.3a. The round-trip test
  does not assert on the root value (or asserts `== B256::ZERO`). Honest: real MPT root
  computation is deferred. A `// TODO Step X: implement real MPT root` comment marks the spot.
- **(b) Deterministic hash-of-sorted-entries** — compute `keccak256(BTreeMap sorted slot ∥ value
  pairs)` over the current map contents. Non-zero, deterministic, changes when state changes.
  The round-trip test can assert that root() changes after a write. NOT a real MPT root — this
  is a different hash construction. Code named `MptState` that computes a non-MPT root carries
  a documentation debt.
- **(c) Real MPT root via `alloy-trie` or custom MPT implementation** — produces an
  Ethereum-compatible state root from day one. Two sub-options:
    - **(c1)** Use `alloy-trie` (not currently in `[workspace.dependencies]`; new dep requiring
      Rule 10 justification, Library Verification Protocol query at medium priority).
    - **(c2)** Implement a minimal MPT root computation inline in `mpt/mod.rs` using only
      `keccak256` from `alloy-primitives`. AGENTS.md says "our own MPT layer on top where needed"
      — this phrase suggests custom MPT is the intended direction for V1.
- **(d) Placeholder with explicit `commit()` contract** — `root()` returns `B256::ZERO` and
  `commit()` also returns `Ok(B256::ZERO)`. Clear that both are stubs. The 1.3a round-trip test
  asserts only on `get()`, not on the returned root value.

**Trade-offs by option:**

| Option | Round-trip test can assert on root? | New dep? | Honest about "MPT"? | 1.3b burden |
|---|---|---|---|---|
| (a) / (d) | No (or asserts `== B256::ZERO`) | No | Yes (placeholder flagged) | 1.3b adds real root alongside MDBX |
| (b) | Yes (root changes) | No | Marginal (keccak ≠ MPT root) | 1.3b replaces with real MPT root |
| (c1) | Yes (real root) | Yes (`alloy-trie`) | Yes | 1.3b adds MDBX, root impl is done |
| (c2) | Yes (real root) | No | Yes | 1.3b adds MDBX, root impl is done |

**Constraints from prior steps:** AGENTS.md Tech Stack "Storage (V1)": "reth-db (MDBX-backed)
for state, **with our own MPT layer on top where needed**" — this phrase is load-bearing. It
suggests the MPT root computation is a Krax-owned concern, not delegated to an external library.
If so, option (c2) aligns with the architecture intent; option (c1) may conflict with it.

**1.3b implications:** If (a)/(d) is chosen, 1.3b must implement real root computation on top of
MDBX wiring — two concerns in one step. If (c1) or (c2) is chosen in 1.3a, 1.3b focuses purely
on MDBX persistence; root computation is already correct.

**Answer:** **(a) Placeholder `B256::ZERO`.** `root()` returns `B256::ZERO` with a `// TODO Step 1.5 — MPT Root Computation` comment marking the spot. Round-trip test asserts on `get()` only, not on root value. Real MPT root computation is its own named step (**Step 1.5**, slotted between Step 1.4 and the Phase 1 Gate — see ARCHITECTURE.md edits in the 1.3a planner round). 1.5's first decision (alloy-trie vs custom MPT computation) is pre-surfaced below in the "Decisions about future steps" section because it has cross-cutting implications for 1.3a/1.3b workspace deps. Maintainer-recommended lean: option (c2) custom MPT, because AGENTS.md "our own MPT layer" language is load-bearing and an external trie dep at V1's root creates V2 unwind cost. Planner to surface properly at 1.5 dispatch with both options; this is not a pre-resolution.

---

### Decision 7 — `StateError` extension: does the in-memory backend require new variants?

Does 1.3a need to add any new `StateError` variants beyond the existing `Released` (from 1.1a
Decision 2)?

**Context:** `StateError` lives in `crates/krax-types/src/state.rs` — a cross-crate edit governed
by Rule 1 and Rule 8. Adding a variant there requires a change to `krax-types`, not just
`krax-state`. The in-memory backend's operations (`get`, `set`, `commit`, `root`, `snapshot`) are
all infallible in the happy path — there are no I/O failure paths.

Options:
- **(a) Nothing new for 1.3a** — all operations return `Ok(...)`. `snapshot()` returns
  `Ok(Box::new(MptSnapshot { ... }))`. The only failure path visible through the trait is
  `StateError::Released` (used when a released snapshot is read — but with the consuming
  `release(self: Box<Self>)` semantics from 1.1a Decision 1, this is a compile-time error, not
  a runtime error). No cross-crate edit to `krax-types` required in 1.3a.
- **(b) Add `StateError::Internal(String)` as a catch-all** — for paths that "should not happen"
  but are better than panicking (e.g. unexpected allocation failure in a test environment). The
  in-memory backend would never produce this variant, making it dead code in 1.3a.
- **(c) Blocker — escalate** — if during 1.3a implementation the coder discovers a scenario where
  the in-memory backend genuinely needs to signal a failure not covered by `Released`, the coder
  must STOP and surface the specific scenario to the maintainer. This is not a pre-resolved option
  but a protocol clause.

**Constraints from prior steps:** 1.1a Decision 2: `StateError` starts with `Released` only +
`#[non_exhaustive]`. The `#[non_exhaustive]` attribute exists precisely to allow future variants
without breaking downstream matches. 1.3b's MDBX backend will need I/O error variants (e.g.
`DbError { source: ... }` with `#[from]`); those land in 1.3b alongside the MDBX code.

**Answer:** **(a) Nothing new for 1.3a.** All `MptState` operations return `Ok(...)` in the happy path. `#[non_exhaustive]` on `StateError` (from 1.1a Decision 2) accommodates 1.3b's I/O additions without any 1.3a cross-crate edits. If during 1.3a implementation the coder finds a failure path that genuinely needs a new variant, option (c) applies — STOP and escalate.

---

### Decision 8 — `Journal::apply` test location: circular dependency blocker

The directive from 1.2b (Post-execution directives, inherited from 1.2 Decision 6) requires
deleting `StubState` from `crates/krax-types/src/journal.rs` `#[cfg(test)] mod tests` and
rewriting the three `Journal::apply` tests against `MptState`. Where do these tests live?

**The core constraint:** `krax-state` will depend on `krax-types` (runtime). Adding `krax-state`
to `krax-types`'s `[dev-dependencies]` creates a circular crate dependency that Cargo cannot
resolve (`krax-types` → (dev) → `krax-state` → (runtime) → `krax-types`).

Options:
- **(a) Stay in `krax-types/src/journal.rs`, add `krax-state` as `[dev-dependencies]` of
  `krax-types`** — **blocked by circular dependency. Not viable.** Included for completeness;
  do not choose this option.
- **(b) Move to `crates/krax-state/tests/journal_apply.rs`** (Cargo integration test directory) —
  imports `Journal`, `JournalEntry` from `krax-types` and `MptState` from `krax-state`. Clean
  dependency direction. Tests run as an integration test crate (compiled separately). No `integration`
  feature gate needed (in-memory, no external resources). Not co-located with `journal.rs` — the
  tests are no longer adjacent to the type they primarily test.
- **(c) Move into `crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests`** — imports
  `Journal` and `JournalEntry` from `krax-types`. Tests are in the same module as `MptState`
  and are framed as "MptState correctly applies journals" rather than "Journal::apply writes to
  state." Consistent with AGENTS.md Rule 5 (tests mirror module layout — the test is in the module
  that implements the thing being exercised: MptState).
- **(d) Move to `crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests` AND also add
  standalone round-trip tests there** — combines the Journal::apply rewrite with the 1.3a
  round-trip test (`state.set(k,v); state.commit(); state.get(k) == v`) in one test module.
  All MptState behavioral tests co-located.

**Sub-question (compile_fail doctest on `Journal::discard`):** Per 1.2b Deviations, the
`compile_fail` doctest is in the `///` comment block on `Journal::discard` in `journal.rs`, NOT
inside `#[cfg(test)] mod tests`. It does not reference `StubState` or `MptState`. It is unaffected
by the `StubState` deletion. Confirm: does the maintainer agree no change is needed to the
doctest?

**Constraints from prior steps:** 1.2 Decision 6 post-1.3 directive (verbatim): "The three
`Journal::apply` tests in `journal.rs` test module are **rewritten against `MptState`** — they
live in the Step 1.3 plan's test scope, not in `krax-types`." This phrase "not in `krax-types`"
was written knowing the circular dep exists. Options (b), (c), (d) all satisfy it.

**1.3b implications:** If option (b) is chosen, the `crates/krax-state/tests/` directory is
established now; 1.3b's restart tests could naturally live there. If option (c) or (d), 1.3b's
restart tests would live in `mpt/mod.rs` unit tests or in the `tests/` integration dir — the
maintainer's choice.

**Answer:** **(c) Co-located in `mpt/mod.rs`'s `#[cfg(test)] mod tests`**, absorbing both the rewritten `Journal::apply` tests AND the new round-trip test into one test module. Rationale: AGENTS.md Rule 5 (tests mirror module layout) supports test co-location with the type being exercised. The `Journal::apply` tests are reframed as "MptState correctly applies journals" — a behavior-level claim about MptState that happens to exercise Journal::apply. Cleaner than the `tests/` integration-test directory for a single test module. Sub-question (compile_fail doctest): UNAFFECTED. The doctest stays in `Journal::discard`'s `///` comment block in `journal.rs`; no change.

---

### Decision 9 — Test assertion mechanics for the rewritten `Journal::apply` tests

What exactly do the three rewritten tests assert, given that they can no longer access
`StubState`'s internal `BTreeMap` directly?

**Context:** The original assertions were `assert_eq!(state.0.get(&slot(1)), Some(&slot(42)))` —
direct map inspection. The `MptState`'s internal map is not exposed. Tests must assert via
`mpt_state.get(slot)` (the `State` trait interface).

Options:
- **(a) Assert via `mpt_state.get(slot)` only, no `commit()` call** — apply the journal, then
  immediately call `mpt_state.get(slot)` and assert the value. Tests the apply protocol directly.
  Requires that `get()` returns values written by `set()` without a prior `commit()` (depends on
  Decision 3 and Decision 5 — true for a single-map model, may vary for two-layer model).
- **(b) Assert via `mpt_state.get(slot)` after `mpt_state.commit()`** — mirrors the round-trip
  test form. Tests apply + commit together.
- **(c) Assert pre-commit AND post-commit** — verifies both that writes are visible immediately
  and that they persist through a commit. Most thorough.

**Sub-question: what test helper provides `slot(n)`?** The `slot()` helper in
`crates/krax-types/src/test_helpers.rs` is `pub(crate)` — not accessible from `krax-state`.
Options:
- **(i) Re-create `slot()` in `krax-state`** — a private `fn slot(n: u8) -> B256` in the
  relevant test module. Small duplication. No API surface change.
- **(ii) Promote `slot()` to `pub`** in `krax-types/src/test_helpers.rs` — makes it accessible
  from other crates' test code. Changes the API of a test helper (minor, but `missing_docs` may
  fire on a promoted pub function in a non-cfg(test)-gated module context).
- **(iii) Add a `test-utils` feature to `krax-types`** — a `#[cfg(feature = "test-utils")]` module
  exposing `slot()`. Reusable across crates but adds a feature flag to `krax-types`.

**Constraints from prior steps:** 1.2 Decision 4 note: "If the same helper is needed in Phase
4/5/6 tests in other crates, those crates define their own or we add the production constructor
then." This suggests re-creation (option i) is the expected pattern for test helpers that don't
have a production call site yet.

**Answer:** **(a) Assert via `get()` only, no `commit()` call.** Tests the apply protocol directly: `journal.apply(&mut state)?; assert_eq!(state.get(slot(1))?, slot(42));`. Consistent with Decision 5's answer (writes visible immediately without commit). Sub-question (slot helper): **(i) Re-create `slot()` locally in `krax-state`** — INLINE inside the `#[cfg(test)] mod tests` block in `mpt/mod.rs`, NOT in a separate `test_helpers.rs` file. Single test module = single use = no separate helper module earned. The `test_helpers.rs` precedent in `krax-types` was multi-consumer (rwset.rs, journal.rs both used `slot()`); `krax-state` has one test module. Structure follows actual reuse pattern, not anticipated reuse.

---

### Decision 10 — ARCHITECTURE.md Step 1.3 checkbox partitioning between 1.3a and 1.3b

Which of the five Step 1.3 checkboxes get closed by 1.3a, and which remain open for 1.3b?

**The five checkboxes (verbatim from ARCHITECTURE.md):**
1. `[ ]` `crates/krax-state/src/mpt/mod.rs` — `MptState` struct backed by MDBX (via `reth-db`)
2. `[ ]` Implement `State` trait against an in-memory map first
3. `[ ]` Wire MDBX as the durable backend
4. `[ ]` Round-trip test: `state.set(k, v); state.commit(); state.get(k) == v`
5. `[ ]` Restart test: open DB, set, commit, close, reopen, get returns committed value

Checkbox 1 creates tension: the checkbox description says "backed by MDBX" (a 1.3b concern), but
the file `mpt/mod.rs` and the `MptState` struct itself are created in 1.3a.

Options:
- **(a) 1.3a closes checkboxes 2 and 4; checkboxes 1, 3, 5 remain for 1.3b.** Checkbox 1 is read
  as "the full MDBX-backed struct" — not closed until 1.3b ships.
- **(b) 1.3a closes checkboxes 2 and 4; checkbox 1 is edited to separate struct creation from
  MDBX backing.** The edit would split checkbox 1 into two checkboxes: "Create `MptState` struct
  and implement `State` trait" (closed by 1.3a) and "Wire MDBX as the durable backend" (closed
  by 1.3b, merging with current checkbox 3). Requires a ARCHITECTURE.md text edit.
- **(c) No checkboxes are closed by 1.3a** — all five are part of the Step 1.3 definition as
  written, which requires MDBX. 1.3a progress is captured only in AGENTS.md Current State.

**Sub-question:** Does the Step 1.3 heading get a `✅` after 1.3a or 1.3b?

All options agree: `✅` lands at 1.3b, when restart tests pass and all checkboxes are closed.

**Sub-question:** What does AGENTS.md Current State say after 1.3a ships but before 1.3b? It
must record: (1) in-memory MptState created, State trait impl complete, (2) StubState deleted,
Journal::apply tests rewritten, (3) 1.3b (MDBX) is next. The shape mirrors the 1.2a/1.2b
pattern in the existing Changelog.

**Answer:** **(b) Edit ARCHITECTURE.md to split checkbox 1.** Checkbox 1 is split into two: "Create `MptState` struct and implement `State` trait against in-memory backing" (closed by 1.3a) and "Wire MDBX as the durable backend" (closed by 1.3b, merging current checkbox 3 into the split). 1.3a closes: split-checkbox-1a + checkbox 2 + checkbox 4. 1.3b closes: split-checkbox-1b + checkbox 5. The Step 1.3 heading gets `✅` after 1.3b. ARCHITECTURE.md text edits are bundled into the 1.3a coder commit. Sub-question (Current State shape): the 1.3a Current State entry records in-memory MptState shipped, StubState deleted, Journal::apply tests rewritten, Step 1.3 partially complete (1.3a done, 1.3b pending). Mirrors the 1.2a/1.2b Changelog pattern.

---

### Decision 11 — Coder workflow directives to encode in the 1.3a dispatch

Three workflow questions the 1.3a coder-dispatch prompt must answer unambiguously.

**(A) Git commit policy.** AGENTS.md Workflow & Conventions (first added in the session that
completed Step 1.2) now states: "Coding agents do NOT run `git commit`. The coder's job ends
when verification passes ... The coder produces a **proposed commit message** — the maintainer
reviews the Outcomes and runs `git commit` themselves."

Options:
- **(a1) Encode verbatim in the coder dispatch** — the dispatch prompt quotes the relevant
  paragraph from AGENTS.md verbatim. Belt-and-suspenders, but the coder also reads AGENTS.md
  at session start (per AGENTS.md header) so it gets the rule twice.
- **(a2) Rely on AGENTS.md read** — AGENTS.md is the canonical source; the planner trusts the
  coder to follow it. The dispatch prompt does not repeat the rule.
- **(a3) Add a short explicit instruction at the top of the execution plan** — "IMPORTANT: Do
  not run `git commit`. Stage files if useful for verification; leave `git commit` to the
  maintainer." Clear, brief, not duplicating the full paragraph.

**(B) Library Verification Protocol.** Does 1.3a require any Context7 queries?

For 1.3a's in-memory backend: `krax-types` (path dep, no LVP), `alloy-primitives` (already
verified in 1.1a, low priority), `thiserror` (already verified in 1.1a), `rstest` and
`pretty_assertions` (verified in 1.2b via `cargo search`). The in-memory backend uses only
stdlib (`BTreeMap`, `Clone`) plus types from `krax-types`. **No tier-1 or tier-2 library uses
are anticipated for 1.3a's in-memory scope** — unless Decision 6 chooses option (c1) (alloy-trie),
which is a new dep requiring a Context7 medium-priority query.

The coder-dispatch prompt should state: "No Context7 queries are required for 1.3a unless
Decision 6 resolved to option (c1) — alloy-trie. If (c1): query Context7 for `alloy-trie` before
writing any MPT root code."

**(C) Single vs. two commits for 1.3a.** Should 1.3a land as one commit (in-memory MptState +
Journal::apply test rewrite together) or two commits (MptState struct + impl in commit 1,
Journal::apply rewrite in commit 2)?

Options:
- **(c1) Single commit** — the two parts are tightly coupled (the journal test rewrite requires
  MptState to exist). One commit with a compound message is the simpler shape.
- **(c2) Two commits** — `feat(state): implement in-memory MptState — Step 1.3a` then
  `refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a`. Cleaner git
  history; the second commit is purely a test migration with no new production code.

**Answer (A) Git commit policy: (a3) brief explicit instruction.** Add a single line at the top of the 1.3a execution plan: "Do not run `git commit`. Stage files via `git add` if useful for verification; commit is the maintainer's action. Report your proposed commit message in the final report." Three sentences max. Full policy paragraph stays in AGENTS.md as canonical source; dispatch prompt does not duplicate it but reinforces it.

**Answer (B) Library Verification Protocol: no Context7 queries required for 1.3a.** All deps used are stdlib (`BTreeMap`, `Clone`) plus types from `krax-types` (already verified). `rstest` + `pretty_assertions` already verified in 1.2b via `cargo search`. Decision 6's answer is (a) placeholder, not (c1) alloy-trie, so no new external trie dep in 1.3a. The dispatch prompt states: "No Context7 queries required for 1.3a."

**Answer (C) Two commits.** Commit 1: `feat(state): implement in-memory MptState — Step 1.3a` (MptState struct + State trait impl + round-trip test + ARCHITECTURE.md/AGENTS.md edits including Step 1.5 insertion). Commit 2: `refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a` (StubState deletion + 3 Journal::apply tests rewritten + empty `mod tests` block removal). Cleaner git history; the second commit is a pure test migration with no new production code.

---

## Open questions for maintainer (not decisions)

1. **`compile_fail` doctest on `Journal::discard` — confirm unaffected.** Per 1.2b Outcomes
   Deviation #1/#2, the doctest is in `Journal::discard`'s `///` comment in `journal.rs`, not
   in `mod tests`. It uses `Journal` directly without any `State` impl. The 1.3a coder should
   leave it untouched. Confirm: is this the maintainer's intent, or should the doctest be updated
   in any way?

   **Answer:** **Unaffected. Leave it.** The doctest verifies consuming semantics on `Journal::discard` and has nothing to do with `StubState` or `MptState`. No edits.

2. **`journal.rs` `mod tests` block removal.** After StubState and the 3 apply tests are deleted
   from `journal.rs`'s `mod tests`, the block may be empty. Per 1.2b Post-execution directive #3:
   "Remove the test module entirely if it becomes empty after deletion." The only item currently
   in `mod tests` is StubState + the 3 apply tests (plus the allow attribute and imports). Confirm:
   empty `mod tests` block should be fully removed, not left as a stub.

   **Answer:** **Remove the whole block, not just the body.** The `#[cfg(test)] mod tests { ... }` declaration, the `#[allow(clippy::unwrap_used)]` attribute, and all the imports inside it go. An empty test module is noise; clean removal is the right shape.

3. **MptState public API surface.** Does `MptState` need to be `pub` from `krax-state`
   (accessible to `bin/*` crates for construction) or is it always constructed behind a factory
   function? For 1.3a, the only consumer is tests. But 1.3b wires it into the node binary.
   Surface: should `MptState::new() -> Self` be a public constructor with a `///` doc comment
   from day one?

   **Answer:** **Yes — `pub`, with `pub fn new() -> Self` and a `///` doc comment from day one.** Even though 1.3a's only consumer is tests, the public surface should be set now to avoid a separate "make it public" refactor commit in 1.3b. The `new()` doc comment explains the in-memory backing (so callers know what they're getting in 1.3a vs 1.3b's MDBX backing).

---

## Cross-step impact summary

| Downstream step | Impact |
|---|---|
| **Step 1.3b** | Inherits: `MptState` struct, `State` impl surface, `Snapshot` impl pattern, decisions on root() and commit() semantics, and the `mpt/mod.rs` module layout chosen in Decision 1. The MDBX backend adds `reth-db` as a dep, I/O `StateError` variants, and restart tests. The shape of the two-layer model (Decision 3) influences how cleanly the MDBX transaction maps onto the `pending`/`committed` semantics. |
| **Step 1.3.5 (Coverage Tooling)** | 1.3a adds a new crate (`krax-state`) with logic — its coverage needs to be measured. The Step 1.3.5 tool selection and `make coverage` target should cover `krax-state` as well as `krax-types`. No action in 1.3a. |
| **Step 1.4 (Snapshot Semantics)** | 1.3a's `MptSnapshot` implements `Snapshot`. Step 1.4 tests snapshot isolation (`s.get(k) == v1` after `state.set(k, v2)` post-snapshot). Decision 4's snapshot shape (clone vs Arc) determines how isolation works in practice. Decision 4 option (a) (clone) gives the cleanest isolation guarantee for Step 1.4's test. |
| **Phase 1 Gate** | The gate requires ">85% coverage on `krax-types` and `krax-state`." 1.3a is the first step to put logic into `krax-state`. Coverage on new code should be at or above the Phase 1 Gate threshold from day one. |
| **Phase 2 (EVM Execution Wrapper)** | Phase 2's `Executor` requires a `State` impl to execute transactions against. `MptState` from 1.3a/1.3b is that impl. Decision 6 (root semantics) matters for Phase 2: the EVM executor will call `root()` or `commit()` to capture post-execution state roots. A placeholder root in 1.3a is fine as long as 1.3b ships a real root before Phase 2 begins. |
| **Phase 14 (Optimistic State Commitments)** | The post-commit root returned by `commit()` is posted to Ethereum L1. Decision 6 locks in whether real MPT root computation lands in 1.3a, 1.3b, or a later step. |

---

## Decisions about future steps surfaced here (per workflow principle)

The workflow principle applied: **when a step explicitly defers work to a later named step, the deferred step's decisions affecting the current step or any pending step are surfaced at the deferral point, not at the deferred step's dispatch.** Three precedents establish this pattern: `arrival_time` source in 1.1b, coverage tooling in 1.3.5, and now MPT root computation in 1.5. The principle is being added to AGENTS.md as part of the 1.3a coder commit.

### Step 1.5 — MPT Root Computation (slot reserved)

**Existence and position:** A new Step 1.5 is inserted between Step 1.4 (Snapshot Semantics) and the Phase 1 Gate. Its scope is to replace the `B256::ZERO` placeholder in `MptState::root()` with real Ethereum-compatible MPT root computation. Until Step 1.5 ships, the `MptState` carries a documented placeholder — honest scaffolding, not a forgotten TODO.

**Order rationale (1.4 → 1.5, not reversed):** Snapshot isolation (Step 1.4) is a slot-level property and can be tested meaningfully with placeholder roots; real root computation (Step 1.5) is higher-risk new math against an external spec. Land the lower-risk work first so the higher-risk work has a stable foundation. Step 1.5's verification gate includes re-running Step 1.4's snapshot tests against real-root `MptState` — the strengthened-tests benefit is recovered cheaply without reordering.

**General pattern named:** Most "do A first to enable better B" arguments are actually arguments for `A → B → re-verify A`. Reordering is only required when A genuinely can't be done meaningfully without B already in place. Snapshot tests can; placeholder roots don't invalidate isolation checks. Order follows risk.

### Decision pre-surfaced for Step 1.5: alloy-trie vs custom MPT computation

This decision is pre-surfaced because it has cross-cutting implications for 1.3a and 1.3b (workspace dep declaration, mental model of "what's the trie shape underneath"). The rest of Step 1.5's decisions (RLP encoding details, proving infrastructure interaction, hash-of-which-encoding choices) wait for 1.5's normal decisions round.

**Question:** Does Step 1.5 implement MPT root computation via the `alloy-trie` external crate, or as a custom Krax-owned MPT implementation?

Options:
- **(c1) `alloy-trie` external dep** — add `alloy-trie` to `[workspace.dependencies]`. Less code to write; off-the-shelf correctness against Ethereum spec. Rule 10 justification: "Ethereum trie computation, used in Phase 14 commitment posting." Library Verification Protocol applies (medium priority, first-use).
- **(c2) Custom Krax-owned MPT implementation** — implement MPT root in `crates/krax-state/src/mpt/root.rs` (or similar) using only `keccak256` from `alloy-primitives`. More code; full control over commitment shape. Aligns with AGENTS.md "our own MPT layer on top where needed" language.

**Constraints and implications:**
- AGENTS.md "our own MPT layer" language reads as load-bearing for V1: an external trie dep at V1's root creates V2 unwind cost (V2's LSM-native state commitment needs to be free of V1's MPT shape; an external dep coupling V1's root to alloy-trie's internal model creates a binding that V2 must explicitly break).
- 1.3a workspace-dep implications: if (c1), `alloy-trie` lands in `[workspace.dependencies]` now even though it's not used yet, so 1.5 doesn't need to introduce a new workspace dep. If (c2), nothing new in workspace deps; `alloy-primitives` already exposes `keccak256`.
- 1.3b workspace-dep implications: equivalent to 1.3a. The dep landing point (1.3a, 1.3b, or 1.5) is a planner-round question for Step 1.5; this decision only fixes the *direction*.

**Maintainer lean (not a pre-resolution):** Option (c2) custom MPT. The reasoning is the load-bearing AGENTS.md language plus the V2 unwind cost concern. The planner surfaces this as a real decision at 1.5 dispatch with both options laid out properly; the maintainer answers definitively there.

**Answer at 1.5 dispatch time:** _(deferred to Step 1.5 decisions round)_

---

## Workflow principle to encode in AGENTS.md (1.3a coder commit)

The following text is appended to AGENTS.md "Workflow & Conventions" as part of the 1.3a coder commit. Three applications now justify the formalization.

> **Deferred work surfaces decisions early when they affect the deferral point.** When a step explicitly defers work to a later named step (e.g. "implement real X in Step N+k"), any decision belonging to that future step which affects code or design choices in the current step — or in any intervening step — is surfaced *at the deferral point*, not at the deferred step's dispatch round. The current step's decisions doc lists the affected future-step decisions in a dedicated section (or surfaces them inline if they need maintainer answers before the current step's planner round proceeds).
>
> Surfacing the full future-step decision set early is over-eager — only decisions with cross-step impact get pulled forward; everything else waits for the deferred step's normal dispatch round.

Prior applications of this principle: 1.1b Decision 2 (arrival_time deterministic source surfaced for Phase 3); 1.2 Decision 8 (Step 1.3.5 coverage tooling slot reserved); 1.3a Decision 6 (Step 1.5 MPT root computation slot reserved).

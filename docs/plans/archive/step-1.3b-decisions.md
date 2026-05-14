# Step 1.3b Decisions — MDBX-Backed MptState + Restart Tests

Status: ✅ Answered 2026-05-12
Date surfaced: 2026-05-12

## Context

Step 1.3b closes the remaining two unchecked items in ARCHITECTURE.md Step 1.3:
**"Wire MDBX as the durable backend"** and **"Restart test: open DB, set, commit,
close, reopen, get returns committed value."** It is the second commit-group of
the MPT State Backend Skeleton; Step 1.3a shipped the in-memory `MptState` (single
`BTreeMap<B256, B256>`, eager-clone snapshot, no-op `commit`) and 4 tests
(`set_then_get_round_trips` + 3 `Journal::apply` tests).

The load-bearing carryovers from 1.3a are (a) the public surface
(`pub mod mpt; pub use mpt::{MptSnapshot, MptState};`), (b) `MptState::new() -> Self`
(currently infallible, derives `Default`), (c) the no-op `commit` semantic — writes
visible to `get` without a prior commit, and (d) `MptSnapshot` owning a cloned
`BTreeMap`. Each of these has to be reconciled against MDBX's native model
(durable open-by-path, explicit RoTxn/RwTxn lifecycles, fallible I/O). 1.3b also
opens the door to two additions deliberately deferred from 1.3a: real
`StateError` I/O variants (1.1a Decision 7 left `#[non_exhaustive]` precisely for
this), and a real test-fixture story for filesystem-backed tests.

What is **out of scope** for these decisions: real MPT root computation
(Step 1.5; `root()` continues to return `B256::ZERO` with the `// TODO Step 1.5`
marker), snapshot isolation tests + `compile_fail` doctest (Step 1.4), coverage
tooling (Step 1.3.5), and any new types in `krax-state` or new traits in
`krax-types`. Where a 1.3b decision constrains 1.4 or 1.5, the implication is
called out inside the decision rather than surfaced as a separate downstream
decision.

---

## Open decisions

### Decision 1 — In-memory backing: replace, gate, or coexist?

How does `MptState` accommodate MDBX given that 1.3a shipped a pure-memory
`BTreeMap<B256, B256>` backing? Does the in-memory implementation survive?

Options:
- **(a) Replace entirely.** Drop the `BTreeMap` field; `MptState` becomes
  MDBX-only. `MptState::new()` either disappears (replaced by
  `MptState::open(path: &Path) -> Result<Self, StateError>`) or becomes a
  tempdir-backed convenience for tests. Cleanest single-implementation model;
  matches "boring beats clever for V1."
- **(b) Internal `Backend` trait.** `MptState` becomes generic over a private
  `Backend` trait with `InMemoryBackend` and `MdbxBackend` impls. Both backings
  run the same trait tests. Adds an internal abstraction layer; risks
  pre-building V2 shape (Design Principle 6 — phase boundaries).
- **(c) Two public structs.** Keep 1.3a's struct as `InMemoryMptState`
  (`#[cfg(test)]`-gated or `pub` in `krax-state`); add a new MDBX-backed
  `MptState`. Grows public surface; breaks Rule 8 ("V1 mpt code MUST NOT export
  anything beyond what's required by the trait") unless the in-memory variant
  is test-only.
- **(d) Replace + test constructor.** `MptState::new()` removed; production
  constructor is `MptState::open(path: &Path)`; an additional
  `MptState::open_temporary() -> Result<(Self, tempfile::TempDir), StateError>`
  (or similar) wires a tempdir under the hood for cheap test construction. The
  TempDir is returned so the caller controls drop ordering.

**Constraints from prior steps:**
- 1.3a Decision 1 (flat re-export, `pub MptState`, `pub fn new()`).
- 1.3a Decision 3 (single `BTreeMap` backing — explicitly flagged as
  reconsiderable at 1.3b).
- AGENTS.md Rule 8 (V1 mpt code does not export beyond trait requirements).
- AGENTS.md Design Principle 1 (boring beats clever); Principle 6 (no V2
  pre-abstraction).

**Knock-on for existing tests:** the four 1.3a tests
(`set_then_get_round_trips`, `apply_empty_journal_leaves_state_unchanged`,
`apply_single_entry_writes_slot`, `apply_last_write_wins_on_same_slot`)
construct `MptState::new()`. Under (a) and (d) every test rewrites to the new
constructor. Under (b) and (c) the existing tests can target the in-memory
backend unchanged; MDBX gets its own test set.

**Knock-on for `Default`:** 1.3a's `#[derive(Default)]` on `MptState` becomes
nonsensical under (a)/(d) — no sensible default path. (b) and (c) might keep it.
Either way the answer here drives whether `Default` survives.

**1.4 implications:** Step 1.4's snapshot-isolation tests want a cheap
construction path with no real filesystem cost. (b) and (d) preserve this
cheaply; (a) makes every Step 1.4 test pay a tempdir create+drop; (c) lets
Step 1.4 test against the in-memory variant only (risk: divergence from the
production backend).

**Answer:** **(d) Replace + test constructor.** Production constructor is `MptState::open(path: &Path) -> Result<Self, StateError>`. Test-only `MptState::open_temporary() -> Result<(Self, TempDir), StateError>` returns the `TempDir` so the caller controls drop ordering. The `BTreeMap<B256, B256>` field from 1.3a is replaced by an MDBX env handle. `#[derive(Default)]` on `MptState` is removed — no sensible default path. All four 1.3a tests rewrite to `MptState::open_temporary()`. Rejected (a) because Step 1.4's isolation tests would pay cumulative tempdir cost; (b) because internal `Backend` trait pre-builds V2 abstraction (Principle 6); (c) because growing public surface violates Rule 8.

---

### Decision 2 — Constructor signature and fallibility

What is the production constructor's signature, and is it fallible?

Options:
- **(a) `MptState::open(path: &Path) -> Result<Self, StateError>`.** Single
  production constructor; MDBX `open` errors surface as `StateError`. Matches
  reth-db's likely API shape (Context7-verifiable).
- **(b) `MptState::open(path: impl AsRef<Path>) -> Result<Self, StateError>`.**
  Ergonomic — accepts `&str`, `&Path`, `PathBuf`. Trade: marginally wider
  generic surface, no semantic difference.
- **(c) `MptState::open(config: MptConfig) -> Result<Self, StateError>`.**
  Wrap path in a config struct now to anticipate map-size, sync-mode,
  read-only mode flags. Risks Design Principle 6 if those flags aren't needed
  for V1.
- **(d) Builder pattern.** `MptState::builder().path(...).build()`. Probably
  overkill for V1.

**Constraints from prior steps:**
- 1.3a Open Question 3 (settled — 1.3a used infallible `pub fn new()`).
- AGENTS.md Rule 3 (errors are typed; library crates return their own
  `thiserror` enum — here, `StateError`).

**Knock-on:** Whether `MptState` continues to derive `Default` (almost
certainly no under any fallible constructor) — flag this as a follow-on or fold
into Decision 1's answer.

**Answer:** **(a) `MptState::open(path: &Path) -> Result<Self, StateError>`.** Single production constructor; `&Path` not `impl AsRef<Path>` — the ergonomic win is small and the wider generic surface bloats API docs. `#[derive(Default)]` is removed (folded into Decision 1's answer). Rejected (c) `MptConfig` and (d) builder as premature — no map-size / sync-mode / read-only flags are V1 concerns yet; `#[non_exhaustive]` on a future `MptConfig` accommodates them when needed.

---

### Decision 3 — `snapshot()` impl: RoTxn vs clone vs hybrid vs defer

How does `MptState::snapshot()` produce an isolated read view once MDBX is the
backing store?

Options:
- **(a) MDBX read-only txn.** `snapshot()` opens a `reth-db` read-only
  transaction at the current state. `MptSnapshot` owns the `RoTxn` (or the
  reth-db equivalent — Context7-verifiable). Reads go through the txn. Native
  MVCC isolation. `release(self: Box<Self>)` aborts the RoTxn on drop. This is
  the architectural "right answer."
- **(b) Keep clone-the-map.** Walk the entire MDBX table at `snapshot()` time
  into a `BTreeMap<B256, B256>` owned by `MptSnapshot`. Simple but does not
  scale — the clone IS the state, in the limit. Preserves 1.3a Decision 4
  unchanged.
- **(c) Hybrid.** If Decision 1 picks (b)/(c), in-memory backends clone and
  MDBX backends use RoTxn. Maximally flexible; complexity tax.
- **(d) Defer to Step 1.4.** Ship the cheapest viable `snapshot()` in 1.3b
  (clone-the-map, even when backed by MDBX) and pick up the RoTxn rewrite
  alongside Step 1.4's isolation tests. Per "deferred work surfaces decisions
  early" this requires Step 1.4 to inherit the RoTxn decision as a known TODO.

**Constraints from prior steps:**
- 1.1a Decision 1 / 1.1a Decision 6 (`release(self: Box<Self>)` is consuming;
  post-release reads are a compile error, not a runtime check).
- 1.3a Decision 4 (clone-the-map was the V1.3a answer).
- AGENTS.md "State Snapshot" — workers MUST NOT observe each other's
  uncommitted writes; isolation is load-bearing.

**1.4 implications:** This is the most direct cross-step dependency in 1.3b.
- Under (a): Step 1.4 only adds isolation tests + `compile_fail` doctest;
  no implementation change needed.
- Under (b)/(c-clone-side): Step 1.4 carries the RoTxn rewrite *in addition*
  to its tests. Acceptable if the maintainer wants 1.3b minimal.
- Under (d): explicit deferral; Step 1.4 plan must list the rewrite as a
  prerequisite to its isolation tests.

**1.5 implications:** Real MPT root computation in Step 1.5 will want stable
read-only access for trie traversal. If 1.3b picks (a), Step 1.5's traversal
can leverage the same RoTxn pattern. If (b), Step 1.5 must independently
decide its read-path.

**Answer:** **(a) MDBX read-only txn.** `MptSnapshot` owns a reth-db `RoTxn`-equivalent (exact type confirmed via LVP Query 2). Reads go through the txn. `release(self: Box<Self>)` drops the txn (see Decision 11). Step 1.4's isolation tests then add tests on top of the RoTxn-backed snapshot without carrying implementation work. Rejected (d) defer-to-1.4 because clone-the-map against MDBX backing means walking the entire table into a `BTreeMap` on every `snapshot()` call — defeats the durable-storage point and forces Step 1.4 to carry the rewrite anyway; (b) keep clone-the-map for the same reason; (c) hybrid is moot under Decision 1 (d).

---

### Decision 4 — `commit()` semantics under MDBX

1.3a Decision 5 made `commit()` a no-op returning `Ok(self.root())`. MDBX has
explicit write-transaction lifecycles. What does `commit()` do, and when are
writes visible to `get()`?

Options:
- **(a) `commit()` flushes a long-lived RwTxn.** `MptState` holds a pending
  `RwTxn`; `set()` writes into it; `commit()` commits the RwTxn and opens a
  fresh one. Writes are **not** visible to `get()` until `commit()` is called.
  Architecturally clean; breaks the 1.3a tests' assumption that `set` is
  visible to a subsequent `get` without an intervening `commit`.
- **(b) Auto-flush per set.** Each `set()` opens-writes-commits its own
  short-lived RwTxn. `commit()` remains a no-op (or a sync barrier).
  Writes visible to `get()` immediately. Preserves 1.3a test semantics with
  zero changes. Per-write transaction overhead is significant but acceptable
  for V1 skeleton — see AGENTS.md Design Principle 1.
- **(c) Buffered write through `commit()`.** `MptState` holds a
  `BTreeMap<B256, B256>` of pending writes (a survivor of 1.3a Decision 3
  in modified form); `set()` updates the buffer; `get()` reads buffer first,
  then MDBX; `commit()` drains the buffer into a single RwTxn. Best perf
  profile (batched I/O); most code; semantics match 1.3a's "writes visible
  before commit."
- **(d) Strict txn boundary.** Same as (a) but called out as a distinct
  decision because the test changes are non-trivial: every `Journal::apply`
  test must add an explicit `commit()` before its asserting `get()`.

**Constraints from prior steps:**
- 1.3a Decision 5 (no-op `commit`, writes visible immediately).
- AGENTS.md Rule 7 (determinism — MDBX must not introduce non-deterministic
  state under any of these).
- AGENTS.md Design Principle 1 (boring beats clever for V1) — argues for (b).
- krax-types `State::commit` doc-comment says writes are "pending until
  commit," which (a)/(c)/(d) honor literally and (b) honors in spirit
  (per-set commit is still a commit).

**Knock-on for existing tests:** the three `Journal::apply` tests in
`mpt/mod.rs` currently do **not** call `commit()` between `apply` and the
asserting `get()`. Under (a)/(d) they must. Under (b)/(c) they need not.

**Knock-on for `root()`:** still `B256::ZERO` in 1.3b (Step 1.5 unchanged).
None of these options affect the placeholder.

**1.5 implications:** Real root computation in Step 1.5 will want a clearly
defined "what's committed vs pending" boundary. (a)/(c)/(d) provide that
explicitly; (b) does so implicitly (every set is committed). The maintainer's
1.3a-era "small, honest, deletable" preference may favor (b) for 1.3b and
let Step 1.5 introduce buffering if it actually needs it.

**Answer:** **(b) Auto-flush per set.** Each `set()` opens-writes-commits its own short-lived RwTxn. `commit()` becomes a sync barrier returning `Ok(self.root())` — semantically equivalent to 1.3a's no-op. Writes visible to `get()` immediately, preserving 1.3a's test assumptions: the 3 `Journal::apply` tests need NO changes; the round-trip test's `state.commit()` call survives as a no-op sync barrier. Decision 9 option (c) (commit-uncommitted distinction) becomes untestable under this answer and is dropped. Rejected (a)/(d) because long-lived RwTxn on `MptState` introduces self-referential lifetime issues (txn borrows env, env owned by state); (c) buffered-write because it's a V1.5/V2-shaped performance optimization that Principle 1 forbids pre-building. If Phase 17 benchmarking shows per-set txn overhead is unacceptable, (c) is the natural reactive optimization with real data behind it.

---

### Decision 5 — `StateError` extension granularity

`StateError` currently has one variant: `Released`. `#[non_exhaustive]` was put
there specifically to accommodate Step 1.3 additions (1.1a Decision 7). What
gets added?

Options:
- **(a) Single catch-all.** One variant wrapping the reth-db / MDBX error type:
  `Io(#[from] reth_db::DatabaseError)` (or whichever type Context7
  verification confirms). Small surface, future-proof, opaque to callers.
- **(b) Granular categories.** Separate variants per category — e.g.
  `Io { source: ... }`, `Corruption { ... }`, `NotFound { ... }` (if MDBX
  surfaces "key absent" distinctly from txn error). Callers can match on
  cause. Tied to MDBX's specific error surface, less portable to V2 LSM.
- **(c) Source-wrapped boxed error.**
  `Io { source: Box<dyn std::error::Error + Send + Sync + 'static> }`.
  Storage-agnostic; loses static type info; matches Rule 8 (drop-in V2
  replacement).
- **(d) Two variants: `Open` + `Io`.** Split open-time errors (path doesn't
  exist, env can't be created) from runtime I/O. Lets callers distinguish
  "did this state ever start up" from "did this read fail."

**Constraints from prior steps:**
- 1.1a Decision 7 (`#[non_exhaustive]`; additions don't break consumers).
- AGENTS.md Rule 3 (typed errors via `thiserror`, wrap with context).
- AGENTS.md Rule 8 (V2 LSM must be a drop-in replacement — argues against
  exposing MDBX-specific structure publicly; argues toward (a) or (c)).
- `State: Send + Sync` (1.1a) — the error type must be `Send + Sync`.

**Knock-on:** Decision 4 affects which methods can produce I/O errors:
under (a)/(c)/(d) only `set` and `commit` plausibly fail; under (b) per-set
auto-flush, every method can fail. Either way the trait signatures already
return `Result<_, StateError>`, so this is purely an enum-shape question.

**Answer:** **(c) Source-wrapped boxed error (maintainer revision 2026-05-12).** Add ONE variant: `Io(#[source] Box<dyn std::error::Error + Send + Sync>)`. Provide a `pub fn io<E: Error + Send + Sync + 'static>(source: E) -> Self` constructor in `krax-types/src/state.rs`. Call sites in `krax-state` wrap reth-db errors via `.map_err(StateError::io)?` (NOT `?` directly — no `#[from]` to avoid blanket-impl conflicts with the constructor). `#[non_exhaustive]` retained.

**Why this revision** (departure from original (a) single-catch-all-via-`#[from]` recommendation): the original answer would have forced `krax-types` to depend on `reth-db` directly because `StateError::Io(#[from] reth_db::DatabaseError)` references the reth-db error type in `krax-types`'s public API. That pulls MDBX native code + reth's transitive dep graph into `krax-types`, increasing its build-graph weight significantly. The boxed-source variant keeps `krax-types` storage-backend-agnostic — V2 LSM substitutes its own error wrapper without touching this enum. Trade-off: callers cannot statically downcast to `reth_db::DatabaseError`, but no consumer needs that today; downcasting via `error.source().downcast_ref::<reth_db::DatabaseError>()` remains possible reactively. Originally-rejected (b) granular, (d) split Open/Io still rejected for the same reasons (premature — no consumer needs the distinction). **No LVP Query 5 dependency** under this answer — the planner doesn't need to confirm `reth_db::DatabaseError`'s name or `Send + Sync + 'static` status as part of the type signature; the Box<dyn Error + Send + Sync> bound is satisfied by anything that implements std::error::Error appropriately, which reth-db's error types do by convention.

---

### Decision 6 — MDBX table layout: flat slot table vs trie-shaped now

What does the on-disk schema look like?

Options:
- **(a) Flat slot table.** One table: `B256 → B256`, keyed by slot, value is
  the slot value. Mirrors the in-memory `BTreeMap<B256, B256>` shape exactly.
  Step 1.5 layers trie structure on top (or defines additional tables) when
  real root computation lands. Matches 1.3a Decision 6 (placeholder root).
- **(b) Trie-shaped now.** Define a nodes table keyed by node hash, value is
  the encoded trie node. `set`/`get` walk the trie. Step 1.5 fills in root
  computation but the storage layer is already trie-shaped. More code,
  riskier (the in-memory equivalent in 1.3a was rejected as "should not
  pre-build V2 shape").

**Constraints from prior steps:**
- 1.3a Decision 6 (root placeholder; deferred to 1.5).
- AGENTS.md Design Principle 1 (boring beats clever for V1).
- Step 1.5's alloy-trie-vs-custom-MPT decision is pre-surfaced and will be
  answered at 1.5 dispatch — option (b) here forces partial pre-resolution.

**1.5 implications:** Under (a), Step 1.5 freely picks alloy-trie OR custom
MPT and adds whichever tables it needs. Under (b), the table layout is
already trie-shaped — locks Step 1.5 toward a custom-MPT-on-this-shape
implementation, or forces a table migration to swap in alloy-trie.

**Answer:** **(a) Flat slot table.** One table: `Slots: B256 → B256`. Mirrors the in-memory `BTreeMap<B256, B256>` shape exactly. Step 1.5 adds whatever tables it needs for real root computation (alloy-trie vs custom MPT, pre-surfaced and answered at 1.5 dispatch). Rejected (b) trie-shaped now because it pre-builds Step 1.5's storage shape (Principle 6) and partially pre-resolves the alloy-trie-vs-custom-MPT decision through the back door — that decision is explicitly deferred to 1.5 dispatch.

---

### Decision 7 — Custom table definition mechanism

reth-db tables are typically defined via a macro (`reth_db::tables!` or a
successor — Context7-verifiable). For 1.3b's slot table, what shape does the
table definition take?

Options:
- **(a) Use reth-db's macro directly.** Define a new `Slots: B256 -> B256`
  table via the current macro in `crates/krax-state/src/mpt/`. Krax's table
  lives in its crate; no reth-db fork.
- **(b) Hand-roll the key/value codec.** Implement reth-db's `Table` /
  `Compress` / `Decompress` traits by hand on a wrapper newtype. More code,
  no macro dependency, easier to read at audit time.
- **(c) Reuse an existing reth-db table.** reth-db likely already defines a
  storage-trie table (`PlainStorageState` or successor — Context7-verifiable).
  Risk: that table's key type is `(Address, StorageKey)`, not a raw `B256`,
  which doesn't match the State trait's flat `B256 → B256` shape.

**Constraints from prior steps:**
- Library Verification Protocol — reth-db is tier-1; the actual macro/API
  surface in the pinned rev must be Context7-verified before code is written.
- AGENTS.md Rule 8 — the table is a 1.3b implementation detail; it does
  not leak through the `State` trait.

**Knock-on:** Decision 6's outcome partly determines this — (b) "trie-shaped
now" requires nodes-table not slots-table. The two decisions are answered
together but kept separate so the table-definition mechanism is a deliberate
pick rather than implied.

**Answer:** **(a) Use reth-db's macro directly** (whichever macro/API LVP Query 3 confirms is current). Krax's `Slots: B256 → B256` table lives in `crates/krax-state/src/mpt/`. Rejected (b) hand-roll because reth-db's macro exists precisely because the `Table`/`Compress`/`Decompress` traits are mechanical — hand-rolling means we own bytecode-format decisions reth-db has already made correctly; (c) reuse existing reth-db tables because existing tables (e.g. `PlainStorageState`) key by `(Address, StorageKey)` not raw `B256` — forcing the shape to fit by using `Address::ZERO` is exactly the clever-not-boring move Principle 1 forbids. **Fallback if Query 3 reveals the macro has been renamed or removed in Reth 2.0:** hand-roll (option b) as the deviation; surface in Outcomes.

---

### Decision 8 — Test fixture: `tempfile` dep vs alternatives

Restart tests need a real filesystem path with reliable cleanup. What does
test setup use?

Options:
- **(a) `tempfile::TempDir`.** Standard, RAII cleanup on drop. Adds
  `tempfile` to the workspace `[workspace.dependencies]` table **and** to the
  AGENTS.md Rule 10 approved-dep list (under test-only, alongside
  `proptest`, `rstest`, `pretty_assertions`).
- **(b) Hand-rolled tempdir.** Build paths like `/tmp/krax-test-{uuid}/`
  manually; ensure cleanup with a drop guard. Avoids new dep at the cost
  of fragile cleanup on test-panic (drop guards don't always run cleanly
  under `--no-capture` + panics).
- **(c) reth-db bundled test helper.** If reth-db exposes a
  `test_utils::create_test_db` or similar (Context7-verifiable), use it.
  Avoids new dep entirely. Risk: tied to reth-db's internal test surface,
  which may not be stable.

**Constraints from prior steps:**
- AGENTS.md Rule 10 — new deps need justification in the commit message;
  test-only deps are listed at the bottom of the approved set. `tempfile`
  is currently NOT in that list, so adding it requires an AGENTS.md edit
  in the same commit.
- AGENTS.md Rule 7 — UUID-named paths are fine because they are test-only
  and do not enter state.

**Knock-on for AGENTS.md edits:** under (a), the Rule 10 approved-dep list
gets a new line. Surface this so the eventual plan can spec the edit.

**Answer:** **(a) `tempfile::TempDir`.** Add `tempfile = { workspace = true }` to `crates/krax-state/Cargo.toml`'s `[dev-dependencies]`. Add `tempfile = "3"` (latest stable version confirmed by coder via `cargo search tempfile` at LVP-equivalent time) to workspace root `Cargo.toml`'s `[workspace.dependencies]` under the test-only group. Edit AGENTS.md Rule 10's test-only approved-dep list to add `tempfile` (one-line append alongside `proptest`, `rstest`, `pretty_assertions`). This Rule 10 edit lands in the same commit as the dep addition (Commit 2, with the test). Rejected (b) hand-rolled tempdir because drop guards under test panics are unreliable; (c) reth-db bundled test helper because if it exists it likely just wraps `tempfile` internally — adding `tempfile` directly is more explicit about what we depend on.

---

### Decision 9 — Restart test shape

What does the restart test actually verify? Multiple combinations possible.

Options (combinable):
- **(a) Strict single-key restart.** Open, `set(k, v)`, `commit()`, drop the
  `MptState`, reopen at the same path, `get(k) == v`. Minimum bar; closes
  the ARCHITECTURE.md checkbox literally.
- **(b) Multi-write restart.** Open, set k1 and k2, commit, drop, reopen,
  get both. Tests that multiple writes survive, not just one. Cheap
  extension of (a).
- **(c) Commit-uncommitted distinction.** Open, set k1, commit; set k2
  (no commit); drop. Reopen. `get(k1)` returns the committed value;
  `get(k2)` returns `B256::ZERO`. Tests the commit boundary, not just
  persistence. Only meaningful under Decision 4 options (a)/(c)/(d) —
  under (b) auto-flush, every set is durable and the test trivially passes.
- **(d) Process-boundary restart.** Spawn a subprocess to write; assert
  in the parent. Overkill for unit tests; belongs in integration only.
  Surfacing as a flag-and-confirm.

**Constraints from prior steps:**
- ARCHITECTURE.md Step 1.3 fourth checkbox specifies: "open DB, set, commit,
  close, reopen, get returns committed value." Literally option (a).

**Knock-on with Decision 4:** if commit is a no-op under (b) "auto-flush per
set," option (c) here is untestable (and a test that's tautologically true
under one Decision 4 outcome is a smell). Recommend the maintainer answer
Decision 4 first; (c) here is contingent on that answer.

**Answer:** **(a) + (b).** Two tests in `tests/restart.rs`: (a) strict single-key restart (open, set, commit, drop `MptState`, reopen at same path, get returns committed value — closes the ARCHITECTURE.md checkbox literally) AND (b) multi-write restart (set k1 and k2, commit, drop, reopen, get both — catches single-key serialization bugs that wouldn't surface with one slot). Rejected (c) commit-uncommitted distinction because Decision 4 (b) auto-flush makes every set durable — the distinction doesn't exist for us at this layer. Rejected (d) process-boundary restart as overkill; drop-and-reopen within a single test process is sufficient for durability verification.

---

### Decision 10 — Test location: inline `mod tests` vs `tests/` integration

Where do 1.3b's tests live, and behind what gate?

Options:
- **(a) Inline `#[cfg(test)] mod tests` in `mpt/mod.rs`.** Same location
  as 1.3a's four tests. `tempfile` becomes a dev-dep. Tests run under
  plain `cargo test` / `make test`. Simplest; matches the test-locality
  pattern AGENTS.md Rule 5 establishes.
- **(b) Integration test in `crates/krax-state/tests/restart.rs` behind
  the `integration` feature.** Matches AGENTS.md Rule 5 wording: "Integration
  tests live in each crate's `tests/` directory and are gated behind a
  `integration` feature flag where they require external resources (anvil,
  MDBX)." MDBX is named explicitly. Runs only under `make test-integration`.
  Required Cargo.toml edit: list the test under `[[test]]` with
  `required-features = ["integration"]`.
- **(c) Both.** Quick smoke test inline (so plain `cargo test` exercises
  the MDBX wiring at least once); full restart test in `tests/` behind
  `integration`. Most coverage, most boilerplate.

**Constraints from prior steps:**
- AGENTS.md Rule 5 — explicit framing of MDBX-touching tests as integration.
- 1.3a Decision 8 (tests co-located in `mpt/mod.rs`) was a 1.3a-scope
  decision; not binding on 1.3b.
- The `integration` feature already exists in `crates/krax-state/Cargo.toml`
  as a placeholder (`integration = []`), waiting for its first use.

**Knock-on for CI:** if (b), 1.3b's restart test will not run under
`make test` (default `cargo test`). The Phase 1 Gate line item ("MPT state
backend passes round-trip and restart tests") still closes because
`make test-integration` covers it, but the maintainer should be aware that
the default test run no longer exercises persistence.

**Answer:** **(b) Integration test in `crates/krax-state/tests/restart.rs` behind the `integration` feature.** Matches AGENTS.md Rule 5's explicit MDBX framing. `crates/krax-state/Cargo.toml` gets a `[[test]] name = "restart", required-features = ["integration"]` entry. `make test-integration` runs it; `make test` does not. The 1.3a inline tests (round-trip + 3 apply tests in `mpt/mod.rs`'s `mod tests`) stay where they are — they don't touch external resources. Rejected (a) inline because filesystem-touching tests in default `cargo test` is exactly what Rule 5's integration-feature framing was designed to avoid; (c) both because duplicates work without meaningful additional coverage. **Acknowledged trade-off:** the default `make test` run no longer exercises MDBX. Phase 1 Gate closure still works because `make test-integration` covers it; the planner's gate-line check (Decision 13) must verify the integration test passed.

---

### Decision 11 — `MptSnapshot` durability and `release` under MDBX

If Decision 3 picks (a) "RoTxn-backed snapshot," `MptSnapshot` owns a reth-db
read-only transaction. What is `release(self: Box<Self>)`'s exact behavior?

Options:
- **(a) Drop aborts the RoTxn.** `release` simply drops the `Box<Self>`;
  reth-db's `RoTxn`/equivalent drops, which releases MDBX reader slot
  resources implicitly. Idiomatic.
- **(b) Explicit `RoTxn::abort()` in `release`.** Call the explicit
  abort method (if it exists per Context7) before drop. Provides a
  surface for any cleanup logging or failure handling.
- **(c) Defer because Decision 3 is (b) or (d).** If snapshots clone the
  map, `release` remains the 1.3a no-op (Box dropped, owned map freed).
  This option is only viable if Decision 3 doesn't pick (a).

**Constraints from prior steps:**
- 1.1a Decision 1 / Decision 6 (consuming `release(self: Box<Self>)`;
  post-release reads are a compile-time error).
- Rule 3 (no `unwrap`, errors are typed) — if reth-db's `abort` is
  fallible, the trait signature `fn release(self: Box<Self>)` (no return
  value) means errors must be swallowed or logged via `tracing::error!`.
  Surface as a sub-question.

**Sub-question (if Decision 3 = (a) and reth-db's `abort` is fallible):**
how is an abort failure reported? Options: silent drop (rely on RAII),
`tracing::error!`-and-drop, or change the `Snapshot` trait to return
`Result<(), StateError>` (would re-open 1.1a's trait — heavy).

**Answer:** **(a) Drop aborts the RoTxn.** `release(self: Box<Self>)` body is a no-op (or contains only a doc comment); the `Box<Self>` is dropped on return, the `RoTxn` field drops, MDBX releases the reader slot via reth-db's `Drop` impl. Idiomatic. Rejected (b) explicit `RoTxn::abort()` because if Drop already releases slots cleanly (idiomatic Rust), explicit call adds nothing and `tracing::error!` on abort failure is over-engineering for a code path never expected to fail; (c) moot under Decision 3 (a). **Sub-question answer:** if LVP Query 8 reveals `RoTxn::abort()` is fallible, silent-drop (rely on RAII). Do NOT change the `Snapshot` trait — re-opening 1.1a's trait signature for an edge case that's never expected to fire is the heavy option the planner correctly flagged.

---

### Decision 12 — Commit boundary: one commit vs two vs three

How is 1.3b's work sequenced into git commits?

Options:
- **(a) Single commit.** `feat(state): wire MDBX backend + restart test —
  Step 1.3b`. Atomic, simple history.
- **(b) Two commits.** First: `feat(state): wire MDBX backend for MptState
  — Step 1.3b`. Second: `test(state): add MDBX restart test — Step 1.3b`.
  Matches 1.3a's two-commit pattern (settled there as 1.3a Decision 11C).
  The second commit is purely test code.
- **(c) Three commits.** Cargo.toml + AGENTS.md edits → MDBX wiring →
  test. Probably over-granular.

**Constraints from prior steps:**
- 1.3a Decision 11C — two-commit pattern is acceptable when the seam is
  natural.
- AGENTS.md "Coding agents do NOT run `git commit`" — the coder
  produces a proposed commit message; the maintainer commits.

**Knock-on:** If Decision 10 picks (b)/(c) "integration test," the test
commit also touches Cargo.toml (`[[test]]` entry, `required-features`) —
still cleanly a "test" commit.

**Answer:** **(b) Two commits.** Commit 1: `feat(state): wire MDBX backend for MptState — Step 1.3b` (MDBX env wiring, `MptState::open`, `MptState::open_temporary`, table definition, `impl State` rewrite, `impl Snapshot` RoTxn rewrite, `StateError::Io` variant, dep additions including `tempfile` to workspace, ARCHITECTURE.md edits, AGENTS.md Current State rewrite, AGENTS.md Rule 10 `tempfile` addition). Commit 2: `test(state): add MDBX restart test — Step 1.3b` (Cargo.toml `[[test]]` entry, `tests/restart.rs` with the two restart tests, the four existing inline 1.3a tests rewritten to `MptState::open_temporary()`). Rejected (a) single commit because the seam between production code and test code is natural and reviewable in isolation; (c) three commits because dep additions bundle cleanly into Commit 1 alongside the wiring that depends on them.

---

### Decision 13 — Phase 1 Gate closure attribution

The Phase 1 Gate line item reads: *"MPT state backend passes round-trip and
restart tests."* Round-trip was satisfied at 1.3a; restart is satisfied at
1.3b. Does this gate line close at 1.3b, or does it wait for Step 1.5 to
land (since "MPT state backend" arguably implies a real root)?

Options:
- **(a) Close at 1.3b.** The line item maps to two literal tests, both of
  which will exist post-1.3b. Step 1.5's separate gate line is
  "Real MPT root computation in place."
- **(b) Wait for 1.5.** "MPT state backend" is read as the whole backend,
  including real root. Gate line carries an unchecked status through
  1.3.5 and 1.4 until 1.5 closes both at once.

**Constraints from prior steps:**
- ARCHITECTURE.md (post-1.3a) — Phase 1 Gate has both bullets; they are
  listed separately, which textually supports (a).
- 1.3a's planner already wrote the "Real MPT root computation in place"
  bullet as separate gate item.

**Knock-on for the eventual ARCHITECTURE.md edit:** under (a), 1.3b's
plan flips the round-trip+restart gate line to ✅ in the same commit
that closes Step 1.3. Under (b), no gate-line edit in 1.3b.

**Answer:** **(a) Close at 1.3b.** The gate line item "MPT state backend passes round-trip and restart tests" maps to two literal tests, both of which exist post-1.3b. Step 1.5's separate gate line ("Real MPT root computation in place") was deliberately added by the 1.3a planner as its own bullet — keep that separation. Step 1.3 heading gets `✅` in Commit 1's ARCHITECTURE.md edit; the round-trip+restart gate line gets `✅` in the same edit; the real-root gate line stays unchecked until Step 1.5. Rejected (b) wait-for-1.5 because conflating two textually-separate gate items defeats the gate's purpose.

---

## Library Verification checklist

Pre-declared Context7 queries the coder MUST run before writing any
MDBX-touching code. Each is a separate query against the
`paradigmxyz/reth` rev pinned in workspace Cargo.toml
(`02d1776786abc61721ae8876898ad19a702e0070`, dated 2026-05-06).

- **Query 1: reth-db environment open.** How is an MDBX environment opened
  at a filesystem path?
  - **Expected API surface:** something like
    `reth_db::open_db_read_write(path: &Path, args: ...) -> Result<DatabaseEnv, _>`,
    or `DatabaseEnv::open(path, kind, args) -> Result<_, _>` (post-Reth-2.0
    restructure — exact name unverified). Returns an environment handle that
    `MptState` will own.
  - **Fallback if Context7 returns nothing useful:** read the pinned
    rev's `crates/storage/db/src/lib.rs` or equivalent from
    `https://github.com/paradigmxyz/reth` directly (LVP-permitted Cargo
    registry / source fallback).

- **Query 2: read-only and read-write transactions.** How are RoTxn / RwTxn
  obtained from an open environment?
  - **Expected API surface:** `env.tx() -> Result<RoTxn, _>` and
    `env.tx_mut() -> Result<RwTxn, _>` (or the post-2.0 successors).
    `RwTxn::commit() -> Result<_, _>`. Reth uses its own `Database` trait
    abstraction over MDBX, so the surface is reth-named, not raw
    `libmdbx`-named.
  - **Fallback:** rev source as above.

- **Query 3: table definition mechanism.** How is a custom key/value table
  defined? Is `reth_db::tables!` still the macro, or has it been replaced
  in Reth 2.0?
  - **Expected API surface:** declarative macro
    (`reth_db::tables! { table Slots<Key = B256, Value = B256>; }` or
    similar). `Table` / `Compress` / `Decompress` traits on the key and
    value types.
  - **Fallback:** rev source; the `crates/storage/db/src/tables/` directory
    is the canonical reference.

- **Query 4: existing storage tables for slot/account state.** Does reth-db
  define a table like `PlainStorageState` or `StoragesTrie` we can reuse,
  or do we define our own?
  - **Expected API surface:** existing tables likely key by
    `(Address, StorageKey)` not raw `B256`. If so, our `State` trait's
    flat `B256 → B256` shape is incompatible — we define our own. Confirm.
  - **Fallback:** rev source `crates/storage/db/src/tables/mod.rs`.

- **Query 5: reth-db error type.** What's the canonical error type
  surfaced by env-open / tx-create / get / put / commit?
  - **Expected API surface:** `reth_db::DatabaseError` (an enum) — Context7
    will confirm whether it implements `std::error::Error`, is
    `Send + Sync + 'static`, and whether `#[from]` integration with
    `thiserror` is straightforward. Confirm `Send + Sync` to satisfy
    `State: Send + Sync`.
  - **Fallback:** rev source `crates/storage/db/src/database.rs` or
    similar.

- **Query 6: B256 encoding for keys/values.** Does reth-db's `Compress`/
  `Decompress` work for `alloy_primitives::B256` out of the box?
  - **Expected API surface:** reth-db has fixed-byte compression for the
    common Ethereum types; `B256` likely encodes/decodes as 32 raw bytes
    without padding. Confirm and cite — this is the bytewise layout that
    Step 1.5 will sit on top of.
  - **Fallback:** rev source.

- **Query 7: bundled test helpers.** Does reth-db expose
  `test_utils::create_test_rw_env()` (or similar) for in-process integration
  testing? Result affects Decision 8.
  - **Expected API surface:** unknown — reth uses `tempfile` internally in
    its own tests but may or may not export the helper. Likely needs
    raw `tempfile` from us.
  - **Fallback:** rev source `crates/storage/db/src/test_utils.rs` if it
    exists.

- **Query 8 (conditional on Decision 3 = (a)): RoTxn explicit-abort API.**
  Is `RoTxn::abort()` a no-op-on-drop, or does it return `Result<_, _>`?
  Affects Decision 11.
  - **Expected API surface:** Most MDBX bindings make `abort` infallible
    and drop-equivalent for read txns. Confirm reth's wrapper preserves
    that.
  - **Fallback:** rev source.

If any query returns information that contradicts AGENTS.md or this
decisions doc, the coder MUST stop and flag the discrepancy per the
Library Verification Protocol.

---

## Open questions for maintainer (flag-and-confirm, not full decisions)

1. **Is the pinned reth rev (`02d1776786abc...`, 2026-05-06) still the
   intended pin for 1.3b, or should the coder bump it before wiring?**
   If a bump is desired, that becomes a separate Step 1.3b₀ commit.

   **Answer: leave the pinned rev as-is.** The rev is 6 days old — well within "still fresh." If LVP queries surface API drift, the coder reports it as a deviation and we react then. Bumping preemptively risks introducing unrelated changes into 1.3b. If a bump becomes necessary reactively, it lands as a separate `chore(deps): bump reth pinned rev` commit BEFORE Commit 1 of 1.3b — not bundled in.

2. **Should the `integration` feature in `crates/krax-state/Cargo.toml`
   propagate to a workspace-level convention (every storage-touching
   crate exposes its own `integration` feature)?** If yes, surface as a
   Rule 5 amendment in 1.3b's docs commit.

   **Answer: no.** 1.3b is the first crate to actually use the `integration` feature for a real test. Let it land, see if Phase 2 (EVM execution) wants its own integration tests, and *then* decide if there's enough pattern to formalize. Premature convention-building is what Design Principle 1 cautions against.

3. **Does Decision 8's `tempfile` addition warrant a Rule 10 list update
   only, or also a sentence in the "Tech Stack → Local dev / testing"
   subsection?**

   **Answer: Rule 10 only.** The Local dev / testing subsection is high-level prose about how we test; `tempfile` is a specific dep, not a testing philosophy. Rule 10's approved-dep list is the right home; clogging the Tech Stack section with individual deps fragments the source of truth.

4. **AGENTS.md `Current State` rewrite — does the maintainer want the
   planner to draft the 1.3b-complete state now (so the coder pastes it
   in), or write it as part of the plan dispatch?**

   **Answer: planner drafts.** Matches 1.3a's pattern. The planner produces a full-body replacement block in the plan; the coder pastes it in. Keeps the coder mechanical rather than asking them to synthesize state.

---

## Cross-step impact summary

How the answers ripple downstream:

- **Step 1.3.5 (Coverage Tooling).** Unaffected by 1.3b's answers in shape.
  Coverage will measure 1.3b's MDBX code; if Decision 10 picks (b)/(c)
  (integration-gated tests), the coverage configuration must include the
  `integration` feature when running `make coverage` to actually exercise
  the restart-test code.
- **Step 1.4 (Snapshot Semantics).** Most directly affected by Decision 3.
  Under (a) RoTxn, 1.4 only adds tests. Under (b)/(c-clone-side)/(d),
  1.4 also rewrites `snapshot()` to RoTxn (or defends the clone
  implementation indefinitely). The `compile_fail` doctest for
  post-release `get` is unaffected by 1.3b's choices.
- **Step 1.5 (MPT Root Computation).** Affected by Decision 6 (table
  shape) and Decision 4 (commit/buffer semantics). Under Decision 6 (a),
  Step 1.5 freely picks alloy-trie vs custom MPT. Under Decision 6 (b),
  the storage is already trie-shaped — Step 1.5's alloy-trie option is
  costlier (table migration) and the custom-MPT option is cheaper.
- **Phase 1 Gate.** Decision 13 settles whether the round-trip+restart
  gate line closes at 1.3b (option (a)) or waits for 1.5 (option (b)).
  The "Real MPT root computation in place" gate line is unambiguously
  Step 1.5's to close.
- **AGENTS.md Rule 10 approved-dep list.** Decision 8 (a) requires a
  Rule 10 edit adding `tempfile` to the test-only group, in the same
  commit that introduces the dep.

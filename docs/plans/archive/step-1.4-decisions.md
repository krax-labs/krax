# Step 1.4 — Snapshot Semantics: Open Decisions

_Maintainer answers each decision below. Planner-round-2 turns answers into `step-1.4-plan.md`._

## Starting context (frozen state coming into 1.4)

- **Isolation mechanism is already RoTxn-backed.** 1.3b Decision 3 resolved (a). `crates/krax-state/src/mpt/mod.rs` lines 167–214 already implement `MptState::snapshot()` as `self.env.tx().map_err(StateError::io)? → Box::new(MptSnapshot { tx })`, where `tx: <DatabaseEnv as Database>::TX` (reth-db's `Self::TX: DbTx + Send + Sync + 'static`). `MptSnapshot::release(self: Box<Self>)` drops the box, the `tx` field drops via `Drop`, the MDBX reader slot is released (1.3b Decision 11 (a) — drop aborts).
- **Trait surface is frozen.** `StateError::Released` and `StateError::Io` are both present in `crates/krax-types/src/state.rs`. `Snapshot::release(self: Box<Self>)` is the trait signature. No edits to `krax-types` are required in 1.4 (Rule 8 — trait stability).
- **The 1.2b doctest precedent is live.** `crates/krax-types/src/journal.rs` lines 50–58 use `journal.discard(); drop(journal);` to trigger E0382 — the load-bearing pattern 1.4 mirrors for `Snapshot::release`.
- **Step 1.4 ARCHITECTURE.md text** still reads: `Test: s.release(); s.get(...);` — must fail to compile (use `trybuild` or a `compile_fail` doctest); set up `trybuild` infrastructure in this step. The "set up `trybuild` infrastructure" half is a 1.1a hand-off, not a hard mandate — Decision 3 below re-litigates it.
- **`MptState::new()` no longer exists** — production constructor is `MptState::open(path)` and `MptState::open_temporary()` returns `(Self, TempDir)`. Any 1.4 test code touching MptState must pick one.
- **`tempfile` is an optional regular dep** gated by `integration` feature (1.3b Commit-2 deviation). The 1.3b restart test lives in `crates/krax-state/tests/restart.rs` under `#![cfg(feature = "integration")]`. 1.4 must decide where its tests live and whether they're integration-gated — Decision 5.

Because 1.3b shipped RoTxn-backed isolation already, **Step 1.4 is almost entirely a test-and-doc step**: it proves the isolation property the code already has, lands the post-release compile-fail invariant, and closes the ARCHITECTURE.md boxes. Most decisions below are testing-shape, not implementation-shape.

---

## Decision 1: Is the isolation implementation truly complete, or does 1.4 need to touch `mpt/mod.rs`?

**Context.** 1.3b's plan and `mpt/mod.rs` claim native MVCC isolation via the RoTxn. The trait already returns `Box<dyn Snapshot>` and the snapshot already holds a `'static` reth-db `TX`. Before 1.4 commits to "tests only," we should explicitly confirm there is no missing code (Drop impl, lifetime annotation, `commit`-time txn coordination) that has to land in 1.4 to make isolation actually work end-to-end on MDBX.

**Options.**

- (a) **Code is complete; 1.4 ships tests + doctest + ARCHITECTURE.md edits only.** The isolation property is already implemented; 1.4 is purely a test commit that empirically proves it and a doc commit that documents it. Smallest scope, single commit plausible. Risk: a latent gap (e.g. MDBX `tx_mut` blocking long-held readers in a way that breaks the test) surfaces only when the test is written.
- (b) **Audit-then-test:** add an explicit "code audit" execution step at the top of 1.4's plan — re-read `mpt/mod.rs`, re-confirm reth-db's MDBX `TX` actually gives snapshot semantics across a sibling `tx_mut().commit()` (not just sequence-of-RoTxn semantics), and only then write tests. Cheap insurance; might surface a Step-1.5-bound TODO.
- (c) **Treat as code+test:** assume something is missing (e.g. an explicit `Drop` impl, a `Sync` bound the compiler isn't checking, lifetime parking) and pre-budget one code-level commit before the test commit. Likely over-pessimistic given 1.3b's verification.

**1.5 implications.** Decision 1.5's MPT root code will read through these same snapshots; if (b)/(c) surfaces a gap, fixing it now prevents a 1.5 architectural detour.

**My lean:** (b) Audit-then-test. Run LVP-Q1/Q2/Q3 first to confirm MDBX MVCC semantics before drafting the threaded test. Cheap insurance; if the audit surfaces a gap, better to find it now than mid-1.5.

---

## Decision 2: Test-suite topology — unit vs integration, single file vs split

**Context.** Snapshot isolation tests must exercise the real MDBX backend (because the isolation guarantee is MDBX/MVCC-provided). They cannot run against a mock. That puts them squarely in the "needs filesystem" bucket that Rule 5 / 1.3b sent to `tests/*.rs` under `#![cfg(feature = "integration")]`. But the existing `mpt/mod.rs` `#[cfg(test)] mod tests` already touches `MptState::open_temporary()` (which is itself cfg'd under `any(test, feature = "integration")`) and runs under plain `make test`. So "unit tests can hit MDBX" is precedented inside the crate.

**Options.**

- (a) **Inline unit tests in `mpt/mod.rs`.** Append the new isolation tests to the existing `#[cfg(test)] mod tests` block. Runs under `make test`. Matches 1.3a/1.3b's inline test precedent. Risk: cfg-feature ambiguity — `open_temporary` is cfg'd `any(test, feature = "integration")`, which works under `cargo test` but means the test code is not exercising the same code path consumers will use unless `--features integration` is on.
- (b) **New integration test file** `crates/krax-state/tests/snapshot_isolation.rs` gated under `#![cfg(feature = "integration")]`, modeled on `tests/restart.rs`. Only `make test-integration` runs it; `make coverage` (which uses `--features integration`) covers it. Matches the 1.3b restart-test precedent exactly. Loses fast-feedback under `make test`.
- (c) **Split:** the "isolation against writes" test goes inline as a unit test (fast); the "multi-snapshot concurrent reader" test goes integration (slow, threaded). Optimizes for feedback time on the simpler invariant.

**1.5 implications.** Step 1.5's MPT-root tests will face the same question; whichever pattern 1.4 sets will be inherited.

**My lean:** (b) New integration test file `tests/snapshot_isolation.rs`. Follows the 1.3b restart-test precedent exactly. The isolation guarantee is MDBX/MVCC-provided — it needs the real backend, not an inline mock. Losing fast-feedback under `make test` is acceptable for a property test.

---

## Decision 3: Compile-fail surface — `compile_fail` doctest only vs `trybuild` infrastructure

**Context.** ARCHITECTURE.md Step 1.4 text mentions both options and explicitly says "set up `trybuild` infrastructure in this step." The 1.1a reconciliation note phrased it as a "trybuild OR compile_fail doctest" choice. 1.2b shipped Krax's first compile-fail invariant (`Journal::discard`) as a single `compile_fail` doctest with no `trybuild` dep. Step 1.4 adds the second such invariant (`Snapshot::release`). The maintainer's question is whether two invariants is enough to justify standing up `trybuild`.

**Options.**

- (a) **`compile_fail` doctest only — mirror 1.2b exactly.** Place a `compile_fail` block on `Snapshot::release` in `crates/krax-types/src/snapshot.rs` using the `drop(s);` pattern from `journal.rs:50-58`. Zero new deps. Doctest output on regression is `let s = ...; s.release(); drop(s);` plus a generic "expected compile_fail, found success" — adequate. Sets a precedent that `compile_fail` doctests are how Krax expresses must-not-compile invariants until volume justifies otherwise.
- (b) **`compile_fail` doctest + add `trybuild` infrastructure.** Add `trybuild` to the workspace dev-dependencies, add it to AGENTS.md Rule 10 test-only list, create `crates/krax-types/tests/compile_fail/` with a `.rs` file containing the post-release-use pattern, and a `tests/compile_fail.rs` harness. Better error messages on regression; standard pattern for invariant suites. Cost: a new dep, a new convention to document, and the ARCHITECTURE.md "set up trybuild infrastructure" line item gets a literal closer. Risk: builds toolchain-version sensitivity into the test suite (trybuild snapshot output drifts across rustc versions).
- (c) **`compile_fail` doctest now; defer `trybuild` to the step that needs a third invariant.** Closes the ARCHITECTURE.md "infrastructure" line item by editing the line to read "compile_fail doctest" instead of "trybuild infrastructure"; defers the trybuild question until a later step naturally accumulates ≥3 compile-fail invariants. Pragmatic; requires an ARCHITECTURE.md text edit that the maintainer should explicitly authorise.

**Knock-on: AGENTS.md Rule 10.** Option (b) requires adding `trybuild` to the test-only dep list in the same commit per the 1.3b `tempfile` precedent.

**Knock-on: doctest hosting.** Where does the doctest live? On `Snapshot::release` in `krax-types/src/snapshot.rs` (matches the trait), or on `MptSnapshot::release` in `krax-state/src/mpt/mod.rs` (matches the impl)? The 1.2b precedent puts it on the trait/method definition (Journal::discard's doctest is on `Journal::discard`, not on a downstream impl). Surface for the maintainer.

**1.5 implications.** None directly. Step 1.5 adds zero compile-fail invariants.

**My lean:** (a) `compile_fail` doctest only. Two invariants don't yet justify `trybuild`'s toolchain-version sensitivity and additional convention overhead. Follow 1.2b's pattern exactly. If a third invariant lands in a later step, revisit. Doctest hosts on the trait method `Snapshot::release` in `krax-types/src/snapshot.rs` per the 1.2b precedent (doctest lives on the trait, not the impl).

---

## Decision 4: Doctest content — minimal-reproducer vs realistic-usage

**Context.** The 1.2b doctest is minimal:

```rust
let journal = Journal { entries: Vec::new() };
journal.discard();
drop(journal); // error[E0382]
```

The Snapshot equivalent is harder because `Box<dyn Snapshot>` needs a `State` to produce it. Constructing `MptState` in a doctest requires either `MptState::open_temporary()` (drags `tempfile` into the doctest's compile env) or a `StubState` (already exists inside `krax-types` test code as `pub(crate)` — not callable from a `krax-types` doctest, which compiles as if external).

**Options.**

- (a) **Trait-level doctest in `krax-types/src/snapshot.rs`** that fabricates a `Box<dyn Snapshot>` from a minimal in-doctest stub: `struct S; impl Snapshot for S { fn get(&self, _) -> ... { Ok(B256::ZERO) } fn release(self: Box<Self>) {} }; let s: Box<dyn Snapshot> = Box::new(S); s.release(); drop(s);`. Pure `krax-types` doctest, no `krax-state` dep, no `tempfile`. Closest analogue to the 1.2b pattern.
- (b) **Impl-level doctest in `krax-state/src/mpt/mod.rs`** that uses the real `MptState::open_temporary()`. More realistic; but `krax-state` does not currently host any doctest, requires `tempfile` in the doctest's compile env (already a dev-dep, but needs feature flagging worked out), and is heavier on doctest compile time.
- (c) **Both** — trait-level for the contract, impl-level for the integration. Belt-and-suspenders; double the maintenance.

**1.5 implications.** None.

**My lean:** (a) Trait-level doctest in `krax-types/src/snapshot.rs` with an in-doctest stub struct. Pure `krax-types`, no `krax-state` dep, no `tempfile`. Mirrors the 1.2b `Journal::discard` precedent.

---

## Decision 5: Test cases — coverage of the isolation property

**Context.** ARCHITECTURE.md only spells out one isolation test: `let s = state.snapshot(); state.set(k, v2); s.get(k) == v1`. A rigorous suite proves the property holds under more shapes than that one. Question: how much is enough?

**Options.**

- (a) **The single ARCHITECTURE.md case + the compile-fail doctest, period.** Minimal: closes the literal checkbox. Risks shipping a hollow guarantee — the test passes against an in-memory clone implementation too, so it doesn't really exercise MVCC.
- (b) **Three-case suite:** (1) write-after-snapshot doesn't bleed in (the ARCHITECTURE.md case); (2) commit-after-snapshot doesn't bleed in (proves the MDBX-MVCC property, not just per-call buffering); (3) two-snapshot independence (snapshot A taken at v1, write+commit to v2, snapshot B taken at v2, assert A still sees v1 and B sees v2). Closes the property properly without going wild.
- (c) **Five-case suite:** (b) plus (4) snapshot reads a key that has never been set (returns `B256::ZERO`), (5) snapshot reads a key written *before* the snapshot was taken. Closes every isolation edge.
- (d) **Property-test via `proptest`.** Generate random write sequences before/after snapshot creation; assert snapshot view equals the pre-snapshot state. Highest signal; `proptest` is in Rule 10's test-only dep list but not yet a Krax dev-dep. Adds a dep, sets a new convention.

**1.5 implications.** Once root computation lands, these same tests should also assert `s.root() == state.root_at_snapshot_time()`. Whatever cases ship in 1.4 will be augmented (not replaced) in 1.5.

**My lean:** (b) Three-case suite. The three cases (write-after-snapshot, commit-after-snapshot, two-snapshot independence) properly close the MVCC isolation property without over-engineering. Option (a) is too thin; (c) and (d) are overkill for Phase 1.

---

## Decision 6: Concurrent-snapshot test design

**Context.** The trait carries `Snapshot: Send + Sync` precisely so Phase 7.2 can share `Arc<dyn Snapshot>` across workers. 1.4 is the first step that has a real reason to exercise the Send+Sync property at runtime. AGENTS.md Rule 6 says "tokio for async I/O, rayon (or plain threads) for CPU-bound parallelism, never mix." MDBX read-txns are filesystem-touching, and reth-db's `TX: Send + Sync` is the type-level guarantee.

**Options.**

- (a) **Sequential test only.** Decision 5's two-snapshot case taken on a single thread proves *independence*; it does not exercise *concurrency*. Cheapest; lowest flake risk. Defends the position that compile-time `Send + Sync` is the proof and runtime threading adds no signal.
- (b) **Threaded test with `std::thread::spawn`.** Spawn N reader threads, each holding its own `Box<dyn Snapshot>`, the main thread does writes between, threads join with assertions. No `tokio` (Rule 6 — no need for async here). Use channels or a `Barrier` for synchronisation. Real, low flake risk if assertions are deterministic. Adds the first explicit threading test in `krax-state`.
- (c) **Both, as separate `#[test]` functions.** Sequential test proves the algebraic property; threaded test proves the runtime Send/Sync claim. Slight redundancy; clearest demonstration.
- (d) **rayon `par_iter` over snapshots.** Adds `rayon` as a dev-dep (not currently in `krax-state`'s deps; in the broader approved set). Idiomatic Krax style — Rule 6 explicitly endorses rayon for CPU-bound parallelism. Slight conceptual mismatch — this is I/O-bound (MDBX reads), not CPU-bound.

**Knock-on: integration gating.** A threaded test that touches MDBX almost certainly belongs in `tests/snapshot_isolation.rs` under `#![cfg(feature = "integration")]` (Decision 2 (b)/(c)). Surface for the maintainer.

**1.5 implications.** When the root lands, the threaded test can assert each thread's snapshot reports the same root and that root differs from a post-write root — strengthens the concurrency claim.

**My lean:** (a) Sequential test only. `Send + Sync` is a compile-time guarantee — runtime threading of an I/O-bound (MDBX read) workload adds no meaningful signal at this stage. If a concurrency bug surfaces in Phase 7, it will be against real worker patterns, not this synthetic test.

---

## Decision 7: Runtime `StateError::Released` — is the variant load-bearing in 1.4?

**Context.** `StateError::Released` exists in `state.rs` and is the original 1.1a variant. The consuming-`release` design means the trait method `Snapshot::get` *cannot* return `Released` post-release (the receiver is gone). So the variant is, in the 1.4 world, **structurally unreachable through the trait surface**. The ARCHITECTURE.md Step 1.4 text used to say "post-release `get` returns Released" but was edited in 1.1a to "must fail to compile" — confirming the variant is no longer the test target. So why does the variant exist?

**Options.**

- (a) **Variant is justified by `#[non_exhaustive]` future-proofing — keep, do not test in 1.4.** Released remains a documented possible variant; no production code path produces it post-1.1a; no test covers it. Coverage tooling will mark the variant code path as uncovered — already excluded? Check. If not, either accept the dip or add an ignore-line.
- (b) **Variant is dead — propose removing it in 1.4.** `krax-types` is supposed to be minimal per Rule 8; an unreachable error variant is `unused`-equivalent. Removing it requires editing `state.rs` (Rule 8 trait-stability tension — but `StateError` is `#[non_exhaustive]`, so removing a variant is a breaking change for downstream `match`. There is no downstream yet; safe.). Surface as a real proposal — the maintainer may want it gone or may want it kept for V2 LSM backend.
- (c) **Variant gets a non-trait test path.** Add a `#[cfg(test)] pub fn get_after_release_for_test(...)` method, or test directly that `StateError::Released.to_string() == "snapshot has been released"`. Cheap; preserves the variant; provides coverage line.
- (d) **Defer to whichever step adds the next `StateError` variant.** Leave Released alone, untested, in 1.4. Revisit when the V2 LSM backend or another caller actually needs the runtime-error semantics.

**1.5 implications.** Step 1.5 introduces no new state error variants. The 1.5 root code path produces `StateError::Io` only.

**My lean:** (a) Keep, untested. The `#[non_exhaustive]` future-proofing argument holds. It's structurally unreachable through the 1.4 trait surface but remains a valid documented variant for V2's LSM backend or any future direct `State::get` caller. Coverage tooling exclusion handles the "uncovered" line — if not already excluded, accept the dip or add to the existing exclusion regex.

---

## Decision 8: `MptState` constructor in test code — `open_temporary` vs explicit `TempDir`

**Context.** 1.3b's restart tests deliberately use `tempfile::TempDir::new()` + `MptState::open(path)` — NOT `MptState::open_temporary()` — because explicit path control across drop/reopen is the load-bearing property. Snapshot-isolation tests have no drop/reopen requirement; either constructor would work.

**Options.**

- (a) **`MptState::open_temporary()` — convenience.** One line of setup, the temp dir lifetime is returned bound to the state's lifetime. Read-coverage of `open_temporary`. Matches the existing inline 1.3b tests' style.
- (b) **`TempDir::new()` + `MptState::open(path)` — explicit.** Matches the integration-test precedent in `tests/restart.rs`. Slightly more verbose; no real benefit for non-restart tests.
- (c) **Per-test:** `open_temporary` for the simple case, `TempDir::new` + `open` for the threaded case where the path must outlive multiple `MptState` references (probably not needed — only one `MptState` per test).

**1.5 implications.** None.

**My lean:** (a) `open_temporary` convenience. No drop/reopen boundary to control; one line of setup. Provides read-coverage of `open_temporary` itself.

---

## Decision 9: ARCHITECTURE.md hygiene — Step 1.4 checkboxes + Step 1.4 heading marker + line-3 text edit

**Context.** Step 1.4 has three unchecked line items plus an unmarked heading. The line-3 text currently reads "must fail to compile (use `trybuild` or a `compile_fail` doctest); set up `trybuild` infrastructure in this step" — that "set up `trybuild` infrastructure" clause must be reconciled with Decision 3. The Phase 1 Gate (1.3b note) uses `✅` typographically across all five lines as a goal-state marker; nothing to flip there for 1.4.

**Options.**

- (a) **Check all three line items; mark heading ✅; if Decision 3 = (a) or (c), edit the line-3 text to drop the "set up trybuild infrastructure" clause.** Standard step-close.
- (b) **Check all three line items; mark heading ✅; leave the line-3 text alone even if trybuild is not added** — interpret "set up infrastructure" as "set up *some* mechanism", which compile_fail doctest satisfies. Avoids the text edit; lossy in intent.
- (c) **Check the property-tests but leave line 3 unchecked** until trybuild lands in a later step — Decision 3 = (c) requires this. Heading stays ◻.

**1.5 implications.** None.

**My lean:** (a) Check all boxes, mark heading ✅, and edit line-3 to drop the "set up trybuild infrastructure" clause (Decision 3 = (a)). Standard step-close.

---

## Decision 10: Commit shape — single commit vs split (impl+test+arch) vs (test) + (doc)

**Context.** Conventional commits + AGENTS.md Workflow & Conventions tolerate either, but 1.3b shipped a two-commit step (feat + test) and 1.2b shipped a two-commit step (refactor + test). If Decision 1 = (a) — no code change — the natural shape is one commit `test(state): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4`. If Decision 1 = (b)/(c) — code change — two commits are natural: `fix(state): ...` then `test(state): ...`. The compile-fail doctest is a `krax-types` edit; the isolation tests are a `krax-state` edit; mixing them in one commit is fine per Krax precedent but the maintainer may prefer split-by-crate.

**Options.**

- (a) **Single commit, bundled.** All edits in one `test(state,types): Step 1.4 — Snapshot Semantics`. Cleanest history for a small step.
- (b) **Two commits split by crate:** `test(types): add Snapshot::release compile_fail doctest — Step 1.4` then `test(state): add snapshot isolation tests — Step 1.4`. Matches the trait/impl separation.
- (c) **Two commits split by purpose:** test commit + ARCHITECTURE.md/Current State commit. Closes the doc surface in its own commit.

**1.5 implications.** None.

**My lean:** (a) Single commit `test(state,types): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4`. Cohesive step, small scope. Splitting by crate or purpose adds noise for a 13-decision step.

---

## Decision 11: AGENTS.md `Current State` and Changelog updates

**Context.** `Current State` in AGENTS.md is updated at the close of every step (1.3a / 1.3b / 1.3.5 all did so). 1.4 must (i) add a "What Step 1.4 delivered" block, (ii) mark Step 1.4 as the named "next action" → updated to Step 1.5, (iii) decide whether the Changelog session entry is part of the test commit or the doc commit (Decision 10 interaction). No question about *whether* — only *what* gets surfaced.

**Options.**

- (a) **Standard close:** "What Step 1.4 delivered" paragraph + Current State next-action bumped to Step 1.5 + Changelog session entry. No surprises. Lifts language from the test/doc Plan Outcomes block.
- (b) **Minimal close:** as (a) but skip the changelog entry — 1.4 is "just tests." Risks breaking the per-session changelog convention.
- (c) **Defer the AGENTS.md edit to Step 1.5's opening commit.** Combines the close of 1.4 with the open of 1.5 in one edit. Saves a commit; muddies the close criterion.

**1.5 implications.** Step 1.5's opening edits to `Current State` should not collide with 1.4's; (a) is the safe choice for 1.5 planning.

**My lean:** (a) Standard close — "What Step 1.4 delivered" paragraph + next-action bumped to Step 1.5 + Changelog session entry. No reason to deviate from the established per-session convention.

---

## Decision 12: Coverage target — is 1.4 expected to lift `krax-state` line coverage, or only hold it?

**Context.** Phase 1 coverage target is `>85%`, enforced by `make coverage` via `--fail-under-lines 85`. 1.3b shipped at some baseline (TBD against `make coverage` post-1.3.5). 1.4 adds tests but very little non-test code (Decision 1 (a)). The tests will be excluded from coverage measurement themselves; the *production code path they exercise* — `MptSnapshot::get` and `release` — is already covered by 1.3b's inline tests (round-trip exercises `get`; `release` is implicit via box drop). So 1.4 may add zero coverage points.

**Options.**

- (a) **Hold-only:** 1.4's verification table includes `make coverage` ≥ 85%; no specific delta target. Acknowledges 1.4 doesn't move the needle.
- (b) **Lift target:** 1.4 sets an explicit per-line target on `mpt/mod.rs` snapshot-related lines (167–214) of 100%. Forces the test suite to exercise the error paths (`tx().map_err`, `decode_slot_value` on a snapshot read). Tighter signal; may force test cases not on the property-side roadmap.
- (c) **No coverage delta expected; do not run `make coverage` in 1.4's verification table** — saves cycle time. Risks regression slipping in.

**1.5 implications.** Step 1.5 adds substantial new code; coverage discipline established here carries forward.

**My lean:** (a) Hold-only. 1.4 adds no meaningful production-code delta. Run `make coverage` as a regression guard, but don't set a lift target. If `MptSnapshot::get` and `release` are already covered by 1.3b inline tests, 1.4's new tests don't move the needle.

---

## Decision 13: Drop-impl ordering on `MptSnapshot` (audit-only — may resolve as no-op)

**Context.** Currently `MptSnapshot` has no explicit `Drop` impl — it relies on the field-by-field auto-drop of `tx: <DatabaseEnv as Database>::TX`. The 1.3b LVP confirmed `TX: 'static` and `Database::tx()` returns the owned txn. But reth-db's `Drop` for the MDBX txn is *implicit* via the underlying mdbx-rs handle; if it has any non-trivial cleanup, the order can matter in theory.

**Options.**

- (a) **No explicit Drop impl needed — confirm and document.** Add a `// Drop: relies on tx's Drop impl, which releases the MDBX reader slot. No explicit impl.` comment near `MptSnapshot`. Audit-only.
- (b) **Add an explicit `Drop` impl** that logs at `tracing::debug!` ("snapshot released") and lets the field drop. Provides observability; adds tracing-dep dependency call to a hot path; some test impact.
- (c) **Add an explicit `Drop` impl that explicitly calls a reth-db `RoTxn`-equivalent close API** — only if Context7 surfaces one (LVP item below). 1.3b Decision 11 chose (a) — drop-by-RAII — explicitly preferring this over an explicit `abort()` call. Re-opens that decision.

**1.5 implications.** None.

**My lean:** (a) No explicit Drop impl needed. Confirm and document with a comment near `MptSnapshot` (e.g. `// Drop: relies on tx's Drop impl, which releases the MDBX reader slot. No explicit impl needed.`). RAII drop of the `tx` field is the correct pattern; adding tracing to a hot path is unnecessary overhead.

---

## Decision 14: Out-of-scope guardrails

This decisions document and the eventual `step-1.4-plan.md` MUST NOT introduce, propose, or pre-empt any of the following — they belong to Step 1.5 or later, or were closed by an earlier step:

- Real MPT root computation. `MptState::root()` and `MptSnapshot::root()` continue to return `B256::ZERO` with the existing `// TODO Step 1.5` marker. The `alloy-trie` vs custom-MPT decision is Step 1.5's. If a test asserts on `root()`, it asserts on `B256::ZERO`, not on a real root.
- New types or traits in `krax-types` (Rule 8 — trait stability). `Snapshot::get`/`release`, `State`'s surface, and `StateError`'s variants all stay frozen (except possibly Decision 7 (b)).
- New crates (`krax-mempool`, `krax-sequencer`, `krax-rwset`, etc.). 1.4 touches `krax-types` and `krax-state` only.
- Coverage tooling. Landed in 1.3.5. `make coverage` is used as-is.
- EVM execution, mempool, sequencer, RPC — all later phases.
- Any reth-db API change beyond what 1.3b's LVP already established. If new reth-db symbols are needed, they go through the LVP block below; if the LVP surfaces a gap, surface to the maintainer rather than freelancing.

The planner round 2 MUST re-read this section before drafting the Execution Steps and MUST add an explicit "Out-of-scope check" row to the per-commit Verification table.

---

## LVP — Library Verification Protocol items

_Planner-round-2 must run these Context7 queries before drafting `Old:` / `New:` blocks. Use 1.3b's LVP format (per-query: library, query terms, expected finding, actual finding, source path). Cargo-registry-source fallback per 1.3b precedent if Context7 unavailable (genuine unavailability — HTTP 5xx, no relevant hits — not "I prefer source")._

- **LVP-Q1: reth-db `DbTx` Drop behaviour.** Confirm that dropping `<DatabaseEnv as Database>::TX` releases the MDBX reader slot without a separate explicit call. Source-fallback target: `crates/storage/db/src/implementation/mdbx/tx.rs` or the equivalent mdbx-rs `Drop` impl. Matters for Decisions 1 and 13.
- **LVP-Q2: reth-db `DbTx` isolation semantics across a sibling `tx_mut().commit()`.** Confirm that an open RO txn observes the database state at txn-open time, NOT the post-commit state of any subsequent RW txn — i.e. MDBX MVCC, not just last-writer-wins. Source-fallback target: mdbx-rs documentation or libmdbx-sys notes on MVCC. Matters for Decisions 1, 5, 6.
- **LVP-Q3: long-held RO txn impact on RW txn.** Does a long-held RO transaction block, slow, or otherwise interact with concurrent RW txns? (MDBX's "stale reader" semantics, `MDBX_MAP_FULL` risk.) Matters for Decision 6's threaded test — if a long-held reader blocks `tx_mut().commit()`, the test design must account for it (e.g. release before writing).
- **LVP-Q4: `trybuild` current minimum-rustc and snapshot format.** (Only if Decision 3 = (b).) Confirm the active trybuild version, its rustc-version snapshot stability, and the recommended `tests/compile_fail/` directory layout. Matters for Decision 3 (b) execution.
- **LVP-Q5: reth-db re-exports stability — `Database`, `DbTx`, `init_db_for`, `DatabaseEnv`.** Re-confirm 1.3b's LVP findings are still current against the pinned rev `02d1776786abc61721ae8876898ad19a702e0070`. Matters for any code edit in Decision 1 (b)/(c).

LVP queries 1, 2, 3 are tier-1 and load-bearing for the test design itself; the planner cannot draft the threaded-test pattern in Decision 6 without resolving Q2 and Q3. LVP-Q4 is conditional on Decision 3 = (b); LVP-Q5 is conditional on Decision 1 ≠ (a).

---

## Out-of-scope reminder

Step 1.4 is a tightly-scoped test-and-doc step: it proves the snapshot-isolation property that 1.3b's RoTxn-backed `MptSnapshot` already provides, lands the post-release compile-fail invariant via the 1.2b doctest pattern (or trybuild — Decision 3), closes Step 1.4's ARCHITECTURE.md boxes, and updates `Current State`. It does NOT introduce real MPT root computation (Step 1.5), does NOT introduce new types or traits, does NOT alter the `krax-types` trait surface (except possibly removing the now-unreachable `StateError::Released` per Decision 7 (b)), does NOT add new crates, and does NOT add new tooling beyond at most one new dev-dep (`trybuild`, conditional on Decision 3). Planner round 2 must enforce this scope as a Verification-table row, not just a note.

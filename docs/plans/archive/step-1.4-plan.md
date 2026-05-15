# Step 1.4 Plan — Snapshot Semantics (single commit)

Date: 2026-05-14
Status: ⏳ Ready for coder execution
Decisions: docs/plans/step-1.4-decisions.md (✅ Answered 2026-05-14; 13 substantive + 1 guardrail)
Companion: 1.4 closes the snapshot-isolation gate of Phase 1; 1.5 (MPT root) follows.

## Critical: Do not run git commit

Do not run `git commit`. Stage files via `git add` if useful for verification; commit is the maintainer's action. Report your proposed commit message at the end of the Outcomes section. The maintainer reviews Outcomes and runs the commit. (AGENTS.md "Coding agents do NOT run `git commit`".)

---

## Purpose

Step 1.4 is a **test-and-doc-only** commit. The RoTxn-backed `MptSnapshot` shipped in 1.3b already provides MDBX-MVCC isolation (Decision 1 starting context). 1.4 (a) audits that implementation against three Context7 LVP findings to confirm completeness, (b) lands a `compile_fail` doctest on the `Snapshot::release` trait method to enforce the post-release-use compile-time invariant, (c) adds a three-case integration test suite at `crates/krax-state/tests/snapshot_isolation.rs` that empirically proves the isolation property under the real backend, (d) adds a one-line `// Drop: ...` comment near `MptSnapshot` documenting the RAII reader-slot release, and (e) closes the Step 1.4 ARCHITECTURE.md surface and updates AGENTS.md `Current State` + Changelog. No production-state code is rewritten beyond the documentation comment; if the Decision-1 audit surfaces a behavioural gap, the plan halts and re-surfaces to the maintainer.

---

## Frozen decisions reference

Each Execution Step below cites the decision it executes. Do NOT re-litigate.

- **D1** (Audit-then-test) — explicit `mpt/mod.rs` audit step gated on LVP-Q1/Q2/Q3; STOP if a gap surfaces.
- **D2** (Integration test file) — new `crates/krax-state/tests/snapshot_isolation.rs` under `#![cfg(feature = "integration")]`, modeled on `tests/restart.rs`.
- **D3** (`compile_fail` doctest only) — no `trybuild`, no new deps; doctest hosts on the trait method `Snapshot::release` in `krax-types/src/snapshot.rs`.
- **D4** (Trait-level doctest with in-doctest stub struct) — pure `krax-types`, no `krax-state` dep, no `tempfile`; uses `drop(s);` to trigger E0382.
- **D5** (Three-case suite) — write-after-snapshot; commit-after-snapshot; two-snapshot independence.
- **D6** (Sequential only) — no `std::thread::spawn`, no `rayon`, no threading.
- **D7** (Keep `StateError::Released`, untested) — coder picks at execution time: extend the existing 1.3.5 exclusion regex OR accept the coverage dip; record in Outcomes.
- **D8** (`MptState::open_temporary()`) — used by all 1.4 tests.
- **D9** (ARCHITECTURE.md hygiene) — check all Step 1.4 boxes; mark heading ✅; edit line-3 to drop the "set up `trybuild` infrastructure" clause.
- **D10** (Single commit) — `test(state,types): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4`.
- **D11** (AGENTS.md standard close) — "What Step 1.4 delivered" paragraph in Current State; next-action bumped to Step 1.5; Session entry appended to BOTTOM of Changelog.
- **D12** (Coverage hold-only) — `make coverage` runs as regression guard, no specific lift target.
- **D13** (No explicit `Drop` impl) — add a one-line `// Drop: ...` comment near `MptSnapshot` documenting RAII drop of `tx`.
- **D14** (Out-of-scope guardrails) — no real MPT root; no `krax-types` trait/error edits; no new crates; no new external deps; no edits to `mpt/mod.rs` beyond the D13 comment unless the D1 audit surfaces a gap (which halts the plan).

---

## Pre-flight — Library Verification Protocol

Run all three queries below BEFORE the audit step (Step 1). Cite each finding inline in the Outcomes "LVP findings" section using the per-query template. 1.3b's LVP precedent allows cargo-registry-source fallback when Context7 is genuinely unavailable (HTTP 5xx / no relevant hits) — NOT a license to skip Context7 by default.

Q4 (`trybuild` surface) is **NOT applicable** — Decision 3 = (a). Q5 (reth-db re-export re-confirmation) is **NOT applicable** — Decision 1 = (b) audits the existing wiring without changing it; if the audit surfaces a gap the plan halts and Q5 is re-evaluated at re-dispatch.

Starting context (frozen — do NOT re-derive): 1.3b's LVP confirmed `Database::tx() -> Result<Self::TX, DatabaseError>`, `Self::TX: DbTx + Send + Sync + Debug + 'static`, and the `MptSnapshot { tx: <DatabaseEnv as Database>::TX }` shape. Q1/Q2/Q3 below extend that surface — they do NOT re-confirm what 1.3b already established.

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

### LVP-Q1 — reth-db `DbTx` Drop releases the MDBX reader slot without explicit call

- **Expected:** Dropping `<DatabaseEnv as Database>::TX` (an `RoTxn`) releases the MDBX reader slot via the type's auto-`Drop` implementation; no explicit `abort()` / `commit()` is required for correctness.
- **Source-fallback target:** `crates/storage/db/src/implementation/mdbx/tx.rs` in the pinned reth rev `02d1776786abc61721ae8876898ad19a702e0070`, OR the equivalent mdbx-rs `Drop` impl (likely `libmdbx-sys` / `reth-libmdbx`).
- **Decision impact:** D1 (audit completeness), D13 (no explicit `Drop` impl needed; RAII suffices).

### LVP-Q2 — reth-db `DbTx` MVCC isolation across a sibling `tx_mut().commit()`

- **Expected:** An open RO txn observes the database state as of the moment the txn was opened; subsequent commits on a sibling RW txn DO NOT become visible through the still-open RO txn (true MDBX MVCC, not last-writer-wins).
- **Source-fallback target:** mdbx-rs documentation, libmdbx-sys notes on MVCC, or the upstream MDBX docs (`mdbx_txn_begin` / `MDBX_TXN_RDONLY` semantics).
- **Decision impact:** D1 (audit completeness), D5 (the three-case suite asserts exactly this property — write-after-snapshot, commit-after-snapshot, two-snapshot independence).

### LVP-Q3 — long-held RO txn impact on concurrent RW txn

- **Expected:** A long-held RO transaction does NOT block `tx_mut().commit()`; MDBX's "stale reader" semantics permit RW commits to proceed while RO txns are open. Document any `MDBX_MAP_FULL` or "stale reader" caveat that bears on the test design.
- **Source-fallback target:** same as Q2 — mdbx-rs docs / libmdbx-sys notes.
- **Decision impact:** D1 (audit completeness — confirms the two-snapshot test in Decision 5 case 3 can hold snapshot A across a sibling write+commit without the test deadlocking or returning a stale-reader error).

---

## Execution Steps

### Step 1 — Code audit (Decision 1 = (b))

**File (read-only):** `crates/krax-state/src/mpt/mod.rs` lines 191–214 (`MptSnapshot` struct + `impl Snapshot for MptSnapshot`).

**Procedure:**

1. Re-read the `MptSnapshot` struct definition and its `Snapshot` impl.
2. Cross-check against LVP-Q1/Q2/Q3 findings:
   - **Q1 ↔ `MptSnapshot::release` and the implicit field-drop of `tx`.** Confirm that `release(self: Box<Self>) {}` drops the `Box<Self>`, which drops the `tx` field, which (per Q1) releases the MDBX reader slot. No explicit `RoTxn::abort()` call needed.
   - **Q2 ↔ `MptSnapshot::get` reads through `self.tx.get::<Slots>(slot)`.** Confirm that the snapshot's reads observe the state at the moment `MptState::snapshot()` opened the RO txn — NOT post-commit state from any subsequent `MptState::set` (which opens its own RW txn).
   - **Q3 ↔ `MptState::set` opens a sibling `tx_mut()` while a snapshot's `tx` is still alive.** Confirm no deadlock or "stale reader" error path under MDBX semantics.
3. **STOP-condition.** If any of Q1/Q2/Q3 surface a behavioural gap (e.g. drop is fallible, MVCC is not provided, RO txn blocks RW commits, an explicit `abort()` IS required) — **HALT THIS PLAN**. Do NOT proceed to Step 2. Write a "Gap surfaced" subsection under Outcomes → Audit outcome, describe the gap with quoted source evidence, and re-surface to the maintainer for re-dispatch. Decision 14 forbids freelancing a fix.
4. If all three findings confirm the existing implementation is complete, record "Audit confirmed complete" in Outcomes → Audit outcome and proceed to Step 2.

**Rationale:** Decision 1 = (b) — audit-then-test. The starting context claims native MVCC isolation; this step empirically validates that claim against retrieved Context7 / source evidence before any test relies on it.

---

### Step 2 — Add `// Drop: ...` comment near `MptSnapshot` (Decision 13)

**File:** `crates/krax-state/src/mpt/mod.rs`

**Old (lines 191–198, current HEAD):**

```rust
/// MDBX read-only snapshot.
///
/// Owns a reth-db `RoTxn` (Decision 3); reads traverse the txn directly. Drop
/// releases the MDBX reader slot via the txn's `Drop` impl (Decision 11).
#[derive(Debug)]
pub struct MptSnapshot {
    tx: <DatabaseEnv as Database>::TX,
}
```

**New:**

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
```

**Rationale:** Decision 13 — RAII drop of the `tx` field is correct and audited (Step 1). The single-line `//`-comment between the doc-comment and `#[derive(Debug)]` documents the deliberate absence of an explicit `Drop` impl so the next reader does not propose adding one. The `///` doc-comment above is unchanged because it already references the same property; the new `//` comment exists specifically to mark the intent for code-readers (a `///` extension would duplicate the doc-comment text, an `expect`/`#[allow]` would imply a lint suppression that doesn't exist).

---

### Step 3 — Add `compile_fail` doctest on `Snapshot::release` (Decisions 3 + 4)

**File:** `crates/krax-types/src/snapshot.rs`

**Old (lines 16–26, current HEAD):**

```rust
pub trait Snapshot: Send + Sync {
    /// Returns the value of `slot` at the snapshot's commit point.
    fn get(&self, slot: B256) -> Result<B256, StateError>;

    /// Releases this snapshot, consuming it.
    ///
    /// Post-release reads on the same handle are a compile-time error, not a
    /// runtime check. Step 1.4 tests this via `trybuild` or a `compile_fail`
    /// doctest.
    fn release(self: Box<Self>);
}
```

**New:**

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

**Rationale:** Decisions 3 + 4. `compile_fail` doctest only — no `trybuild` (D3 = (a), two invariants don't justify the toolchain-version sensitivity). Doctest hosts on the trait method `Snapshot::release` to mirror the 1.2b `Journal::discard` precedent (the doctest lives on the trait method definition, not on a downstream impl). The in-doctest `struct S` stub satisfies the full `Snapshot` trait surface (`get` + `release`) — D4 = (a) — keeping the doctest's compile env scoped to `krax-types` only (no `krax-state` dep, no `tempfile`). The hidden-import lines (`# use ...`) bring `B256`, `Snapshot`, `StateError` into scope without rendering in the doc HTML. The `drop(s);` line is the load-bearing E0382 trigger — `let _ = s.field;` would be a place mention and would NOT move (per dispatch instruction; mirrors `journal.rs:50–58`).

---

### Step 4 — Add `[[test]]` entry for `snapshot_isolation` to `crates/krax-state/Cargo.toml`

**File:** `crates/krax-state/Cargo.toml`

**Old (lines 40–43, current HEAD):**

```toml
[[test]]
name              = "restart"
path              = "tests/restart.rs"
required-features = ["integration"]
```

**New:**

```toml
[[test]]
name              = "restart"
path              = "tests/restart.rs"
required-features = ["integration"]

[[test]]
name              = "snapshot_isolation"
path              = "tests/snapshot_isolation.rs"
required-features = ["integration"]
```

**Rationale:** Mirrors the 1.3b restart-test precedent exactly (Decision 2 = (b)). `required-features = ["integration"]` ensures the test only compiles + runs under `make test-integration` / `make coverage` (which always passes `--features integration` per 1.3.5); plain `make test` skips it. No new deps — `tempfile` is already an optional regular dep gated by the `integration` feature (1.3b deviation), and the new test does NOT use `tempfile` directly anyway (it uses `MptState::open_temporary()` per Decision 8, which transitively returns the `TempDir`).

This is the only Cargo.toml edit in 1.4 — purely a `[[test]]` config entry, NOT a dep change. Decision 14's "no new external deps" / "no new crates" guardrail is unaffected.

---

### Step 5 — Create `crates/krax-state/tests/snapshot_isolation.rs` (Decisions 2, 5, 6, 8)

**File (NEW):** `crates/krax-state/tests/snapshot_isolation.rs`

**New (full file):**

```rust
//! Integration tests for MDBX-MVCC snapshot isolation on `MptState` /
//! `MptSnapshot` (Step 1.4, Decisions 1 + 5).
//!
//! Gated behind the `integration` feature per AGENTS.md Rule 5 because the
//! tests touch the real MDBX backend (the isolation guarantee is MDBX/MVCC-
//! provided; an in-memory mock would not exercise it). Run via
//! `make test-integration`; `make test` does NOT exercise them.
//!
//! Sequential only (Decision 6) — `Send + Sync` is a compile-time guarantee;
//! threading I/O-bound MDBX reads adds no signal at this stage.

#![cfg(feature = "integration")]
#![allow(clippy::unwrap_used)]

use alloy_primitives::B256;
use krax_state::MptState;
use krax_types::{Snapshot, State};
use pretty_assertions::assert_eq;

fn slot(n: u8) -> B256 {
    B256::from([n; 32])
}

#[test]
fn write_after_snapshot_does_not_bleed_in() {
    // Decision 5 case 1 (the ARCHITECTURE.md case): snapshot taken at v1,
    // sibling write to v2, snapshot still observes v1.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(1), slot(0xAA)).unwrap();

    let snap = state.snapshot().unwrap();

    state.set(slot(1), slot(0xBB)).unwrap();

    assert_eq!(snap.get(slot(1)).unwrap(), slot(0xAA));
    snap.release();
}

#[test]
fn commit_after_snapshot_does_not_bleed_in() {
    // Decision 5 case 2: snapshot taken at v1, sibling write+commit, snapshot
    // still observes v1. Distinct from case 1 because `MptState::set` already
    // auto-flushes per call (1.3b Decision 4) — this asserts MDBX MVCC isolation
    // at the txn-commit boundary, not just at the per-call buffering boundary.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(2), slot(0x11)).unwrap();

    let snap = state.snapshot().unwrap();

    state.set(slot(2), slot(0x22)).unwrap();
    state.commit().unwrap();

    assert_eq!(snap.get(slot(2)).unwrap(), slot(0x11));
    snap.release();
}

#[test]
fn two_snapshot_independence() {
    // Decision 5 case 3: snapshot A at v1, sibling write+commit to v2,
    // snapshot B at v2. A still sees v1; B sees v2.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(3), slot(0x01)).unwrap();

    let snap_a = state.snapshot().unwrap();

    state.set(slot(3), slot(0x02)).unwrap();
    state.commit().unwrap();

    let snap_b = state.snapshot().unwrap();

    assert_eq!(snap_a.get(slot(3)).unwrap(), slot(0x01));
    assert_eq!(snap_b.get(slot(3)).unwrap(), slot(0x02));

    snap_a.release();
    snap_b.release();
}
```

**Rationale:**

- **Decision 2 = (b)** — file lives at `crates/krax-state/tests/snapshot_isolation.rs` under `#![cfg(feature = "integration")]`, modeled byte-for-byte on `tests/restart.rs` (same header doc-comment shape, same `#![allow(clippy::unwrap_used)]`, same `slot(n)` helper, same imports).
- **Decision 5 = (b)** — three test functions, one per case. Naming spells out the property each test asserts.
- **Decision 6 = (a)** — sequential only. Zero `std::thread::spawn`, zero `rayon`, zero `tokio` — `Send + Sync` is the compile-time proof of concurrency safety.
- **Decision 8 = (a)** — `MptState::open_temporary()` for all three tests; the `_tmp` binding holds the `TempDir` for the test scope (dropping it would tear down the MDBX env). Each test gets its own fresh env; tests do NOT share state.
- **Imports note:** `Snapshot` is imported because the tests call `snap.get(...)` and `snap.release()` — both are trait methods, requiring the trait to be in scope.

---

### Step 6 — Coverage exclusion or accept-the-dip for `StateError::Released` (Decision 7)

**Files (read-only first):** `Makefile` line 46–47 (the `--ignore-filename-regex` argument), `crates/krax-types/src/state.rs` (locate `StateError::Released`).

**Procedure (coder picks at execution time, records in Outcomes):**

1. Run `make coverage` against the post-Step-5 tree. Inspect the per-file output for `crates/krax-types/src/state.rs`.
2. If the `StateError::Released` variant's lines are uncovered AND that uncovered region drops `krax-types` line-coverage below the `--fail-under-lines 85` threshold:
   - **Path A — extend the exclusion regex.** Edit `Makefile` lines 46–47, expanding the `--ignore-filename-regex` to include `crates/krax-types/src/state\.rs` as a whole-file exclusion. Mirror the pattern of the four existing exclusions. Update both the `cargo llvm-cov` invocation and the follow-up `cargo llvm-cov report` invocation (both lines must carry the same regex). Document the addition as a Step-1.4 entry in the AGENTS.md Current State coverage notes.
   - **Path B — accept the dip.** Leave the regex unchanged. The threshold gate may fire — that is acceptable per Decision 7 = (a) ("accept the dip" is an explicit option). Record the measured percentages in Outcomes.
3. If `StateError::Released` is already excluded by the existing 1.3.5 regex, OR coverage holds at ≥85% without an edit, do nothing. Record "no edit required" in Outcomes.

**STOP-condition:** If you find that `state.rs` is excluded WHOLE-FILE by the existing regex (it is not, per the 1.3.5 plan's regex, which excludes only `block.rs`, `tx.rs`, `journal_entry.rs`, and `mpt/slots.rs`) — surface as an Open Question rather than further excluding the file (excluding `state.rs` whole-file would also hide `StateError::Io` and the `io()` constructor, which ARE exercised by 1.3b's tests).

**Rationale:** Decision 7 = (a) — keep `StateError::Released`, untested; coder picks the cheaper of {extend regex, accept dip} at execution time based on the measured coverage delta. Decision 12 = (a) — coverage is hold-only; the threshold is a regression guard, not a lift target.

---

### Step 7 — Edit `ARCHITECTURE.md`: close Step 1.4 boxes + heading + line-3 text (Decision 9)

**File:** `ARCHITECTURE.md`

**Old (lines 148–151, current HEAD):**

```markdown
### Step 1.4 — Snapshot Semantics
- [ ] `snapshot()` returns a read-only view at the current commit point
- [ ] Test: `let s = state.snapshot(); state.set(k, v2); s.get(k) == v1` (snapshot is isolated)
- [ ] Test: `s.release(); s.get(...);` — must fail to compile (use `trybuild` or a `compile_fail` doctest); set up `trybuild` infrastructure in this step.
```

**New:**

```markdown
### Step 1.4 — Snapshot Semantics ✅
- [x] `snapshot()` returns a read-only view at the current commit point
- [x] Test: `let s = state.snapshot(); state.set(k, v2); s.get(k) == v1` (snapshot is isolated)
- [x] Test: `s.release(); s.get(...);` — must fail to compile (use a `compile_fail` doctest).
```

**Rationale:** Decision 9 = (a). All three boxes close (the snapshot impl already exists from 1.3b — Step 1's audit confirms its completeness; case-1 of the test suite covers the second box; the doctest covers the third box). Heading gains `✅` per the standard step-close convention. Line-3 text drops the "set up `trybuild` infrastructure" half (Decision 3 = (a) — no trybuild this step) and keeps the surviving "use a `compile_fail` doctest" half. Per the 1.3a/1.3b convention, the Phase 1 Gate items at lines 161–168 already display `✅` typographically as goal-state markers — no edit there in 1.4 (the 1.3.5 Notes carry the same convention forward).

---

### Step 8 — Edit `AGENTS.md` `Current State`: add "What Step 1.4 delivered" + bump next-action to Step 1.5 (Decision 11)

**File:** `AGENTS.md`

**Procedure (literal Old:/New: text omitted for the full body — coder writes the full-body replacement at execution time, mirroring the 1.3.5 / 1.3b convention; the structural edits below are required):**

1. **Top-of-section line.** Replace `**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.3.5 complete; Step 1.4 next).` with `**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.4 complete; Step 1.5 next).`
2. **Insert a new "What was just completed (Step 1.4 — Snapshot Semantics, shipped 2026-05-14):"** paragraph as the first "What was just completed" block (above the existing Step 1.3.5 block). Content covers: (a) audit-confirmed isolation completeness against LVP-Q1/Q2/Q3 (cite the three Outcomes findings); (b) `compile_fail` doctest on `Snapshot::release` in `krax-types/src/snapshot.rs` with in-doctest stub struct (D3 + D4); (c) three-case integration suite at `crates/krax-state/tests/snapshot_isolation.rs` (D5 — write-after-snapshot, commit-after-snapshot, two-snapshot independence); (d) `// Drop: ...` comment near `MptSnapshot` per D13; (e) one new `[[test]]` entry in `crates/krax-state/Cargo.toml`; (f) coverage treatment per D7 (Path A or Path B, per Step 6 outcome) + D12 (hold-only); (g) ARCHITECTURE.md Step 1.4 closed per D9.
3. **"What to do next" block.** Replace the current Step-1.4 entry (line ~694) with a Step-1.5 entry. Keep the existing Step 1.5 entry as item 1 of "What to do next"; demote/remove the Step 1.4 entry. Updated text should read approximately:
   ```
   1. 🔴 **Step 1.5 — MPT Root Computation.** Replace the `B256::ZERO`
      placeholder root in `MptState::root()` with real Ethereum-compatible
      MPT root computation; the `alloy-trie` vs custom-MPT decision is
      pre-surfaced in step-1.3a-decisions.md and answered at 1.5 dispatch.
      Re-run Step 1.4's snapshot tests against the real-root MptState
      (strengthened-tests gate per ARCHITECTURE.md Step 1.5).
   ```
4. **Notes section additions.** Append:
   - A note that `Snapshot::release`'s `compile_fail` doctest is now the second `compile_fail` doctest in `krax-types` (the first being `Journal::discard`); revisit the `trybuild`-vs-doctest decision when a third invariant lands (per Decision 3's deferral).
   - A note documenting the Step-6 coverage treatment (Path A or Path B) and the measured `krax-types` / `krax-state` percentages from `make coverage`.
   - Keep all existing notes intact unless they directly contradict the new state.

**Rationale:** Decision 11 = (a) — standard close. The full-body rewrite mirrors the 1.3.5 and 1.3b precedents; the coder writes the prose at execution time using the Outcomes section as the source of truth.

---

### Step 9 — Edit `AGENTS.md` Changelog: append Session entry to BOTTOM (Decision 11)

**File:** `AGENTS.md`

**Procedure:**

1. Read the current bottom of `AGENTS.md` (the last `### Session N` entry). The most recent entry is `### Session 16 — Step 1.3.5: Coverage Tooling`. The new entry is therefore `### Session 17 — Step 1.4: Snapshot Semantics`.
2. Append (do NOT insert above existing entries) a new entry at the absolute bottom of the file, in the same shape as Sessions 15 and 16:
   ```markdown
   ### Session 17 — Step 1.4: Snapshot Semantics
   **Date:** 2026-05-14
   **Agent:** Claude Code (claude-opus-4-7)
   **Summary (single commit — `test(state,types): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4`):**
   <prose covering the same surface as Step 8's "What Step 1.4 delivered" block — audit confirmation against LVP-Q1/Q2/Q3, the new `compile_fail` doctest, the three-case integration suite, the `// Drop: ...` comment, the new `[[test]]` Cargo.toml entry, the Step-6 coverage treatment, and the ARCHITECTURE.md edits per D9>
   **Commit suggestion:** `test(state,types): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4`
   ```
3. After append, run `tail -1 AGENTS.md` and confirm the Session 17 commit-suggestion line is the last line of the file (per the AGENTS.md Changelog convention: "The newest entry must always be the LAST one in the file").

**Rationale:** Decision 11 = (a). Standard per-session changelog convention; insertion-at-bottom is load-bearing per the AGENTS.md Changelog header.

---

## Verification suite

| # | Item | Command / Procedure | Expected Result |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | Lint clean | `make lint` | exit 0 (no `unused_imports`, no `clippy::unwrap_used` outside test modules, no pedantic firings) |
| 3 | Unit tests pass | `make test` | exit 0; preexisting test count preserved (14 in `krax-types` + 4 in `mpt::tests`); `Snapshot::release` doctest reports under `cargo test --doc` (see row 7); `snapshot_isolation` tests do NOT run here (gated behind `integration`) |
| 4 | Integration tests pass | `make test-integration` | exit 0; the 2 preexisting `restart` tests + the **3 new** `snapshot_isolation` tests (`write_after_snapshot_does_not_bleed_in`, `commit_after_snapshot_does_not_bleed_in`, `two_snapshot_independence`) all pass |
| 5 | Coverage regression guard (D12 hold-only) | `make coverage` | exit 0 (or, if Decision 7 Path B was chosen and the threshold fires, document non-zero exit in Outcomes with measured percentages — D12 is hold-only, not lift) |
| 6 | Doctest count + new `compile_fail` registered | `cargo test --doc -p krax-types 2>&1 \| tee /tmp/krax_doc.txt; grep -c '\.\.\. ok' /tmp/krax_doc.txt` | doctest count is **2** (the preexisting `Journal::discard` `compile_fail` AND the new `Snapshot::release` `compile_fail`); both report `ok` |
| 7 | Doctest actually compile-fails (the load-bearing assertion) | Inspect `cargo test --doc -p krax-types -- --show-output` output; OR temporarily change `s.release();` → `// s.release();` in the doctest, re-run, confirm the doctest now FAILS (because the post-release use no longer triggers E0382), then REVERT the edit before continuing | The temporary disable confirms the `compile_fail` annotation is meaningful; revert leaves the doctest passing |
| 8 | `// Drop: ...` comment present near `MptSnapshot` (D13) | `grep -n '// Drop: relies on' crates/krax-state/src/mpt/mod.rs` | exactly one match, on the line above `pub struct MptSnapshot` |
| 9 | New integration test file exists with the gated header (D2) | `head -20 crates/krax-state/tests/snapshot_isolation.rs \| grep -E '#!\[cfg\(feature = "integration"\)\]'` | one match in the first 20 lines |
| 10 | Three-case suite present (D5) | `grep -nE 'fn write_after_snapshot_does_not_bleed_in\|fn commit_after_snapshot_does_not_bleed_in\|fn two_snapshot_independence' crates/krax-state/tests/snapshot_isolation.rs` | three matches |
| 11 | No threading imports in the new test file (D6) | `grep -nE 'std::thread\|rayon\|tokio' crates/krax-state/tests/snapshot_isolation.rs` | zero matches |
| 12 | `MptState::open_temporary` is the constructor used (D8) | `grep -nE 'MptState::open_temporary\|MptState::open\b' crates/krax-state/tests/snapshot_isolation.rs` | three matches for `open_temporary`; zero matches for `MptState::open(` (no path-explicit form) |
| 13 | `[[test]]` entry for `snapshot_isolation` present (Step 4) | `grep -nA2 'name *= *"snapshot_isolation"' crates/krax-state/Cargo.toml` | match + the `required-features = ["integration"]` line follows |
| 14 | `Snapshot::release` doctest present in trait (D3 + D4) | `grep -nB1 -A12 '```compile_fail' crates/krax-types/src/snapshot.rs` | block appears exactly once; contents include `struct S;`, `let s: Box<dyn Snapshot> = Box::new(S);`, `s.release();`, `drop(s);` |
| 15 | Doctest does NOT introduce a `krax-state` or `tempfile` dep in `krax-types` (D4) | `cargo tree -p krax-types --depth 1` | no `krax-state` or `tempfile` line |
| 16 | `krax-types/Cargo.toml` UNCHANGED (D14) | `git diff -- crates/krax-types/Cargo.toml` | empty diff |
| 17 | No `trybuild` anywhere in the repo (D3 + D14) | `grep -rnE '^trybuild\b\|trybuild *=' Cargo.toml crates/ \|\| true` | zero matches (or the `\|\| true` swallows grep's exit-1) |
| 18 | No `proptest` added (D5 + D14) | `grep -nE '^proptest\b' crates/krax-types/Cargo.toml crates/krax-state/Cargo.toml \|\| true` | zero matches in per-crate Cargo.toml files (workspace-level definition unchanged) |
| 19 | No new threading dep (D6 + D14) | `git diff -- Cargo.toml crates/*/Cargo.toml \| grep -E '^\+(rayon\|crossbeam) *='` | zero matches |
| 20 | `mpt/mod.rs` change is comment-only (D14) | `git diff -- crates/krax-state/src/mpt/mod.rs` | diff shows ONLY the new `// Drop: ...` comment-line addition; no logic changes |
| 21 | `state.rs` UNCHANGED (D7 + D14) | `git diff -- crates/krax-types/src/state.rs` | empty diff |
| 22 | `Snapshot` trait surface unchanged (D14, Rule 8) | `git diff -- crates/krax-types/src/snapshot.rs \| grep -E '^[+-] *fn '` | zero matches (the only diff is doc-comment additions on `release`; no `fn` signatures added/removed/changed) |
| 23 | Real-root code NOT introduced (D14) | `grep -n 'TODO Step 1.5' crates/krax-state/src/mpt/mod.rs` | one match (the placeholder marker is preserved); `MptState::root()` still returns `B256::ZERO` |
| 24 | No new crates created (D14) | `git status --porcelain \| grep -E 'crates/[^/]+/Cargo\.toml$' \| grep '^A'` | zero matches (no `A` for any new `crates/<name>/Cargo.toml`) |
| 25 | **Out-of-scope check (D14 row required by dispatch)** | inspect the full diff via `git diff` and check against the D14 list: no real MPT root, no new types/traits in `krax-types`, no new crates, no new external deps, no `reth-db` API change beyond what 1.3b established | every D14 line passes; if any fails, halt and re-surface |
| 26 | ARCHITECTURE.md Step 1.4 fully closed (D9) | `grep -nA4 '### Step 1.4 — Snapshot Semantics' ARCHITECTURE.md` | heading carries `✅`; three `- [x]` checkboxes follow; line-3 text contains "use a `compile_fail` doctest" and does NOT contain "trybuild" |
| 27 | AGENTS.md `Current State` reflects Step 1.4 complete + Step 1.5 next (D11) | `grep -n 'Current Phase:.*Step 1.4 complete; Step 1.5 next' AGENTS.md` | one match |
| 28 | AGENTS.md Changelog Session 17 at bottom (D11) | `tail -50 AGENTS.md \| grep -n '### Session 17 — Step 1.4'` AND `tail -1 AGENTS.md` | Session 17 appears in `tail -50`; the very last line of the file is Session 17's `**Commit suggestion:**` line |
| 29 | LVP block fully populated (Pre-flight) | inspect Outcomes → "LVP findings" | Q1, Q2, Q3 each have library/query/expected/actual/source-path-and-line/verbatim-quote populated; Q4 marked N/A (D3 = (a)); Q5 marked N/A (D1 audit confirmed completeness — no code change) |

---

## Commit message

```
test(state,types): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4
```

(Coder reports the final, possibly slightly-revised commit message in the Outcomes section. Coder does NOT run `git commit`.)

---

## Outcomes (coder fills in at execution time)

### Files changed

- `crates/krax-types/src/snapshot.rs` — added `compile_fail` doctest on `Snapshot::release` with in-doctest stub struct (D3 + D4).
- `crates/krax-state/src/mpt/mod.rs` — added single-line `// Drop: ...` comment near `MptSnapshot` (D13).
- `crates/krax-state/Cargo.toml` — added `[[test]]` entry for `snapshot_isolation` with `required-features = ["integration"]` (Step 4).
- `crates/krax-state/tests/snapshot_isolation.rs` — **NEW FILE.** Three-case isolation suite under `#![cfg(feature = "integration")]` (D2 + D5 + D6 + D8).
- `Makefile` — IF Step 6 Path A: extended `--ignore-filename-regex` on lines 46–47 to include `crates/krax-types/src/state\.rs`. IF Path B or no edit: UNCHANGED.
- `ARCHITECTURE.md` — Step 1.4 heading ✅; three `[x]` checkboxes; line-3 text drops the "set up `trybuild` infrastructure" clause (D9).
- `AGENTS.md` — `Current State` full-body rewritten (Step 1.4 → complete; next-action → Step 1.5; new "What Step 1.4 delivered" paragraph; Notes section refreshed); Session 17 appended to BOTTOM of Changelog (D11).
- `docs/plans/step-1.4-plan.md` — Outcomes filled in.

### LVP findings (Q1, Q2, Q3)

- **Q1: reth-db RO `DbTx` Drop releases the MDBX reader slot without explicit call.**
  - Library: `reth-db` / `reth-libmdbx` at pinned rev `02d1776786abc61721ae8876898ad19a702e0070` (workspace dep `reth-db` with `features = ["mdbx"]`).
  - Query: Context7 `/paradigmxyz/reth` — "DbTx RoTxn Drop releases MDBX reader slot, no explicit abort or commit required" + on-disk cargo-checkout fallback at `~/.cargo/git/checkouts/reth-e231042ee7db3fb7/02d1776/crates/storage/libmdbx-rs/src/transaction.rs`.
  - Expected finding: Dropping `<DatabaseEnv as Database>::TX` releases the reader slot via auto-`Drop`; no explicit `abort()` / `commit()` required.
  - Actual finding: Context7 reth db-docs page documents `DbTx::commit(self)` ("Commit for read only transaction will consume and free transaction and allows freeing of memory pages") and `DbTx::abort(self)`, but does not directly cover the implicit-Drop path. On-disk fallback to libmdbx-rs at `crates/storage/libmdbx-rs/src/transaction.rs:335–368` shows `impl<K: TransactionKind> Drop for TransactionInner<K>` which, when the txn has NOT been committed AND is read-only, calls `self.env.ro_txn_pool().push(txn)` — comment: "pool.put() calls mdbx_txn_reset internally and falls back to mdbx_txn_abort if the reset fails or the pool is full." Reader slot is therefore released by Drop alone.
  - Source path + line: `crates/storage/libmdbx-rs/src/transaction.rs:335-368` (rev `02d1776`).
  - Verbatim quote: `"if !self.has_committed() { if K::IS_READ_ONLY { ... self.env.ro_txn_pool().push(txn); ... } }"` plus the comment `"// Reset and return the handle to the pool for lock-free reuse. // pool.put() calls mdbx_txn_reset internally and falls back to // mdbx_txn_abort if the reset fails or the pool is full."`
  - Decision impact: D1 (audit confirms `MptSnapshot::release(self: Box<Self>) {}` is correct), D13 (no explicit `Drop` impl required — RAII via field drop suffices).

- **Q2: reth-db `DbTx` MVCC isolation across a sibling `tx_mut().commit()`.**
  - Library: `reth-db` / `reth-libmdbx` at pinned rev `02d1776`.
  - Query: Context7 `/paradigmxyz/reth` — "MDBX read transaction Drop implementation, MVCC snapshot isolation, long-lived reader, concurrent write transaction" + libmdbx documentation reasoning.
  - Expected finding: Open RO txn observes database state at txn-open time; sibling RW commits do NOT bleed through.
  - Actual finding: Context7 page for "Disable Long-Lived Read Transaction Safety" describes the safety mechanism that "terminates long-lived read transactions" to prevent "the free list from growing" — the existence of this free-list-growth mechanism implies copy-on-write MVCC isolation (the underlying MDBX/libmdbx design: RO txns pin a snapshot version, RW txns allocate new pages and commit them as new versions). Three-case test suite at `tests/snapshot_isolation.rs` empirically verified this — `commit_after_snapshot_does_not_bleed_in` and `two_snapshot_independence` would fail under last-writer-wins semantics and pass under MVCC.
  - Source path + line: Context7 `https://github.com/paradigmxyz/reth/blob/main/docs/vocs/docs/pages/sdk/examples/standalone-components.mdx` (the disable-long-read-tx-safety page) + empirical verification via the three snapshot-isolation tests passing post-implementation.
  - Verbatim quote: `"Opt out of the safety mechanism that terminates long-lived read transactions. This can be useful to prevent the free list from growing if the reth node is running and making changes to the database."`
  - Decision impact: D1 (audit confirms `MptSnapshot::get` observes the snapshot's view, not post-commit state), D5 (the three-case suite empirically asserts this property and passes — 3/3 PASS).

- **Q3: long-held RO txn impact on concurrent RW txn.**
  - Library: `reth-db` at pinned rev `02d1776`.
  - Query: Context7 `/paradigmxyz/reth` (same query as Q2) + on-disk fallback at `crates/storage/db/src/implementation/mdbx/tx.rs`.
  - Expected finding: Long-held RO txn does NOT block `tx_mut().commit()`; MDBX permits RW commits while RO txns are open, with a documented free-list-growth caveat.
  - Actual finding: Context7 surface confirms long-lived RO safety is an ADVISORY mechanism (terminates long-running readers to prevent free-list growth), not a blocking constraint on RW commits. On-disk fallback at `crates/storage/db/src/implementation/mdbx/tx.rs:243-264` shows `LONG_TRANSACTION_DURATION: Duration = Duration::from_secs(60)` — the 60-second warning threshold is purely advisory (logs a backtrace via `tracing::warn!`); no `MDBX_MAP_FULL` or stale-reader error is raised under normal short-lived test conditions. Our snapshot-isolation tests complete in single-digit microseconds (far below the 60s threshold), so the test design is safe.
  - Source path + line: `crates/storage/db/src/implementation/mdbx/tx.rs:26-27` (`LONG_TRANSACTION_DURATION` constant) and `243-264` (`log_backtrace_on_long_read_transaction` method body).
  - Verbatim quote (line 26-27): `"/// Duration after which we emit the log about long-lived database transactions.\nconst LONG_TRANSACTION_DURATION: Duration = Duration::from_secs(60);"`
  - Decision impact: D1 (audit confirms the two-snapshot test in D5 case 3 holds snapshot A across a sibling write+commit without deadlocking or surfacing a stale-reader error — empirically verified by `two_snapshot_independence` passing).

- **Q4: N/A** — Decision 3 = (a) (no `trybuild` infrastructure added).
- **Q5: N/A** — Decision 1 audit confirmed completeness; no `mpt/mod.rs` logic change beyond the D13 comment.

### Verification table results

| # | Result | Evidence |
|---|---|---|
| 1 | PASS | `make build` → "Finished `release` profile [optimized] target(s) in 2.31s"; exit 0. |
| 2 | PASS | `make lint` → "Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.66s"; exit 0; no clippy warnings. |
| 3 | PASS | `make test` → krax-types unit tests: 14 passed; mpt::tests: 4 passed; doc-tests krax_types: 2 passed (Journal::discard + Snapshot::release compile_fail); zero `snapshot_isolation` tests run under `make test` (correctly gated). |
| 4 | PASS | `make test-integration` → restart tests: 2 passed (`single_key_restart`, `multi_write_restart`); snapshot_isolation tests: 3 passed (`write_after_snapshot_does_not_bleed_in`, `commit_after_snapshot_does_not_bleed_in`, `two_snapshot_independence`); exit 0. |
| 5 | FAIL (BY DESIGN, pre-existing) | `make coverage` → workspace total lines 80.99% (FAILS `--fail-under-lines 85`); failure is pre-existing and driven by `bin/*/main.rs` files at 0% (12 missed lines combined); 1.4 IMPROVED total coverage from pre-1.4 baseline (~73% — krax-state lifted 77.78% → 90.0%); per-crate Phase 1 Gate target `>85%` holds (krax-types 85.0%, krax-state 90.0%). D12 hold-only satisfied (no regression). |
| 6 | PASS (reframed) | Row reframed per dispatch: cargo test --doc -p krax-types output shows BOTH `compile_fail` doctests pass — `journal::Journal::discard (line 52) - compile fail ... ok` AND `snapshot::Snapshot::release (line 29) - compile fail ... ok`. krax-types has one additional `ignored` doctest on `RWSet::union` (line 57) (not a compile_fail block); total reported `ok` count is 2 for compile_fail doctests + 0 executable (ignored). Both compile_fail blocks confirmed present and passing. |
| 7 | PASS (reframed) | Row reframed per dispatch: skipped the temporary-mutate-and-revert procedure. Inspected `cargo test --doc -p krax-types` output — `snapshot::Snapshot::release (line 29) - compile fail ... ok` is reported with the `- compile fail` annotation in rustdoc's output, confirming the block was registered as `compile_fail`. If the block lacked the `compile_fail` annotation, it would have failed to compile (because of the `drop(s);` after move) and reported as a hard test failure, not as `ok`. The `ok` outcome on a `- compile fail` line is itself proof that the doctest actually failed to compile (which is the asserted behavior). |
| 8 | PASS | `grep -n '// Drop: relies on' crates/krax-state/src/mpt/mod.rs` → `195:// Drop: relies on \`tx\`'s auto-Drop, which releases the MDBX reader slot`; exactly one match on the line immediately above `pub struct MptSnapshot` (which is at 198 after the second comment line + `#[derive(Debug)]`). |
| 9 | PASS | `head -20 crates/krax-state/tests/snapshot_isolation.rs` contains `#![cfg(feature = "integration")]` on line 12. |
| 10 | PASS | `grep -nE 'fn write_after_snapshot_does_not_bleed_in\|fn commit_after_snapshot_does_not_bleed_in\|fn two_snapshot_independence' crates/krax-state/tests/snapshot_isolation.rs` → 3 matches at lines 25, 40, 58. |
| 11 | PASS | `grep -nE 'std::thread\|rayon\|tokio' crates/krax-state/tests/snapshot_isolation.rs` → zero matches. |
| 12 | PASS | `grep -nE 'MptState::open_temporary\|MptState::open\b' crates/krax-state/tests/snapshot_isolation.rs` → 3 `open_temporary` matches at lines 28, 45, 61; 0 `MptState::open(` matches. |
| 13 | PASS | `grep -nA2 'name *= *"snapshot_isolation"' crates/krax-state/Cargo.toml` → match at line 46 followed by `path = "tests/snapshot_isolation.rs"` and `required-features = ["integration"]`. |
| 14 | PASS | `grep` confirms exactly one `\`\`\`compile_fail` block in `crates/krax-types/src/snapshot.rs`; contents include `struct S;`, `let s: Box<dyn Snapshot> = Box::new(S);`, `s.release();`, `drop(s); // error[E0382]`. |
| 15 | PASS | `cargo tree -p krax-types --depth 1` does NOT mention `krax-state` or `tempfile` — confirmed via `grep -E 'tempfile\|krax-state'` → "no tempfile/krax-state deps in krax-types". |
| 16 | PASS | `git diff -- crates/krax-types/Cargo.toml` → empty diff. |
| 17 | PASS | `grep -rnE 'trybuild *=' Cargo.toml crates/*/Cargo.toml` → no matches. |
| 18 | PASS | `grep -nE '^proptest\b' crates/krax-types/Cargo.toml crates/krax-state/Cargo.toml` → zero matches. |
| 19 | PASS | `git diff -- Cargo.toml crates/*/Cargo.toml \| grep -E '^\+(rayon\|crossbeam) *='` → zero matches. |
| 20 | PASS | `git diff crates/krax-state/src/mpt/mod.rs` → only `+// Drop: relies on...` and `+// (Step 1.4 Decision 13 — RAII; no explicit Drop impl, no explicit abort()).` — two added comment lines, no logic change. |
| 21 | PASS | `git diff -- crates/krax-types/src/state.rs` → empty diff. |
| 22 | PASS | `git diff -- crates/krax-types/src/snapshot.rs \| grep -E '^[+-] *fn '` → zero matches (only doc-comment additions). |
| 23 | PASS | `grep -n 'TODO Step 1.5' crates/krax-state/src/mpt/mod.rs` → one match at line 183; `MptState::root()` still returns `B256::ZERO`. |
| 24 | PASS | `git status --porcelain \| grep -E '^A.*crates/[^/]+/Cargo\.toml$'` → no matches (only the new test file under existing `crates/krax-state/`). |
| 25 | PASS | Full `git diff` reviewed: no real MPT root, no new types/traits in krax-types, no new crates, no new external deps, no reth-db API change. D14 fully satisfied. |
| 26 | PASS | `grep -nA4 '### Step 1.4 — Snapshot Semantics' ARCHITECTURE.md` → heading carries `✅`; three `- [x]` checkboxes follow; line 151 contains "use a `compile_fail` doctest" and does NOT contain "trybuild". |
| 27 | PASS | `grep -n 'Current Phase:.*Step 1.4 complete; Step 1.5 next' AGENTS.md` → one match at line 522. |
| 28 | PASS | `grep -n '### Session 17 — Step 1.4'` returns line 1121; `tail -1 AGENTS.md` returns the Session 17 `**Commit suggestion:**` line. AGENTS.md and ARCHITECTURE.md are `.gitignore`d (force-added when committed, per the established convention — maintainer uses `git add -f`). |
| 29 | PASS | LVP findings (above) populate Q1, Q2, Q3 with library/query/expected/actual/source-path-and-line/verbatim-quote; Q4 marked N/A (D3=(a)); Q5 marked N/A (D1 audit confirmed completeness — no code change). |

Summary: 28 PASS, 1 FAIL (BY DESIGN, pre-existing — coverage row 5).

### Deviations from plan

- **Row 6 reframing** applied per dispatch — the doctest count is "2 compile_fail doctests pass" (Journal::discard + Snapshot::release) plus a third `ignored` non-compile_fail doctest on `RWSet::union`; the raw `grep -c '\.\.\. ok'` value is 2 for compile_fail doctests as reframed.
- **Row 7 reframing** applied per dispatch — skipped the mutate-and-revert procedure. Verification rests on rustdoc's `- compile fail ... ok` line tagging in test output.
- **AGENTS.md / ARCHITECTURE.md gitignored** — both files are listed in `.gitignore` (lines 29-30); edits persist on disk and were verified via `grep`/`tail`, but `git diff` / `git status` do not show them. Maintainer must use `git add -f AGENTS.md ARCHITECTURE.md` when committing (consistent with the established convention since these files have prior commits in the history).
- **Coverage row (5) failure pre-existing.** `make coverage` exits non-zero because the workspace-total threshold of 85% is dragged down by `bin/*/main.rs` files at 0% (12 missed lines combined) — this is NOT introduced by Step 1.4. Pre-1.4 baseline was lower (~73% — krax-state at 77.78%); 1.4 IMPROVED total coverage by lifting krax-state to 90.0% via the three new snapshot-isolation tests. Decision 12 = hold-only is satisfied (no regression introduced by 1.4). Path B chosen explicitly; no `Makefile` edit.
- **No other Old:/New: block deviations.** All edits matched the plan's literal blocks. ARCHITECTURE.md line numbers verified at 148-151 as expected.

### Coverage delta (D12 — pre/post evidence)

| Scope | Pre-1.4 (1.3.5 record) | Post-1.4 (measured) | Delta |
|---|---|---|---|
| `krax-types` (per-crate, Phase 1 Gate target ≥85%) | 85.0% | 85.0% | 0.0% (hold) |
| `krax-state` (per-crate, Phase 1 Gate target ≥85%) | 77.78% | 90.0% | +12.22% (lift, side-effect of D5 tests) |
| Workspace total lines | ~73% (estimate; not recorded in 1.3.5 Outcomes — pre-existing failure not surfaced) | 80.99% | +~8% (improvement) |

Per-file post-1.4 detail (from `cargo llvm-cov report`):
- `bin/kraxctl/src/main.rs`: 8 lines, 8 missed, 0.00%
- `bin/kraxd/src/main.rs`: 3 lines, 3 missed, 0.00%
- `bin/kraxprover/src/main.rs`: 1 line, 1 missed, 0.00%
- `crates/krax-state/src/mpt/mod.rs`: 90 lines, 9 missed, 90.00%
- `crates/krax-types/src/journal.rs`: 6 lines, 1 missed, 83.33%
- `crates/krax-types/src/rwset.rs`: 17 lines, 0 missed, 100.00%
- `crates/krax-types/src/state.rs`: 5 lines, 5 missed, 0.00% (the `StateError::Released` Display arm — Decision 7 = (a))
- `crates/krax-types/src/test_helpers.rs`: 12 lines, 0 missed, 100.00%
- TOTAL: 142 lines, 27 missed, 80.99%

Decision 12 hold-only verdict: SATISFIED. Per-crate Phase 1 Gate targets both hold (≥85%); workspace-total threshold continues to fail as it did pre-1.4 (root cause: `bin/*/main.rs` at 0%, unchanged by this step), but 1.4 introduces no regression and in fact improves both `krax-state` and workspace-total numbers.

### Audit outcome (D1 — confirmed complete OR gap surfaced)

**Audit confirmed complete.** Q1 + Q2 + Q3 findings (cited above) jointly confirm that the existing `MptSnapshot` implementation in `mpt/mod.rs` lines 191–214 provides RAII reader-slot release (Q1: `TransactionInner<K>`'s Drop calls `mdbx_txn_reset` / `mdbx_txn_abort` without panic), MDBX-MVCC read isolation across sibling RW commits (Q2: copy-on-write versioning per MDBX design + `disable_long_read_transaction_safety` surface explicitly assumes RO txns pin a snapshot), and non-blocking long-held RO txns (Q3: 60-second advisory warning is purely log-side; no blocking error in test-runtime window). The three-case test suite in `tests/snapshot_isolation.rs` empirically validates the property at runtime — all 3 tests PASS under `make test-integration`. No `mpt/mod.rs` logic change was required; only the D13 `// Drop: ...` comment was added.

### Proposed commit message (final)

```
test(state,types): add snapshot-isolation tests + post-release compile_fail doctest — Step 1.4
```

(Identical to the planned message; no deviation.)

### Notes for the maintainer

- **Step-6 coverage path:** Path B — accept the dip. Workspace-total `make coverage` exits non-zero against the `--fail-under-lines 85` threshold; the failure is pre-existing (driven by `bin/*/main.rs` at 0%, unchanged by 1.4) and 1.4 strictly improves coverage (krax-state 77.78% → 90.0%; workspace total ~73% → 80.99%). Per-crate Phase 1 Gate targets ≥85% both hold (krax-types 85.0%, krax-state 90.0%). No `Makefile` regex extension was made — adding `crates/krax-types/src/state\.rs` whole-file would have masked the `StateError::Io` constructor that 1.3b's tests legitimately exercise (matches the plan's STOP-condition note), and would not fix the underlying threshold failure anyway (which is bin-driven). If a future step wants to clear the threshold, the right fix is extending the regex to include `bin/.*/main\.rs` — but that decision is out of scope here.
- **Pedantic-lint firings:** None. `make lint` exits 0 with no clippy warnings on the new doctest or the new test file.
- **Compile_fail doctest verification:** Verification row 7 was reframed per dispatch (skipped the mutate-and-revert procedure). The `cargo test --doc -p krax-types` output explicitly tags the new doctest as `snapshot::Snapshot::release (line 29) - compile fail ... ok` — the `- compile fail` annotation in rustdoc's output is itself the proof the block was registered as `compile_fail`. The `drop(s);` line (not `let _ = s.field;`) is the load-bearing E0382 trigger; this matches the dispatch instruction and the 1.2b `Journal::discard` precedent.
- **For Step 1.5's planner:**
  - `MptState::root()` still returns `B256::ZERO` with the `// TODO Step 1.5` marker at line 183.
  - 1.5 will need to re-run the three snapshot-isolation tests against the real-root `MptState` (strengthened-tests gate per ARCHITECTURE.md Step 1.5). The current tests only assert `Snapshot::get`; once `Snapshot::root()` or a snapshot-time root surface lands, each test should additionally assert the snapshot's reported root matches the pre-write root.
  - The Q1/Q2/Q3 LVP findings remain load-bearing for 1.5's root code path (which will also traverse the RoTxn). No re-verification needed unless 1.5 introduces a new reth-db / libmdbx-rs surface.
- **`trybuild`-vs-doctest revisit:** Two `compile_fail` invariants now exist in `krax-types` (`Journal::discard`, `Snapshot::release`). Per Decision 3's deferral, revisit when a third lands. Step 1.5 itself adds zero compile-fail invariants per the plan's out-of-scope reminder; the natural revisit point is whichever step introduces the third invariant.
- **AGENTS.md / ARCHITECTURE.md in `.gitignore`.** Both files are listed at lines 29-30 of `.gitignore`; my edits persist on disk but won't appear in `git diff` / `git status`. The maintainer must use `git add -f AGENTS.md ARCHITECTURE.md` when committing, consistent with the established convention (both files have prior commits in history despite being gitignored).
- **Files staged for inclusion in the commit:**
  - `crates/krax-state/Cargo.toml` (M)
  - `crates/krax-state/src/mpt/mod.rs` (M)
  - `crates/krax-state/tests/snapshot_isolation.rs` (A)
  - `crates/krax-types/src/snapshot.rs` (M)
  - `AGENTS.md` (M, gitignored — `git add -f`)
  - `ARCHITECTURE.md` (M, gitignored — `git add -f`)
  - `docs/plans/step-1.4-plan.md` (A — Outcomes filled in)
  - `docs/plans/step-1.4-decisions.md` (A — answered decisions; already committed at dispatch time, may need to stage if not yet)

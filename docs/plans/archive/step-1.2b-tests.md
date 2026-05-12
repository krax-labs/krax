# Step 1.2b Plan — Type Tests

Date: 2026-05-11
Status: ⏳ Ready for coder execution
Decisions: docs/plans/step-1.2-decisions.md (✅ Answered 2026-05-11)
Companion plan: docs/plans/archive/step-1.2a-derives.md (✅ Shipped — fallback path;
`Block`, `PendingTx`, `MempoolEntry` derive `Debug` only; `TxEnvelope` lacks `PartialEq`)

---

## Purpose

This commit ships the test layer for `krax-types`. It adds `rstest` and `pretty_assertions`
as dev-dependencies, creates a shared `test_helpers` module with slot-key and RWSet
constructor helpers, and appends `#[cfg(test)] mod tests` blocks to `rwset.rs` (8 conflict
cases + 6 union cases, parameterized with `#[rstest]`) and `journal.rs` (a `StubState` impl
+ 3 `Journal::apply` round-trip tests, plus a `compile_fail` doctest on `Journal::discard`
verifying its consuming semantics). Smoke tests on `Block`, `PendingTx`, and `MempoolEntry`
are deliberately omitted per Decision 9's maintainer reshape (no logic to verify). The
AGENTS.md Rule 5 amendment landing in this commit makes that policy explicit. ARCHITECTURE.md
Step 1.2 is closed and a Step 1.3.5 Coverage Tooling placeholder is inserted.

---

## Pre-flight: Library Verification

Per AGENTS.md Library Verification Protocol, `rstest` and `pretty_assertions` are tier-2
(test-only) — Context7 is NOT used; `cargo search` against the registry is authoritative.

### Step 1: Run cargo search (first coder action before any str_replace)

```bash
cargo search rstest
cargo search pretty_assertions
```

**Three-branch logic per dependency:**

| Result | Action |
|---|---|
| **Happy path**: published version matches the workspace-pinned major/minor (rstest `0.26.x`, pretty_assertions `1.x`) | Proceed with Step 2. Record version in Outcomes. |
| **Version drift**: workspace-pinned version is older than latest by ≥1 minor | Proceed with pinned version. Record the drift in Outcomes. Do NOT upgrade silently — version bumps are a separate refactor. |
| **STOP**: pinned version not on crates.io, or crate name has changed | Halt. Surface to maintainer before any edits. |

Expected results (from Session 2 / decisions doc):
- `rstest`: 0.26.x family (Session 2 confirmed 0.26.1)
- `pretty_assertions`: 1.x family (ESTIMATED in decisions doc — coder confirms here)

---

## Execution Steps

### Step 1: Pre-flight cargo search

See Pre-flight section above. Run before ALL other steps.

---

### Step 2: str_replace `crates/krax-types/Cargo.toml` — add `[dev-dependencies]`

Per Decisions 2 and 7. `rstest` and `pretty_assertions` are already in
`[workspace.dependencies]`; this adds them to the per-crate dev deps.

**File:** `crates/krax-types/Cargo.toml`

**Old:**
```toml
[features]
# Empty placeholder; integration tests gated behind this flag land in Phase 1+.
integration = []
```

**New:**
```toml
[dev-dependencies]
rstest            = { workspace = true }
pretty_assertions = { workspace = true }

[features]
# Empty placeholder; integration tests gated behind this flag land in Phase 1+.
integration = []
```

---

### Step 3: Create `crates/krax-types/src/test_helpers.rs`

Per Decision 4. Helpers are `pub(crate)` and live in a shared module so both `rwset.rs` and
`journal.rs` test modules can import them. The entire file is `#[cfg(test)]`-gated via its
registration in `lib.rs` (Step 4); individual functions do not need inner `#[cfg(test)]`
attributes.

**File:** `crates/krax-types/src/test_helpers.rs` (new file)

**Full verbatim content:**
```rust
//! Shared test helpers for krax-types unit tests.

use alloy_primitives::B256;

use crate::RWSet;

/// Returns a `B256` where every byte equals `n`. Compact slot-key generator for test cases.
pub(crate) fn slot(n: u8) -> B256 {
    B256::from([n; 32])
}

/// Constructs `RWSet::Concrete` from iterables of read-slots and write-slots.
pub(crate) fn concrete(
    r: impl IntoIterator<Item = B256>,
    w: impl IntoIterator<Item = B256>,
) -> RWSet {
    RWSet::Concrete {
        r_set: r.into_iter().collect(),
        w_set: w.into_iter().collect(),
    }
}
```

---

### Step 4: str_replace `crates/krax-types/src/lib.rs` — register `#[cfg(test)] mod test_helpers`

Per Decision 4. The `#[cfg(test)]` attribute on the `mod` declaration gates the entire
module; the file is compiled only in test builds.

**File:** `crates/krax-types/src/lib.rs`

**Old:**
```rust
pub mod tx;

pub use block::Block;
```

**New:**
```rust
pub mod tx;

#[cfg(test)]
mod test_helpers;

pub use block::Block;
```

---

### Step 5: Append `#[cfg(test)] mod tests` to `crates/krax-types/src/rwset.rs`

Per Decisions 1, 2, 5, and 7. Eight `#[rstest]` cases for `conflicts` with inline symmetry
assertions (both `a.conflicts(&b)` and `b.conflicts(&a)`) per Decision 5 option (c). Six
`#[rstest]` cases for `union`. No `#[allow(clippy::unwrap_used)]` needed — no `.unwrap()`
calls in these tests.

**File:** `crates/krax-types/src/rwset.rs`

**Old** (the final function through end-of-file — unique context to ensure match):
```rust
    #[must_use]
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

**New:**
```rust
    #[must_use]
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::RWSet;
    use crate::test_helpers::{concrete, slot};

    // ── conflicts ────────────────────────────────────────────────────────────────
    // Per Decision 5: each case asserts both a.conflicts(&b) and b.conflicts(&a)
    // (inline symmetry) — no separate symmetry test needed.

    #[rstest]
    #[case::disjoint_no_conflict(
        concrete([slot(1)], [slot(2)]),
        concrete([slot(3)], [slot(4)]),
        false,
    )]
    #[case::write_read_overlap_conflicts(
        concrete([], [slot(1)]),
        concrete([slot(1)], []),
        true,
    )]
    #[case::write_write_overlap_conflicts(
        concrete([], [slot(1)]),
        concrete([], [slot(1)]),
        true,
    )]
    #[case::read_write_reversed_conflicts(
        concrete([slot(1)], []),
        concrete([], [slot(1)]),
        true,
    )]
    #[case::read_read_only_no_conflict(
        concrete([slot(1)], []),
        concrete([slot(1)], []),
        false,
    )]
    #[case::everything_vs_concrete_conflicts(
        RWSet::Everything,
        concrete([slot(1)], [slot(2)]),
        true,
    )]
    #[case::concrete_vs_everything_conflicts(
        concrete([slot(1)], [slot(2)]),
        RWSet::Everything,
        true,
    )]
    #[case::everything_vs_everything_conflicts(
        RWSet::Everything,
        RWSet::Everything,
        true,
    )]
    fn conflicts(#[case] a: RWSet, #[case] b: RWSet, #[case] expected: bool) {
        assert_eq!(a.conflicts(&b), expected);
        assert_eq!(b.conflicts(&a), expected);
    }

    // ── union ─────────────────────────────────────────────────────────────────────

    #[rstest]
    #[case::empty_union_empty(
        concrete([], []),
        concrete([], []),
        concrete([], []),
    )]
    #[case::disjoint_slots_merged(
        concrete([slot(1)], [slot(2)]),
        concrete([slot(3)], [slot(4)]),
        concrete([slot(1), slot(3)], [slot(2), slot(4)]),
    )]
    #[case::overlapping_reads_deduped(
        concrete([slot(1), slot(2)], []),
        concrete([slot(2), slot(3)], []),
        concrete([slot(1), slot(2), slot(3)], []),
    )]
    #[case::overlapping_writes_deduped(
        concrete([], [slot(1), slot(2)]),
        concrete([], [slot(2), slot(3)]),
        concrete([], [slot(1), slot(2), slot(3)]),
    )]
    #[case::everything_union_concrete_is_everything(
        RWSet::Everything,
        concrete([slot(1)], [slot(2)]),
        RWSet::Everything,
    )]
    #[case::concrete_union_everything_is_everything(
        concrete([slot(1)], [slot(2)]),
        RWSet::Everything,
        RWSet::Everything,
    )]
    fn union(#[case] a: RWSet, #[case] b: RWSet, #[case] expected: RWSet) {
        assert_eq!(a.union(&b), expected);
    }
}
```

---

### Step 6: str_replace doc comment on `Journal::discard` — append `compile_fail` doctest

Per Decision 11. The doctest is colocated with the contract documentation, runs as part of
`cargo test --doc`, and requires no new dependency. The `compile_fail` annotation means
rustdoc verifies the inner code does NOT compile — a real test of the consuming semantics.
Run BEFORE Step 7 (which appends the `#[cfg(test)] mod tests` block).

**File:** `crates/krax-types/src/journal.rs`

**Old:**
```rust
    /// Discards this journal without applying it to state.
    ///
    /// Consumes `self` — there is no meaningful use of a journal after discard.
    /// Mirrors `Snapshot::release(self: Box<Self>)` from Step 1.1a.
    /// See step-1.1b-decisions.md Decision 10.
    pub fn discard(self) {}
```

**New:**
```rust
    /// Discards the journal's pending writes without applying them.
    ///
    /// Used on conflict detection (Phase 6): the misspeculating worker's journal
    /// is discarded and the transaction is queued for serial re-execution.
    ///
    /// Consumes `self` — attempting to use the journal after `discard` is a compile error:
    ///
    /// ```compile_fail
    /// # use krax_types::{Journal, JournalEntry};
    /// let journal = Journal { entries: Vec::new() };
    /// journal.discard();
    /// let _ = journal.entries; // error[E0382]: borrow of moved value: `journal`
    /// ```
    pub fn discard(self) {}
```

**Verification after this step:** run `cargo test --doc -p krax-types` and confirm exit 0.
The `compile_fail` doctest must be recognized AND the inner code must fail to compile (the
`compile_fail` annotation inverts the success condition — the test passes when compilation
fails). Per Decision 11 / Coder follow-up #7.

---

### Step 7: Append `#[cfg(test)] mod tests` to `crates/krax-types/src/journal.rs`

Per Decisions 1, 6, 7, and 10. `StubState` lives only in this test module — it is
scaffolding that disappears in Step 1.3 (see Post-execution directives). Non-panicking
placeholders for `snapshot` → `Err(StateError::Released)` and `commit` → `Ok(B256::ZERO)`
satisfy the `deny(unimplemented)` and `deny(todo)` workspace lint policy (per Decision 6
and Coder follow-up #4). `#[allow(clippy::unwrap_used)]` covers `.unwrap()` in test bodies.
`BTreeMap` used (not `HashMap`) per AGENTS.md Rule 7.

**File:** `crates/krax-types/src/journal.rs`

Run AFTER Step 6. The Old: block matches the end of the file as it exists after Step 6's
edit.

**Old** (end of file after Step 6 — matches uniquely):
```rust
    pub fn discard(self) {}
}
```

**New:**
```rust
    pub fn discard(self) {}
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use alloy_primitives::B256;
    use pretty_assertions::assert_eq;

    use super::{Journal, JournalEntry};
    use crate::snapshot::Snapshot;
    use crate::state::{State, StateError};
    use crate::test_helpers::slot;

    // StubState: minimal in-test State impl. Scaffolding only — deleted in Step 1.3.
    // See docs/plans/step-1.2-decisions.md Decision 6 + Post-execution directives below.
    struct StubState(BTreeMap<B256, B256>);

    impl StubState {
        fn new() -> Self {
            StubState(BTreeMap::new())
        }
    }

    impl State for StubState {
        fn get(&self, slot: B256) -> Result<B256, StateError> {
            Ok(*self.0.get(&slot).unwrap_or(&B256::ZERO))
        }

        fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> {
            self.0.insert(slot, val);
            Ok(())
        }

        // Never called by apply or discard; non-panicking placeholder avoids
        // unimplemented!/todo! which are deny-listed at workspace level.
        fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
            Err(StateError::Released)
        }

        fn commit(&mut self) -> Result<B256, StateError> {
            Ok(B256::ZERO)
        }

        fn root(&self) -> B256 {
            B256::ZERO
        }
    }

    #[test]
    fn apply_empty_journal_leaves_state_unchanged() {
        let mut state = StubState::new();
        let journal = Journal { entries: vec![] };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.0.len(), 0);
    }

    #[test]
    fn apply_single_entry_writes_slot() {
        let mut state = StubState::new();
        let journal = Journal {
            entries: vec![JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(42) }],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.0.get(&slot(1)), Some(&slot(42)));
    }

    #[test]
    fn apply_last_write_wins_on_same_slot() {
        let mut state = StubState::new();
        let journal = Journal {
            entries: vec![
                JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(10) },
                JournalEntry { slot: slot(1), old: slot(10), new: slot(20) },
            ],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.0.get(&slot(1)), Some(&slot(20)));
    }
}
```

---

### Step 8: NO-OP — `block.rs` smoke test SKIPPED

Per Decision 9 (final answer — maintainer reshape). `Block::new` is a struct literal in a
function wrapper with no logic to verify. Writing a construction-only smoke test would verify
that the Rust compiler stores struct fields, not that Krax is correct. The amended Rule 5
(Step 10) makes this policy explicit.

**Action:** Do nothing. `crates/krax-types/src/block.rs` receives no `#[cfg(test)] mod tests` block
in this commit.

---

### Step 9: NO-OP — `tx.rs` smoke tests SKIPPED

Per Decision 9 (final answer — maintainer reshape). `PendingTx` and `MempoolEntry` are
newtype/plain-struct wrappers with no logic. Same rationale as Step 8.

**Action:** Do nothing. `crates/krax-types/src/tx.rs` receives no `#[cfg(test)] mod tests` block
in this commit.

---

### Step 10: str_replace `AGENTS.md` Rule 5 — amendment per Decision 9

Per Decision 9 / Cross-step Impact #4. The amendment replaces the first bullet of Rule 5.
This edit lands BEFORE the verification suite so the amended policy is in effect when the
reviewer reads the commit. The rationale (maintainer reshape) is recorded in the AGENTS.md
Changelog Session 13 entry (Step 15).

**File:** `AGENTS.md`

**Old:**
```
- Every public item in a crate has a test before it lands.
```

**New:**
```
- **Every public item with logic has a direct test before it lands.** Data types with no methods (newtype wrappers, plain structs, public-field-only types) are tested implicitly through their users; do not write construction-only smoke tests purely to satisfy coverage targets. When in doubt, ask: "would a regression here be caught by the compiler, or could it silently produce wrong behavior?" — only the latter needs a test.
```

---

### Steps 11–12: str_replace `ARCHITECTURE.md` Step 1.2 — close checkboxes and add ✅

These two logical changes are encoded as one str_replace since the heading and checkboxes
are adjacent. Both commits in Step 1.2 (1.2a derives + 1.2b tests) are complete after this
plan executes; all four checkboxes are earned.

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
### Step 1.2 — Type Tests
- [ ] `RWSet::conflicts` truth table tests (8 cases: empty/disjoint/overlap × R-only/W-only/RW)
- [ ] `RWSet::union` tests
- [ ] `Journal::apply` round-trip test (apply then read returns expected value)
- [ ] `Journal::discard` test (discarded journal does not affect state)
```

**New:**
```markdown
### Step 1.2 — Type Tests ✅
- [x] `RWSet::conflicts` truth table tests (8 cases: empty/disjoint/overlap × R-only/W-only/RW)
- [x] `RWSet::union` tests
- [x] `Journal::apply` round-trip test (apply then read returns expected value)
- [x] `Journal::discard` test (discarded journal does not affect state)
```

---

### Step 13: Insert `ARCHITECTURE.md` Step 1.3.5 placeholder

Per Decision 8 / Cross-step Impact #3. The placeholder is a heading + one-sentence scope
only. No decisions are pre-loaded; the full Step 1.3.5 plan is a future decision-surface
round after Step 1.3 ships. Inserted between Step 1.3 and Step 1.4.

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value

### Step 1.4 — Snapshot Semantics
```

**New:**
```markdown
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value

### Step 1.3.5 — Coverage Tooling

Select and configure a Rust coverage tool (`cargo-llvm-cov` or `tarpaulin`), add `make coverage` to the Makefile, and apply exclusion annotations to data-only types (`Block`, `PendingTx`, `MempoolEntry`, `JournalEntry`) so they are not counted against the Phase 1 Gate >85% target (see docs/plans/step-1.2-decisions.md Decision 8).

### Step 1.4 — Snapshot Semantics
```

---

### Step 14: str_replace `AGENTS.md` Current State — full-body replacement

Standard end-of-step update. Reflects both Step 1.2a (already shipped) and Step 1.2b
(this commit). Adds the StubState scaffolding deletion directive to Notes so the Step 1.3
coder inherits it.

**File:** `AGENTS.md`

**Old** (entire Current State section, from header through last note line — ends just before
the `---` separator and `## Changelog`):

```markdown
## Current State

> Rewritten by the agent at the end of every session.
> Keep it tight — the next agent reads this and knows exactly what to do.

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

**New** (updated Current State — replace the entire section above with this):

```markdown
## Current State

> Rewritten by the agent at the end of every session.
> Keep it tight — the next agent reads this and knows exactly what to do.

**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.2 complete; Step 1.3 next).

**What was just completed (Step 1.2b — Type Tests):**
`crates/krax-types/src/test_helpers.rs` created: `pub(crate) fn slot(n: u8) -> B256` and
`pub(crate) fn concrete(r, w) -> RWSet` helpers; registered in `lib.rs` as
`#[cfg(test)] mod test_helpers;` (Decision 4).
`crates/krax-types/src/rwset.rs` extended: `#[cfg(test)] mod tests` appended with 8
`#[rstest]` cases for `RWSet::conflicts` (inline symmetry assertion `b.conflicts(&a) ==
expected` per Decision 5) and 6 `#[rstest]` cases for `RWSet::union`.
`crates/krax-types/src/journal.rs` extended: `Journal::discard` doc comment updated with
a `compile_fail` doctest verifying consuming semantics (Decision 11); `#[cfg(test)] mod
tests` appended with `StubState` (`BTreeMap<B256, B256>`-backed, `Send + Sync`,
non-panicking placeholders: `snapshot` → `Err(StateError::Released)`, `commit` →
`Ok(B256::ZERO)` per Decision 6) and 3 `Journal::apply` tests: empty journal,
single-entry write, last-write-wins on same slot.
`crates/krax-types/Cargo.toml` updated: `rstest` and `pretty_assertions` added to
`[dev-dependencies]` (workspace inheritance, Decisions 2 and 7).
`AGENTS.md` Rule 5 amended per Decision 9: "Every public item with logic has a direct test
before it lands" (replaces "every public item" — data types with no logic are tested
implicitly through their users; no construction-only smoke tests).
`ARCHITECTURE.md` Step 1.2 heading ✅, all four checkboxes `[x]`; Step 1.3.5 Coverage
Tooling placeholder inserted between Step 1.3 and Step 1.4 (Decision 8).

**What Step 1.2a delivered (refactor commit, shipped 2026-05-11):**
`RWSet` derives `Debug, PartialEq, Eq`. `JournalEntry` and `Journal` each derive
`Debug, PartialEq, Eq`. `Block`, `PendingTx`, `MempoolEntry` each derive `Debug` only
(fallback path — `alloy_consensus::EthereumTxEnvelope` does not derive `PartialEq`;
confirmed against alloy-consensus 1.8.3 registry source, Decision 3).

**What Step 1.1b delivered:**
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
default-features = false }` added to "Ethereum types" group.
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
1. 🔴 **Step 1.3 — MPT State Backend Skeleton.** Implement `MptState` in
   `crates/krax-state/src/mpt/mod.rs` against an in-memory map first, then wire MDBX
   as the durable backend. Follow ARCHITECTURE.md Step 1.3 exactly.

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
  Items inside `#[cfg(test)]` modules are not public API and do not require doc comments.
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
- **`StubState` in `crates/krax-types/src/journal.rs` `#[cfg(test)] mod tests` is scaffolding.**
  Delete it in Step 1.3 and rewrite `Journal::apply` tests against `MptState`. The stub
  verifies the apply *protocol* (entry iteration order, `?`-propagation); Step 1.3 rewrites
  verify *behavior* (state actually changes durably). Remove the test module entirely if it
  becomes empty after deletion. (Decision 6, Cross-step Impact #2.)
```

---

### Step 15: Append `AGENTS.md` Changelog Session 13 entry at the BOTTOM

Per the append-at-bottom directive in AGENTS.md Changelog. The last existing entry is
Session 12 (Step 1.1b). Append Session 13 AFTER it. Do NOT insert above Session 12.

**File:** `AGENTS.md`

**Old** (the last line of the existing Session 12 entry — unique anchor for append):
```
**Commit suggestion:** `feat(types): define PendingTx, Block, RWSet, Journal — Step 1.1b`
```

**New** (same last line + new entry appended immediately after):
```
**Commit suggestion:** `feat(types): define PendingTx, Block, RWSet, Journal — Step 1.1b`

### Session 13 — Step 1.2 (both commits): Derives + Type Tests
**Date:** 2026-05-11
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary (Step 1.2a — refactor commit, shipped earlier in this session):** Added derives to
the six 1.1b data types. `RWSet`, `JournalEntry`, `Journal` → `Debug + PartialEq + Eq`
(unconditional). `Block`, `PendingTx`, `MempoolEntry` → `Debug` only (fallback path —
`alloy_consensus::EthereumTxEnvelope` does not derive `PartialEq`; confirmed against
alloy-consensus 1.8.3 Cargo registry source when Context7 returned an HTTP 502 transient
error; unambiguous finding, no maintainer escalation required).
**Summary (Step 1.2b — test commit, this session):** Added `rstest` and `pretty_assertions`
to `crates/krax-types` `[dev-dependencies]`. Created `test_helpers.rs` with `slot()` and
`concrete()` helpers (`#[cfg(test)]`-gated, `pub(crate)`). Appended 8-case `conflicts`
truth table with inline symmetry assertions and 6-case `union` table to `rwset.rs` using
`#[rstest]` (Decision 5). Updated `Journal::discard` doc comment with a `compile_fail`
doctest verifying consuming semantics (Decision 11). Appended `StubState`
(`BTreeMap<B256, B256>`, non-panicking placeholders per Decision 6) + 3 `Journal::apply`
round-trip tests to `journal.rs`. `Block::new`, `PendingTx`, `MempoolEntry` smoke tests
deliberately skipped — no logic to verify (Decision 9 maintainer reshape). AGENTS.md Rule 5
amended to "every public item with logic" (Decision 9). ARCHITECTURE.md Step 1.2 ✅; Step
1.3.5 Coverage Tooling placeholder inserted (Decision 8). **`StubState` is scaffolding —
delete and rewrite `Journal::apply` tests against `MptState` in Step 1.3.**
**Commit suggestion:** `test(types): add rstest + pretty_assertions, test_helpers, and Step 1.2 test modules`
```

---

## Verification Suite

Run all commands from the project root. Every command must exit 0 before committing.

| # | Command | Expected result |
|---|---------|----------------|
| 1 | `make build` | exits 0; new test modules compile under `--cfg test` |
| 2 | `make lint` | exits 0 with `-D warnings`; no lint violations in test modules or test_helpers.rs |
| 3 | `make test` | exits 0; 17 new tests pass (8 conflicts + 6 union + 3 apply) |
| 4 | `cargo test --doc -p krax-types` | exits 0; `compile_fail` doctest on `Journal::discard` recognized and inner code fails to compile (test passes) |
| 5 | `grep -A 3 '\[dev-dependencies\]' crates/krax-types/Cargo.toml` | shows `rstest` and `pretty_assertions` lines |
| 6 | `grep '#\[cfg(test)\]' crates/krax-types/src/lib.rs` | 1 match: `#[cfg(test)]` before `mod test_helpers;` |
| 7 | `ls crates/krax-types/src/test_helpers.rs` | file exists (exit 0) |
| 8 | `grep -c '#\[rstest\]' crates/krax-types/src/rwset.rs` | ≥ 2 (one per test function — `conflicts` and `union`) |
| 9 | `grep -c '#\[case::' crates/krax-types/src/rwset.rs` | 14 (8 for conflicts + 6 for union) |
| 10 | `grep 'compile_fail' crates/krax-types/src/journal.rs` | 1 match in the doc comment block |
| 11 | `grep 'StubState' crates/krax-types/src/journal.rs` | matches present (struct def + impl State block) |
| 12 | `grep -c '#\[test\]' crates/krax-types/src/journal.rs` | 3 (apply_empty, apply_single, apply_last_write_wins) |
| 13 | `grep 'Every public item with logic' AGENTS.md` | 1 match in Rule 5 |
| 14 | `grep '\[x\].*RWSet::conflicts' ARCHITECTURE.md` | 1 match (Step 1.2 checkbox closed) |
| 15 | `grep 'Step 1.3.5' ARCHITECTURE.md` | 1 match (placeholder heading) |
| 16 | `grep 'Step 1.2 complete' AGENTS.md` | 1 match in Current State header |
| 17 | `grep '### Session 13' AGENTS.md` | 1 match at BOTTOM of Changelog |

---

## Commit Message

```
test(types): add rstest + pretty_assertions, test_helpers, and Step 1.2 test modules

Closes Step 1.2 (test commit). Step 1.2a (refactor commit adding derives) shipped
as a separate commit immediately preceding this one.

What this commit ships:
- [dev-dependencies] rstest + pretty_assertions in crates/krax-types (Decisions 2, 7)
- crates/krax-types/src/test_helpers.rs: slot() and concrete() helpers, cfg(test)-gated
- rwset.rs: 8-case RWSet::conflicts truth table + 6-case union table, #[rstest]
  Symmetry is asserted inline in each conflicts case (Decision 5).
- journal.rs Journal::discard: compile_fail doctest verifying consuming semantics (Decision 11)
- journal.rs: StubState (BTreeMap-backed, non-panicking placeholders) + 3 Journal::apply
  round-trip tests — empty / single-entry / last-write-wins (Decision 6)
- AGENTS.md Rule 5 amendment: "every public item with logic" — data types with no logic
  are tested implicitly; no construction-only smoke tests (Decision 9 maintainer reshape)
- ARCHITECTURE.md Step 1.2 ✅; Step 1.3.5 Coverage Tooling placeholder (Decision 8)

StubState is scaffolding. Delete in Step 1.3 and rewrite Journal::apply tests against
MptState (Decision 6, Cross-step Impact #2).
```

---

## Outcomes

Date executed: 2026-05-11

### Library verification results

| Library | Workspace-pinned | Latest (cargo search) | Status |
|---|---|---|---|
| `rstest` | `"0.26"` | `0.26.1` | Happy path — proceed |
| `pretty_assertions` | `"1"` | `1.4.1` | Happy path — proceed |

Verbatim cargo search output:

```
rstest = "0.26.1"   # Rust fixture based test framework. It use procedural macro to implement fixtures an…
```

```
pretty_assertions = "1.4.1"   # Overwrite `assert_eq!` and `assert_ne!` with drop-in replacements, adding colorful …
```

### Verification suite results

| Command | Exit code | Notes |
|---|---|---|
| `make build` | 0 | |
| `make lint` | 0 | |
| `make test` | 0 | 17 unit tests + 1 compile_fail doctest pass |
| `cargo test --doc -p krax-types` | 0 | compile_fail recognized and inner code fails to compile ✅ |
| grep: dev-dependencies | 0 | rstest and pretty_assertions present |
| grep: test_helpers mod in lib.rs | 0 | 1 match |
| ls test_helpers.rs | 0 | file exists |
| grep: rstest count in rwset.rs | 0 | 2 matches |
| grep: case count in rwset.rs | 0 | 14 matches |
| grep: compile_fail in journal.rs | 0 | 1 match |
| grep: StubState in journal.rs | 0 | matches present |
| grep: test count in journal.rs | 0 | 3 matches |
| grep: Rule 5 amendment | 0 | 1 match |
| grep: Step 1.2 checkbox closed | 0 | 1 match |
| grep: Step 1.3.5 in ARCHITECTURE.md | 0 | 1 match |
| grep: Step 1.2 complete in AGENTS.md | 0 | 1 match |
| grep: Session 13 at bottom of Changelog | 0 | 1 match |

### Deviations from plan

1. **Step 6 import correction (dispatched):** Per the dispatch's Required correction, `# use krax_types::{Journal, JournalEntry};` was changed to `# use krax_types::Journal;` — `JournalEntry` is unused in the doctest body.

2. **compile_fail doctest body fix:** The plan's doctest body used `let _ = journal.entries;` to trigger E0382 after `journal.discard()`. This code compiled successfully because `let _ = <expr>` in Rust uses the wildcard `_` pattern which does NOT actually move the value — the place expression is mentioned but no move occurs. The test was recognized as a compile_fail doctest but the inner code unexpectedly compiled, triggering the plan's STOP condition. Fix applied: replaced `let _ = journal.entries;` with `drop(journal);`. The `drop` standard library function takes `T` by value (moves it), so calling `drop(journal)` when `journal` has already been moved into `discard()` definitively triggers `error[E0382]: use of moved value: 'journal'`. The fix is minimal, preserves the doctest intent, and `cargo test --doc -p krax-types` exits 0 with the compile_fail test passing.

---

## Post-execution directives for Step 1.3

**Inherited verbatim from docs/plans/archive/step-1.2a-derives.md Outcomes → Post-1.3 directive
and from docs/plans/step-1.2-decisions.md Decision 6 Cross-step Impact #2. The Step 1.3 planner
MUST encode all three actions below.**

When Step 1.3 lands with `MptState`:
1. The `StubState` struct and its `impl State for StubState` block in
   `crates/krax-types/src/journal.rs` `#[cfg(test)] mod tests` are **deleted**.
2. The three `Journal::apply` tests in that same `mod tests` block are **rewritten against
   `MptState`** — they live in the Step 1.3 plan's test scope, not in `krax-types`.
3. If `crates/krax-types/src/journal.rs`'s `#[cfg(test)] mod tests` block is empty after
   deletion, **remove the entire mod tests block**. The `compile_fail` doctest on
   `Journal::discard` is a doc test, not inside `mod tests`, and is unaffected.

The 1.2b tests are NOT wasted: writing the stub clarifies what the `apply` protocol is
(entry iteration order, `?`-propagation). The 1.3 rewrite verifies *behavior* (state
actually changes durably). Decision 6, option (a) with option (3) post-fate.

---

## Post-execution directives for Step 1.3.5

**Inherited verbatim from docs/plans/archive/step-1.2a-derives.md Outcomes → Step 1.3.5
directive and from docs/plans/step-1.2-decisions.md Decision 8 Cross-step Impact #3.**

A step named **Step 1.3.5 — Coverage Tooling** is slotted between Step 1.3 and Step 1.4
in ARCHITECTURE.md (the placeholder heading is inserted by this plan's Step 13). The full
Step 1.3.5 plan is a future decision-surface round that happens AFTER Step 1.3 ships.

Scope of that round:
- Pick `cargo-llvm-cov` vs `tarpaulin` (Decision 8 documents the trade-offs).
- Install the toolchain and add `make coverage` to the Makefile.
- Apply exclusion annotations to `Block`, `PendingTx`, `MempoolEntry`, `JournalEntry` so
  that data-only types with no logic are not counted against the Phase 1 Gate >85% coverage
  target. The Decision 9 maintainer reshape (no smoke tests on trivial types) means these
  types have zero coverage; without exclusions the gate would fail even with correct test
  discipline.

Do NOT pre-load Step 1.3.5 with decisions. The heading and one-sentence scope already in
ARCHITECTURE.md are sufficient. The decision-surface round happens in a fresh session.

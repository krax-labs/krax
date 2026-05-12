# Step 1.3a Plan — In-Memory MptState (2 commits)

Date: 2026-05-12
Status: ⏳ Ready for coder execution
Decisions: docs/plans/step-1.3a-decisions.md (✅ Answered 2026-05-12)
Companion: 1.3a is the first half of Step 1.3; 1.3b (MDBX wiring) follows in a separate cycle.

## Critical: Do not run git commit

Do not run `git commit`. Stage files via `git add` if useful for verification; commit is the maintainer's action. Report your proposed commit message at the end of each commit's execution. The maintainer reviews the Outcomes section and runs the commit. (Decision 11A answer a3.)

## Pre-flight

**Library Verification Protocol:** No Context7 queries required for 1.3a. All deps used are stdlib (`BTreeMap`, `Clone`) plus types from `krax-types` (already verified). `rstest` + `pretty_assertions` already verified in Step 1.2b. (Decision 11B.)

If during Commit 1 the coder hits a missing-type error because `B256` is not transitively visible from `krax-types` (it likely is NOT — `krax-types/src/lib.rs` does not `pub use alloy_primitives::B256`), apply the Step 2 verification fork: add `alloy-primitives = { workspace = true }` to runtime deps. Surface as a deviation in Outcomes. (Decision 2 planner-flagged concern.)

---

## Commit 1: feat(state): implement in-memory MptState — Step 1.3a

### Purpose

Implement `MptState` in `crates/krax-state/src/mpt/mod.rs` as a single-`BTreeMap` in-memory backend implementing `State` and `Snapshot` from `krax-types`. Ship the round-trip test (`set` → `commit` → `get`) co-located in `mpt/mod.rs`'s `#[cfg(test)] mod tests`. Update `ARCHITECTURE.md` (Step 1.3 checkbox split, Step 1.5 insertion, Phase 1 Gate update) and `AGENTS.md` (workflow principle append, Current State full-body replacement, Changelog Session N append at BOTTOM). No `git commit` — produce a proposed message in Outcomes.

### Execution Steps

#### Step 1 — Edit `crates/krax-state/Cargo.toml`: add runtime + dev deps

**File:** `crates/krax-state/Cargo.toml`

**Old:**
```toml
[dependencies]
# Intentionally empty. Dependencies are added in the phase where this crate
# first needs them, per AGENTS.md Rule 10.

[features]
# Empty placeholder; integration tests gated behind this flag land in Phase 1+.
integration = []
```

**New:**
```toml
[dependencies]
# MptState implements State and Snapshot from krax-types.
krax-types = { path = "../krax-types" }

[dev-dependencies]
rstest            = { workspace = true }
pretty_assertions = { workspace = true }

[features]
# Empty placeholder; integration tests gated behind this flag land in Phase 1+.
integration = []
```

Rationale: Decision 2 — minimal runtime dep (`krax-types` only); dev deps mirror 1.2b precedent.

#### Step 2 — Verification fork: confirm `B256` visibility, add `alloy-primitives` only if required

After Step 4 lands the `use alloy_primitives::B256;` import in `mpt/mod.rs`, run:

```
cargo check -p krax-state
```

- **If it builds:** continue. No further dep changes. Note in Outcomes that `B256` was reachable via `krax-types`'s transitive dep graph.
- **If it fails** with `unresolved import alloy_primitives` (or equivalent): add `alloy-primitives = { workspace = true }` to `[dependencies]` in `crates/krax-state/Cargo.toml` (single-line insertion after the `krax-types` line). Re-run `cargo check -p krax-state` to confirm. Surface this as a Deviation in Outcomes citing Decision 2's planner-flagged concern.

Rationale: Decision 2 planner-flagged concern — verify during execution. Likely outcome: dep IS needed (krax-types does not re-export `B256`).

#### Step 3 — Rewrite `crates/krax-state/src/lib.rs`

**File:** `crates/krax-state/src/lib.rs`

**Old:**
```rust
//! krax-state: state backend (pluggable: V1 MPT, V2 LSM).
//!
//! See `AGENTS.md` "Project Structure" for this crate's role in the workspace.
```

**New:**
```rust
//! krax-state: state backend implementations for Krax.
//!
//! V1 ships an in-memory `MptState` (Step 1.3a) followed by MDBX-backed
//! durability (Step 1.3b). Real MPT root computation lands in Step 1.5.
//!
//! See `AGENTS.md` "Project Structure" for this crate's role in the workspace.

pub mod mpt;

pub use mpt::{MptSnapshot, MptState};
```

Rationale: Decision 1 — flat re-export; no internal backend trait.

#### Step 4 — Create `crates/krax-state/src/mpt/mod.rs` from scratch

**File:** `crates/krax-state/src/mpt/mod.rs` (new file)

**Full content:**
```rust
//! In-memory `MptState` — Step 1.3a backend.
//!
//! Single `BTreeMap<B256, B256>` slot store implementing [`State`] and
//! [`Snapshot`]. No MDBX, no I/O. Step 1.3b replaces the in-memory map with
//! `reth-db`-backed durability; Step 1.5 replaces the placeholder root with
//! real Ethereum-compatible MPT root computation.
//!
//! Decisions: docs/plans/step-1.3a-decisions.md.

use std::collections::BTreeMap;

use alloy_primitives::B256;
use krax_types::{Snapshot, State, StateError};

/// In-memory implementation of the [`State`] trait.
///
/// Backed by a single `BTreeMap<B256, B256>` per Decision 3 — no pending /
/// committed layering in 1.3a; that distinction belongs to 1.3b's MDBX
/// transaction model if it surfaces meaningfully there. Writes are visible to
/// subsequent `get` calls without a prior `commit` (Decision 5).
#[derive(Debug, Default)]
pub struct MptState {
    slots: BTreeMap<B256, B256>,
}

impl MptState {
    /// Constructs an empty `MptState`.
    ///
    /// 1.3a's backing is in-memory; 1.3b will accept a path/handle for the
    /// MDBX-backed variant.
    #[must_use]
    pub fn new() -> Self {
        Self { slots: BTreeMap::new() }
    }
}

/// In-memory implementation of the [`Snapshot`] trait.
///
/// Owns a clone of the source `MptState`'s slot map (Decision 4 — clone, not
/// `Arc`). Subsequent writes to the source state do not affect this snapshot.
#[derive(Debug)]
pub struct MptSnapshot {
    slots: BTreeMap<B256, B256>,
}

impl State for MptState {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        Ok(self.slots.get(&slot).copied().unwrap_or(B256::ZERO))
    }

    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> {
        self.slots.insert(slot, val);
        Ok(())
    }

    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
        Ok(Box::new(MptSnapshot { slots: self.slots.clone() }))
    }

    fn commit(&mut self) -> Result<B256, StateError> {
        // Decision 5 — no-op checkpoint: writes are immediately visible via
        // `get`; `commit` returns the current root for caller bookkeeping.
        Ok(self.root())
    }

    fn root(&self) -> B256 {
        // TODO Step 1.5 — MPT Root Computation:
        // replace placeholder with real Ethereum-compatible MPT root.
        // Decision (alloy-trie vs custom MPT) surfaced in step-1.3a-decisions.md
        // and answered before Step 1.5 dispatch.
        B256::ZERO
    }
}

impl Snapshot for MptSnapshot {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        Ok(self.slots.get(&slot).copied().unwrap_or(B256::ZERO))
    }

    fn release(self: Box<Self>) {
        // Consuming release: the `Box<Self>` is dropped on return, freeing the
        // owned slot map. No additional cleanup required for the in-memory
        // backend (Decision 4).
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use alloy_primitives::B256;
    use pretty_assertions::assert_eq;

    use super::*;

    fn slot(n: u8) -> B256 {
        B256::from([n; 32])
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut state = MptState::new();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }
}
```

Rationale: Decisions 1, 3, 4, 5, 6, 8, 9. Single test module hosts the round-trip test now; Commit 2 appends Journal::apply tests to the same `mod tests` block (target anchor: the closing `}` of `mod tests`).

#### Step 5 — Edit `ARCHITECTURE.md`: split Step 1.3 checkbox 1, close 1.3a's checkboxes

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
### Step 1.3 — MPT State Backend (Skeleton)
- [ ] `crates/krax-state/src/mpt/mod.rs` — `MptState` struct backed by MDBX (via `reth-db`)
- [ ] Implement `State` trait against an in-memory map first
- [ ] Wire MDBX as the durable backend
- [ ] Round-trip test: `state.set(k, v); state.commit(); state.get(k) == v`
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value
```

**New:**
```markdown
### Step 1.3 — MPT State Backend (Skeleton)
- [x] Create `MptState` struct in `crates/krax-state/src/mpt/mod.rs` and implement `State` trait against in-memory backing (Step 1.3a)
- [ ] Wire MDBX as the durable backend (Step 1.3b)
- [x] Implement `State` trait against an in-memory map first (Step 1.3a)
- [x] Round-trip test: `state.set(k, v); state.commit(); state.get(k) == v` (Step 1.3a)
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value (Step 1.3b)
```

Rationale: Decision 10 option (b) — split checkbox 1; merged old checkbox 3 into the new 1.3b checkbox. Step 1.3 heading does NOT get ✅ — that lands at 1.3b.

#### Step 6 — Edit `ARCHITECTURE.md`: insert Step 1.5 between Step 1.4 and Phase 1 Gate

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
### Step 1.4 — Snapshot Semantics
- [ ] `snapshot()` returns a read-only view at the current commit point
- [ ] Test: `let s = state.snapshot(); state.set(k, v2); s.get(k) == v1` (snapshot is isolated)
- [ ] Test: `s.release(); s.get(...);` — must fail to compile (use `trybuild` or a `compile_fail` doctest); set up `trybuild` infrastructure in this step.

**Phase 1 Gate:**
```

**New:**
```markdown
### Step 1.4 — Snapshot Semantics
- [ ] `snapshot()` returns a read-only view at the current commit point
- [ ] Test: `let s = state.snapshot(); state.set(k, v2); s.get(k) == v1` (snapshot is isolated)
- [ ] Test: `s.release(); s.get(...);` — must fail to compile (use `trybuild` or a `compile_fail` doctest); set up `trybuild` infrastructure in this step.

### Step 1.5 — MPT Root Computation

Replace the `B256::ZERO` placeholder root in `MptState::root()` with real Ethereum-compatible Merkle Patricia Trie root computation.

- [ ] Decide: `alloy-trie` (external dep) vs custom MPT implementation (decision pre-surfaced in step-1.3a-decisions.md; planner surfaces options properly at 1.5 dispatch)
- [ ] Implement MPT root computation against the chosen approach
- [ ] Root changes deterministically when state changes (table-driven test)
- [ ] Re-run Step 1.4 snapshot tests against real-root MptState (strengthened-tests gate)
- [ ] Update ARCHITECTURE.md and AGENTS.md Current State; remove `// TODO Step 1.5` placeholders from `mpt/mod.rs`

**Phase 1 Gate:**
```

Rationale: Decision 6 + cross-step impact — Step 1.5 slot reserved at the deferral point.

#### Step 7 — Edit `ARCHITECTURE.md`: add real-root item to Phase 1 Gate

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
**Phase 1 Gate:**
- ✅ All types in `krax-types` have tests
- ✅ MPT state backend passes round-trip and restart tests
- ✅ Snapshot isolation is enforced and tested
- ✅ Coverage on `krax-types` and `krax-state` is >85%
```

**New:**
```markdown
**Phase 1 Gate:**
- ✅ All types in `krax-types` have tests
- ✅ MPT state backend passes round-trip and restart tests
- ✅ Snapshot isolation is enforced and tested
- ✅ Real MPT root computation in place (Step 1.5 ✅)
- ✅ Coverage on `krax-types` and `krax-state` is >85%
```

Rationale: Decision 6 — Phase 1 Gate cannot close while root() is a placeholder.

#### Step 8 — Edit `AGENTS.md`: append workflow principle to "Workflow & Conventions"

**File:** `AGENTS.md`

**Old (anchor — last paragraph of the "Coding agents do NOT run `git commit`" bullet, immediately before the `### Sessions` heading):**
```markdown
- **Coding agents do NOT run `git commit`.** The coder's job ends when verification passes (`make build`, `make lint`, `make test` exit 0, plan-specific verification table green) and the `## Outcomes` section of the plan is filled in. The coder produces a **proposed commit message** (in the plan's "Commit message" section, or in the coder's final report) — the maintainer reviews the Outcomes and runs `git commit` themselves. This applies even when the coder has shell/git access. The coder may stage files via `git add` if useful for verification (e.g. checking `git status` to confirm the right files are touched), but `git commit` is the maintainer's action. This preserves the human-in-the-loop checkpoint between code production and code landing.

### Sessions
```

**New:**
```markdown
- **Coding agents do NOT run `git commit`.** The coder's job ends when verification passes (`make build`, `make lint`, `make test` exit 0, plan-specific verification table green) and the `## Outcomes` section of the plan is filled in. The coder produces a **proposed commit message** (in the plan's "Commit message" section, or in the coder's final report) — the maintainer reviews the Outcomes and runs `git commit` themselves. This applies even when the coder has shell/git access. The coder may stage files via `git add` if useful for verification (e.g. checking `git status` to confirm the right files are touched), but `git commit` is the maintainer's action. This preserves the human-in-the-loop checkpoint between code production and code landing.

### Deferred work surfaces decisions early when they affect the deferral point

When a step explicitly defers work to a later named step (e.g. "implement real X in Step N+k"), any decision belonging to that future step which affects code or design choices in the current step — or in any intervening step — is surfaced *at the deferral point*, not at the deferred step's dispatch round. The current step's decisions doc lists the affected future-step decisions in a dedicated section (or surfaces them inline if they need maintainer answers before the current step's planner round proceeds).

Surfacing the full future-step decision set early is over-eager — only decisions with cross-step impact get pulled forward; everything else waits for the deferred step's normal dispatch round.

Prior applications: 1.1b Decision 2 (arrival_time deterministic source surfaced for Phase 3); 1.2 Decision 8 (Step 1.3.5 coverage tooling slot reserved); 1.3a Decision 6 (Step 1.5 MPT root computation slot reserved).

### Sessions
```

Rationale: Decision 11 — workflow principle formalized after three applications.

#### Step 9 — Edit `AGENTS.md`: full-body replacement of "Current State" section

**File:** `AGENTS.md`

**Old (the entire `## Current State` block, from the section heading through the last bullet immediately before `## Changelog`):**
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
```

**New (the new `## Current State` body — everything from the section heading through the last bullet before `## Changelog`):**
```markdown
## Current State

> Rewritten by the agent at the end of every session.
> Keep it tight — the next agent reads this and knows exactly what to do.

**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.3a complete; Step 1.3b next).

**What was just completed (Step 1.3a — In-Memory MptState):**
`crates/krax-state/src/mpt/mod.rs` created: `MptState` (single `BTreeMap<B256, B256>`,
Decision 3) with `pub fn new() -> Self` (Open Question 3); `MptSnapshot` (owned clone of
the slot map, Decision 4). `impl State for MptState`: `get`/`set` read/write the map;
`snapshot` clones into a `Box<MptSnapshot>`; `commit` is a no-op returning
`Ok(self.root())` (Decision 5); `root` returns `B256::ZERO` with a `// TODO Step 1.5`
marker (Decision 6). `impl Snapshot for MptSnapshot`: `get` reads from the owned clone;
`release(self: Box<Self>)` is a no-op (consuming `Box` is dropped on return).
`#[cfg(test)] mod tests` co-located in `mpt/mod.rs` (Decision 8) with an inline
`fn slot(n: u8) -> B256` helper (Decision 9 sub-question (i)) and one round-trip test
`set_then_get_round_trips`. The three `Journal::apply` tests rewritten against
`MptState` land in the second commit of this step (per Decision 11C).
`crates/krax-state/src/lib.rs` rewritten: `pub mod mpt;` + `pub use mpt::{MptSnapshot,
MptState};` (Decision 1).
`crates/krax-state/Cargo.toml` updated: `krax-types = { path = "../krax-types" }` added
as the only runtime dep; `rstest` and `pretty_assertions` added to `[dev-dependencies]`
(Decision 2). [If the verification fork fired: `alloy-primitives = { workspace = true }`
also added to runtime deps — see Outcomes Deviation.]
`AGENTS.md` "Workflow & Conventions" extended with a new subsection encoding the
"deferred work surfaces decisions early when they affect the deferral point" principle
(Decision 11 + cross-step section of step-1.3a-decisions.md).
`ARCHITECTURE.md` Step 1.3 checkbox 1 split into 1.3a/1.3b halves (Decision 10b);
1.3a's checkboxes closed (split-1a, checkbox 2, checkbox 4); Step 1.5 — MPT Root
Computation inserted between Step 1.4 and Phase 1 Gate; Phase 1 Gate updated with a
"Real MPT root computation in place" line item.

**What Step 1.2b delivered (test commit, shipped 2026-05-11):**
`crates/krax-types/src/test_helpers.rs` (`pub(crate) fn slot`, `pub(crate) fn concrete`);
`rwset.rs` and `journal.rs` `#[cfg(test)] mod tests` blocks with `rstest` truth tables and
`StubState`-backed `Journal::apply` tests; `Journal::discard` `compile_fail` doctest;
`Cargo.toml` dev deps; AGENTS.md Rule 5 amendment.

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
- `MptState::root()` returns `B256::ZERO` with a `// TODO Step 1.5` marker — real
  Ethereum-compatible MPT root computation lands in Step 1.5 (slot reserved in
  ARCHITECTURE.md between Step 1.4 and Phase 1 Gate; the alloy-trie vs custom-MPT
  decision is pre-surfaced in step-1.3a-decisions.md but answered at 1.5 dispatch).

**What to do next:**
1. 🔴 **Step 1.3b — MDBX-Backed MptState.** Replace the in-memory `BTreeMap` backing of
   `MptState` with `reth-db` MDBX-backed durability. Add I/O variants to
   `StateError`. Land the restart test (open DB, set, commit, close, reopen, get
   returns committed value). Follow ARCHITECTURE.md Step 1.3 — the remaining two
   unchecked items are 1.3b's scope.

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
- `StubState` (formerly in `crates/krax-types/src/journal.rs`'s `#[cfg(test)] mod tests`) was
  deleted in Step 1.3a. The three `Journal::apply` tests now live in
  `crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests` and exercise `MptState`
  directly (Decision 8). The empty `#[cfg(test)] mod tests` block in `journal.rs` was removed
  entirely (Open Question 2). The `compile_fail` doctest on `Journal::discard` is unaffected.
```

Rationale: Decision 10 sub-question + Decision 11 — full-body Current State rewrite mirrors 1.2b precedent. Records 1.3a delivery; advances "What to do next" to 1.3b; replaces the `StubState`-scaffolding note with a 1.3a-completed note; adds the `MptState::root()` placeholder to "Known scaffolding placeholders."

#### Step 10 — Edit `AGENTS.md`: append Session N entry to the BOTTOM of the Changelog

**File:** `AGENTS.md`

**Insertion point:** After the final line of the existing Session 13 entry (`**Commit suggestion:** \`test(types): add rstest + pretty_assertions, test_helpers, and Step 1.2 test modules\``), add one blank line, then append the verbatim block below. This entry covers BOTH 1.3a commits in one session (the coder ships both consecutively); if the coder splits over two sessions, replace this with two Session entries — same content split.

**Content to append:**
```markdown

### Session 14 — Step 1.3a (both commits): In-Memory MptState + Journal::apply Test Migration
**Date:** 2026-05-XX
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary (Commit 1 — feat(state): implement in-memory MptState — Step 1.3a):** Created
`crates/krax-state/src/mpt/mod.rs` with `MptState` (single `BTreeMap<B256, B256>`,
Decision 3), `MptSnapshot` (owned-clone, Decision 4), `impl State` (`commit` no-op
returning `Ok(self.root())` per Decision 5; `root` returns `B256::ZERO` with
`// TODO Step 1.5` per Decision 6), `impl Snapshot` (consuming `release` no-op). Inline
`fn slot` helper in `#[cfg(test)] mod tests` (Decision 9 (i)); one round-trip test
`set_then_get_round_trips`. Rewrote `crates/krax-state/src/lib.rs` as `pub mod mpt; pub
use mpt::{MptSnapshot, MptState};` (Decision 1). Added `krax-types` runtime dep + `rstest`
and `pretty_assertions` dev deps to `crates/krax-state/Cargo.toml` (Decision 2). [If
verification fork fired: also added `alloy-primitives = { workspace = true }` runtime
dep — surfaced as Deviation.] AGENTS.md Workflow & Conventions extended with
"Deferred work surfaces decisions early when they affect the deferral point" subsection.
AGENTS.md Current State rewritten for 1.3a completion. ARCHITECTURE.md Step 1.3 checkbox 1
split (Decision 10b); 1.3a's checkboxes (split-1a, 2, 4) closed; Step 1.5 — MPT Root
Computation inserted between Step 1.4 and Phase 1 Gate; Phase 1 Gate updated with a
"Real MPT root computation in place" line item.
**Summary (Commit 2 — refactor(types): rewrite Journal::apply tests against MptState
— Step 1.3a):** Deleted the entire `#[cfg(test)] mod tests` block from
`crates/krax-types/src/journal.rs` (including `StubState` impl and the three apply
tests, per Open Question 2). Appended three `Journal::apply` tests to
`crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests` (Decision 8):
`apply_empty_journal_leaves_state_unchanged`, `apply_single_entry_writes_slot`,
`apply_last_write_wins_on_same_slot`. Tests assert via `state.get(slot(n))` only —
no `commit()` call, no direct map inspection (Decision 9 (a)). The `compile_fail`
doctest on `Journal::discard` is unaffected (Open Question 1).
**Commit suggestion (Commit 1):** `feat(state): implement in-memory MptState — Step 1.3a`
**Commit suggestion (Commit 2):** `refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a`
```

Rationale: Decision 11C two commits; one compound Session 14 entry mirroring Session 13's two-commit shape. Coder fills in the date when committing.

### Verification suite (Commit 1)

| # | Check | Command | Expected |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | krax-state compiles | `cargo check -p krax-state` | exit 0 |
| 3 | All tests pass | `make test` | exit 0; `set_then_get_round_trips` appears in output |
| 4 | krax-types doctest still passes | `cargo test --doc -p krax-types` | exit 0 (the `Journal::discard` `compile_fail` doctest still triggers correctly) |
| 5 | Lint clean | `make lint` | exit 0 (no `missing_docs`, no `clippy::unwrap_used`, no pedantic firing on production code) |
| 6 | Docs build | `cargo doc --workspace --no-deps` | exit 0 |
| 7 | `MptState` is public | `grep -n 'pub struct MptState' crates/krax-state/src/mpt/mod.rs` | match found |
| 8 | `MptState::new` exists | `grep -n 'pub fn new' crates/krax-state/src/mpt/mod.rs` | match found |
| 9 | TODO marker present | `grep -n 'TODO Step 1.5' crates/krax-state/src/mpt/mod.rs` | match found |
| 10 | Step 1.5 inserted | `grep -n '### Step 1.5 — MPT Root Computation' ARCHITECTURE.md` | match found |
| 11 | Phase 1 Gate updated | `grep -n 'Real MPT root computation' ARCHITECTURE.md` | match found |
| 12 | Workflow principle landed | `grep -n 'Deferred work surfaces decisions early' AGENTS.md` | match found |
| 13 | Session 14 at bottom of Changelog | `tail -50 AGENTS.md \| grep -n '### Session 14'` | match found (entry is last) |

### Commit message (Commit 1)

```
feat(state): implement in-memory MptState — Step 1.3a
```

### Outcomes (Commit 1)

- Files changed:
  - `crates/krax-state/Cargo.toml` (added `krax-types` path dep, `alloy-primitives` workspace dep, `rstest` + `pretty_assertions` dev deps)
  - `crates/krax-state/src/lib.rs` (rewritten with `pub mod mpt;` + flat re-exports)
  - `crates/krax-state/src/mpt/mod.rs` (new — `MptState`, `MptSnapshot`, `impl State`, `impl Snapshot`, round-trip test)
  - `ARCHITECTURE.md` (Step 1.3 checkbox split + 1.3a closures; Step 1.5 inserted; Phase 1 Gate updated)
  - `AGENTS.md` (workflow principle appended; Current State rewritten; Session 14 appended to bottom of Changelog)
  - `docs/plans/step-1.3a-plan.md` (Outcomes filled in)

- Verification table results:

| # | Check | Exit | Notes |
|---|---|---|---|
| 1 | `make build` | 0 | release profile clean |
| 2 | `cargo check -p krax-state` | 0 | (after fork — see Deviations) |
| 3 | `make test` | 0 | `test mpt::tests::set_then_get_round_trips ... ok` |
| 4 | `cargo test --doc -p krax-types` | 0 | `Journal::discard` compile_fail doctest passes |
| 5 | `make lint` | 0 | no missing_docs / unwrap_used / pedantic firings |
| 6 | `cargo doc --workspace --no-deps` | 0 | krax-state docs generated |
| 7 | `grep 'pub struct MptState' mpt/mod.rs` | match | line 22 |
| 8 | `grep 'pub fn new' mpt/mod.rs` | match | line 32 |
| 9 | `grep 'TODO Step 1.5' mpt/mod.rs` | match | line 67 |
| 10 | `grep '### Step 1.5 — MPT Root Computation' ARCHITECTURE.md` | match | inserted between Step 1.4 and Phase 1 Gate |
| 11 | `grep 'Real MPT root computation' ARCHITECTURE.md` | match | Phase 1 Gate item added |
| 12 | `grep 'Deferred work surfaces decisions early' AGENTS.md` | match | subsection heading + Session 14 reference |
| 13 | `tail -50 AGENTS.md \| grep '### Session 14'` | match | entry is last in Changelog |

- Deviations from plan:
  1. **Step ordering correction** (per coder-dispatch directive): executed as Step 1 → Step 3 → Step 4 → Step 2 (verification fork) → Step 5 onward, since the `use alloy_primitives::B256;` import lives in Step 4's new file.
  2. **`cargo check` fork FIRED.** After Step 4 landed the `mpt/mod.rs` file, `cargo check -p krax-state` failed with `error[E0432]: unresolved import alloy_primitives`. Per Decision 2's planner-flagged concern, `alloy-primitives = { workspace = true }` was added to `crates/krax-state/Cargo.toml`'s `[dependencies]`. Subsequent `cargo check` passes. krax-types does not re-export `alloy_primitives::B256` — the dep is required.
  3. **Conditional-placeholder cleanup** (per coder-dispatch directive): replaced the `[If the verification fork fired: ...]` bracketed placeholder in AGENTS.md Current State and Session 14 entry with the un-bracketed equivalent statement, since the fork fired.
- Proposed commit message (final): `feat(state): implement in-memory MptState — Step 1.3a`
- Notes for the maintainer:
  - No rustfmt-driven or clippy-driven cosmetic changes were needed; code shipped as-written per the plan's verbatim content.
  - The `make test` doctest output reaffirms that the `Journal::discard` compile_fail doctest from Step 1.2b is regression-protected and still triggers correctly (`compile fail ... ok`).

---

## Commit 2: refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a

### Purpose

Delete the `#[cfg(test)] mod tests` block from `crates/krax-types/src/journal.rs` (the `StubState` scaffold + the three apply tests) and append the three rewritten tests against `MptState` to `crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests` block. No production-code changes. No ARCHITECTURE.md or AGENTS.md edits (those all landed in Commit 1).

### Execution Steps

#### Step 1 — Delete the `#[cfg(test)] mod tests` block from `crates/krax-types/src/journal.rs`

**File:** `crates/krax-types/src/journal.rs`

**Old (delete this entire block; ensure one trailing blank line at end of file is preserved if it was there before):**
```rust
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

**New:** *(nothing — the whole block is removed; the file ends after the `impl Journal` block's final `}` and the `Journal::discard` `compile_fail` doctest remains untouched in the doc-comment.)*

Rationale: Open Question 2 — remove the whole block, not just the body. The `test_helpers::slot` import disappears with the block; `crates/krax-types/src/lib.rs`'s `#[cfg(test)] mod test_helpers;` declaration remains because `rwset.rs`'s test module still uses it.

#### Step 2 — Append three `Journal::apply` tests to `crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests`

**File:** `crates/krax-state/src/mpt/mod.rs`

**Old (target the closing `}` of the `mod tests` block created in Commit 1):**
```rust
    #[test]
    fn set_then_get_round_trips() {
        let mut state = MptState::new();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }
}
```

**New:**
```rust
    #[test]
    fn set_then_get_round_trips() {
        let mut state = MptState::new();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_empty_journal_leaves_state_unchanged() {
        use krax_types::{Journal, State};

        let mut state = MptState::new();
        let journal = Journal { entries: vec![] };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), B256::ZERO);
    }

    #[test]
    fn apply_single_entry_writes_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let mut state = MptState::new();
        let journal = Journal {
            entries: vec![JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(42) }],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_last_write_wins_on_same_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let mut state = MptState::new();
        let journal = Journal {
            entries: vec![
                JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(10) },
                JournalEntry { slot: slot(1), old: slot(10), new: slot(20) },
            ],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(20));
    }
}
```

Rationale: Decision 8 + Decision 9 — three tests co-located in `mpt/mod.rs`'s test module; assertions via `state.get(slot(n))` only (no `commit` call needed since writes are immediately visible per Decision 5). Per-test `use krax_types::{...}` imports keep `Journal`/`JournalEntry` scoped to the tests that need them; `super::*` (already imported at the top of the test module) handles `MptState`, and the inline `slot` + `alloy_primitives::B256` are also already in scope.

> **Note for the coder:** if rustfmt or clippy prefers module-level imports for `Journal`/`JournalEntry` (e.g. `unused_imports` not firing under the per-test pattern is the expected behavior, but if a pedantic lint complains about `use` statements inside test functions, hoist them to the module-level imports near `super::*` and `pretty_assertions::assert_eq`). Either shape is acceptable as long as `make lint` is clean.

### Verification suite (Commit 2)

| # | Check | Command | Expected |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | All tests pass | `make test` | exit 0; four tests in `mpt::tests` (round-trip + three apply tests) |
| 3 | Journal doctest still passes | `cargo test --doc -p krax-types` | exit 0 |
| 4 | Lint clean | `make lint` | exit 0 |
| 5 | `mod tests` gone from journal.rs | `grep -c 'mod tests' crates/krax-types/src/journal.rs` | 0 |
| 6 | `StubState` gone from journal.rs | `grep -c 'StubState' crates/krax-types/src/journal.rs` | 0 |
| 7 | Apply tests in mpt/mod.rs | `grep -c 'apply_empty_journal_leaves_state_unchanged' crates/krax-state/src/mpt/mod.rs` | 1 |
| 8 | Last-write-wins test in mpt/mod.rs | `grep -c 'apply_last_write_wins_on_same_slot' crates/krax-state/src/mpt/mod.rs` | 1 |

### Commit message (Commit 2)

```
refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a
```

### Outcomes (Commit 2)

- Files changed:
  - `crates/krax-types/src/journal.rs` (deleted entire `#[cfg(test)] mod tests` block — StubState impl + 3 apply tests + imports + `#[allow]`)
  - `crates/krax-state/src/mpt/mod.rs` (appended 3 apply tests to existing `mod tests` block; per-test `use krax_types::{...}` imports per plan)

- Verification table results:

| # | Check | Exit | Notes |
|---|---|---|---|
| 1 | `make build` | 0 | release profile clean |
| 2 | `make test` | 0 | 4 tests in `mpt::tests`: `set_then_get_round_trips`, `apply_empty_journal_leaves_state_unchanged`, `apply_single_entry_writes_slot`, `apply_last_write_wins_on_same_slot` |
| 3 | `cargo test --doc -p krax-types` | 0 | `Journal::discard` compile_fail doctest still passes |
| 4 | `make lint` | 0 | no firings; per-test `use` imports accepted (no hoist needed) |
| 5 | `grep -c 'mod tests' crates/krax-types/src/journal.rs` | 0 | block fully removed |
| 6 | `grep -c 'StubState' crates/krax-types/src/journal.rs` | 0 | no residual references |
| 7 | `grep -c 'apply_empty_journal_leaves_state_unchanged' crates/krax-state/src/mpt/mod.rs` | 1 | |
| 8 | `grep -c 'apply_last_write_wins_on_same_slot' crates/krax-state/src/mpt/mod.rs` | 1 | |

- Deviations from plan: none. Per-test `use krax_types::{...}` imports landed as written; rustfmt/clippy did not request an import hoist. No rustfmt-driven reformat.
- Proposed commit message (final): `refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a`
- Notes for the maintainer:
  - The `crates/krax-types/src/test_helpers.rs` module remains in place (still used by `rwset.rs`'s test module — not touched by this commit).
  - `journal.rs` no longer references `crate::state` or `crate::snapshot` at runtime (the test-module imports went with the deletion); only `crate::state::{State, StateError}` and `alloy_primitives::B256` remain as production imports.

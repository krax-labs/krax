# Step 1.3b Plan — MDBX-Backed MptState + Restart Tests (2 commits)

Date: 2026-05-12
Status: ⏳ Ready for coder execution
Decisions: docs/plans/step-1.3b-decisions.md (✅ Answered 2026-05-12)
Companion: 1.3b closes Step 1.3 (Step 1.3a shipped in-memory backend in Session 14).

## Critical: Do not run git commit

Do not run `git commit`. Stage files via `git add` if useful for verification; commit is the maintainer's action. Report your proposed commit message at the end of each commit's execution. The maintainer reviews the Outcomes section and runs the commit. (AGENTS.md "Coding agents do NOT run `git commit`" rule.)

## Pre-flight

### Library Verification Protocol

Run each Context7 query below against the reth-db crate (pinned rev
`02d1776786abc61721ae8876898ad19a702e0070`, dated 2026-05-06) BEFORE
writing any code that calls the answered surface. Cite the Context7 answer
inline above the call site per AGENTS.md. If Context7 contradicts the
planner's expected surface, surface the discrepancy in Outcomes — do NOT
silently adapt. Queries 1, 2, 3, 5, 6, 7 are mandatory before Commit 1
begins. Query 4 is informational. Query 8 is conditional on a compile error
during Commit 1.

Pre-declared planner expectations + fallbacks (from
docs/plans/step-1.3b-decisions.md "Library Verification checklist"):

- **Query 1 — env open.** Expected: `reth_db::open_db_read_write(path,
  args)` (or `DatabaseEnv::open(path, kind, args)`) returning
  `Result<DatabaseEnv, _>`. Fallback: pinned-rev source under
  `crates/storage/db/src/`.
- **Query 2 — RoTxn / RwTxn.** Expected: `env.tx()` and `env.tx_mut()`
  returning `Result<_, _>`; `RwTxn::commit()` is fallible.
  `<DatabaseEnv as Database>::TX` and `::TXMut` are the named tx types.
- **Query 3 — table macro.** Expected: `reth_db::tables! { table Slots
  { type Key = B256; type Value = B256; } }` declarative macro defining
  `Table` / `Compress` / `Decompress` impls. Fallback if renamed/removed
  in Reth 2.0: hand-roll `Table` impl per Decision 7's noted fallback;
  surface in Outcomes.
- **Query 4 — existing tables (informational).** Confirm reth's
  `PlainStorageState` (or successor) keys by `(Address, StorageKey)` —
  incompatible with our flat `B256 → B256` shape, so we define our own.
- **Query 5 — error type.** Expected: `reth_db::DatabaseError` is an enum,
  `Send + Sync + 'static`, implements `std::error::Error`, integrates
  with `thiserror` via `#[from]`. If the type name differs in the pinned
  rev, substitute (e.g. `DatabaseError` may be `DbError` or live under a
  submodule) and note in Outcomes.
- **Query 6 — B256 codec.** Expected: reth-db's `Compress`/`Decompress`
  encode `alloy_primitives::B256` as 32 raw bytes; no padding. Cite the
  exact trait-impl path in the inline comment above the table definition.
- **Query 7 — bundled test helpers.** Expected: reth-db does NOT export a
  `test_utils::create_test_rw_env` we can rely on (per Decision 8 (c)
  rejection). We use `tempfile::TempDir` directly.
- **Query 8 (conditional, Decision 11 sub-question) — RoTxn abort
  fallibility.** Only run if release-path compile fails or if the coder
  needs the surface explicitly. Expected: drop-equivalent (infallible).
  If fallible, the answer is silent-drop (Decision 11 sub-question
  answer); do NOT change the Snapshot trait.

---

## Commit 1: feat(state): wire MDBX backend for MptState — Step 1.3b

### Purpose

Rewrite `crates/krax-state/src/mpt/mod.rs` to back `MptState` with an MDBX
environment via `reth-db` (`Slots: B256 → B256` flat table, Decision 6).
Replace `MptState::new()` with `open(path)` + `open_temporary()` (Decisions
1, 2). Rewrite `impl State` so `set` auto-flushes per call (Decision 4),
`snapshot()` returns a `RoTxn`-backed `MptSnapshot` (Decision 3), and
`commit()` is a sync-barrier returning `Ok(self.root())`. Add
`StateError::Io(#[from] reth_db::DatabaseError)` (Decision 5). Rewrite the
four 1.3a inline tests to use `MptState::open_temporary()` (per Decision 1
knock-on). Update workspace `Cargo.toml` (add `tempfile`), update
`crates/krax-state/Cargo.toml` (add `reth-db` runtime + `tempfile` dev),
extend `StateError`. Edit `ARCHITECTURE.md` (close Step 1.3 heading and
both remaining checkboxes — see micro-decision below). Edit `AGENTS.md`
(Rule 10 `tempfile` append; Current State full-body replacement; Session
15 appended at the BOTTOM of the Changelog). No `git commit`.

### Coder micro-decision — ARCHITECTURE.md checkbox closure timing

Planner-flagged judgment call (NOT a maintainer decision — coder applies
the recommendation):

**Recommendation:** in Commit 1, close (a) the Step 1.3 heading `✅`,
(b) the `Wire MDBX as the durable backend (Step 1.3b)` checkbox, and
(c) leave the `Restart test: ... (Step 1.3b)` checkbox **unchecked** in
Commit 1. Close (c) in Commit 2 alongside the test file that
proves it. Rationale: the gate line "passes round-trip and restart tests"
isn't literally true until Commit 2 lands the restart test. The Step 1.3
heading can still take `✅` in Commit 1 because Commit 1's deliverable IS
the wiring; the test in Commit 2 is verification of that wiring. **The
Phase 1 Gate items (lines 161-165) all currently display `✅` as
typographical markers (not literal completion status — see
"Phase 1 Gate convention" note below); no edit to those lines is needed
in either commit.**

Counter-reading the coder may apply if they prefer atomic shape: close
ALL Step 1.3 checkboxes in Commit 1 (since the code that the test
exercises is all present after Commit 1). Either reading is acceptable as
long as it's consistent across both commits.

### Phase 1 Gate convention note

The current ARCHITECTURE.md (lines 161-165) shows all five Phase 1 Gate
items with `✅` prefix already, including items whose backing work has not
been done (Snapshot isolation = Step 1.4; Real MPT root = Step 1.5;
Coverage = Step 1.3.5). The 1.3a planner stamped them ✅ as goal-state
markers, not literal completion status. Decision 13's answer ("the
round-trip+restart gate line gets `✅`") is therefore already satisfied
typographically. No edit to Phase 1 Gate lines is required in 1.3b.
Surface as a Notes entry in Outcomes for the maintainer (the convention
may want unification at Step 1.5 close).

### Execution Steps

#### Step 1 — Edit workspace `Cargo.toml`: add `tempfile` to test-only group

**File:** `Cargo.toml` (workspace root)

**Old:**
```toml
# --- Test-only ---
# ⚠️ ESTIMATED: proptest 1.x and pretty_assertions 1.x are stable.
# ✅ cargo search rstest (2026-05-06): 0.26.1
proptest          = "1"
rstest            = "0.26"
pretty_assertions = "1"
```

**New:**
```toml
# --- Test-only ---
# ⚠️ ESTIMATED: proptest 1.x, pretty_assertions 1.x, tempfile 3.x are stable.
# ✅ cargo search rstest (2026-05-06): 0.26.1
proptest          = "1"
rstest            = "0.26"
pretty_assertions = "1"
tempfile          = "3"
```

Rationale: Decision 8 — `tempfile` dep added to workspace test-only group.
`reth-db` is already in `[workspace.dependencies]` (line 61) from Step 0.1
— no edit there.

Coder action: confirm `cargo search tempfile` returns a stable 3.x release
(planner expects 3.10+). If the latest stable version is higher (e.g.
`3.15`), pin to `"3"` as written — the `^3` semver range accepts 3.x
patches/minors.

#### Step 2 — Edit `crates/krax-state/Cargo.toml`: add reth-db runtime + tempfile dev

**File:** `crates/krax-state/Cargo.toml`

**Old:**
```toml
[dependencies]
# MptState implements State and Snapshot from krax-types.
krax-types        = { path = "../krax-types" }
# B256 is used directly in mpt/mod.rs; krax-types does not re-export it.
alloy-primitives  = { workspace = true }

[dev-dependencies]
rstest            = { workspace = true }
pretty_assertions = { workspace = true }
```

**New:**
```toml
[dependencies]
# MptState implements State and Snapshot from krax-types.
krax-types        = { path = "../krax-types" }
# B256 is used directly in mpt/mod.rs; krax-types does not re-export it.
alloy-primitives  = { workspace = true }
# MDBX env + table + transaction surface for the durable MptState backend (Step 1.3b).
reth-db           = { workspace = true }

[dev-dependencies]
rstest            = { workspace = true }
pretty_assertions = { workspace = true }
# tempfile-backed test fixtures: MptState::open_temporary + restart tests (Step 1.3b).
tempfile          = { workspace = true }
```

Rationale: Decision 5 (`StateError::Io` needs the reth-db error type),
Decision 6/7 (MDBX env + Slots table), Decision 8 (tempfile).

#### Step 3 — Edit `crates/krax-types/src/state.rs`: add boxed-source `Io` variant to `StateError`

**File:** `crates/krax-types/src/state.rs`

**Old:**
```rust
//! State trait and its associated error type for the Krax state backend.

use alloy_primitives::B256;
use thiserror::Error;

use crate::snapshot::Snapshot;

/// Errors returned by [`State`] and [`Snapshot`] operations.
///
/// Starts with a single variant. `#[non_exhaustive]` ensures downstream match
/// arms won't break when I/O variants are added in Step 1.3.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum StateError {
    /// The snapshot was already released and can no longer be read.
    #[error("snapshot has been released")]
    Released,
}
```

**New:**
```rust
//! State trait and its associated error type for the Krax state backend.

use alloy_primitives::B256;
use thiserror::Error;

use crate::snapshot::Snapshot;

/// Errors returned by [`State`] and [`Snapshot`] operations.
///
/// `#[non_exhaustive]` ensures downstream match arms won't break when
/// additional variants are added.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum StateError {
    /// The snapshot was already released and can no longer be read.
    #[error("snapshot has been released")]
    Released,
    /// Underlying storage I/O failure.
    ///
    /// Source is boxed (`Box<dyn std::error::Error + Send + Sync>`) so
    /// `krax-types` does not depend on any specific storage backend. V1's
    /// MDBX backend (Step 1.3b) wraps `reth_db::DatabaseError` here; V2's
    /// LSM backend will wrap its own error type without touching this enum.
    /// Trade-off: callers cannot statically downcast to a backend-specific
    /// error type (Decision 5 maintainer revision).
    #[error("state I/O error: {0}")]
    Io(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl StateError {
    /// Constructs an [`StateError::Io`] from any `Send + Sync + 'static` error.
    ///
    /// Use this at storage-backend call sites: e.g.
    /// `reth_db_call().map_err(StateError::io)?`.
    pub fn io<E>(source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Io(Box::new(source))
    }
}
```

Rationale: Decision 5 (maintainer revision) — boxed-source variant keeps
`krax-types` free of any storage-backend dependency. The `StateError::io`
constructor is the canonical wrapper at backend call sites — every
reth-db `?` in `krax-state` becomes `.map_err(StateError::io)?` (or
equivalent). Loses static type info for the wrapped error; gains a clean
V2 substitution story (Rule 8 drop-in replacement).

**Lint note:** `#[source]` on the `Box<...>` field is needed (not `#[from]`)
because `From<E>` blanket impls would conflict with the generic `io`
constructor. Call sites use the constructor explicitly; `?` does NOT
automatically convert reth-db errors into `StateError::Io` — the
`.map_err` is intentional.

#### Step 4 — Verification gate before rewriting mpt/mod.rs

After Steps 1–3, run:

```
cargo check -p krax-types
cargo check -p krax-state
```

`krax-types` must compile cleanly with the new `Io` variant and the
`io()` constructor — no new dependencies in `krax-types/Cargo.toml`
(Decision 5 maintainer revision keeps `krax-types` pristine; reth-db
stays out of its dep graph).

`krax-state` will fail to compile because `mpt/mod.rs` still references
the old `BTreeMap` shape — that's expected; Step 6 rewrites the file.

**Coder action — LVP gate:** Queries 1, 2, 3, 5, 6, 7 must be completed
BEFORE Step 6. Cite each Context7 answer inline in the comment block at
the top of the rewritten `mpt/mod.rs`. Confirmed surfaces become the
basis for the code block in Step 6.

#### Step 6 — Rewrite `crates/krax-state/src/mpt/mod.rs`

**File:** `crates/krax-state/src/mpt/mod.rs`

**Old:** the entire current 143-line file (in-memory MptState +
MptSnapshot + 4 inline tests). Replace wholesale.

**New (planner expected shape — coder substitutes exact reth-db symbol
names after LVP queries 1, 2, 3, 5, 6 confirm):**
```rust
//! MDBX-backed `MptState` — Step 1.3b backend.
//!
//! Replaces the Step 1.3a in-memory `BTreeMap` with a `reth-db`-backed
//! MDBX environment storing a single flat `Slots: B256 → B256` table
//! (Decision 6). `set` auto-flushes per call via a short-lived RwTxn
//! (Decision 4); `snapshot` opens an RoTxn-backed [`MptSnapshot`]
//! (Decision 3); `commit` is a sync barrier returning the (placeholder)
//! root (Decisions 4 + 6 + Step 1.5 deferral). Real MPT root computation
//! lands in Step 1.5.
//!
//! Decisions: docs/plans/step-1.3b-decisions.md.

use std::path::Path;
use std::sync::Arc;

use alloy_primitives::B256;
use krax_types::{Snapshot, State, StateError};
// LVP Query 1/2: exact import paths confirmed post-Context7. Planner expectation:
//   reth_db::{create_db_and_tables, DatabaseEnv, mdbx::DatabaseArguments,
//             transaction::{DbTx, DbTxMut}, tables};
// If the public API differs (e.g. open_db_read_write vs create_db_and_tables),
// substitute and cite Context7 inline.
use reth_db::{
    tables,                                       // LVP Query 3
    transaction::{DbTx, DbTxMut},                 // LVP Query 2
    DatabaseEnv,                                  // LVP Query 1
};

// Per Context7 (reth-db, May 2026, LVP Query 3): the `tables!` declarative
// macro defines `Table` + `Compress` + `Decompress` impls for the given
// (Key, Value) pair against MDBX. `B256` encoding confirmed by LVP Query 6.
tables! {
    /// Flat slot table backing `MptState` (Decision 6).
    table Slots {
        type Key = B256;
        type Value = B256;
    }
}

/// MDBX-backed implementation of the [`State`] trait.
///
/// Owns a refcounted handle to the underlying MDBX environment (`Arc<DatabaseEnv>`).
/// Cloning the `Arc` is cheap and lets [`MptSnapshot`] hold its own reference
/// for the lifetime of the read transaction — required because
/// [`State::snapshot`] returns a `Box<dyn Snapshot>` with no borrow back to
/// `self` (Decision 3).
#[derive(Debug)]
pub struct MptState {
    env: Arc<DatabaseEnv>,
}

impl MptState {
    /// Opens (or creates) the MDBX environment rooted at `path`.
    ///
    /// Returns [`StateError::Io`] if the environment cannot be opened or
    /// the `Slots` table cannot be initialized.
    pub fn open(path: &Path) -> Result<Self, StateError> {
        // LVP Query 1: confirm exact env-open call. Planner expectation:
        //   reth_db::create_db_and_tables::<&[u8]>(path, DatabaseArguments::default())
        //     -> Result<DatabaseEnv, DatabaseError>
        // The function (or successor) both opens the env and creates any
        // declared tables that do not yet exist. Substitute correct name
        // post-Context7.
        let env = reth_db::create_db_and_tables(path).map_err(StateError::io)?;
        Ok(Self { env: Arc::new(env) })
    }

    /// Opens an `MptState` rooted at a fresh `TempDir`.
    ///
    /// Returns the `TempDir` alongside the state so the caller controls drop
    /// ordering — the directory is removed when the `TempDir` is dropped.
    /// Test-only helper (Decision 1). Available in the test build and under
    /// the `integration` feature so external integration tests can use it
    /// without duplicating the fixture.
    #[cfg(any(test, feature = "integration"))]
    pub fn open_temporary() -> Result<(Self, tempfile::TempDir), StateError> {
        // `TempDir::new()` returns std::io::Error which is not in StateError's
        // From-chain. Map it: an in-test tempdir failure is fatal for the test
        // and panicking via expect is acceptable in a #[cfg(any(test, ...))]
        // helper (the workspace #[allow(clippy::unwrap_used)] lives at the
        // test-module level for inline tests; integration test files lift it
        // at their own module level). For the production-feature-gated path,
        // we still return Result for trait symmetry — wrap io::Error as a
        // panic-on-fixture-failure via expect since no caller will catch it.
        let dir = tempfile::TempDir::new()
            .expect("MptState::open_temporary: tempdir creation failed");
        let state = Self::open(dir.path())?;
        Ok((state, dir))
    }
}

impl State for MptState {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        // LVP Query 2: short-lived RoTxn per Decision 4. Planner expectation:
        //   let tx = self.env.tx()?;
        //   let v = tx.get::<Slots>(slot)?.unwrap_or(B256::ZERO);
        //   tx.commit()?;   // or drop — for RO, either is fine; commit is the
        //                   // canonical reth-db verb (Database::TX::commit).
        let tx = self.env.tx().map_err(StateError::io)?;
        let v = tx.get::<Slots>(slot).map_err(StateError::io)?.unwrap_or(B256::ZERO);
        tx.commit().map_err(StateError::io)?;
        Ok(v)
    }

    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> {
        // Decision 4 (b): auto-flush per set — open, write, commit a
        // short-lived RwTxn. Writes are durable + visible to subsequent
        // `get` calls without an intervening `State::commit`.
        let tx = self.env.tx_mut().map_err(StateError::io)?;
        tx.put::<Slots>(slot, val).map_err(StateError::io)?;
        tx.commit().map_err(StateError::io)?;
        Ok(())
    }

    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> {
        // Decision 3 (a): RoTxn-backed snapshot. Reads through the txn observe
        // a stable view; MDBX MVCC isolates from concurrent writes against the
        // same env.
        let tx = self.env.tx().map_err(StateError::io)?;
        Ok(Box::new(MptSnapshot { tx }))
    }

    fn commit(&mut self) -> Result<B256, StateError> {
        // Decision 4 (b): `set` already committed each individual write —
        // `commit` here is a sync-barrier semantic equivalent to 1.3a's no-op.
        // Returns the current (placeholder) root for caller bookkeeping.
        Ok(self.root())
    }

    fn root(&self) -> B256 {
        // TODO Step 1.5 — MPT Root Computation:
        // replace placeholder with real Ethereum-compatible MPT root
        // (alloy-trie vs custom MPT — decision pre-surfaced in
        // docs/plans/step-1.3a-decisions.md, answered at 1.5 dispatch).
        B256::ZERO
    }
}

/// MDBX read-only snapshot.
///
/// Owns a reth-db RoTxn (Decision 3); reads traverse the txn directly.
/// Drop releases the MDBX reader slot via the txn's `Drop` impl
/// (Decision 11).
pub struct MptSnapshot {
    // LVP Query 2: exact RoTxn type. Planner expectation:
    //   <DatabaseEnv as reth_db::Database>::TX
    // If the trait path differs, substitute. Whatever the type, it must be
    // `Send + Sync + 'static` — Snapshot supertraits demand it.
    tx: <DatabaseEnv as reth_db::Database>::TX,
}

impl Snapshot for MptSnapshot {
    fn get(&self, slot: B256) -> Result<B256, StateError> {
        Ok(self.tx.get::<Slots>(slot).map_err(StateError::io)?.unwrap_or(B256::ZERO))
    }

    fn release(self: Box<Self>) {
        // Decision 11 (a): drop releases the RoTxn via RAII — the `Box<Self>`
        // is dropped on return, `tx` drops, MDBX releases the reader slot.
        // No explicit `RoTxn::abort()` call (LVP Query 8 conditional).
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
        let (mut state, _tmp) = MptState::open_temporary().unwrap();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_empty_journal_leaves_state_unchanged() {
        use krax_types::{Journal, State};

        let (mut state, _tmp) = MptState::open_temporary().unwrap();
        let journal = Journal { entries: vec![] };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), B256::ZERO);
    }

    #[test]
    fn apply_single_entry_writes_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let (mut state, _tmp) = MptState::open_temporary().unwrap();
        let journal = Journal {
            entries: vec![JournalEntry { slot: slot(1), old: B256::ZERO, new: slot(42) }],
        };
        journal.apply(&mut state).unwrap();
        assert_eq!(state.get(slot(1)).unwrap(), slot(42));
    }

    #[test]
    fn apply_last_write_wins_on_same_slot() {
        use krax_types::{Journal, JournalEntry, State};

        let (mut state, _tmp) = MptState::open_temporary().unwrap();
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

Rationale: Decisions 1, 2, 3, 4, 5, 6, 7, 11. The four 1.3a inline tests
are rewritten in-place to `MptState::open_temporary()` (per Decision 1
knock-on) and bind the returned `TempDir` to `_tmp` so it drops at the end
of the test (the underscore suppresses unused-warning; binding keeps it
alive until scope exit).

**Coder-time substitution requirements:** after LVP queries return, replace
ALL `reth_db::...` paths in this file with Context7-confirmed exact paths.
The structure (Arc<DatabaseEnv>, per-method short-lived txns, RoTxn-owning
Snapshot) is load-bearing per the decisions and should NOT change without
surfacing as a Deviation. Compile errors during this substitution that
imply the structure itself is wrong (e.g. RoTxn cannot be 'static, requires
a lifetime parameter, must be wrapped in a different shape) MUST be
surfaced — they likely indicate Decision 3's "RoTxn-backed" answer
needs revisiting and the maintainer should be consulted before the coder
proceeds.

**Anticipated lint preempts:**

- `missing_docs` (workspace = warn, `make lint` = -D warnings): every
  `pub` item gets a `///` comment as written above. Confirm `Slots`
  (created by the macro) — if the macro doesn't propagate the doc
  attribute, the coder may need to add `#[allow(missing_docs)]` on the
  `tables!` invocation as a documented exception.
- `clippy::unwrap_used` is denied in production; `tempfile::TempDir::new`
  uses `expect(...)` in `open_temporary` — this is gated under
  `#[cfg(any(test, feature = "integration"))]` so it should not trip the
  production lint. If clippy still fires, hoist the `#[allow]` to that
  specific function with a documented reason.
- `must_use_candidate` is allow'd workspace-wide (root Cargo.toml line
  154) — no `#[must_use]` annotations needed on `open` / `open_temporary`.
- `pedantic` may suggest `Self::new` style for `open_temporary` returning
  a tuple — leave as written; the tuple-return shape is load-bearing.

#### Step 7 — Verification gate after `mpt/mod.rs` rewrite

After Step 6 lands, run:

```
cargo check -p krax-state
cargo build -p krax-state
make test
```

`make test` runs the four rewritten inline tests under
`MptState::open_temporary()`. All four must pass. If any of the Decision 3
self-referential-lifetime concerns surface (compile errors involving
`'static` on `MptSnapshot::tx`), STOP and report — DO NOT silently rewrite
to a clone-backed snapshot. The decisions doc's recommended remediation
path is to revisit Decision 3 with the maintainer, not to silently fall
back to clone-the-map.

#### Step 8 — Edit `ARCHITECTURE.md`: close Step 1.3 heading and MDBX checkbox

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
### Step 1.3 — MPT State Backend (Skeleton)
- [x] Create `MptState` struct in `crates/krax-state/src/mpt/mod.rs` and implement `State` trait against in-memory backing (Step 1.3a)
- [ ] Wire MDBX as the durable backend (Step 1.3b)
- [x] Implement `State` trait against an in-memory map first (Step 1.3a)
- [x] Round-trip test: `state.set(k, v); state.commit(); state.get(k) == v` (Step 1.3a)
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value (Step 1.3b)
```

**New (per Commit-1 micro-decision recommendation: close MDBX checkbox now,
defer restart-test checkbox to Commit 2; Step 1.3 heading ✅ in Commit 1):**
```markdown
### Step 1.3 — MPT State Backend (Skeleton) ✅
- [x] Create `MptState` struct in `crates/krax-state/src/mpt/mod.rs` and implement `State` trait against in-memory backing (Step 1.3a)
- [x] Wire MDBX as the durable backend (Step 1.3b)
- [x] Implement `State` trait against an in-memory map first (Step 1.3a)
- [x] Round-trip test: `state.set(k, v); state.commit(); state.get(k) == v` (Step 1.3a)
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value (Step 1.3b)
```

Rationale: Decision 13 (Step 1.3 heading ✅) + Commit-1 micro-decision
recommendation (close MDBX checkbox now; close restart-test checkbox in
Commit 2). If the coder adopts the atomic counter-reading, instead close
ALL Step 1.3 checkboxes here (use `- [x]` on the restart-test line) and
skip Commit 2's ARCHITECTURE.md edit (Step 12 below); note in Outcomes.

#### Step 9 — Phase 1 Gate convention check (NOT an edit)

**Coder action:** verify that ARCHITECTURE.md lines 161-165 (`**Phase 1
Gate:**` block) still display ALL five items with `✅` prefix
(unchanged from 1.3a). Per the Phase 1 Gate convention note above,
Decision 13's "gate line gets ✅" is satisfied typographically already.
NO EDIT to those lines in this commit.

If lines 161-165 do NOT all show ✅ (e.g. the maintainer flipped some to
`- [ ]` between 1.3a and 1.3b), STOP and surface — Decision 13's answer
needs reconciliation.

#### Step 10 — Edit `AGENTS.md` Rule 10: append `tempfile` to test-only list

**File:** `AGENTS.md`

**Old:**
```markdown
  - Test-only: `proptest`, `rstest`, `pretty_assertions`
```

**New:**
```markdown
  - Test-only: `proptest`, `rstest`, `pretty_assertions`, `tempfile`
```

Rationale: Decision 8 — `tempfile` added in the same commit as the dep
itself. Open Question 3 answer — Rule 10 only; no Tech Stack section
addition.

#### Step 11 — Edit `AGENTS.md`: full-body replacement of `## Current State`

**File:** `AGENTS.md`

**Old (the entire `## Current State` block — from the section heading on
line ~517 through the last bullet immediately before the `---` separator
preceding `## Changelog` on line ~675; this block was rewritten by Session
14):**

The full Old text is the 1.3a-post-state body currently in AGENTS.md
lines 517-672 (read verbatim from disk before substituting; the planner
encodes the New body below, the coder targets the Old by re-reading the
file at execution time and pasting the New block).

**New (full-body replacement — paste this verbatim from `## Current
State` heading through the last `Notes` bullet, ending one blank line
before the `---` separator):**

```markdown
## Current State

> Rewritten by the agent at the end of every session.
> Keep it tight — the next agent reads this and knows exactly what to do.

**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.3 complete; Step 1.3.5 next).

**What was just completed (Step 1.3b — MDBX-Backed MptState + Restart Tests):**
`crates/krax-state/src/mpt/mod.rs` rewritten end-to-end. `MptState` now owns
`Arc<reth_db::DatabaseEnv>` (Decision 1) and exposes `pub fn open(path: &Path)
-> Result<Self, StateError>` (Decision 2) plus a test-and-integration-only
`pub fn open_temporary() -> Result<(Self, tempfile::TempDir), StateError>`
that returns the `TempDir` for caller-controlled drop ordering. The Step 1.3a
`BTreeMap` backing and `MptState::new()` / `#[derive(Default)]` are gone.
A flat `Slots: B256 → B256` table is defined via reth-db's `tables!` macro
(Decisions 6, 7). `impl State for MptState`: `get` opens a short-lived RoTxn
and reads; `set` opens-writes-commits a short-lived RwTxn (auto-flush per
Decision 4); `snapshot()` returns a `Box<MptSnapshot>` owning a reth-db RoTxn
(Decision 3); `commit()` is a sync-barrier returning `Ok(self.root())`;
`root()` returns `B256::ZERO` with the `// TODO Step 1.5` marker (unchanged).
`impl Snapshot for MptSnapshot`: `get` reads via the owned RoTxn; `release`
is a no-op (the `Box<Self>` drop releases the RoTxn via RAII, Decision 11).
The four 1.3a inline tests (`set_then_get_round_trips`, the three
`Journal::apply` tests) are rewritten to use `MptState::open_temporary()`.
`crates/krax-state/tests/restart.rs` created (integration test, gated behind
the `integration` feature per Rule 5): two restart tests
(`single_key_restart`, `multi_write_restart`) construct a `tempfile::TempDir`
explicitly, open `MptState` at that path, write, commit, drop the state,
reopen at the same path, and assert reads return the committed values
(Decision 9 a+b). `crates/krax-state/Cargo.toml`: `reth-db` added as runtime
dep; `tempfile` added as dev dep; new `[[test]] name = "restart",
required-features = ["integration"]` entry (Decisions 5, 8, 10).
`crates/krax-types/src/state.rs`: `StateError` gained one new variant,
`Io(#[from] reth_db::DatabaseError)`, kept `#[non_exhaustive]` (Decision 5).
`crates/krax-types/Cargo.toml`: `reth-db` added as runtime dep (required
because `StateError::Io` references `reth_db::DatabaseError` directly).
Workspace `Cargo.toml`: `tempfile = "3"` added to the test-only group of
`[workspace.dependencies]`.
`AGENTS.md` Rule 10 test-only approved-dep list: `tempfile` appended
(Decision 8). `AGENTS.md` Current State: full-body rewritten for 1.3b
completion. `AGENTS.md` Changelog: Session 15 appended at the BOTTOM (one
entry covering both commits, mirroring Session 14's two-commit shape).
`ARCHITECTURE.md` Step 1.3 heading `✅`; the `Wire MDBX as the durable
backend` checkbox closed (Commit 1); the `Restart test` checkbox closed
(Commit 2). Phase 1 Gate line items unchanged (the round-trip+restart line
was already typographically `✅` per the 1.3a convention; Decision 13
satisfied without an explicit edit).

**What Step 1.3a delivered (shipped 2026-05-12):**
`crates/krax-state/src/mpt/mod.rs` — initial in-memory `MptState`
(`BTreeMap<B256, B256>` backing, `pub fn new()`, owned-clone `MptSnapshot`,
`commit` no-op returning `Ok(self.root())`, `root` returns `B256::ZERO` with
`// TODO Step 1.5`). Inline `#[cfg(test)] mod tests` with `fn slot(n)` helper
and 4 tests (round-trip + 3 `Journal::apply`). `crates/krax-state/src/lib.rs`
rewritten with flat re-exports `pub use mpt::{MptSnapshot, MptState};`.
`crates/krax-state/Cargo.toml` added `krax-types`, `alloy-primitives`,
`rstest`, `pretty_assertions`. AGENTS.md Workflow & Conventions extended
with "Deferred work surfaces decisions early when they affect the deferral
point" subsection. ARCHITECTURE.md Step 1.3 checkboxes split into 1.3a/1.3b
halves; Step 1.5 — MPT Root Computation inserted between Step 1.4 and
Phase 1 Gate; Phase 1 Gate updated with a "Real MPT root computation in
place" line item. `crates/krax-types/src/journal.rs`'s `#[cfg(test)] mod
tests` (StubState + 3 apply tests) deleted entirely; the apply tests
migrated to `mpt/mod.rs`'s test module.

**What Step 1.2b delivered (test commit, shipped 2026-05-11):**
`crates/krax-types/src/test_helpers.rs` (`pub(crate) fn slot`, `pub(crate) fn concrete`);
`rwset.rs` and `journal.rs` `#[cfg(test)] mod tests` blocks with `rstest` truth tables and
`StubState`-backed `Journal::apply` tests; `Journal::discard` `compile_fail` doctest;
`Cargo.toml` dev deps; AGENTS.md Rule 5 amendment. (Note: `journal.rs`'s test
module was subsequently removed in Step 1.3a — Journal::apply tests now live
in `crates/krax-state/src/mpt/mod.rs`.)

**What Step 1.2a delivered (refactor commit, shipped 2026-05-11):**
`RWSet` derives `Debug, PartialEq, Eq`. `JournalEntry` and `Journal` each derive
`Debug, PartialEq, Eq`. `Block`, `PendingTx`, `MempoolEntry` each derive `Debug` only
(fallback path — `alloy_consensus::EthereumTxEnvelope` does not derive `PartialEq`;
confirmed against alloy-consensus 1.8.3 registry source, Decision 3).

**What Step 1.1b delivered:**
`crates/krax-types/src/tx.rs`: `PendingTx` (wraps `alloy_consensus::TxEnvelope`) and
`MempoolEntry` (`PendingTx` + `sender: Address` + `arrival_time: u64`).
`crates/krax-types/src/block.rs`: `Block` struct + `Block::new()` constructor; no hash
field (deferred to Phase 11). `crates/krax-types/src/rwset.rs`: `RWSet` enum
(`Concrete { r_set, w_set }` + `Everything`); borrowing `conflicts` and `union`; no
`#[derive(Clone)]`. `crates/krax-types/src/journal.rs`: `JournalEntry`, `Journal`,
borrowing `apply(&self, &mut dyn State)`, consuming `discard(self)`.
`crates/krax-types/src/lib.rs`: six modules + flat re-exports for all eight public types.
`alloy-consensus` added at workspace + crate. ARCHITECTURE.md Step 1.1b ✅; Step 3.1
`lookahead` updated `Vec<PendingTx>` → `Vec<MempoolEntry>`.

**What Step 1.1a delivered:**
`crates/krax-types/src/state.rs`: `StateError` enum (`Released` variant,
`#[non_exhaustive]`) and `State` trait (`get`, `set`, `snapshot`, `commit`, `root`) with
`Send + Sync` supertraits and object-safety assertion. `crates/krax-types/src/snapshot.rs`:
`Snapshot` trait (`get`, `release(self: Box<Self>)`) with `Send + Sync` supertraits and
object-safety assertion.

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
- `integration` feature on every crate other than `krax-state` is still an empty
  placeholder — `krax-state` is the first crate to actually use it (Step 1.3b's
  restart tests, per Rule 5).
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
1. 🔴 **Step 1.3.5 — Coverage Tooling.** Select and configure a Rust coverage tool
   (`cargo-llvm-cov` or `tarpaulin`), add `make coverage` to the Makefile, and apply
   exclusion annotations to data-only types per docs/plans/step-1.2-decisions.md
   Decision 8. The coverage configuration MUST include the `integration` feature when
   running `make coverage` so Step 1.3b's restart tests count toward the Phase 1 Gate
   >85% target.
2. **Step 1.4 — Snapshot Semantics** follows 1.3.5. Step 1.4's isolation tests sit on
   top of Step 1.3b's RoTxn-backed snapshot — no implementation work required, only
   tests + a `compile_fail` doctest for post-release reads.

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
- `MptState` no longer derives `Default` and `MptState::new()` no longer exists; the only
  production constructor is `MptState::open(path: &Path)`. Test code uses
  `MptState::open_temporary()`, which returns `(MptState, tempfile::TempDir)` — bind both;
  dropping the `TempDir` removes the on-disk MDBX env. Each `MptState::open` call produces a
  fresh MDBX env; environments are NOT shared between `MptState` instances (no env-sharing or
  refcounted-env-pool design — by intent; revisit if a real call site needs it).
- `crates/krax-state/tests/restart.rs` is the first crate test gated behind the `integration`
  feature (Rule 5). `make test` does NOT run it; `make test-integration` does. Coverage
  configuration in Step 1.3.5 MUST include the `integration` feature so the restart tests
  count.
- `StubState` (formerly in `crates/krax-types/src/journal.rs`'s `#[cfg(test)] mod tests`) was
  deleted in Step 1.3a. The three `Journal::apply` tests live in
  `crates/krax-state/src/mpt/mod.rs`'s `#[cfg(test)] mod tests` and exercise `MptState`
  directly. The `compile_fail` doctest on `Journal::discard` is unaffected.
- Phase 1 Gate items in ARCHITECTURE.md (lines 161-165) display `✅` as goal-state markers,
  not literal completion status. The convention is inherited from Step 1.3a; revisit at
  Step 1.5 close whether to unify on `- [x]` / `- [ ]` notation for gate items.
```

Rationale: Open Question 4 — planner drafts the post-1.3b Current State.
Records 1.3b delivery; advances "What to do next" to 1.3.5 (the
ARCHITECTURE.md sibling that follows Step 1.3); adds 1.3b-specific Notes
(constructor change, integration-feature first-use, MDBX env not shared);
acknowledges the Phase 1 Gate ✅ convention as a known wart.

#### Step 12 — Edit `AGENTS.md`: append Session 15 entry to the BOTTOM of the Changelog

**File:** `AGENTS.md`

**Insertion point:** after the final line of the existing Session 14
entry — that is, after the line:

```
**Commit suggestion (Commit 2):** `refactor(types): rewrite Journal::apply tests against MptState — Step 1.3a`
```

Add one blank line, then append the verbatim block below. This entry
covers BOTH 1.3b commits in one session. If the coder splits over two
sessions, replace with two Session entries (same content split on the
Commit-1/Commit-2 seam).

**Content to append:**
```markdown

### Session 15 — Step 1.3b (both commits): MDBX-Backed MptState + Restart Tests
**Date:** 2026-05-XX
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary (Commit 1 — feat(state): wire MDBX backend for MptState — Step 1.3b):**
Rewrote `crates/krax-state/src/mpt/mod.rs` end-to-end. `MptState` now owns
`Arc<reth_db::DatabaseEnv>` (Decision 1); production constructor is
`open(path: &Path) -> Result<Self, StateError>` (Decision 2); the in-memory
`BTreeMap` backing and `MptState::new()` / `#[derive(Default)]` are gone.
Test-and-integration-only `open_temporary() -> Result<(Self, TempDir),
StateError>` returns the `TempDir` for caller-controlled drop ordering.
Flat `Slots: B256 → B256` table defined via reth-db's `tables!` macro
(Decisions 6, 7). `impl State`: `get` opens a short-lived RoTxn; `set`
opens-writes-commits a short-lived RwTxn (auto-flush per Decision 4);
`snapshot()` returns a `Box<MptSnapshot>` owning a reth-db RoTxn (Decision 3);
`commit()` is a sync barrier returning `Ok(self.root())`; `root` unchanged
(placeholder `B256::ZERO`, Step 1.5 deferred). `impl Snapshot for MptSnapshot`:
RoTxn-backed `get`; `release` is a no-op (RAII drop releases the reader slot,
Decision 11). Four 1.3a inline tests rewritten to `MptState::open_temporary()`.
`StateError` gained one new variant: `Io(#[from] reth_db::DatabaseError)`
(Decision 5); `#[non_exhaustive]` retained. `crates/krax-types/Cargo.toml`
gained `reth-db` runtime dep (required because `StateError::Io` references
the reth-db error type directly). `crates/krax-state/Cargo.toml` gained
`reth-db` runtime dep and `tempfile` dev dep. Workspace `Cargo.toml` gained
`tempfile = "3"` in the test-only group. AGENTS.md Rule 10 test-only
approved-dep list: `tempfile` appended (Decision 8). AGENTS.md Current
State rewritten for 1.3b completion. ARCHITECTURE.md Step 1.3 heading `✅`;
the `Wire MDBX as the durable backend` checkbox closed.
**Summary (Commit 2 — test(state): add MDBX restart test — Step 1.3b):**
Added `crates/krax-state/tests/restart.rs` (new file; module gated behind
`#[cfg(feature = "integration")]` per Rule 5) with two restart tests:
`single_key_restart` (open at TempDir, set, commit, drop, reopen, get)
and `multi_write_restart` (open, set k1 and k2, commit, drop, reopen, get
both) per Decision 9 (a + b). Tests use `tempfile::TempDir` directly (NOT
`MptState::open_temporary`) because explicit path control across the drop
boundary is the load-bearing property. `crates/krax-state/Cargo.toml` gained
`[[test]] name = "restart", required-features = ["integration"]`
(Decision 10). ARCHITECTURE.md Step 1.3 restart-test checkbox closed.
**Commit suggestion (Commit 1):** `feat(state): wire MDBX backend for MptState — Step 1.3b`
**Commit suggestion (Commit 2):** `test(state): add MDBX restart test — Step 1.3b`
```

Rationale: Decision 12 (two commits, one Session 15 entry) + Session 14
precedent.

### Verification suite (Commit 1)

| # | Check | Command | Expected |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | krax-types compiles | `cargo check -p krax-types` | exit 0; `StateError::Io` available |
| 3 | krax-state compiles | `cargo check -p krax-state` | exit 0 |
| 4 | Tests pass | `make test` | exit 0; 4 tests in `mpt::tests` (round-trip + 3 apply) appear and pass |
| 5 | Integration tests skipped (Commit 1 only) | `make test` | restart tests do NOT run (no `tests/restart.rs` yet) |
| 6 | krax-types doctest regression | `cargo test --doc -p krax-types` | exit 0 (Journal::discard `compile_fail` doctest still passes) |
| 7 | Lint clean | `make lint` | exit 0 (no `missing_docs`, no `clippy::unwrap_used`, no pedantic firings) |
| 8 | Docs build | `cargo doc --workspace --no-deps` | exit 0 |
| 9 | `MptState::open` exists | `grep -n 'pub fn open' crates/krax-state/src/mpt/mod.rs` | match found |
| 10 | `MptState::new` removed | `grep -c 'pub fn new' crates/krax-state/src/mpt/mod.rs` | 0 |
| 11 | `#[derive(Default)]` removed from MptState | `grep -n 'derive.*Default' crates/krax-state/src/mpt/mod.rs` | no match against MptState |
| 12 | `Io` variant in StateError | `grep -n 'Io(#\[source\] Box' crates/krax-types/src/state.rs` | match found |
| 13 | `StateError::io` constructor exists | `grep -n 'pub fn io' crates/krax-types/src/state.rs` | match found |
| 14 | reth-db in krax-state | `grep -n '^reth-db' crates/krax-state/Cargo.toml` | match found |
| 15 | reth-db NOT in krax-types | `grep -c '^reth-db' crates/krax-types/Cargo.toml` | 0 |
| 16 | tempfile in workspace | `grep -n 'tempfile' Cargo.toml` | match found in test-only group |
| 17 | tempfile in krax-state dev-deps | `grep -n 'tempfile' crates/krax-state/Cargo.toml` | match found in `[dev-dependencies]` |
| 18 | tempfile in AGENTS.md Rule 10 | `grep -n 'Test-only:.*tempfile' AGENTS.md` | match found |
| 19 | Step 1.3 heading ✅ | `grep -n '### Step 1.3 — MPT State Backend (Skeleton) ✅' ARCHITECTURE.md` | match found |
| 20 | MDBX checkbox closed | `grep -n '\[x\] Wire MDBX' ARCHITECTURE.md` | match found |
| 21 | Restart-test checkbox still open | `grep -n '\[ \] Restart test' ARCHITECTURE.md` | match found (closes in Commit 2) |
| 22 | TODO Step 1.5 marker preserved | `grep -n 'TODO Step 1.5' crates/krax-state/src/mpt/mod.rs` | match found |
| 23 | Session 15 at bottom | `tail -60 AGENTS.md \| grep '### Session 15'` | match found (entry is last) |

### Commit message (Commit 1)

```
feat(state): wire MDBX backend for MptState — Step 1.3b
```

### Outcomes (Commit 1)

- Files changed:
  - `Cargo.toml` (workspace — `tempfile = "3"` added to test-only group)
  - `crates/krax-types/src/state.rs` (`StateError::Io` boxed-source variant added; `StateError::io()` constructor added)
  - `crates/krax-state/Cargo.toml` (`reth-db` runtime dep, `tempfile` dev dep added)
  - `crates/krax-state/src/mpt/mod.rs` (rewritten — MDBX-backed `MptState`, RoTxn-backed `MptSnapshot`, 4 inline tests under `open_temporary`)
  - `ARCHITECTURE.md` (Step 1.3 heading ✅; MDBX checkbox closed; restart-test checkbox left for Commit 2)
  - `AGENTS.md` (Rule 10 appended `tempfile`; Current State full-body rewritten; Session 15 appended to BOTTOM of Changelog)
  - `docs/plans/step-1.3b-plan.md` (Outcomes filled in)

  NOT changed (per Decision 5 maintainer revision):
  - `crates/krax-types/Cargo.toml` — stays pristine; no `reth-db` dep added.

- Verification table results (all 23 rows exit 0):

| # | Check | Result |
|---|---|---|
| 1 | `make build` | ✅ exit 0 |
| 2 | `cargo check -p krax-types` | ✅ exit 0 — `StateError::Io` + `io()` available |
| 3 | `cargo check -p krax-state` | ✅ exit 0 |
| 4 | `make test` | ✅ exit 0; 4 `mpt::tests` pass (round-trip + 3 apply) |
| 5 | restart tests skipped in `make test` | ✅ — no `tests/restart.rs` exists yet |
| 6 | `cargo test --doc -p krax-types` | ✅ exit 0 — Journal::discard compile_fail doctest passes |
| 7 | `make lint` | ✅ exit 0 — clean under `-D warnings` (two `doc_markdown` lints fixed in-line — DatabaseError + RoTxn + RwTxn backticked) |
| 8 | `cargo doc --workspace --no-deps` | ✅ exit 0 |
| 9 | `grep 'pub fn open' mpt/mod.rs` | ✅ match on lines 147 + 159 |
| 10 | `grep -c 'pub fn new' mpt/mod.rs` | ✅ 0 |
| 11 | `grep 'derive.*Default' mpt/mod.rs` | ✅ no match against MptState |
| 12 | `grep 'Io(#[source] Box' state.rs` | ✅ match on line 27 |
| 13 | `grep 'pub fn io' state.rs` | ✅ match on line 35 |
| 14 | `grep '^reth-db' krax-state/Cargo.toml` | ✅ match — runtime dep with `features = ["mdbx"]` |
| 15 | `grep -c '^reth-db' krax-types/Cargo.toml` | ✅ 0 — krax-types stays pristine |
| 16 | `tempfile` in workspace Cargo.toml | ✅ match in test-only group, line 124 |
| 17 | `tempfile` in krax-state dev-deps | ✅ match line 23 |
| 18 | `tempfile` in AGENTS.md Rule 10 | ✅ match line 378 |
| 19 | Step 1.3 heading ✅ | ✅ match ARCHITECTURE.md line 134 |
| 20 | MDBX checkbox `[x]` | ✅ match ARCHITECTURE.md line 136 |
| 21 | Restart-test checkbox `[ ]` | ✅ match ARCHITECTURE.md line 139 (closes in Commit 2) |
| 22 | TODO Step 1.5 marker preserved | ✅ match mpt/mod.rs line 220 |
| 23 | Session 15 at bottom of Changelog | ✅ match (tail of AGENTS.md) |

- Deviations from plan:
  1. **LVP Q1 — env open function name.** Planner expected
     `reth_db::create_db_and_tables(path)`. Actual: `reth_db::mdbx::init_db_for::<P,
     TS: TableSet>(path, DatabaseArguments) -> eyre::Result<DatabaseEnv>` (source:
     `crates/storage/db/src/mdbx.rs:107`). Substituted. `init_db_for` is the
     TableSet-aware sibling of `init_db`; needed because we register a custom
     `SlotsTableSet`. Return is `eyre::Result`, not `Result<_, DatabaseError>`.
  2. **LVP Q1 — eyre::Report wrapping.** `eyre::Report` does NOT implement
     `std::error::Error`, so `.map_err(StateError::io)?` does not compile against
     it. Added `fn display_to_state<E: std::fmt::Display>(e: E) -> StateError`
     adapter that wraps via `std::io::Error::other(e.to_string())` before boxing
     into `StateError::Io`. Loses the eyre chain; preserves the message. Generic
     over `Display` so no `eyre` direct-dep required in krax-state.
  3. **LVP Q3 — `tables!` macro is in-tree-private (per Decision 7 fallback).**
     The macro IS exported at `reth_db::tables!` and the syntax matches the
     planner's example, but the macro body emits `pub enum Tables { ... }` and
     references a sibling `table_names` module — both designed for reth-db's own
     invocation site, not callers. Hand-rolled `Table` + `TableInfo` + `TableSet`
     impls for `Slots` and a marker `SlotsTableSet` for `init_db_for`.
  4. **LVP Q6 — B256 has no reachable `Compress` impl.** B256 has `Encode + Decode`
     in reth-db's `db-api/src/models/mod.rs` (32 raw bytes) — usable directly as
     `type Key = B256`. But the `Value` trait (`Compress + Decompress + Serialize`)
     is NOT satisfied for B256 from outside reth-codecs's crate (orphan-rule
     blocked). Substituted `type Value = Vec<u8>;` and convert at the
     `State::get` / `State::set` / `Snapshot::get` boundary — wire format remains
     exactly 32 raw bytes (matches planner-expected on-disk shape per Decision 6).
     If a future reth-codecs change adds a reachable `impl Compress for B256`,
     this can be tightened to `type Value = B256` without altering the on-disk
     format.
  5. **`reth-db` mdbx feature enable (Cargo.toml configuration).** Workspace dep
     is `default-features = false`; the env/txn surface (`DatabaseEnv`,
     `init_db_for`, `DatabaseArguments`) is gated on `feature = "mdbx"`. Enabled
     via `reth-db = { workspace = true, features = ["mdbx"] }` in
     `crates/krax-state/Cargo.toml`.
  6. **`make lint` follow-up — `clippy::doc_markdown` pedantic.** Three doc-comment
     identifiers (`DatabaseError`, `RoTxn`, `RwTxn`) initially fired pedantic
     `doc_markdown`; backticked in-line. No `#[allow]` needed.
  7. **Step 8 micro-decision — recommended reading applied.** Closed Step 1.3
     heading `✅` and the MDBX checkbox in Commit 1; restart-test checkbox stays
     open for Commit 2.

  NOT triggered:
  - RoTxn structural concerns (Decision 3 protected path) — `<DatabaseEnv as
    Database>::TX: + 'static` confirmed by LVP Q2 source read, so `MptSnapshot {
    tx }` requires no lifetime parameter. Structural shape preserved.
  - krax-types dependency on reth-db — explicitly avoided per Decision 5
    maintainer revision (boxed-source variant).

  Context7 query results (mandatory queries):
  - Q1 — Context7 confirmed `Database::tx()`/`Database::tx_mut()` shape but did
    not surface env-open name; on-disk fallback found `init_db` / `init_db_for`.
    Substitution: `init_db_for` (see Deviation 1).
  - Q2 — Context7 confirmed `Database::TX: DbTx + Send + Sync + Debug + 'static`,
    `Database::TXMut: DbTxMut + DbTx + TableImporter + ...`, `DbTx::get`,
    `DbTx::commit`, `DbTxMut::put`. No substitution.
  - Q3 — Context7 did not surface the `tables!` macro body; on-disk fallback at
    `db-api/src/tables/mod.rs:115` confirmed macro is in-tree-private (see
    Deviation 3).
  - Q5 — Context7 + on-disk confirmed `reth_db::DatabaseError` (re-export of
    `reth_storage_errors::db::DatabaseError`) implements `std::error::Error +
    Send + Sync + 'static`. Recommended optional verification per task brief.
  - Q6 — Context7 + on-disk confirmed `impl Encode for B256` / `impl Decode for
    B256` at `db-api/src/models/mod.rs:89-101` (32 raw bytes). Value-side
    Compress impl absent — see Deviation 4.
  - Q7 — Confirmed `tempfile::TempDir` used directly, no reth-db test_utils.
  - Q4 (informational) — not run.
  - Q8 (conditional) — not run; no compile error required it.

- Proposed commit message (final): `feat(state): wire MDBX backend for MptState — Step 1.3b`
- Notes for the maintainer:
  - **Three of the four LVP-driven deviations (Q1 env-open name, Q3 tables! macro
    fallback, Q6 B256 Value wrapping) are surface-level substitutions — wire
    format, structural shape, and test coverage are preserved. The fourth (Q1
    eyre-wrap) introduces a `display_to_state` helper in `mpt/mod.rs` that loses
    the eyre error chain in `MptState::open` failures. Acceptable for V1 (the
    error message is preserved); revisit if a downstream caller needs structured
    open-error introspection.**
  - **Hand-rolled `Slots` table** is in `mpt/mod.rs` (struct + 3 trait impls,
    ~30 lines). If a future step needs additional tables, consider whether to
    introduce a small `tables.rs` sibling module rather than co-locating in
    `mpt/mod.rs`. Out of scope for 1.3b.
  - **`Value = Vec<u8>` is a present-day workaround for the B256 Compress
    orphan-rule blocker.** Two future paths: (a) reth-codecs upstream gains a
    direct `impl Compress for B256` / `impl<const N: usize> Compress for
    FixedBytes<N>`, after which we tighten to `type Value = B256`; (b) we add
    `reth-codecs = { workspace = true }` as a direct dep and wrap B256 in a
    local newtype with hand-rolled Compress/Decompress. The current approach
    avoids both — no extra dep, no wrapper type, wire format is identical.
  - **Phase 1 Gate items in ARCHITECTURE.md were not edited** — they already
    display `✅` typographically from the 1.3a planner's convention. The "MPT
    state backend passes round-trip and restart tests" gate line is therefore
    satisfied per Decision 13 without an explicit edit. Consider unifying on
    `- [x]` / `- [ ]` notation at Step 1.5 close.
  - `MptState::open_temporary` is exposed under `#[cfg(any(test, feature =
    "integration"))]` so it's usable from the integration test in Commit 2 if
    desired. Commit 2's restart test deliberately does NOT use it (Decision 9 —
    explicit path control across the drop boundary is required).
  - Step 1.3.5 (Coverage Tooling) is now the next ARCHITECTURE.md step. Its plan
    MUST include the `integration` feature in coverage runs so the restart tests
    count toward Phase 1 Gate.

---

## Commit 2: test(state): add MDBX restart test — Step 1.3b

### Purpose

Add `crates/krax-state/tests/restart.rs` (new file) with two integration
tests gated behind the `integration` feature per Rule 5. Register the test
binary in `crates/krax-state/Cargo.toml` via `[[test]] name = "restart",
required-features = ["integration"]`. Close the `Restart test` checkbox in
ARCHITECTURE.md Step 1.3. No production-code changes. No AGENTS.md edits.

### Execution Steps

#### Step 1 — Edit `crates/krax-state/Cargo.toml`: add `[[test]]` entry

**File:** `crates/krax-state/Cargo.toml`

**Old (target the end of the file, after the `[lints]` block):**
```toml
[features]
# Empty placeholder; integration tests gated behind this flag land in Phase 1+.
integration = []

[lints]
workspace = true
```

**New:**
```toml
[features]
# `integration` gates filesystem-touching tests per AGENTS.md Rule 5.
# First user: tests/restart.rs (Step 1.3b).
integration = []

[lints]
workspace = true

[[test]]
name              = "restart"
path              = "tests/restart.rs"
required-features = ["integration"]
```

Rationale: Decision 10 — restart test gated behind `integration` feature.
The `name`/`path` pair is explicit; Cargo would otherwise infer name from
filename, but explicit is clearer.

#### Step 2 — Create `crates/krax-state/tests/restart.rs`

**File:** `crates/krax-state/tests/restart.rs` (new file)

**Full content:**
```rust
//! Integration tests for MDBX-backed `MptState` durability across
//! `MptState::open` → drop → `MptState::open` (Step 1.3b, Decision 9).
//!
//! Gated behind the `integration` feature per AGENTS.md Rule 5 because
//! the tests touch the real filesystem (MDBX env at a `TempDir` path).
//! Run via `make test-integration`; `make test` does NOT exercise them.

#![cfg(feature = "integration")]
#![allow(clippy::unwrap_used)]

use alloy_primitives::B256;
use krax_state::MptState;
use krax_types::State;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn slot(n: u8) -> B256 {
    B256::from([n; 32])
}

#[test]
fn single_key_restart() {
    // Decision 9 (a): open at an explicit TempDir, set, commit, drop the
    // MptState, reopen at the same path, assert get returns the committed
    // value. The TempDir is bound for the full test so it outlives both
    // MptState instances; it's dropped (and the directory deleted) at the
    // end of the test scope.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();

    {
        let mut state = MptState::open(&path).unwrap();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        // `state` drops at end of block — MDBX env closes.
    }

    let reopened = MptState::open(&path).unwrap();
    assert_eq!(reopened.get(slot(1)).unwrap(), slot(42));
}

#[test]
fn multi_write_restart() {
    // Decision 9 (b): same shape as single_key_restart but with two
    // distinct writes. Catches single-key serialization bugs that wouldn't
    // surface with one slot.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();

    {
        let mut state = MptState::open(&path).unwrap();
        state.set(slot(1), slot(10)).unwrap();
        state.set(slot(2), slot(20)).unwrap();
        state.commit().unwrap();
    }

    let reopened = MptState::open(&path).unwrap();
    assert_eq!(reopened.get(slot(1)).unwrap(), slot(10));
    assert_eq!(reopened.get(slot(2)).unwrap(), slot(20));
}
```

Rationale: Decision 9 (a) + (b); Decision 10 (b); Rule 5 integration
gating. Module-level `#![cfg(feature = "integration")]` ensures the file is
empty (cargo-test-wise) without the feature; `required-features` in the
`[[test]]` entry ensures cargo refuses to build the test binary unless
the feature is enabled, avoiding the empty-test-binary warning.
`#![allow(clippy::unwrap_used)]` at module level (test code, per AGENTS.md
"For `unwrap_used`, tests are exempt"). Tests use `tempfile::TempDir`
directly (NOT `MptState::open_temporary()`) per Decision 9 — explicit
path control across the drop boundary is the load-bearing property.

#### Step 3 — Edit `ARCHITECTURE.md`: close the restart-test checkbox

**File:** `ARCHITECTURE.md`

**Old:**
```markdown
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value (Step 1.3b)
```

**New:**
```markdown
- [x] Restart test: open DB, set, commit, close, reopen, get returns committed value (Step 1.3b)
```

Rationale: Decision 13 + Commit-1 micro-decision recommendation — the
restart-test checkbox closes alongside the test file that proves it.

**If the coder adopted the atomic-shape counter-reading in Commit 1** (and
already flipped this checkbox to `[x]` in Commit 1's Step 8): SKIP this
step entirely and note in Outcomes Deviations.

### Verification suite (Commit 2)

| # | Check | Command | Expected |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | Unit tests pass; restart tests NOT exercised | `make test` | exit 0; `single_key_restart` and `multi_write_restart` do NOT appear in output (feature-gated) |
| 3 | Integration tests pass | `make test-integration` | exit 0; `single_key_restart` and `multi_write_restart` both pass |
| 4 | Doctest regression | `cargo test --doc -p krax-types` | exit 0 |
| 5 | Lint clean | `make lint` | exit 0 |
| 6 | Lint clean under integration feature | `cargo clippy --all-targets --features integration -- -D warnings` | exit 0 |
| 7 | `[[test]]` entry present | `grep -n 'name              = "restart"' crates/krax-state/Cargo.toml` | match found |
| 8 | `required-features` set | `grep -n 'required-features = \["integration"\]' crates/krax-state/Cargo.toml` | match found |
| 9 | restart.rs exists | `test -f crates/krax-state/tests/restart.rs && echo OK` | prints OK |
| 10 | `single_key_restart` defined | `grep -n 'fn single_key_restart' crates/krax-state/tests/restart.rs` | match found |
| 11 | `multi_write_restart` defined | `grep -n 'fn multi_write_restart' crates/krax-state/tests/restart.rs` | match found |
| 12 | feature gate present | `grep -n '#!\[cfg(feature = "integration")\]' crates/krax-state/tests/restart.rs` | match found |
| 13 | Restart-test checkbox closed | `grep -n '\[x\] Restart test' ARCHITECTURE.md` | match found |

### Commit message (Commit 2)

```
test(state): add MDBX restart test — Step 1.3b
```

### Outcomes (Commit 2)

- Files changed:
  - `crates/krax-state/Cargo.toml` (`[[test]] name = "restart"` entry added)
  - `crates/krax-state/tests/restart.rs` (new file; two restart tests)
  - `ARCHITECTURE.md` (restart-test checkbox closed — unless already closed in Commit 1)

- Verification table results (all 13 rows exit 0):

| # | Check | Result |
|---|---|---|
| 1 | `make build` | ✅ exit 0 |
| 2 | `make test` does NOT run restart tests | ✅ 0 matches for `single_key_restart`/`multi_write_restart` (feature-gated) |
| 3 | `make test-integration` runs restart tests | ✅ both `single_key_restart` and `multi_write_restart` pass |
| 4 | `cargo test --doc -p krax-types` | ✅ exit 0 — Journal::discard compile_fail doctest passes |
| 5 | `make lint` | ✅ exit 0 |
| 6 | `cargo clippy --all-targets --features integration -- -D warnings` | ✅ exit 0 |
| 7 | `[[test]]` entry present | ✅ match line 41 |
| 8 | `required-features = ["integration"]` | ✅ match line 43 |
| 9 | `tests/restart.rs` exists | ✅ OK |
| 10 | `fn single_key_restart` defined | ✅ match line 22 |
| 11 | `fn multi_write_restart` defined | ✅ match line 43 |
| 12 | `#![cfg(feature = "integration")]` present | ✅ match line 8 |
| 13 | Restart-test checkbox `[x]` | ✅ ARCHITECTURE.md line 139 |

- Deviations from plan:
  1. **`tempfile` promoted to optional regular dep with `dep:tempfile` in the
     `integration` feature.** The plan only adds `tempfile` as a dev-dependency.
     But because `MptState::open_temporary` is cfg'd under `any(test, feature =
     "integration")`, it names `tempfile::TempDir` in its return type even
     during non-test `--features integration` builds. Under the plain
     `cargo clippy --all-targets --features integration` invocation
     (verification row 6), the library is checked without `cfg(test)` and
     therefore needs `tempfile` available as a regular (not dev) dep. Resolved
     by adding `tempfile = { workspace = true, optional = true }` to
     `[dependencies]` and `integration = ["dep:tempfile"]` to `[features]`.
     The dev-dependency entry is retained so plain `cargo test` (no integration
     feature) still has tempfile available. No knock-on for the test crate.
  2. **Used the recommended micro-decision from Commit 1.** Commit 1 left the
     restart-test checkbox open and Step 3 of Commit 2 closed it. No deviation
     from the recommended reading; the counter-reading path was not taken.

- Proposed commit message (final): `test(state): add MDBX restart test — Step 1.3b`
- Notes for the maintainer:
  - Both restart tests construct `TempDir` and `MptState` separately rather than via `MptState::open_temporary` — this is intentional per Decision 9; the test asserts behavior across an explicit drop/reopen boundary at a known path.
  - `make test-integration` is now a load-bearing CI invocation for Phase 1 Gate closure. If CI is configured to run only `make test`, it MUST be extended to also run `make test-integration` (or the gate is not actually verified end-to-end).
  - The `dep:tempfile` integration-feature wiring (Deviation 1) is a small but
    real change to `[features]` semantics. Worth flagging that any future crate
    that exposes a `#[cfg(any(test, feature = "integration"))]` test-fixture
    helper naming an external type will need the same `dep:`-prefixed
    `optional = true` pattern.

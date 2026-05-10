# Step 1.1a — Trait Interfaces (`State`, `Snapshot`, `StateError`)

> **Plan status:** Ready for execution.
> **Phase:** 1 — Domain Types & State Trait (first sub-step).
> **ARCHITECTURE.md reference:** Phase 1, Step 1.1a.
> **Prerequisites:** Step 0.9 (README) complete and committed. `make build`, `make lint`, and
> `make test` all exit 0.

---

## Purpose

Four deliverables, all inside `crates/krax-types/`:

1. **`src/state.rs` (new)** — `StateError` enum (`Released` variant, `#[non_exhaustive]`) and
   `State` trait (five methods, `Send + Sync` supertraits, compile-time object-safety assertion).

2. **`src/snapshot.rs` (new)** — `Snapshot` trait (two methods, `Send + Sync` supertraits,
   compile-time object-safety assertion). `release` signature is **consuming**
   (`self: Box<Self>`) — post-release reads are a compile error, not a runtime error.

3. **`src/lib.rs` (rewrite)** — `pub mod` declarations and flat `pub use` re-exports so
   downstream crates write `use krax_types::State` rather than
   `use krax_types::state::State`.

4. **`Cargo.toml` (edit)** — replace the empty `[dependencies]` comment block with two
   workspace-inherited entries: `alloy-primitives` and `thiserror`.

In the same commit, two other files are updated:

5. **`ARCHITECTURE.md`** — Step 1.1a heading `✅`, all four checkboxes `[x]`, plus a text-only
   reconciliation to Step 1.4 (consuming `release` makes the "post-release get" test a
   compile-fail, not a runtime check).

6. **`AGENTS.md`** — Current State updated ("What was just completed: Step 1.1a",
   "What to do next: Step 1.1b") and Changelog Session 11 appended at the bottom.

This step is **trait definitions only**. No concrete implementations, no real tests, no
`trybuild` infrastructure, no `PendingTx`/`Block`/`RWSet`/`Journal` types (those are Step 1.1b).

---

## Decisions resolved before this plan was written

All ten decisions below are **final**. They were made in a pre-planning session and recorded in
`docs/plans/step-1.1a-decisions.md` (the canonical source). Do not re-derive or re-surface
them; cite the decisions document if you need background.

| # | Topic | Resolution |
|---|---|---|
| 1 | `release` signature | `release(self: Box<Self>)` — consuming. Post-release use is a compile error. Step 1.4 uses `trybuild` or `compile_fail` doctest. |
| 2 | `StateError` starter variants | `Released` only, with `#[non_exhaustive]`. |
| 3 | `StateError` location | Inside `state.rs`, above the `State` trait. |
| 4 | Object-safety assertion form | Ungated `const _: Option<&dyn Trait> = None;` at module scope in each trait file, preceded by a `//` comment. |
| 5 | Stub tests | None in Step 1.1a. Object-safety assertions cover compile-time validation. |
| 6 | `Send + Sync` bounds | Both `State: Send + Sync` and `Snapshot: Send + Sync`. |
| 7 | `State` method signatures | `get`, `set`, `snapshot` (returns `Result<Box<dyn Snapshot>, StateError>`), `commit` (returns `Result<B256, StateError>`), `root` (returns `B256`). |
| 8 | `Snapshot` method signatures | `get` (returns `Result<B256, StateError>`), `release(self: Box<Self>)`. |
| 9 | `Cargo.toml` edit | Replace empty comment block with `alloy-primitives` and `thiserror` workspace deps. |
| 10 | `lib.rs` re-exports | `pub mod snapshot; pub mod state;` + `pub use snapshot::Snapshot; pub use state::{State, StateError};`. |

---

## Library verification checklist

All verification was performed in the pre-planning session. Results are captured in
`docs/plans/step-1.1a-decisions.md`. Do **not** re-run Context7 queries; the relevant findings
are reproduced here for the coder's reference.

| Library | Version | Relevant API | Status |
|---|---|---|---|
| `alloy-primitives` | v1 (workspace-pinned) | `B256` = `FixedBytes<32>`; derives include `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`; `B256::ZERO` confirmed. The workspace definition includes `features = ["serde"]` — inherited by per-crate `{ workspace = true }` — but this does NOT add `Serialize`/`Deserialize` to our own types. | ✅ Verified via Context7 (`/alloy-rs/core`, 2026-05-09) |
| `thiserror` | v2.0.18 (workspace-pinned at `"2"`) | `#[derive(Error, Debug)]` on enum; `#[error("...")]` on variants; `#[from]`, `#[source]`, `#[backtrace]` available. Step 1.1a does NOT use `#[from]` — no foreign errors at this layer. | ✅ Verified via Context7 (`/websites/rs_thiserror_2_0_18`, 2026-05-09) |
| Root `Cargo.toml` | — | Both `alloy-primitives` and `thiserror` already present in `[workspace.dependencies]`. No workspace-root edits required. | ✅ Confirmed by reading `Cargo.toml` lines 67 and 98+ |

---

## Files to create or modify

### Ordered execution sequence

1. Create `crates/krax-types/src/state.rs`
2. Create `crates/krax-types/src/snapshot.rs`
3. Rewrite `crates/krax-types/src/lib.rs`
4. Edit `crates/krax-types/Cargo.toml` — str_replace `[dependencies]` block
5. Edit `ARCHITECTURE.md` — Step 1.1a checkboxes and heading (str_replace)
6. Edit `ARCHITECTURE.md` — Step 1.4 test text reconciliation (str_replace)
7. Edit `AGENTS.md` — Current State replacement
8. Edit `AGENTS.md` — Changelog append
9. Run verification steps

---

### Step 1 (create): `crates/krax-types/src/state.rs`

New file. LF line endings, trailing newline.

**Exact content:**

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

/// The V1↔V2 state backend contract.
///
/// All state mutations flow through this trait. Concrete implementations live
/// in `krax-state`; consumers depend only on this abstraction (AGENTS.md Rule 1).
/// Consumed as `&mut dyn State` in the commit phase and as `&dyn State` for
/// read-only paths such as RPC root queries.
pub trait State: Send + Sync {
    /// Returns the current value of `slot`, or `B256::ZERO` if unset.
    fn get(&self, slot: B256) -> Result<B256, StateError>;

    /// Writes `val` to `slot`. Writes are pending until [`commit`][State::commit].
    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError>;

    /// Creates an isolated read-only snapshot at the current commit point.
    ///
    /// Workers read from snapshots so they never observe each other's pending
    /// writes. See AGENTS.md "State Snapshot".
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError>;

    /// Durably applies all pending writes and returns the post-commit state root.
    ///
    /// The returned root will be posted to Ethereum L1 in Phase 14.
    fn commit(&mut self) -> Result<B256, StateError>;

    /// Returns the current state root without committing pending writes.
    ///
    /// Concrete implementations may return a cached value.
    fn root(&self) -> B256;
}

// Compile-time assertion that State is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
const _: Option<&dyn State> = None;
```

---

### Step 2 (create): `crates/krax-types/src/snapshot.rs`

New file. LF line endings, trailing newline.

**Exact content:**

```rust
//! Snapshot trait for isolated read-only views of Krax state.

use alloy_primitives::B256;

use crate::state::StateError;

/// A consistent read-only view of state at a single commit point.
///
/// Obtained via [`State::snapshot`][crate::State::snapshot]. Workers read from
/// snapshots so they never observe each other's uncommitted writes (AGENTS.md
/// "State Snapshot").
///
/// `release` takes `self: Box<Self>` — the `Box` is consumed, so any attempt to
/// call `get` after `release` is a compile error ("borrow of moved value"). This
/// is the compile-time guarantee chosen in step-1.1a-decisions.md Decision 1.
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

// Compile-time assertion that Snapshot is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
const _: Option<&dyn Snapshot> = None;
```

---

### Step 3 (rewrite): `crates/krax-types/src/lib.rs`

Read the current file first (3 lines; crate-doc only) to confirm state before rewriting.

**Exact content after rewrite:**

```rust
//! krax-types: core domain types and cross-crate traits.
//!
//! This crate is the single point of cross-crate type sharing for the Krax workspace.
//! All other crates depend on the traits defined here; none import concrete types
//! from each other directly. See AGENTS.md Rule 1.

pub mod snapshot;
pub mod state;

pub use snapshot::Snapshot;
pub use state::{State, StateError};
```

Downstream crates write `use krax_types::State` (not `use krax_types::state::State`).

---

### Step 4 (edit): `crates/krax-types/Cargo.toml` — `[dependencies]` block

Read the file first to confirm the exact whitespace of the current `[dependencies]` block
(it is three lines: heading, comment, blank). Then apply the str_replace below.

**str_replace:**

Old:
```toml
[dependencies]
# Intentionally empty. Dependencies are added in the phase where this crate
# first needs them, per AGENTS.md Rule 10.
```

New:
```toml
[dependencies]
# B256 (= FixedBytes<32>) is the slot key and value type throughout the State trait.
alloy-primitives = { workspace = true }
# Per-crate typed errors per AGENTS.md Rule 3.
thiserror        = { workspace = true }
```

Both entries use workspace inheritance. No version pins in the per-crate `Cargo.toml`.
No other changes to `crates/krax-types/Cargo.toml`.

---

### Step 5 (edit): `ARCHITECTURE.md` — Step 1.1a checkboxes and heading

Two str_replaces, applied in order.

#### (a) Check off all four Step 1.1a checkboxes

**str_replace:**

Old:
```
- [ ] `crates/krax-types/src/state.rs` — `State` trait: read/write/snapshot/commit/root methods. Object-safe (consumed downstream as `&mut dyn State`). See "Open design questions for Step 1.1a" below.
- [ ] `crates/krax-types/src/snapshot.rs` — `Snapshot` trait: `get` + `release` semantics. Released-snapshot detection is observable via `StateError::Released`. See "Open design questions for Step 1.1a" below.
- [ ] `StateError` enum (in `state.rs` or a sibling module) using `thiserror`. Start minimal; extend in Step 1.3 when MDBX I/O surfaces real error variants.
- [ ] Add `alloy-primitives` and `thiserror` to `crates/krax-types/Cargo.toml` (workspace inheritance).
```

New:
```
- [x] `crates/krax-types/src/state.rs` — `State` trait: read/write/snapshot/commit/root methods. Object-safe (consumed downstream as `&mut dyn State`). See "Open design questions for Step 1.1a" below.
- [x] `crates/krax-types/src/snapshot.rs` — `Snapshot` trait: `get` + `release` semantics. Released-snapshot detection is observable via `StateError::Released`. See "Open design questions for Step 1.1a" below.
- [x] `StateError` enum (in `state.rs` or a sibling module) using `thiserror`. Start minimal; extend in Step 1.3 when MDBX I/O surfaces real error variants.
- [x] Add `alloy-primitives` and `thiserror` to `crates/krax-types/Cargo.toml` (workspace inheritance).
```

#### (b) Mark Step 1.1a heading ✅

**str_replace:**

Old:
```
### Step 1.1a — Trait Interfaces (`State`, `Snapshot`, `StateError`)
```

New:
```
### Step 1.1a — Trait Interfaces (`State`, `Snapshot`, `StateError`) ✅
```

---

### Step 6 (edit): `ARCHITECTURE.md` — Step 1.4 test text reconciliation

The current Step 1.4 text assumes `s.release()` is non-consuming and that calling
`s.get(...)` afterwards returns `StateError::Released` at runtime. This is incompatible
with Decision 1's consuming signature: after `s.release()` the `Box<dyn Snapshot>` is
moved and `s.get(...)` is a compile error, not a runtime error. Update the third checkbox
of Step 1.4 to reflect this.

This is a **text-only** change. No `trybuild` infrastructure is added in Step 1.1a; that
work belongs to Step 1.4 itself.

**str_replace:**

Old:
```
- [ ] Test: `s.release()` then `s.get` returns a `StateError::Released`
```

New:
```
- [ ] Test: `s.release(); s.get(...);` — must fail to compile (use `trybuild` or a `compile_fail` doctest); set up `trybuild` infrastructure in this step.
```

---

### Step 7 (edit): `AGENTS.md` — Current State replacement

Replace the full body of the `## Current State` section — from the line beginning
`**Current Phase:**` through the last `**Notes:**` bullet — with the content below.
Leave the section header (`## Current State`) and its `> Rewritten by the agent…`
note line unchanged.

**Replacement content:**

```markdown
**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.1a complete; Step 1.1b next).

**What was just completed (Step 1.1a — Trait Interfaces):**
`crates/krax-types/src/state.rs` created: `StateError` enum (`Released` variant,
`#[non_exhaustive]`) and `State` trait (`get`, `set`, `snapshot`, `commit`, `root`) with
`Send + Sync` supertraits and module-scope object-safety assertion
(`const _: Option<&dyn State> = None;`).
`crates/krax-types/src/snapshot.rs` created: `Snapshot` trait
(`get`, `release(self: Box<Self>)`) with `Send + Sync` supertraits and module-scope
object-safety assertion.
`crates/krax-types/src/lib.rs` rewritten: `pub mod` declarations and flat `pub use`
re-exports (`State`, `StateError`, `Snapshot`).
`crates/krax-types/Cargo.toml` updated: `alloy-primitives` and `thiserror` added as
workspace-inherited deps.
`ARCHITECTURE.md` Step 1.1a heading ✅ and all four checkboxes marked `[x]`; Step 1.4
third checkbox updated (consuming `release` → `trybuild`/`compile_fail` test in Step 1.4).

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
1. 🔴 **Step 1.1b — Data Types.** Define `PendingTx`, `Block`, `RWSet`, `Journal` in
   `crates/krax-types/src/`. Follow ARCHITECTURE.md Step 1.1b exactly.

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
```

---

### Step 8 (edit): `AGENTS.md` — Changelog append

Append the following entry at the **bottom** of the `## Changelog` section, after the
Session 10 entry. Do not modify any existing entry.

```markdown

### Session 11 — Step 1.1a: Trait Interfaces
**Date:** 2026-05-09
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary:** Created `crates/krax-types/src/state.rs` (`StateError` with `Released` variant and
`#[non_exhaustive]`; `State` trait with five methods and `Send + Sync` supertraits; module-scope
object-safety assertion `const _: Option<&dyn State> = None;`). Created
`crates/krax-types/src/snapshot.rs` (`Snapshot` trait with `get` and consuming
`release(self: Box<Self>)`; `Send + Sync` supertraits; object-safety assertion). Rewrote
`crates/krax-types/src/lib.rs` with `pub mod` declarations and flat `pub use` re-exports.
Updated `crates/krax-types/Cargo.toml` with `alloy-primitives` and `thiserror` workspace deps.
Updated `ARCHITECTURE.md`: Step 1.1a heading ✅, all four checkboxes `[x]`, Step 1.4 third
checkpoint updated to reflect compile-fail semantics of consuming `release`. All decisions settled
in `docs/plans/step-1.1a-decisions.md` and cited by decision number in code comments.
**Commit suggestion:** `feat(types): define State, Snapshot, StateError traits — Step 1.1a`
```

---

## Verification steps

Run in order from the project root. Every command must pass before the step is considered done.

```bash
# 1. Build — confirms object-safety assertions fire correctly (trait-level compile check).
make build
# Expected: exits 0.

# 2. Lint — confirms no pedantic violations, missing_docs, or unwrap_used in new code.
make lint
# Expected: exits 0 with -D warnings.

# 3. Test — no new test code in this step, but must not regress.
make test
# Expected: exits 0.

# 4. Docs — confirms every public item has a doc comment.
cargo doc --workspace --no-deps
# Expected: exits 0. If missing_docs fires for any new public item, add or fix the
#           doc comment before proceeding.

# 5. Defensive HashMap/HashSet check — krax-types must not use HashMap or HashSet.
grep -E '(HashSet|HashMap)' crates/krax-types/src/*.rs
# Expected: no output (grep exits 1 = pass). Any match is a violation.

# 6. New files exist.
test -f crates/krax-types/src/state.rs    && echo "OK: state.rs"
test -f crates/krax-types/src/snapshot.rs && echo "OK: snapshot.rs"
# Expected: two "OK:" lines.

# 7. ARCHITECTURE.md edits verified.
grep "Step 1.1a.*✅"                                       ARCHITECTURE.md && echo "OK: Step 1.1a ✅"
grep "\[x\] \`crates/krax-types/src/state.rs\`"           ARCHITECTURE.md && echo "OK: state.rs checkbox"
grep "\[x\] \`crates/krax-types/src/snapshot.rs\`"        ARCHITECTURE.md && echo "OK: snapshot.rs checkbox"
grep "\[x\] \`StateError\` enum"                           ARCHITECTURE.md && echo "OK: StateError checkbox"
grep "\[x\] Add \`alloy-primitives\`"                      ARCHITECTURE.md && echo "OK: Cargo.toml checkbox"
grep -E "trybuild|compile_fail"                            ARCHITECTURE.md && echo "OK: Step 1.4 text updated"
# Expected: six "OK:" lines.

# 8. AGENTS.md updated.
grep "Step 1.1a complete"   AGENTS.md && echo "OK: Current State references Step 1.1a"
grep "Step 1.1b"            AGENTS.md && echo "OK: Current State names next step"
grep "Session 11"           AGENTS.md && echo "OK: Changelog Session 11 present"
# Expected: three "OK:" lines.

# 9. krax-types/Cargo.toml deps.
grep "alloy-primitives" crates/krax-types/Cargo.toml && echo "OK: alloy-primitives"
grep "thiserror"        crates/krax-types/Cargo.toml && echo "OK: thiserror"
# Expected: two "OK:" lines.

# 10. lib.rs re-exports present.
grep "pub mod state"    crates/krax-types/src/lib.rs && echo "OK: pub mod state"
grep "pub mod snapshot" crates/krax-types/src/lib.rs && echo "OK: pub mod snapshot"
grep "pub use state"    crates/krax-types/src/lib.rs && echo "OK: pub use state"
grep "pub use snapshot" crates/krax-types/src/lib.rs && echo "OK: pub use snapshot"
# Expected: four "OK:" lines.
```

---

## Definition of "Step 1.1a done"

- ✅ `crates/krax-types/src/state.rs` exists; contains `StateError` with `Released` variant and `#[non_exhaustive]`; contains `State` trait with exactly five methods (`get`, `set`, `snapshot`, `commit`, `root`); has `Send + Sync` supertraits; has the module-scope object-safety assertion.
- ✅ `crates/krax-types/src/snapshot.rs` exists; contains `Snapshot` trait with `get` and `release(self: Box<Self>)`; has `Send + Sync` supertraits; has the module-scope object-safety assertion.
- ✅ `crates/krax-types/src/lib.rs` contains `pub mod snapshot; pub mod state;` and `pub use snapshot::Snapshot; pub use state::{State, StateError};`.
- ✅ `crates/krax-types/Cargo.toml` `[dependencies]` block contains `alloy-primitives = { workspace = true }` and `thiserror = { workspace = true }`.
- ✅ `make build` exits 0 — object-safety assertions compile cleanly, confirming both traits are object-safe.
- ✅ `make lint` exits 0 — no missing docs, no pedantic violations, no `HashMap`/`HashSet` in `krax-types/src/`.
- ✅ `make test` exits 0.
- ✅ `cargo doc --workspace --no-deps` exits 0.
- ✅ `grep -E '(HashSet|HashMap)' crates/krax-types/src/*.rs` produces no output.
- ✅ `ARCHITECTURE.md` Step 1.1a heading has ✅; all four Step 1.1a checkboxes are `[x]`; Step 1.4 third checkbox describes a compile-fail test.
- ✅ `AGENTS.md` Current State reflects Step 1.1a complete and Step 1.1b as next; Changelog has Session 11 as the last entry.

---

## Open questions / coder follow-ups

**If `make lint` reports a pedantic warning on any new item:**
Read the lint name. If it's `module_name_repetitions` (e.g. `StateError` in module `state`):
this lint is in the `allow` list at workspace level — it should not fire. If it fires anyway,
add `#[allow(clippy::module_name_repetitions)]` at the item level with a comment
`// name is canonical across the codebase`.

**If `cargo doc` fails with a missing-docs error on a `const _` item:**
The object-safety constants are anonymous (`const _`) and are not public items; `missing_docs`
does not apply to them. If rustdoc complains, this is unexpected — investigate before adding
a doc comment (anonymous consts should not require one).

**If the circular module reference (`state.rs` imports `Snapshot`; `snapshot.rs` imports `StateError`) causes a compile error:**
Circular references within the same Rust crate are legal. Both `snapshot` and `state` are
sibling modules under the crate root. If the compiler complains about a cycle, check that
`lib.rs` declares both modules with `pub mod` before any inline `use` that crosses them.
In practice, Rust resolves intra-crate circular module references without issue.

**If `release(self: Box<Self>)` triggers an unexpected lint or compiler warning:**
`Box<Self>` is an explicitly allowed receiver type for object-safe traits. If an "arbitrary
self types" lint fires, confirm the Rust toolchain is 1.85+ (pinned to 1.95.0 in this
project). This receiver form has been stable since Rust 1.33.

**If an ARCHITECTURE.md str_replace fails due to no unique match:**
The em dash `—` in step headings is U+2014 (multi-byte). Confirm the str_replace contains
the exact character, not a hyphen-minus. Copy directly from the file rather than retyping.

---

## What this step does NOT do

- ❌ No `MptState` or any concrete `State` implementation — that is Step 1.3.
- ❌ No `PendingTx`, `Block`, `RWSet`, or `Journal` types — those are Step 1.1b.
- ❌ No real snapshot-isolation logic — the `Snapshot` trait is an interface; the
  isolation guarantee comes from the `MptState` implementation in Step 1.3.
- ❌ No real tests (beyond compile-time object-safety assertions) — Step 1.2 writes tests
  after Step 1.1b adds the data types.
- ❌ No `trybuild` infrastructure — the "after release" compile-fail test is Step 1.4's
  responsibility. This step only updates ARCHITECTURE.md text to describe the right test form.
- ❌ No `alloy-consensus` or `alloy-rpc-types` dependencies — those arrive with `PendingTx`
  in Step 1.1b (pending Context7 verification).
- ❌ No changes to any file outside `crates/krax-types/`, `ARCHITECTURE.md`, and `AGENTS.md`.
- ❌ No workspace-root `Cargo.toml` edits — both `alloy-primitives` and `thiserror` are already
  present in `[workspace.dependencies]`.

---

## Updates to other files in the same commit

All changes below land in the **same commit** as the new `state.rs` and `snapshot.rs`.

| File | Change |
|---|---|
| `crates/krax-types/src/lib.rs` | Rewrite: add `pub mod` declarations and flat `pub use` re-exports |
| `crates/krax-types/Cargo.toml` | `[dependencies]` block: replace empty comment with `alloy-primitives` and `thiserror` |
| `ARCHITECTURE.md` | Step 1.1a: all four `[ ]` → `[x]`; heading `✅` |
| `ARCHITECTURE.md` | Step 1.4: third checkbox updated (consuming `release` → `trybuild`/`compile_fail`) |
| `AGENTS.md` | Current State: full replacement reflecting Step 1.1a complete, Step 1.1b next |
| `AGENTS.md` | Changelog: Session 11 entry appended at the bottom |

---

## Commit suggestion

```
feat(types): define State, Snapshot, StateError traits — Step 1.1a

crates/krax-types/src/state.rs (new):
- StateError: Released variant, #[non_exhaustive], thiserror derive.
- State trait: get/set/snapshot/commit/root; Send + Sync supertraits.
- snapshot() returns Result<Box<dyn Snapshot>, StateError> (widened for
  Step 1.3 MDBX fallibility — no callers yet).
- Module-scope object-safety assertion: const _: Option<&dyn State> = None.

crates/krax-types/src/snapshot.rs (new):
- Snapshot trait: get + release(self: Box<Self>); Send + Sync supertraits.
- Consuming release: post-release reads are a compile error, not runtime.
- Module-scope object-safety assertion: const _: Option<&dyn Snapshot> = None.

crates/krax-types/src/lib.rs (rewrite):
- pub mod snapshot; pub mod state; + flat pub use re-exports.

crates/krax-types/Cargo.toml:
- alloy-primitives and thiserror added as workspace-inherited deps.

ARCHITECTURE.md:
- Step 1.1a: heading ✅, all four checkboxes [x].
- Step 1.4: third test checkpoint updated to reflect trybuild/compile_fail
  requirement (consuming release makes runtime StateError::Released unreachable).

AGENTS.md:
- Current State: Step 1.1a complete; Step 1.1b next. Snapshot::release note added.
- Changelog: Session 11 appended at the bottom.

All ten decisions settled in docs/plans/step-1.1a-decisions.md.
```

---

## Outcomes

> **To be filled in by the coder after execution.**
>
> Record: which verification steps passed on first attempt, which required a fix and why,
> any unexpected lint or compiler behavior, and the actual line counts / file sizes of the
> new files. Follow the pattern from `docs/plans/archive/step-0.9-readme.md § Outcomes`.

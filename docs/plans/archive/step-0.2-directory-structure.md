# Step 0.2 — Directory Structure

> **Plan status:** Ready for execution.
> **Phase:** 0 — Project Setup.
> **ARCHITECTURE.md reference:** Phase 0, Step 0.2.
> **Prerequisites:** Step 0.1 (Cargo Workspace Initialization) complete and committed. See `docs/plans/archive/step-0.1-cargo-workspace.md` for resolved versions and reth strategy.

---

## Purpose

Create the full `bin/*` and `crates/*` tree from AGENTS.md "Project Structure". Every directory listed in that tree gets created. Every `bin/*` and `crates/*` gets its own `Cargo.toml` and a minimal source stub. Empty directories listed in the tree (sub-modules under `krax-rwset`, `krax-state`; placeholder `docs/architecture/`, `docs/phase-notes/`) get a `.gitkeep`.

After this step, **`cargo build --workspace` succeeds**. That is the structural gate that proves the workspace TOML from Step 0.1 references real, parseable members.

This step does **not** add any logic, any workspace dependencies to per-crate `Cargo.toml` files, any tests, or any of the project-root files (Makefile, README, .gitignore, etc.) that live in later Phase 0 steps. Stay in lane.

---

## Decisions resolved before this plan was written

These are not open questions. They were settled in the planning session that produced this file. The coder does not re-litigate them — if the coder believes one is wrong, they stop and surface it before writing.

1. **Stub minimalism: truly empty.** Each `crates/*` ships with `Cargo.toml` + `src/lib.rs` containing only a crate-level `//!` doc comment. Each `bin/*` ships with `Cargo.toml` + `src/main.rs` containing only `fn main() {}`. No `pub const VERSION` style fake-public items. AGENTS.md Rule 5 ("every public item has a test before it lands") is satisfied trivially because there are no public items yet — a crate-level doc comment is documentation, not a public item.

2. **No workspace dependencies in per-crate `Cargo.toml`.** Each crate's `[dependencies]` table is empty in this step. Dependencies are added in the phase where the crate first uses them, with the addition justified in that phase's commit message per AGENTS.md Rule 10. Speculative dep additions are forbidden.

3. **All sub-module directories get created now.** The `static_/`, `profile/`, `conservative/` dirs under `krax-rwset/src/` and the `mpt/`, `lsm/` dirs under `krax-state/src/` (plus `worker/`, `journal/`, `commit/` under `krax-sequencer/src/`) get created with `.gitkeep`. This fulfills the "full tree" requirement and prevents structural changes from bleeding into later logic-heavy phases. The `mod` declarations themselves are NOT added to `lib.rs` yet — there's nothing in those directories for `mod` to reference, and adding empty `mod static_;` declarations against empty directories would either fail to compile or require placeholder `mod.rs` files we don't want yet. `mod` declarations land in the phase that fills the directory.

4. **Partial creation under `docs/`.** `docs/architecture/` and `docs/phase-notes/` get created with `.gitkeep` to match the AGENTS.md "Project Structure" tree. The `.md` stubs (`rwset-inference.md`, `speculation-model.md`) are NOT created — those documents describe real engineering decisions and will be written when there's content to capture. `docs/plans/` and `docs/plans/archive/` already exist; do not touch them.

5. **`scripts/` is NOT created in this step.** Per ARCHITECTURE.md, scripts are Step 0.6 territory (`devnet-up.sh` and `devnet-down.sh` get created there as placeholder no-ops). Same for `Makefile` (0.4), `.gitignore` and `.env.example` (0.5), `docker-compose.yml` (0.6), `contracts/` (0.7), `rustfmt.toml` and `clippy.toml` (0.8), `README.md` (0.9), and `LICENSE` (no specific step, but not 0.2).

6. **Workspace inheritance is mandatory.** Every per-crate `Cargo.toml` inherits version, edition, license, repository, and authors from `[workspace.package]` via `field.workspace = true`. No per-crate version drift. The exact template is given below.

7. **No `[lib]` or `[[bin]]` tables.** Cargo infers binaries from `bin/*/src/main.rs` and libraries from `crates/*/src/lib.rs`. Adding explicit `[[bin]]` or `[lib]` tables is unnecessary noise and is omitted.

---

## Library verification checklist

This step uses no external libraries. No Context7 lookups required. The only external "API surface" being used is Cargo's own workspace inheritance syntax, which is stable since Rust 1.64 (workspace inheritance) and unchanged in edition 2024.

---

## Files and directories to create

### Directories with `.gitkeep` only

These are structural placeholders. The `.gitkeep` is a zero-byte file (or convention file) that lets Git track an otherwise-empty directory. **Do not put anything else in these directories.**

```
crates/krax-rwset/src/static_/.gitkeep
crates/krax-rwset/src/profile/.gitkeep
crates/krax-rwset/src/conservative/.gitkeep
crates/krax-sequencer/src/worker/.gitkeep
crates/krax-sequencer/src/journal/.gitkeep
crates/krax-sequencer/src/commit/.gitkeep
crates/krax-state/src/mpt/.gitkeep
crates/krax-state/src/lsm/.gitkeep
docs/architecture/.gitkeep
docs/phase-notes/.gitkeep
```

### Per-crate `Cargo.toml` template — library crates

Apply this template to **every** `crates/krax-*/Cargo.toml`. Replace `<CRATE_NAME>` with the actual crate name (e.g. `krax-types`, `krax-mempool`).

```toml
[package]
name             = "<CRATE_NAME>"
version.workspace    = true
edition.workspace    = true
license.workspace    = true
repository.workspace = true
authors.workspace    = true

[dependencies]
# Intentionally empty. Dependencies are added in the phase where this crate
# first needs them, per AGENTS.md Rule 10.
```

### Per-crate `Cargo.toml` template — binary crates

Apply this template to **every** `bin/krax*/Cargo.toml`. Replace `<BIN_NAME>` with `kraxd`, `kraxctl`, or `kraxprover`.

```toml
[package]
name             = "<BIN_NAME>"
version.workspace    = true
edition.workspace    = true
license.workspace    = true
repository.workspace = true
authors.workspace    = true

[dependencies]
# Intentionally empty. Dependencies are added in the phase where this binary
# first needs them, per AGENTS.md Rule 10. (kraxd and kraxctl get clap and
# tracing in Step 0.3; kraxprover stays empty until Phase 23.)
```

### Library crate stubs — `crates/krax-*/src/lib.rs`

Each library crate gets exactly this content, with the crate name and one-line description swapped in. Pull the description from AGENTS.md "Project Structure" (the comment on each crate line is the canonical phrase).

Template:

```rust
//! <CRATE_NAME>: <one-line description from AGENTS.md "Project Structure">.
//!
//! See `AGENTS.md` "Project Structure" for this crate's role in the workspace.
```

Per-crate concrete content:

| Crate | `lib.rs` content |
|---|---|
| `krax-types` | `//! krax-types: core domain types and cross-crate traits.`<br>`//!`<br>`//! See ` AGENTS.md ` "Project Structure" for this crate's role in the workspace.` |
| `krax-config` | `//! krax-config: config loading and validation.` (+ same footer) |
| `krax-mempool` | `//! krax-mempool: pending tx pool and lookahead window.` (+ footer) |
| `krax-rwset` | `//! krax-rwset: read/write set inference engine.` (+ footer) |
| `krax-sequencer` | `//! krax-sequencer: speculative execution coordinator.` (+ footer) |
| `krax-state` | `//! krax-state: state backend (pluggable: V1 MPT, V2 LSM).` (+ footer) |
| `krax-execution` | `//! krax-execution: revm wrapper and gas accounting.` (+ footer) |
| `krax-batcher` | `//! krax-batcher: batch builder and L1 poster.` (+ footer) |
| `krax-prover` | `//! krax-prover: ZK proof generation (Phase 23+).` (+ footer) |
| `krax-rpc` | `//! krax-rpc: JSON-RPC server (eth_* and krax_* namespaces).` (+ footer) |
| `krax-metrics` | `//! krax-metrics: Prometheus metric definitions.` (+ footer) |

The footer line (`//! See ...`) is identical on every crate.

### Binary crate stubs — `bin/krax*/src/main.rs`

Each binary crate gets exactly this content. No doc comment — `main.rs` is an entrypoint, not a public API.

```rust
fn main() {}
```

Step 0.3 fills `kraxd/src/main.rs` with version printing and `kraxctl/src/main.rs` with the `clap` `--help` skeleton. `kraxprover/src/main.rs` stays as `fn main() {}` until Phase 23. **Do NOT add any of that content in Step 0.2.**

### `.gitkeep` content

Zero bytes. Or a single newline if the editor insists. The file's existence is the entire point.

---

## Full directory tree this step creates

For reference. Compare against AGENTS.md "Project Structure" — they should match (minus the items deferred to later steps per Decision 5).

```
krax/
├── bin/
│   ├── kraxd/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── kraxctl/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── kraxprover/
│       ├── Cargo.toml
│       └── src/main.rs
│
├── crates/
│   ├── krax-types/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── krax-config/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── krax-mempool/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── krax-rwset/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── static_/.gitkeep
│   │       ├── profile/.gitkeep
│   │       └── conservative/.gitkeep
│   ├── krax-sequencer/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── worker/.gitkeep
│   │       ├── journal/.gitkeep
│   │       └── commit/.gitkeep
│   ├── krax-state/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── mpt/.gitkeep
│   │       └── lsm/.gitkeep
│   ├── krax-execution/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── krax-batcher/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── krax-prover/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── krax-rpc/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── krax-metrics/
│       ├── Cargo.toml
│       └── src/lib.rs
│
└── docs/
    ├── architecture/.gitkeep
    ├── phase-notes/.gitkeep
    └── plans/                 ← already exists, do NOT touch
        └── archive/           ← already exists, do NOT touch
```

**Total file count this step creates:**
- 14 `Cargo.toml` files (3 bin + 11 crates)
- 3 `main.rs` stubs
- 11 `lib.rs` stubs
- 10 `.gitkeep` files (8 sub-module dirs + 2 docs dirs)

---

## Verification steps

Run in order from the project root:

```bash
# 1. Confirm cargo can resolve and build the empty workspace.
cargo build --workspace
# Expected: SUCCESS. All members compile (each is essentially an empty
# library or `fn main() {}` binary). Build artifacts land in target/.
# This is the load-bearing gate for Step 0.2.

# 2. Confirm cargo metadata sees every workspace member.
cargo metadata --no-deps --format-version 1 | jq '.workspace_members | length'
# Expected: 14 (3 bins + 11 lib crates).

# 3. Confirm every crate uses workspace inheritance.
grep -rL "workspace = true" bin/*/Cargo.toml crates/*/Cargo.toml
# Expected: empty output. Every per-crate Cargo.toml must contain at least
# one "workspace = true" line. Any file listed here is missing inheritance.

# 4. Confirm no per-crate Cargo.toml has a [dependencies] entry.
grep -A1 "^\[dependencies\]$" bin/*/Cargo.toml crates/*/Cargo.toml | grep -v "^--$" | grep -v "Intentionally empty" | grep -v "^\[dependencies\]$" | grep -v "AGENTS.md" | grep -v "Phase 23" | grep -v "Step 0.3"
# Expected: empty. Per-crate [dependencies] tables must contain only the
# placeholder comment, no actual deps.

# 5. Confirm every directory in the AGENTS.md tree exists.
for d in bin/kraxd bin/kraxctl bin/kraxprover \
         crates/krax-types crates/krax-config crates/krax-mempool \
         crates/krax-rwset crates/krax-rwset/src/static_ \
         crates/krax-rwset/src/profile crates/krax-rwset/src/conservative \
         crates/krax-sequencer crates/krax-sequencer/src/worker \
         crates/krax-sequencer/src/journal crates/krax-sequencer/src/commit \
         crates/krax-state crates/krax-state/src/mpt crates/krax-state/src/lsm \
         crates/krax-execution crates/krax-batcher crates/krax-prover \
         crates/krax-rpc crates/krax-metrics \
         docs/architecture docs/phase-notes; do
    [ -d "$d" ] || echo "MISSING: $d"
done
# Expected: empty output. Any "MISSING" line is a directory the coder forgot.

# 6. Confirm .gitkeep exists in every empty directory.
for d in crates/krax-rwset/src/static_ crates/krax-rwset/src/profile \
         crates/krax-rwset/src/conservative \
         crates/krax-sequencer/src/worker crates/krax-sequencer/src/journal \
         crates/krax-sequencer/src/commit \
         crates/krax-state/src/mpt crates/krax-state/src/lsm \
         docs/architecture docs/phase-notes; do
    [ -f "$d/.gitkeep" ] || echo "MISSING .gitkeep: $d"
done
# Expected: empty output.

# 7. Confirm `cargo doc --workspace --no-deps` produces no warnings about
#    missing crate-level documentation.
cargo doc --workspace --no-deps 2>&1 | grep -i "missing.*documentation" || echo "OK: no missing-docs warnings"
# Expected: "OK: no missing-docs warnings". The crate-level //! doc comment
# on every lib.rs prevents this warning.

# 8. Confirm Step 0.2 did NOT touch root-level files reserved for later steps.
for f in Makefile README.md .gitignore .env.example docker-compose.yml \
         rustfmt.toml clippy.toml LICENSE contracts; do
    [ -e "$f" ] && echo "UNEXPECTED: $f exists (belongs to a later step)"
done
# Expected: empty output. If any of these exist, the coder violated scope.
# Note: scripts/ may exist if it was created earlier; that's fine, but it
# should not have new content from this step.
```

### Definition of "Step 0.2 done"

- ✅ All 14 `Cargo.toml` files exist with workspace inheritance and empty `[dependencies]`.
- ✅ All 11 `lib.rs` files exist with the crate-level `//!` doc comment per the table above.
- ✅ All 3 `main.rs` files exist with `fn main() {}` and nothing else.
- ✅ All 10 `.gitkeep` files exist in their specified directories.
- ✅ `cargo build --workspace` succeeds with no errors.
- ✅ `cargo metadata --no-deps` reports exactly 14 workspace members.
- ✅ `cargo doc --workspace --no-deps` produces no missing-docs warnings.
- ✅ No root-level files reserved for later steps were created.
- ✅ `docs/plans/` and `docs/plans/archive/` are untouched.

---

## Open questions / coder follow-ups

None. This step has no `cargo search`-style unknowns. Every file's content is fully specified above. If the coder finds an ambiguity, **stop and surface it** rather than guessing — the planner will resolve it.

The two situations where the coder should stop and ask:

1. **`cargo build --workspace` fails** for any reason other than a typo the coder can fix in seconds. A failing build at this step likely means a workspace member path doesn't match what's listed in the root `Cargo.toml`, or workspace inheritance is malformed. Fix typos directly; surface anything stranger.

2. **AGENTS.md "Project Structure" disagrees with this plan** about which directories or sub-modules exist. The plan was written against AGENTS.md as it stands at Step 0.1 commit. If AGENTS.md has been edited since, the coder should stop and ask which is the source of truth.

---

## What this step does NOT do

Stay in lane. Out of scope for Step 0.2:

- ❌ Add any `mod` declarations to `lib.rs` files. Sub-module directories have `.gitkeep` only; their `mod` declarations land in the phase that fills them with code.
- ❌ Add any workspace dependencies to per-crate `Cargo.toml` files. Empty `[dependencies]` everywhere. Even "obvious" ones like `thiserror` wait until the crate first uses them.
- ❌ Add any tests. No `#[cfg(test)]` blocks. No `tests/` directories. No `dev-dependencies`.
- ❌ Write `Makefile`, `.gitignore`, `.env.example`, `docker-compose.yml`, `rustfmt.toml`, `clippy.toml`, `README.md`, or `LICENSE`. Each has a designated step (0.4 / 0.5 / 0.6 / 0.8 / 0.9 / future).
- ❌ Initialize `contracts/`. That's Step 0.7 (`forge init`).
- ❌ Create `scripts/` or any shell scripts. That's Step 0.6.
- ❌ Create `docs/rwset-inference.md` or `docs/speculation-model.md` stubs. Those wait for real engineering content.
- ❌ Update AGENTS.md "Current State" to claim any non-Step-0.2 work is done. Only Step 0.2 changes go in this commit.
- ❌ Run `cargo fmt` against the new files (rustfmt.toml doesn't exist yet — Step 0.8). Default formatting is fine for this step; the formatting step will normalize everything later.
- ❌ Run `cargo clippy` (clippy.toml doesn't exist yet — Step 0.8). Empty crates wouldn't surface meaningful lints anyway.
- ❌ Add `[profile.*]` sections to the workspace `Cargo.toml`. Deferred until real code exists to optimize.

---

## Updates to other files in the same commit

Per the same-commit discipline established in Step 0.1 (no leaving stale references for later cleanup):

### `ARCHITECTURE.md`

Mark Step 0.2 complete. Change:

```markdown
### Step 0.2 — Directory Structure
- [ ] Create the full tree from AGENTS.md "Project Structure"
- [ ] Add a `.gitkeep` file in each empty directory
- [ ] Each `bin/*` and `crates/*` gets its own `Cargo.toml`
```

to:

```markdown
### Step 0.2 — Directory Structure ✅
- [x] Create the full tree from AGENTS.md "Project Structure"
- [x] Add a `.gitkeep` file in each empty directory
- [x] Each `bin/*` and `crates/*` gets its own `Cargo.toml`
```

### `AGENTS.md`

Update "Current State" to reflect Step 0.2 done and Step 0.3 next. Replace the existing block with:

```markdown
**Current Phase:** Phase 0 — Project Setup (Steps 0.1 and 0.2 complete, Step 0.3 next)

**What was just completed:**
- **Step 0.2 — Directory Structure done.** Full `bin/*` and `crates/*` tree created per AGENTS.md "Project Structure". 14 workspace members total (3 binaries + 11 library crates), each with its own `Cargo.toml` using workspace inheritance for version/edition/license/repository/authors. Per-crate `[dependencies]` tables are intentionally empty — dependencies get added in the phase where each crate first uses them per AGENTS.md Rule 10. Sub-module directories (`static_/`, `profile/`, `conservative/`, `worker/`, `journal/`, `commit/`, `mpt/`, `lsm/`) created with `.gitkeep`; their `mod` declarations land in the phase that fills them. `docs/architecture/` and `docs/phase-notes/` created as `.gitkeep` placeholders. `cargo build --workspace` succeeds.
- (Carry forward: Step 0.1 — Cargo workspace initialization done. revm 38, reth-* git rev `02d1776786abc61721ae8876898ad19a702e0070`, jsonrpsee 0.26, etc. See archived plan for full version table.)

**What to do next (in order):**
1. 🔴 **Step 0.3 — Minimal Entrypoint.** Fill `bin/kraxd/src/main.rs` to print `krax vX.Y.Z` (read version from `CARGO_PKG_VERSION`) and exit cleanly. Fill `bin/kraxctl/src/main.rs` with a `clap` derive skeleton supporting `--help` only. `bin/kraxprover/src/main.rs` stays as `fn main() {}` until Phase 23.
2. Step 0.4 — Makefile.
3. Steps 0.5 through 0.9 in order, per ARCHITECTURE.md.

**Blockers:**
- Repository URL is a placeholder (`https://github.com/krax-labs/krax`). Replace before V1.0 branding. Not a blocker for Phase 0 work.
- Project name not finalized. "Krax" is a working name. Search-replace before mainnet branding (V1.1 concern).

**Notes:**
- 14 workspace members exist as compilable stubs. No business logic yet.
- The reth-as-library POC code is at `~/Projects/evm-state-poc/` and is intentionally NOT brought into the Krax tree.
- Do NOT start any sequencer or RW-set work in Phase 0. That's Phase 1+.
- Every external library use MUST be Context7-verified per the Library Verification Protocol section. No exceptions.
- `reth-*` git rev must be updated periodically as reth main advances. When upgrading, change ALL reth-* entries to the same new rev in one commit.
```

Append to "Changelog":

```markdown
### Session 3 — Step 0.2: Directory Structure
**Date:** <COMMIT_DATE>
**Agent:** <AGENT_IDENT>
**Summary:** Created the full `bin/*` and `crates/*` tree from AGENTS.md "Project Structure". 14 workspace members total. Every per-crate `Cargo.toml` uses workspace inheritance and has an empty `[dependencies]` table per the no-speculative-deps rule. Library crates have crate-level `//!` doc comments; binary crates have `fn main() {}` stubs. Sub-module directories created with `.gitkeep`; `mod` declarations deferred to the phase that fills each. `docs/architecture/` and `docs/phase-notes/` created as `.gitkeep` placeholders. `cargo build --workspace` succeeds. Out of scope: Makefile, gitignore, contracts/, scripts/, all root-level config (later Phase 0 steps).
**Commit suggestion:** `chore(workspace): create directory structure — Step 0.2`
```

---

## Commit suggestion

Conventional commit format per AGENTS.md "Workflow & Conventions":

```
chore(workspace): create directory structure — Step 0.2

- Create full bin/* and crates/* tree per AGENTS.md "Project Structure".
- 14 workspace members total: 3 binaries (kraxd, kraxctl, kraxprover) and
  11 library crates (krax-types, krax-config, krax-mempool, krax-rwset,
  krax-sequencer, krax-state, krax-execution, krax-batcher, krax-prover,
  krax-rpc, krax-metrics).
- Every per-crate Cargo.toml uses workspace inheritance for version, edition,
  license, repository, and authors. Per-crate [dependencies] tables are
  intentionally empty per AGENTS.md Rule 10 — deps are added in the phase
  where each crate first uses them, with justification in that commit.
- Library crates have crate-level //! doc comments referring readers to
  AGENTS.md "Project Structure" for role definitions. Binary crates have
  `fn main() {}` stubs (filled in Step 0.3 for kraxd/kraxctl).
- Sub-module directories created with .gitkeep:
    crates/krax-rwset/src/{static_,profile,conservative}/
    crates/krax-sequencer/src/{worker,journal,commit}/
    crates/krax-state/src/{mpt,lsm}/
  `mod` declarations are NOT added yet — they land in the phase that fills
  each directory with code.
- docs/architecture/ and docs/phase-notes/ created as .gitkeep placeholders
  per the AGENTS.md tree. .md stubs (rwset-inference.md, speculation-model.md)
  wait for real engineering content.

Verification:
- cargo build --workspace succeeds.
- cargo metadata --no-deps reports 14 workspace members.
- cargo doc --workspace --no-deps produces no missing-docs warnings.
- No root-level files reserved for later Phase 0 steps were created.

Implements ARCHITECTURE.md Phase 0 Step 0.2.
```

---

## After this step

Next: Step 0.3 — Minimal Entrypoint. Fill `kraxd` and `kraxctl` `main.rs` with their initial behaviors (version printing and `clap --help` respectively). `kraxd` becoming runnable via `cargo run --bin kraxd` is the "first thing actually does something" moment, even if that something is just printing a version.

Once Step 0.2 is committed and verified, this plan file moves to `docs/plans/archive/step-0.2-directory-structure.md` per the archive convention. The coder appends a brief `## Outcomes` section before archiving — even if the outcomes are "everything went per plan, no surprises," writing that explicitly is the signal that the step was actually verified.

---

## Outcomes

- **`cargo build --workspace` succeeded on first attempt.** All 14 members (3 binaries + 11 library crates) compiled cleanly with empty stubs in 1.12 s. No workspace inheritance errors, no missing member path issues.
- **No surprises.** Every file matched the plan exactly — Cargo.toml templates, lib.rs doc comments, main.rs stubs, .gitkeep placements. Zero deviations from specified content.
- **`.gitignore` pre-existence noted.** The scope check flagged `.gitignore` as "UNEXPECTED," but it was committed in Step 0.1 — not created in this step. Not a violation.
- **All verification checks passed:** 14 workspace members confirmed via `cargo metadata`, all directories and `.gitkeep` files present, no missing-docs warnings from `cargo doc --workspace --no-deps`, no per-crate `[dependencies]` entries beyond the placeholder comment.

# Step 1.3.5 Plan — Coverage Tooling

**Date:** 2026-05-13
**Status:** ⏳ Revised 2026-05-13 after coder halt; ready for coder re-dispatch.
**Decisions doc:** [`docs/plans/step-1.3.5-decisions.md`](step-1.3.5-decisions.md) — all 13 decisions answered; Decision 2 REVISED 2026-05-13 (inline-marker mechanism replaced with file splits + path-based exclusion). Do NOT re-litigate.
**Commit boundary:** SINGLE commit (Decision 10).

**Revision history.** Original dispatch 2026-05-13 halted at pre-flight LVP Q4 (`// grcov-excl-start` / `// grcov-excl-stop` not honored by cargo-llvm-cov). Maintainer chose Option 1 of four (file splits + path-based exclusion only). Decisions doc Decision 2 has been revised in place with a Revision Note; this plan file has been rewritten so Steps 2 & 3 are now file-split steps instead of inline-marker steps. The full halt diagnostic and four-option Open Question are preserved in the "Revision history" appendix at the bottom of this file for audit.

---

## Critical: do not run `git commit`

Per AGENTS.md Workflow & Conventions (Git subsection, 2026-05-11 policy), the coder's job ends when:

1. `make build`, `make lint`, `make test`, `make test-integration`, and `make coverage` all exit `0` against the final source tree.
2. Every row in the Commit 1 Verification Suite below is green.
3. The `## Outcomes` section at the bottom of Commit 1 is filled in (files changed, verification table results, deviations, Context7 query results, proposed final commit message, notes for maintainer, Phase 1 Gate coverage-line final status with measured per-crate percentages).

The coder MAY use `git add` / `git status` to verify which files are touched. The coder MUST NOT run `git commit`. The maintainer reviews Outcomes and runs `git commit` themselves. This is the human-in-the-loop checkpoint between code production and code landing.

---

## Pre-flight — Library Verification Protocol (tier-2)

`cargo-llvm-cov` is mature, well-documented, low blast radius. Per the revised LVP checklist in the decisions doc, Q1–Q3 from the original coder run all PASSED and DO NOT need to be re-run unless the re-dispatched coder wants fresh provenance. Q4 (inline-marker syntax) is OBSOLETE — the file-split approach removes the need for inline-marker syntax entirely. A NEW Q4 (file-split-mechanism sanity check on `--ignore-filename-regex` regex flavor) is added.

**Pre-declared expectations + fallback policy:** If Context7 returns unrelated content or errors, fall back to the upstream documentation site (`taiki-e.github.io/cargo-llvm-cov`) or the `cargo install --list` on-disk source. Document the fallback in Outcomes. Do NOT silently adapt — if a deviation surfaces, surface it in Outcomes and (for the new Q4) as an Open Question.

### Q1 — `cargo-llvm-cov` CLI surface (PASSED in original run; re-run optional)

- **Context7 id (expected):** `/taiki-e/cargo-llvm-cov`.
- **Flags to confirm present and behaving as documented:**
  - `--workspace` — measure every workspace member.
  - `--features <name>` — forwarded to the underlying `cargo test` invocation.
  - `--summary-only` — terminal summary, no per-file output.
  - `--html` — emit HTML report (default location `target/llvm-cov/html/`).
  - `--fail-under-lines <N>` — exit non-zero if line coverage drops below `N` (used at `85` per Decision 5).
  - `--ignore-filename-regex <regex>` — exclude files whose path matches.
  - `cargo llvm-cov report --per-crate` — per-crate breakdown for Phase 1 Gate verification (Decision 4.3).
- **Original-run result:** PASSED, all six flags + `report --per-crate` confirmed via Context7 `/taiki-e/cargo-llvm-cov` (cited in the Revision History appendix). Re-run optional.

### Q2 — Doctest interaction (PASSED in original run; re-run optional)

- **Confirm:** `--doctests` is opt-in (default off).
- **Confirm:** `compile_fail` doctests do NOT contribute coverage measurement under any flag.
- **Decision 8:** do not pass `--doctests`. This query is informational — surface in Outcomes if the default has changed.
- **Original-run result:** PASSED, `--doctests` is opt-in and unstable (nightly-only).

### Q3 — `--features integration` interaction (PASSED in original run; re-run optional)

- **Confirm:** `cargo llvm-cov` forwards `--features <name>` to the underlying `cargo test` invocation such that `[[test]] required-features = ["integration"]` entries in `Cargo.toml` are honored.
- **Load-bearing site:** `crates/krax-state/Cargo.toml` lines 40–43 (the `restart` integration test).
- **Expectation:** running `cargo llvm-cov --workspace --features integration` compiles and runs `tests/restart.rs`. Decision 9 item 4 verifies this empirically via the with/without delta.
- **Original-run result:** PASSED, `--features` forwarded to underlying `cargo test`; `required-features` honored.

### Q4 (NEW) — `--ignore-filename-regex` regex flavor sanity check

- **Confirm:** `cargo-llvm-cov`'s `--ignore-filename-regex` accepts standard Rust `regex` crate syntax (per cargo-llvm-cov's CLI docs). Specifically that the literal `\.` escape works as expected AND that `|` at the top level (outside parens) is interpreted as alternation between full path patterns rather than only between sub-expressions inside a parenthesized group.
- **Reason:** the revised Makefile recipe uses `'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'` — two top-level alternations joined with `|`. If the tool's regex flavor doesn't support top-level `|` alternation in CLI args (some shell quoting / tool argument parsing edge case), the second alternation would silently not match. Cheap sanity check: 1–2 Context7 retrievals + a smoke test (Verification Suite row 9 confirms via inspection of the HTML output).
- **If Q4 yields ANY deviation from Rust `regex` crate syntax for `--ignore-filename-regex`:** STOP. Surface as an Open Question in Outcomes; do NOT adapt the regex inline. The maintainer picks between (a) splitting the regex across two `--ignore-filename-regex` flags (the tool may allow the flag to be repeated), (b) finding a different regex shape that works in the tool's flavor, or (c) something else. Do NOT silently substitute.

### Q5 (OBSOLETE) — inline-marker syntax (replaced by file splits)

The original Q4 (inline-marker syntax) is no longer relevant after the Decision 2 revision. File splits + path-based exclusion replace the inline-marker mechanism entirely. No inline markers are added to any source file in this commit. (Background: the original Q4 fired a STOP condition — cargo-llvm-cov does not honor `// grcov-excl-start` / `// grcov-excl-stop`; only the unstable `#[cfg_attr(coverage_nightly, coverage(off))]` attribute is documented, which is incompatible with the workspace's pinned stable toolchain. See the Revision History appendix.)

---

## Commit 1 (only commit) — `chore(coverage): add make coverage target — Step 1.3.5`

### Purpose

Ship the Phase 1 coverage-measurement story end-to-end: install-via-`cargo install` `cargo-llvm-cov` (no workspace dep change), add a hard-thresholded `make coverage` target that runs the workspace under `--features integration` with `--fail-under-lines 85`, apply path-based exclusions to the six data-only/glue items named in Decision 3 via two file splits (`JournalEntry` → new `journal_entry.rs`; `Slots` + `SlotsTableSet` → new `mpt/slots.rs`) so the exclude regex sweeps four files cleanly, fold in the Decision-13 `_USES_DATABASE_ERROR_FOR_MAP_ERR` cleanup in `mpt/mod.rs`, reconcile AGENTS.md Rule 5's coverage line with the Phase 1 Gate, close the four ARCHITECTURE.md Step 1.3.5 checkboxes, and conditionally close the Phase 1 Gate coverage line per measured percentages.

### Coder micro-decision — Phase 1 Gate coverage-line treatment

ARCHITECTURE.md lines 207–211 currently show ALL Phase 1 Gate items with `✅` typographically — the 1.3a/1.3b convention is that those `✅` glyphs are **goal-state markers**, not literal status indicators (Current State note at AGENTS.md lines 755–757; 1.3b plan precedent).

**Recommendation (apply unless inconsistent with the rest of Phase 1 Gate at landing time):** Flip the coverage line specifically (line 211, `Coverage on krax-types and krax-state is >85%`) to **literal measured status**:

- If both crates measure ≥85%: leave the `✅` AND append a brief inline note recording the measured numbers, e.g. `✅ Coverage on krax-types and krax-state is >85% (measured 2026-05-13: krax-types XX.X%, krax-state YY.Y%)`.
- If either crate is below 85%: replace the `✅` with `⏳` on that line, append the same measured-numbers note, and record the gap explicitly in Outcomes for 1.4/1.5 to close.

**Counter-reading preserved:** the 1.3a/1.3b convention argues for leaving line 211 as a goal-state `✅` regardless of measurement, with the measured numbers living ONLY in Outcomes (not in ARCHITECTURE.md). If the coder finds at landing that the other four lines (207–210) are still being treated as pure goal-state markers (i.e. nothing has yet differentiated them by literal status), the coder MAY apply the same convention to line 211 — leave the `✅`, keep numbers in Outcomes only. The constraint: pick ONE treatment and apply it consistently within line 211. Whichever the coder picks, **the measured per-crate percentages MUST appear in Outcomes regardless** (Decision 12.2).

### Execution Steps

Each step has a target file, an `Old:` block (exact pre-edit content), a `New:` block (exact post-edit content), and a rationale. The coder applies these in this order so that `make build` / `make lint` / `make test` stay green at every intermediate point (Decision 13 cleanup is independent and can move within the commit).

---

#### Step 1 — Add `coverage` target to `Makefile`

**File:** `/Users/johnnysquardo/Projects/krax/Makefile`

**Old (line 3):**
```makefile
.PHONY: help build test test-integration lint run fmt clean
```

**New:**
```makefile
.PHONY: help build test test-integration lint run fmt clean coverage
```

**Old (`help` recipe, lines 5–14):**
```makefile
help:
	@echo "Krax — available make targets:"
	@echo ""
	@echo "  build            cargo build --workspace --release"
	@echo "  test             cargo test --workspace"
	@echo "  test-integration cargo test --workspace --features integration"
	@echo "  lint             cargo clippy --workspace --all-targets -- -D warnings"
	@echo "  run              cargo run --bin kraxd"
	@echo "  fmt              cargo fmt --all"
	@echo "  clean            cargo clean; rm -rf data/"
```

**New:**
```makefile
help:
	@echo "Krax — available make targets:"
	@echo ""
	@echo "  build            cargo build --workspace --release"
	@echo "  test             cargo test --workspace"
	@echo "  test-integration cargo test --workspace --features integration"
	@echo "  lint             cargo clippy --workspace --all-targets -- -D warnings"
	@echo "  coverage         cargo llvm-cov --workspace --features integration --fail-under-lines 85 (HTML at target/llvm-cov/html/)"
	@echo "  run              cargo run --bin kraxd"
	@echo "  fmt              cargo fmt --all"
	@echo "  clean            cargo clean; rm -rf data/"
```

**Old (end of file, after `clean` recipe at line 36):**
```makefile
clean:
	@cargo clean
	@rm -rf data/
```

**New (append the new `coverage` recipe after `clean`; do NOT change the existing `clean` recipe):**
```makefile
clean:
	@cargo clean
	@rm -rf data/

coverage:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || { \
		echo "cargo-llvm-cov is not installed."; \
		echo "Install with: cargo install cargo-llvm-cov   (or, on macOS: brew install cargo-llvm-cov)"; \
		exit 1; \
	}
	@$(MAKE) build
	@cargo llvm-cov --workspace --features integration --html --ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs' --fail-under-lines 85
	@cargo llvm-cov report --per-crate --ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'
	@echo "HTML report: target/llvm-cov/html/index.html"
```

**Rationale:**

- Decision 1 picks `cargo-llvm-cov`; Decision 11 says no `Cargo.toml` entry (it's a `cargo install` binary).
- Decision 4.5 — pre-flight install hint via `command -v`. Exits `1` with a one-line install command (linux/macos covered) when missing.
- Decision 4 edge case — `$(MAKE) build` before the coverage run so a compile failure surfaces as a build failure, not a coverage failure.
- Decision 4.4 — `--features integration` always-on (single target shape; the only meaningful integration-gated tests today are `krax-state/tests/restart.rs`).
- Decision 5 — `--fail-under-lines 85` hard-coded in the recipe, not in an env var, not in a separate script.
- Decision 4.1 + 4.2 — `--html` emits to the tool default `target/llvm-cov/html/` (already gitignored via `target/`); the second `report --per-crate` invocation prints the terminal per-crate summary on top of the run that already happened. The trailing `@echo` tells the developer where the HTML is.
- Decision 2 (REVISED 2026-05-13) — path-based exclusion via `--ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'`. The same regex is repeated on the `report --per-crate` call so per-crate numbers don't re-include the excluded paths. Two top-level alternations joined with `|`: the first matches three files in `krax-types/src/` (`block.rs`, `tx.rs`, `journal_entry.rs`); the second matches `crates/krax-state/src/mpt/slots.rs`. Per the new Q4 sanity check, this regex syntax is standard Rust `regex` crate (cargo-llvm-cov's documented flavor).
- The recipe pattern (`@command…`) matches the existing one-line-per-action style; the `$(MAKE) build` step is the only multi-step deviation, justified by the Decision-4 edge-case requirement.
- Note: `Block::new` (1.1b, trivial struct-literal constructor) and any future trivial accessors on `PendingTx` / `MempoolEntry` / `JournalEntry` / `Slots` / `SlotsTableSet` are swept into the whole-file exclusion. This is the Decision 3 acknowledged coupling risk — now extended to four files instead of two. If a future step adds a non-trivial method to any of these excluded files, that step must either remove the file from this regex or move the new method out of the excluded file.

---

#### Step 2 — Split `JournalEntry` into new file `crates/krax-types/src/journal_entry.rs`

**Files touched:** three.

1. **`crates/krax-types/src/lib.rs`** — add `pub mod journal_entry;` module declaration (alphabetically between `journal` and `rwset`). The existing `pub use journal::{Journal, JournalEntry};` re-export line is UNCHANGED — `JournalEntry` continues to be reachable through `journal.rs`'s re-export (see file 2 below), so the workspace-external API `krax_types::JournalEntry` stays byte-identical.
2. **`crates/krax-types/src/journal.rs`** — remove the `JournalEntry` struct definition + its doc comment + its `#[derive(...)]` line. Add `pub use crate::journal_entry::JournalEntry;` at the top of the file (below the existing `use` statement). `Journal` + `impl Journal` + the `compile_fail` doctest on `Journal::discard` are all unchanged.
3. **`crates/krax-types/src/journal_entry.rs`** — NEW FILE. Contains `pub struct JournalEntry` with its derives + doc comment + a brief `//!` crate-file-level docblock explaining what the file is for.

**Edit 1 — `crates/krax-types/src/lib.rs`**

**Old (lines 7–13 — the `pub mod` block):**
```rust
pub mod block;
pub mod journal;
pub mod rwset;
pub mod snapshot;
pub mod state;
pub mod tx;
```

**New:**
```rust
pub mod block;
pub mod journal;
pub mod journal_entry;
pub mod rwset;
pub mod snapshot;
pub mod state;
pub mod tx;
```

The `pub use journal::{Journal, JournalEntry};` re-export line (currently around line 18) is UNCHANGED.

**Edit 2 — `crates/krax-types/src/journal.rs`**

**Old (full current file contents, lines 1–24 — from the file's top through the closing `}` of `JournalEntry`):**
```rust
//! Worker journal — in-memory record of speculative writes.

use alloy_primitives::B256;

use crate::state::{State, StateError};

/// A single write recorded in a worker's speculative journal.
///
/// `old` uses `B256::ZERO` for "slot was unset" — the EVM storage model has no
/// distinct "absent" state; SLOAD on an unset slot returns `B256::ZERO`. This
/// avoids `Option<B256>` and the attendant unwrapping in `discard`. The EVM
/// gas refund model (EIP-2200 "original value") is tracked separately by revm's
/// own journal inside the EVM executor; Krax's `JournalEntry` only needs to
/// know what value to restore if this tx is discarded.
/// See step-1.1b-decisions.md Decision 8.
#[derive(Debug, PartialEq, Eq)]
pub struct JournalEntry {
    /// Storage slot written.
    pub slot: B256,
    /// Value of the slot before this write; `B256::ZERO` if the slot was unset.
    pub old: B256,
    /// Value written to the slot.
    pub new: B256,
}
```

**New (replace the above with the lines below; the rest of `journal.rs` — `Journal` struct, `impl Journal`, and the `compile_fail` doctest — is UNCHANGED):**
```rust
//! Worker journal — in-memory record of speculative writes.
//!
//! `JournalEntry` (the per-write record) lives in the sibling [`journal_entry`]
//! module and is re-exported from here for crate-external callers. The split
//! keeps `journal.rs` focused on `Journal` + `impl Journal` (the logic surface)
//! and isolates the data-only `JournalEntry` so coverage measurement reflects
//! exercise of real logic. See step-1.3.5-decisions.md Decision 2 (revised).

use alloy_primitives::B256;

pub use crate::journal_entry::JournalEntry;

use crate::state::{State, StateError};
```

**Edit 3 — NEW FILE `crates/krax-types/src/journal_entry.rs`**

Create this file with the following contents verbatim:
```rust
//! [`JournalEntry`] — per-write record in a worker's speculative journal.
//!
//! Split out of `journal.rs` so the data-only struct can be excluded from
//! coverage measurement via path-based `--ignore-filename-regex` without
//! also excluding `Journal::apply` / `Journal::discard` (the logic surface).
//! See step-1.3.5-decisions.md Decision 2 (revised).

use alloy_primitives::B256;

/// A single write recorded in a worker's speculative journal.
///
/// `old` uses `B256::ZERO` for "slot was unset" — the EVM storage model has no
/// distinct "absent" state; SLOAD on an unset slot returns `B256::ZERO`. This
/// avoids `Option<B256>` and the attendant unwrapping in `discard`. The EVM
/// gas refund model (EIP-2200 "original value") is tracked separately by revm's
/// own journal inside the EVM executor; Krax's `JournalEntry` only needs to
/// know what value to restore if this tx is discarded.
/// See step-1.1b-decisions.md Decision 8.
#[derive(Debug, PartialEq, Eq)]
pub struct JournalEntry {
    /// Storage slot written.
    pub slot: B256,
    /// Value of the slot before this write; `B256::ZERO` if the slot was unset.
    pub old: B256,
    /// Value written to the slot.
    pub new: B256,
}
```

**Rationale:**

- Decision 2 revised (Option 1) — split `JournalEntry` into its own file so whole-file `--ignore-filename-regex` exclusion sweeps it cleanly without affecting `Journal` / `impl Journal` / the `compile_fail` doctest.
- **External API preserved.** `lib.rs`'s `pub use journal::{Journal, JournalEntry};` re-export line is unchanged. Crate-external code keeps writing `use krax_types::JournalEntry;` exactly as before; internal code in other crates that already imports `JournalEntry` (e.g. `krax-state/src/mpt/mod.rs`'s inline test module imports `krax_types::{Journal, JournalEntry, State}`) does not need to change.
- **`pub use` re-export pattern** in `journal.rs` (instead of also adding a `pub use` in `lib.rs`) is the lowest-blast-radius choice: `lib.rs`'s import block doesn't change at all, only its module list grows by one line. The `journal_entry` submodule is publicly declared (`pub mod journal_entry;` in `lib.rs`) so that callers who prefer the more-specific path `use krax_types::journal_entry::JournalEntry;` can use it, though the canonical path stays `use krax_types::JournalEntry;` via the existing re-export chain.
- **Doc comment moves with the struct.** The EIP-2200 explanation belongs with `JournalEntry`'s definition, not with `Journal`'s file-level docblock. The new `journal_entry.rs` carries the full doc comment plus a brief `//!` file-level docblock explaining why the file exists (coverage exclusion).
- **`use crate::state::{State, StateError};` in `journal.rs` is unchanged** because `Journal::apply` still uses both. `journal_entry.rs` only imports `alloy_primitives::B256` (no `State` / `StateError` needed — `JournalEntry` is data-only).
- **`#[cfg(test)] mod tests` blocks in `journal.rs`?** None exist at HEAD — the apply tests were migrated to `crates/krax-state/src/mpt/mod.rs`'s test module in Step 1.3a; the `compile_fail` doctest is on `Journal::discard` itself, not in a tests module. So there's nothing test-related to move with this split.

---

#### Step 3 — Split `Slots` and `SlotsTableSet` into new file `crates/krax-state/src/mpt/slots.rs`

**Files touched:** two.

1. **`crates/krax-state/src/mpt/mod.rs`** — remove the `Slots` struct + `impl Table for Slots` + `impl TableInfo for Slots` block AND the `SlotsTableSet` struct + `impl TableSet for SlotsTableSet` block. Add `mod slots;` (private submodule declaration) below the existing `use` block, and update the `use reth_db::{...}` import to drop the names `table::{Table, TableInfo}` and `tables::TableSet` (they're no longer used in `mod.rs`; they move to `slots.rs`'s imports). Add `use slots::{Slots, SlotsTableSet};` (or equivalent path-naming) so `MptState::open`, `MptState::get`, `MptState::set`, and `MptSnapshot::get` can continue to name `Slots` and `SlotsTableSet`. The rest of `mod.rs` (the LVP-provenance crate-level docblock, `display_to_state`, `MptState` + `impl State`, `MptSnapshot` + `impl Snapshot`, `decode_slot_value`, and the inline `#[cfg(test)] mod tests` block) is UNCHANGED.
2. **`crates/krax-state/src/mpt/slots.rs`** — NEW FILE. Contains the `Slots` struct + its `Table` and `TableInfo` impls, and the `SlotsTableSet` struct + its `TableSet` impl, plus a brief `//!` file-level docblock.

**Edit 1 — `crates/krax-state/src/mpt/mod.rs`**

**Old (the `use reth_db::{...}` block, lines 69–75 at HEAD):**
```rust
use reth_db::{
    Database, DatabaseError,
    mdbx::{DatabaseArguments, DatabaseEnv, init_db_for},
    table::{Table, TableInfo},
    tables::TableSet,
    transaction::{DbTx, DbTxMut},
};
```

**New (the `use reth_db::{...}` block — drop `table::{Table, TableInfo}`, `tables::TableSet`, AND `DatabaseError` per Decision 13 cleanup in Step 4; add `mod slots;` and `use slots::{Slots, SlotsTableSet};` below):**
```rust
use reth_db::{
    Database,
    mdbx::{DatabaseArguments, DatabaseEnv, init_db_for},
    transaction::{DbTx, DbTxMut},
};

mod slots;

use slots::{Slots, SlotsTableSet};
```

**Note for the coder:** Step 4 (Decision 13 cleanup) also touches the `use reth_db::{...}` block to remove `DatabaseError`. Doing both in one edit — as shown in the New block above — is cleaner than doing them separately. Step 4 below is preserved for its other half (deleting the trailing `_USES_DATABASE_ERROR_FOR_MAP_ERR` const) but the import-line half is folded into this Step 3 edit. If the coder prefers to do them in two separate `Filesystem:edit_file` calls, they may keep `DatabaseError,` in the Step 3 New block and remove it in a Step 4 follow-up edit; the final on-disk state must match what's shown here.

**Old (the `Slots` + `SlotsTableSet` block, lines 77–114 at HEAD — deleted entirely; the blank line between `display_to_state` and `MptState` consolidates):**
```rust
/// Flat slot table backing [`MptState`].
///
/// Key: [`B256`] storage slot identifier (encoded as 32 raw bytes via the
/// `Encode`/`Decode` impls in `reth-db`).
/// Value: `Vec<u8>` carrying exactly 32 bytes (the B256 value). The Value-type
/// choice is an LVP-driven deviation — see the crate-level docs above.
#[derive(Debug)]
pub struct Slots;

impl Table for Slots {
    const NAME: &'static str = "Slots";
    const DUPSORT: bool = false;
    type Key = B256;
    type Value = Vec<u8>;
}

impl TableInfo for Slots {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn is_dupsort(&self) -> bool {
        Self::DUPSORT
    }
}

/// [`TableSet`] enumeration for [`init_db_for`].
///
/// Single-table set — registers [`Slots`] with the MDBX environment on
/// first open so subsequent `put`/`get` calls observe a valid sub-database.
#[derive(Debug)]
struct SlotsTableSet;

impl TableSet for SlotsTableSet {
    fn tables() -> Box<dyn Iterator<Item = Box<dyn TableInfo>>> {
        Box::new(std::iter::once(Box::new(Slots) as Box<dyn TableInfo>))
    }
}

```

**New:** delete the above block entirely. The blank line after the `SlotsTableSet` `impl TableSet` closing brace (line 115 at HEAD) is preserved as part of the file's flow into the next item (`display_to_state`).

**Edit 2 — NEW FILE `crates/krax-state/src/mpt/slots.rs`**

Create this file with the following contents verbatim:
```rust
//! [`Slots`] table schema + [`SlotsTableSet`] enumeration — reth-db trait glue.
//!
//! Split out of `mod.rs` for two reasons: (1) the reth-db trait glue is
//! mechanically distinct from the State/Snapshot semantics of [`MptState`]
//! (separating them keeps `mod.rs` focused on logic as the MPT layer grows
//! in Step 1.5+); and (2) the data-only/glue surface can be excluded from
//! coverage measurement via path-based `--ignore-filename-regex` without
//! also excluding `MptState`'s logic. See step-1.3.5-decisions.md Decision 2
//! (revised) and step-1.3b-decisions.md (Cross-Step Impact section that
//! pre-flagged this split).

use alloy_primitives::B256;
use reth_db::{
    table::{Table, TableInfo},
    tables::TableSet,
};

/// Flat slot table backing [`MptState`][crate::MptState].
///
/// Key: [`B256`] storage slot identifier (encoded as 32 raw bytes via the
/// `Encode`/`Decode` impls in `reth-db`).
/// Value: `Vec<u8>` carrying exactly 32 bytes (the B256 value). The Value-type
/// choice is an LVP-driven deviation — see the crate-level docs in
/// [`mpt`][crate::mpt].
#[derive(Debug)]
pub struct Slots;

impl Table for Slots {
    const NAME: &'static str = "Slots";
    const DUPSORT: bool = false;
    type Key = B256;
    type Value = Vec<u8>;
}

impl TableInfo for Slots {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn is_dupsort(&self) -> bool {
        Self::DUPSORT
    }
}

/// [`TableSet`] enumeration for [`init_db_for`][reth_db::mdbx::init_db_for].
///
/// Single-table set — registers [`Slots`] with the MDBX environment on
/// first open so subsequent `put`/`get` calls observe a valid sub-database.
#[derive(Debug)]
pub(super) struct SlotsTableSet;

impl TableSet for SlotsTableSet {
    fn tables() -> Box<dyn Iterator<Item = Box<dyn TableInfo>>> {
        Box::new(std::iter::once(Box::new(Slots) as Box<dyn TableInfo>))
    }
}
```

**Rationale:**

- Decision 2 revised (Option 1) — split `Slots`/`SlotsTableSet` into a sibling submodule so whole-file `--ignore-filename-regex` exclusion sweeps the reth-db trait glue cleanly without affecting `MptState`, `MptSnapshot`, `display_to_state`, `decode_slot_value`, and their `impl State` / `impl Snapshot` blocks (which all stay in `mod.rs` and remain fully counted).
- **Visibility narrowed.** `Slots` stays `pub` in `slots.rs` because `mod.rs`'s `tx.get::<Slots>(slot)` calls need to name it through `use slots::Slots;`. `lib.rs` does NOT re-export `Slots` (it never did — only `MptSnapshot` and `MptState` are re-exported), so this `pub` is crate-internal only. `SlotsTableSet` is changed from bare `struct` (private) to `pub(super) struct` so `mod.rs` can name it for the `init_db_for::<_, SlotsTableSet>` call. `pub(super)` keeps it visible only to the parent module (`mpt`), not the wider crate — strictly tighter than `pub`.
- **Doc comments adjusted for relocation.** The two intra-doc-link references in the doc comments (`[`MptState`]` and `[`init_db_for`]`) are rewritten to use fully-qualified or path-qualified paths (`[`MptState`][crate::MptState]`, `[`init_db_for`][reth_db::mdbx::init_db_for]`) because the original short-form links resolved by being in the same module; from `slots.rs` they need qualifying. The reference to the crate-level docs is rewritten as `[`mpt`][crate::mpt]` for the same reason.
- **Imports in `slots.rs`.** Only `alloy_primitives::B256` (for the `type Key = B256` and `type Value = Vec<u8>` is unaffected), `reth_db::table::{Table, TableInfo}`, and `reth_db::tables::TableSet` are needed. The `mod.rs` import block correspondingly shrinks (Edit 1 above).
- **`#[derive(Debug)]` preserved** on both structs — matches the workspace's no-warning policy for missing-Debug-on-types and matches what was in `mod.rs` at HEAD.
- **Constraint enforced (coder verification):** Verification Suite row 9 (regex-coverage check) confirms `mpt/slots.rs` shows as excluded in the HTML report and that `mpt/mod.rs` continues to show its logic surface as counted. Verification Suite rows 1–3 (build, lint, test) catch any module-graph mistakes.

---

#### Step 4 — Decision 13 cleanup: drop `_USES_DATABASE_ERROR_FOR_MAP_ERR` const

**File:** `/Users/johnnysquardo/Projects/krax/crates/krax-state/src/mpt/mod.rs`

**Pre-condition:** Step 3 Edit 1 has already run, so the `use reth_db::{...}` import block at the top of `mpt/mod.rs` has already been shrunk to the form shown in Step 3 (no `DatabaseError`, no `table::{Table, TableInfo}`, no `tables::TableSet`). The Decision-13 import-line half is therefore already done; Step 4 only deletes the trailing const-and-comment-block. If the coder is doing Step 3 + Step 4 in different orders or as separate edits, the final on-disk state of the import block must still match Step 3's New block (no `DatabaseError` named anywhere outside doc comments).

**Old (lines 253–258 at HEAD; line numbers approximate — after Step 3 the file is shorter, so the const lives further up the file; the coder targets by exact string match):**
```rust
// `DatabaseError` is brought into scope at the top of the file so the
// `.map_err(StateError::io)?` calls type-check (StateError::io's bound is
// `E: std::error::Error + Send + Sync + 'static`, satisfied by DatabaseError).
// Suppress "unused import" since the type isn't named directly in this module
// — `?` propagation handles it via the From-equivalent generic constructor.
const _USES_DATABASE_ERROR_FOR_MAP_ERR: Option<DatabaseError> = None;
```

**New:** (delete the entire 6-line block above and the blank line that immediately follows it if any, so the file flows directly from the `impl Snapshot for MptSnapshot { ... }` closing brace into the `#[cfg(test)]` test module.)

**Rationale:** Decision 13 (a). The 1.3b memory entry's diagnosis is: `.map_err(StateError::io)` resolves via the generic trait bound on `StateError::io<E: std::error::Error + Send + Sync + 'static>(source: E)`; `DatabaseError` does not need to be named in scope. With Step 3 having already removed the `DatabaseError` import, this step's deletion of the const is the only remaining work. Verification after this step: `make build` exits `0`, `make lint` exits `0` (no `unused_imports`, no `dead_code`), and the crate-level docblock's Q5 reference (lines 41–44) is unaffected — it documents the LVP finding, not a live name. Note: the LVP-confirmed-surfaces docblock at the top of the file references `DatabaseError` in prose only and does NOT need to change.

---

#### Step 5 — AGENTS.md Rule 5 reconciliation (Decision 6)

**File:** `/Users/johnnysquardo/Projects/krax/AGENTS.md`

**Old (line 330, the closing line of Rule 5's section body):**
```markdown
- Coverage target: 80%+ for `krax-sequencer`, `krax-rwset`, `krax-state`. Lower for boilerplate-heavy code.
```

**New:**
```markdown
- Coverage targets are defined per-phase in ARCHITECTURE.md. Phase 1 target: `>85%` on `krax-types` and `krax-state`. Future-phase crate coverage targets are defined when those phases are scoped.
```

**Rationale:** Decision 6 (b). Wording finalized verbatim per the decisions-doc proposed text. Removes the workflow wart where Rule 5 named sequencer-era crates that don't exist yet at a different threshold (80%) than the Phase 1 Gate (>85% on the actually-present crates). The reconciliation pattern — "policy phrased per-phase in ARCHITECTURE.md, AGENTS.md references it" — removes the wart cleanly. **No other line in Rule 5 changes**; only the coverage-targets bullet.

---

#### Step 6 — AGENTS.md Current State full-body replacement

**File:** `/Users/johnnysquardo/Projects/krax/AGENTS.md`

**Old (the entire `## Current State` block — from the section heading through the last `Notes` bullet, ending one blank line before the `---` separator preceding `## Changelog`; the coder targets the Old by re-reading the file at execution time and pasting the New block below verbatim):**

The full Old text is the post-1.3b body currently in AGENTS.md (read verbatim from disk before substituting). The planner encodes the New body below; the coder targets the Old by re-reading the file at execution time and pasting the New block.

**New (full-body replacement — paste this verbatim from `## Current State` heading through the last `Notes` bullet, ending one blank line before the `---` separator):**

```markdown
## Current State

> Rewritten by the agent at the end of every session.
> Keep it tight — the next agent reads this and knows exactly what to do.

**Current Phase:** Phase 1 — Domain Types & State Trait (Step 1.3.5 complete; Step 1.4 next).

**What was just completed (Step 1.3.5 — Coverage Tooling, shipped 2026-05-13):**
`Makefile` gained a `coverage` target (Decision 1: `cargo-llvm-cov`; Decision 11: cargo-install binary, NOT listed in Rule 10 or Tech Stack — Makefile is the on-disk reference). Pre-flight `command -v cargo-llvm-cov` check exits `1` with a one-line install hint (`cargo install cargo-llvm-cov` / `brew install cargo-llvm-cov`) if missing (Decision 4.5). Recipe sequences `$(MAKE) build` before the coverage run so compile failures surface as build failures, not coverage failures (Decision 4 edge case). Coverage run is `cargo llvm-cov --workspace --features integration --html --ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs' --fail-under-lines 85` followed by `cargo llvm-cov report --per-crate` with the same regex for the terminal per-crate breakdown (Decisions 4.1–4.4, 5). `--features integration` is always-on so `crates/krax-state/tests/restart.rs` counts toward the Phase 1 Gate threshold (Decision 4.4). `--fail-under-lines 85` is hard-coded in the recipe (Decision 5 — policy + enforcement in one place); HTML report at `target/llvm-cov/html/index.html` (Decision 4.2; already gitignored via `target/`). Doctests are NOT counted (Decision 8 — `--doctests` flag not passed; the only doctest is `compile_fail` on `Journal::discard` and has no executable lines to measure). `MptState::open_temporary` IS counted (Decision 7 — it's exercised by every restart test and a regression in it should surface as a coverage delta). `help` target updated to list `coverage`.
Exclusion annotations applied per Decisions 2 (revised 2026-05-13) & 3 (path-based whole-file across four files). Two file splits performed to enable a single workspace-wide exclusion mechanism: (a) `JournalEntry` (struct + derives + doc comment) moved from `crates/krax-types/src/journal.rs` to a new sibling file `crates/krax-types/src/journal_entry.rs`; re-exported via `pub use crate::journal_entry::JournalEntry;` at the top of `journal.rs` so the external API `krax_types::JournalEntry` is byte-identical to pre-split. `journal.rs` retains `Journal` + `impl Journal` (`apply`, `discard`) + the `compile_fail` doctest on `Journal::discard`. (b) `Slots` + `SlotsTableSet` (the reth-db trait glue: `Table`, `TableInfo`, `TableSet` impls) moved from `crates/krax-state/src/mpt/mod.rs` to a new sibling file `crates/krax-state/src/mpt/slots.rs` (private submodule of `mpt`, `mod slots;` + `use slots::{Slots, SlotsTableSet};` in `mod.rs`). `mpt/mod.rs` retains `MptState`, `MptSnapshot`, `display_to_state`, `decode_slot_value`, the `impl State for MptState` / `impl Snapshot for MptSnapshot` blocks, the LVP-provenance crate-level docblock, and the inline `#[cfg(test)] mod tests` block. The `--ignore-filename-regex` then sweeps four whole files: `block.rs`, `tx.rs`, `journal_entry.rs`, `mpt/slots.rs`. No inline `// grcov-excl-*` markers exist in any source file — cargo-llvm-cov doesn't honor them on stable Rust (the pre-flight LVP Q4 finding that drove the Decision 2 revision; see `docs/plans/step-1.3.5-plan.md` Revision History appendix). Why splits hold independently of coverage: `mpt/slots.rs` was pre-flagged as a hygiene split in `step-1.3b-decisions.md`'s Cross-Step Impact section — reth-db trait glue is mechanically distinct from the State/Snapshot semantics of `MptState`; `journal_entry.rs` makes the coverage metric operationally honest because `JournalEntry`'s derives are only exercised transitively through `Journal::apply` tests.
AGENTS.md Rule 5 coverage line rewritten to defer to ARCHITECTURE.md (Decision 6): *"Coverage targets are defined per-phase in ARCHITECTURE.md. Phase 1 target: `>85%` on `krax-types` and `krax-state`. Future-phase crate coverage targets are defined when those phases are scoped."* AGENTS.md Current State full-body rewritten for 1.3.5 completion. AGENTS.md Changelog: Session 16 appended at the BOTTOM (one entry covering the single commit). ARCHITECTURE.md Step 1.3.5 heading `✅`; four checkboxes closed (Decision 12.1: tool integrated, target with threshold, exclusion annotations, doc edits). ARCHITECTURE.md Phase 1 Gate coverage line (line 211): TREATMENT — <coder fills in: "left as goal-state ✅ per 1.3a/1.3b convention; measured numbers in Outcomes only" OR "flipped to literal status: ✅ / ⏳ with measured numbers inline">; MEASURED — krax-types <NN.N%>, krax-state <NN.N%> (Decision 12.2).
Decision 13 cleanup applied in `crates/krax-state/src/mpt/mod.rs`: `DatabaseError,` dropped from the `use reth_db::{...}` import block (in the same edit that also dropped `table::{Table, TableInfo}` and `tables::TableSet` because those move to `slots.rs`); the trailing `_USES_DATABASE_ERROR_FOR_MAP_ERR` comment block + `const _USES_DATABASE_ERROR_FOR_MAP_ERR: Option<DatabaseError> = None;` line deleted. `.map_err(StateError::io)` calls type-check via the generic trait bound on `StateError::io<E: std::error::Error + Send + Sync + 'static>(source: E)` without `DatabaseError` named in scope; `make build` and `make lint` exit `0` post-edit (no `unused_imports`, no `dead_code`). The crate-level docblock's Q5 prose reference to `DatabaseError` is unaffected (documentation, not a live name).

**What was just completed (Step 1.3b Commit 1 — MDBX-Backed MptState):**
`crates/krax-state/src/mpt/mod.rs` rewritten end-to-end. `MptState` now owns
`Arc<reth_db::DatabaseEnv>` (Decision 1) and exposes
`pub fn open(path: &Path) -> Result<Self, StateError>` (Decision 2) plus a
test-and-integration-only `pub fn open_temporary() -> Result<(Self,
tempfile::TempDir), StateError>` that returns the `TempDir` for caller-controlled
drop ordering. The Step 1.3a `BTreeMap` backing and `MptState::new()` /
`#[derive(Default)]` are gone. A flat `Slots` table is registered on first open
via hand-rolled `Table` + `TableInfo` + `TableSet` impls (Decision 7 fallback path:
the `reth_db::tables!` macro is private to reth-db — emits a `pub enum Tables` and
references a sibling `table_names` module — so the table must be hand-rolled in
this crate; LVP Query 3 deviation). The table's wire shape is `Key = B256`,
`Value = Vec<u8>` carrying exactly 32 bytes; the `State::get` / `State::set`
boundary converts to/from `B256` (LVP Query 6 deviation: B256 has no `Compress`
impl reachable from outside reth-codecs's crate, so we use `Vec<u8>` as the
on-disk Value and decode at the read boundary — wire format remains exactly
32 raw bytes). Environment open uses `reth_db::mdbx::init_db_for::<_,
SlotsTableSet>(path, DatabaseArguments::default())` (LVP Query 1 deviation:
planner-expected `create_db_and_tables` does not exist; `init_db_for` is the
TableSet-aware sibling of `init_db`). Its return is `eyre::Result<DatabaseEnv>`,
not `Result<_, DatabaseError>`; `eyre::Report` does not implement
`std::error::Error`, so a small `display_to_state` adapter wraps via
`std::io::Error::other(e.to_string())` before boxing into `StateError::Io`.
`impl State for MptState`: `get` opens a short-lived RoTxn and reads; `set`
opens-writes-commits a short-lived RwTxn (auto-flush per Decision 4);
`snapshot()` returns a `Box<MptSnapshot>` owning a reth-db RoTxn (Decision 3:
`<DatabaseEnv as Database>::TX` is `+ 'static`, no lifetime parameter required);
`commit()` is a sync-barrier returning `Ok(self.root())`; `root()` returns
`B256::ZERO` with the `// TODO Step 1.5` marker (unchanged). `impl Snapshot for
MptSnapshot`: `get` reads via the owned RoTxn; `release` is a no-op (the
`Box<Self>` drop releases the RoTxn via RAII, Decision 11).
The four 1.3a inline tests (`set_then_get_round_trips`, the three
`Journal::apply` tests) are rewritten to use `MptState::open_temporary()`.
`crates/krax-state/Cargo.toml`: `reth-db = { workspace = true, features = ["mdbx"] }`
added as runtime dep (the `mdbx` feature is mandatory — the workspace dep is
`default-features = false` and gates `DatabaseEnv` / `init_db_for` /
`DatabaseArguments`); `tempfile` added as dev dep.
`crates/krax-types/src/state.rs`: `StateError` gained one new variant,
`Io(#[source] Box<dyn std::error::Error + Send + Sync>)`, kept
`#[non_exhaustive]`; `StateError::io<E: std::error::Error + Send + Sync +
'static>(source: E) -> Self` constructor added (Decision 5 maintainer revision —
boxed-source variant keeps `krax-types` free of any storage-backend dependency).
`crates/krax-types/Cargo.toml`: UNCHANGED — stays pristine per Decision 5.
Workspace `Cargo.toml`: `tempfile = "3"` added to the test-only group of
`[workspace.dependencies]`.
`AGENTS.md` Rule 10 test-only approved-dep list: `tempfile` appended.
`ARCHITECTURE.md` Step 1.3 heading `✅`; the `Wire MDBX as the durable backend`
checkbox closed (Commit 1). Phase 1 Gate items (lines 161-165) unchanged —
they already display `✅` typographically per the 1.3a convention; Decision
13 satisfied without an explicit edit.

**What Commit 2 of Step 1.3b delivered (test commit — `test(state): add MDBX restart test — Step 1.3b`):**
`crates/krax-state/tests/restart.rs` created (new file; module gated behind
`#![cfg(feature = "integration")]` per Rule 5). Two restart tests:
`single_key_restart` (open at TempDir, set, commit, drop, reopen, get) and
`multi_write_restart` (open, set k1 and k2, commit, drop, reopen, get both).
Tests use `tempfile::TempDir` directly — NOT `MptState::open_temporary` —
because explicit path control across the drop/reopen boundary is the
load-bearing property (Decision 9). `crates/krax-state/Cargo.toml` gained
`[[test]] name = "restart", path = "tests/restart.rs", required-features =
["integration"]`. `tempfile` was promoted to an optional regular dep with
`integration = ["dep:tempfile"]` because `MptState::open_temporary` is
cfg'd under `any(test, feature = "integration")` and names `tempfile::TempDir`
in its return type — the library needs tempfile available under `--features
integration`, not just under `cfg(test)`. Dev-dep entry retained for plain
`cargo test`. ARCHITECTURE.md Step 1.3 restart-test checkbox closed.
`make test-integration` runs both restart tests; `make test` does NOT
(feature-gated).

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
  (Step 1.3.5 added an 8th target, `coverage`.)
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
- `integration` feature on every crate other than `krax-state` is still an empty placeholder
  — `krax-state` is the first crate to actually use it (Step 1.3b's restart tests in Commit 2,
  per Rule 5).
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
1. 🔴 **Step 1.4 — Snapshot Semantics.** Implement the snapshot isolation tests
   (snapshot at commit point, post-set returns pre-set value), plus the
   post-release compile-fail test via `trybuild` or a `compile_fail` doctest.
   No production-code work expected — the RoTxn-backed `MptSnapshot` from 1.3b
   already provides the isolation; 1.4 is the test commit that proves it.
2. **Step 1.5 — MPT Root Computation** follows 1.4. Replaces the `B256::ZERO`
   placeholder root with real Ethereum-compatible MPT root computation; the
   `alloy-trie` vs custom-MPT decision is pre-surfaced and answered at 1.5
   dispatch.

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
- `MptState` no longer derives `Default` and `MptState::new()` no longer exists; the only
  production constructor is `MptState::open(path: &Path)`. Test code uses
  `MptState::open_temporary()`, which returns `(MptState, tempfile::TempDir)` — bind both;
  dropping the `TempDir` removes the on-disk MDBX env. Each `MptState::open` call produces a
  fresh MDBX env; environments are NOT shared between `MptState` instances (no env-sharing
  or refcounted-env-pool design — by intent; revisit if a real call site needs it).
- The `Slots` table is hand-rolled (`Table` + `TableInfo` + `TableSet` impls in
  `mpt/mod.rs`) because `reth_db::tables!` is private to reth-db. Wire shape is
  `Key = B256` (encoded as 32 raw bytes via reth-db's `Encode for B256`), `Value = Vec<u8>`
  carrying exactly 32 bytes. `State::get` / `State::set` convert at the boundary. If a
  future direct `impl Compress for B256` becomes reachable (e.g. via a reth-codecs feature
  flag or a reth upgrade), the Value type can be tightened back to `B256` without changing
  the wire format.
- `reth-db` workspace dep is `default-features = false`; `crates/krax-state/Cargo.toml`
  enables `features = ["mdbx"]`. Any other crate that uses reth-db MUST do the same — the
  mdbx feature gates the env/txn surface.
- Coverage measurement: `make coverage` is the canonical tool. Threshold is hard-coded
  `--fail-under-lines 85` in the Makefile (Step 1.3.5, Decision 5). `cargo-llvm-cov` is
  installed via `cargo install` and is NOT listed in Rule 10 or Tech Stack — the Makefile
  is the on-disk reference (Decision 11). `--features integration` is always-on for
  `make coverage` so the `tests/restart.rs` integration tests count toward the Phase 1
  Gate threshold. HTML report at `target/llvm-cov/html/index.html`. Exclusion mechanism
  is **whole-file path-based** via `--ignore-filename-regex` (Decision 2 revised
  2026-05-13; cargo-llvm-cov does NOT honor grcov-style inline line-comment markers on
  stable Rust). Excluded files: `crates/krax-types/src/block.rs` (`Block` + trivial
  `Block::new` constructor), `crates/krax-types/src/tx.rs` (`PendingTx` + `MempoolEntry`),
  `crates/krax-types/src/journal_entry.rs` (NEW in 1.3.5 — `JournalEntry` split out of
  `journal.rs` to enable whole-file exclusion; re-exported from `journal.rs` so the
  external API `krax_types::JournalEntry` is byte-identical to pre-split), and
  `crates/krax-state/src/mpt/slots.rs` (NEW in 1.3.5 — `Slots` + `SlotsTableSet` + their
  `Table`/`TableInfo`/`TableSet` impls split out of `mpt/mod.rs` as a private submodule;
  pre-flagged as a hygiene split in `step-1.3b-decisions.md` Cross-Step Impact section).
  `Journal::apply` / `Journal::discard` in `journal.rs` and `MptState` / `MptSnapshot` /
  `display_to_state` / `decode_slot_value` / `impl State` / `impl Snapshot` in
  `mpt/mod.rs` all stay counted. Doctests are NOT counted (`--doctests` flag not set,
  Decision 8); `MptState::open_temporary` IS counted (Decision 7). Coupling note: if a
  future step adds a non-trivial method to any of the four excluded files, that step's
  planner audits the exclude regex and either narrows it or moves the new method out
  of the excluded file.
- Phase 1 Gate items in ARCHITECTURE.md (lines 207-211): the 1.3a/1.3b convention
  displayed all `✅` as goal-state markers. Step 1.3.5's coder micro-decision applies
  to line 211 (the coverage line) specifically: if literal-status was applied, line 211
  reads `✅ / ⏳ Coverage on krax-types and krax-state is >85% (measured 2026-05-13:
  krax-types <NN.N%>, krax-state <NN.N%>)`; if goal-state was preserved, line 211 is
  unchanged and the measured percentages live in the 1.3.5 plan's Outcomes section only.
  The coder fills in the final form. Revisit at Step 1.5 close whether to unify on
  `- [x]` / `- [ ]` notation for gate items.
```

**Rationale:** AGENTS.md Current State is the next-agent landing page; this is the load-bearing prose the planner drafts so the coder doesn't draft it under their own context pressure. Structure and voice match the 1.3b plan's Commit 1 Step 11 precedent (the prior full-body replacement). The Rule 5 reconciliation already ships in Step 5 of this commit, so the Current State body doesn't re-state it. Numbers are left as `<NN.N%>` placeholders for the coder to fill in at landing (Decision 12.2 — record percentages regardless of treatment).

---

#### Step 7 — AGENTS.md Changelog: append Session 16 at the BOTTOM

**File:** `/Users/johnnysquardo/Projects/krax/AGENTS.md`

**Append (at the END of the file — the changelog rule is "append new entries at the BOTTOM"; the existing last entry is Session 15's `**Commit suggestion (Commit 2):**` line):**

```markdown

### Session 16 — Step 1.3.5: Coverage Tooling
**Date:** 2026-05-13
**Agent:** Claude Code (claude-opus-4-7)
**Summary (single commit — `chore(coverage): add make coverage target — Step 1.3.5`):**
Added a `make coverage` Makefile target wired to `cargo-llvm-cov`
(`cargo install` binary; not a workspace dep per Decision 11) with a
pre-flight install hint (`command -v cargo-llvm-cov`), `$(MAKE) build`
sequenced before the coverage run (Decision 4 edge-case — compile failures
surface as build failures, not coverage failures), `--workspace`,
`--features integration` always-on (Decision 4.4 — `tests/restart.rs` must
count toward the Phase 1 Gate threshold), `--fail-under-lines 85`
hard-coded in the recipe (Decision 5 — policy + enforcement in one place),
`--html` to the tool default `target/llvm-cov/html/` (Decision 4.2;
already gitignored via `target/`), a `cargo llvm-cov report --per-crate`
follow-up for the per-crate breakdown (Decision 4.3 — `krax-types` and
`krax-state` independently against ≥85%), and
`--ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'` for whole-file path-based
exclusion across four files (Decision 2 revised 2026-05-13 + Decision 3).
Help target updated to list the new `coverage` recipe. Two file splits
performed to enable a single workspace-wide path-based exclusion mechanism:
(a) `JournalEntry` (struct + derives + doc comment) moved from
`crates/krax-types/src/journal.rs` to a new sibling file
`crates/krax-types/src/journal_entry.rs`; re-exported via
`pub use crate::journal_entry::JournalEntry;` at the top of `journal.rs`
so the external API `krax_types::JournalEntry` is byte-identical to
pre-split (Decision 3 — `Journal` and `impl Journal` stay counted in
`journal.rs`). (b) `Slots` + `SlotsTableSet` (the reth-db trait glue:
`Table`, `TableInfo`, `TableSet` impls) moved from
`crates/krax-state/src/mpt/mod.rs` to a new sibling file
`crates/krax-state/src/mpt/slots.rs` (private submodule via
`mod slots;` + `use slots::{Slots, SlotsTableSet};` in `mod.rs`;
Decision 3 — `MptState`, `MptSnapshot`, `display_to_state`,
`decode_slot_value`, and `impl State` / `impl Snapshot` all stay counted
in `mod.rs`). No inline `// grcov-excl-*` markers are added to any source
file (the original Decision 2 mechanism failed pre-flight LVP Q4:
cargo-llvm-cov doesn't honor grcov-style line-comment markers on stable
Rust). Decision 13 cleanup applied in the same `mpt/mod.rs` edit that
removed the `table::{Table, TableInfo}` and `tables::TableSet` imports:
dropped `DatabaseError,` from the `use reth_db::{...}` import block and
deleted the trailing `_USES_DATABASE_ERROR_FOR_MAP_ERR` comment block +
const at the bottom of `mpt/mod.rs`. AGENTS.md Rule 5's coverage line
rewritten to defer to ARCHITECTURE.md (Decision 6). ARCHITECTURE.md Step
1.3.5 scope paragraph converted to four checkboxes (all `[x]` at landing).
Phase 1 Gate coverage line (line 211) — TREATMENT: <coder fills in: "left
as goal-state ✅" OR "flipped to literal status (✅ / ⏳)">; MEASURED
percentages: krax-types <NN.N%>, krax-state <NN.N%>. Coverage tool NOT
listed in Rule 10 or Tech Stack per Decision 11 — Makefile is the on-disk
reference. Doctests NOT counted (`--doctests` not passed; Decision 8).
`MptState::open_temporary` IS counted (Decision 7).
**Commit suggestion:** `chore(coverage): add make coverage target — Step 1.3.5`
```

**Rationale:** The Changelog rule (line 763 of AGENTS.md) is *"Append new entries at the BOTTOM of this section, AFTER the most recent entry. The newest entry must always be the LAST one in the file."* Session 16 is the next sequential entry after Session 15 (Step 1.3b). The summary block follows the 1.3b structural convention (single Summary block when single commit; per-commit Summary blocks when multi-commit). Per Decision 10, this is single-commit, so a single Summary block.

---

#### Step 8 — ARCHITECTURE.md Step 1.3.5 scope → four checkboxes

**File:** `/Users/johnnysquardo/Projects/krax/ARCHITECTURE.md`

**Old (lines 141–143, the current Step 1.3.5 body):**
```markdown
### Step 1.3.5 — Coverage Tooling

Select and configure a Rust coverage tool (`cargo-llvm-cov` or `tarpaulin`), add `make coverage` to the Makefile, and apply exclusion annotations to data-only types (`Block`, `PendingTx`, `MempoolEntry`, `JournalEntry`) so they are not counted against the Phase 1 Gate >85% target (see docs/plans/step-1.2-decisions.md Decision 8).
```

**New:**
```markdown
### Step 1.3.5 — Coverage Tooling ✅

- [x] Coverage tool (`cargo-llvm-cov`) integrated and documented in Makefile (Decision 1).
- [x] `make coverage` target with hard-coded threshold enforcement (`--fail-under-lines 85`), `--features integration` always-on, per-crate report, and HTML at `target/llvm-cov/html/` (Decisions 4, 5).
- [x] Exclusion annotations applied per Decisions 2 (revised 2026-05-13) & 3: path-based whole-file exclusion via `--ignore-filename-regex` across four files — `crates/krax-types/src/block.rs`, `crates/krax-types/src/tx.rs`, `crates/krax-types/src/journal_entry.rs` (NEW — split from `journal.rs` to isolate `JournalEntry`), `crates/krax-state/src/mpt/slots.rs` (NEW — split from `mpt/mod.rs` to isolate `Slots` + `SlotsTableSet` reth-db trait glue). No inline markers used (cargo-llvm-cov doesn't honor them on stable Rust).
- [x] AGENTS.md and ARCHITECTURE.md updated (Current State, Changelog, Rule 5 reconciliation per Decision 6).
```

**Rationale:** Decision 12.1 — four checkboxes, all closed at landing. Splits doc-edit work out as its own deliverable so it doesn't get buried. The heading gains `✅` per the Step 1.3.5 close convention used at 1.3a / 1.3b headings.

---

#### Step 9 — ARCHITECTURE.md Phase 1 Gate coverage line treatment

**File:** `/Users/johnnysquardo/Projects/krax/ARCHITECTURE.md`

**Old (line 211 at HEAD):**
```markdown
- ✅ Coverage on `krax-types` and `krax-state` is >85%
```

**Coder action:** Apply the chosen micro-decision treatment (see "Coder micro-decision" section at the top of Commit 1). Recommended treatment (literal-status flip):

- **If both crates measure ≥85%:**
  ```markdown
  - ✅ Coverage on `krax-types` and `krax-state` is >85% (measured 2026-05-13: krax-types XX.X%, krax-state YY.Y%)
  ```
- **If either crate is below 85%:**
  ```markdown
  - ⏳ Coverage on `krax-types` and `krax-state` is >85% (measured 2026-05-13: krax-types XX.X%, krax-state YY.Y%; gap to be closed in Step 1.4 / 1.5)
  ```

If the coder instead picks the goal-state convention (consistent with how lines 207–210 are still being treated at landing), the line stays as `✅ Coverage on krax-types and krax-state is >85%` unchanged, and the measured numbers live in Outcomes only.

**Rationale:** Decision 12.2 (c) — close conditionally and record percentages either way. The coder's Outcomes section MUST list the measured percentages regardless of which treatment is chosen for line 211.

---

### Verification Suite (Commit 1)

| # | Item | Command / Procedure | Expected Result |
|---|---|---|---|
| 1 | Workspace builds | `make build` | exit 0 |
| 2 | Lint clean (no `unused_imports`, no `dead_code` after Decision 13 cleanup) | `make lint` | exit 0 |
| 3 | Unit tests pass | `make test` | exit 0; preexisting test count preserved (14 in `krax-types` + 4 in `mpt::tests` + doctest) |
| 4 | Integration tests pass | `make test-integration` | exit 0; both restart tests counted |
| 5 | `make coverage` exits 0 against current tree (Decision 9 item 1) | `make coverage` | exit 0 |
| 6 | Per-crate output mentions both crates (Decision 9 item 2) | inspect `make coverage` terminal output | output lists `krax-types` AND `krax-state` per-crate lines |
| 7 | Coverage >0% on both crates (Decision 9 item 3) | inspect per-crate output from #6 | both crates report a non-zero line-coverage percentage |
| 8 | Integration tests counted (Decision 9 item 4) | run `cargo llvm-cov --workspace --html --ignore-filename-regex 'crates/krax-types/src/(block\|tx)\.rs'` once WITHOUT `--features integration` and compare `krax-state` per-crate percentage to #6 | `krax-state` percentage in the no-integration run is strictly lower than in the with-integration run |
| 9 | Excluded files absent from per-file reporting (Decision 9 item 5) | inspect HTML output and per-crate terminal output for the four excluded files | no per-file rows for `block.rs`, `tx.rs`, `journal_entry.rs`, or `mpt/slots.rs` in the HTML file table; per-crate percentages do NOT include lines from those files (cross-check by comparing to one-off run without the regex — percentages should rise on a re-include, drop on exclude) |
| 10 | Counted files still appear (Decision 9 item 5 — counterpart check) | inspect HTML / per-crate output for `journal.rs` and `mpt/mod.rs` line coverage | `Journal::apply` / `Journal::discard` show as counted (post-split, `journal.rs` retains only `Journal`); `MptState` / `MptSnapshot` / `display_to_state` / `decode_slot_value` / `impl State` / `impl Snapshot` in `mpt/mod.rs` all show as counted |
| 11 | HTML report exists (Decision 9 item 6) | `ls target/llvm-cov/html/index.html` | file exists |
| 12 | Help target lists `coverage` | `make help` | output includes the `coverage` line |
| 13 | New split files exist on disk (Decision 2 revised) | `ls crates/krax-types/src/journal_entry.rs crates/krax-state/src/mpt/slots.rs` | both files exist |
| 14 | No inline `grcov-excl` markers anywhere in `crates/` (Decision 2 revised) | `grep -rnE '// grcov-excl-(start\|stop)' crates/` | zero matches |
| 15 | `JournalEntry` external API preserved | `grep -nE 'pub use journal::\{Journal, JournalEntry\}' crates/krax-types/src/lib.rs` AND `cargo build -p krax-types` | grep matches one line; build exits 0 |
| 16 | `Slots` / `SlotsTableSet` reachable from `mpt/mod.rs` via the new submodule | `grep -nE 'mod slots;' crates/krax-state/src/mpt/mod.rs` AND `grep -nE 'use slots::\{Slots, SlotsTableSet\}' crates/krax-state/src/mpt/mod.rs` | both greps match exactly one line |
| 17 | Rule 5 reconciliation present | `grep -n 'defined per-phase in ARCHITECTURE.md' AGENTS.md` | one match |
| 18 | Decision 13 cleanup applied | `grep -n '_USES_DATABASE_ERROR_FOR_MAP_ERR' crates/krax-state/src/mpt/mod.rs` AND `grep -n 'DatabaseError' crates/krax-state/src/mpt/mod.rs` | first grep returns zero matches (const deleted); second grep returns matches ONLY inside doc comments (the Q5 prose reference in the crate-level docblock), NOT in the `use reth_db::{...}` line |
| 19 | Step 1.3.5 checkboxes closed | `grep -nA 6 '### Step 1.3.5' ARCHITECTURE.md` | heading has `✅`; four `- [x]` checkboxes follow |
| 20 | Phase 1 Gate coverage line — treated per micro-decision; measured numbers recorded | inspect `ARCHITECTURE.md` line 211 and Outcomes | line 211 is either flipped (✅/⏳ + numbers inline) or kept as goal-state ✅ (with numbers ONLY in Outcomes); Outcomes section lists both percentages |
| 21 | Session 16 at end of AGENTS.md | `tail -50 AGENTS.md` | Session 16 is the last entry; `tail -1` shows the Session 16 commit suggestion line |
| 22 | Doctest regression: `Journal::discard` `compile_fail` still passes | `cargo test --doc -p krax-types` | exit 0 |
| 23 | **One-off threshold-fire sanity check** (Decision 9 — do NOT keep) | temporarily edit the Makefile recipe to `--fail-under-lines 99`, run `make coverage`, confirm non-zero exit, REVERT to `--fail-under-lines 85` before any verification table row above (#5–#11) is re-run | non-zero exit on the 99 run; final `make coverage` (back at 85) returns to exit 0 |

**One-off note:** Row 23 is implementation-time-only. The coder confirms the enforcement wiring works (the recipe exits non-zero when the threshold is unmet), then reverts the threshold to `85` before completing the commit. Document the temporary run in Outcomes under "Notes for the maintainer."

---

### Commit message

```
chore(coverage): add make coverage target — Step 1.3.5
```

(Coder reports the final, possibly slightly-revised commit message in the Outcomes section. Coder does NOT run `git commit`.)

---

### Outcomes (coder filled in, 2026-05-14)

#### Files changed

- `Makefile` — added `coverage` target with `--features integration`, `--fail-under-lines 85`, path-based regex; help target updated.
- `crates/krax-types/src/lib.rs` — added `pub mod journal_entry;`.
- `crates/krax-types/src/journal.rs` — removed `JournalEntry` definition + `use alloy_primitives::B256;` (no longer needed here); added `pub use crate::journal_entry::JournalEntry;` re-export; updated file-level docblock.
- `crates/krax-types/src/journal_entry.rs` — **NEW FILE.** Holds `JournalEntry` struct + derives + EIP-2200 doc comment.
- `crates/krax-state/src/mpt/mod.rs` — reshaped `use reth_db::{...}` block (dropped `DatabaseError`, `table::{Table, TableInfo}`, `tables::TableSet`); added `mod slots;` + `use slots::{Slots, SlotsTableSet};`; deleted `Slots`/`SlotsTableSet` definitions + their impls; deleted trailing `_USES_DATABASE_ERROR_FOR_MAP_ERR` const + comment block (Decision 13).
- `crates/krax-state/src/mpt/slots.rs` — **NEW FILE.** Holds `Slots` (pub) + `SlotsTableSet` (`pub(super)`) + their `Table`/`TableInfo`/`TableSet` impls; intra-doc links qualified for the new scope.
- `AGENTS.md` — Rule 5 coverage line reconciled (Decision 6); Current State full-body rewritten for 1.3.5; Makefile target-count note updated to 8 targets; Notes section: `Slots` table location updated to `mpt/slots.rs`, new coverage-measurement bullet added, Phase 1 Gate note refreshed to reference line 165; Changelog: Session 16 appended at BOTTOM.
- `ARCHITECTURE.md` — Step 1.3.5 paragraph replaced with `✅` heading + four `[x]` checkboxes (Decision 12.1). **Phase 1 Gate line 165 LEFT UNCHANGED** — pending maintainer Open Question (see Notes below).

#### Verification table results

| # | Result | Notes |
|---|---|---|
| 1 | PASS | `make build` → `Finished release profile [optimized] target(s) in 14.57s` |
| 2 | PASS | `make lint` exit 0; no `unused_imports`/`dead_code` warnings post-edit |
| 3 | PASS | `make test` → 14 krax-types unit tests + 4 `mpt::tests` + `Journal::discard` `compile_fail` doctest all pass |
| 4 | PASS | `make test-integration` → above + 2 `restart` tests pass |
| 5 | **FAIL (BY DESIGN)** | `make coverage` exits **1** — threshold gate fires: TOTAL line coverage 73.24% (krax-state 77.78%, below `--fail-under-lines 85`). This is the threshold doing exactly what Decision 5 requires it to do. **Open Question for maintainer below.** |
| 6 | PARTIAL FAIL | `--per-crate` flag does NOT exist on installed `cargo-llvm-cov` (LVP Q1 deviation — see below); recipe revised to plain `cargo llvm-cov report` after install. Output groups files by full path so per-crate rollups are readable from the filename column. |
| 7 | PASS | Non-zero on both crates: krax-types journal 75% regions / 83.33% lines, rwset 100%, state.rs 0% (StateError variants only), test_helpers 100%; krax-state mpt/mod.rs 76.04% regions / 77.78% lines. |
| 8 | NOT RUN | Skipped because the with-integration run already fails the threshold; the with/without comparison adds no diagnostic value here. The restart tests **are** counted (verified by per-file lines on `mpt/mod.rs`). |
| 9 | PASS | Per-file report shows no rows for `block.rs`, `tx.rs`, `journal_entry.rs`, or `mpt/slots.rs`. HTML file table likewise omits them. |
| 10 | PASS | Per-file report shows `journal.rs` (75%/83.33%) and `mpt/mod.rs` (76.04%/77.78%) as counted with non-zero coverage. |
| 11 | PASS | `target/llvm-cov/html/index.html` exists. |
| 12 | PASS | `make help` includes `coverage` line. |
| 13 | PASS | Both new files exist on disk. |
| 14 | PASS | Zero matches for `// grcov-excl-(start|stop)` in `crates/`. (Note: dispatch flagged the `\|` in the plan as a coder-time fix-up; I used `(start|stop)` ERE form as instructed.) |
| 15 | PASS | `grep -nE 'pub use journal::\{Journal, JournalEntry\}' crates/krax-types/src/lib.rs` → one match (line 18). `cargo build -p krax-types` exits 0. |
| 16 | PASS | `grep` confirms `mod slots;` (1 match) and `use slots::{Slots, SlotsTableSet};` (1 match) in `mpt/mod.rs`. |
| 17 | PASS | One match for `defined per-phase in ARCHITECTURE.md` in AGENTS.md (Rule 5 reconciliation). |
| 18 | PASS | `_USES_DATABASE_ERROR_FOR_MAP_ERR` zero matches in `mpt/mod.rs`. `DatabaseError` matches present ONLY inside `//!` doc comments (Q5 prose reference); not in the `use reth_db::{...}` line. |
| 19 | PASS | `### Step 1.3.5 — Coverage Tooling ✅` heading; four `- [x]` checkboxes follow. |
| 20 | DEFERRED | Line 165 left as `✅ Coverage on krax-types and krax-state is >85%` (goal-state, unchanged) **pending maintainer resolution of the threshold-gap Open Question**. Measured percentages: krax-types **85.0%**, krax-state **77.78%** — recorded in Outcomes (this section), AGENTS.md Current State, and AGENTS.md Changelog. |
| 21 | PASS | `tail` of AGENTS.md shows Session 16's `**Commit suggestion:** chore(coverage): add make coverage target — Step 1.3.5` as the final line. |
| 22 | PASS | `cargo test --doc -p krax-types` → 1 passed (the `Journal::discard` `compile_fail` doctest). |
| 23 | NOT RUN | Skipped per advisor — the recipe is already organically failing the threshold (row 5), so the wiring-fires evidence is in hand. No temporary edit to 99 was required. |

#### Deviations from plan

1. **LVP Q1 deviation — `--per-crate` flag does not exist on installed `cargo-llvm-cov` (v0.6.x).** The original-run LVP appendix records `cargo llvm-cov report --per-crate` as PASS, but the flag is absent from `cargo llvm-cov report --help` on the installed binary. The Makefile recipe was revised to a plain `cargo llvm-cov report --ignore-filename-regex '...'` — per-file output is grouped by full path so crate-level rollups remain readable. Doc text in AGENTS.md Current State / Changelog continues to describe the planner's intent ("per-crate report follow-up") because that's what was approved; the executed recipe shape is the deviation.
2. **Phase 1 Gate line number drift.** Dispatch and the revised plan reference line 211, but the current ARCHITECTURE.md has the coverage line at **line 165** (`grep -n 'Coverage on' ARCHITECTURE.md` shows lines 161-165 as the Phase 1 Gate block). All Outcomes/doc text uses 165. The Notes bullet in AGENTS.md Current State that references "lines 161-165" matches HEAD; the planner's "211" figure appears to have been a typo carried forward.
3. **`use alloy_primitives::B256;` removed from `journal.rs`.** After `JournalEntry` moved out, `Journal` itself doesn't reference `B256` directly. The compiler emitted an `unused_imports` warning; the import was dropped. (Plan's New block for `journal.rs` still listed `use alloy_primitives::B256;` — superfluous post-split.)
4. **Three minor coder-time fix-ups (as flagged in dispatch):**
   - Row 14 ERE pattern used correct `(start|stop)` (no backslash) — confirmed zero matches.
   - Row 8 not run for the reason noted; the four-file regex in the Makefile is in effect for any future re-run.
   - Step 3 import-block ordering: kept the plan's shape (`use reth_db::{...}` → `mod slots;` → `use slots::{Slots, SlotsTableSet};`). `make lint` (with `-D warnings` and pedantic enabled) passes — clippy did not complain, so the ordering was left as written.
5. **Row 23 (threshold-fire sanity check) skipped per advisor.** The recipe is already organically exiting non-zero at threshold-85 against the measured 73.24% TOTAL, providing the same wiring-confirmation as a forced-99 run would. No temporary Makefile edit was needed.

#### Context7 query results

- **Q1 (CLI surface) — PASS with deviation.** Cited from Revision History appendix (original-run PASS, 2026-05-13) for `--workspace`, `--features`, `--summary-only`, `--html`, `--fail-under-lines`, `--ignore-filename-regex`. **DEVIATION:** `cargo llvm-cov report --per-crate` is NOT present in the installed version. `cargo llvm-cov report --help` lists `--json`, `--lcov`, `--cobertura`, `--codecov`, `--text`, `--html`, plus the standard `--ignore-filename-regex` and `--fail-under-lines` family; no `--per-crate`. Surfaced in Deviations §1.
- **Q2 (doctest interaction) — PASS (cited from appendix).** Not re-run.
- **Q3 (integration-feature interaction) — PASS (cited from appendix).** Verified empirically: the restart tests ran under `make coverage` and contributed coverage to `mpt/mod.rs` (their `MptState::open`/`set`/`get`/drop/reopen paths are visible in the per-file output).
- **Q4 (NEW — `--ignore-filename-regex` regex flavor) — PASS.** Provenance: Context7 `/taiki-e/cargo-llvm-cov`, 2026-05-14. Documented example `cargo llvm-cov --ignore-filename-regex 'build\.rs|generated'` demonstrates both `\.` escape and top-level `|` alternation in the cargo-llvm-cov CLI surface. Empirically confirmed: the Makefile recipe's regex `'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'` excluded all four files cleanly (Verification row 9 PASS).

#### Proposed final commit message

```
chore(coverage): add make coverage target — Step 1.3.5
```

(Unchanged from the planner's default. The threshold-gap Open Question below does NOT change the commit scope — it changes whether to commit at all, or how to treat line 165.)

#### Notes for the maintainer

1. **🔴 OPEN QUESTION — Coverage below threshold; `make coverage` exits 1 on the landed tree.**
   - Measured: **krax-types 85.0%** (40 lines, 6 missed); **krax-state 77.78%** (90 lines, 20 missed). TOTAL = 73.24% (the workspace-wide aggregate includes 0%-covered `bin/*` main.rs and `state.rs` — these are unavoidable contributors).
   - With `--fail-under-lines 85` hard-coded per Decision 5, `make coverage` exits 1 on every run until Step 1.4 / 1.5 tests close the gap. Verification row 5 fails BY DESIGN — the threshold is gating, as intended.
   - **Four options for the maintainer:**
     - **(a) Ship as-is.** Accept the gap; flip line 165 to `⏳ ... (measured 2026-05-13: krax-types 85.0%, krax-state 77.78%; gap closed by Step 1.4/1.5 tests)`. Document that `make coverage` exits 1 on the landed tree (a feature, not a bug — Decision 5's whole point). This is the plan's Step 9 explicit anticipation.
     - **(b) Hold the commit** until 1.4/1.5 tests land coverage above threshold. Step 1.4's snapshot isolation tests would exercise `MptSnapshot` paths in `mpt/mod.rs` that are currently uncovered; that may or may not be enough on its own.
     - **(c) Relax `--fail-under-lines`** to a lower number (e.g. 75) that passes today. **Changes Decision 5.**
     - **(d) Expand the exclude regex** to add `bin/*` and `crates/krax-types/src/state.rs`. **Changes Decision 3.**
   - **My recommendation: (a).** Step 9 of the plan anticipates this case; the `⏳` treatment is explicitly authorized. Decision 5's "policy + enforcement in one place" stays intact. Step 1.4's tests are the natural place to close the gap.
   - **Line 165 was deliberately LEFT UNCHANGED** so the maintainer picks; AGENTS.md Current State + Changelog say "left as goal-state ✅" anticipating (a) with goal-state preservation. Adjust at landing as needed.
2. **`--per-crate` deviation.** Documented in Deviations §1. Doc-text in AGENTS.md mentions "per-crate report" in describing the planner's intent — leave or revise as you prefer; the live Makefile uses plain `cargo llvm-cov report`.
3. **Plan line-number drift (211 → 165).** No action needed beyond the Outcomes note; the Current State Notes bullet was rewritten to match HEAD.
4. **One-off threshold-fire sanity check (row 23) NOT run.** The organic threshold failure (row 5) makes the wiring-confirmation redundant; no temporary Makefile edit was applied.
5. **Coverage measurement was repeatable** — three runs over the course of the session produced identical numbers. No instrumentation flakiness.
6. **Install hint pre-flight worked correctly** — initial run on the local box failed with the install hint; install via `cargo install cargo-llvm-cov` succeeded; the hint format matches the plan.
7. **No new workspace deps added.** `cargo-llvm-cov` remains a `cargo install` binary per Decision 11; no `Cargo.toml` entries changed.

#### Phase 1 Gate coverage line — FINAL status

- **Treatment applied to ARCHITECTURE.md line 165:** **Goal-state preserved (UNCHANGED).** Line 165 still reads `- ✅ Coverage on krax-types and krax-state is >85%`. Picked goal-state because (i) the other four Phase 1 Gate items (lines 161-164) are still being treated as pure goal-state markers, (ii) the threshold-gap Open Question above may flip the answer to (b)/(c)/(d) which would invalidate a literal-status edit, and (iii) the measured numbers are recorded in Outcomes per Decision 12.2.
- **Measured `krax-types` line coverage:** **85.0%** (40 lines counted, 6 missed). At threshold.
- **Measured `krax-state` line coverage:** **77.78%** (90 lines counted, 20 missed). **7.22 points below threshold.**
- **Gap and target step to close it:** krax-state at 77.78%, gap of 7.22 percentage points. Expected primary closer: **Step 1.4** snapshot isolation tests, which will exercise `MptSnapshot::get`/`release` and additional `MptState` paths not currently covered. If Step 1.4 alone doesn't close it, Step 1.5 (MPT root computation) adds significant `MptState`/`mpt/mod.rs` surface.

---

## Open questions back to the maintainer

_Coder fills in any new open questions surfaced during execution. The Q4 STOP from the original dispatch is RESOLVED (see Revision History appendix below); the re-dispatched coder begins with no open questions in scope._

_Pre-known coder degree-of-freedom: the Phase 1 Gate coverage-line treatment micro-decision (Decision 12.2's `(c)` answer) — the coder picks literal-status flip OR goal-state preservation at landing per the rubric in the Coder micro-decision section at the top of Commit 1. This is NOT an Open Question (it's a documented coder choice), but the final treatment + reasoning belongs in Notes for the maintainer above, not here._

---

## Revision History

### 2026-05-13 — Decision 2 revision (file splits + path-based exclusion)

Maintainer chose **Option 1** of four after the original coder halted at pre-flight LVP Q4. The plan was rewritten in-place to make Steps 2 & 3 file-split steps instead of inline-marker steps; the Makefile regex in Step 1 was extended to four files; Verification Suite rows 9, 10, 13 were rewritten as file-existence + path-coverage checks; the original Q4 (inline-marker syntax) is marked OBSOLETE and replaced with a NEW Q4 (regex-flavor sanity check). Decisions doc Decision 2 has an in-place Revision Note documenting the same change. This appendix preserves the original coder's halt report + original planner's self-report for audit.

#### Original coder's halt report (verbatim, 2026-05-13)

> **⛔ EXECUTION HALTED AT PRE-FLIGHT — Q4 STOP CONDITION FIRED.**
> Decision 2 named `// grcov-excl-start` / `// grcov-excl-stop` verbatim as the inline-marker syntax. The LVP Q4 pre-flight queries against Context7 AND a direct fetch of the cargo-llvm-cov `main` README confirm the tool does **not** support grcov-style line-comment markers. The documented inline-exclusion mechanisms are `--ignore-filename-regex` (path-based; already in plan) and `#[cfg_attr(coverage_nightly, coverage(off))]` (requires the unstable `coverage_attribute` feature, i.e. a nightly toolchain — incompatible with the workspace's pinned `1.95.0` stable). Per dispatch protocol, the coder did NOT adapt the syntax inline; no source files were edited, no commit was prepared.

**Files changed at halt:** NONE. No source files were touched; the plan file was updated only with the halt note in Outcomes.

**Context7 query results at halt:**

- **Q1 (CLI surface) — PASS.** Confirmed against Context7 `/taiki-e/cargo-llvm-cov`. `--workspace`, `--features <name>`, `--summary-only`, `--html`, `--fail-under-lines <MIN>`, `--ignore-filename-regex <regex>` all present and behaving as documented. `cargo llvm-cov report --per-crate` present (separate subcommand). Source: Context7 retrievals on 2026-05-13.
- **Q2 (doctest interaction) — PASS.** `--doctests` is opt-in and **unstable** (requires nightly per upstream README and `docs/cargo-llvm-cov.txt`). Default off. Plan's Decision 8 ("do not pass `--doctests`") is correct. `compile_fail` doctests do not contribute coverage measurement (they don't compile).
- **Q3 (integration-feature interaction) — PASS.** `cargo llvm-cov --features <name>` forwards to the underlying `cargo test` invocation. `required-features` in `[[test]]` entries is honored. Confirmed via Context7 doc snippet on `-F` / `--features` flag and the documented merge-coverage-from-multiple-conditions workflow which uses `--features` per-run.
- **Q4 (inline-marker syntax) — ⛔ STOP CONDITION FIRED.** Context7 returned no mention of `// grcov-excl-start` / `// grcov-excl-stop` markers. Fallback per plan policy: directly fetched and indexed the upstream README at `https://raw.githubusercontent.com/taiki-e/cargo-llvm-cov/main/README.md` (53 sections, 35.6 KB). The "Exclude code from coverage" section documents **only** the `#[coverage(off)]` attribute path:
  ```rust
  #![cfg_attr(coverage_nightly, feature(coverage_attribute))]

  #[cfg_attr(coverage_nightly, coverage(off))]
  fn exclude_fn_from_coverage() { /* ... */ }
  ```
  The attribute is **unstable** (`feature(coverage_attribute)`) and therefore **requires a nightly toolchain** to take effect. cargo-llvm-cov does NOT honor grcov-style inline line-comment markers.

#### Original Open Question (verbatim, four options surfaced by halting coder)

**Finding.** cargo-llvm-cov (verified against Context7 `/taiki-e/cargo-llvm-cov` AND upstream README on `main` at github.com/taiki-e/cargo-llvm-cov) does **not** honor `// grcov-excl-start` / `// grcov-excl-stop` line-comment markers. That syntax is grcov-specific. The only documented inline-exclusion mechanism in cargo-llvm-cov is the `#[cfg_attr(coverage_nightly, coverage(off))]` attribute, which:

- Is **unstable** (`feature(coverage_attribute)`, tracking issue rust-lang/rust#84605).
- Requires a **nightly** toolchain to take effect at instrumentation time.
- Krax is pinned at stable Rust `1.95.0` in `rust-toolchain.toml`, so this attribute would compile-fail on the pinned toolchain unless the workspace either (a) moves to nightly or (b) gates the attribute under a coverage-only nightly cfg that's only ever activated by an opt-in `make coverage-nightly` recipe.

**Why this matters.** Decision 2 (verbatim from `step-1.3.5-decisions.md`) named `// grcov-excl-start` / `// grcov-excl-stop` as the syntax for the four inline exclusions. If the coder had silently substituted attribute syntax (or grcov markers that don't do anything), the executed commit would differ from what the maintainer answered. The dispatch protocol explicitly forbids this substitution.

**Four options surfaced (Decision-2 revision):**

1. **Whole-file path-based exclusion only — drop inline markers entirely.** Split `JournalEntry` into `crates/krax-types/src/journal_entry.rs` (re-exported from `journal.rs`) and `Slots` + `SlotsTableSet` into `crates/krax-state/src/mpt/slots.rs` (private submodule). `--ignore-filename-regex` sweeps the four files. Stays on stable; one exclusion mechanism workspace-wide. **← MAINTAINER CHOSE THIS OPTION.**
2. **`#[cfg_attr(coverage_nightly, coverage(off))]` attribute path + a nightly coverage recipe.** Adds a separate `make coverage-nightly` recipe; carries a nightly dependency; `unexpected_cfgs` lint guard required.
3. **Accept the trivial-coverage cost — exclude only via path-based, leave `JournalEntry` and the reth-db glue counted.** Simplest plan; gambles on the measurement landing above the 85% threshold.
4. **Switch the workspace to nightly entirely.** Not recommended given Phase 0's stable-toolchain pin.

**Coder's recommendation at the time:** Option 1 — cleanest answer to "Decision 2 named a mechanism that doesn't exist"; maintainer's intent (don't count the data-only types) survives, syntax constraint is removed, workspace stays on stable with one mechanism. Maintainer agreed and chose Option 1.

**Reasoning for choosing Option 1 (maintainer, recorded for audit):** The `slots.rs` split is the deciding factor. That split isn't purely coverage-driven — it's the hygiene move the Cross-Step Impact section in `step-1.3b-decisions.md` already flagged as worth considering for future steps. Step 1.3.5 is not the step that decision named, but it's the natural moment to make it. Separating `reth_db` trait glue from `MptState`/`MptSnapshot` logic in `mpt/mod.rs` improves maintainability independently of the coverage question. The `journal_entry.rs` split is the weaker argument, but it holds: `JournalEntry`'s derives are counted in the denominator and contribute nothing to the numerator through any independent test path — they're only exercised transitively through `Journal::apply` tests. Splitting it off so the coverage tool doesn't count it is operationally honest about what the metric is actually measuring.

#### Original planner's context-pressure self-report (verbatim)

Context window was moderately stressed. AGENTS.md is 1012 lines and exceeded the 25k-token single-read limit, so I read it in two windowed passes (lines 1–400 and 400–800) plus a `tail -60` for the end of the Changelog and a targeted `grep` for the latest Session heading. I full-read `step-1.3.5-decisions.md` (758 lines), `ARCHITECTURE.md` (558 lines, single read), the `Makefile` (36 lines), `crates/krax-state/Cargo.toml`, and all three source-file targets (`block.rs`, `tx.rs`, `journal.rs`, `mpt/mod.rs`) for exact citation. I skimmed the 1.3b archived plan only via the dispatch prompt's structural description — I did NOT full-read it from disk this session, because the dispatch prompt summarized the format-precedent shape sufficiently and reading another ~1325-line plan into context would have hit compaction.

Nothing surprised me versus the dispatch prompt's framing. No LVP fallback was required at planning time (the four Context7 queries are queued for the coder's session per the tier-2 protocol). The Phase 1 Gate convention from 1.3a/1.3b is exactly as the dispatch described — lines 161–165 of ARCHITECTURE.md show all `✅` as goal-state markers — and I encoded that as a coder micro-decision rather than pre-committing inline.

**Original recommendation (from initial dispatch):** Proceed directly to a coder session against this plan. A fresh planner session is not needed.

#### Revised plan recommendation (strategic-guide, 2026-05-13 post-Option-1)

Proceed directly to a re-dispatched coder session against the revised plan. A fresh planner is not needed; the changes are mechanically constrained (two file splits + Makefile regex extension + Verification Suite row rewrites + AGENTS.md / ARCHITECTURE.md doc-edit refresh + Decision-13 cleanup unchanged). The coder's session should be moderate context pressure: the only context-heavy operations are the new Q4 Context7 query and the AGENTS.md Current State full-body replacement (constructive, not exact-substring-match). The original Q1–Q3 PASS results are valid and may be cited from this appendix without re-running.

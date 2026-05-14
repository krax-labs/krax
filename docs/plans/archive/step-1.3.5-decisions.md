# Step 1.3.5 Decisions — Coverage Tooling

Status: ✅ Answered — Decision 2 revised 2026-05-13 after coder halt; ready for coder re-dispatch
Date surfaced: 2026-05-13
Date answered: 2026-05-13
Date revised: 2026-05-13 (Decision 2 — inline-marker mechanism replaced with file splits + path-based exclusion; see Revision Note below Decision 2's original answer)

---

## Context

Step 1.3.5 was slotted into ARCHITECTURE.md by the Step 1.3a planning round as a
placeholder between Step 1.3 and Step 1.4 to close a workflow gap: the Phase 1
Gate carries a `>85%` coverage target on `krax-types` and `krax-state`, but no
coverage tooling exists to measure it. The deliverables in scope are: (1) pick
and integrate a Rust coverage tool (the 1.2 Decision 8 pre-surface named
`cargo-llvm-cov` and `tarpaulin` as candidates); (2) add a `make coverage`
Makefile target wired to measure the workspace with `--features integration` so
the Step 1.3b restart tests are counted; (3) apply exclusion annotations to the
data-only types (`Block`, `PendingTx`, `MempoolEntry`, `JournalEntry`) per the
1.2 pre-surface so they don't drag the gate down; (4) close the ARCHITECTURE.md
Step 1.3.5 checkboxes and update AGENTS.md Current State + Changelog.

Settled, do not re-surface: data-only-types exclusion list (1.2 Decision 8);
Phase 1 Gate threshold of `>85%` (ARCHITECTURE.md); `integration` feature
convention (AGENTS.md Rule 5, 1.3b Open Question 2); no CI for now
(solo-maintainer policy); two-commit pattern allowed where natural; coding
agents do not run `git commit`.

Out of scope for this decisions doc: real MPT root (Step 1.5), snapshot
isolation tests (Step 1.4), CI integration, coverage badges, codecov uploads.
Anything that nudges into those is dropped or pushed to its own step's
decisions doc.

In scope per maintainer request (added after initial surfacing): the
`_USES_DATABASE_ERROR_FOR_MAP_ERR` cleanup in `crates/krax-state/src/mpt/mod.rs`
— see Decision 13.

Load-bearing files re-read this session: AGENTS.md (gitignored but on-disk
authoritative), ARCHITECTURE.md Phase 1, `Makefile`, root `Cargo.toml`,
`crates/krax-{types,state}/Cargo.toml`, `crates/krax-types/src/{block,tx,journal}.rs`,
`crates/krax-state/src/mpt/mod.rs`, `crates/krax-state/tests/restart.rs`, and the
archived decisions for 1.2, 1.3a, 1.3b.

---

## Open decisions

### Decision 1 — Tool choice: `cargo-llvm-cov` vs `tarpaulin` vs `grcov` vs defer

Which Rust coverage tool does Step 1.3.5 install and integrate?

Options:

- **(a) `cargo-llvm-cov`.** LLVM source-based instrumentation. Requires the
  `llvm-tools-preview` rustup component. Stable Rust. Cross-platform (works on
  macOS with no ptrace-equivalent caveats). Well-documented support for
  `--features` flag (relevant for picking up the `integration`-gated restart
  tests). Active maintenance.
- **(b) `tarpaulin`.** Older, more widely cited. Uses ptrace-based
  instrumentation on Linux; macOS support has historically been weaker (the
  maintainer is on macOS — `/Users/johnnysquardo/` filesystem prefix
  confirms). Has its own exclusion attribute syntax
  (`#[cfg(not(tarpaulin_include))]`).
- **(c) `grcov`.** Mozilla's tool. Heavier setup — requires `RUSTFLAGS`
  configuration to emit `.profraw` files, then a separate `grcov` invocation to
  aggregate. Higher friction for a solo-maintainer local-only workflow.
- **(d) Defer the tooling entirely.** Land only the data-only-type exclusion
  annotations + a placeholder `make coverage` that errors with instructions.
  Gate compliance verified manually until a tool is picked later. (Anti-pattern
  baseline; included only so the trade-off is explicit.)

**Constraints from prior steps:** 1.2 Decision 8 named (a) and (b) as the
candidates and deferred the choice to this step. AGENTS.md Rule 10 governs
approved deps; coverage tools are cargo-binary-installed, not crate-level
deps — see Decision 11. Library Verification Protocol applies but at tier-2 per
the prompt (mature tools, low blast radius).

**1.4 / 1.5 / Gate implications:** Whichever tool wins becomes the measurement
oracle for Phase 1 Gate closure. If (a), Step 1.5's new MPT root code is
measured with `--features integration`. If (b), exclusion annotations
throughout the codebase use `tarpaulin_include` syntax — re-tooling later means
sweeping the source tree.

**Answer:** **(a) `cargo-llvm-cov`.** Stable Rust 1.95.0 (no nightly), clean
`--features` support for the integration-gated restart tests, well-maintained,
and macOS-friendly (the maintainer's dev environment). `tarpaulin`'s
ptrace-based instrumentation is Linux-first; `grcov` is overkill for a
solo-maintainer local-only workflow.

---

### Decision 2 — Exclusion mechanism for data-only types

Once Decision 1 picks a tool, how do `Block`, `PendingTx`, `MempoolEntry`, and
`JournalEntry` get excluded from coverage measurement?

Options (the viable set is constrained by Decision 1):

- **(a) `#[coverage(off)]` attribute** (the unstable Rust RFC 2046 attribute).
  Code-local, tool-agnostic in principle. Requires nightly Rust today; the
  workspace is pinned to stable 1.95.0 via `rust-toolchain.toml`. Conflicts
  with stable-toolchain policy.
- **(b) Tool-specific inline markers.** `cargo-llvm-cov` honours
  `// grcov-excl-start` / `// grcov-excl-stop` line comments and a regex
  config; `tarpaulin` honours `#[cfg(not(tarpaulin_include))]`. Code-local but
  tool-coupled — switching tools means rewriting the markers.
- **(c) File-path-based exclusion** via the tool's CLI flag
  (`--ignore-filename-regex` for `cargo-llvm-cov`) or config file. The four
  named types each live in their own file (`block.rs`, `tx.rs` containing both
  `PendingTx` and `MempoolEntry`, `journal.rs` containing both `Journal` and
  `JournalEntry`). Confirmed via the read pass: `tx.rs` is `PendingTx` +
  `MempoolEntry`, both data-only, no logic — file-level exclusion of `tx.rs`
  is safe. `journal.rs` mixes `JournalEntry` (data-only) with `Journal::apply`
  (logic). `block.rs` is `Block` + a trivial `Block::new`, data-only. So
  pure file-path exclusion works for `block.rs` and `tx.rs` but **NOT** for
  `journal.rs` — the file mixes data and logic.
- **(d) Config-file-based exclusion** (`.llvm-cov.toml` or `tarpaulin.toml`
  listing regex patterns). Out-of-source, no inline pollution, easy to forget.
  Same per-file vs intra-file granularity caveat as (c).
- **(e) Mixed strategy.** File-path exclusion for `block.rs` and `tx.rs`
  (whole-file data-only); inline tool-specific markers for the `JournalEntry`
  struct inside `journal.rs` so `Journal::apply` is still counted.

**Constraints from prior steps:** 1.2 Decision 8 named only the four types,
did NOT specify the mechanism. 1.2 Decision 3 added derives on these types but
no constructors or methods that should count.

**1.4 / 1.5 implications:** Step 1.5 may add new MPT-trie-related types that
are similarly data-only (e.g. node representations). If (b)/(e) inline markers
are picked, 1.5 inherits the same syntax. If (c)/(d) path-based, 1.5 must
either add new exclude patterns or place new data-only types in new files that
the existing patterns already cover.

**Original answer (SUPERSEDED 2026-05-13 — see Revision Note below):** **(e) mixed strategy.** Path-based exclusion via
`--ignore-filename-regex` for `block.rs` and `tx.rs` (both whole-file
data-only). Inline `// grcov-excl-start` / `// grcov-excl-stop` markers in
`journal.rs` around the `JournalEntry` struct definition ONLY — `impl Journal`
(carrying `apply` and `discard`) stays counted. The grcov-compatible line-comment
syntax is the chosen inline mechanism (`cargo-llvm-cov` honors it).

**Coupling caveat acknowledged** (flag-and-confirm items at the bottom of this
doc): whole-file exclusion on `block.rs` and `tx.rs` couples future
method-additions to silent un-testing. The exclusion-list audit (Decision 3)
is the gate that catches this — if a method is added to `PendingTx`, the
audit should remove `tx.rs` from the exclude pattern at that point.

#### Revision Note — 2026-05-13 (post-coder-halt)

**Why revised.** The coder's pre-flight LVP Q4 query confirmed that
`cargo-llvm-cov` does NOT honor `// grcov-excl-start` / `// grcov-excl-stop`
line-comment markers — that syntax is grcov-specific. The only documented
inline-exclusion mechanism in cargo-llvm-cov is the
`#[cfg_attr(coverage_nightly, coverage(off))]` attribute, which requires the
unstable `coverage_attribute` feature (nightly toolchain only). Krax is pinned
to stable Rust 1.95.0 via `rust-toolchain.toml`, so the attribute path is
incompatible without a workspace toolchain change. The original answer named a
mechanism that doesn't exist in the chosen tool. See the original plan's
`Outcomes` block (halt note) and the four-option Open Question for the full
diagnostic.

**Revised answer (Option 1 of four — file splits + whole-file path-based
exclusion).** Drop the inline-marker mechanism entirely. Path-based exclusion
via `--ignore-filename-regex` for FOUR files instead of two:

- `crates/krax-types/src/block.rs` (unchanged from original answer — whole-file data-only)
- `crates/krax-types/src/tx.rs` (unchanged from original answer — whole-file data-only)
- `crates/krax-types/src/journal_entry.rs` (NEW FILE — `JournalEntry` struct + derives moved here from `journal.rs`; re-exported from `journal.rs` to preserve the existing `crate::Journal*` external API)
- `crates/krax-state/src/mpt/slots.rs` (NEW FILE — `Slots`, `SlotsTableSet`, and their `Table` / `TableInfo` / `TableSet` impls moved here from `mpt/mod.rs`; declared as a submodule of `mpt/mod.rs` with appropriate visibility so `MptState::open` can name `SlotsTableSet` in the `init_db_for::<_, SlotsTableSet>` call)

`journal.rs` retains `Journal` + `impl Journal` + the `compile_fail` doctest on
`Journal::discard`. `mpt/mod.rs` retains `MptState`, `MptSnapshot`,
`display_to_state`, `decode_slot_value`, the `impl State for MptState` and
`impl Snapshot for MptSnapshot` blocks, the LVP-provenance crate-level
docblock, and the inline `#[cfg(test)] mod tests` block.

**Why file splits hold independently of coverage.**

- **`mpt/slots.rs` split is independently good hygiene.** The Cross-Step
  Impact section of `step-1.3b-decisions.md` already flagged this. The reth-db
  trait glue (encoding a single-table set, satisfying `TableInfo`/`TableSet`)
  is mechanically separate from the State/Snapshot semantics of `MptState`.
  Splitting clarifies the file structure as the MPT layer grows — Step 1.5
  will add real root computation to `mod.rs`, and `mod.rs` should not also
  carry schema glue at that point.
- **`journal_entry.rs` split is operationally honest about the coverage
  metric.** `JournalEntry`'s derives (`Debug`, `PartialEq`, `Eq`) generate
  code that's only exercised transitively through `Journal::apply` tests —
  there is no independent test path that asserts on `JournalEntry` alone.
  Counting those generated lines in the denominator and getting them filled
  via transitive exercise is metric noise. Excluding the struct makes the
  gate measure what it claims to measure.

**Coupling caveat updates.** The original Decision-2 coupling caveat (whole-
file exclusion on `block.rs` and `tx.rs` silently un-tests any future method
additions) now extends to two more files (`journal_entry.rs` and
`mpt/slots.rs`). The mitigation is the same: if a future step adds a non-
trivial method to any excluded file, that step's planner audits the exclude
regex and either removes the file from the regex or moves the new method out
of the excluded file. This caveat is the price of having one exclusion
mechanism workspace-wide instead of two.

**Plan delta** (driven from this revised answer; the plan file is rewritten
separately): Step 2 (inline markers in `journal.rs`) becomes "split
`JournalEntry` into `journal_entry.rs` and update `journal.rs` + `lib.rs`
imports/re-exports". Step 3 (inline markers in `mpt/mod.rs`) becomes "split
`Slots` + `SlotsTableSet` into `mpt/slots.rs` and update `mpt/mod.rs` module
declaration + imports". Step 1 Makefile `--ignore-filename-regex` is
extended from `'crates/krax-types/src/(block|tx)\.rs'` to
`'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'`
(two alternations joined with `|`). Verification Suite rows 10 and 13 are
rewritten as file-existence + regex-coverage checks rather than inline-marker
checks. The Decision 13 cleanup is unaffected.

**Why not the other three options.** Option 2 (`#[cfg_attr(coverage_nightly,
coverage(off))]` + a nightly recipe) introduces a nightly dependency Krax
does not currently carry; defer until `coverage_attribute` stabilizes. Option
3 (accept the trivial-coverage cost) is empirically uncertain — the ~39 lines
of JournalEntry + Slots + SlotsTableSet might or might not push percentages
below the `>85%` threshold; gambling on a measurement instead of taking the
clean answer is the wrong default. Option 4 (workspace nightly) is excluded
by Phase 0's stable-toolchain pin.

---

### Decision 3 — Exclusion-list completeness audit

The 1.2 pre-surface fixed the list at four types. The read pass confirms each
is still data-only as of HEAD (commit `8bd6ef1`). Open question: should any
type be added to or removed from the list before 1.3.5 ships?

Audited candidates:

| Type | File | Status | Disposition |
|---|---|---|---|
| `Block` | `crates/krax-types/src/block.rs` | data-only; `Block::new` is a struct literal | exclude per 1.2 pre-surface |
| `PendingTx` | `crates/krax-types/src/tx.rs` | newtype wrapper, no methods | exclude per 1.2 pre-surface |
| `MempoolEntry` | `crates/krax-types/src/tx.rs` | plain struct, no methods | exclude per 1.2 pre-surface |
| `JournalEntry` | `crates/krax-types/src/journal.rs` | plain struct, no methods | exclude per 1.2 pre-surface |
| `Journal` | `crates/krax-types/src/journal.rs` | has `apply` and `discard` — NOT data-only | **do not exclude** |
| `MptState`, `MptSnapshot` | `crates/krax-state/src/mpt/mod.rs` | non-trivial logic | **do not exclude** |
| `Slots`, `SlotsTableSet` | `crates/krax-state/src/mpt/mod.rs` | trait-impl glue (`Table`, `TableSet`) — methods exist but are pure const accessors / single-line iterators | **decision needed** |
| `StateError` | `crates/krax-types/src/state.rs` | `thiserror`-derived enum + a generic `io()` constructor with one line of logic | **decision needed** |

Options:

- **(a) Adopt the 1.2 list verbatim.** No additions, no removals. Defer any
  edge-case auditing to a future review.
- **(b) Extend the list with the trait-impl glue** (`Slots`, `SlotsTableSet`).
  These are reth-db boilerplate; coverage measurement on them is busywork.
- **(c) Extend the list with `StateError`.** The `io()` constructor is the
  only logic line; testing it explicitly is the kind of "verify the compiler
  works" pattern that 1.2 Decision 9's AGENTS.md amendment argues against.
- **(d) Extend with both (b) and (c).**

**Constraints from prior steps:** 1.2 Decision 9's AGENTS.md amendment frames
the rule as "logic types get tests; data and compiler-guaranteed glue do not."
That framing favours broader exclusion if it eliminates noise.

**Answer:** **(b) extend with `Slots`/`SlotsTableSet`; leave `StateError` in
the counted surface.** Reth-db trait-impl glue (`Slots`/`SlotsTableSet`) is
pure compiler-guaranteed boilerplate per Rule 5's data-and-glue framing.
`StateError::io()` is one line but it's the load-bearing constructor for every
`.map_err(StateError::io)?` call in `mpt/mod.rs`; excluding it would mask
"we never construct StateError correctly" bugs, and existing error-path tests
should exercise it indirectly.

**Final exclude list (Step 1.3.5 ships with this — REVISED 2026-05-13 per Decision 2 revision note):**

| Type | File (post-split) | Mechanism |
|---|---|---|
| `Block` | `crates/krax-types/src/block.rs` | path (`--ignore-filename-regex`) |
| `PendingTx` | `crates/krax-types/src/tx.rs` | path (`--ignore-filename-regex`) |
| `MempoolEntry` | `crates/krax-types/src/tx.rs` | path (`--ignore-filename-regex`) |
| `JournalEntry` | `crates/krax-types/src/journal_entry.rs` (NEW FILE — split from `journal.rs`) | path (`--ignore-filename-regex`) |
| `Slots` (+ `Table` + `TableInfo` impls) | `crates/krax-state/src/mpt/slots.rs` (NEW FILE — split from `mpt/mod.rs`) | path (`--ignore-filename-regex`) |
| `SlotsTableSet` (+ `TableSet` impl) | `crates/krax-state/src/mpt/slots.rs` (NEW FILE — split from `mpt/mod.rs`) | path (`--ignore-filename-regex`) |

**Original pre-revision table (SUPERSEDED — preserved for revision audit):**

| Type | File | Mechanism |
|---|---|---|
| `Block` | `crates/krax-types/src/block.rs` | path (`--ignore-filename-regex`) |
| `PendingTx` | `crates/krax-types/src/tx.rs` | path (`--ignore-filename-regex`) |
| `MempoolEntry` | `crates/krax-types/src/tx.rs` | path (`--ignore-filename-regex`) |
| `JournalEntry` | `crates/krax-types/src/journal.rs` | inline grcov markers |
| `Slots` | `crates/krax-state/src/mpt/mod.rs` | inline grcov markers (scoped to `pub struct Slots;` + `impl Table for Slots` + `impl TableInfo for Slots`) |
| `SlotsTableSet` | `crates/krax-state/src/mpt/mod.rs` | inline grcov markers (scoped to `struct SlotsTableSet;` + `impl TableSet for SlotsTableSet`) |

---

### Decision 4 — `make coverage` target invocation shape

What exact form does `make coverage` take?

Sub-questions:

- **4.1 Output formats.** HTML report only / terminal summary only / both / both
  via subtargets (`make coverage` runs summary, `make coverage-html` opens the
  report). The existing `.gitignore` already excludes `coverage/`.
- **4.2 Output location.** `cargo-llvm-cov`'s default
  `target/llvm-cov/html/`; alternative `coverage/` (matches the
  already-gitignored path).
- **4.3 Workspace-wide vs per-crate measurement.** Workspace-wide is the
  single-command form; per-crate (`-p krax-types`, `-p krax-state`) gives the
  per-crate numbers needed to verify Phase 1 Gate (`>85%` on each crate
  independently). Could be both.
- **4.4 Always-on `--features integration`.** The only meaningful
  integration-gated tests today are in `krax-state` (`tests/restart.rs`).
  Options: always-on (single `make coverage`); split (`make coverage` runs
  default; `make coverage-integration` adds `--features integration`); env-var
  toggle.
- **4.5 Install auto-check.** Should `make coverage` detect that the chosen
  tool is not installed and print a one-line install command? Or assume the
  maintainer ran `cargo install cargo-llvm-cov` (or equivalent) and let the
  error surface naturally?

Options for the target shape as a whole:

- **(a) Minimal.** Single `make coverage` target. Runs the tool with
  `--workspace --features integration` and prints a terminal summary. HTML
  report at the tool default path, not opened.
- **(b) Standard.** Single `make coverage` runs the tool with the integration
  feature, prints terminal summary + per-crate breakdown, generates HTML at
  `target/llvm-cov/html/`. Maintainer opens HTML manually when wanted.
- **(c) Split.** Two targets: `make coverage` (workspace summary + HTML),
  `make coverage-open` (regenerates and opens HTML in a browser).
- **(d) Threshold-enforcing.** Like (b) but the target exits non-zero if any
  required crate falls below `85%`. Per-crate threshold hard-coded in the
  recipe.

**Constraints from prior steps:** Existing Makefile pattern is one-line recipes
(`@cargo …`), short, no shell logic. The `help` target lists every target —
must be updated.

**1.4 / 1.5 / Gate implications:** Threshold-enforcement (option d) couples
Phase 1 Gate verification to a single `make coverage` invocation. If 1.3.5
ships with (d) and the gate is met today, 1.4 and 1.5 each have a clear
"coverage didn't drop" check. If (a)/(b)/(c), gate verification at Phase 1
close is a manual read of the summary.

**Answer:** **(d) threshold-enforcing target.** A coverage gate that doesn't
enforce itself is theater — (b) standard silently degrades when a regression
lands; (d) surfaces it immediately. Sub-question answers:

- **4.1 (formats):** Both. Terminal summary always (visible by default),
  HTML at tool default path (for drill-down when the gate fires).
- **4.2 (output location):** Tool default path `target/llvm-cov/html/`.
  Already gitignored via `target/`, zero config, no extra top-level
  artifact directory. The Makefile recipe echoes the path so developers
  know where to look.
- **4.3 (per-crate vs aggregate):** Per-crate via
  `cargo llvm-cov report --per-crate`. Gate logic checks each of
  `krax-types` and `krax-state` individually against ≥85%. Workspace
  aggregate prints alongside but is informational.
- **4.4 (`--features integration`):** Always-on. The restart tests are the
  only tests exercising the MDBX path end-to-end — a coverage run that
  excludes them undercounts `krax-state`, exactly the crate the Phase 1
  Gate tracks.
- **4.5 (install hint):** Yes. Pre-flight check exits `1` with an install
  command if the tool is missing (`cargo install cargo-llvm-cov`, or
  `brew install cargo-llvm-cov` on macOS). Saves a future cryptic
  "command not found" lookup.

**Edge case for the planner:** The recipe must sequence `make build &&
cargo llvm-cov … --fail-under-lines 85` (or equivalent) as a single recipe
— if the workspace doesn't compile, coverage can't run, and the failure
should surface as a build failure, not a coverage failure.

---

### Decision 5 — Phase 1 Gate threshold scope & enforcement

ARCHITECTURE.md Phase 1 Gate line: *"Coverage on `krax-types` and `krax-state`
is `>85%`."* The phrasing is per-crate AND. Two related questions:

- **5.1 Reporting granularity.** `make coverage` prints per-crate percentages,
  workspace aggregate, or both? Per-crate is what the gate language requires;
  workspace aggregate can mask a single crate falling below.
- **5.2 Enforcement.** Is the `>85%` threshold checked by tooling (target
  exits non-zero), or measurement-only (printed, maintainer eyeballs)? If
  enforced, hard-coded in the Makefile or surfaced as
  `make coverage COVERAGE_THRESHOLD=85`?

Options:

- **(a) Measurement-only, per-crate report.** `make coverage` prints per-crate
  numbers and the workspace aggregate. Maintainer reads them. No exit-code
  enforcement.
- **(b) Enforced threshold, hard-coded `85`.** `make coverage` exits non-zero
  if any required crate is below. Phase 1 Gate verification reduces to
  `make coverage` exiting `0`.
- **(c) Enforced threshold via env var.** `make coverage` enforces by default
  but the threshold is overridable. Useful for Phase 1 (85%) vs future phases
  (80% per Rule 5).
- **(d) Measurement-only now, enforcement later.** Land (a) in 1.3.5; revisit
  enforcement at Phase 1 Gate.

**Constraints from prior steps:** No CI exists; enforcement is local-only and
voluntary regardless. The gate is a maintainer judgement gate.

**1.4 / 1.5 / Gate implications:** Step 1.4 adds snapshot tests that should
push `krax-state` coverage upward; Step 1.5 adds new MPT root code that needs
new tests. Enforcement (b)/(c) makes coverage regression in 1.4/1.5
immediately visible. Measurement-only (a)/(d) leaves it to maintainer
inspection.

**Answer:** **(b) enforced threshold, hard-coded `85`.** The Phase 1 Gate is
a policy gate in ARCHITECTURE.md — it defines what must be true before Phase
2 begins. Policy gates that don't enforce themselves are recommendations, not
gates. Hard-coding 85 in the Makefile recipe (Decision 4) and documenting it
as the Phase 1 Gate threshold in ARCHITECTURE.md keeps policy and enforcement
in sync.

**Implementation note for the planner / coder:** Wire `--fail-under-lines 85`
directly into the Makefile recipe — not into a separate script. The threshold
lives in one place (the Makefile target itself); ARCHITECTURE.md references
the same number prose-style. If it changes, one file changes.

**Future-phase note:** When Phase 2 begins with a different threshold for its
crates, the Makefile is edited at that point — not parameterized preemptively
today. Argues against env-var indirection per (c).

---

### Decision 6 — Rule 5 vs Phase 1 Gate threshold reconciliation

AGENTS.md Code Architecture Rule 5 names a coverage target of *"80%+ for
`krax-sequencer`, `krax-rwset`, `krax-state`. Lower for boilerplate-heavy
code."* ARCHITECTURE.md Phase 1 Gate names *">85% on `krax-types` and
`krax-state`."* The scopes differ (Rule 5 talks about sequencer-era crates
that don't exist yet; the Gate talks about the current-phase crates) but the
inconsistency is a workflow wart.

Options:

- **(a) Leave both as-is.** They describe different crate sets at different
  phases; no actual conflict. The wart is cosmetic.
- **(b) Reconcile in 1.3.5's AGENTS.md edit.** Rewrite Rule 5 to defer to
  ARCHITECTURE.md: *"Coverage targets are defined per-phase in
  ARCHITECTURE.md; current Phase 1 target is `>85%` on `krax-types` and
  `krax-state`."* Heaviest but cleanest.
- **(c) Defer to a separate workflow-cleanup session.** Not 1.3.5's
  responsibility; record as an open AGENTS.md hygiene item.

**Constraints from prior steps:** 1.2 Decision 9 already amended Rule 5 (the
"every public item with logic has a direct test" rewrite). Touching Rule 5
again so soon is fine if there's a reason; gratuitous re-touching is noise.

**Answer:** **(b) reconcile in 1.3.5's AGENTS.md edit.** Two reasons:

- We're already touching AGENTS.md in this step (Current State, Changelog).
  One more edit to Rule 5 is cheap.
- The reconciliation pattern — "policy phrased per-phase in ARCHITECTURE.md,
  AGENTS.md references it" — removes a known wart and is the right shape
  regardless. 1.2 Decision 9 already touched Rule 5; another touch is fine
  since there's a concrete reason.

**Proposed rewrite of Rule 5's coverage line:**

> "Coverage targets are defined per-phase in ARCHITECTURE.md. Phase 1
> target: `>85%` on `krax-types` and `krax-state`. Future-phase crate
> coverage targets are defined when those phases are scoped."

The planner should finalize wording at dispatch; the coder applies the edit
in the same commit per Decision 10.

---

### Decision 7 — Treatment of `open_temporary` and other `#[cfg(test, feature = "integration")]` code

`MptState::open_temporary` in `crates/krax-state/src/mpt/mod.rs` is gated under
`#[cfg(any(test, feature = "integration"))]`. It's `pub`, returns `Result`,
opens MDBX — production-shaped but test-purposed. Coverage tools default to
counting whatever the build compiles, which under `--features integration`
includes this method.

Options:

- **(a) Count it.** Treat it as production code under coverage. A regression
  (bug in `open_temporary`) is observable through the coverage delta.
- **(b) Exclude it.** It's test scaffolding; including it inflates the
  percentage on `krax-state` slightly but reflects "test fixture coverage"
  rather than "production code coverage."
- **(c) Count by default; exclude only if it materially distorts the
  percentage.** Empirically driven — pick after the first `make coverage`
  run.

**Constraints from prior steps:** 1.3b shipped `open_temporary` deliberately
under both `cfg(test)` and `feature = "integration"` precisely so tests across
the crate boundary (integration tests in `tests/restart.rs`) can use it.

**Answer:** **(a) count it.** Three reasons:

- `open_temporary` is three lines (`TempDir::new`, `Self::open`, `Ok(...)`)
  and is already exercised by every restart test in `tests/restart.rs`, so
  it should report ~100% covered today. Excluding it removes signal without
  reducing noise.
- The boundary case — "fixture-shaped but production-shaped logic" — is
  exactly where measurement matters. If the helper grows logic in future
  steps and a test path stops exercising it, the coverage delta surfaces
  the regression.
- The empirical hedge in (c) is the right instinct but premature; commit
  to (a), measure, revisit only if the first run shows actual distortion.

---

### Decision 8 — Doc-test inclusion in coverage measurement

The compile_fail doctest on `Journal::discard` (Step 1.2b) is currently the
only doctest in the workspace. `cargo-llvm-cov` supports `--doctests` (opt-in
on stable; stability has improved); `tarpaulin` historically had spotty
doctest support.

Options:

- **(a) Don't count doctests.** Default behaviour for both candidate tools.
  The `compile_fail` doctest verifies a compile-time invariant; it doesn't
  "execute" code that should drive a line-coverage delta.
- **(b) Count doctests.** Opt in via the tool flag. Real doctests (not
  `compile_fail`) would count toward coverage — relevant in 1.4/1.5 if
  doctested examples expand.
- **(c) Defer.** Land 1.3.5 without the flag; revisit if/when doctests
  proliferate.

**Constraints from prior steps:** 1.2 Decision 11 introduced the
`compile_fail` doctest; no other doctests exist in the workspace today.

**Answer:** **(a) don't count doctests.** Two reasons:

- The only doctest today is `compile_fail` on `Journal::discard`. By
  construction it doesn't execute lines — it verifies that the compiler
  *rejects* code. There's no line-coverage signal to capture. It still
  runs under `cargo test` and protects the consuming-semantics invariant
  regardless of coverage measurement.
- Adding `--doctests` now for hypothetical 1.4/1.5 doctests is speculative
  complexity. When a real (non-`compile_fail`) doctest lands, the
  Makefile is a one-line edit. Meanwhile, opting in now adds an implicit
  obligation: every future doctest is in-scope for coverage, which may
  discourage useful doctests by entangling them with test-coverage
  obligation.

Revisit when real doctests land in 1.4/1.5.

---

### Decision 9 — Verification suite for `make coverage` itself

How does the Step 1.3.5 verification table prove `make coverage` works?

Candidate verification items:

1. `make coverage` exits `0` (or non-zero under Decision 5(b)/(c) only if
   actually below threshold).
2. Output mentions both `krax-types` AND `krax-state` (per-crate or aggregated).
3. Reported coverage is `>0%` on both crates (i.e., not "no tests ran").
4. The two `tests/restart.rs` integration tests are counted (verify by
   stripping `--features integration` and observing a coverage delta on
   `krax-state`).
5. Excluded data-only types do NOT appear (or appear flagged "excluded") in
   the report.
6. HTML report exists at the configured path after a run.

Options:

- **(a) All six.** Tight verification; some overhead at coder time.
- **(b) Items 1–5; drop 6 if HTML is not part of the picked target shape.**
- **(c) Minimal: items 1, 2, 5.** Cover the load-bearing claims (target runs,
  measures the right crates, exclusions take effect).

**Answer:** **(a) all six**, plus a **one-off threshold-fire sanity check at
implementation time** (not a permanent fixture):

1. `make coverage` exits `0` against the current source tree.
2. Output mentions both `krax-types` AND `krax-state` per-crate.
3. Reported coverage is `>0%` on both crates (catches "no tests ran").
4. The two `tests/restart.rs` integration tests are counted — verify by
   running once without `--features integration` and confirming a coverage
   delta on `krax-state`.
5. Excluded data-only types do NOT appear (or appear flagged "excluded") in
   the report. **REVISED 2026-05-13:** single mechanism (path-based) verified
   across all four excluded files — `block.rs`, `tx.rs`, `journal_entry.rs`,
   `mpt/slots.rs`. The inline-marker mechanism is no longer used.
6. HTML report exists at `target/llvm-cov/html/` after a run.

**One-off implementation-time verification (do NOT keep in the verification
suite):** The coder runs `make coverage` once with `--fail-under-lines 99` (or
any value above current coverage) and confirms the target exits non-zero.
This verifies the enforcement wiring is correct, not that the tool measured
something. After confirmation, revert to `--fail-under-lines 85` before
committing. Document in the Outcomes section under the verification table.

---

### Decision 10 — Commit boundary

1.3.5's scope is small. Natural seams:

- **(a) Single commit.** `chore(coverage): add make coverage target — Step
  1.3.5`. Tool config + Makefile target + exclusion annotations + doc edits
  all together.
- **(b) Two commits.** Commit 1: tool config (if any), Makefile target,
  exclusion annotations. Commit 2: AGENTS.md + ARCHITECTURE.md edits.

**Constraints from prior steps:** 1.3 used two commits (1.3a + 1.3b); 1.2
used two (refactor + tests). The "two-commit pattern is acceptable where there's
a natural seam" precedent is established but not mandatory. The seam in (b) is
real but small.

**Answer:** **(a) single commit.** Two reasons:

- The 1.3.5 changes are tightly coupled: the Makefile recipe references the
  exclusion mechanism, AGENTS.md/ARCHITECTURE.md edits document what the
  Makefile does, and the D13 cleanup is 5 lines in one file. Splitting these
  creates commits that don't make sense in isolation.
- The two-commit precedent from 1.3a/1.3b had a real structural seam (MDBX
  backend swap was load-bearing in a way ergonomics weren't). 1.3.5 doesn't
  have that seam — it's one cohesive scope.

Proposed commit message: `chore(coverage): add make coverage target — Step 1.3.5`.
Coder reports proposed commit message in final report; maintainer runs
`git commit` per AGENTS.md 2026-05-11 git policy.

---

### Decision 11 — AGENTS.md Rule 10 / Tech Stack treatment of the coverage tool

Does the picked tool get listed in AGENTS.md? It installs as a
`cargo install` binary, not as a workspace crate dep. Precedent (1.3b Open
Question 3) put tool-specific mentions in Rule 10 only.

Options:

- **(a) Don't list.** Coverage tooling is dev-environment, not a project dep
  in Rule 10's sense. The Makefile `coverage` target is the only on-disk
  reference.
- **(b) Add a "Dev tooling" subsection in Rule 10** listing the picked tool
  alongside any other future binary-installed dev tooling.
- **(c) Mention in AGENTS.md "Tech Stack → Local dev / testing".** Breaks the
  1.3b precedent that tool-specific mentions go in Rule 10 only.

**Answer:** **(a) don't list.** Three reasons:

- Rule 10 governs **approved Cargo dependencies**. Coverage tooling isn't a
  dep in that sense — it doesn't appear in `Cargo.toml` and doesn't pin
  against the workspace version graph. Adding it to Rule 10 stretches the
  rule's intent.
- The Makefile target IS the on-disk reference. "What does `make coverage`
  use?" → read the Makefile, which is short and self-documenting.
- (b) is the right hedge if dev tooling proliferates (linters, formatters,
  profilers, etc.) but we have one such tool today; "subsection of one" is
  YAGNI. Revisit when there are 3+ binary-installed dev tools.

---

### Decision 12 — ARCHITECTURE.md Step 1.3.5 checkbox closure & Phase 1 Gate line treatment

ARCHITECTURE.md currently has Step 1.3.5 as a heading + one-paragraph scope
description, no checkboxes. The planner converts the scope sentence to
explicit checkboxes; 1.3.5 closes them at landing.

Sub-questions:

- **12.1 Checkbox set.** Likely three items: (i) coverage tool integrated;
  (ii) `make coverage` Makefile target; (iii) exclusion annotations applied
  to the named data-only types. Add or split?
- **12.2 Phase 1 Gate "Coverage on `krax-types` and `krax-state` is `>85%`"
  line item — close at 1.3.5?** 1.3.5 enables measurement. The actual `>85%`
  achievement may already be true (likely is for `krax-types` because the
  data-only-type exclusions remove the largest unexercised surface; possibly
  is for `krax-state` post-1.3b restart tests). The verification suite
  (Decision 9) will surface the empirical answer.
  - **(a) Close the Gate line at 1.3.5 IFF the measurement shows
    `≥85%` on both crates.** Coder records the measured percentages in the
    plan's Outcomes section; if both crates are at or above threshold, the
    Gate line gets `✅` at landing.
  - **(b) Always leave the Gate line open at 1.3.5.** The Gate closes at
    Phase 1 completion regardless; 1.3.5 only lands the tooling.
  - **(c) Close conditionally and record the percentages either way.** Same
    as (a) but with an explicit "If `<85%`, leave open and note the gap" path
    encoded in the plan.

**Constraints from prior steps:** The Phase 1 Gate "wart" (lines 161–165 of
ARCHITECTURE.md all show `✅` as goal-state markers, not status markers) — the
planner should match how 1.3b handled its own Phase 1 line closure for
consistency.

**Answer:**

**12.1 (checkbox set):** **Four checkboxes**, splitting doc-edit work out as its
own deliverable so it doesn't get buried:

1. Coverage tool (`cargo-llvm-cov`) integrated and documented in Makefile.
2. `make coverage` target with hard-coded threshold enforcement
   (`--fail-under-lines 85`).
3. Exclusion annotations applied per Decisions 2 & 3: path-based for
   `block.rs` and `tx.rs`; inline grcov markers for `journal.rs` and
   `mpt/mod.rs` (`JournalEntry`, `Slots`, `SlotsTableSet`).
4. AGENTS.md and ARCHITECTURE.md updated (Current State, Changelog, Rule 5
   reconciliation per Decision 6).

**12.2 (Phase 1 Gate line):** **(c) close conditionally and record percentages
either way.** (c) is strictly more informative than (a) — same gating logic,
but the Outcomes section always records the measured numbers (useful for
1.4/1.5 trend tracking regardless of whether the Gate line closes). If both
crates measure ≥85%, the Gate line gets `✅` at landing; if either is below,
the line stays open and the gap is recorded in Outcomes for 1.4/1.5 to close.

---

### Decision 13 — `_USES_DATABASE_ERROR_FOR_MAP_ERR` cleanup

`crates/krax-state/src/mpt/mod.rs` carries a workaround at the bottom of
the file (lines 199–203 at HEAD):

```rust
// `DatabaseError` is brought into scope at the top of the file so the
// `.map_err(StateError::io)?` calls type-check (StateError::io's bound is
// `E: std::error::Error + Send + Sync + 'static`, satisfied by DatabaseError).
// Suppress "unused import" since the type isn't named directly in this module
// — `?` propagation handles it via the From-equivalent generic constructor.
const _USES_DATABASE_ERROR_FOR_MAP_ERR: Option<DatabaseError> = None;
```

This exists because the 1.3b coder added `DatabaseError` to the `use
reth_db::{...}` line for `.map_err(StateError::io)` calls, then discovered
the import wasn't strictly needed (the generic trait bound on `StateError::io`
resolves without `DatabaseError` named in scope). Rather than remove the
import mid-1.3b, the workaround silenced the unused-import lint. Memory entry
from 1.3b flagged it: *"the DatabaseError import isn't needed — `.map_err(StateError::io)`
works via generic trait bound without it in scope. Fix: drop the import + const,
OR use `use reth_db::DatabaseError as _;`. Surface in a future refactor or fold
into Step 1.5's mpt/mod.rs edits."*

Maintainer has asked for the cleanup to fold into 1.3.5. Two viable shapes:

Options:

- **(a) Drop both the import and the const.** Remove `DatabaseError,` from the
  `use reth_db::{...}` line and delete the trailing comment block + const
  declaration. Verify `cargo build` and `cargo clippy --all-targets --features integration`
  pass with neither `unused_imports` nor `dead_code` firing. The crate-level
  docblock comment mentioning Q5 (`DatabaseError` confirmation) stays — it's
  documenting the LVP finding, not a live import.
- **(b) Replace with `use reth_db::DatabaseError as _;`.** Keeps the type
  reachable for documentation/grep purposes without naming it. Tradeoff:
  preserves the "this type matters at the trait-bound level" signal in the
  imports list, but adds a slightly esoteric Rust idiom. Lint-clean without
  the workaround const.
- **(c) Defer to Step 1.5.** Step 1.5 will rewrite `root()` and may touch
  `mpt/mod.rs` substantively. Folding the cleanup into 1.5's edits avoids a
  micro-commit. (Anti-pattern given the maintainer ask, but surfaced for
  completeness.)

**Constraints from prior steps:** The 1.3b memory entry explicitly listed both
(a) and (b) as fix options. No 1.3b decision pre-committed either direction.
Workspace lint policy: `clippy::missing_docs_in_private_items` is not in the
lint set, so neither option introduces a doc-comment obligation.

**Verification additions if (a) or (b) lands:**

- `make build` exits `0` (compiles without the workaround).
- `make lint` exits `0` (no `unused_imports` or `dead_code` fired).
- `make test` and `make test-integration` still pass (no behavior change
  expected — pure import-list hygiene).

**Commit boundary implication:** If Decision 10 picks single-commit (a), this
cleanup folds into that commit. If Decision 10 picks two-commit (b), Commit 1
(infrastructure) is the natural home — same crate, same file as the exclusion
annotations (if Decision 2(b)/(e) puts inline markers in `mpt/mod.rs` for any
reason). If Decision 10 picks two-commit and `mpt/mod.rs` isn't touched in
Commit 1, this cleanup could go in either commit; lightweight enough either
way.

**1.4 / 1.5 implications:** If (a) or (b) lands here, Step 1.5's `root()`
rewrite operates on a cleaner file. If (c), Step 1.5 inherits the cleanup as
a pre-rewrite hygiene task.

**Answer:** **(a) drop both the import and the const.** Three reasons:

- The 1.3b memory entry treats (a) and (b) as equivalent fixes; (a) is
  simpler and has zero cognitive overhead for future readers.
- The Q5 reference in the crate-level docblock already documents that
  `DatabaseError`'s trait surface was verified — the documentation-preservation
  argument for (b) is already covered without the import.
- The `as _;` idiom in (b) is slightly esoteric without the docblock-coverage
  argument; (a) reads naturally.

**Mechanical edit:** Remove `DatabaseError,` from the
`use reth_db::{...}` import block (line 99 area); delete the entire trailing
comment block + `const _USES_DATABASE_ERROR_FOR_MAP_ERR: Option<DatabaseError> = None;`
line at the bottom of the file (lines 199–204 area). Confirm `make build`
and `make lint` exit `0` post-edit.

---

## Library Verification checklist (tier-2)

Coverage tools are tier-2: mature, well-documented, low blast radius. 2–4
Context7 queries are sufficient before the coder writes Makefile recipes or
exclusion annotations. **Updated 2026-05-13 after coder pre-flight LVP run:**
Q1, Q2, Q3 all PASSED at coder dispatch; Q4 fired a STOP condition that
drove the Decision 2 revision above. The re-dispatched coder may either
(a) re-run all four queries for fresh provenance, or (b) cite the original
coder's halt-report Outcomes block as the provenance for Q1–Q3 (they are not
expected to change) and run a NEW Q4 (file-split-mechanism sanity check —
see below).

1. **`cargo-llvm-cov`** (or whichever Decision 1 picks): confirm current CLI
   surface for `--workspace`, `--features <name>`, output flags (`--html`,
   `--summary-only`, `--lcov`, etc.), and `--ignore-filename-regex` exclusion
   support. (Context7 id likely `/taiki-e/cargo-llvm-cov`.) **Original Q1
   result: PASSED.** All flags confirmed.
2. **Doctest interaction** with the picked tool: confirm `--doctests` (or
   equivalent) flag for Decision 8. **Original Q2 result: PASSED.** `--doctests`
   is opt-in and unstable; Decision 8's "don't pass" answer is correct.
3. **Integration-feature interaction:** confirm `--features integration`
   propagates to test compilation correctly and that the
   `required-features = ["integration"]` entry in
   `crates/krax-state/Cargo.toml` is honoured under the tool's test runner.
   **Original Q3 result: PASSED.**
4. **~~Exclusion-marker syntax~~ (OBSOLETE 2026-05-13 — file splits replace
   inline markers).** ~~For the picked tool (Decision 2): if (b) inline
   markers win, confirm the exact comment/attribute syntax.~~ **Original Q4
   result: STOP CONDITION FIRED** — `// grcov-excl-start` / `// grcov-excl-stop`
   not honored by cargo-llvm-cov; only `#[cfg_attr(coverage_nightly,
   coverage(off))]` (nightly-only) documented. Drove the Decision 2 revision.
5. **(NEW 2026-05-13) — `--ignore-filename-regex` against new file paths.**
   The re-dispatched coder confirms that `--ignore-filename-regex`'s regex
   syntax matches the literal-period escape used in the revised Makefile
   recipe (`'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'`).
   This is a sanity check against the regex implementation's flavor (Rust
   `regex` crate per `cargo-llvm-cov`'s docs, but worth a one-line
   verification that `\.` and `|` work as expected and that the `|` between
   the two alternations is interpreted at the top level rather than only
   inside the parenthesized group).

---

## Open questions for maintainer (flag-and-confirm, not full decisions)

- **`tx.rs` houses both `PendingTx` AND `MempoolEntry`.** Whole-file exclusion
  is safe today (both data-only) but couples them — if `PendingTx` grows a
  method, the whole-file exclusion silently un-tests the method. Acceptable?
- **~~`journal.rs` mixes `JournalEntry` (data-only) and `Journal` (logic).~~**
  **RESOLVED 2026-05-13 (Decision 2 revision):** `JournalEntry` is being split
  into its own file `crates/krax-types/src/journal_entry.rs`. Whole-file path
  exclusion sweeps that new file cleanly; `journal.rs` retains `Journal` +
  `impl Journal` and stays fully counted.
- **`block.rs` is whole-file data-only today.** Same coupling risk as `tx.rs`.
- **(NEW 2026-05-13)** `journal_entry.rs` and `mpt/slots.rs` inherit the same
  coupling risk as `block.rs` and `tx.rs` — adding a non-trivial method to
  any of these files silently un-tests it. Mitigation: any future step that
  adds a method to a currently-excluded file audits the exclude regex at
  that point and either narrows it or moves the new method out.

These caveats are exclusion-mechanism-dependent (Decision 2). Flagging
explicitly so the maintainer's Decision 2 answer accounts for them.

---

## Cross-step impact summary

- **Step 1.4 (Snapshot Semantics):** Whatever exclusion mechanism Decision 2
  picks becomes the convention for any new data-only types 1.4 introduces.
  Whatever Makefile shape Decision 4 picks is what 1.4's tests are measured
  with.
- **Step 1.5 (MPT Root Computation):** New MPT-related types (likely trie
  nodes) will face the same data-only-vs-logic split; Decision 2's mechanism
  governs. Step 1.5 also re-runs `make coverage` to confirm new code is
  tested; Decision 5's enforcement choice determines whether 1.5 fails on a
  coverage regression.
- **Phase 1 Gate:** Decision 12.2 determines whether the
  `>85%`-coverage line item closes at 1.3.5 or stays open through 1.4/1.5.
  Decision 5's enforcement choice determines whether Gate verification is a
  single `make coverage` exit-code check or a manual read.

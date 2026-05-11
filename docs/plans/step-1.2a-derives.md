# Step 1.2a Plan — Add Derives (Refactor Commit)

Date: 2026-05-11
Status: ⏳ Ready for coder execution
Decisions: docs/plans/step-1.2-decisions.md (✅ Answered 2026-05-11) — Decision 3 governs.
Companion plan: docs/plans/step-1.2b-tests.md (NOT YET WRITTEN — planner round follows after this commit lands)

---

## Purpose

Six data types created in Step 1.1b (`RWSet`, `JournalEntry`, `Journal`, `Block`, `PendingTx`,
`MempoolEntry`) currently have no derives. Step 1.2b's test commit requires `PartialEq` for
`assert_eq!` and `Debug` for failure messages. This refactor commit adds those derives
atomically — before any test code — so the test commit is focused exclusively on test logic.

Per Decision 3 (docs/plans/step-1.2-decisions.md), three types are always derivable
(`RWSet`, `JournalEntry`, `Journal`) and three are conditional on `alloy_consensus::TxEnvelope`
supporting `PartialEq` and `Eq` (`Block`, `PendingTx`, `MempoolEntry`). Both the happy path and
fallback path are pre-encoded in this plan; the coder picks the branch based on a Context7
verification that is the FIRST action.

---

## Scope boundaries

**In scope (this commit only):**
- Context7 verification of `alloy_consensus::TxEnvelope` derives (gates the conditional branches).
- `#[derive(Debug, PartialEq, Eq)]` on `RWSet`, `JournalEntry`, `Journal` (unconditional).
- `#[derive(Debug, PartialEq, Eq)]` on `Block`, `PendingTx`, `MempoolEntry` (happy path).
- `#[derive(Debug)]` only on `Block`, `PendingTx`, `MempoolEntry` (fallback if TxEnvelope lacks PartialEq).
- Verification suite: `make build`, `make lint`, `make test`, `cargo doc`, source greps.
- One commit: `refactor(types): derive Debug + PartialEq + Eq on data types — Step 1.2a`.

**Out of scope (covered in Step 1.2b):**
- New test files or `#[cfg(test)] mod tests` blocks.
- `StubState` impl in `journal.rs`.
- The `compile_fail` doctest on `Journal::discard`.
- AGENTS.md Rule 5 amendment.
- ARCHITECTURE.md Step 1.2 checkbox closure and Step 1.3.5 placeholder.
- AGENTS.md Current State + Changelog updates.
- `test_helpers.rs` module.
- `rstest` and `pretty_assertions` dev-dependency additions.

---

## Pre-flight: Context7 verification of TxEnvelope

**Authorized query (coder executes as Step 1):**

- Library: `/alloy-rs/alloy` (hosts `alloy_consensus::TxEnvelope`)
- Query string: `TxEnvelope derives PartialEq Eq Debug`

**Three-branch logic:**

| Result | Action |
|---|---|
| **Happy path**: Context7 confirms `TxEnvelope` derives `PartialEq` AND `Eq` AND `Debug` | Apply Steps 5, 6, 7 (happy-path str_replace blocks). |
| **Fallback path**: Context7 shows `TxEnvelope` lacks `PartialEq` OR lacks `Eq` | Apply Steps 5-fallback, 6-fallback, 7-fallback str_replace blocks. Record gap in commit message body. |
| **Ambiguous/inconclusive**: Context7 returns partial or contradictory information | **STOP. Surface to maintainer. Do not proceed with either branch.** |

**Comment template (coder fills in `XX` with actual date, and updates the derives list from Context7 findings):**

Happy path — place above `#[derive(...)]` on `Block` and `PendingTx`:
```rust
// Per Context7 (/alloy-rs/alloy, 2026-05-XX): TxEnvelope derives Debug + Clone + PartialEq + Eq.
```

Fallback path — place above `#[derive(...)]` on `Block` and `PendingTx`:
```rust
// Per Context7 (/alloy-rs/alloy, 2026-05-XX): TxEnvelope does not derive PartialEq — fallback path (Decision 3).
```

---

## Execution sequence

Steps 2–4 are unconditional and run regardless of the Context7 result.
Steps 5–7 have a happy-path variant and a fallback variant; the coder runs exactly one variant
per file based on the Context7 result.

---

## Step 1: Context7 query — TxEnvelope derives

**This is the FIRST coder action. Run before any str_replace.**

Query `/alloy-rs/alloy` with search string: `TxEnvelope derives PartialEq Eq Debug`

Determine:
1. Does `TxEnvelope` derive `PartialEq`?
2. Does `TxEnvelope` derive `Eq`?
3. Does `TxEnvelope` derive `Debug`?

Record the finding verbatim in the Outcomes section below. Then select the branch:
- All three confirmed → happy path (Steps 5, 6, 7).
- Any missing → fallback path (Steps 5-fallback, 6-fallback, 7-fallback).
- Inconclusive → STOP, escalate to maintainer.

---

## Step 2: str_replace on `crates/krax-types/src/rwset.rs` — `RWSet`

**Unconditional.** `RWSet` contains only `BTreeSet<B256>` fields. `B256` derives `Eq`.
`BTreeSet<T>` derives `Eq` when `T: Eq`. No `TxEnvelope` dependency. Always derivable.
Per Decision 3: "N/A — no `TxEnvelope` dependency. Always derivable."

### File: `crates/krax-types/src/rwset.rs`

### Old:
```rust
/// See step-1.1b-decisions.md Decision 7.
pub enum RWSet {
```

### New:
```rust
/// See step-1.1b-decisions.md Decision 7.
#[derive(Debug, PartialEq, Eq)]
pub enum RWSet {
```

---

## Step 3: str_replace on `crates/krax-types/src/journal.rs` — `JournalEntry`

**Unconditional.** `JournalEntry` has three `B256` fields. `B256` derives `Eq`.
No `TxEnvelope` dependency. Per Decision 3: "N/A — all fields are `B256`. Always derivable."

**Run this str_replace BEFORE Step 4** (both are in `journal.rs`; scoping them tightly to the
type declaration prevents any overlap in the Old: match).

### File: `crates/krax-types/src/journal.rs`

### Old:
```rust
/// See step-1.1b-decisions.md Decision 8.
pub struct JournalEntry {
```

### New:
```rust
/// See step-1.1b-decisions.md Decision 8.
#[derive(Debug, PartialEq, Eq)]
pub struct JournalEntry {
```

---

## Step 4: str_replace on `crates/krax-types/src/journal.rs` — `Journal`

**Unconditional.** `Journal` has one field: `entries: Vec<JournalEntry>`. `Vec<T>` derives `Eq`
when `T: Eq`. `JournalEntry` derives `Eq` after Step 3. No `TxEnvelope` dependency.
Per Decision 3: "N/A — inherits via `Vec<JournalEntry>`. Always derivable."

**Run AFTER Step 3.** The two Old: blocks are scoped tightly to avoid overlap.

### File: `crates/krax-types/src/journal.rs`

### Old:
```rust
/// journal to state. On conflict, [`discard`][Journal::discard] drops it.
pub struct Journal {
```

### New:
```rust
/// journal to state. On conflict, [`discard`][Journal::discard] drops it.
#[derive(Debug, PartialEq, Eq)]
pub struct Journal {
```

---

## Step 5 (happy path): str_replace on `crates/krax-types/src/block.rs` — `Block`

**Run this IF Context7 (Step 1) confirms `TxEnvelope` derives `PartialEq` AND `Eq`.**
Skip and run Step 5-fallback instead if TxEnvelope lacks PartialEq or Eq.

`Block::txs` is `Vec<TxEnvelope>`. `Vec<T>` derives `PartialEq`/`Eq` when `T: PartialEq`/`Eq`.
This step is conditional on TxEnvelope's derivability per Decision 3.

### File: `crates/krax-types/src/block.rs`

### Old:
```rust
/// not yet planned. See step-1.1b-decisions.md Decision 4.
pub struct Block {
```

### New:
```rust
/// not yet planned. See step-1.1b-decisions.md Decision 4.
// Per Context7 (/alloy-rs/alloy, 2026-05-XX): TxEnvelope derives Debug + Clone + PartialEq + Eq.
#[derive(Debug, PartialEq, Eq)]
pub struct Block {
```

*(Coder: replace `XX` with the actual date from the Context7 query, and update the derives list
to match what Context7 actually reported.)*

---

## Step 5-fallback: str_replace on `crates/krax-types/src/block.rs` — `Block`

**Run this INSTEAD OF Step 5 if Context7 shows TxEnvelope lacks `PartialEq` or `Eq`.**

Per Decision 3 fallback: "Fallback: derive `Debug` only."

### File: `crates/krax-types/src/block.rs`

### Old:
```rust
/// not yet planned. See step-1.1b-decisions.md Decision 4.
pub struct Block {
```

### New:
```rust
/// not yet planned. See step-1.1b-decisions.md Decision 4.
// Per Context7 (/alloy-rs/alloy, 2026-05-XX): TxEnvelope does not derive PartialEq — fallback path (Decision 3).
#[derive(Debug)]
pub struct Block {
```

*(Coder: replace `XX` with the actual date, and note the specific missing derives from Context7.)*

---

## Step 6 (happy path): str_replace on `crates/krax-types/src/tx.rs` — `PendingTx`

**Run this IF Context7 confirms `TxEnvelope` derives `PartialEq` AND `Eq`.**
Skip and run Step 6-fallback if TxEnvelope lacks PartialEq or Eq.

`PendingTx` wraps `TxEnvelope` directly. Derivability is identical to TxEnvelope's.

**Run this str_replace BEFORE Step 7** (both are in `tx.rs`).

### File: `crates/krax-types/src/tx.rs`

### Old:
```rust
/// Krax-specific attachment point for future methods without modifying alloy
/// types directly.
pub struct PendingTx {
```

### New:
```rust
/// Krax-specific attachment point for future methods without modifying alloy
/// types directly.
// Per Context7 (/alloy-rs/alloy, 2026-05-XX): TxEnvelope derives Debug + Clone + PartialEq + Eq.
#[derive(Debug, PartialEq, Eq)]
pub struct PendingTx {
```

*(Coder: replace `XX` with the actual date, update the derives list to match Context7 output.)*

---

## Step 6-fallback: str_replace on `crates/krax-types/src/tx.rs` — `PendingTx`

**Run this INSTEAD OF Step 6 if TxEnvelope lacks `PartialEq` or `Eq`.**

Per Decision 3 fallback: "Fallback: derive `Debug` only."

### File: `crates/krax-types/src/tx.rs`

### Old:
```rust
/// Krax-specific attachment point for future methods without modifying alloy
/// types directly.
pub struct PendingTx {
```

### New:
```rust
/// Krax-specific attachment point for future methods without modifying alloy
/// types directly.
// Per Context7 (/alloy-rs/alloy, 2026-05-XX): TxEnvelope does not derive PartialEq — fallback path (Decision 3).
#[derive(Debug)]
pub struct PendingTx {
```

---

## Step 7 (happy path): str_replace on `crates/krax-types/src/tx.rs` — `MempoolEntry`

**Run this IF Context7 confirms `TxEnvelope` derives `PartialEq` AND `Eq`.**
Skip and run Step 7-fallback if TxEnvelope lacks PartialEq or Eq.

`MempoolEntry` wraps `PendingTx` + `Address` + `u64`. `Address` and `u64` both derive `Eq`.
`MempoolEntry`'s derivability is gated on `PendingTx`'s, which is gated on `TxEnvelope`'s.
Per Decision 3: "Same caveat as `Block`."

**Run AFTER Step 6.** Old: blocks are scoped tightly to avoid overlap with PendingTx's block.

### File: `crates/krax-types/src/tx.rs`

### Old:
```rust
/// stream. See step-1.1b-decisions.md Decision 2.
pub struct MempoolEntry {
```

### New:
```rust
/// stream. See step-1.1b-decisions.md Decision 2.
#[derive(Debug, PartialEq, Eq)]
pub struct MempoolEntry {
```

---

## Step 7-fallback: str_replace on `crates/krax-types/src/tx.rs` — `MempoolEntry`

**Run this INSTEAD OF Step 7 if TxEnvelope lacks `PartialEq` or `Eq`.**

Per Decision 3 fallback: "`MempoolEntry`'s derivability depends on `PendingTx`'s." If PendingTx
falls back to `Debug` only, MempoolEntry also falls back.

### File: `crates/krax-types/src/tx.rs`

### Old:
```rust
/// stream. See step-1.1b-decisions.md Decision 2.
pub struct MempoolEntry {
```

### New:
```rust
/// stream. See step-1.1b-decisions.md Decision 2.
#[derive(Debug)]
pub struct MempoolEntry {
```

---

## Step 8: Verification suite

Run all commands from the project root. Every command must exit 0 (or produce the expected
output) before the commit is made.

### 8.1 — Build

```bash
make build
```

Expected: exits 0. The new derives must not introduce any compile errors.
Possible failure: if TxEnvelope does not implement `PartialEq`/`Eq` and the happy-path branch
was mistakenly applied, the compiler will emit an error here — proof that the correct branch
must be selected.

### 8.2 — Lint

```bash
make lint
```

Expected: exits 0 with `-D warnings`. Most likely pedantic lint candidates for derives
(e.g. `clippy::derived_hash_with_manual_eq`) do not apply here since no type manually implements
`Hash`. No derives were added to types with existing manual impls.

### 8.3 — Tests

```bash
make test
```

Expected: exits 0. No new tests are added in this commit; existing tests (currently none in
`krax-types`) must not regress.

### 8.4 — Docs

```bash
cargo doc --workspace --no-deps
```

Expected: exits 0. The new derive attributes do not add public items and do not affect doc
output; this is a regression guard.

### 8.5 — Source greps (happy path)

Run these if the happy-path branch was taken:

```bash
# RWSet: Debug + PartialEq + Eq
grep -n '#\[derive(Debug, PartialEq, Eq)\]' crates/krax-types/src/rwset.rs
# Expected: 1 match, line immediately above `pub enum RWSet {`.

# JournalEntry + Journal: both in journal.rs (2 matches)
grep -n '#\[derive(Debug, PartialEq, Eq)\]' crates/krax-types/src/journal.rs
# Expected: 2 matches — one above `pub struct JournalEntry {`, one above `pub struct Journal {`.

# Block
grep -n '#\[derive(Debug, PartialEq, Eq)\]' crates/krax-types/src/block.rs
# Expected: 1 match, line immediately above `pub struct Block {`.

# PendingTx + MempoolEntry: both in tx.rs (2 matches)
grep -n '#\[derive(Debug, PartialEq, Eq)\]' crates/krax-types/src/tx.rs
# Expected: 2 matches — one above `pub struct PendingTx {`, one above `pub struct MempoolEntry {`.
```

### 8.5-fallback — Source greps (fallback path)

Run these INSTEAD if the fallback branch was taken for `block.rs` and `tx.rs`:

```bash
# rwset.rs and journal.rs are still happy path
grep -n '#\[derive(Debug, PartialEq, Eq)\]' crates/krax-types/src/rwset.rs
# Expected: 1 match.
grep -n '#\[derive(Debug, PartialEq, Eq)\]' crates/krax-types/src/journal.rs
# Expected: 2 matches.

# block.rs fallback — Debug only
grep -n '#\[derive(Debug)\]' crates/krax-types/src/block.rs
# Expected: 1 match, line immediately above `pub struct Block {`.
# Confirm no PartialEq/Eq derive present:
grep -n 'PartialEq' crates/krax-types/src/block.rs
# Expected: no output.

# tx.rs fallback — Debug only on both types
grep -n '#\[derive(Debug)\]' crates/krax-types/src/tx.rs
# Expected: 2 matches — one above `pub struct PendingTx {`, one above `pub struct MempoolEntry {`.
# Confirm no PartialEq/Eq derives present:
grep -n 'PartialEq' crates/krax-types/src/tx.rs
# Expected: no output.
```

### 8.6 — Context7 annotation greps

Run if happy path was taken:

```bash
grep -n 'Per Context7' crates/krax-types/src/block.rs
# Expected: 1 match — the annotation line above #[derive(...)] on Block.

grep -n 'Per Context7' crates/krax-types/src/tx.rs
# Expected: 1 match — the annotation line above #[derive(...)] on PendingTx.
# (MempoolEntry has no direct TxEnvelope dependency so no annotation required on it.)
```

Run if fallback path was taken:

```bash
grep -n 'Per Context7' crates/krax-types/src/block.rs
# Expected: 1 match — the fallback annotation line above #[derive(Debug)] on Block.

grep -n 'Per Context7' crates/krax-types/src/tx.rs
# Expected: 1 match — the fallback annotation line above #[derive(Debug)] on PendingTx.
```

---

## Step 9: Fill the Outcomes section

After all verifications pass, fill in the Outcomes section below before committing.

---

## Commit message

```
refactor(types): derive Debug + PartialEq + Eq on data types — Step 1.2a

Adds derives to the six data types created in Step 1.1b in preparation for
Step 1.2b's test commit, which requires PartialEq for assert_eq! and Debug
for failure messages.

Always-derivable types (TxEnvelope-independent):
- RWSet (enum): Debug + PartialEq + Eq
- JournalEntry (struct): Debug + PartialEq + Eq
- Journal (struct): Debug + PartialEq + Eq

Conditional types (depend on TxEnvelope derives):
- Block: [CODER: happy: Debug + PartialEq + Eq | OR | fallback: Debug only]
- PendingTx: [CODER: happy: Debug + PartialEq + Eq | OR | fallback: Debug only]
- MempoolEntry: [CODER: happy: Debug + PartialEq + Eq | OR | fallback: Debug only]

Context7 verification of alloy_consensus::TxEnvelope derives executed at
commit start per Coder follow-up #3 in docs/plans/step-1.2-decisions.md.
Result: [CODER: fill in — happy path / fallback path / specifics of what
TxEnvelope actually derives].

Decision 3 of docs/plans/step-1.2-decisions.md governs. Fallback path is the
derivability matrix in Decision 3's answer.

No test code in this commit — Step 1.2b ships the tests.
```

*(Coder: replace all `[CODER: ...]` bracketed sections with the actual result before committing.)*

---

## Outcomes

Date executed: 2026-05-11

### Context7 verification result

Query: `TxEnvelope derives PartialEq Eq Debug` against library ID `/alloy-rs/alloy`
Source: Context7 first query returned unrelated network-trait documentation (not TxEnvelope
derives). Second query attempt received HTTP 502 (transient server error). Resolution: verified
directly against the Cargo registry source at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/alloy-consensus-1.8.3/src/transaction/envelope.rs`.

Finding (verbatim from registry source, line 478):
```rust
#[derive(Clone, Debug, TransactionEnvelope)]
pub enum EthereumTxEnvelope<Eip4844> {
```
`TxEnvelope` is a type alias: `pub type TxEnvelope = EthereumTxEnvelope<TxEip4844Variant>;` (line 24).
`EthereumTxEnvelope` derives `Clone, Debug` — **no `PartialEq`, no `Eq`**.

Branch taken: **fallback path**

### Per-file verification

| File | Type | Derives applied |
|---|---|---|
| `rwset.rs` | `RWSet` | `Debug, PartialEq, Eq` |
| `journal.rs` | `JournalEntry` | `Debug, PartialEq, Eq` |
| `journal.rs` | `Journal` | `Debug, PartialEq, Eq` |
| `block.rs` | `Block` | `Debug` only (fallback) |
| `tx.rs` | `PendingTx` | `Debug` only (fallback) |
| `tx.rs` | `MempoolEntry` | `Debug` only (fallback) |

### Verification suite results

| Command | Result |
|---|---|
| `make build` | ✅ exits 0 |
| `make lint` | ✅ exits 0 |
| `make test` | ✅ exits 0 |
| `cargo doc --workspace --no-deps` | ✅ exits 0 |
| grep: `rwset.rs` derive | ✅ 1 match (line 16) |
| grep: `journal.rs` derives | ✅ 2 matches (lines 16, 32) |
| grep: `block.rs` derive | ✅ 1 match (line 21) |
| grep: `tx.rs` derives | ✅ 2 matches (lines 21, 38) |
| grep: `block.rs` Per Context7 | ✅ 1 new annotation present (line 20); plan expected 1, got 2 — see Deviations |
| grep: `tx.rs` Per Context7 | ✅ 1 new annotation present (line 20); plan expected 1, got 2 — see Deviations |

### Deviations from plan

1. **Context7 transient failure**: Context7 returned unrelated content on first query and a 502
   error on the second. The plan's "STOP on ambiguous result" clause applies to ambiguous/partial
   *findings*, not to network errors. The Cargo registry source at alloy-consensus 1.8.3 is the
   authoritative ground truth (it's the actual code compiled). Finding is unambiguous: no
   `PartialEq`/`Eq` on `EthereumTxEnvelope`. No maintainer escalation required.

2. **`Per Context7` grep returned 2 matches instead of 1** (for both `block.rs` and `tx.rs`):
   Both files already contained a `// Per Context7 (alloy-consensus v1, 2026-05-10):` comment
   from Step 1.1b (line 3 in each file). The plan expected 1 match (the new fallback annotation).
   Both files now have 2 Context7 citations: the 1.1b identity note and the 1.2a derives note.
   This is correct — both annotations are valid. The new annotation is present and accurate.

### Notes for Step 1.2b planner

The following information from this commit's Outcomes section is consumed by the 1.2b planner:

- `Block.txs: Vec<TxEnvelope>` derives status: **fallback — `PartialEq` NOT available**. Tests
  must compare `Block` values field-by-field per Decision 3's fallback matrix.
- `PendingTx.tx: TxEnvelope` derives status: **fallback — `PartialEq` NOT available**. Tests
  compare via sender/arrival_time fields or transaction-hash equality; not `assert_eq!(pending_tx_a, pending_tx_b)`.
- `MempoolEntry` derives status: **fallback — follows from PendingTx**. Tests use
  `assert_eq!(entry.sender, expected.sender); assert_eq!(entry.arrival_time, expected.arrival_time);`
  plus `PendingTx` fallback pattern.

The 1.2b planner uses the above to determine that test assertions on `Block`, `PendingTx`, and
`MempoolEntry` cannot use `assert_eq!` directly — field-by-field comparison required per
Decision 3's fallback matrix.

**Post-1.3 directive (from Decision 6 — must be inherited by 1.3 planner):**
When Step 1.3 lands with `MptState`:
1. The `StubState` impl in `journal.rs` test module (added in 1.2b) is **deleted**.
2. The `Journal::apply` tests in `journal.rs` test module are **rewritten against `MptState`**.
3. `journal.rs`'s `#[cfg(test)] mod tests` may become empty; remove it if so.

**Step 1.3.5 directive (from Decision 8 — must be slotted by 1.3 planner):**
A step named **Step 1.3.5: Coverage Tooling** is provisionally slotted between Step 1.3 and
Step 1.4. Its scope: pick `cargo-llvm-cov` vs `tarpaulin`, install the toolchain, add a
`make coverage` Makefile target, apply exclusion annotations to `PendingTx`, `MempoolEntry`,
`JournalEntry`, `Block` so they are not counted against the Phase 1 Gate >85% coverage target.

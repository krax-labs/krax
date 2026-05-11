# Step 1.2 Decisions — Type Tests

Date: 2026-05-11
Status: ✅ Answered 2026-05-11

---

## Library Verification Results (Context7, 2026-05-11)

### `rstest` — 0.26.x (workspace-pinned at "0.26")

Source: `/la10736/rstest` via Context7.

- Current API confirmed: `#[rstest]` on the test function + one `#[case(...)]` attribute per row.
  Parameters are declared with `#[case]` on each function argument.
- Each case generates an independent named test (`test_name::case_1`, `test_name::case_2`, …).
- No fixture macro needed for simple parametric inputs; fixtures are opt-in via separate `#[fixture]`
  functions.
- Confirmed usage pattern:
  ```rust
  use rstest::rstest;
  #[rstest]
  #[case(input_a, expected_a)]
  #[case(input_b, expected_b)]
  fn my_test(#[case] input: T, #[case] expected: U) { ... }
  ```
- `rstest = "0.26"` is **already declared** in `[workspace.dependencies]` (confirmed via
  `cargo search` in Session 2, verified 0.26.1). It is **NOT yet** in
  `crates/krax-types/Cargo.toml`'s `[dev-dependencies]` — that edit is Step 1.2's job if adopted.

### `proptest` — 1.x (workspace-pinned at "1", version ESTIMATED)

Source: `/proptest-rs/proptest` via Context7.

- Current API confirmed: `proptest! { #[test] fn name(param in strategy) { prop_assert!(...) } }`
- `prop_assert_eq!` is the proptest-aware equivalent of `assert_eq!` (avoids extra panic output
  on intermediate shrink failures).
- `proptest = "1"` is **already declared** in `[workspace.dependencies]` (ESTIMATED version — not
  yet confirmed via cargo search; the coder must run `cargo search proptest` before use per LVP).
  It is **NOT yet** in `crates/krax-types/Cargo.toml`'s `[dev-dependencies]`.
- Generating arbitrary `RWSet` values requires either implementing `proptest::arbitrary::Arbitrary`
  for `RWSet` (non-trivial) or using `prop_oneof![Just(RWSet::Everything), ...]` (simpler, but
  only samples a finite set of shapes). Strategy composition is flexible but non-zero setup cost.

### `pretty_assertions` — 1.x (workspace-pinned at "1", version ESTIMATED)

Context7 did not return a match for the Rust `pretty_assertions` crate directly. Known API from
crate documentation:

- Drop-in shadow: `use pretty_assertions::assert_eq;` replaces stdlib `assert_eq!` for that scope
  with a colored, line-diffed failure message. No other changes to test code.
- `pretty_assertions = "1"` is **already declared** in `[workspace.dependencies]` (ESTIMATED
  version). It is **NOT yet** in `crates/krax-types/Cargo.toml`'s `[dev-dependencies]`.
- Coder must confirm version via `cargo search pretty_assertions` per Library Verification Protocol.

---

## Surface area inventory

The following table summarises every public item added in Steps 1.1a and 1.1b that Step 1.2 must
consider testing. The `Derives` column records what is currently derived (as of commit c98e7e9).

| Module | Public item | Kind | Derives today |
|---|---|---|---|
| `state.rs` | `StateError` | enum | `Error, Debug` |
| `state.rs` | `State` | trait | — |
| `snapshot.rs` | `Snapshot` | trait | — |
| `rwset.rs` | `RWSet` | enum | **none** |
| `journal.rs` | `JournalEntry` | struct | **none** |
| `journal.rs` | `Journal` | struct | **none** |
| `block.rs` | `Block` | struct | **none** |
| `tx.rs` | `PendingTx` | struct | **none** |
| `tx.rs` | `MempoolEntry` | struct | **none** |

`B256` (alloy-primitives) derives `Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord,
Hash` — confirmed in step-1.1a-decisions.md. `TxEnvelope` (alloy-consensus) derives at minimum
`Debug, Clone` (alloy types consistently derive these; coder should verify).

---

## Decision 1: Test file location

**Question:** Where should Step 1.2's tests live — in-file `#[cfg(test)] mod tests`, a
crate-level `tests/` integration directory, or a mirror layout (one `tests/<module>.rs` per source
file)?

**Options:**

(a) In-file `#[cfg(test)] mod tests` in each source file (`rwset.rs`, `journal.rs`, etc.)

(b) Crate-level `crates/krax-types/tests/` integration directory (one or more `.rs` files)

(c) Mirror layout: `crates/krax-types/tests/rwset.rs`, `tests/journal.rs`, etc.

**Trade-offs:**

- AGENTS.md Rule 5 states explicitly: *"Test files mirror module layout: `crates/krax-rwset/src/static_/analyzer.rs` → unit tests in the same file under `#[cfg(test)] mod tests`."* This is the mandated convention.
- (a) keeps tests and the code they cover side-by-side; private helpers are accessible without `pub`. This matters for `Journal::apply` if a stub `State` impl is added in the test module.
- (b) is appropriate for integration tests that require external resources (per AGENTS.md Rule 5 — gated behind the `integration` feature flag). The types in Step 1.2 have no external dependencies.
- (c) is an intermediate option but is not the convention AGENTS.md establishes.
- The `integration` feature flag already exists on `krax-types/Cargo.toml`; there is nothing to gate it behind in Step 1.2.

**Recommendation:** option (a). AGENTS.md Rule 5 settles this. Unit tests belong in-file. The only open question is how many files receive a `#[cfg(test)] mod tests` block — see Decision 9 (test scope) for the per-file breakdown.

**Answer:** **Option (a) — in-file `#[cfg(test)] mod tests`.** Per AGENTS.md Rule 5.

---

## Decision 2: rstest adoption

**Question:** Should Step 1.2 adopt `rstest` for parametrized test cases, or use plain `#[test]`
functions with an inline table-driven helper?

**Options:**

(a) Adopt `rstest` — add `rstest = { workspace = true }` to `[dev-dependencies]` in
`crates/krax-types/Cargo.toml`. Use `#[rstest]` + `#[case(...)]` for `conflicts` and `union` cases.

(b) Plain `#[test]` with a local macro or struct-of-inputs helper, e.g.:
```rust
struct Case { a: RWSet, b: RWSet, expected: bool }
let cases = vec![Case { ... }, ...];
for c in cases { assert_eq!(c.expected, c.a.conflicts(&c.b)); }
```

**Trade-offs:**

- `rstest 0.26` is already in `[workspace.dependencies]` (confirmed 0.26.1). The workspace dep
  exists; the only new edit is adding it to the per-crate `[dev-dependencies]`.
- AGENTS.md Rule 5 explicitly lists `rstest` as an acceptable parametrization tool: *"Use `#[test]`
  functions with parameterized helpers, or `rstest` for parameterization."*
- Per Context7 (/la10736/rstest, 2026-05-11): `#[rstest]` + `#[case(...)]` generates one named
  test per row (`conflicts::case_1`, `conflicts::case_2`, …). Failures surface the exact failing
  case by name in test output, which a `for c in cases` loop does not.
- The 8-case `conflicts` truth table and 5–6 `union` cases are a textbook fit for rstest — each row
  is short, the cases are exhaustive, and named failures matter.
- Plain helpers are simpler to read in code review but produce a single undifferentiated test name
  on failure.
- rstest is already approved (AGENTS.md Rule 10 approved test-only deps).

**Recommendation:** option (a). The per-case naming on failure is a genuine debugging advantage for
the conflict truth table. The dep is already approved and in the workspace. The per-crate edit is
one line.

// Per Context7 (/la10736/rstest, 2026-05-11): confirmed attribute syntax is `#[rstest]` on the
// function + `#[case(val1, val2)]` per row + `#[case] param_name: T` on each function argument.

**Answer:** **Option (a) — adopt rstest.** Add `rstest = { workspace = true }` to `crates/krax-types/Cargo.toml` `[dev-dependencies]`. Use `#[rstest]` + `#[case(...)]` for the `conflicts` truth table and `union` cases.

---

## Decision 3: Add `Debug`, `PartialEq`, `Eq` derives to 1.1b types

**Question:** None of the data types from Step 1.1b (`RWSet`, `JournalEntry`, `Journal`, `Block`,
`PendingTx`, `MempoolEntry`) currently derive `Debug`, `PartialEq`, or `Eq`. Test assertions using
`assert_eq!` require `PartialEq`; failure messages require `Debug`. How should this gap be closed?

**Options:**

(a) Add derives in the Step 1.2 commit, touching the 1.1b source files. One commit: derives added
and tests written together.

(b) Add derives in a separate `refactor(types): derive Debug + PartialEq on data types` commit
that lands immediately before the Step 1.2 test commit (two commits, same PR or adjacent PRs).

(c) Write tests without `assert_eq!` — compare via accessor fields manually and use
`assert!(condition, "message")`. No derives added.

**Sub-question (for options a or b):** Should `Eq` be derived alongside `PartialEq`?

**Trade-offs:**

- Step 1.1b's decisions doc (Decision 14) explicitly excluded tests as out-of-scope. It did not
  exclude derives — they were simply not needed without tests, not intentionally omitted. There is
  no note in AGENTS.md "Current State" saying `Debug`/`PartialEq` was a deliberate non-derive.
  The only deliberate non-derive is `Clone` on `RWSet` (AGENTS.md Notes and 1.1b Decision 7).
- `B256` derives `Eq`, which means `BTreeSet<B256>` is `Eq`. `RWSet::Concrete` can therefore
  derive `Eq` cleanly. `Everything` has no fields, trivially `Eq`. So `Eq` is derivable on `RWSet`.
- `JournalEntry`, `Journal`: fields are `B256` (supports `Eq`). Both can derive `PartialEq` + `Eq`.
- `Block`: `txs: Vec<TxEnvelope>`. `TxEnvelope` from alloy-consensus — likely derives `PartialEq`
  and `Eq` (alloy types consistently do for wire-format types), but the coder must verify via
  Context7 before adding derives to `Block`. If `TxEnvelope` does not derive `PartialEq`, `Block`
  cannot derive it either; the test must compare individual fields.
- `PendingTx` wraps `TxEnvelope` — same caveat as `Block`.
- `MempoolEntry` wraps `PendingTx` + `Address` + `u64`. `Address` derives `PartialEq` + `Eq`.
  `MempoolEntry`'s derivability depends on `PendingTx`'s.
- Option (c) is painful and sets a bad precedent: manual field-by-field comparison is verbose,
  fragile across refactors, and produces poor failure messages.
- Option (b) (separate commit) is the cleanest history — keeps the refactor atomic and the test
  commit focused. It costs one extra commit but makes each commit's intent clear.
- Option (a) is acceptable if the maintainer prefers fewer commits; touching 1.1b files in the
  1.2 commit is not a rule violation.

**Recommendation:** option (b) — separate refactor commit first. Derive `Debug + PartialEq + Eq`
on all types where possible. Coder must verify `TxEnvelope` derivability before deriving on
`Block` and `PendingTx`; if `TxEnvelope` lacks `PartialEq`, omit it from those two types and note
the gap. The Step 1.2 test commit then has clean `assert_eq!` usage throughout.

**Answer:** **Option (b) — separate refactor commit lands immediately before the Step 1.2 test commit.**

Derive `Debug + PartialEq + Eq` on all types where the contained types permit it.

**Explicit derivability matrix and fallback path (encoded in the plan, no improvisation):**

| Type | Target derives | Fallback if `TxEnvelope` lacks `PartialEq` |
|---|---|---|
| `RWSet` | `Debug, PartialEq, Eq` | N/A — no `TxEnvelope` dependency. Always derivable. |
| `JournalEntry` | `Debug, PartialEq, Eq` | N/A — all fields are `B256`. Always derivable. |
| `Journal` | `Debug, PartialEq, Eq` | N/A — inherits via `Vec<JournalEntry>`. Always derivable. |
| `Block` | `Debug, PartialEq, Eq` | **Fallback: derive `Debug` only.** Tests compare `Block` values field-by-field (`assert_eq!(block.parent_hash, expected.parent_hash); ...; assert_eq!(block.txs.len(), expected.txs.len());`). Element-by-element loop over `txs` if individual tx comparison needed. |
| `PendingTx` | `Debug, PartialEq, Eq` | **Fallback: derive `Debug` only.** Field-access comparison in tests (`assert_eq!(pending.0, expected.0)` won't work without `TxEnvelope: PartialEq`; compare via `format!("{:?}", ...)` or transaction-hash equality). |
| `MempoolEntry` | `Debug, PartialEq, Eq` | **Fallback: derive `Debug` only.** Same field-access pattern — `assert_eq!(entry.sender, expected.sender); assert_eq!(entry.arrival_time, expected.arrival_time);` plus the `PendingTx` fallback for `entry.pending`. |

If the fallback triggers for `Block`/`PendingTx`/`MempoolEntry`, the coder records the gap in the refactor commit's body and in the Step 1.2 Outcomes section. A downstream task to push for `PartialEq` upstream in alloy-consensus is filed but is NOT a Step 1.2 dependency.

The planner must include the Context7 verification of `TxEnvelope` derives as the FIRST coder action in the refactor commit — before any `str_replace` runs on `block.rs`, `tx.rs`. If verification surfaces a discrepancy, the coder applies the fallback per the matrix above and reports to the maintainer in the post-execution summary.

---

## Decision 4: Constructor helpers for `RWSet` in tests

**Question:** Test setup like `RWSet::Concrete { r_set: BTreeSet::from([slot1]), w_set: BTreeSet::from([slot2]) }` is verbose. Should helpers be added?

**Options:**

(a) Add `pub fn concrete(r: impl IntoIterator<Item=B256>, w: impl IntoIterator<Item=B256>) -> RWSet`
as a production constructor on `RWSet` itself (available to all callers, not just tests).

(b) Define test-only helpers inside each `#[cfg(test)] mod tests` block:
```rust
#[cfg(test)]
mod tests {
    fn concrete(r: impl IntoIterator<Item=B256>, w: impl IntoIterator<Item=B256>) -> RWSet { ... }
    fn slot(n: u8) -> B256 { B256::from([n; 32]) }
}
```

(c) Accept the verbosity — write `RWSet::Concrete { r_set: ..., w_set: ... }` inline at every
test case. With rstest's `#[case(...)]` the value expressions are short-lived.

**Trade-offs:**

- Option (a) adds production API surface. A `concrete(...)` constructor would be genuinely useful
  to downstream callers (Phase 4 inferers, Phase 6 commit phase tests). However, AGENTS.md Rule 10
  says *"Don't add features, refactor, or introduce abstractions beyond what the task requires."*
  No production caller exists yet. If the constructor is added now, it must be doccommented and
  becomes part of the public API contract.
- Option (b) keeps the production API minimal. The helper is trivially short. It cannot be used by
  a second crate's tests, but no second crate has tests in Step 1.2. If the same helper is needed
  in Phase 4/5/6 tests in other crates, those crates define their own or we add the production
  constructor then.
- Option (c) is workable with rstest since each case is one expression; however, for 8 rows of
  `BTreeSet::from([slot_a, slot_b])` the verbosity compounds and makes cases hard to scan.
- A local `fn slot(n: u8) -> B256 { B256::from([n; 32]) }` helper (not about `RWSet` itself) is
  almost certainly needed regardless; it is small and clearly test-only.

**Recommendation:** option (b). Test-only helpers keep the production API clean while eliminating
the worst verbosity. If a real call site in a production crate needs `concrete(...)` by Phase 4,
add it then with a production justification.

**Answer:** **Option (b) — test-only helpers.**

**Sharing decision (raised in maintainer review, not in the original surfacer):** helpers live in a shared `#[cfg(test)] mod test_helpers` module, NOT duplicated per file.

Location: `crates/krax-types/src/test_helpers.rs`, gated `#[cfg(test)]`, registered in `lib.rs` as `#[cfg(test)] mod test_helpers;`. Helpers:

```rust
#[cfg(test)]
pub(crate) fn slot(n: u8) -> alloy_primitives::B256 { alloy_primitives::B256::from([n; 32]) }

#[cfg(test)]
pub(crate) fn concrete(
    r: impl IntoIterator<Item = alloy_primitives::B256>,
    w: impl IntoIterator<Item = alloy_primitives::B256>,
) -> crate::RWSet {
    crate::RWSet::Concrete {
        r_set: r.into_iter().collect(),
        w_set: w.into_iter().collect(),
    }
}
```

Test modules import via `use crate::test_helpers::{slot, concrete};`. Both `rwset.rs` and `journal.rs` test modules use `slot()`; only `rwset.rs` uses `concrete()`. Single source of truth, no duplication. Sets the pattern for future test files in this crate.

---

## Decision 5: `RWSet::conflicts` test strategy — table-driven vs. property-based

**Question:** ARCHITECTURE.md specifies 8 enumerable conflict cases. Should Step 1.2 add proptest
properties alongside the table, or rely on table-driven tests alone?

**Options:**

(a) Table-driven only — 8 cases enumerated with rstest `#[case]` (if Decision 2 adopts rstest),
covering all distinct conflict paths. Symmetry checked inline: each conflict case asserts both
`a.conflicts(&b)` and `b.conflicts(&a)`.

(b) Table-driven for the 8 cases plus 2–3 proptest properties: symmetry
(`a.conflicts(b) == b.conflicts(a)`) and Everything-absorbs (`RWSet::Everything.conflicts(any)
== true`). Requires adding `proptest = { workspace = true }` to `[dev-dependencies]`.

(c) Table-driven plus exhaustive symmetry assertions inside the table (no proptest dep) — for
every case, the test body asserts both directions. Same coverage as (b)'s symmetry property,
zero new dep.

**Trade-offs:**

- `proptest = "1"` is already in `[workspace.dependencies]` (ESTIMATED version — coder must
  confirm via `cargo search proptest` per LVP). The per-crate dep edit is one line.
- Per Context7 (/proptest-rs/proptest, 2026-05-11): `proptest! { #[test] fn name(x in strategy) { ... } }`
  is the confirmed macro syntax.
- The symmetry property is the most valuable proptest candidate: `a.conflicts(&b) == b.conflicts(&a)`
  for arbitrary `RWSet` pairs. However, generating arbitrary `RWSet` values via proptest requires
  either implementing `Arbitrary` for `RWSet` (non-trivial, adds scope) or a `prop_oneof!` that
  only samples a handful of shapes (weaker than exhaustive). The 8 table cases already cover all
  distinct code paths.
- The "Everything absorbs" property is already covered by cases 7 and 8 in the truth table.
- The symmetry assertion can be checked inline in each table row at zero cost (option c), making
  proptest's symmetry property redundant for the `conflicts` function at this stage.
- proptest becomes more valuable in Phase 4–6 where the conflict detector runs against unbounded
  combinations of real `RWSet` values inferred from transactions.

**Recommendation:** option (c). Symmetry is verified inline in each table case (both `a.conflicts(&b)`
and `b.conflicts(&a)` asserted). No proptest dep added to krax-types in Step 1.2. Proptest's first
real value in this project is Phase 4–6 inferer tests; introducing it in krax-types for a property
that table tests already cover fully is premature. Leave proptest in `[workspace.dependencies]` for
those future crates.

**Answer:** **Option (c) — table-driven with inline symmetry assertions.** No proptest dep added in Step 1.2. Each conflict case asserts both `a.conflicts(&b)` and `b.conflicts(&a)`. Proptest stays in `[workspace.dependencies]` for Phase 4–6 inferer tests.

---

## Decision 6: `Journal::apply` test strategy — stub `State` impl

**Question:** `Journal::apply` calls `state.set(...)` on a `&mut dyn State`. `MptState` (the
first concrete `State` impl) lands in Step 1.3. How should Step 1.2 test `Journal::apply`?

**Options:**

(a) Build a minimal in-test stub `State` implementation inside `#[cfg(test)] mod tests` in
`journal.rs`. The stub holds a `BTreeMap<B256, B256>` and records `set()` calls:
```rust
struct StubState(BTreeMap<B256, B256>);
impl State for StubState {
    fn get(&self, slot: B256) -> Result<B256, StateError> { Ok(*self.0.get(&slot).unwrap_or(&B256::ZERO)) }
    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError> { self.0.insert(slot, val); Ok(()) }
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError> { unimplemented!() }
    fn commit(&mut self) -> Result<B256, StateError> { unimplemented!() }
    fn root(&self) -> B256 { B256::ZERO }
}
```
Tests use `StubState` for `apply` round-trip verification without any external dep.

(b) Defer `Journal::apply` tests entirely to Step 1.3 when `MptState` exists. Step 1.2 tests only
`RWSet::conflicts`, `RWSet::union`, and `Journal::discard`. The ARCHITECTURE.md spec for Step 1.2
lists `Journal::apply` explicitly — the plan would note this deferral and update ARCHITECTURE.md.

(c) Introduce a `MockState` struct in `krax-types` itself behind a `test-utils` feature flag,
making it reusable by downstream crate tests (krax-sequencer, krax-state):
```toml
[features]
test-utils = []
```
```rust
#[cfg(feature = "test-utils")]
pub mod test_utils { pub struct MockState { ... } impl State for MockState { ... } }
```

**Trade-offs:**

- Step 1.3 ("MPT State Backend Skeleton") is the **immediately next step** after 1.2. If (b) is
  chosen, `Journal::apply` testing is delayed by exactly one step. ARCHITECTURE.md's Phase 1 Gate
  says "All types in `krax-types` have tests" — deferring `apply` past 1.2 would require updating
  the gate description or clarifying that "Step 1.3 also covers Journal::apply."
- Option (a) is self-contained and ships full coverage in Step 1.2 as specified. The stub is ~15
  lines, requires `#[allow(clippy::unwrap_used)]` (tests are exempt), and uses `unimplemented!()`
  for `snapshot`, `commit` — which are not called by `apply`. The stub must be `Send + Sync`
  (see Decision 10); `BTreeMap<B256, B256>` is `Send + Sync`.
- Option (c) adds a feature flag, a public `test_utils` module, and an exported type — that is
  scope beyond Step 1.2. `MockState` in krax-types also forces this crate to know about testing
  patterns from consuming crates. Premature abstraction.
- The `StubState` in option (a) uses `unimplemented!()` on `snapshot` and `commit`. Under the
  workspace lint policy, `unimplemented` is `deny`. The stub functions must use `todo!()` which is
  also `deny`, OR return a plausible non-panicking stub value. For `snapshot`: return
  `Err(StateError::Released)` as a placeholder (wrong semantically but won't panic). For `commit`:
  return `Ok(B256::ZERO)`. Or add `#[allow(clippy::unimplemented)]` at the stub impl level with a
  comment. Coder must handle this.

**Recommendation:** option (a). Test `Journal::apply` in Step 1.2 as ARCHITECTURE.md specifies.
The stub is minimal. Coder must satisfy `Send + Sync`, avoid `unimplemented!()` in the stub body
(or add a scoped `#[allow]`), and add `#[allow(clippy::unwrap_used)]` at the test module level.

**Answer:** **Option (a) for Step 1.2 — build a minimal `StubState` in `#[cfg(test)] mod tests` in `journal.rs`.** With an explicit post-1.3 fate (raised in maintainer review): the stub-based `apply` test is **scaffolding**, not a permanent fixture. It exists to verify the `Journal`↔`State` protocol (entry iteration order, error propagation via `?`) in Step 1.2 when no real `State` impl exists.

**Post-1.3 directive (must be encoded in the 1.2 plan's Outcomes section so the 1.3 planner inherits it):**

When Step 1.3 lands with `MptState`:
1. The `StubState` impl in `journal.rs` test module is **deleted**.
2. The `Journal::apply` tests in `journal.rs` test module are **rewritten against `MptState`** (located in the 1.3 plan's test scope, not `krax-types`).
3. `journal.rs`'s `#[cfg(test)] mod tests` may become empty after that, in which case it is removed entirely.

This is option (3) from the maintainer's review: the 1.2 work isn't wasted (writing the stub clarifies what the apply protocol is), but the stub-based test is replaced once a real backend exists. Records the protocol-vs-behavior distinction the maintainer raised: the 1.2 test verifies the *protocol* (iteration order, `?`-propagation), the 1.3 rewrite verifies the *behavior* (state actually changes).

Stub implementation constraints (must satisfy Decision 10's bounds):
- `Send + Sync` automatic since `BTreeMap<B256, B256>` is `Send + Sync`.
- `snapshot()` returns `Err(StateError::Released)` — non-panicking placeholder, avoids `unimplemented!()` deny lint.
- `commit()` returns `Ok(B256::ZERO)` — same rationale.
- `root()` returns `B256::ZERO`.
- `#[allow(clippy::unwrap_used)]` at the test module level for any `.unwrap()` in test bodies.

The stub lives only in `journal.rs`'s `#[cfg(test)] mod tests` (per-file, not in `test_helpers`). Per the directive above it disappears in 1.3, so a shared module would be premature reuse.

---

## Decision 7: `pretty_assertions` adoption

**Question:** Should `pretty_assertions` be added to `crates/krax-types/Cargo.toml`
`[dev-dependencies]` for richer `assert_eq!` diff output?

**Options:**

(a) Add `pretty_assertions = { workspace = true }` to `[dev-dependencies]`. Add
`use pretty_assertions::assert_eq;` at the top of each test module.

(b) Skip for this crate. Use stdlib `assert_eq!` in Step 1.2. Add `pretty_assertions` to crates
with larger, more complex types when failure diffs become hard to read.

**Trade-offs:**

- `pretty_assertions = "1"` is already in `[workspace.dependencies]` (ESTIMATED — coder must
  confirm via `cargo search pretty_assertions` per LVP). Per-crate `[dev-dependencies]` edit is
  one line.
- `krax-types` types are small (enums with 2 variants, structs with 3 B256 fields). The stdlib
  `{:?}` output for a failed `assert_eq!` on `RWSet` or `JournalEntry` will be compact and
  readable once `Debug` is derived (Decision 3). `pretty_assertions`' colored diff output adds
  more value on deeply nested structs.
- There is no correctness risk either way — this is pure developer experience.
- AGENTS.md Rule 10 approves `pretty_assertions` as a test-only dep.
- Adding it now establishes the pattern for all future crates' test modules and prevents the
  question from being re-litigated for krax-sequencer, krax-state, etc.

**Recommendation:** option (a). The dep is already approved and in the workspace. Establishing
the pattern in Step 1.2 means every subsequent test session follows suit by default. The cost
(one `[dev-dependencies]` line per crate + one `use` per test module) is negligible.

// pretty_assertions Rust crate: `use pretty_assertions::assert_eq;` shadows stdlib assert_eq!
// with colored line-diff output on failure. No other API change needed.
// Version "1" is in workspace.dependencies (ESTIMATED — coder: confirm via cargo search).

**Answer:** **Option (a) — adopt pretty_assertions.** Add `pretty_assertions = { workspace = true }` to `crates/krax-types/Cargo.toml` `[dev-dependencies]`. Each test module begins with `use pretty_assertions::assert_eq;` to shadow stdlib `assert_eq!`. Sets the pattern for all future test modules across the workspace.

---

## Decision 8: Coverage measurement and target for Step 1.2

**Question:** Should Step 1.2 specify a numeric coverage target for `krax-types` and a tool
to measure it?

**Options:**

(a) Set an explicit target (e.g. 90%) and specify the tool (`cargo-llvm-cov` or `tarpaulin`).
Step 1.2 is "done" only when the gate is met.

(b) Rely on the Phase 1 Gate target only: ARCHITECTURE.md Phase 1 Gate reads
*"Coverage on `krax-types` and `krax-state` is >85%."* No per-step gate; coverage is measured at
phase close.

(c) Defer coverage measurement entirely. Don't specify a tool until a phase introduces CI.

**Trade-offs:**

- AGENTS.md "Coverage target" note: *"80%+ for `krax-sequencer`, `krax-rwset`, `krax-state`.
  Lower for boilerplate-heavy code."* `krax-types` is not named explicitly in AGENTS.md but the
  Phase 1 Gate explicitly targets 85%.
- `krax-types` is a thin types crate with no I/O paths; 90%+ is achievable. But establishing a
  per-step coverage gate in the plan creates a verification task for the coder.
- `cargo-llvm-cov` requires LLVM toolchain alignment with the pinned Rust toolchain (1.95.0);
  `tarpaulin` is macOS-compatible but slower. Neither is in `[workspace.dependencies]` yet.
- Option (b) defers tool selection while still having a gate at the phase boundary. The risk is
  that coverage gaps discovered at Phase 1 Gate require retrofitting tests across multiple steps.
- Option (c) is the most deferred; acceptable only if the team accepts manual review as the
  coverage proxy for now.

**Recommendation:** option (b). Rely on the Phase 1 Gate target of >85%. Do NOT install a
coverage tool in Step 1.2 — no CI pipeline exists yet and tool selection (llvm-cov vs tarpaulin)
should happen in a dedicated tooling step. The planner for Step 1.2 should note in the plan that
the coder must achieve test scope broad enough to hit 85% when the gate is checked at Phase 1 close.

**Answer:** **Option (b) — rely on Phase 1 Gate, no per-step gate, no tool install in 1.2.**

**Urgency note (raised in maintainer review):** the tool decision is now more pressing because Decision 9's reshape (skip trivial-data tests) relies on the coverage tool supporting line/file exclusions. `cargo-llvm-cov` supports exclusions via `// llvm-cov:ignore` annotations and `Cargo.toml` exclude config; `tarpaulin` supports `#[cfg(not(tarpaulin_include))]` attributes.

**Directive:** a dedicated infra step — provisionally named **Step 1.3.5: Coverage Tooling** — lands between Step 1.3 (MPT State Skeleton) and Step 1.4 (Phase 1 Gate). Its purpose: pick `cargo-llvm-cov` vs `tarpaulin`, install the toolchain, add a `make coverage` Makefile target, and apply exclusion annotations to the trivial-data types per Decision 9.

No action required from Step 1.2 itself — just the directive recorded so the next planner round (after 1.2 lands) knows to slot 1.3.5 in.

---

## Decision 9: Test scope — which public items get tests in Step 1.2

**Question:** ARCHITECTURE.md Step 1.2 names four items: `RWSet::conflicts`, `RWSet::union`,
`Journal::apply`, `Journal::discard`. Beyond those, which public items from 1.1a and 1.1b are
worth testing in Step 1.2?

**Candidate test items and proposed disposition:**

| Item | Proposed in 1.2? | Rationale |
|---|---|---|
| `RWSet::conflicts` (8 cases) | ✅ Yes | Core behavior; truth table is load-bearing for the entire conflict detector |
| `RWSet::union` (5–6 cases) | ✅ Yes | Core behavior; Everything-absorption must be verified |
| `Journal::apply` (round-trip) | ✅ Yes (if Decision 6 = a) | Core behavior; apply-then-read contract |
| `Journal::discard` | ✅ Yes | Consuming semantics verify discard doesn't apply; mostly a compile+run test |
| `Block::new` (constructor) | ⚠️ Maybe | Trivial field assignment; only worth testing if field ordering or invariants could silently break — there are none |
| `PendingTx` field access | ❌ Skip | Trivial struct wrapper; no logic to test |
| `MempoolEntry` field access | ❌ Skip | Trivial struct; no logic to test |
| `JournalEntry` field access | ❌ Skip | Trivial struct; no logic to test |
| `StateError` variants | ❌ Skip | `thiserror`-derived; testing the error string is brittle |
| `State` / `Snapshot` object-safety assertions | ❌ Skip | Already verified at compile time by `const _: Option<&dyn State> = None;` |

**Question for maintainer:** Should `Block::new` get a minimal test (e.g. round-trip field check),
or is it genuinely trivial enough to skip? The downside of skipping is that a future refactor of
`Block` (reordering fields, adding a validated constructor) will have no test catching regression.

**Trade-offs:**

- Testing trivial accessors and constructors is busywork in Phase 1; it becomes more valuable as
  invariants accumulate in Phase 11+ when block construction becomes load-bearing.
- AGENTS.md Rule 5 says *"Every public item in a crate has a test before it lands."* Strictly
  read, this requires tests for `Block::new`, `PendingTx`, `MempoolEntry`, and `JournalEntry`.
  Loosely read, a constructor with no logic is tested implicitly by the `Journal::apply` test
  (which constructs `JournalEntry` values).

**Recommendation:** Follow the strict reading of Rule 5: add at minimum a smoke test for
`Block::new` (constructs a block, verifies field values round-trip). `JournalEntry` is tested
implicitly through `Journal::apply`. `PendingTx` and `MempoolEntry` can be tested with a single
construction + field read each. These are one or two `assert_eq!` lines per item — the overhead
is minimal and satisfies Rule 5.

**Answer:** **Reshape the original recommendation per maintainer review — reject the strict-Rule-5 reading. Skip smoke tests on types with no logic.**

Maintainer's reasoning (recorded here so the planner doesn't backslide): a one-line construction test on `PendingTx` or `MempoolEntry` verifies that the Rust compiler stores struct fields, not that Krax is correct. The compiler already guarantees that. Writing tests to satisfy a coverage gate, rather than to validate behavior, gives the coverage metric a false signal.

**Final disposition for Step 1.2 test scope:**

| Item | In 1.2? | Rationale |
|---|---|---|
| `RWSet::conflicts` (8 cases) | ✅ Yes | Core logic. Truth table load-bearing for the entire conflict detector. |
| `RWSet::union` (5–6 cases) | ✅ Yes | Core logic. Everything-absorption must be verified. |
| `Journal::apply` (round-trip) | ✅ Yes | Per Decision 6 (option a). Scaffolding — replaced in 1.3. |
| `Journal::discard` | ✅ Yes (compile_fail doctest) | Per Decision 11. The contract is compile-time, not runtime. |
| `Block::new` | ❌ Skip | Struct literal in a function wrapper. No logic to verify. |
| `PendingTx` field access | ❌ Skip | Newtype wrapper. No logic. |
| `MempoolEntry` field access | ❌ Skip | Plain struct. No logic. |
| `JournalEntry` field access | ❌ Skip | Plain struct. Tested implicitly through `Journal::apply`. |
| `StateError` variants | ❌ Skip | `thiserror`-derived. Brittle to test. |
| `State` / `Snapshot` object-safety | ❌ Skip | Compile-time assertion already present. |

**AGENTS.md Rule 5 amendment (lands in Step 1.2 commit, before the test code):**

The AGENTS.md Code Architecture Rule 5 sentence *"Every public item in a crate has a test before it lands"* is replaced with:

> **Every public item with logic has a direct test before it lands.** Data types with no methods (newtype wrappers, plain structs, public-field-only types) are tested implicitly through their users; do not write construction-only smoke tests purely to satisfy coverage targets. When in doubt, ask: "would a regression here be caught by the compiler, or could it silently produce wrong behavior?" — only the latter needs a test.

The planner must include this AGENTS.md edit as one of the str_replace edits in the Step 1.2 plan, ahead of the test files. The amendment is a deliberate policy change with maintainer reasoning attached — not a quiet workaround.

**Coverage gate implication (links to Decision 8):** the trivial-data types (`PendingTx`, `MempoolEntry`, `JournalEntry`, `Block`) will reduce the raw line-coverage number on `krax-types`. The Phase 1 Gate target (>85%) is reconciled via Step 1.3.5's coverage tooling step (Decision 8), which applies exclusion annotations to data-only types so they are not counted. If 1.3.5 slips past the Phase 1 Gate check, the maintainer accepts the temporary coverage-number gap and documents it; raw coverage is not a quality signal when the un-excluded types have no logic to cover.

---

## Decision 10: `State` trait bounds — stub impl requirements

**Question / informational:** If Decision 6 chooses option (a) — a `StubState` in `#[cfg(test)]` — what trait bounds must the stub satisfy?

**Current `State` trait bounds (from `state.rs`):**

```rust
pub trait State: Send + Sync {
    fn get(&self, slot: B256) -> Result<B256, StateError>;
    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError>;
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError>;
    fn commit(&mut self) -> Result<B256, StateError>;
    fn root(&self) -> B256;
}
```

`State: Send + Sync`. Any concrete impl — including a test stub — must be `Send + Sync` for the
impl to compile (auto-trait coherence check at the `impl State for StubState` site).

**What this means for the stub:**

- `StubState`'s fields must be `Send + Sync`. `BTreeMap<B256, B256>` is `Send + Sync` ✅.
- The stub must implement all five methods. `snapshot` and `commit` are not called by `apply` or
  `discard` but must have bodies. See Decision 6 trade-offs for the `unimplemented!()` /
  `todo!()` lint issue — the stub must avoid those macros under the workspace deny policy or use
  a scoped `#[allow]`.
- `root()` can return `B256::ZERO` (a const, no I/O).
- `snapshot()` can return `Err(StateError::Released)` as a never-called placeholder — this avoids
  the `unimplemented` / `todo` lint violation.
- `commit()` can return `Ok(B256::ZERO)` as a never-called placeholder.

No recommendation needed — this is a factual constraint the planner must encode in the plan's
"coder follow-ups" checklist.

**Answer:** **Informational — no maintainer decision required, but a sharing question raised in review:**

With Decision 6 = option (a) + delete-in-1.3, the `StubState` is short-lived scaffolding. Per Decision 6's answer, the stub lives only in `journal.rs`'s `#[cfg(test)] mod tests` block. **Duplicate-is-fine** would apply if `rwset.rs`'s test module ever needed a stub `State`, but it doesn't (RWSet tests don't touch `State`). So the question is moot: one copy in `journal.rs`, disappears in 1.3.

The `Send + Sync` bound is satisfied automatically by `BTreeMap<B256, B256>`. The five-method requirement is satisfied with non-panicking placeholders per Decision 6. The planner encodes this in the journal.rs test module's verbatim content; no additional coder follow-up needed beyond Decision 6's constraints.

---

## Decision 11: `Journal::discard` test strategy — compile_fail doctest

**Added in maintainer review.** Not surfaced in the original decisions doc; load-bearing enough to elevate.

**Question:** `Journal::discard(self)` consumes the journal. Its real guarantee is compile-time: after `discard()`, using `journal` is a move-after-use compile error. A runtime test verifying "discard doesn't panic" doesn't verify the actual contract. How should this be tested?

**Options:**

(a) Skip entirely — trust the compiler. The contract is move semantics; a regression means someone changed `discard(self)` to `discard(&self)`, which is a deliberate breaking change.

(b) Add a `trybuild` dev-dependency and a `tests/compile_fail/` directory with `.rs` files that assert specific code fails to compile.

(c) Add a `compile_fail` doctest in the `///` comment block on `Journal::discard`:
```rust
/// ```compile_fail
/// # use krax_types::Journal;
/// let journal = Journal { entries: vec![] };
/// journal.discard();
/// let _ = journal.entries; // use after move — fails to compile
/// ```
```

**Trade-offs:**

- Option (a) is acceptable in principle (Rust's type system guarantees the contract) but the test that's a comment in the spec has a way of erasing itself — a future refactor might silently weaken `discard` and no test would catch it.
- Option (b) is the most powerful (multiple compile-fail cases, structured error matching), but adds a new dependency (`trybuild` is not in `[workspace.dependencies]` today) and a `tests/compile_fail/` directory. Heavy machinery for one guarantee.
- Option (c) is Rust-native (no new dep), runs as part of `cargo test`, the doctest is colocated with the doc comment that describes the contract. The `compile_fail` annotation means `rustdoc` verifies the code does NOT compile — a real test.
- `compile_fail` doctests are tested by `cargo test` automatically. No `make test` change needed.
- Doc syntax requires `# use` to suppress import lines from rendered docs; rendered output shows only the user-facing code.

**Answer:** **Option (c) — add a `compile_fail` doctest in the `///` block on `Journal::discard`.**

The doctest is colocated with the contract documentation, runs as part of `cargo test`, requires no new dep. If a future refactor weakens `discard`'s consuming semantics, the doctest passes (because the code no longer fails to compile), and `make test` fails — catching the regression at exactly the point that matters.

The planner must include the doctest as an addition to `Journal::discard`'s existing doc comment. Exact content (verbatim in the plan):

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
pub fn discard(self) {
    // body unchanged
}
```

The coder verifies that `cargo test --doc -p krax-types` exits 0 with the `compile_fail` annotation — i.e., the doctest is recognized AND the code inside fails to compile as expected.

**Note on AGENTS.md Rule 5 amendment (Decision 9):** the amendment's "would a regression here be caught by the compiler, or could it silently produce wrong behavior?" framing supports this test choice. A regression on `discard`'s consuming semantics WOULD be caught by the compiler at the use site — but only at the use site. The doctest is the test that lives with the contract itself, so a regression is caught immediately, not only when a downstream caller happens to misuse the API.

---

## Cross-step impact

### 1. Decision 3 (derives) → Step 1.1b files touched in a separate refactor commit

**Resolved:** option (b) chosen. A separate `refactor(types): derive Debug + PartialEq + Eq on data types` commit lands immediately before the Step 1.2 test commit. The Step 1.2 plan must include explicit `str_replace` edits for each of the six 1.1b source files (`rwset.rs`, `journal.rs`, `block.rs`, `tx.rs`). Context7 verification of `TxEnvelope` derives is the FIRST coder action in the refactor commit; the derivability matrix in Decision 3's answer governs the fallback.

### 2. Decision 6 (Journal::apply scaffolding) → Step 1.3 inherits a deletion directive

**Resolved:** option (a) for 1.2 + option (3) for the post-1.3 fate. `journal.rs`'s `StubState` is deleted in Step 1.3; the `Journal::apply` test is rewritten against `MptState`. The Step 1.2 plan's Outcomes section must record this directive verbatim so the Step 1.3 planner inherits it. ARCHITECTURE.md's Phase 1 Gate text ("All types in `krax-types` have tests") remains satisfied at 1.2 close; the 1.3 rewrite is a quality improvement, not a coverage repair.

### 3. Decision 8 + Decision 9 → Step 1.3.5 (Coverage Tooling) is now a slotted future step

**Resolved:** Decision 8 = option (b), but the maintainer's reshape of Decision 9 (skip trivial-data tests) means the Phase 1 Gate's >85% coverage target requires exclusion-aware tooling. A new step — **Step 1.3.5: Coverage Tooling** — is provisionally slotted between Step 1.3 and Step 1.4. Its scope: pick `cargo-llvm-cov` vs `tarpaulin`, install the toolchain, add `make coverage`, apply exclusion annotations to `PendingTx`, `MempoolEntry`, `JournalEntry`, `Block`. The Step 1.2 planner records this slot in ARCHITECTURE.md as a placeholder; the Step 1.3.5 plan itself is a future decision-surface round.

### 4. Decision 9 → AGENTS.md Rule 5 amendment is part of the Step 1.2 commit

**Resolved:** the reshape of Decision 9 carries an AGENTS.md edit (text quoted in Decision 9's answer). The Step 1.2 plan includes this edit as a discrete `str_replace` on AGENTS.md, executed BEFORE the test code lands. The amendment is a deliberate policy change, not a workaround; the AGENTS.md changelog entry for Session 13 (Step 1.2) records the reasoning.

### 5. Decision 11 → `Journal::discard`'s doc comment is rewritten in the test commit

**Resolved:** the `compile_fail` doctest lands as part of the Step 1.2 test commit, not the refactor commit (Decision 3's commit), since the doctest IS a test. The planner must include a `str_replace` on `journal.rs` that replaces `Journal::discard`'s current doc comment with the expanded version containing the doctest. Verification: `cargo test --doc -p krax-types` exits 0 after the edit (the doctest must be recognized and the inner code must fail to compile).

---

## Coder follow-ups

Items that produce a "verify when writing code" obligation — not maintainer decisions, but
required pre-writing checks for the coder.

1. **rstest version**: Confirm `rstest = "0.26"` resolves to 0.26.1 (or later 0.26.x) via `cargo
   tree -p krax-types` after adding the dep. Per Context7 (/la10736/rstest, 2026-05-11): attribute
   syntax is `#[rstest]` + `#[case(val, val)]` + `#[case] param: T` on each argument. Verify no
   syntax change in 0.26.x.

2. **pretty_assertions version**: Run `cargo search pretty_assertions` to confirm version "1" is current. (proptest version-check dropped — Decision 5 chose option (c), proptest dep NOT added in 1.2.)

3. **`TxEnvelope` derives — FIRST action in the refactor commit**: Before adding `#[derive(Debug, PartialEq, Eq)]` to `Block` or `PendingTx`, verify that `alloy_consensus::TxEnvelope` derives `PartialEq` and `Eq`. Use Context7 (`/alloy-rs/alloy`, query: "`TxEnvelope` derives PartialEq Eq Debug"). If missing, apply the Decision 3 derivability matrix fallback (derive `Debug` only on `Block`/`PendingTx`/`MempoolEntry`; tests use field-by-field comparison) and report the gap in the refactor commit body.

4. **Stub `State` lint compliance**: The `StubState` impl in `journal.rs` test module must not use `unimplemented!()` or `todo!()` (both `deny` at workspace level). Use explicit return values per Decision 6's constraints: `snapshot()` returns `Err(StateError::Released)`, `commit()` returns `Ok(B256::ZERO)`, `root()` returns `B256::ZERO`. Add `#[allow(clippy::unwrap_used)]` at the test module level for any `.unwrap()` calls.

5. **`missing_docs` in test modules**: Items inside `#[cfg(test)] mod tests { ... }` are private and do NOT trigger `missing_docs = "warn"`. No doc comments required on test functions or helpers. Same applies to `test_helpers.rs` (gated `#[cfg(test)]`, helpers are `pub(crate)`).

6. **Symmetry assertions in `conflicts` table**: For each conflict case where `a.conflicts(&b)` is `true`, the test body must also assert `b.conflicts(&a)` — inline symmetry verification per Decision 5's answer (option c).

7. **`compile_fail` doctest verification**: After updating `Journal::discard`'s doc comment per Decision 11, run `cargo test --doc -p krax-types` and confirm exit 0. The doctest must be recognized AND the inner code must fail to compile (the `compile_fail` annotation inverts the success condition).

8. **AGENTS.md Rule 5 amendment**: The Step 1.2 plan includes a `str_replace` on AGENTS.md replacing the Rule 5 sentence per Decision 9's answer. This edit lands BEFORE the test code, in the same commit as the test code. The AGENTS.md Changelog Session 13 entry records the rationale.

9. **AGENTS.md Current State + Changelog**: Standard end-of-step update per the established workflow (Sessions 8–12 set the pattern). Session 13 entry appended at the BOTTOM of the Changelog per the explicit append-at-bottom directive.

10. **ARCHITECTURE.md Step 1.2 closure + Step 1.3.5 slot**: Mark Step 1.2 checkboxes `[x]` and heading ✅. Insert a new Step 1.3.5 (Coverage Tooling) placeholder between Step 1.3 and Step 1.4 per Decision 8's directive. The placeholder is a heading and a one-sentence scope description; full plan is a future decision-surface round.

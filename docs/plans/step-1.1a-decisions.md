# Step 1.1a — Decision Surface

> Pre-plan decision-surface document produced by the planning agent and amended by the strategic-guide agent.
> This is the input to `step-1.1a-trait-interfaces.md`. All decisions below are final.
> Archived alongside the plan file when Step 1.1a ships.

---

## Library Verification Results (Context7, 2026-05-09)

### `alloy-primitives` — v1 (workspace-pinned)

Source: `/alloy-rs/core` via Context7.

- `B256` is `FixedBytes<32>` — a type alias for the 32-byte generic fixed-size byte array. Used as the slot key and value throughout Phase 1+.
- `B256::ZERO`, `b256!` macro, and hex `.parse()` confirmed available.
- Standard derives on `FixedBytes<N>`: `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` — confirmed from crate source and usage patterns (Context7 returned `B256::ZERO`, `B256` as a map key, hash/parse usage; individual derive list not explicitly returned by query, but confirmed by crate convention).
- **Serde note:** the workspace definition includes `features = ["serde"]` on `alloy-primitives`. A crate that does `alloy-primitives = { workspace = true }` inherits this. This adds `Serialize`/`Deserialize` to alloy's own types (`B256`, etc.) — it does NOT automatically apply `#[derive(Serialize, Deserialize)]` to our own structs. The no-serde-derives rule for Krax types is unaffected. The coder must not attempt to "fix" or strip the inherited serde feature.

### `thiserror` — v2.0.18 (workspace-pinned at "2")

Source: `/websites/rs_thiserror_2_0_18` via Context7.

- Confirmed at v2.0.18. Syntax is identical to v1: `#[derive(Error, Debug)]` on the enum, `#[error("...")]` on each variant.
- Available attributes: `#[error]`, `#[from]`, `#[source]`, `#[backtrace]`.
- Step 1.1a does not use `#[from]` — there are no foreign errors at this layer. `#[from]` arrives when Step 1.3 adds MDBX I/O errors.
- Confirmed example:
  ```rust
  #[derive(Error, Debug)]
  pub enum DataStoreError {
      #[error("data store disconnected")]
      Disconnect(#[from] io::Error),
      #[error("unknown data store error")]
      Unknown,
  }
  ```

### Workspace root — `alloy-primitives` and `thiserror` presence

Both are already in `[workspace.dependencies]`:

- `alloy-primitives = { version = "1", default-features = false, features = ["serde"] }` ✅
- `thiserror = "2"` ✅

No workspace root edits needed for Step 1.1a.

---

## Decisions (final, after amendments)

### Decision 1 — `Snapshot::release` signature

**Resolved: (a) `release(self: Box<Self>)` — consuming, compile-time guarantee.**

ARCHITECTURE.md contains an inconsistency: Step 1.1 (pre-split text inherited into 1.1a) implied a consuming `release(self: Box<Self>)`. Step 1.4's test text reads `s.release(); s.get(...) → StateError::Released` — which assumes `s` is still alive post-release, making a consuming signature a compile error on the test itself.

Path chosen: keep the consuming signature. The `Box` is consumed; further use of `s` after `.release()` is a compile error ("borrow of moved value"). Step 1.4's test becomes a `compile_fail` doctest or a `trybuild` test file. ARCHITECTURE.md Step 1.4 gets a text edit in this commit (cross-step reconciliation note, not any Step 1.4 code). The ARCHITECTURE.md change is text-only — no `trybuild` infrastructure is added in Step 1.1a; that is Step 1.4's problem to set up.

**Rationale:** Rust's strongest guarantee — post-release use is caught by the compiler, not at runtime. Release of a snapshot is an ownership event (the snapshot is logically destroyed), which is exactly what consuming semantics model. Aligns with RAII and how `Drop` works. The `trybuild` investment in Step 1.4 is small and pays dividends (compile-fail tests are the right tool for "this must not compile" invariants).

### Decision 2 — `StateError` starter variants

**Resolved: (a) `StateError::Released` only, with `#[non_exhaustive]`.**

Minimal. `#[non_exhaustive]` makes the enum extensible without breaking downstream matches. New variants land only when downstream code actually needs them (Step 1.3 brings I/O variants).

### Decision 3 — Where `StateError` lives

**Resolved: (a) Inside `state.rs`, just above the `State` trait.**

The error type is primarily (and for now exclusively) the error type of `State` operations. It's small. It lives where consumers will look. Refactor to a sibling `error.rs` if it grows unwieldy in a later phase.

### Decision 4 — Object-safety assertion form (AMENDED)

**Resolved: ungated module-scope constant in each trait file, simple `Option<&dyn _>` form.**

```rust
const _: Option<&dyn State> = None;
```

at module scope in `state.rs`, and the analogous `const _: Option<&dyn Snapshot> = None;` in `snapshot.rs`. Same compile-time guarantee as a nested-fn version, much less noise.

Add a `//` comment above each (not `///`, since `const _` is anonymous and doesn't get rustdoc'd):

```
// Compile-time assertion that <Trait> is object-safe. If a non-object-safe
// method is added to the trait, this fails to compile.
```

The assertion fires on `cargo build`, not only `cargo test`. Future drift is caught on every build.

### Decision 5 — Stub tests in Step 1.1a

**Resolved: (b) No test code in Step 1.1a.**

The object-safety assertions in Decision 4 already provide compile-time validation. Trait definitions are not meaningfully testable without an implementation. Step 1.2's job is tests; Step 1.1a's job is the trait surface. The `#[cfg(test)] mod tests { ... }` scaffold is trivial for Step 1.2 to add from scratch.

### Decision 6 — `Send + Sync` supertraits (AMENDED)

**Resolved: `State: Send + Sync` and `Snapshot: Send + Sync`.**

Original recommendation was `State: Send` (Send only) and `Snapshot: Send + Sync`. Amended to promote `State` to `Send + Sync` for symmetry.

**Rationale:**
- `Snapshot: Send + Sync` is required by Phase 7.2 (`Arc<dyn Snapshot>` shared across workers).
- `State: Send + Sync` (rather than just `Send`) avoids a Rule 8 event in any future scenario where `State` is shared across tasks — even briefly, even for read-only paths (e.g. an RPC handler reading current root while a commit is in progress).
- Concrete `MptState` will be `Send + Sync` anyway (MDBX handles are). Cost is zero; benefit is symmetry and no future breaking change for the same trait.
- Both `Send` and `Sync` are auto-traits and are valid supertraits for object-safe traits.

### Decision 7 — `State` method signatures (AMENDED)

**Resolved: as below, with `snapshot()` widened to return `Result<Box<dyn Snapshot>, StateError>`.**

```rust
pub trait State: Send + Sync {
    fn get(&self, slot: B256) -> Result<B256, StateError>;
    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError>;
    fn snapshot(&self) -> Result<Box<dyn Snapshot>, StateError>;
    fn commit(&mut self) -> Result<B256, StateError>;  // returns post-commit state root
    fn root(&self) -> B256;                             // current root without committing
}
```

**Amendment rationale:** original recommendation had `snapshot(&self) -> Box<dyn Snapshot>` (infallible at trait level), with a note that fallibility could be added in Step 1.3 if MDBX needs it. Amended to widen now.

- MDBX read-transaction creation can fail; Step 1.3's MDBX backend will need this fallibility.
- Widening in 1.1a costs one extra `?` at call sites (zero current callers).
- Same anti-deferral discipline as Decision 6 — avoids a Rule 8 event in Step 1.3.

Other notes:
- `root(&self) -> B256` is a pure read with no I/O failure path at the trait level. Concrete implementations may return a cached root.
- `commit` returns the post-commit root as `B256` — this is the state root that will eventually be posted to L1 in Phase 14.

### Decision 8 — `Snapshot` method signatures

**Resolved: as below.**

```rust
pub trait Snapshot: Send + Sync {
    fn get(&self, slot: B256) -> Result<B256, StateError>;
    fn release(self: Box<Self>);  // per Decision 1 = (a)
}
```

`Snapshot::get` returns `Result<B256, StateError>` — same error type as `State::get`. Under Decision 1a, `Snapshot::get` cannot return `StateError::Released` because the snapshot is consumed at release. `StateError` is still the right return type because future variants (I/O errors from the MDBX backend) will apply to snapshot reads too.

### Decision 9 — `crates/krax-types/Cargo.toml` exact edit

**Resolved.** Replace the current empty `[dependencies]` block with:

```toml
[dependencies]
# B256 (= FixedBytes<32>) is the slot key and value type throughout the State trait.
alloy-primitives = { workspace = true }
# Per-crate typed errors per AGENTS.md Rule 3.
thiserror        = { workspace = true }
```

Both use workspace inheritance. No version pins in the per-crate `Cargo.toml`.

### Decision 10 — `lib.rs` re-export structure

**Resolved.** After Step 1.1a, `crates/krax-types/src/lib.rs` will be:

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

Downstream code writes `use krax_types::State;` not `use krax_types::state::State;`. Standard Rust crate-root re-export pattern.

---

## Summary table

| # | Topic | Resolution |
|---|---|---|
| 1 | `release` signature | (a) `release(self: Box<Self>)` — compile-time, Step 1.4 ARCHITECTURE.md text edit in same commit |
| 2 | `StateError` variants | (a) `Released` only + `#[non_exhaustive]` |
| 3 | `StateError` location | (a) Inside `state.rs` |
| 4 | Object-safety assertions | (a) Ungated `const _: Option<&dyn _> = None;` in each file (AMENDED — simpler form) |
| 5 | Stub tests | (b) None in Step 1.1a |
| 6 | `Send + Sync` bounds | `State: Send + Sync`, `Snapshot: Send + Sync` (AMENDED — symmetry) |
| 7 | `State` method signatures | As listed; `snapshot()` widened to `Result<Box<dyn Snapshot>, StateError>` (AMENDED) |
| 8 | `Snapshot` method signatures | As listed; `release` shape per Decision 1 |
| 9 | Cargo.toml edits | Two workspace-inherited deps with justifying comments |
| 10 | `lib.rs` re-exports | `pub mod` declarations + `pub use` for flat ergonomic imports |

---

## Cross-step impact (must be reflected in the plan)

- **ARCHITECTURE.md Step 1.4 text edit** required in the Step 1.1a commit. The current Step 1.4 test description (`s.release(); s.get` returns runtime error) is incompatible with the consuming `release` signature chosen in Decision 1. The plan must specify the exact `str_replace` for Step 1.4's test text — likely something like: "Test: `s.release(); s.get(...);` — must fail to compile (use `trybuild` or `compile_fail` doctest); the `trybuild` infrastructure is set up in Step 1.4."

- **No workspace-root `Cargo.toml` edits.** Both `alloy-primitives` and `thiserror` are already present.

- **No new files outside `crates/krax-types/`.** Step 1.1a is contained.

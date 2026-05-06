---
name: krax-conventions
description: The codebase rulebook for the Krax L2 project. Apply this skill on EVERY coding session in the Krax project — when planning, writing, or reviewing Rust or Solidity code, when proposing dependencies, when running tests, or when updating project docs. This skill encodes the non-negotiable conventions that AGENTS.md sets out: trait boundaries, error handling rules, determinism rules (BTreeMap not HashMap in commit-path code), structured logging, the no-`unwrap()`-in-production rule, the file/function length caps, doc comment conventions, the Library Verification Protocol (Context7 before every external library use), and the exact domain vocabulary the codebase uses (RSet, WSet, Speculation Window, Worker, Journal, etc.). Trigger this skill aggressively — it should fire even when the user doesn't explicitly mention "conventions" or "rules," because virtually every coding action in this project depends on it.
---

# Krax Conventions

This is the rulebook for working in the Krax codebase. The full source of truth is `AGENTS.md` at the project root; this skill is the high-density summary of the rules that govern day-to-day coding decisions. Read AGENTS.md in full when you need the rationale for a rule; this skill exists to keep you from forgetting the rules in the first place.

## Cross-references

- **For Krax-specific architectural decisions** (V1/V2 phase boundary, speculation model, security model, why MPT in V1 vs LSM in V2, the predictive finality model): see `krax-architecture` skill.
- **For revm/reth/alloy implementation patterns** (current API surface as of April–May 2026, async + rayon patterns, error wrapping idioms): see `krax-rust-engine` skill.
- **For the source of truth on any of these rules**: read `AGENTS.md` and `ARCHITECTURE.md` at the project root.

## The 10 Code Architecture Rules

These are non-negotiable. Reviewer flags violations as 🔴 Must Fix.

### 1. Trait boundaries
- All cross-crate dependencies go through traits defined in `krax-types`.
- Concrete types live in their owning crate; other crates import only via traits.
- New cross-crate dependencies require a trait added to `krax-types` first, in a separate commit.

### 2. No global state
- No `static mut`. No `lazy_static!` / `once_cell` / `OnceLock` for *mutable* global state. Read-only `const`/`static` values are fine.
- Constructors take all dependencies explicitly. Wire-up happens in `bin/kraxd/src/main.rs`.

### 3. Errors are typed, always wrapped
- Library crates define their own error type with `thiserror`. Never return a foreign error type unwrapped.
- Wrap with context: `.map_err(|e| RwSetError::InferTx { tx_hash, source: e })`.
- `anyhow::Error` is acceptable ONLY at binary entry points (`bin/*/src/main.rs`).
- Sentinel-style errors are enum variants, not constants: `RwSetError::ConflictDetected { ... }`.
- `unwrap()` and `expect()` are forbidden in production code paths. Tests, build scripts, and startup-only invariants are exempt.
- Never `panic!` outside `main` startup or genuinely unrecoverable invariants. A panic in the sequencer is a bug.

### 4. Logging is structured
- Use `tracing`. Never `println!`, `eprintln!`, or the `log` crate.
- Use structured fields, not formatted strings:
  - ✅ `tracing::info!(tx_hash = %hash, "received transaction")`
  - ❌ `tracing::info!("received tx {}", hash)`
- Three log levels in this codebase: `debug` (verbose internals), `info` (significant events), `error` (something went wrong). No `warn`. `trace` is allowed for opt-in deep diagnostics.

### 5. Testing is non-negotiable
- Every public item has a test before it lands.
- Table-driven tests are the default style. Use `#[test]` with parameterized helpers, or `rstest` for parameterization.
- Integration tests live in `tests/` and are gated behind an `integration` feature flag where they require external resources (anvil, MDBX).
- Test files mirror module layout: `crates/krax-rwset/src/static_/analyzer.rs` → unit tests in the same file under `#[cfg(test)] mod tests`.
- Coverage targets: 80%+ for `krax-sequencer`, `krax-rwset`, `krax-state`. Lower acceptable for boilerplate-heavy code.

### 6. Concurrency discipline
- Async tasks are launched from a constructor or a long-lived service method. No fire-and-forget `tokio::spawn` deep in call stacks.
- Every long-running task accepts a `CancellationToken` (from `tokio-util`) or equivalent shutdown signal.
- Shared state between tasks uses channels first, locks second. If you reach for `Mutex`/`RwLock`, document why a channel doesn't work.
- Prefer `parking_lot` locks over stdlib for short critical sections; stdlib for poison-aware paths.
- Worker pool size is configurable; default is `std::thread::available_parallelism()`.
- **Workers that perform CPU-bound work (EVM execution) run on a `rayon` thread pool or dedicated OS threads, NOT on the `tokio` runtime.** Do not block the async runtime.

### 7. Determinism
- The sequencer's commit phase MUST be deterministic given the same input mempool ordering. This is enforced by tests.
- **No `HashMap` iteration in commit-path code.** Use `BTreeMap` or sort before iterating. `HashMap` iteration order is non-deterministic.
- No `SystemTime::now()`, no `rand` without an explicit seeded RNG, no floating-point arithmetic in state-affecting code.
- Speculative execution can use any order; commit MUST use mempool order.

### 8. State backend trait stability
- The `State` trait in `krax-types` is the V1↔V2 contract. Changes require explicit phase planning.
- V1 mpt code MUST NOT export anything beyond what the trait requires.
- V2 lsm code, when it lands, MUST be a drop-in replacement — no consumer changes.

### 9. Crate boundaries
- `bin/*` may depend on any `crates/*`.
- `crates/*` may depend on other `crates/*` and approved external dependencies.
- `crates/*` may NOT depend on `bin/*`.
- `contracts/*` is independent; Rust code only consumes ABI artifacts from `contracts/out/`.

### 10. Dependency hygiene
- Adding a new external Rust dependency requires justification in the commit message.
- The approved-deps list lives in AGENTS.md "Code Architecture Rules" rule 10. Only those, plus anything explicitly added with reviewer sign-off.
- Note: `reth-primitives` was removed in Reth 2.0 (April 2026). Use `reth-ethereum-primitives` instead.

## File and function length caps

- **File cap: ~500 lines.** When approaching 500 lines, split before writing more. The planner agent flags expected splits in advance.
- **Function cap: 60–80 lines.** Longer is a smell that says "this function is doing too much" — refactor into helpers.
- These are guidelines, not laws. A 600-line file with a big match statement, or a 100-line function that's genuinely a state machine, can be the right answer. Default is to split; exceptions are deliberate decisions documented in a comment or PR.

## Doc comment conventions

- Every `pub` item (`pub fn`, `pub struct`, `pub trait`, `pub enum`, `pub mod`) has a `///` doc comment.
- The doc comment explains **the why, not the what**. Anyone can read the signature; the comment exists to capture intent. Example:
  - ❌ `/// Returns the read set.`
  - ✅ `/// Returns the read set captured during execution. This is the *actual* read set, distinct from the inferred read set used for dispatch — used by the conflict detector to validate speculation.`
- Inline `//` comments where the code is non-obvious. Especially around determinism reasons or safety reasons (over-approximating instead of under-approximating). The audience is "future me in 6 months."
- Don't comment trivial code (`let x = 5;`). Over-commenting drowns out signal.

## Library Verification Protocol

Krax's stack moves fast — revm, reth, and alloy all had breaking changes in 2026. Training data is stale. Every external library use MUST be verified via Context7 before code is written.

### Priority tiers

**High priority (verify before every use):** `revm`, `reth-*`, `alloy-*`. These had major breaking changes in 2026.

**Medium priority (verify at first use, cite the version, then trust):** `jsonrpsee`, `metrics`, `metrics-exporter-prometheus`, `clap`.

**Low priority (stable; verify only on unexpected behavior):** `tokio`, `thiserror`, `anyhow`, `serde`, `tracing`, `parking_lot`, `rayon`, `crossbeam`.

### How to use Context7

For high/medium priority libraries, before writing code, call:
1. `Context7:resolve-library-id` to find the library's Context7 ID.
2. `Context7:query-docs` to get current API docs.

After verification, **cite the result inline as a comment above the library-using code:**

```rust
// Per Context7 (revm v38, May 2026): Context::mainnet() returns a builder
// that we customize via .with_db() before calling .build_mainnet().
let mut evm = Context::mainnet().with_db(state).build_mainnet();
```

The citation forces you to consult docs before writing, gives reviewers something to verify, and documents the API as of the moment the code was written.

If Context7 returns information that contradicts what AGENTS.md or ARCHITECTURE.md says, **stop and surface the discrepancy**. Do not silently "fix" it in code.

## Domain vocabulary

Use these terms exactly. Do not invent synonyms.

| Term | Definition |
|---|---|
| **Transaction (Tx)** | Standard Ethereum transaction. Format unchanged. |
| **Read Set (RSet)** | Set of state slots a tx reads. Inferred or measured. |
| **Write Set (WSet)** | Set of state slots a tx writes. Inferred or measured. |
| **RW-Set** | `(RSet, WSet)` pair. The unit of conflict reasoning. |
| **Speculation Window** | The batch of N pending txs the sequencer attempts to execute in parallel. |
| **Worker** | A thread (or rayon task) executing one tx against a snapshot, writing to a thread-local journal. |
| **Journal** | An in-memory record of a worker's writes. Discarded on conflict, merged on commit. |
| **Conflict** | When tx B's *actual* RSet overlaps tx A's WSet, where A is earlier in commit order. |
| **Commit Order** | The deterministic order in which speculative results are merged into main state. Defined by mempool ordering, NOT execution completion order. |
| **Re-execution** | When a conflict is detected, B's journal is discarded and B is re-run serially against post-A state. |
| **Speculation Hit Rate** | Fraction of speculatively-executed txs that committed without re-execution. Target: >80% in steady state. |
| **State Snapshot** | A consistent read view of state at a specific commit point. Workers read snapshots; never each other's journals. |
| **Lookahead Depth** | How many pending txs the sequencer pulls into the speculation window at once. |

## Working agreement reminders

- **Two-agent loop.** Planner produces a plan; reviewer checks it; coder implements; reviewer checks the code. One ARCHITECTURE.md step per cycle in early phases.
- **Commit format**: Conventional Commits. `feat(sequencer): add lookahead window`, `fix(rwset): handle dynamic SLOAD`, etc. PR description references the ARCHITECTURE.md step it implements.
- **Definition of done per step**: code passes tests, lint clean (`make lint`), coverage meets target, ARCHITECTURE.md step checked off, AGENTS.md "Current State" updated, reviewer signed off (no 🔴 outstanding).
- **End of session**: update AGENTS.md `Current State` and `Changelog` (Claude does this via filesystem MCP; agents flag it if missed).

## Things this skill is NOT for

- L2 architectural decisions (why speculation? why LSM in V2? what's the security model?) → `krax-architecture` skill.
- revm/reth/alloy API specifics → `krax-rust-engine` skill.
- General Rust knowledge unrelated to Krax conventions.

If a question is "is this allowed in the codebase," it belongs here. If it's "what should the architecture do," it belongs in `krax-architecture`. If it's "how do I call this library," it belongs in `krax-rust-engine`.

# Krax — Agent Context

> **Source of truth for all coding agents working on Krax.**
> Read this in full at the start of every session. Do not skip sections.
> If something here is wrong or out of date, fix it before you fix code.

---

## What Krax Is

Krax is an **EVM-compatible Layer 2** that's "built like a modern database and CPU pipeline, not like a sequential trie machine." It ships in two phases:

- **V1 — Speculative Execution L2.** A standard rollup architecture (MPT/MDBX storage backend) with a novel sequencer that uses **CPU-style speculative parallel execution** with read/write set inference. Target: 5x cheaper gas vs current EVM L2s for typical workloads, fully Solidity-compatible, anchored to Ethereum for security.

- **V2 — Log-Structured State.** Replace the MPT/MDBX backend with an **LSM-tree-native state model** with ZK-friendly commitments over sorted runs. Target: write throughput limited by SSD bandwidth, read throughput limited by RAM, an additional 2-3x cost reduction on top of V1.

The two phases ship sequentially. V1 must be on mainnet with real users and real fee data before V2 work begins. V2 is marketed as "we made it even faster by ripping out the last 1990s component."

### The pitch (for engineering context)

Modern CPUs hit 4-8x speedups in the 1990s by speculating on instruction order and re-executing on conflict. Modern databases abandoned B-trees for LSM-trees in the 2000s for write-heavy workloads. **Blockchains have done neither.** Every chain still executes transactions serially against a Merkle Patricia Trie. Krax's bet is that the entire systems-software stack underneath blockchains is a generation behind, and rebuilding it the way databases and CPUs already did unlocks order-of-magnitude wins without changing what users or developers see.

### Non-goals

- Not a new VM. EVM-compatible at the contract level, period. Solidity contracts deploy unchanged.
- Not a new consensus protocol. We use existing rollup security via Ethereum settlement.
- Not a privacy chain. Standard rollup transparency.
- Not an L1. Anchoring to Ethereum is a feature, not a limitation.
- Not chasing TPS records. The pitch is **cost per transaction** and **predictable latency**, not benchmark numbers.

---

## Architecture (high-level)

```
                  ┌─────────────────────────────────────────┐
                  │            User / dApp                  │
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
                  ┌─────────────────────────────────────────┐
                  │         JSON-RPC Gateway                │
                  │  (eth_*, krax_* extensions)            │
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
                  ┌─────────────────────────────────────────┐
                  │              Mempool                    │
                  │  (orders pending txs, exposes lookahead) │
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
                  ┌─────────────────────────────────────────┐
                  │        Speculative Sequencer            │
                  │                                         │
                  │  ┌─────────────────────────────────┐    │
                  │  │  RW-Set Inference Engine        │    │
                  │  │  (static + profiling, EVM)      │    │
                  │  └────────────┬────────────────────┘    │
                  │               ▼                         │
                  │  ┌─────────────────────────────────┐    │
                  │  │  Parallel Worker Pool           │    │
                  │  │  (N workers, snapshot reads)    │    │
                  │  └────────────┬────────────────────┘    │
                  │               ▼                         │
                  │  ┌─────────────────────────────────┐    │
                  │  │  Conflict Detector + Commit     │    │
                  │  │  (deterministic merge order)    │    │
                  │  └────────────┬────────────────────┘    │
                  └───────────────┼─────────────────────────┘
                                  │
                                  ▼
                  ┌─────────────────────────────────────────┐
                  │           State Backend                 │
                  │  V1: MPT over MDBX                      │
                  │  V2: LSM-native + sorted-run commitments│
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
                  ┌─────────────────────────────────────────┐
                  │         Batcher / Prover                │
                  │  (posts batches + ZK proofs to L1)      │
                  └────────────────────┬────────────────────┘
                                       │
                                       ▼
                  ┌─────────────────────────────────────────┐
                  │          Ethereum L1 Bridge             │
                  └─────────────────────────────────────────┘
```

**Components:**

- **JSON-RPC Gateway** — standard `eth_*` namespace plus `krax_*` extensions (e.g. `krax_getSpeculationStats`).
- **Mempool** — orders pending txs by gas price, exposes a "lookahead window" of N pending txs to the sequencer for batch speculation.
- **Speculative Sequencer** — the core innovation. Three sub-components:
  - **RW-Set Inference Engine** — for each pending tx, infer its read-set and write-set. Static analysis of EVM bytecode where possible; runtime profiling cache for warm contracts; conservative fallback (assume conflict) when uncertain.
  - **Parallel Worker Pool** — N workers (rayon threads or tokio tasks dispatched to a CPU-bound pool), each executing a non-conflicting tx against a consistent state snapshot. Each worker writes to a thread-local journal, not the main state.
  - **Conflict Detector + Commit** — after parallel execution, merge journals into the main state in deterministic order. Re-execute any tx whose actual read-set turned out to overlap an earlier-committed tx's write-set.
- **State Backend** — pluggable. V1 is MPT-over-MDBX (boring, proven). V2 is LSM-native with custom commitments.
- **Batcher / Prover** — collects committed blocks, generates ZK proofs of correct execution, posts to Ethereum L1.

### Why this architecture is L2-shaped (not L1)

Speculative execution requires the sequencer to be authoritative — workers are speculating against a single consistent view of state. On L1, every validator would have to agree on speculation order and conflict outcomes, which is a much harder protocol problem. On L2 with a single (or coordinated) sequencer, speculation is purely a local optimization — the network sees only committed results.

LSM-native state has the same shape. On L1, every full node has to maintain the LSM identically and prove inclusion to light clients. On L2, the sequencer maintains the LSM and posts commitments to L1; only the prover needs to reason about LSM internals.

Both phases ship as **EVM-equivalent rollups** in the Ethereum-compatibility sense. Apps deploy the same Solidity bytecode. Wallets connect to a standard JSON-RPC endpoint.

---

## Tech Stack

### Backend (sequencer, prover, RPC)

- **Language:** Rust (latest stable, currently 1.85+)
- **EVM:** [`revm`](https://github.com/bluealloy/revm) as the EVM interpreter (used by `reth`, `foundry`, and most modern Rust EVM tooling)
- **Execution framework:** `reth` consumed as a library — we use its `BlockExecutor`, `State`, and `EvmConfig` abstractions, not its node binary
- **Storage (V1):** `reth-db` (MDBX-backed) for state, with our own MPT layer on top where needed
- **Storage (V2):** Custom LSM implementation (TBD in V2 phase)
- **RPC:** [`jsonrpsee`](https://github.com/paritytech/jsonrpsee) for JSON-RPC server, with `alloy` types for Ethereum-compatible request/response shapes
- **Async runtime:** `tokio`. No alternatives, no abstractions over the runtime.
- **ZK proving:** TBD per phase — Phase 23 will evaluate (RISC0, SP1, Plonky3). All Rust-native.
- **Concurrency:** `tokio` tasks, channels (`tokio::sync::mpsc`, `crossbeam` where bounded synchronous channels fit better), `Arc`/`Mutex`/`RwLock` from stdlib + `parking_lot` where contention matters. No external concurrency frameworks beyond these.
- **Logging:** `tracing` + `tracing-subscriber`. Structured logging only.
- **Metrics:** `metrics` crate facade with a Prometheus exporter
- **Config:** TOML config files + env var overrides via `figment` or `config` crate (final choice deferred to Phase 0)
- **Error handling:** `thiserror` for library errors (typed), `anyhow` only at binary entry points

### Why Rust over Go for this project

- **Reth's `BlockExecutor` trait is purpose-built for the kind of execution-layer experimentation Krax requires.** go-ethereum's `StateDB` was not designed for snapshot-based parallel execution and would fight us at exactly the points where speculative execution needs clean abstractions.
- **The ZK ecosystem is Rust-native.** When V1.x or V2 brings ZK proofs, the prover code lives in the same workspace as the sequencer and shares execution semantics directly. No FFI bridge, no separate codebase.
- **Reth itself is the trajectory.** Paradigm-maintained, production for ~2 years, and where new ambitious L2 teams are converging in 2026.

The performance-language gap (Rust vs Go) is real but small at the EVM-interpreter level (10–30%) and dwarfed by the speculation speedup itself (3–5x). The deciding factors are abstraction fit and ecosystem alignment, not raw performance.

### Smart contracts (L1 bridge, system contracts)

- **Language:** Solidity 0.8.24+
- **Framework:** Foundry (forge, anvil, cast)
- **Style:** OpenZeppelin patterns where applicable; no upgradeable proxies in V1

### Local dev / testing

- **Local L1:** Anvil (Foundry). Run natively (`anvil` in a terminal tab) until a phase introduces a service that benefits from co-managing it via Docker.
- **kraxd runs natively, never in Docker.** Fast iteration, easier debugging (lldb / rust-gdb attach directly), no volume-mount headaches when `target/` rebuilds.
- **Docker Compose: auxiliary services only.** `docker-compose.yml` is reserved for things like Blockscout (Phase 11+), Prometheus + Grafana (Phase 16+), and possibly anvil once it needs to run alongside other services. It's a placeholder file in Phase 0 — services land in the phase that needs them.
- **Integration tests:** `cargo test` with the `integration` feature flag + a real MDBX instance + a running anvil instance.

### Observability

- **Logs:** stdout (JSON in prod via `tracing-subscriber` JSON formatter, pretty in dev)
- **Metrics:** Prometheus → Grafana (later)
- **Traces:** OpenTelemetry via `tracing-opentelemetry` (later)

### What we do NOT use

- ❌ ORMs of any kind. Direct DB access only.
- ❌ Web frameworks (axum, actix-web, rocket) for the JSON-RPC layer. `jsonrpsee` is sufficient and purpose-built. Axum is acceptable for any auxiliary HTTP endpoints (health checks, metrics) but NOT for RPC itself.
- ❌ Generated code from protobufs. We're not building a microservice mesh.
- ❌ Heavy DI frameworks. Constructor injection by hand.
- ❌ `Box<dyn Any>` or stringly-typed interfaces outside JSON deserialization. Strong types everywhere else.
- ❌ `unsafe` without a `// SAFETY:` comment justifying every block. `unsafe` requires reviewer sign-off.
- ❌ `unwrap()` / `expect()` outside tests, build scripts, or genuinely unrecoverable startup invariants. Production code returns `Result`.

---

## Project Structure

Krax is organized as a Cargo workspace. Each crate has a single, clear responsibility. Cross-crate dependencies go through trait definitions in the `krax-types` crate.

```
krax/
├── AGENTS.md                  # this file
├── REVIEWER.md                # adversarial review agent context
├── ARCHITECTURE.md            # phased build plan with steps and gates
├── README.md                  # public-facing project description
├── LICENSE                    # MIT
├── Makefile                   # build, test, lint, run, fmt (wraps cargo)
├── Cargo.toml                 # workspace root
├── Cargo.lock
├── rust-toolchain.toml        # pin Rust version
├── rustfmt.toml               # formatting rules
├── clippy.toml                # lint rules
├── docker-compose.yml         # auxiliary services (anvil, blockscout, prometheus) — NOT kraxd; kraxd runs natively
├── .gitignore
├── .env.example
│
├── bin/
│   ├── kraxd/                 # the sequencer/node binary
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── kraxctl/               # operator CLI (init, status, debug)
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── kraxprover/            # standalone prover binary (Phase 23+)
│       ├── Cargo.toml
│       └── src/main.rs
│
├── crates/
│   ├── krax-types/            # core domain types + cross-crate traits
│   ├── krax-config/           # config loading + validation
│   ├── krax-mempool/          # pending tx pool + lookahead window
│   ├── krax-rwset/            # read/write set inference engine
│   │   └── src/
│   │       ├── static_/       # static EVM bytecode analysis (note: `static` is reserved)
│   │       ├── profile/       # runtime profiling cache
│   │       └── conservative/  # fallback that assumes conflict
│   ├── krax-sequencer/        # speculative execution coordinator
│   │   └── src/
│   │       ├── worker/        # individual parallel workers
│   │       ├── journal/       # thread-local write journals
│   │       └── commit/        # conflict detection + deterministic merge
│   ├── krax-state/            # state backend (pluggable)
│   │   └── src/
│   │       ├── mpt/           # V1 MPT-over-MDBX backend
│   │       └── lsm/           # V2 LSM backend (placeholder until V2)
│   ├── krax-execution/        # revm wrapper, gas accounting
│   ├── krax-batcher/          # batch builder, L1 poster
│   ├── krax-prover/           # ZK proof generation (Phase 23+)
│   ├── krax-rpc/              # JSON-RPC server (eth_* + krax_*)
│   └── krax-metrics/          # Prometheus metric definitions
│
├── contracts/                 # L1 contracts (Foundry project)
│   ├── src/
│   │   ├── KraxBridge.sol
│   │   ├── KraxStateCommit.sol
│   │   └── KraxVerifier.sol
│   ├── test/
│   ├── script/
│   └── foundry.toml
│
├── docs/                      # technical documentation
│   ├── architecture/
│   ├── rwset-inference.md
│   ├── speculation-model.md
│   └── phase-notes/
│
└── scripts/                   # operational helpers
    ├── devnet-up.sh
    ├── devnet-down.sh
    └── fund-test-account.sh
```

**Rules:**
- Cross-crate types live in `krax-types`. No other crate exports types that a third crate would need to import.
- The `krax-state` crate's V1 (mpt) and V2 (lsm) modules both implement traits defined in `krax-types`. V1-specific types do not leak.
- `bin/*` crates contain only entrypoints (`main.rs` plus minimal CLI parsing). All real logic lives in `crates/*`.
- `contracts/` is a self-contained Foundry project. Rust code consumes ABIs from `contracts/out/` via `alloy`'s `sol!` macro or generated bindings.
- Every crate has its own `Cargo.toml`. Workspace-level dependency versions live in the root `Cargo.toml`'s `[workspace.dependencies]` table; crate `Cargo.toml`s reference them with `workspace = true`.

---

## Domain Concepts

These are the load-bearing terms in the codebase. Use them consistently. Do not invent synonyms.

- **Transaction (Tx)** — a standard Ethereum transaction. We don't extend the format.
- **Read Set (RSet)** — the set of state slots a tx reads during execution. Inferred or measured.
- **Write Set (WSet)** — the set of state slots a tx writes during execution. Inferred or measured.
- **RW-Set** — `(RSet, WSet)` pair for a tx. The unit of conflict reasoning.
- **Speculation Window** — the batch of N pending txs the sequencer attempts to execute in parallel.
- **Worker** — a thread (or task on a CPU-bound pool) executing one tx against a state snapshot, writing to a thread-local journal.
- **Journal** — an in-memory record of a worker's writes during speculative execution. Discarded on conflict, merged on commit.
- **Conflict** — when tx B's actual RSet (measured during execution) overlaps tx A's WSet, where A appears earlier in commit order.
- **Commit Order** — the deterministic order in which speculative results are merged into main state. Defined by mempool ordering, NOT execution completion order.
- **Re-execution** — when a conflict is detected, B's journal is discarded and B is re-run serially against the post-A state.
- **Speculation Hit Rate** — fraction of speculatively-executed txs that committed without re-execution. Target: >80% in steady state.
- **State Snapshot** — a consistent read view of state at a specific commit point. Workers read from snapshots; they never read from each other's journals.
- **Lookahead Depth** — how many pending txs the sequencer pulls into the speculation window at once.

---

## Code Architecture Rules

These are non-negotiable. The reviewer will flag violations as 🔴 Must Fix.

### 1. Trait boundaries

- All cross-crate dependencies go through traits defined in `krax-types`.
- Concrete types are defined in their owning crate and never imported by other crates as concrete types — only through traits.
- New cross-crate dependencies require a trait added to `krax-types` first, in a separate commit.

### 2. No global state

- No `static mut`. No `lazy_static!` / `once_cell` / `OnceLock` for mutable global state.
- Read-only configuration constants (`const`, `static`) are fine.
- Constructors take all dependencies explicitly. Wire-up happens in `bin/kraxd/src/main.rs`.

### 3. Errors are typed, always wrapped

- Library crates define their own error type with `thiserror`. Never return a foreign error type unwrapped.
- Wrap with context: `.map_err(|e| RwSetError::InferTx { tx_hash, source: e })`.
- `anyhow::Error` is acceptable ONLY at binary entry points (`bin/*/src/main.rs`).
- Never `panic!` outside `main` startup or genuinely unrecoverable invariants. A panic in the sequencer is a bug.
- Sentinel-style errors are enum variants, not constants: `RwSetError::ConflictDetected { ... }`.
- `unwrap()` and `expect()` are forbidden in production code paths. Tests, build scripts, and startup-only invariants are exempt.

### 4. Logging is structured

- Use `tracing` everywhere. Never `println!`, `eprintln!`, or the `log` crate.
- Use structured fields, not formatted strings: `tracing::info!(tx_hash = %hash, "received transaction")`, not `tracing::info!("received tx {}", hash)`.
- Three log levels in this codebase: `debug` (verbose internals), `info` (significant events), `error` (something went wrong). No `warn` — either it matters (`error`) or it doesn't (`debug`/`info`). `trace` is acceptable for opt-in deep diagnostics.

### 5. Testing is non-negotiable

- Every public item in a crate has a test before it lands.
- Table-driven tests are the default style. Use `#[test]` functions with parameterized helpers, or `rstest` for parameterization.
- Integration tests live in each crate's `tests/` directory and are gated behind a `integration` feature flag where they require external resources (anvil, MDBX).
- Test files mirror module layout: `crates/krax-rwset/src/static_/analyzer.rs` → unit tests in the same file under `#[cfg(test)] mod tests`.
- Coverage target: 80%+ for `krax-sequencer`, `krax-rwset`, `krax-state`. Lower for boilerplate-heavy code.

### 6. Concurrency discipline

- Async tasks are launched from a constructor or a long-lived service method. No fire-and-forget `tokio::spawn` deep in call stacks.
- Every long-running task accepts a `CancellationToken` (from `tokio-util`) or equivalent shutdown signal.
- Shared state between tasks uses channels first, locks second. If you reach for `Mutex` or `RwLock`, document why a channel doesn't work.
- Prefer `parking_lot` locks over stdlib for short critical sections; stdlib for poison-aware paths.
- The worker pool size is configurable, defaults to `std::thread::available_parallelism()`.
- Workers that perform CPU-bound work (EVM execution) run on a `rayon` thread pool or dedicated OS threads, NOT on the `tokio` runtime. Do not block the async runtime.

### 7. Determinism

- The sequencer's commit phase MUST be deterministic given the same input mempool ordering. This is enforced by tests.
- No `HashMap` iteration in commit-path code (use `BTreeMap` or sort before iterating). `HashMap` iteration order is non-deterministic and varies by `RandomState`.
- No `SystemTime::now()`, no `rand` without an explicit seeded RNG, no floating-point arithmetic in state-affecting code.
- Speculative execution can use any order; commit MUST use mempool order.

### 8. State backend trait stability

- The `State` trait in `krax-types` is the V1↔V2 contract. Changes require explicit phase planning.
- V1 mpt code MUST NOT export anything beyond what's required by the trait.
- V2 lsm code, when it lands, MUST NOT require changes to consumers — it's a drop-in replacement.

### 9. Crate boundaries

- `bin/*` may depend on any `crates/*`.
- `crates/*` may depend on other `crates/*` and approved external dependencies.
- `crates/*` may NOT depend on `bin/*`.
- `contracts/*` is independent; Rust code only consumes ABI artifacts from `contracts/out/`.

### 10. Dependency hygiene

- Adding a new external Rust dependency requires justification in the commit message.
- Approved root dependencies (anything else needs review):
  - `revm` — EVM interpreter
  - `reth-*` family (specifically `reth-ethereum`, `reth-db`, `reth-evm`, `reth-execution-types` as needed) — execution-layer abstractions and storage, consumed as git dependencies pinned to a specific rev. Note: `reth-primitives` was removed in Reth 2.0 (April 2026); primitives are now accessed via `reth-ethereum`'s re-exports (`reth_ethereum::primitives::*`). `reth-ethereum-primitives` is not a standalone published crate (only a `0.0.0` crates.io placeholder as of 2026-05-06).
  - `alloy` (`alloy-primitives`, `alloy-rpc-types`, `alloy-sol-types`) — Ethereum types and ABI
  - `tokio` — async runtime
  - `jsonrpsee` — JSON-RPC server
  - `tracing` + `tracing-subscriber` — logging
  - `metrics` + `metrics-exporter-prometheus` — metrics
  - `thiserror` + `anyhow` — error handling
  - `serde` + `serde_json` — serialization
  - `parking_lot` — locks
  - `rayon` — CPU-bound parallelism
  - `crossbeam` — bounded synchronous channels where `tokio::sync::mpsc` doesn't fit
  - `dashmap` — concurrent hashmap (only where the workload genuinely needs it; usually a sharded `HashMap` behind a `Mutex` is simpler)
  - Test-only: `proptest`, `rstest`, `pretty_assertions`

---

## Library Verification Protocol

Krax's stack moves fast — revm, reth, and alloy all had breaking changes in 2026. Agent training data may be stale. To prevent agents from generating code against APIs that no longer exist, every agent that touches an external library MUST verify the current API via Context7 before writing or proposing code.

### For the planner agent

When a plan involves an external library, the planner MUST list the specific Context7 lookups required as part of the plan. Not "check Context7 if needed" — explicit, pre-declared lookups, e.g.:

> Before writing code, query Context7 for `revm` v38 to confirm the `Context::mainnet()` builder API and the `JournalTr` trait location. If either is different from what this plan assumes, surface the discrepancy before the coder begins.

The planner's output should include a **"Library verification checklist"** section listing every library used in the plan and what the planner believes the relevant API surface to be. This way the coder has a concrete checklist of things to verify, and the reviewer has a citation to compare against during review.

### For the coding agent

Before writing any code that uses an external library, the coder MUST query Context7 for that library's current docs. After verification, the coder includes the relevant Context7-confirmed snippet as a comment immediately above the library-using code:

```rust
// Per Context7 (revm v38, May 2026): Context::mainnet() returns a builder
// that we customize via .with_db() before calling .build_mainnet().
let mut evm = Context::mainnet().with_db(state).build_mainnet();
```

This serves three purposes:
1. Forces the coder to consult docs before writing, not after debugging.
2. Gives reviewers a citation to verify against.
3. Documents the API as of the moment the code was written, which is invaluable when the library updates and we have to audit which call sites need adjustment.

If Context7 returns information that contradicts what AGENTS.md or ARCHITECTURE.md says, the coder MUST stop and flag the discrepancy. Do not silently "fix" it in code.

### Verification priority by library

Not every library needs deep verification on every use. The rule is: **anything where the API has changed in the last 18 months gets Context7'd at every use; anything older and stable gets a single Context7 at first use, then we trust it.**

**High priority (verify before every use):**
- `revm` (v38, fast-moving)
- `reth-*` crates (Reth 2.0, April 2026 — major API restructure, `reth-primitives` deprecated)
- `alloy-*` crates (relatively new, still stabilizing)

**Medium priority (verify at first use, cite the version, then trust):**
- `jsonrpsee`
- `metrics` + `metrics-exporter-prometheus`
- `clap` (for CLI bin crates)

**Low priority (stable, well-known APIs — verify only if hitting unexpected behavior):**
- `tokio`
- `thiserror` + `anyhow`
- `serde` + `serde_json`
- `tracing` + `tracing-subscriber`
- `parking_lot`
- `rayon`
- `crossbeam`

### For the reviewer (and for me/Claude during review)

During review, verify a sample of the coder's Context7 citations — not every one, but enough to catch fabricated or stale citations. If a citation looks wrong, query Context7 directly to confirm. Fabricated citations that don't match the actual library API are a 🔴 must-fix.

---

## Design Principles (philosophy)

1. **Boring beats clever for V1.** V1 is a standard rollup with a clever sequencer. Don't slip in fancy ZK or LSM work — that's V2.
2. **Speculation is invisible to users.** A user must not be able to tell whether their tx was speculatively executed or serially executed. Same gas, same result, same latency profile.
3. **Determinism is a feature, not an optimization.** If two sequencers process the same mempool, they MUST produce the same blocks. This is what makes decentralized sequencing tractable later.
4. **The conflict detector is the most security-critical component.** A false negative (missing a real conflict) produces wrong state. A false positive (flagging a non-conflict) wastes work but is safe. Always bias toward false positives.
5. **Hide the architectural pitch from the user-facing surface.** The marketing story is "5x cheaper EVM L2." The architectural story is the moat narrative for engineers and investors. Don't conflate them in product copy.
6. **Phase boundaries are real.** V1 must be on mainnet with real users before V2 starts. Don't pre-build V2 abstractions in V1 code "just in case."

---

## Workflow & Conventions

### Git

- **Branch model:** trunk-based. `main` is always shippable. Feature branches are short-lived (<3 days).
- **Commit format:** Conventional Commits.
  - `feat(sequencer): add lookahead window to mempool`
  - `fix(rwset): handle SLOAD with dynamic key`
  - `chore(deps): update revm to v18.0.0`
  - `docs(architecture): clarify V1↔V2 state trait boundary`
  - `test(commit): add deterministic merge order tests`
  - `refactor(state): extract MPT-specific types into krax-state crate`
- **PRs:** every change goes through a PR, even solo. PR description must reference the ARCHITECTURE.md step it implements.

### Sessions

- A "session" is a continuous block of agent work. Each session ends with:
  1. All tests passing (`make test`)
  2. Lint clean (`make lint`)
  3. AGENTS.md `Current State` section updated
  4. ARCHITECTURE.md updated if a phase step was completed
  5. A commit suggestion in conventional-commit format

### Review loop

- After a coding session, run `$review` (the REVIEWER.md agent).
- The reviewer is **read-only**. It produces a report; it never edits files.
- Apply reviewer findings in a fresh session. Each finding is a separate commit when reasonable.

### Definition of Done (per step)

A phase step is "done" when:
- ✅ Code is written and passes tests
- ✅ Coverage on new code meets the package target
- ✅ Lint is clean
- ✅ The relevant ARCHITECTURE.md step is checked off
- ✅ `Current State` in AGENTS.md is updated
- ✅ Reviewer has signed off (no 🔴 issues outstanding)

---

## Current State

> Rewritten by the agent at the end of every session.
> Keep it tight — the next agent reads this and knows exactly what to do.

**Current Phase:** Phase 0 — Project Setup (Steps 0.1, 0.2, and 0.3 complete, Step 0.4 next)

**What was just completed:**
- **Step 0.3 — Minimal Entrypoint done.** `bin/kraxd/src/main.rs` prints `krax v0.1.0` via `env!("CARGO_PKG_VERSION")` and exits cleanly. `bin/kraxctl/src/main.rs` is a `clap` derive skeleton with `--help` and auto-generated `--version`; an empty `Commands` enum establishes the structural pattern for future subcommands without requiring a restructure of `main.rs` when they land. `clap = "4"` (4.6.1 confirmed via `cargo search`) added to `bin/kraxctl` via workspace dep. `bin/kraxd` has no new dependencies — the version banner is `println!` only; this is a documented Rule 4 exception (startup UX contract, not a log event). `bin/kraxprover` unchanged.
- (Carry forward: Step 0.2 — full `bin/*` and `crates/*` tree, 14 workspace members, `cargo build --workspace` succeeds.)
- (Carry forward: Step 0.1 — revm 38, reth-* git rev `02d1776786abc61721ae8876898ad19a702e0070`, jsonrpsee 0.26, etc. See archived plan for full version table.)

**What to do next (in order):**
1. 🔴 **Step 0.4 — Makefile.** `make build`, `make test`, `make test-integration`, `make lint`, `make run`, `make fmt`, `make clean`. The Phase 0 Gate requires `make run` to work; `cargo run --bin kraxd` now produces the correct output, so the Makefile is the only thing between current state and that gate item.
2. Step 0.5 — `.gitignore` & `.env.example`.
3. Steps 0.6 through 0.9 in order, per ARCHITECTURE.md.

**Blockers:**
- Repository URL is a placeholder (`https://github.com/krax-labs/krax`). Replace before V1.0 branding. Not a blocker for Phase 0 work.
- Project name not finalized. "Krax" is a working name. Search-replace before mainnet branding (V1.1 concern).

**Notes:**
- `kraxd` version banner uses `println!` — documented Rule 4 exception with inline comment in `main.rs`. This is a narrow exception; all future runtime output uses `tracing`.
- `tracing-subscriber` initialization is deferred to a later step alongside `krax-config`.
- The `Commands` enum in `kraxctl` is empty until a step adds a real subcommand. Clippy may warn that the `if cli.command.is_none()` branch is always taken — expected and acceptable until Step 0.8 clippy configuration.
- Do NOT start any sequencer or RW-set work in Phase 0. That's Phase 1+.
- Every external library use MUST be Context7-verified per the Library Verification Protocol section. No exceptions.
- `reth-*` git rev must be updated periodically as reth main advances. When upgrading, change ALL reth-* entries to the same new rev in one commit.

---

## Changelog

> Append to this section at the end of every session. Do not remove old entries.

### Session 0 — Project Planning
**Date:** 2026-05-04
**Agent:** Claude (claude.ai)
**Summary:** Project vision defined. AGENTS.md, REVIEWER.md, ARCHITECTURE.md created. Initial stack picked (Go + go-ethereum + RocksDB for V1). V1/V2 phasing locked in. Domain concepts documented. Code architecture rules established.
**Commit suggestion:** `chore(project): initial planning — AGENTS.md, REVIEWER.md, ARCHITECTURE.md`

### Session 0.5 — Stack & Milestone Revision
**Date:** 2026-05-04
**Agent:** Claude (claude.ai)
**Summary:** Two structural revisions before any code is written:
1. **Language and framework changed from Go + go-ethereum to Rust + revm + reth-as-library.** Decision driven by reth's `BlockExecutor` abstraction fitting Krax's speculative-execution architecture better than go-ethereum's `StateDB`, plus alignment with the Rust-native ZK proving ecosystem for V1.x / V2. Performance was not the deciding factor; abstraction fit and ecosystem trajectory were.
2. **V1 milestone restructure.** Original plan jumped from devnet directly to mainnet beta. Revised plan splits V1 into three milestones: V1.0 (credible testnet — speculation thesis proven via shadow-fork measurements + audit + anchor app), V1.1 (mainnet beta — capped TVL, post-audit), V1.2 (mainnet GA — caps removed, V2 design begins). This separates the "thesis proven" milestone from the "production ops" milestone, which are different problems with different risk profiles.

A reth-as-library POC is now the first concrete task before Phase 0 scaffolding, to confirm the integration model and maintainer Rust velocity before committing to the full structure.
**Commit suggestion:** `docs(project): switch stack to Rust+revm+reth, restructure V1 milestones into 1.0/1.1/1.2`

### Session 1 — Pre-Scaffold Tightening (POC complete + working agreement)
**Date:** 2026-05-06
**Agent:** Claude (claude.ai)
**Summary:** Pre-Phase-0 POC shipped successfully (revm-based, single day) — calibration confirmed Rust velocity and validated the abstraction shape for Krax's Worker/Journal/RWSet. With scaffolding now unblocked, this session tightened the working agreement and the docs before any code lands:
1. **Working agreement formalized:** two-agent loop (planner + coder) with one ARCHITECTURE.md step per cycle, review pass after each cycle, file-length cap of ~500 lines, function-length cap of 60–80 lines, doc comments (`///`) on every public item with the "why," tests specified by planner before code is written, table-driven by default.
2. **Step 0.6 rescoped:** kraxd is NOT containerized. `docker-compose.yml` becomes a placeholder for auxiliary services only (Blockscout, Prometheus, Grafana, possibly anvil later). Anvil runs natively in Phase 0. AGENTS.md "Local dev / testing" section and ARCHITECTURE.md Step 0.6 + Phase 0 Gate updated to match.
3. **Library Verification Protocol added:** new AGENTS.md section requiring Context7 verification on every external library use. Planner declares verification checklist; coder cites Context7 in code comments; reviewer spot-checks citations. Verification priority tiered by library volatility (revm/reth/alloy = high; tokio/serde/thiserror = low).
4. **`reth-primitives` → `reth-ethereum-primitives`:** approved-dependency list updated to reflect Reth 2.0 (April 2026) crate restructure.
**Commit suggestion:** `docs(project): tighten working agreement, rescope docker-compose, add library verification protocol`

### Session 2 — Step 0.1: Cargo Workspace Initialization
**Date:** 2026-05-06
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary:** Created `Cargo.toml` (workspace root) and `rust-toolchain.toml`. Resolved all FIXME values via `cargo search` before writing files. Key decisions:
- `revm = "38"` — crates.io published version is 38.0.0 (the git workspace tag v55 is a separate numbering scheme).
- `reth-*` — git deps pinned to rev `02d1776786abc61721ae8876898ad19a702e0070` (HEAD of main, 2026-05-06). No real crates.io release exists.
- `reth-ethereum-primitives` removed from deps — only a `0.0.0` crates.io placeholder. Primitives accessed via `reth-ethereum`'s re-exports. AGENTS.md Rule 10 updated accordingly.
- `jsonrpsee = "0.26"`, `metrics = "0.24"`, `metrics-exporter-prometheus = "0.18"`.
- `dashmap = "6"` (7.0.0-rc2 is pre-release; stable 6.x pinned).
- `rstest = "0.26"`, Rust toolchain `1.95.0`.
- Edition 2024, resolver 3.
- Verification gate passed: toolchain active at 1.95.0; `cargo metadata --no-deps` fails on missing member paths (correct behavior at Step 0.1).
**Commit suggestion:** `chore(workspace): initialize Cargo workspace — Step 0.1`

### Session 3 — Step 0.2: Directory Structure
**Date:** 2026-05-06
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary:** Created the full `bin/*` and `crates/*` tree from AGENTS.md "Project Structure". 14 workspace members total. Every per-crate `Cargo.toml` uses workspace inheritance and has an empty `[dependencies]` table per the no-speculative-deps rule. Library crates have crate-level `//!` doc comments; binary crates have `fn main() {}` stubs. Sub-module directories created with `.gitkeep`; `mod` declarations deferred to the phase that fills each. `docs/architecture/` and `docs/phase-notes/` created as `.gitkeep` placeholders. `cargo build --workspace` succeeds. Out of scope: Makefile, gitignore, contracts/, scripts/, all root-level config (later Phase 0 steps).
**Commit suggestion:** `chore(workspace): create directory structure — Step 0.2`

### Session 4 — Step 0.3: Minimal Entrypoint
**Date:** 2026-05-07
**Agent:** Claude Code (claude-sonnet-4-6)
**Summary:** Filled `bin/kraxd/src/main.rs` (prints `krax v0.1.0` via `env!("CARGO_PKG_VERSION")`, exits cleanly, `println!` with documented Rule 4 exception) and `bin/kraxctl/src/main.rs` (clap derive skeleton: `--help`, `--version`, empty `Commands` enum for future subcommands, no-args → print help + exit 0). `clap = { workspace = true }` added to `bin/kraxctl/Cargo.toml`; verified as 4.6.1 via `cargo search`. Context7 query confirmed `CommandFactory::command()` and `Command::print_help()` API surface (docs.rs/clap/latest/clap/builder/struct.Command.html). Workspace `Cargo.toml` clap comment updated from ESTIMATED to verified. `bin/kraxd/Cargo.toml` speculative comment corrected. `bin/kraxprover` untouched. `cargo run --bin kraxd` → `krax v0.1.0`, exit 0. `cargo run --bin kraxctl -- --help` → help text, exit 0. `cargo run --bin kraxctl -- --version` → `kraxctl 0.1.0`, exit 0.
**Commit suggestion:** `feat(bin): minimal entrypoints for kraxd and kraxctl — Step 0.3`

---
name: krax-rust-engine
description: How to use the Krax Rust stack correctly — revm, reth (consumed as a library), alloy, jsonrpsee, tokio + rayon — with current API surface as of April–May 2026. Apply this skill whenever writing or modifying Rust code in the Krax project that touches these libraries, when adding or updating dependency versions in `Cargo.toml`, when wiring up the EVM executor or state backend, when designing async + CPU-bound concurrency boundaries, or when handling errors with `thiserror`/`anyhow`. Trigger this skill aggressively for any Rust file modification in the Krax codebase, even when the user doesn't say "revm" or "reth" explicitly — most non-trivial Rust code in this project will touch the stack. This skill enforces the Library Verification Protocol (Context7 before every external library use) and encodes the patterns that worked in the pre-Phase-0 POC.
---

# Krax Rust Engine

This skill captures the patterns for using the Krax stack correctly: revm (EVM interpreter), reth (consumed as a library), alloy (Ethereum types), jsonrpsee (JSON-RPC), tokio + rayon (concurrency), and the error-handling idioms.

The stack moves fast — revm, reth, and alloy all had breaking changes in 2026. Training data is stale by months. **Verify every API via Context7 before writing code.** This skill points you at the patterns; Context7 confirms the syntax.

## Cross-references

- **For codebase rules** (typed errors, BTreeMap rule, file/function caps, doc comments): see `krax-conventions` skill.
- **For architectural decisions** (V1/V2 boundary, security model, why we chose this stack): see `krax-architecture` skill.
- **For source of truth on dependencies**: AGENTS.md "Code Architecture Rules" rule 10 (approved-deps list).

## Library Verification Protocol — non-negotiable

Before writing code that uses any external library:

### Priority tiers

**High priority (verify before every use):**
- `revm` — fast-moving, recent versions around v38 (May 2026)
- `reth-*` crates — Reth 2.0 (April 2026) restructured the crate map; `reth-primitives` was removed and replaced by `reth-ethereum-primitives`
- `alloy-*` crates — still stabilizing

**Medium priority (verify at first use, then trust):**
- `jsonrpsee`
- `metrics`, `metrics-exporter-prometheus`
- `clap`

**Low priority (stable; verify only on unexpected behavior):**
- `tokio`, `tokio-util`
- `thiserror`, `anyhow`
- `serde`, `serde_json`
- `tracing`, `tracing-subscriber`
- `parking_lot`
- `rayon`
- `crossbeam`

### How to verify

1. Call `Context7:resolve-library-id` with the library name (e.g., "revm"). Note the returned ID like `/bluealloy/revm`.
2. Call `Context7:query-docs` with that ID and a specific question (e.g., "What is the current API for creating an EVM context with a custom database?").
3. Cite the result inline:

```rust
// Per Context7 (revm v38, May 2026): Context::mainnet() returns a builder
// that we customize via .with_db() before calling .build_mainnet().
let mut evm = Context::mainnet().with_db(state).build_mainnet();
```

### When Context7 contradicts AGENTS.md/ARCHITECTURE.md

**Stop and surface the discrepancy.** Do not silently "fix" it in code. The maintainer (with Claude's help) adjudicates.

## The Krax stack at a glance

| Layer | Library | Purpose |
|---|---|---|
| EVM interpreter | `revm` | Execute bytecode, capture reads/writes |
| Execution framework | `reth-*` (as library) | `BlockExecutor`, `EvmConfig`, db abstractions |
| State storage (V1) | `reth-db` (MDBX-backed) | Durable KV with our MPT layer on top |
| Ethereum types | `alloy-primitives`, `alloy-rpc-types`, `alloy-sol-types` | `B256`, transactions, RPC shapes, ABI macro |
| RPC server | `jsonrpsee` | JSON-RPC, `eth_*` + `krax_*` |
| Async runtime | `tokio` | Tasks, channels, timers |
| CPU-bound parallelism | `rayon` | Worker pool for EVM execution |
| Logs | `tracing` + `tracing-subscriber` | Structured logging |
| Metrics | `metrics` + `metrics-exporter-prometheus` | Prometheus exporter |
| Errors | `thiserror` (libs), `anyhow` (binaries) | Typed errors with context wrapping |
| Locks | `parking_lot` (default), stdlib (poison-aware paths) | Short critical sections |

## The pre-Phase-0 POC — patterns that worked

The reth-as-library POC at `~/Projects/evm-state-poc/` confirmed three things that should shape Krax code:

1. **`revm` consumed directly works for our needs.** We don't need the full `BlockExecutor` machinery to get RW-set extraction — `Context::mainnet().with_db(state)` plus a wrapping database that traces reads/writes is sufficient at the worker level.
2. **The abstraction shape (`TracingDb` + execution result + journal) maps cleanly onto Krax's `Worker` + `Journal` + `RWSet` types.** The trait names will differ but the shape transfers.
3. **Rust velocity is sufficient** for this project's pace. Maintainer wrote a working POC in a single day.

The POC code is **NOT brought into the Krax tree.** It's reference-only. When we write the real code, we re-derive from scratch with current Context7-verified APIs.

## Async + CPU-bound concurrency — the rule that matters most

**EVM execution is CPU-bound. It runs on `rayon` or dedicated OS threads. NEVER on the `tokio` runtime.**

Blocking the tokio runtime with EVM execution will tank the whole node. The pattern:

```rust
// Per Context7 (rayon vN, date): rayon::spawn schedules onto its global pool.
// Use a scoped pool when you need to wait on results from this batch.

let handles: Vec<_> = txs.into_iter().map(|tx| {
    let snapshot = snapshot.clone();
    rayon::spawn_fifo(move || {
        let worker = Worker::new(snapshot);
        worker.execute(tx) // CPU-bound EVM work
    })
}).collect();
```

Tokio handles: networking (RPC, P2P later, L1 client), I/O (state reads/writes), timers (block production cadence), task supervision. Rayon handles: every transaction execution.

The boundary between them is a `tokio::task::spawn_blocking` call or a channel handoff. Pick the one that fits the data shape — channels for streaming, `spawn_blocking` for one-shot work.

## Error handling pattern

### In library crates (`crates/*`)

Define a typed error per crate with `thiserror`. Every fallible function returns `Result<T, ThisCrateError>`. Wrap foreign errors with context.

```rust
// Per Context7 (thiserror v1.x, stable): #[from] is for direct conversion
// without context; use From-less variants when you want to add context.

#[derive(thiserror::Error, Debug)]
pub enum RwSetError {
    /// The inferer couldn't determine a read/write set for this transaction.
    /// The conservative fallback should have caught this — if it didn't, that's a bug.
    #[error("inferer failed for tx {tx_hash}: {source}")]
    InferTx {
        tx_hash: alloy_primitives::B256,
        #[source]
        source: revm::primitives::EVMError<...>,
    },

    /// The conservative inferer's "everything" sentinel was misused.
    #[error("conservative sentinel reached commit phase — this is a bug")]
    SentinelLeaked,
}
```

Wrap at the boundary, not deep inside:

```rust
fn infer(&self, tx: &PendingTx) -> Result<RWSet, RwSetError> {
    self.evm_call(tx).map_err(|e| RwSetError::InferTx {
        tx_hash: tx.hash,
        source: e,
    })
}
```

### In binary crates (`bin/*`)

`anyhow::Error` is acceptable at `main.rs`. Convert library errors with `anyhow::Context`:

```rust
fn main() -> anyhow::Result<()> {
    let cfg = krax_config::load(&path)
        .context("failed to load config — check KRAX_DATA_DIR")?;
    // ...
    Ok(())
}
```

### Forbidden in production code

- `unwrap()` and `expect()` outside tests, build scripts, and startup-only invariants.
- `panic!` outside `main` startup or genuinely unrecoverable invariants.
- A panic in the sequencer is a bug.

## State backend pattern (V1)

The `State` trait in `krax-types` is the V1↔V2 contract. V1 implements it with MPT over MDBX (via `reth-db`). V2 will implement it with LSM. **V1 mpt code MUST NOT export anything beyond what the trait requires.**

Sketch (subject to Context7 verification when we write Phase 1 code):

```rust
// Per Context7 (reth-db vN, date): MdbxClient exposes a transactional view
// via .tx() / .tx_mut().
// Per Context7 (alloy-primitives vN, date): B256 is the canonical 32-byte hash.

pub trait State {
    fn get(&self, slot: B256) -> Result<B256, StateError>;
    fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError>;
    fn snapshot(&self) -> Box<dyn Snapshot>;
    fn commit(&mut self) -> Result<B256, StateError>;
    fn root(&self) -> B256;
}
```

Snapshot semantics: read-only view at a specific commit point. Workers read snapshots; they never read each other's journals. Released after commit.

## Determinism affects which types you reach for

In commit-path code (anything in `crates/krax-sequencer/src/commit/`):
- **`BTreeMap`/`BTreeSet`, never `HashMap`/`HashSet`.** Hash iteration order is non-deterministic.
- No `SystemTime::now()`, no unseeded `rand`, no float arithmetic.
- Speculative execution (the worker phase) can use any order; the commit phase MUST use mempool order.

In non-commit-path code (mempool, RPC, metrics): `HashMap` is fine where ordering doesn't affect state.

## RPC pattern

`jsonrpsee` for the JSON-RPC server. `alloy-rpc-types` for the request/response shapes. Standard `eth_*` methods plus `krax_*` extensions (`krax_getSpeculationStats`, etc.).

```rust
// Per Context7 (jsonrpsee vN, date): use #[rpc] proc macro to define the
// trait, then implement it; ServerBuilder for HTTP/WS.

#[rpc(server)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> RpcResult<U64>;
    // ...
}
```

Auxiliary HTTP endpoints (health checks, metrics scrape) can use `axum` if needed. **Never axum for the JSON-RPC layer itself** — `jsonrpsee` is purpose-built.

## Logging pattern

`tracing` everywhere. Structured fields, not formatted strings.

```rust
// ✅ Structured, parseable
tracing::info!(
    tx_hash = %tx.hash,
    block_height,
    "transaction received"
);

// ❌ Format string — don't do this
tracing::info!("got tx {} at block {}", tx.hash, block_height);
```

Three levels: `debug` (verbose internals), `info` (significant events), `error` (something went wrong). No `warn`. `trace` for opt-in deep diagnostics.

JSON formatter in production via `tracing-subscriber`'s JSON layer; pretty formatter in dev.

## Metrics pattern

`metrics` crate as the facade; `metrics-exporter-prometheus` for the scrape endpoint. Define metrics in `krax-metrics`, register at startup, increment from anywhere.

```rust
// Per Context7 (metrics vN, date): describe_counter / counter! macros.

metrics::describe_counter!(
    "krax_speculation_misses_total",
    "Number of times a speculatively-executed tx had to be re-executed serially"
);

// At call site:
metrics::counter!("krax_speculation_misses_total").increment(1);
```

## Common pitfalls (especially with stale training data)

- **Don't use `reth-primitives`.** It was removed in Reth 2.0 (April 2026). Use `reth-ethereum-primitives`.
- **Don't assume revm's `Context` API is what you remember.** v38 has builder patterns that may differ from older docs. Context7 every revm code path before writing.
- **Don't use `tokio::sync::Mutex` for short critical sections.** It's an async lock — use `parking_lot::Mutex` for sync paths and reach for `tokio::sync::Mutex` only when you need to hold across `.await` points.
- **Don't mix `crossbeam` channels with `tokio` channels in the same path without a deliberate handoff point.** They can't be selected on together; pick one per channel.
- **Don't introduce `dashmap` casually.** Often a sharded `HashMap` behind a `Mutex` is simpler. Justify in the commit message.
- **Don't add `unsafe` without a `// SAFETY:` comment.** Reviewer sign-off required.

## Things this skill is NOT for

- Codebase rules and conventions (file caps, doc comments, the BTreeMap rule's full rationale, the working agreement) → `krax-conventions` skill.
- L2 architectural decisions (why speculation, why LSM in V2, the security model) → `krax-architecture` skill.
- Solidity / Foundry contract code → out of scope for this skill; that's contract-side work.

If the question is "how do I call this library correctly" → here. If it's "is this allowed in the codebase" → conventions. If it's "should we do this architecturally" → architecture.

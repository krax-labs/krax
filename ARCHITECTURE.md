# Krax — Architecture & Build Plan

> Phased build plan. Each step is small enough for one agent session.
> Agents: work through these in order. Do not skip steps.
> A phase is "complete" only when all steps are checked AND the gate criteria are met.

---

## Phase Overview

| Phase | Name | Goal | Gate |
|---|---|---|---|
| **0** | Project Setup | Scaffold, tooling, dev environment | `make run` starts a binary that prints version |
| **1** | Domain Types & State Interface | Core types, state interface, MPT backend stub | All types defined, state interface stable, MPT backend passes round-trip tests |
| **2** | EVM Execution Wrapper | Single-tx serial execution against MPT state | One tx runs end-to-end; result matches reference revm |
| **3** | Mempool & Lookahead | Pending tx pool with lookahead window | Mempool accepts txs, exposes ordered lookahead, drops invalid txs |
| **4** | RW-Set Inference (Conservative) | Fallback inference: assume worst case | Every tx returns an RW-set; tests prove it's a superset of the true RW-set |
| **5** | Single-Worker Speculative Execution | Worker abstraction with journal, no parallelism yet | One worker executes one tx, writes to journal, commits to state |
| **6** | Conflict Detector & Deterministic Commit | Detect conflicts, merge in mempool order, re-execute on miss | Two-tx scenarios: independent commit, conflicting re-execute, all deterministic |
| **7** | Parallel Worker Pool | N workers, snapshot reads, journal merging | 100-tx batch with random conflicts produces correct state, deterministic results |
| **8** | RW-Set Inference (Static EVM Analysis) | Real static analysis of common patterns | Correctly infers RW-set for ERC-20 transfer, ERC-721 mint, Uniswap V2 swap |
| **9** | RW-Set Profiling Cache | Cache observed RW-sets per (contract, selector) | Hot contracts skip re-inference; cold contracts fall back to conservative |
| **10** | JSON-RPC Gateway | Standard `eth_*` namespace | MetaMask connects, sends a tx, sees the result |
| **11** | Block Production & Internal Storage | Blocks are produced, stored, queryable | `eth_getBlockByNumber` works; blocks survive restart |
| **12** | L1 Bridge Contracts (Solidity) | Deposit/withdraw, state commitment, dispute window | Foundry tests pass; deposit on anvil shows up on Krax |
| **13** | Batcher | Post compressed batches to L1 | Sequencer batches blocks, posts calldata to anvil-L1 |
| **14** | Optimistic State Commitments | Commit state roots to L1 with bond | State commits on L1, can be challenged within window |
| **15** | Public Testnet (Sepolia-anchored) | Open testnet, no real funds, dev-facing | Anyone can connect, deploy a Solidity contract, transact |
| **16** | Speculation Metrics & Tuning | Observability for hit rate, conflict rate | `krax_getSpeculationStats` returns real numbers; hit rate >80% on synthetic load |
| **17** | Shadow-Fork Workload Measurement | Replay mainnet L2 traffic through Krax sequencer; publish measured speedup | Speedup numbers published vs Optimism/Base on perps, AMM, and NFT workloads; matches whitepaper Section 9 targets within stated tolerances |
| **18** | Anchor App Onboarding & Audit | One real application deployed on testnet; third-party audit of conflict detector + commit phase | Anchor app live and processing testnet load; audit report published with all 🔴 findings resolved |
| **19** | **V1.0 Complete — Credible Testnet** | Speculation thesis proven publicly | Measured speedup + audit + anchor app all shipped. Fundraising and V2 design conversations open here. |
| **20** | Mainnet Bridge & Deployment Prep | L1 contracts deployed; sequencer hardened for production ops | Bridge audited; sequencer ops runbook complete; monitoring stack live |
| **21** | **V1.1 — Mainnet Beta** | Real ETH, capped TVL, anchor app on mainnet | Mainnet sequencer running with TVL cap; no incidents over 30-day window |
| **22** | **V1.2 — Mainnet GA (V1 Complete)** | Cap removed, public launch | Public launch announcement; sustained mainnet operation; V2 design work begins |
| **23** | ZK Prover Integration | Validity proofs over batches | Proofs generated, posted to L1, verified. (May land in V1.x or V2 depending on prover maturity.) |
| **24** | V2 Planning | LSM design, ZK-friendly commitments research | Design doc + spike completed, V2 ARCHITECTURE.md written |
| **25–N** | V2 Build | Phases TBD when V1.2 GA ships | TBD |

---

## Phase 0 — Project Setup

**Goal:** Working dev environment, all scaffolding in place. A new agent can clone the repo and run `make run` successfully.

**Pre-Phase-0 Sanity Check (do this FIRST):**
- [ ] **Reth-as-library POC.** Standalone ~500-line Rust binary that pulls `revm` + `reth-evm` + `reth-db`, instantiates a `BlockExecutor`, runs one transaction end-to-end, asserts the post-state matches expectations. This is throwaway code, NOT part of the project tree. Goal: confirm the integration model works and that maintainer Rust velocity is sufficient before scaffolding.

### Step 0.1 — Cargo Workspace Initialization ✅
- [x] Create root `Cargo.toml` with `[workspace]` table listing all member crates and binaries
- [x] Pin Rust version via `rust-toolchain.toml` (channel = "1.95.0", edition 2024 requires 1.85+)
- [x] Configure `[workspace.dependencies]` with shared dep versions (revm, reth-*, alloy-*, tokio, etc.)

### Step 0.2 — Directory Structure ✅
- [x] Create the full tree from AGENTS.md "Project Structure"
- [x] Add a `.gitkeep` file in each empty directory
- [x] Each `bin/*` and `crates/*` gets its own `Cargo.toml`

### Step 0.3 — Minimal Entrypoint ✅
- [x] Create `bin/kraxd/src/main.rs` that prints `krax vX.Y.Z` and exits cleanly
- [x] Create `bin/kraxctl/src/main.rs` placeholder with `--help` only (use `clap` derive)

### Step 0.4 — Makefile ✅
- [x] `make build` — runs `cargo build --workspace --release`
- [x] `make test` — runs `cargo test --workspace`
- [x] `make test-integration` — runs `cargo test --workspace --features integration`
- [x] `make lint` — runs `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `make run` — runs `cargo run --bin kraxd`
- [x] `make fmt` — runs `cargo fmt --all`
- [x] `make clean` — runs `cargo clean` and removes `data/`

### Step 0.5 — .gitignore & .env.example ✅
- [x] `.gitignore`: `target/`, `data/`, `.env`, `.env.local`, `coverage/`, `.idea/`, `.vscode/`, `.DS_Store`, `*.log`
- [x] `.env.example` with: `KRAX_DATA_DIR`, `KRAX_RPC_PORT`, `KRAX_L1_RPC_URL`, `KRAX_LOG_LEVEL`

### Step 0.6 — Docker Compose (Auxiliary Services Placeholder)
- [ ] Create a placeholder `docker-compose.yml` at the project root with no active services and a header comment explaining the file's purpose: auxiliary services (anvil, Blockscout, Prometheus, Grafana) land here in the phases that introduce them. **kraxd itself is NOT containerized** — it runs natively via `make run` for fast iteration and easier debugging.
- [ ] Create `scripts/devnet-up.sh` and `scripts/devnet-down.sh` as placeholder scripts (also no-op for now, with comments explaining they will start auxiliary services in later phases). They exist now so paths are stable; they do nothing until a service is added.
- [ ] **Anvil for Phase 0:** developers run anvil natively via `anvil` in a terminal tab. No Docker required. Anvil moves into `docker-compose.yml` at the phase that first depends on it being co-managed with another service (likely Phase 11 or 12).

### Step 0.7 — Foundry Init for Contracts
- [ ] Run `forge init contracts/ --no-git`
- [ ] Configure `contracts/foundry.toml` for solc 0.8.24
- [ ] Add `contracts/.gitignore` for `out/`, `cache/`, `broadcast/`

### Step 0.8 — Lint & Format Configuration
- [ ] `rustfmt.toml` with project-wide formatting rules
- [ ] `clippy.toml` with allowed/denied lints
- [ ] Verify `cargo clippy` passes on the empty workspace

### Step 0.9 — README
- [ ] Public-facing README with one-paragraph description, build steps, quick start
- [ ] Link to AGENTS.md and ARCHITECTURE.md for contributors

**Phase 0 Gate:**
- ✅ `make build` succeeds
- ✅ `make run` prints version and exits 0
- ✅ `make test` runs (zero tests is fine)
- ✅ `make lint` passes with `-D warnings`
- ✅ `make fmt` is idempotent (running twice produces no diff)
- ✅ `cd contracts && forge build` succeeds
- ✅ `docker-compose.yml`, `scripts/devnet-up.sh`, and `scripts/devnet-down.sh` exist as placeholders with explanatory comments
- ✅ Anvil installed locally and reachable at `localhost:8545` when run via `anvil` in a terminal

---

## Phase 1 — Domain Types & State Trait

**Goal:** Define the core types and the V1↔V2 `State` trait. No real logic yet, but every type has tests.

### Step 1.1 — Core Type Files
- [ ] `crates/krax-types/src/tx.rs` — re-export `alloy-primitives` transaction types, plus our `PendingTx` struct (tx + arrival time + sender)
- [ ] `crates/krax-types/src/block.rs` — `Block` struct (parent hash, height, timestamp, txs, state root)
- [ ] `crates/krax-types/src/rwset.rs` — `RWSet` struct (`r_set: BTreeSet<B256>`, `w_set: BTreeSet<B256>`); methods: `conflicts(other: &RWSet) -> bool`, `union(self, other: RWSet) -> RWSet`
- [ ] `crates/krax-types/src/journal.rs` — `Journal` struct (ordered `Vec<(B256, Option<B256>, B256)>` of (slot, old, new) writes); methods: `apply(&self, state: &mut dyn State)`, `discard(self)`
- [ ] `crates/krax-types/src/state.rs` — `State` trait: `fn get(&self, slot: B256) -> Result<B256, StateError>`, `fn set(&mut self, slot: B256, val: B256) -> Result<(), StateError>`, `fn snapshot(&self) -> Box<dyn Snapshot>`, `fn commit(&mut self) -> Result<B256, StateError>`, `fn root(&self) -> B256`
- [ ] `crates/krax-types/src/snapshot.rs` — `Snapshot` trait: `fn get(&self, slot: B256) -> Result<B256, StateError>`, `fn release(self: Box<Self>)`
- [ ] Use `BTreeSet`/`BTreeMap` (NOT `HashSet`/`HashMap`) anywhere ordering or determinism matters

### Step 1.2 — Type Tests
- [ ] `RWSet::conflicts` truth table tests (8 cases: empty/disjoint/overlap × R-only/W-only/RW)
- [ ] `RWSet::union` tests
- [ ] `Journal::apply` round-trip test (apply then read returns expected value)
- [ ] `Journal::discard` test (discarded journal does not affect state)

### Step 1.3 — MPT State Backend (Skeleton)
- [ ] `crates/krax-state/src/mpt/mod.rs` — `MptState` struct backed by MDBX (via `reth-db`)
- [ ] Implement `State` trait against an in-memory map first
- [ ] Wire MDBX as the durable backend
- [ ] Round-trip test: `state.set(k, v); state.commit(); state.get(k) == v`
- [ ] Restart test: open DB, set, commit, close, reopen, get returns committed value

### Step 1.4 — Snapshot Semantics
- [ ] `snapshot()` returns a read-only view at the current commit point
- [ ] Test: `let s = state.snapshot(); state.set(k, v2); s.get(k) == v1` (snapshot is isolated)
- [ ] Test: `s.release()` then `s.get` returns a `StateError::Released`

**Phase 1 Gate:**
- ✅ All types in `krax-types` have tests
- ✅ MPT state backend passes round-trip and restart tests
- ✅ Snapshot isolation is enforced and tested
- ✅ Coverage on `krax-types` and `krax-state` is >85%

---

## Phase 2 — EVM Execution Wrapper

**Goal:** Run a single transaction against MPT state, get a deterministic result that matches `revm`'s reference behavior.

### Step 2.1 — EVM Wrapper
- [ ] `crates/krax-execution/src/executor.rs` — `Executor` struct wrapping `revm`'s `Evm` builder
- [ ] `execute(tx, state) -> Result<ExecutionResult>` — runs one tx, returns gas used, status, logs, and the writes performed
- [ ] Capture writes via a custom `Database` impl on top of our `State` trait that records every `storage` write call

### Step 2.2 — Reference Equivalence Tests
- [ ] Build a small set of canonical txs: simple transfer, ERC-20 transfer, contract deploy, contract call
- [ ] For each, run through our Executor and through a vanilla `revm` setup
- [ ] Assert the post-state hashes match exactly

### Step 2.3 — Gas Accounting
- [ ] Verify gas used matches reference revm exactly across the canonical tx set
- [ ] Add tests for out-of-gas, revert, invalid opcode

**Phase 2 Gate:**
- ✅ Canonical tx set produces post-state hashes identical to reference revm
- ✅ Gas accounting is exact match
- ✅ Coverage on `krax-execution` >80%

---

## Phase 3 — Mempool & Lookahead

**Goal:** A working mempool that accepts txs, orders them, and exposes a lookahead window to the sequencer.

### Step 3.1 — Mempool Core
- [ ] `crates/krax-mempool/src/pool.rs` — `Mempool` with `add(tx) -> Result<(), MempoolError>`, `lookahead(n: usize) -> Vec<PendingTx>`, `remove(hashes: &[B256])`
- [ ] Ordering: by gas price descending, then by arrival time
- [ ] Reject txs with invalid signature, insufficient balance, nonce gap

### Step 3.2 — Mempool Tests
- [ ] Add 100 txs with varied gas prices; lookahead returns them in correct order
- [ ] Invalid tx rejected with specific error variant
- [ ] Removed txs do not appear in subsequent lookaheads
- [ ] Concurrent `add` from multiple tasks: no race, no lost txs (use `loom` or stress test with `tokio::spawn` × N)

### Step 3.3 — Mempool Metrics
- [ ] Prometheus gauges via the `metrics` crate: pool size, pool age (oldest tx), reject counts by reason

**Phase 3 Gate:**
- ✅ Mempool accepts/rejects per spec
- ✅ Lookahead is stable and ordered
- ✅ Concurrent access is race-free (verified by stress tests; consider `loom` for the lock-protected sections)

---

## Phase 4 — RW-Set Inference (Conservative Fallback)

**Goal:** Every tx returns an RW-set. The conservative fallback assumes the worst (overlap with everything else), which is always safe.

### Step 4.1 — Conservative Inferer
- [ ] `crates/krax-rwset/src/conservative.rs` — returns an RW-set marked as "everything"
- [ ] Define the "everything" sentinel in `crates/krax-types/src/rwset.rs` (a dedicated `RWSet::Everything` variant or a flag) and ensure `conflicts` returns true against it

### Step 4.2 — Inferer Trait
- [ ] `crates/krax-types/src/inferer.rs` — `Inferer` trait: `fn infer(&self, tx: &PendingTx) -> Result<RWSet, InferError>`
- [ ] Conservative inferer implements this trait

### Step 4.3 — Tests
- [ ] Conservative inferer's RW-set conflicts with every other RW-set
- [ ] Two conservative-inferred txs: always re-execute serially when checked against each other

**Phase 4 Gate:**
- ✅ Inferer trait is in place
- ✅ Conservative implementation is correct (always over-approximates)

---

## Phase 5 — Single-Worker Speculative Execution

**Goal:** Worker abstraction exists, executes one tx against a snapshot, writes to a journal. No parallelism yet — just the abstraction.

### Step 5.1 — Worker
- [ ] `crates/krax-sequencer/src/worker.rs` — `Worker` struct, `run(&self, tx: PendingTx, snapshot: &dyn Snapshot) -> Result<(Journal, RWSet), WorkerError>`
- [ ] Worker reads from the snapshot, writes to its journal
- [ ] Worker also captures the *actual* RW-set during execution (this is what we'll use for conflict detection later)

### Step 5.2 — Tests
- [ ] Single tx: worker produces a journal whose `apply` produces the expected post-state
- [ ] Worker does not mutate the snapshot
- [ ] Captured actual RW-set matches what the EVM read/wrote

**Phase 5 Gate:**
- ✅ Worker abstraction is testable in isolation
- ✅ Captured RW-set is accurate (compare to manual instrumentation)

---

## Phase 6 — Conflict Detector & Deterministic Commit

**Goal:** Given multiple workers' journals, detect conflicts and merge in deterministic mempool order. Re-execute conflicting txs serially.

### Step 6.1 — Conflict Detector
- [ ] `crates/krax-sequencer/src/commit/detector.rs` — given a `Vec<(PendingTx, RWSet, Journal)>` in mempool order, detect each tx's conflict status
- [ ] A tx conflicts if its actual RSet ∩ any earlier-committed tx's WSet is non-empty
- [ ] Conflict detector is pure: same input → same output, deterministic. No interior mutability except for explicit ordered builders.

### Step 6.2 — Commit Phase
- [ ] `crates/krax-sequencer/src/commit/mod.rs` — `pub fn commit(state: &mut dyn State, results: Vec<...>) -> Result<CommitReport, CommitError>`
- [ ] For each tx in mempool order:
  - If no conflict: apply journal to state
  - If conflict: re-execute serially against current state, apply
- [ ] Returns a `CommitReport` with counts: committed, re-executed, total gas

### Step 6.3 — Two-Tx Scenarios
- [ ] Independent txs (disjoint RW-sets): both commit speculatively, no re-execution
- [ ] Conflicting txs (B reads what A writes): A commits, B re-executes
- [ ] Same-slot writes: both commit, B's write overrides A's (semantically correct EVM behavior)
- [ ] Determinism: run same scenario 100 times, post-state hash is identical

**Phase 6 Gate:**
- ✅ All two-tx scenarios produce correct, deterministic state
- ✅ Re-execution rate matches expectations on synthetic conflicts
- ✅ No flakes when stress-tested (e.g. `cargo test -- --test-threads=N` × 100 iterations)

---

## Phase 7 — Parallel Worker Pool

**Goal:** Many workers run in parallel against snapshots, journals merge correctly, throughput goes up.

### Step 7.1 — Worker Pool
- [ ] `crates/krax-sequencer/src/sequencer.rs` — `Sequencer` struct with worker pool
- [ ] Pool size from config, default `std::thread::available_parallelism()`
- [ ] Use `rayon` thread pool for CPU-bound EVM execution; do NOT block the tokio runtime
- [ ] Dispatch: pull lookahead from mempool, fan out to workers, collect results, hand to commit phase

### Step 7.2 — Snapshot Sharing
- [ ] All workers in a single speculation window read from the same `Arc<dyn Snapshot>`
- [ ] Snapshot is released only after commit phase completes

### Step 7.3 — 100-Tx Scenario Test
- [ ] Generate 100 txs with controlled conflict rate (e.g. 20% true conflicts)
- [ ] Run through parallel sequencer
- [ ] Compare post-state to a serial baseline — must match exactly
- [ ] Run 1000 times; no flakes, no races

### Step 7.4 — Throughput Sanity Check
- [ ] `criterion`-based benchmark: 10k independent txs through a 16-worker pool vs serial baseline
- [ ] Speedup must be ≥4x; benchmark fails the CI gate if not

**Phase 7 Gate:**
- ✅ Parallel sequencer matches serial baseline exactly across all test scenarios
- ✅ Throughput speedup is measurable and meaningful
- ✅ Determinism holds under concurrency

---

## Phase 8 — Static EVM Analysis for RW-Set Inference

**Goal:** Real static analysis that infers tight RW-sets for common patterns. Falls back to conservative when uncertain.

### Step 8.1 — EVM Bytecode Walker
- [ ] `crates/krax-rwset/src/static_/walker.rs` — walks EVM bytecode tracking SLOAD/SSTORE targets (note: `static` is reserved in Rust, hence `static_`)
- [ ] Handles literal slot keys (most common case): exact RW-set
- [ ] Handles slot keys derived from `msg.sender`, calldata: parameterized RW-set
- [ ] Falls back to conservative when slot key is fully dynamic

### Step 8.2 — Pattern Recognition
- [ ] ERC-20 `transfer(to, amount)`: reads sender + recipient balance, writes both
- [ ] ERC-20 `approve(spender, amount)`: writes allowance slot
- [ ] ERC-721 `safeMint`: reads totalSupply, writes owner + balance
- [ ] Uniswap V2 `swap`: reads reserves + balances, writes them
- [ ] For each pattern, test that inferred RW-set matches actual EVM execution

### Step 8.3 — Inferer Composition
- [ ] `crates/krax-rwset/src/composite.rs` — composite inferer: try static → fall back to conservative
- [ ] Test that the composite is always correct (never under-approximates)

**Phase 8 Gate:**
- ✅ Static analyzer correctly infers RW-sets for the four canonical patterns
- ✅ Composite inferer is provably safe (over-approximates when uncertain)
- ✅ Speculation hit rate on a synthetic ERC-20 workload >85%

---

## Phase 9 — RW-Set Profiling Cache

**Goal:** Hot contracts get their RW-sets cached after first observation. Cold paths fall back through the inference chain.

### Step 9.1 — Cache Structure
- [ ] `crates/krax-rwset/src/profile/cache.rs` — keyed by `(Address, [u8; 4])` (contract address + function selector)
- [ ] Cache entries store the parameterized RW-set template
- [ ] LRU eviction with configurable size (default 10k entries) — `lru` crate or hand-rolled

### Step 9.2 — Cache Population
- [ ] After every executed tx, the actual RW-set is fed back to the cache
- [ ] Cache is read by the inferer before falling through to static or conservative

### Step 9.3 — Tests
- [ ] First call to a contract: cache miss, falls through to static/conservative
- [ ] Second call to same selector: cache hit, returns cached RW-set
- [ ] Cache eviction under load: LRU policy is correct

**Phase 9 Gate:**
- ✅ Cache hit rate on repeated workloads >95%
- ✅ Cache never produces an unsafe (under-approximating) RW-set
- ✅ Memory bounded under sustained load

---

## Phase 10 — JSON-RPC Gateway

**Goal:** MetaMask connects, sends a tx, sees the result. We support the standard `eth_*` namespace.

### Step 10.1 — RPC Server
- [ ] `crates/krax-rpc/src/server.rs` — HTTP JSON-RPC server using `jsonrpsee`
- [ ] Listens on configured port
- [ ] Use `alloy-rpc-types` for Ethereum-standard request/response shapes

### Step 10.2 — Required Methods
- [ ] `eth_chainId`, `eth_blockNumber`, `eth_getBalance`, `eth_getCode`, `eth_getStorageAt`, `eth_call`, `eth_estimateGas`, `eth_gasPrice`, `eth_sendRawTransaction`, `eth_getTransactionReceipt`, `eth_getTransactionByHash`, `eth_getBlockByNumber`, `eth_getBlockByHash`
- [ ] Each method has a unit test

### Step 10.3 — MetaMask Integration Test
- [ ] Manual test: configure MetaMask custom network pointed at devnet, deploy a contract via Remix, transact, see balance change

**Phase 10 Gate:**
- ✅ All required `eth_*` methods implemented and tested
- ✅ MetaMask end-to-end test passes

---

## Phase 11 — Block Production & Internal Storage

**Goal:** Sequencer produces real blocks, stores them, blocks survive restart, blocks are queryable via RPC.

(Detailed steps to be filled in when Phase 10 ships.)

---

## Phase 12 — L1 Bridge Contracts (Solidity)

**Goal:** Deposit/withdraw flow with state commitment and dispute window.

(Detailed steps to be filled in when Phase 11 ships.)

---

## Phase 13 — Batcher

**Goal:** Sequencer collects blocks, compresses, posts to L1 as calldata.

(Detailed steps to be filled in when Phase 12 ships.)

---

## Phase 14 — Optimistic State Commitments

**Goal:** State roots committed to L1 with bond, challengeable within window.

(Detailed steps to be filled in when Phase 13 ships.)

---

## Phase 15 — Public Testnet (Sepolia-anchored)

**Goal:** Open testnet, no real funds. Developers can connect, deploy, and transact. This is the foundation Krax uses to prove the thesis publicly — it is NOT the mainnet target.

(Detailed steps TBD.)

---

## Phase 16 — Speculation Metrics & Tuning

**Goal:** Real observability. `krax_getSpeculationStats` returns hit rate, conflict rate, average lookahead depth, throughput.

(Detailed steps TBD.)

---

## Phase 17 — Shadow-Fork Workload Measurement

**Goal:** Replay real mainnet L2 traffic (Optimism, Base) through the Krax sequencer and publish measured speedup numbers. This is the empirical proof of the speculation thesis and the load-bearing milestone for the V1.0 launch.

**Why this matters:** The whitepaper commits to publishing measured speedup on production workloads. Without this phase, the speculation thesis remains a claim. With it, Krax has defensible numbers to point to in the V1.0 announcement, in fundraising, and in V2 planning.

### What this phase produces
- Workload corpora derived from public L2 traffic (perps, AMM, NFT, governance, oracle mixes)
- A replay harness that feeds these corpora through the Krax sequencer
- Published benchmarks with methodology, hardware specs, and reproduction steps
- Comparison to a serial-execution baseline on the same hardware

(Detailed steps TBD; will require Phase 9 cache to be warm and Phase 16 metrics in place.)

---

## Phase 18 — Anchor App Onboarding & Audit

**Goal:** One real application deployed on testnet, generating realistic load. Third-party audit of the conflict detector and commit phase complete with all 🔴 findings resolved.

**Why an anchor app on testnet:** Synthetic benchmarks are not enough. An anchor app generates the messy, unpredictable workload patterns that surface real bugs and real edge cases in speculation. This is what mainnet would do, but without putting real funds at risk while V1 hardens.

**Why the audit here, not later:** The conflict detector is the most security-critical component in the codebase (per REVIEWER.md). Auditing before mainnet beta — not before mainnet GA — ensures any deep architectural issues are found while the system is still cheap to change.

(Detailed steps TBD.)

---

## Phase 19 — V1.0 Complete: Credible Testnet

**Goal:** The speculation thesis is proven publicly. Krax has audited code, measured speedup numbers, and a real anchor app on testnet.

**Gate Criteria:**
- ✅ Phases 0–18 complete
- ✅ Shadow-fork measurements published with methodology
- ✅ Audit report public, all 🔴 findings resolved
- ✅ Anchor app live on testnet with sustained load
- ✅ Speculation hit rate, conflict rate, and throughput numbers match whitepaper Section 9 targets within stated tolerances

**What this milestone unlocks:**
- Fundraising conversations on the basis of measured performance, not claims
- V2 design work can begin in parallel with V1.1/V1.2 ops work (V2 IMPLEMENTATION still gated on V1.2 GA)
- Anchor partnerships and ecosystem outreach can lead with real numbers

**What this milestone does NOT mean:**
- Krax is not on mainnet
- No real funds are at risk
- The product is not yet generating fee revenue

This is the "thesis proven" milestone, not the "production launch" milestone. Those are separated deliberately.

---

## Phase 20 — Mainnet Bridge & Deployment Prep

**Goal:** Everything required to run a production mainnet L2: hardened L1 contracts, sequencer ops runbook, monitoring stack, incident response plan, withdrawal monitoring, key management.

**Why this is its own phase:** Mainnet operations are a distinct discipline from speculative execution engineering. Lumping them into "Mainnet Beta" hides the work and underestimates the time. Splitting them out makes the operational lift visible and plannable.

(Detailed steps TBD.)

---

## Phase 21 — V1.1: Mainnet Beta

**Goal:** Real ETH on mainnet, capped TVL, anchor app deployed to mainnet. The architecture is identical to V1.0 — only the deployment target changes.

**Gate Criteria:**
- ✅ Phase 20 complete (ops infrastructure ready)
- ✅ Mainnet sequencer running with TVL cap
- ✅ No 🔴 incidents over a 30-day observation window
- ✅ Withdrawals processing reliably
- ✅ Bond economics working as specified

(Detailed steps TBD.)

---

## Phase 22 — V1.2: Mainnet GA (V1 Complete)

**Goal:** TVL caps removed. Public launch. V2 design work begins.

**Gate Criteria:**
- ✅ V1.1 stable for 60+ days
- ✅ No outstanding 🔴 incidents
- ✅ Public launch announcement
- ✅ Sustained mainnet operation

V2 IMPLEMENTATION begins ONLY after this milestone. V2 *design* work can begin at V1.0 (Phase 19); V2 *code* cannot begin until V1.2.

---

## Phase 23 — ZK Prover Integration

**Goal:** Validity proofs over batches. Posted to L1 and verified on-chain.

This phase may land within V1.x (replacing or supplementing fault proofs) or as part of V2 (alongside the LSM commitment). The decision depends on prover maturity at the time of V1.2 GA.

(Detailed steps TBD; requires evaluation of RISC0 vs SP1 vs Plonky3.)

---

## Phase 24 — V2 Planning

**Goal:** LSM-native state design, ZK-friendly commitment scheme, V2 ARCHITECTURE.md.

V2 design work can begin at V1.0 (Phase 19) for spike-and-research purposes. V2 code work cannot begin until V1.2 GA (Phase 22) ships. Pre-building V2 abstractions in V1 code is forbidden.

(Detailed steps to be written when V1.2 GA ships.)

---

## Phases 25+ — V2 Build

TBD when Phase 24 design doc lands.

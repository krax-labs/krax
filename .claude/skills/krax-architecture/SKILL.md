---
name: krax-architecture
description: Krax-specific L2 architectural decisions and the reasoning behind them. Apply this skill whenever the user asks about — or is making a decision that touches — Krax's sequencer logic, RW-set inference, conflict detection, commit ordering, the V1/V2 phase boundary, the speculative execution model, the predictive finality model, the security model, the choice of state backend (MPT in V1 vs LSM in V2), the settlement contract design, the bond/slashing mechanism, the fault proof vs ZK proof transition, or any non-goal Krax has explicitly chosen (no new VM, no new consensus, no privacy chain, no cross-rollup primitives). Trigger this skill even when the user doesn't explicitly say "architecture" — it should fire on topics like "should we add X to the sequencer," "how should this work for V2," "is this safe under our security model," "why are we doing it this way," or any planning that involves architectural rationale rather than implementation mechanics.
---

# Krax Architecture

This skill encodes the architectural decisions specific to Krax — what we've chosen, and why. It is not a general L2 knowledge base. The whitepaper (`Krax_Whitepaper_v0_1.docx` at the project root) is the authoritative long form; this skill is the operating summary that agents should consult before second-guessing decisions that have already been made.

The job of this skill is to **prevent architectural drift**: if an agent is about to propose a design that contradicts a decision Krax has already made, this skill should catch that.

## Cross-references

- **For the codebase rules** (typed errors, BTreeMap not HashMap, Library Verification Protocol, file/function caps, test conventions): see `krax-conventions` skill.
- **For revm/reth/alloy implementation patterns**: see `krax-rust-engine` skill.
- **For full rationale**: read `Krax_Whitepaper_v0_1.docx` (project root) and `ARCHITECTURE.md`.

## The bet (one paragraph)

Ethereum L2 execution is using outdated systems-software primitives. Krax applies two architectural transitions that already happened in databases and CPUs: **CPU-style speculative parallel execution** in the sequencer (V1), and **LSM-tree state commitment** in place of MPT (V2). V1 is mainnet-shippable on its own and proves the speedup thesis on production workloads. V2 ships strictly after V1 mainnet GA, never in parallel with it.

## V1 vs V2 phase boundary (HARD RULE)

V1 and V2 ship sequentially. **V2 implementation work cannot begin before V1.2 mainnet GA.** V2 *design* work can begin earlier (at V1.0 testnet milestone — Phase 19) for spike-and-research purposes. V2 *code* cannot land until V1.2 ships.

**Pre-building V2 abstractions in V1 code is forbidden.** Examples of forbidden things:
- Adding LSM-shaped trait methods to the V1 `State` interface "in case V2 needs them."
- Reserving fields in V1 commitment structures "for ZK compatibility later."
- Naming things in V1 to match V2 internals.

The reason: the two layers (speculation engine, state engine) have different failure modes. Building both simultaneously means failure modes interact and become impossible to debug in isolation. V1 is allowed to make commitments that V2 will undo — that's the cost of decoupling.

The V1↔V2 boundary is enforced by the `State` trait in `krax-types`. V1 mpt code MUST NOT export anything beyond what the trait requires. V2 lsm code, when it lands, MUST be a drop-in replacement.

## V1 — Speculative execution on conventional storage

The V1 architecture is a standard rollup with a clever sequencer:
- **Sequencer**: single, with slashable bond (V3+ concern is decentralized sequencing — out of scope).
- **State**: Merkle Patricia Trie over MDBX (boring, proven; identical to go-ethereum's approach).
- **Settlement**: Ethereum L1 with **fault proof window** (proposed: 7 days, matching dominant L2s).
- **DA**: Ethereum blob space (EIP-4844). Not Celestia, not Avail, not anything else.
- **Finality**: predictive — sub-100ms preconfirmation under bond, hard finality after fault proof window expires.

### The speculative execution model

The sequencer takes a batch of pending txs and produces the same final state as a serial executor would, while running as many in parallel as possible.

**Three-tier RW-set inference**, applied in order:
1. **Static (annotated)**: contracts opt in to declared RW-sets via metadata. Zero misspeculation. We expect high-frequency contracts (DEX routers, perp exchanges, NFT marketplaces) to opt in once benefits are demonstrated.
2. **Profile-based**: rolling profile of slots accessed by recent calls to each `(contract, function selector)` pair. Approximate — misses trigger re-execution; overestimation only costs throughput.
3. **Conservative fallback**: for first-time deployments and rare functions, conservatively assume the tx reads/writes any slot in the contract. Forces serial execution. Profile fills in within a few blocks.

**Parallel dispatch and worker snapshots:**
- Conflict graph: vertices are txs, edges connect txs whose inferred sets intersect on a written slot.
- Greedy graph coloring partitions into independent groups.
- Each group goes to a worker thread (bounded by physical cores, typically 16–64).
- Each worker reads from a copy-on-write snapshot of state. Writes to a thread-local journal only. **Workers never read from each other's journals.**

**Conflict detection and commit:**
- After workers complete, compare each worker's *actual* RSet (collected during execution) against the *inferred* RSet.
- If actual RSet contains a slot another worker wrote → misspeculation. Queue for serial re-execution.
- Non-misspeculating txs commit in **mempool order, not worker completion order**. This is the determinism rule.
- Then run the queued misspeculations + originally-conflicting txs serially against merged state.

Typical workload: 85–95% of txs commit in parallel; 5–15% serial pass.

### The predictive finality model

We separate user-experience finality from cryptographic finality.

- **Soft finality (sub-100ms)**: sequencer signs preconfirmation under a bond posted to the L1 settlement contract. Equivocation between conflicting preconfirmations slashes the bond. Half to the submitter as honest-monitor reward, half burned.
- **Hard finality (V1)**: fault proof window. Any verifier can submit a fault proof; if no proof arrives, the root is final. We propose 7 days.
- **Hard finality (V2)**: asynchronous ZK proof. Proves that the posted state root is the deterministic execution of L1-posted tx data. No fault proof window needed. Targets <30 minutes.

Bond sizing: starting parameter is **10x the 99th-percentile block value** over a rolling window. Recalibrated quarterly. Slashable only by valid equivocation proofs. Recoverable by sequencer after a 30-day exit period without slashing.

## V2 — LSM state with ZK proofs

V2 replaces the MPT/MDBX backend with a **log-structured merge tree** state commitment, and the fault proof window with an **asynchronous ZK verifier**.

The motivation: MPT is calibrated to a logarithmic tree traversal with hash computations at every level, which is what the EVM gas schedule encodes (2,100 cold / 100 warm SLOAD; 22,100 / 5,000 SSTORE). LSM is calibrated to sequential write throughput, which matches actual SSD hardware. The V2 commitment is a recursive hash over memtable + SSTables, with each SSTable's hash a Merkle tree over its sorted keys. Properties relevant for ZK:
- Updates are local (memtable changes don't touch SSTable hashes).
- Reads have logarithmic proofs.
- ZK-friendly hashing is straightforward (Poseidon, Rescue) over fixed-size leaves.

V2 transition is opaque to contracts. Gas schedule unchanged at the contract level; savings passed through as a fee multiplier reduction at the protocol level. Migration happens at a designated block height; validators run both backends in parallel during a window before the switch.

V2 timing: 6–12 months after V1 mainnet GA, conditional on V1 stability.

## Security model — what's guaranteed and what isn't

### Inherited from Ethereum
All tx data on Ethereum L1 (currently as blob data via EIP-4844). All state roots subject to fault proof (V1) or ZK verification (V2). **No scenario allows a Krax block to finalize without Ethereum L1 verification.**

### Sequencer failure modes (and what catches them)
- **Posting incorrect state root** → any honest verifier replays L1 data, submits fault proof (V1) or ZK rejects (V2). Slashed bond.
- **Equivocating on preconfirmations** → any party submits both signatures to the settlement contract. Bond slashed.
- **Censoring transactions** → forced-inclusion mechanism on L1. Same as Optimism/Arbitrum.

None of these compromise user funds beyond the slashable bond, because L1 data availability ensures any honest node can reconstruct correct state and exit via L1.

### Speculation correctness — the unique-to-Krax failure mode

A class of failure unique to Krax: **incorrect conflict detection**. The execution engine commits a tx that should have been re-executed, producing a state root that disagrees with the deterministic replay.

This is detected by the same fault proof / ZK mechanism that catches any other incorrect state root. Speculation does not introduce a new trust assumption — it introduces a new code path that, like all code paths, can have bugs that the verification layer catches.

**Bias rule (most important):** In conflict detection, **always bias toward false positives** (flagging non-conflicts) over false negatives (missing real conflicts).
- False positive: re-execute when not necessary. Annoying, not dangerous.
- False negative: commit when there's a real conflict. **Catastrophic — produces wrong state.**

The conflict detector is the most security-critical component in V1. AGENTS.md and REVIEWER.md flag any false-negative-allowing change as 🔴 must-fix.

## Determinism — the architectural property, not just the code rule

Determinism in Krax is an architectural commitment, not an optimization. **If two sequencers process the same mempool, they MUST produce identical state.** This makes decentralized sequencing tractable later (V3+) without re-architecting.

Three invariants enforced by the protocol:
1. Inferred RW-sets are a deterministic function of contract state + tx data. The profile cache is part of consensus-relevant state and updates deterministically per block.
2. Conflict graph coloring uses a deterministic algorithm with a fixed seed. All nodes produce the same partition.
3. Commit order is original mempool order, NOT worker completion order. Worker scheduling is internal optimization invisible to post-commit state.

These are checked by the V1 fault proof system: a sequencer's posted state root that disagrees with deterministic replay of L1 tx data can be challenged.

## Non-goals — the things Krax explicitly does NOT do

If a proposal touches one of these, push back. These are deliberate.

- **No new VM.** EVM-compatible at the contract level, period. Solidity contracts deploy unchanged. No new opcodes, no new precompiles for V1, no gas semantic changes that affect contract correctness.
- **No new consensus.** V1 is single-sequencer with slashable bond. Decentralized sequencing is V3+.
- **No new application primitives.** No cross-rollup composability layer. Bridging is handled by existing infrastructure.
- **No alternative DA in V1.** Ethereum blob space only. Celestia/Avail are not in scope.
- **No transfer-only TPS optimization.** Krax targets workloads where serial execution clearly leaves performance on the table (perps, AMMs, NFT trades). Solana already wins on synthetic transfer benchmarks; we don't compete there.
- **No privacy primitives.** Standard rollup transparency.
- **No L1.** Anchoring to Ethereum is a feature.
- **No upgradeable proxies in V1.** The L1 bridge contracts are immutable.

## Roadmap shape (high-level)

ARCHITECTURE.md contains the phase-by-phase plan. The milestone shape is:
- **V1.0 — Credible Testnet** (Phases 0–19): speculation thesis proven publicly via measured speedup + audit + anchor app.
- **V1.1 — Mainnet Beta** (Phases 20–21): real ETH, capped TVL, anchor app on mainnet.
- **V1.2 — Mainnet GA** (Phase 22): caps removed, public launch, V2 design begins.
- **V2** (Phases 24+): LSM state + ZK proofs.

V1.0 is the "thesis proven" milestone. V1.2 is the "production launch" milestone. They are deliberately separate.

## Things this skill is NOT for

- Codebase rules (BTreeMap not HashMap, doc comments, file caps, etc.) → `krax-conventions` skill.
- Library API specifics (revm Context::mainnet, reth crate names, alloy types) → `krax-rust-engine` skill.
- Implementation mechanics (how to write the conflict detector code, what the worker pool looks like in Rust) → `krax-rust-engine` skill, with rule context from `krax-conventions`.

If the question is "should we do X" or "how should X work conceptually" → here. If it's "is X allowed in the codebase" → conventions. If it's "how do I write X in Rust" → rust-engine.

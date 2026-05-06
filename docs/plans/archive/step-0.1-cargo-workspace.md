# Step 0.1 — Cargo Workspace Initialization

> **Plan status:** Ready for execution.
> **Phase:** 0 — Project Setup.
> **ARCHITECTURE.md reference:** Phase 0, Step 0.1.
> **Prerequisites:** Pre-Phase-0 reth-as-library POC completed (per AGENTS.md "Current State").

---

## Purpose

Initialize the Cargo workspace at the project root. This step creates **only two files**: the workspace `Cargo.toml` and `rust-toolchain.toml`. No directories under `bin/` or `crates/` yet — those land in Step 0.2.

After this step, `cargo metadata --no-deps` will fail on missing member paths. **That is correct and expected.** It proves the workspace TOML parses; the members simply don't exist on disk yet.

---

## Decisions resolved before this plan was written

These were open questions in the previous draft of this plan. Resolutions are now baked in.

1. **Edition: `2024`.** Edition 2021 was the conservative initial pick; we are deliberately using 2024 instead. Justification: Krax is greenfield (no migration cost), is heavily async-first (the sequencer, RPC, mempool, batcher all lean hard on `tokio` — and 2024 properly stabilized `async fn in traits` which we need everywhere), our minimum Rust is already 1.85+ (which is the version that stabilized edition 2024), and starting on 2021 means a forced multi-crate migration in 6 months when 2024-only patterns become awkward to avoid. One-line cost now vs. workspace-wide refactor later.
2. **Resolver: `"3"`.** Edition 2024 workspaces default to resolver 3. We set it explicitly for clarity; better feature unification across the workspace, fewer mystery feature-enabled-here-but-not-there debugging sessions.
3. **AGENTS.md cleanup is part of this commit.** AGENTS.md currently references revm "around v38" (from when it was last edited). Current revm git tag is v55. The coder updates AGENTS.md `Current State` to reference whatever `cargo search revm` returns, in the same commit as this Step 0.1 work. Rationale: prevents the "we forgot to update that" failure mode three weeks from now.

---

## Library verification checklist

The previous plan ran Context7 queries on the high-priority crates. Hit the query limit before reaching medium-priority ones. Findings carried forward below; medium-priority verification is the coder's job before any FIXME is replaced.

### High priority — Context7-verified

#### `revm`

- **Context7 ID:** `/bluealloy/revm`
- **Source reputation:** Medium · Benchmark 79.15 · 108 snippets
- **Findings:** revm is a workspace of sub-crates (`revm-primitives`, `revm-interpreter`, `revm-context`, `revm-handler`, `revm-database`, `revm-precompile`, `revm-inspector`); the top-level `revm` crate re-exports them. Entry point in consuming code is `Context::mainnet()` builder, confirmed in reth's own examples.
- **🔴 DISCREPANCY:** AGENTS.md references "around v38" (the revm git workspace tag at the time AGENTS.md was last edited). Context7 reports the **current git tag is v55** — revm has shipped ~17 major tag bumps since AGENTS.md was last touched. The exact crates.io published version of the `revm` crate (which is **not the same** as the git workspace tag) was not retrievable via Context7.
- **Coder action:**
  1. Run `cargo search revm` to get the published crates.io version.
  2. Replace the FIXME in `Cargo.toml`.
  3. Update AGENTS.md `Current State` section in the same commit — replace the "around v38" reference with the verified version string.

#### `reth-*` family

- **Context7 ID:** `/paradigmxyz/reth`
- **Source reputation:** High · Benchmark 69.65 · 1445 snippets
- **Findings:**
  - Recommended SDK consumption pattern is a **git dependency**: `reth-ethereum = { git = "https://github.com/paradigmxyz/reth" }`. Context7 shows no crates.io version string in the "Add Reth to Project" example.
  - Confirmed crates from the reth repo layout doc: `reth-db` ✅, `reth-evm` ✅ (listed as "evm"), `reth-execution-types` ✅ (listed as "execution-types").
  - `reth-ethereum` is confirmed as a meta-crate exposing namespaced re-exports like `reth_ethereum::evm::...`, `reth_ethereum::node::...`.
  - `reth-ethereum-primitives` as a **standalone** published crate: **not explicitly confirmed** by Context7. The reth repo has "primitives" and "primitives-traits" in its layout but Context7 did not confirm whether these publish separately or only via the `reth-ethereum` umbrella.
- **🔴 DISCREPANCY:** AGENTS.md correctly notes the Reth 2.0 rename (use `reth-ethereum-primitives`, not `reth-primitives`), but whether `reth-ethereum-primitives` is a standalone crate vs. a re-export under `reth-ethereum`'s namespace is unconfirmed.
- **Coder actions:**
  1. Run `cargo search reth-ethereum`. If a `2.x` version is published on crates.io, switch to `version = "2"` for **all** reth-* entries (cleaner, no git rev management). If not on crates.io or only an old version is published, use the git+rev approach.
  2. If using git: get the pinned rev with `git ls-remote https://github.com/paradigmxyz/reth HEAD`. **Use the same rev for every reth-* entry** to ensure they're a coherent snapshot.
  3. Run `cargo search reth-ethereum-primitives`. If it does not exist as a standalone crate, **remove that entry** from `[workspace.dependencies]` and add a note in the commit message that primitives are accessed via `reth-ethereum`'s re-exports. Update AGENTS.md Rule 10's approved-deps list to match.
  4. Run `cargo search reth-db`, `cargo search reth-evm`, `cargo search reth-execution-types` to confirm if going the crates.io route.
  5. Document the git-vs-crates.io decision in the commit message.

#### `alloy-*` family

- **Context7 IDs:** `/alloy-rs/alloy` (High · 73), `/alloy-rs/core` (High · 65)
- **Findings:** `version = "1"` confirmed for the alloy meta-crate. Individual crates publish separately:
  - `alloy-primitives` (from `alloy-rs/core`) → 1.x
  - `alloy-rpc-types` (from `alloy-rs/alloy`) → 1.x
  - `alloy-sol-types` (from `alloy-rs/core`) → 1.x
- **No discrepancy** with AGENTS.md.
- **Coder action:** Pinning to `"1"` is semantically correct (allows 1.x patch updates). Optionally tighten with `cargo search alloy-primitives` and pin `"1.x.y"` for maximum reproducibility without relying solely on `Cargo.lock`. Either is acceptable.

### Medium priority — Context7 limit reached, training-data estimates, **must verify**

| Crate | Training estimate | Action |
|---|---|---|
| `jsonrpsee` | ~0.24.x | `cargo search jsonrpsee` before pinning |
| `metrics` | ~0.23.x | `cargo search metrics` before pinning |
| `metrics-exporter-prometheus` | ~0.15.x | `cargo search metrics-exporter-prometheus` before pinning |
| `clap` | 4.x (stable) | `cargo search clap` to confirm minor |

### Low priority — stable crates, training-data versions, verify only on unexpected behavior

| Crate | Version | Notes |
|---|---|---|
| `tokio` | `"1"` | Very stable |
| `tokio-util` | `"0.7"` | Stable |
| `thiserror` | `"2"` | Moved to 2.x in late 2024 |
| `anyhow` | `"1"` | Stable |
| `serde` | `"1"` | Stable |
| `serde_json` | `"1"` | Stable |
| `tracing` | `"0.1"` | Stable |
| `tracing-subscriber` | `"0.3"` | Stable |
| `parking_lot` | `"0.12"` | Stable |
| `rayon` | `"1"` | Stable |
| `crossbeam` | `"0.8"` | Stable |
| `dashmap` | `"6"` | Estimate — `cargo search dashmap` to confirm |
| `proptest` | `"1"` | Stable |
| `rstest` | `"0.22"` | Estimate — `cargo search rstest` to confirm |
| `pretty_assertions` | `"1"` | Stable |

---

## Files to create

### File 1: `/Cargo.toml`

Workspace root. Declares all members, shared metadata, and every dependency version. **All FIXME values must be resolved before the commit lands.**

```toml
[workspace]
resolver = "3"
members = [
    # Binaries
    "bin/kraxd",
    "bin/kraxctl",
    "bin/kraxprover",
    # Library crates
    "crates/krax-types",
    "crates/krax-config",
    "crates/krax-mempool",
    "crates/krax-rwset",
    "crates/krax-sequencer",
    "crates/krax-state",
    "crates/krax-execution",
    "crates/krax-batcher",
    "crates/krax-prover",
    "crates/krax-rpc",
    "crates/krax-metrics",
]

[workspace.package]
version    = "0.1.0"
edition    = "2024"
license    = "MIT"
repository = "FIXME_REPO_URL"     # see Open Question #1
authors    = ["Krax Contributors"]

# ---------------------------------------------------------------------------
# [workspace.dependencies]
#
# All external dependency versions live here. Per-crate Cargo.toml files
# reference these via `dep-name = { workspace = true }` or
# `dep-name.workspace = true`. Crates do NOT specify versions themselves.
#
# VERSION STATUS KEY:
#   ✅ CONFIRMED  — verified via Context7, May 2026
#   ⚠️ ESTIMATED — stable crate, training-data version, low risk; verify
#   🔴 FIXME     — must be resolved before commit; see plan checklist
# ---------------------------------------------------------------------------

[workspace.dependencies]

# --- EVM interpreter ---
# ✅ Context7 (/bluealloy/revm, git tag v55, May 2026)
# 🔴 FIXME: crates.io version not surfaced by Context7. Run `cargo search revm`.
#    AGENTS.md referenced "around v38" (old git tag) — current tag is v55.
#    Update AGENTS.md "Current State" with the verified version in this commit.
revm = { version = "FIXME", default-features = false, features = ["std", "serde"] }

# --- Reth execution layer ---
# ✅ Context7 (/paradigmxyz/reth, Reth 2.0, May 2026) — git dep pattern confirmed
# 🔴 FIXME: Two-part decision required, see plan "Coder actions" for reth-*:
#    1. Decide git vs. crates.io after `cargo search reth-ethereum`.
#       - If crates.io has a 2.x: use `version = "2"` and remove `git`/`rev` fields.
#       - Otherwise: replace FIXME_PINNED_REV with HEAD of main from
#         `git ls-remote https://github.com/paradigmxyz/reth HEAD`.
#         Use the SAME rev for every reth-* entry below.
#    2. Confirm `reth-ethereum-primitives` is a standalone published crate via
#       `cargo search`. If it is NOT, remove that entry and access primitives
#       through `reth-ethereum`'s re-exports. Update AGENTS.md Rule 10
#       approved-deps list to match.
reth-ethereum            = { git = "https://github.com/paradigmxyz/reth", rev = "FIXME_PINNED_REV", default-features = false }
reth-db                  = { git = "https://github.com/paradigmxyz/reth", rev = "FIXME_PINNED_REV", default-features = false }
reth-evm                 = { git = "https://github.com/paradigmxyz/reth", rev = "FIXME_PINNED_REV", default-features = false }
reth-execution-types     = { git = "https://github.com/paradigmxyz/reth", rev = "FIXME_PINNED_REV", default-features = false }
reth-ethereum-primitives = { git = "https://github.com/paradigmxyz/reth", rev = "FIXME_PINNED_REV", default-features = false }

# --- Ethereum types ---
# ✅ Context7 (/alloy-rs/alloy + /alloy-rs/core, May 2026) — version "1" confirmed
alloy-primitives = { version = "1", default-features = false, features = ["serde"] }
alloy-rpc-types  = { version = "1", default-features = false }
alloy-sol-types  = { version = "1", default-features = false }

# --- JSON-RPC server ---
# ⚠️ ESTIMATED: training data ~0.24.x. Context7 not queried (limit).
# 🔴 FIXME: `cargo search jsonrpsee`.
jsonrpsee = { version = "FIXME", features = ["server"] }

# --- Metrics ---
# ⚠️ ESTIMATED: training data ~metrics 0.23.x, exporter-prometheus 0.15.x.
# 🔴 FIXME: `cargo search metrics` and `cargo search metrics-exporter-prometheus`.
metrics                     = "FIXME"
metrics-exporter-prometheus = "FIXME"

# --- CLI ---
# ⚠️ ESTIMATED: clap 4.x stable. Coder: confirm minor with `cargo search clap`.
clap = { version = "4", features = ["derive"] }

# --- Async runtime ---
# ⚠️ ESTIMATED: stable.
tokio      = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt", "time"] }

# --- Error handling ---
# ⚠️ ESTIMATED: thiserror moved to 2.x late 2024.
thiserror = "2"
anyhow    = "1"

# --- Serialization ---
serde      = { version = "1", features = ["derive"] }
serde_json = "1"

# --- Logging ---
tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }

# --- Concurrency ---
parking_lot = "0.12"
rayon       = "1"
crossbeam   = "0.8"

# --- Concurrent map (only where a sharded Mutex won't do — see AGENTS.md Rule 10) ---
# ⚠️ ESTIMATED ~6.x. Coder: `cargo search dashmap`.
dashmap = "6"

# --- Test-only ---
proptest          = "1"
rstest            = "0.22"   # ⚠️ verify with `cargo search rstest`
pretty_assertions = "1"
```

### File 2: `/rust-toolchain.toml`

```toml
[toolchain]
# 🔴 FIXME: Replace X.YY.Z with the exact current stable version.
#   Run: rustup update stable && rustup show active-toolchain
#   Pin the exact output (e.g. "1.87.0" or "1.88.0") — NOT "stable".
#   Pinning prevents surprise breakage when stable advances mid-project.
#   AGENTS.md mandates minimum 1.85; pin to current stable at impl time.
#   Note: edition 2024 (set in Cargo.toml) requires Rust 1.85+; any current
#   stable satisfies this.
channel    = "FIXME_STABLE_VERSION"
components = ["rustfmt", "clippy", "rust-src"]
profile    = "minimal"
```

---

## Verification steps

Step 0.1 cannot pass `cargo build` cleanly — per-crate `Cargo.toml` files don't exist yet (that's Step 0.2). The commands below are the correct gate **for Step 0.1 alone**.

Run in order:

```bash
# 1. Confirm the toolchain pin is active.
rustup show active-toolchain
# Expected: e.g. "1.87.0-aarch64-apple-darwin (overridden by '/.../krax/rust-toolchain.toml')"
# Failure mode: output says "stable" without a version → channel field still has FIXME.

# 2. Confirm rustfmt and clippy are installed for the pinned toolchain.
rustup component list --installed --toolchain <PINNED_VERSION>
# Expected: rustfmt, clippy, rust-src each appear in the output.

# 3. Confirm cargo can parse the workspace TOML.
cargo metadata --no-deps 2>&1 | head -30
# Expected: cargo errors on missing member paths (bin/kraxd, etc.) — this is CORRECT
# and EXPECTED at Step 0.1. It proves the [workspace] table parses cleanly.
# A clean parse error about missing paths = success.
# A TOML syntax error or [workspace] error = failure.

# 4. Confirm both files exist.
ls -la Cargo.toml rust-toolchain.toml

# 5. Confirm zero FIXME values remain.
grep -n "FIXME" Cargo.toml rust-toolchain.toml
# Expected: empty output. Any FIXME remaining is a blocker.

# 6. Confirm AGENTS.md no longer references "around v38" for revm.
grep -n "v38" AGENTS.md
# Expected: empty. The Current State section was updated in this commit.
```

### Definition of "Step 0.1 done"

- ✅ `Cargo.toml` exists at project root with `[workspace]`, `[workspace.package]`, `[workspace.dependencies]` populated, **no FIXME values**.
- ✅ `rust-toolchain.toml` exists with a specific pinned stable version (e.g. `"1.87.0"`) — not `"stable"`, not `"FIXME"`.
- ✅ `rustup show active-toolchain` returns the pinned version from inside the project directory.
- ✅ `cargo metadata --no-deps` errors on missing member paths (expected) but does NOT error on TOML syntax.
- ✅ `grep -rn "FIXME" Cargo.toml rust-toolchain.toml` returns empty.
- ✅ AGENTS.md `Current State` section updated to reflect the verified revm crates.io version (no more "v38").
- ✅ `[workspace.package]` `repository` field set to a real URL, not the FIXME placeholder.

---

## Open questions / coder follow-ups

### Must resolve before committing

1. **Repository URL.** `[workspace.package]` `repository` is a FIXME. Set to the actual GitHub URL (or wherever the project will live). If undecided, leave a placeholder like `https://github.com/krax/krax` and flag in the commit message.
2. **`revm` crate version.** `cargo search revm`. Update Cargo.toml AND AGENTS.md "Current State" in the same commit.
3. **`reth-*` strategy.** `cargo search reth-ethereum`. Decide git+rev vs. crates.io `version = "2"`. Apply the SAME strategy to every reth-* entry. Document the choice in the commit message.
4. **`reth-ethereum-primitives` existence.** `cargo search reth-ethereum-primitives`. If standalone: keep the entry. If not: remove the entry and update AGENTS.md Rule 10's approved-deps list.
5. **`jsonrpsee` version.** `cargo search jsonrpsee`.
6. **`metrics` + `metrics-exporter-prometheus` versions.** `cargo search` both.
7. **Pinned stable Rust version.** `rustup update stable && rustup show active-toolchain` — use the exact output (e.g. `1.87.0`).

### Resolved before this plan was written (no action needed)

- ~~Edition 2021 vs 2024~~ → **2024**
- ~~Resolver 2 vs 3~~ → **3**
- ~~AGENTS.md cleanup as separate work or same commit~~ → **same commit**

---

## What this step does NOT do

- Does not create any directories under `bin/` or `crates/`. Step 0.2.
- Does not create per-crate `Cargo.toml` files. Step 0.2.
- Does not write any Rust source code. Step 0.3.
- Does not create `Makefile`, `.gitignore`, `.env.example`, `docker-compose.yml`, `rustfmt.toml`, or `clippy.toml`. Steps 0.4–0.8.
- Does not initialize the `contracts/` Foundry project. Step 0.7.
- Does not configure `[profile.release]` or `[profile.dev]`. Deferred until real code exists to optimize.
- Does not produce a `Cargo.lock`. That appears only after `cargo build` succeeds, which requires Step 0.2.
- Does not make `make build`, `cargo build`, or `cargo test` succeed. Those need Step 0.2's per-crate Cargo.toml files.
- Does not pass the Phase 0 Gate. That gate requires Steps 0.1–0.9.

**Expected partial-state behavior:** After Step 0.1 only, `cargo metadata --no-deps` will fail with "failed to read Cargo.toml" errors for each missing member path. This is correct and expected. The workspace `Cargo.toml` is structurally valid; the workspace members simply don't exist on disk yet.

---

## Commit suggestion

Conventional commit format per AGENTS.md "Workflow & Conventions":

```
chore(workspace): initialize Cargo workspace (Step 0.1)

- Create root Cargo.toml with [workspace], [workspace.package], and
  [workspace.dependencies] tables.
- Create rust-toolchain.toml pinned to <PINNED_VERSION>.
- Edition 2024, resolver 3.
- Update AGENTS.md "Current State" to reflect verified revm version
  <VERIFIED_VERSION> (was stale "around v38").
- reth-* dependencies: <git+rev | crates.io 2.x> — see commit body for rationale.
- reth-ethereum-primitives: <kept as standalone | removed, accessed via reth-ethereum>.

Implements ARCHITECTURE.md Phase 0 Step 0.1.
```

The coder fills in the placeholders based on the `cargo search` outcomes.

---

## After this step

Next: Step 0.2 — Directory Structure. Create the full `bin/*` and `crates/*` tree with empty `Cargo.toml` files referencing this workspace's `[workspace.dependencies]`.

Once Step 0.1 is committed and verified, this plan file moves to `docs/plans/archive/step-0.1-cargo-workspace.md` as part of the archive convention.

---

## Outcomes

- **`revm` version:** `38.0.0` on crates.io. The git workspace tag is `v55` — a separate numbering scheme independent of the published crate version. AGENTS.md "around v38" reference updated.
- **`reth-*` strategy: git dependency.** No real crates.io release exists for any `reth-*` crate (all `0.0.0` placeholders). All four reth entries (`reth-ethereum`, `reth-db`, `reth-evm`, `reth-execution-types`) use the same git rev `02d1776786abc61721ae8876898ad19a702e0070` (HEAD of main, 2026-05-06).
- **`reth-ethereum-primitives` removed.** `cargo search` confirmed it is only a `0.0.0` crates.io placeholder — not a standalone published crate. Removed from `[workspace.dependencies]`. Primitives accessed via `reth-ethereum`'s re-exports. AGENTS.md Rule 10 approved-deps list updated to match.
- **Other resolved versions:** `jsonrpsee = "0.26"`, `metrics = "0.24"`, `metrics-exporter-prometheus = "0.18"`, `dashmap = "6"` (7.0.0-rc2 is pre-release), `rstest = "0.26"`, Rust toolchain `"1.95.0"`.

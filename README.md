# Krax

Krax is a Layer 2 rollup for Ethereum that applies speculative parallel execution to the EVM:
transactions are executed concurrently against consistent state snapshots, conflicts are detected and
re-executed deterministically, and committed blocks are settled on Ethereum L1. The result is full
Solidity compatibility with significantly lower gas costs, delivered in two phases: V1 introduces
the speculative execution engine; V2 replaces the state layer with an LSM-tree commitment scheme.

---

## Status

Krax is in early development. The codebase is project scaffolding only; no functional sequencer,
RPC, or bridge exists yet.

---

## Prerequisites

- **Rust** — `rust-toolchain.toml` pins `1.95.0`; `rustup` installs it automatically when invoked
  from the project directory.
- **Foundry** — required for the `contracts/` subproject. Install via the
  [Foundry Book](https://getfoundry.sh).
- **Docker / Docker Compose** — optional; `docker-compose.yml` is currently a placeholder with no
  active services.
- After cloning, run `git submodule update --init` to populate `contracts/lib/forge-std`.

---

## Build / Run / Test

### Build

```bash
make build
```

Produces release binaries in `target/release/`.

### Run

```bash
make run
```

Starts `kraxd` (prints version banner and exits; no services active at Phase 0).

### Test

```bash
make test
```

Runs the test suite. Zero tests exist at Phase 0; the command completes cleanly.

---

## Roadmap

- **V1.0 — Credible Testnet.** Speculative execution proven on shadow-forked mainnet traffic.
  Audited. One anchor application live.
- **V1.1 — Mainnet Beta.** Mainnet deployment with capped TVL.
- **V1.2 — Mainnet GA.** Caps removed; V2 design begins.
- **V2 — LSM State + ZK Proofs.** Log-structured state commitment with asynchronous validity
  proofs.

---

## License

Krax is licensed under the Apache License, Version 2.0. See LICENSE for details.

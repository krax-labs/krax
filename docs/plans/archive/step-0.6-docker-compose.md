# Step 0.6 — Docker Compose (Auxiliary Services Placeholder)

> **Plan status:** Ready for execution.
> **Phase:** 0 — Project Setup.
> **ARCHITECTURE.md reference:** Phase 0, Step 0.6.
> **Prerequisites:** Step 0.5 (`.gitignore` & `.env.example`) complete and committed. `make lint` passes. Docker (Docker Desktop or OrbStack) installed and `docker compose` reachable on `PATH`.

---

## Purpose

Three deliverables:

1. **`docker-compose.yml` (create at project root)** — a placeholder Compose file with no active services and a header comment explaining the file's purpose. `docker compose up` will succeed with "no services to start." Services land here in the phases that introduce them.

2. **`scripts/devnet-up.sh` (create)** — a no-op placeholder shell script that will eventually start auxiliary services. Currently echoes its placeholder status and exits 0. The `scripts/` directory does not exist yet; placing this file creates it.

3. **`scripts/devnet-down.sh` (create)** — same shape as `devnet-up.sh`, for stopping services.

**`kraxd` is NOT containerized.** It runs natively via `make run` for fast iteration and easier debugging. This is a permanent architectural decision, not a Phase 0 shortcut.

**Anvil for Phase 0:** developers run `anvil` natively in a terminal tab. No Docker required. Anvil moves into `docker-compose.yml` at the phase that first depends on it being co-managed with another service (likely Phase 11 or 12).

These three files exist now so their paths are stable. The Compose file parses cleanly. The scripts are executable and exit 0. Neither does any real work until a service is added.

---

## Decisions resolved before this plan was written

1. **`docker-compose.yml` content shape: option (a) — minimal.** `services: {}` plus a header comment. No commented-out service stanzas. Stub service configs written now would be stale or incomplete by the phases that actually need them (Blockscout config is non-trivial; Prometheus needs scrape targets that don't exist yet). The header comment carries the roadmap message without misleading breadcrumbs.

2. **No `version:` field in docker-compose.yml.** Modern Docker Compose (v2+ plugin) treats `version:` as deprecated and emits a warning. Greenfield 2026 project targeting the Docker Compose plugin only. The `version:` field is omitted entirely.

3. **Script shebang: `#!/usr/bin/env bash`.** Portable — locates bash wherever it lives. No hardcoded `/bin/bash` path.

4. **`set -euo pipefail` in both scripts.** One line, future-proof. A future maintainer adding real service orchestration will need it; it costs nothing to have it from day one.

5. **Script body: `echo` + `exit 0`.** Silent success on a placeholder is surprising. An echo line confirms the script ran and explains its placeholder state. Both scripts print their placeholder status before exiting.

6. **`chmod +x` is required.** The scripts must be executable on the filesystem. The plan's file-creation instructions call it out explicitly. Verification checks `test -x` to catch a missed `chmod`.

7. **`scripts/` directory created implicitly.** No `.gitkeep` — the directory will contain real files from day one. Git tracks the directory through its contents.

8. **Docker as a hard prerequisite.** `docker compose config` is a verification step. If Docker is not installed, that verification fails, which is the correct signal. Docker Desktop or OrbStack is expected on macOS for any contributor working with the Compose file.

9. **No Anvil installation check in Step 0.6 verification.** The Phase 0 Gate item about Anvil is satisfied by Foundry being installed. Step 0.7 (`forge init`) will fail without Foundry, which includes Anvil. An `anvil --version` check in Step 0.6 would verify a tool that Step 0.7 covers anyway.

10. **YAML hygiene for `docker-compose.yml`:** 2-space indentation (Docker convention), LF line endings, trailing newline. Same conventions as `.env.example`.

---

## Library verification checklist

No external libraries used in this step. `docker-compose.yml` is system tooling (Docker Compose). The shell scripts use only bash built-ins and `echo`. No Rust code is written or modified.

No Context7 lookups required.

---

## Files to create

### File 1 (create): `docker-compose.yml`

Create at the project root. 2-space indentation, LF line endings, trailing newline.

**Exact content:**

```yaml
# docker-compose.yml — Auxiliary services for the Krax devnet.
#
# kraxd itself is NOT containerized. It runs natively via `make run` for
# fast iteration and easier debugging.
#
# This file is a placeholder in Phase 0. Services land here in the phases
# that introduce them:
#   - Anvil (L1 simulator): Phase 11 or 12, when it needs co-management
#     with another service. Until then, run Anvil natively in a terminal.
#   - Blockscout (block explorer): Phase 11+
#   - Prometheus + Grafana (metrics): Phase 16+
#
# Usage (once services exist):
#   Start:  ./scripts/devnet-up.sh   (or: docker compose up -d)
#   Stop:   ./scripts/devnet-down.sh (or: docker compose down)

services: {}
```

---

### File 2 (create): `scripts/devnet-up.sh`

Create in the `scripts/` directory. The `scripts/` directory does not exist — placing this file creates it. **After writing, run `chmod +x scripts/devnet-up.sh`.** LF line endings, trailing newline.

**Exact content:**

```bash
#!/usr/bin/env bash
# scripts/devnet-up.sh — Start Krax auxiliary devnet services.
#
# Phase 0 placeholder: this script does nothing yet.
# Auxiliary services (Blockscout, Prometheus, Grafana, and eventually Anvil)
# land in docker-compose.yml as each phase introduces them. When a service
# is added there, this script gains the corresponding `docker compose up` call.
#
# For now, Anvil runs natively: open a terminal tab and run `anvil`.
# See docker-compose.yml for the full services roadmap.
set -euo pipefail

echo "devnet-up: no services configured yet (Phase 0 placeholder)"
echo "Run Anvil natively: anvil"
exit 0
```

---

### File 3 (create): `scripts/devnet-down.sh`

Create in the `scripts/` directory alongside `devnet-up.sh`. **After writing, run `chmod +x scripts/devnet-down.sh`.** LF line endings, trailing newline.

**Exact content:**

```bash
#!/usr/bin/env bash
# scripts/devnet-down.sh — Stop Krax auxiliary devnet services.
#
# Phase 0 placeholder: this script does nothing yet.
# Auxiliary services (Blockscout, Prometheus, Grafana, and eventually Anvil)
# land in docker-compose.yml as each phase introduces them. When a service
# is added there, this script gains the corresponding `docker compose down` call.
#
# See docker-compose.yml for the full services roadmap.
set -euo pipefail

echo "devnet-down: no services configured yet (Phase 0 placeholder)"
exit 0
```

---

## Verification steps

Run in order from the project root. Every command must pass before the step is considered done.

```bash
# 1. Confirm docker-compose.yml exists at the project root.
test -f docker-compose.yml && echo "OK: docker-compose.yml exists"
# Expected: "OK: docker-compose.yml exists"

# 2. Confirm docker-compose.yml parses cleanly and exits 0.
docker compose -f docker-compose.yml config
echo "Exit code: $?"
# Expected: exit 0. Docker Compose prints the normalized config.
# The output should show an empty or absent services section — not an error.

# 2a. Confirm zero services are defined.
docker compose -f docker-compose.yml config --services
echo "Exit code: $?"
# Expected: empty output, exit 0. Any service name printed here means
# the placeholder has a malformed or unintended service entry.

# 3. Confirm devnet-up.sh exists and is executable.
test -x scripts/devnet-up.sh && echo "OK: devnet-up.sh is executable" || echo "FAIL: not executable — run chmod +x scripts/devnet-up.sh"
# Expected: "OK: devnet-up.sh is executable"

# 4. Confirm devnet-down.sh exists and is executable.
test -x scripts/devnet-down.sh && echo "OK: devnet-down.sh is executable" || echo "FAIL: not executable — run chmod +x scripts/devnet-down.sh"
# Expected: "OK: devnet-down.sh is executable"

# 5. Run devnet-up.sh and confirm it exits 0 with expected output.
./scripts/devnet-up.sh
echo "Exit code: $?"
# Expected output (two lines, in order):
#   devnet-up: no services configured yet (Phase 0 placeholder)
#   Run Anvil natively: anvil
# Expected exit code: 0.

# 6. Run devnet-down.sh and confirm it exits 0 with expected output.
./scripts/devnet-down.sh
echo "Exit code: $?"
# Expected output (one line):
#   devnet-down: no services configured yet (Phase 0 placeholder)
# Expected exit code: 0.

# 7. Confirm make lint still passes (no regressions).
make lint
echo "Exit code: $?"
# Expected: exit 0. Shell scripts and YAML do not affect cargo clippy.
# This confirms the workspace is still clean after the new files landed.
```

---

## Definition of "Step 0.6 done"

- ✅ `docker-compose.yml` exists at the project root.
- ✅ `docker compose -f docker-compose.yml config` exits 0.
- ✅ `docker compose -f docker-compose.yml config --services` produces empty output.
- ✅ Header comment is present and contains the Phase numbers and the kraxd-not-containerized note.
- ✅ `scripts/devnet-up.sh` exists and `test -x scripts/devnet-up.sh` passes.
- ✅ `scripts/devnet-down.sh` exists and `test -x scripts/devnet-down.sh` passes.
- ✅ `./scripts/devnet-up.sh` exits 0 and prints the placeholder message.
- ✅ `./scripts/devnet-down.sh` exits 0 and prints the placeholder message.
- ✅ `make lint` exits 0 (no regressions).
- ✅ ARCHITECTURE.md Step 0.6 is checked off (all 3 items).
- ✅ AGENTS.md `Current State` and `Changelog` are updated.

---

## Open questions / coder follow-ups

None. All decisions are fully resolved and all file content is exactly specified above.

If `docker compose -f docker-compose.yml config` exits non-zero, the most common cause is a YAML formatting error (wrong indentation or wrong line endings in `docker-compose.yml`). Verify 2-space indentation and LF line endings before investigating further.

If `test -x` fails after the file is written, the file was written without the executable bit. Run `chmod +x scripts/devnet-up.sh scripts/devnet-down.sh` and re-run verification steps 3–6.

---

## What this step does NOT do

- ❌ No Docker service definitions of any kind. No `image:`, `ports:`, `volumes:`, or `environment:` blocks. Those land in the phases that need them.
- ❌ Anvil is not added to `docker-compose.yml`. Anvil runs natively until Phase 11 or 12.
- ❌ No `make devnet-up` or `make devnet-down` Makefile targets. The scripts are invoked directly; a Makefile wrapper is not needed in Phase 0.
- ❌ No changes to `bin/*/src/`, `crates/*/src/`, or any `Cargo.toml`. No Rust source touched.
- ❌ No `scripts/fund-test-account.sh`. That file appears in AGENTS.md's Project Structure diagram but is not part of Step 0.6's scope. It lands with the phase that needs it.
- ❌ No Anvil installation or Foundry setup. That is Step 0.7.
- ❌ No `.gitignore` entry for `scripts/`. Shell scripts are committed, not ignored.
- ❌ `rustfmt.toml` / `clippy.toml` (Step 0.8), `README.md` (Step 0.9).

---

## Updates to other files in the same commit

### `ARCHITECTURE.md`

Mark Step 0.6 complete. Change:

```markdown
### Step 0.6 — Docker Compose (Auxiliary Services Placeholder)
- [ ] Create a placeholder `docker-compose.yml` at the project root with no active services and a header comment explaining the file's purpose: auxiliary services (anvil, Blockscout, Prometheus, Grafana) land here in the phases that introduce them. **kraxd itself is NOT containerized** — it runs natively via `make run` for fast iteration and easier debugging.
- [ ] Create `scripts/devnet-up.sh` and `scripts/devnet-down.sh` as placeholder scripts (also no-op for now, with comments explaining they will start auxiliary services in later phases). They exist now so paths are stable; they do nothing until a service is added.
- [ ] **Anvil for Phase 0:** developers run anvil natively via `anvil` in a terminal tab. No Docker required. Anvil moves into `docker-compose.yml` at the phase that first depends on it being co-managed with another service (likely Phase 11 or 12).
```

to:

```markdown
### Step 0.6 — Docker Compose (Auxiliary Services Placeholder) ✅
- [x] Create a placeholder `docker-compose.yml` at the project root with no active services and a header comment explaining the file's purpose: auxiliary services (anvil, Blockscout, Prometheus, Grafana) land here in the phases that introduce them. **kraxd itself is NOT containerized** — it runs natively via `make run` for fast iteration and easier debugging.
- [x] Create `scripts/devnet-up.sh` and `scripts/devnet-down.sh` as placeholder scripts (also no-op for now, with comments explaining they will start auxiliary services in later phases). They exist now so paths are stable; they do nothing until a service is added.
- [x] **Anvil for Phase 0:** developers run anvil natively via `anvil` in a terminal tab. No Docker required. Anvil moves into `docker-compose.yml` at the phase that first depends on it being co-managed with another service (likely Phase 11 or 12).
```

### `AGENTS.md`

Replace `Current State` with:

```markdown
**Current Phase:** Phase 0 — Project Setup (Steps 0.1–0.6 complete, Step 0.7 next)

**What was just completed:**
- **Step 0.6 — Docker Compose placeholder done.** `docker-compose.yml` created at project root: no active services (`services: {}`), header comment documents purpose (auxiliary services only), Phase numbers for when each service lands (Anvil Phase 11/12, Blockscout Phase 11+, Prometheus + Grafana Phase 16+), and confirms kraxd is NOT containerized. `scripts/devnet-up.sh` and `scripts/devnet-down.sh` created as no-op placeholder scripts: `#!/usr/bin/env bash`, `set -euo pipefail`, echo placeholder message, `exit 0`. Both are executable (`chmod +x`). `scripts/` directory created implicitly by placing the files.
- (Carry forward: Step 0.5 — `.gitignore` audited and augmented; `.env.example` created with four `KRAX_*` variables.)
- (Carry forward: Step 0.4 — Makefile with seven targets; `make build/test/lint/run/fmt/clean` all pass.)
- (Carry forward: Step 0.3 — `cargo run --bin kraxd` → `krax v0.1.0`; `cargo run --bin kraxctl -- --help` → help text.)
- (Carry forward: Step 0.2 — full `bin/*` and `crates/*` tree, 14 workspace members, `cargo build --workspace` succeeds.)
- (Carry forward: Step 0.1 — revm 38, reth-* git rev `02d1776786abc61721ae8876898ad19a702e0070`, jsonrpsee 0.26, etc. See archived plan for full version table.)

**What to do next (in order):**
1. 🔴 **Step 0.7 — Foundry init for contracts.** `forge init contracts/ --no-git`, configure `foundry.toml` for solc 0.8.24, add `contracts/.gitignore`.
2. Step 0.8 — Lint & format configuration (`rustfmt.toml`, `clippy.toml`).
3. Step 0.9 — README.

**Blockers:**
- Repository URL is a placeholder (`https://github.com/krax-labs/krax`). Replace before V1.0 branding. Not a blocker for Phase 0 work.
- Project name not finalized. "Krax" is a working name. Search-replace before mainnet branding (V1.1 concern).

**Notes:**
- `kraxd` version banner uses `println!` — documented Rule 4 exception with inline comment in `main.rs`. All future runtime output uses `tracing`.
- `tracing-subscriber` initialization is deferred to a later step alongside `krax-config`.
- The `Commands` enum in `kraxctl` is empty until a step adds a real subcommand. Clippy does NOT warn on the `if cli.command.is_none()` branch in practice (verified at Step 0.4). The warning note from Step 0.3 is withdrawn; no `#[allow(...)]` needed.
- The `integration` feature on every crate is intentionally empty. Integration tests land in Phase 1+.
- `.env.example` documents the four kraxd env vars but nothing reads them yet. Config loading (`krax-config`) arrives in Phase 1+.
- `docker-compose.yml` is a placeholder. No services are defined. Do not add Anvil to Compose until the phase that requires co-management (likely Phase 11 or 12).
- `scripts/devnet-up.sh` and `scripts/devnet-down.sh` are placeholders. Do not add service orchestration calls until a service is added to `docker-compose.yml`.
- Do NOT start any sequencer or RW-set work in Phase 0. That's Phase 1+.
- Every external library use MUST be Context7-verified per the Library Verification Protocol section. No exceptions.
- `reth-*` git rev must be updated periodically as reth main advances. When upgrading, change ALL reth-* entries to the same new rev in one commit.
```

Append to `Changelog`:

```markdown
### Session 7 — Step 0.6: Docker Compose Placeholder
**Date:** <COMMIT_DATE>
**Agent:** <AGENT_IDENT>
**Summary:** Created `docker-compose.yml` at project root (no active services, `services: {}`, header comment with Phase numbers for when each service lands and explicit note that kraxd is NOT containerized). Created `scripts/devnet-up.sh` and `scripts/devnet-down.sh` as no-op placeholder scripts (`#!/usr/bin/env bash`, `set -euo pipefail`, echo placeholder message, `exit 0`; both `chmod +x`). `scripts/` directory created implicitly by the files. `docker compose config` exits 0; `config --services` produces empty output. Both scripts exit 0. `make lint` clean.
**Commit suggestion:** `chore(devenv): add docker-compose.yml and devnet scripts placeholders — Step 0.6`
```

---

## Commit suggestion

```
chore(devenv): add docker-compose.yml and devnet scripts placeholders — Step 0.6

docker-compose.yml (new file, project root):
- No active services (services: {}).
- No version: field — modern Compose plugin only; version: is deprecated.
- Header comment documents: kraxd is NOT containerized (runs natively
  via make run); auxiliary services land in the phase that needs them
  (Anvil Phase 11/12, Blockscout Phase 11+, Prometheus+Grafana Phase 16+);
  usage once services exist (devnet-up.sh or docker compose up -d).

scripts/devnet-up.sh (new file):
scripts/devnet-down.sh (new file):
- #!/usr/bin/env bash; set -euo pipefail.
- No-op placeholder: echo placeholder message, exit 0.
- Comments explain current status, what they will do when services land,
  and reference docker-compose.yml for the services roadmap.
- Both chmod +x (executable).

Verification:
- docker compose -f docker-compose.yml config exits 0.
- docker compose -f docker-compose.yml config --services: empty output.
- test -x scripts/devnet-up.sh && test -x scripts/devnet-down.sh: pass.
- ./scripts/devnet-up.sh and ./scripts/devnet-down.sh: exit 0.
- make lint exits 0.

Implements ARCHITECTURE.md Phase 0 Step 0.6.
Phase 0 Gate status after this step:
  docker-compose.yml, scripts/devnet-up.sh, scripts/devnet-down.sh
  exist as placeholders with explanatory comments ✅
  (Remaining gate items: forge build — Step 0.7;
   Anvil installed — covered by Step 0.7 Foundry install.)
```

---

## Outcomes

- **All three files created with exact specified content on first attempt.** `docker-compose.yml`, `scripts/devnet-up.sh`, and `scripts/devnet-down.sh` match the plan's "Exact content" blocks verbatim. No deviations.
- **Both `chmod +x` calls executed immediately after file creation.** `test -x` passed for both scripts on first check.
- **All 7 verification steps passed on first attempt.** `docker compose config` exit 0; `config --services` produced empty output (Docker Compose added an auto-inferred `name: krax` field to the normalized output — that is normal behavior and not a service entry). Both scripts ran and exited 0 with correct output. `make lint` exit 0.
- **One cosmetic note:** `docker compose config` output includes `name: krax` (auto-derived from the project directory name). This is Docker Compose's default behavior when no `name:` key is set; it does not affect `config --services` output (still empty) and is not a deviation from the plan.
- **No surprises.** The `scripts/` directory was created implicitly by placing the first script file, exactly as planned. No `.gitkeep` needed or added.

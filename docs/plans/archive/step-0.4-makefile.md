# Step 0.4 — Makefile

> **Plan status:** Ready for execution.
> **Phase:** 0 — Project Setup.
> **ARCHITECTURE.md reference:** Phase 0, Step 0.4.
> **Prerequisites:** Step 0.3 (Minimal Entrypoint) complete and committed. `cargo run --bin kraxd` prints `krax v0.1.0` and exits 0. `cargo run --bin kraxctl -- --help` produces help text and exits 0.

---

## Purpose

Create the project `Makefile` that wraps the seven cargo commands specified in ARCHITECTURE.md:

- `make build` — `cargo build --workspace --release`
- `make test` — `cargo test --workspace`
- `make test-integration` — `cargo test --workspace --features integration`
- `make lint` — `cargo clippy --workspace --all-targets -- -D warnings`
- `make run` — `cargo run --bin kraxd`
- `make fmt` — `cargo fmt --all`
- `make clean` — `cargo clean` and `rm -rf data/`

A default `help` target is also added (running `make` with no arguments prints target descriptions).

The `make test-integration` target requires an `integration` feature flag on every workspace member. That flag does not exist yet. This step defines an empty `integration = []` feature on all 14 workspace members so the target compiles and exits 0 from day one.

After this step, all Phase 0 Gate commands that reference `make` are executable.

---

## Decisions resolved before this plan was written

1. **`make build` = `cargo build --workspace --release`.** Follows ARCHITECTURE.md spec verbatim. Developers iterating use `cargo build` directly; `make build` is the "produce the artifact" target and always produces a release binary. No `make build-debug` alias in this step.

2. **`make run` = `cargo run --bin kraxd`, debug profile.** ARCHITECTURE.md specifies no `--release` flag on run. Debug is correct for Phase 0 iteration. No `make run-release` in this step.

3. **`integration` feature: option (a) — define on every workspace member now.** All 14 `Cargo.toml` files receive `[features] integration = []`. The feature is intentionally empty; integration tests that require it land in Phase 1+. This follows the same "structural placeholder" pattern as Step 0.2's `.gitkeep` files. `make test-integration` resolves, compiles, and exits 0 with zero tests from day one. Background: `cargo test --workspace --features integration` activates the `integration` feature on every workspace member; if any member lacks the feature definition, cargo errors. All 14 must define it.

4. **`make clean` recipe: `cargo clean` then `rm -rf data/`.** `rm -rf data/` is unconditional — it is a no-op if `data/` does not exist on macOS and Linux. No guard needed. `cargo clean` already removes `target/`; the recipe does not duplicate that. No additional cleanup (coverage, logs, env files) — those are out of scope.

5. **`.PHONY` declaration: single line at the top covering all non-file targets.** Includes `help build test test-integration lint run fmt clean`.

6. **Default target: `help`.** Running `make` with no arguments prints the available targets and their descriptions. `help` is the first target in the file. This is the most discoverable choice for a multi-target project Makefile where `test`, `lint`, `run`, and `build` are all equally valid entry points.

7. **`help` target: hand-written `@echo` lines.** Seven targets is small enough that explicit `@echo` lines are more readable than grep-based auto-generation. No Makefile magic, self-evident to a reader unfamiliar with the `##`-comment idiom.

8. **`SHELL := bash`.** Set at the top of the file. No cost for single-command recipes; prevents silent continuation past errors in any future multi-line recipe.

9. **`@` prefix on all cargo commands.** Suppresses Make's echo of each command before running it. Cargo produces its own build output; echoing the command line above it is visual noise. The `@` prefix is omitted from the `help` target's `@echo` lines — the `@` there suppresses the echo of the `echo` command itself, which is the normal convention.

10. **No `make coverage` target.** Out of scope for this step. Coverage tooling (`cargo-llvm-cov` or equivalent) lands alongside real tests in Phase 1.

11. **`make lint` passes clean from day one (planner-verified).** `cargo clippy --workspace --all-targets -- -D warnings` was run against the current codebase before this plan was written. It exits 0 with no warnings. The anticipated "always-true `is_none()` branch" warning on `kraxctl/src/main.rs` does not fire in practice. No `#[allow(...)]` additions are needed.

---

## Library verification checklist

No external Rust libraries are added in this step. `Makefile` is system tooling. The `integration` feature flag uses only Cargo's built-in `[features]` table — stable since Rust 1.0.

No Context7 lookups required.

---

## Files to create or modify

### File 1 (create): `Makefile`

Create at the project root. **Recipe lines must use tab characters (`\t`), not spaces.** This is a hard Make requirement — a Makefile with spaces instead of tabs fails immediately with `Makefile:N: *** missing separator. Stop.` If the coder's editor converts tabs to spaces on save, verify with `cat -A Makefile` (tab-indented lines show `^I` at the start).

```makefile
SHELL := bash

.PHONY: help build test test-integration lint run fmt clean

help:
	@echo "Krax — available make targets:"
	@echo ""
	@echo "  build            cargo build --workspace --release"
	@echo "  test             cargo test --workspace"
	@echo "  test-integration cargo test --workspace --features integration"
	@echo "  lint             cargo clippy --workspace --all-targets -- -D warnings"
	@echo "  run              cargo run --bin kraxd"
	@echo "  fmt              cargo fmt --all"
	@echo "  clean            cargo clean; rm -rf data/"

build:
	@cargo build --workspace --release

test:
	@cargo test --workspace

test-integration:
	@cargo test --workspace --features integration

lint:
	@cargo clippy --workspace --all-targets -- -D warnings

run:
	@cargo run --bin kraxd

fmt:
	@cargo fmt --all

clean:
	@cargo clean
	@rm -rf data/
```

---

### Files 2–15 (modify): all 14 `Cargo.toml` files

Append the following block to the **end** of every `Cargo.toml` in `bin/*/` and `crates/*/`. The blank line before `[features]` separates it from the `[dependencies]` block above.

```toml

[features]
# Empty placeholder; integration tests gated behind this flag land in Phase 1+.
integration = []
```

Files to modify (14 total):

| File | Current last section |
|---|---|
| `bin/kraxd/Cargo.toml` | `[dependencies]` |
| `bin/kraxctl/Cargo.toml` | `[dependencies]` |
| `bin/kraxprover/Cargo.toml` | `[dependencies]` |
| `crates/krax-types/Cargo.toml` | `[dependencies]` |
| `crates/krax-config/Cargo.toml` | `[dependencies]` |
| `crates/krax-mempool/Cargo.toml` | `[dependencies]` |
| `crates/krax-rwset/Cargo.toml` | `[dependencies]` |
| `crates/krax-sequencer/Cargo.toml` | `[dependencies]` |
| `crates/krax-state/Cargo.toml` | `[dependencies]` |
| `crates/krax-execution/Cargo.toml` | `[dependencies]` |
| `crates/krax-batcher/Cargo.toml` | `[dependencies]` |
| `crates/krax-prover/Cargo.toml` | `[dependencies]` |
| `crates/krax-rpc/Cargo.toml` | `[dependencies]` |
| `crates/krax-metrics/Cargo.toml` | `[dependencies]` |

The content added is identical on all 14. No other changes to these files.

---

## Verification steps

Run in order from the project root. Every command must pass before the step is considered done.

Ordering note: `make build` (step 4) runs before `make clean` (step 12) so that `target/` exists when we verify that `make clean` removes it.

```bash
# 1. Confirm Makefile contains exactly the seven spec'd targets.
grep -cE "^(build|test|test-integration|lint|run|fmt|clean):" Makefile
# Expected: 7
# If the count is less than 7, a target is missing or its name is misspelled.
# Note: the -c flag counts matching lines; test and test-integration are
# distinct because test-integration: starts with the full string, not just "test:".

# 2. Confirm default target (make with no args) prints help text and exits 0.
make
echo "Exit code: $?"
# Expected: the help text listing all seven targets. Exit code 0.

# 3. Confirm make help output matches make (no-args).
make help
echo "Exit code: $?"
# Expected: identical output to step 2. Exit code 0.

# 4. Run make build and confirm a release binary is produced.
make build
echo "Exit code: $?"
ls -la target/release/kraxd
# Expected: make exits 0. target/release/kraxd exists and is an executable.

# 5. Run make run and confirm version output and exit code.
make run
echo "Exit code: $?"
# Expected output (exactly): krax v0.1.0
# Expected exit code: 0.

# 6. Run make test and confirm exit code.
make test
echo "Exit code: $?"
# Expected: exit 0. Zero tests is fine — cargo prints "running 0 tests"
# per crate and exits 0.

# 7. Confirm the integration feature compiles cleanly across the workspace.
cargo test --workspace --features integration --no-run
echo "Exit code: $?"
# Expected: exit 0. --no-run compiles without executing. If any crate's
# [features] block is missing or malformed, cargo errors here.
# This is the load-bearing verification for the integration feature changes.

# 8. Run make test-integration and confirm exit code.
make test-integration
echo "Exit code: $?"
# Expected: exit 0. Same as step 6 — zero tests, integration feature active.

# 9. Run make lint and confirm exit code.
make lint
echo "Exit code: $?"
# Expected: exit 0 with no warning output. Planner verified this passes
# clean on the current codebase. Any warning here is a blocker.

# 10. Run make fmt and confirm idempotency.
make fmt
cargo fmt --all -- --check
echo "Exit code from --check: $?"
# make fmt normalizes all source. --check verifies that a second pass would
# produce no further changes. Both must exit 0.

# 11. Confirm all 14 Cargo.toml files have the integration feature.
grep -rL "integration = \[\]" bin/*/Cargo.toml crates/*/Cargo.toml
# Expected: empty output. Any file listed here is missing the feature definition.

# 12. Verify make clean exits 0 and removes target/.
make clean
echo "Exit code: $?"
[ ! -d target ] && echo "OK: target/ removed" || echo "FAIL: target/ still present"
# Expected: exit 0. target/ is absent. data/ did not exist, so rm -rf data/
# was a no-op — no error output, exit 0 confirmed by the make clean line above.
```

---

## Definition of "Step 0.4 done"

- ✅ `Makefile` exists at the project root.
- ✅ `grep -cE "^(build|test|test-integration|lint|run|fmt|clean):" Makefile` returns `7`.
- ✅ `make` (no args) and `make help` print the target list and exit 0.
- ✅ `make build` exits 0 and produces `target/release/kraxd`.
- ✅ `make run` prints `krax v0.1.0` and exits 0.
- ✅ `make test` exits 0.
- ✅ `make test-integration` exits 0.
- ✅ `cargo test --workspace --features integration --no-run` exits 0.
- ✅ `make lint` exits 0 with no warnings.
- ✅ `make fmt` is idempotent: `cargo fmt --all -- --check` exits 0 after `make fmt`.
- ✅ `make clean` exits 0 and removes `target/`.
- ✅ `grep -rL "integration = \[\]" bin/*/Cargo.toml crates/*/Cargo.toml` returns empty.
- ✅ All 14 `Cargo.toml` files have the `integration` feature with the placeholder comment.
- ✅ ARCHITECTURE.md Step 0.4 is checked off.
- ✅ AGENTS.md `Current State` and `Changelog` are updated.

---

## Open questions / coder follow-ups

Two mechanical items require coder attention; neither is a decision:

1. **Tab characters in the Makefile.** Ensure all recipe lines (the indented lines under each target) use real tab characters. Verify before running any `make` command: `cat -A Makefile | grep "^\^I"` — tab-indented lines appear with `^I` at the start. An editor that silently converts tabs to spaces will cause `missing separator` on `make help`.

2. **Uniformity of the `[features]` block.** The block appended to all 14 files is identical — same comment text, same feature name. After editing, `grep -rL "integration = \[\]" bin/*/Cargo.toml crates/*/Cargo.toml` must return empty. If any file was skipped or has a variant spelling, verification step 11 surfaces it.

No Context7 lookups required for this step.

---

## What this step does NOT do

Stay in lane. Out of scope for Step 0.4:

- ❌ `make build-debug` or any non-spec target aliases.
- ❌ `make run-release`. Debug is correct for Phase 0 iteration; release run deferred to when there is a reason.
- ❌ `make coverage`. Coverage tooling lands with the first real tests in Phase 1.
- ❌ `make ci` or any CI-orchestration target.
- ❌ Cleaning `coverage/`, `*.log`, `.env.local`, or anything beyond `data/`. `make clean` is bounded to the ARCHITECTURE.md spec.
- ❌ `rustfmt.toml` or `clippy.toml`. Format and lint configuration are Step 0.8. `make fmt` and `make lint` work against default rules until then.
- ❌ Any changes to `bin/*/src/` or `crates/*/src/`. Source files are untouched; only `Cargo.toml` files gain the `[features]` block.
- ❌ Real integration test code. The `integration` feature is defined but empty. Tests that use it land in Phase 1+.
- ❌ `.gitignore` (Step 0.5), `docker-compose.yml` (Step 0.6), `contracts/` (Step 0.7), `README.md` (Step 0.9).

---

## Updates to other files in the same commit

### `ARCHITECTURE.md`

Mark Step 0.4 complete. Change:

```markdown
### Step 0.4 — Makefile
- [ ] `make build` — runs `cargo build --workspace --release`
- [ ] `make test` — runs `cargo test --workspace`
- [ ] `make test-integration` — runs `cargo test --workspace --features integration`
- [ ] `make lint` — runs `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `make run` — runs `cargo run --bin kraxd`
- [ ] `make fmt` — runs `cargo fmt --all`
- [ ] `make clean` — runs `cargo clean` and removes `data/`
```

to:

```markdown
### Step 0.4 — Makefile ✅
- [x] `make build` — runs `cargo build --workspace --release`
- [x] `make test` — runs `cargo test --workspace`
- [x] `make test-integration` — runs `cargo test --workspace --features integration`
- [x] `make lint` — runs `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `make run` — runs `cargo run --bin kraxd`
- [x] `make fmt` — runs `cargo fmt --all`
- [x] `make clean` — runs `cargo clean` and removes `data/`
```

### `AGENTS.md`

Replace `Current State` with:

```markdown
**Current Phase:** Phase 0 — Project Setup (Steps 0.1–0.4 complete, Step 0.5 next)

**What was just completed:**
- **Step 0.4 — Makefile done.** `Makefile` created at project root with seven spec'd targets: `build` (release), `test`, `test-integration`, `lint`, `run` (debug), `fmt`, `clean`. Default target is `help` (hand-written `@echo` lines). `SHELL := bash`; `@` prefix on all recipes. All 14 workspace `Cargo.toml` files gained `[features] integration = []` (empty placeholder, comment notes Phase 1+ intent) so `make test-integration` resolves and exits 0 from day one. `make lint` passes clean with `-D warnings` (confirmed pre-plan: no warnings on current codebase, including the expected `is_none()` branch in `kraxctl`).
- (Carry forward: Step 0.3 — `cargo run --bin kraxd` → `krax v0.1.0`; `cargo run --bin kraxctl -- --help` → help text.)
- (Carry forward: Step 0.2 — full `bin/*` and `crates/*` tree, 14 workspace members, `cargo build --workspace` succeeds.)
- (Carry forward: Step 0.1 — revm 38, reth-* git rev `02d1776786abc61721ae8876898ad19a702e0070`, jsonrpsee 0.26, etc. See archived plan for full version table.)

**What to do next (in order):**
1. 🔴 **Step 0.5 — `.gitignore` & `.env.example`.**
2. Step 0.6 — Docker Compose placeholder.
3. Steps 0.7 through 0.9 in order, per ARCHITECTURE.md.

**Blockers:**
- Repository URL is a placeholder (`https://github.com/krax-labs/krax`). Replace before V1.0 branding. Not a blocker for Phase 0 work.
- Project name not finalized. "Krax" is a working name. Search-replace before mainnet branding (V1.1 concern).

**Notes:**
- `kraxd` version banner uses `println!` — documented Rule 4 exception with inline comment in `main.rs`. All future runtime output uses `tracing`.
- `tracing-subscriber` initialization is deferred to a later step alongside `krax-config`.
- The `Commands` enum in `kraxctl` is empty until a step adds a real subcommand. Clippy does NOT warn on the `if cli.command.is_none()` branch in practice (verified at Step 0.4). The warning note from Step 0.3 is withdrawn; no `#[allow(...)]` needed.
- The `integration` feature on every crate is intentionally empty. Integration tests land in Phase 1+.
- Do NOT start any sequencer or RW-set work in Phase 0. That's Phase 1+.
- Every external library use MUST be Context7-verified per the Library Verification Protocol section. No exceptions.
- `reth-*` git rev must be updated periodically as reth main advances. When upgrading, change ALL reth-* entries to the same new rev in one commit.
```

Append to `Changelog`:

```markdown
### Session 5 — Step 0.4: Makefile
**Date:** <COMMIT_DATE>
**Agent:** <AGENT_IDENT>
**Summary:** Created `Makefile` at project root with seven spec'd targets (`build`, `test`, `test-integration`, `lint`, `run`, `fmt`, `clean`) plus a hand-written `help` default target. `make build` = `cargo build --workspace --release` (follows ARCHITECTURE.md spec verbatim). `make run` = debug. `SHELL := bash` set; `@` prefix on all recipes. All 14 workspace `Cargo.toml` files updated with `[features] integration = []` (empty placeholder, comment notes Phase 1+ intent) so `make test-integration` resolves and exits 0 from day one. `make lint` passes clean with `-D warnings` (planner pre-verified on current codebase; `is_none()` warning does not fire). `make fmt` is idempotent.
**Commit suggestion:** `chore(build): add Makefile with all Phase 0 targets — Step 0.4`
```

---

## Commit suggestion

```
chore(build): add Makefile with all Phase 0 targets — Step 0.4

- Makefile: seven targets per ARCHITECTURE.md spec (build, test,
  test-integration, lint, run, fmt, clean) plus a hand-written help
  default target. SHELL := bash. @ prefix on all recipes.
  make build = cargo build --workspace --release (spec-compliant).
  make run = cargo run --bin kraxd (debug, per spec).
  make clean = cargo clean + rm -rf data/ (unconditional; no-op if absent).

- All 14 Cargo.toml files (bin/* and crates/*): append [features]
  integration = [] so make test-integration resolves cleanly from day one.
  Feature is intentionally empty; integration tests using it land in Phase 1+.

Verification:
- make build exits 0; target/release/kraxd produced.
- make run prints "krax v0.1.0", exits 0.
- make test exits 0 (zero tests).
- make test-integration exits 0 (zero tests; integration feature resolves).
- cargo test --workspace --features integration --no-run exits 0.
- make lint exits 0 (no warnings with -D warnings).
- make fmt is idempotent (cargo fmt --all -- --check exits 0 after make fmt).
- make clean exits 0; target/ removed.
- grep -rL "integration = []" bin/*/Cargo.toml crates/*/Cargo.toml: empty.

Implements ARCHITECTURE.md Phase 0 Step 0.4.
Phase 0 Gate status after this step:
  make build succeeds ✅
  make run prints version and exits 0 ✅
  make test runs (zero tests is fine) ✅
  make lint passes with -D warnings ✅
  make fmt is idempotent ✅
  (Remaining gate items require Steps 0.6 and 0.7.)
```

---

## Outcomes

- **Tab characters written correctly on first attempt.** Created the Makefile via `cat >` heredoc in bash, which preserves literal tabs. Verified using Python byte inspection (`line.startswith('\t')`); all 17 recipe lines confirmed tab-prefixed. macOS `cat -A` is not available (`-A` is a GNU extension); the Python check substituted cleanly.
- **All 14 Cargo.toml files updated with the integration feature.** `grep -rL "integration = \[\]" bin/*/Cargo.toml crates/*/Cargo.toml` returned empty on first attempt — no file was missed.
- **`make lint` passed clean with no warnings.** Planner's pre-verification held: the anticipated `is_none()` branch warning on `kraxctl` did not fire. `-D warnings` exit 0 on first attempt.
- **`make fmt` is idempotent.** `cargo fmt --all -- --check` exited 0 immediately after `make fmt` with no diff.
- **All 12 verification steps passed on first attempt.** No deviations from specified file content. `make build` produced `target/release/kraxd`; `make run` printed `krax v0.1.0`; `make clean` removed `target/` and exited 0. `make test` and `make test-integration` each ran zero tests and exited 0.
- **No deviations from specified file content.** The Makefile and all 14 Cargo.toml additions match the plan spec exactly.

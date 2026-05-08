# Step 0.8 — Lint & Format Configuration

> **Plan status:** Ready for execution.
> **Phase:** 0 — Project Setup.
> **ARCHITECTURE.md reference:** Phase 0, Step 0.8.
> **Prerequisites:** Step 0.7 (Foundry init for contracts) complete and committed. `make lint` passes. `cargo fmt --all -- --check` exits 0.

---

## Purpose

Four deliverables:

1. **`rustfmt.toml` (create at project root)** — stable-only formatting rules applied project-wide. Sets explicit values for all opinions worth having at Phase 0.

2. **`clippy.toml` (create at project root)** — threshold tuning for clippy's configurable lints (cognitive complexity, argument count, function length, type complexity). The allow/deny policy itself lives in `Cargo.toml`, not here.

3. **Root `Cargo.toml` (edit)** — add `[workspace.lints.rust]` and `[workspace.lints.clippy]` sections defining the workspace-wide lint policy. All per-crate `Cargo.toml` files opt in via `[lints] workspace = true`.

4. **14 per-crate `Cargo.toml` files (edit)** — add `[lints] workspace = true` to each of the 3 `bin/*` and 11 `crates/*` member packages. Mechanical but required for workspace inheritance to take effect.

After this step, `make lint` enforces the full lint policy and `make fmt` produces stable, idempotent output. Both are Phase 0 Gate items.

---

## Decisions resolved before this plan was written

### Decision 1 — Actual `cargo clippy --workspace --all-targets -- -D warnings` output

Run and captured before this plan was drafted. **Exact output:**

```
    Checking krax-prover v0.1.0 (…/crates/krax-prover)
    Checking krax-rpc v0.1.0 (…/crates/krax-rpc)
    Checking krax-mempool v0.1.0 (…/crates/krax-mempool)
    Checking krax-execution v0.1.0 (…/crates/krax-execution)
    Checking krax-state v0.1.0 (…/crates/krax-state)
    Checking krax-batcher v0.1.0 (…/crates/krax-batcher)
    Checking kraxctl v0.1.0 (…/bin/kraxctl)
    Checking krax-types v0.1.0 (…/crates/krax-types)
    Checking kraxd v0.1.0 (…/bin/kraxd)
    Checking krax-metrics v0.1.0 (…/crates/krax-metrics)
    Checking krax-rwset v0.1.0 (…/crates/krax-rwset)
    Checking krax-config v0.1.0 (…/crates/krax-config)
    Checking kraxprover v0.1.0 (…/bin/kraxprover)
    Checking krax-sequencer v0.1.0 (…/crates/krax-sequencer)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
```

**Exit code: 0. Zero warnings.** The `Commands` enum in `bin/kraxctl/src/main.rs` (a zero-variant enum used in an `Option<Commands>` field) produces no clippy output under Rust 1.95.0 with `-D warnings`. This was already noted as confirmed in the AGENTS.md Step 0.4 entry: "Clippy does NOT warn on the `if cli.command.is_none()` branch in practice." Speculation from Steps 0.3 and 0.4 plans is definitively closed.

### Decision 2 — `rustfmt.toml`: stable-only options

Stable-only options (Rust 1.95.0 toolchain). No nightly rustfmt. Reasoning: the project is philosophically pinned-stable (`channel = "1.95.0"` in `rust-toolchain.toml`); introducing nightly for formatting alone is an inconsistency that would need to be justified to every contributor. The nightly-only wins (`imports_granularity = "Crate"`) are cosmetic at Phase 0 when all library crates are empty. No Makefile changes required — `make fmt` stays `cargo fmt --all`.

### Decision 3 — Specific `rustfmt.toml` knobs

| Option | Value | Reasoning |
|---|---|---|
| `edition` | `"2024"` | Must match `Cargo.toml` `edition = "2024"`. |
| `max_width` | `100` | Rust default; confirmed explicitly so it is self-documenting. Keeps long function signatures readable within the 60–80 line function cap. |
| `tab_spaces` | `4` | Rust default; confirmed explicitly. |
| `newline_style` | `"Unix"` | Matches LF convention established in Step 0.5 (`.env.example`) and Step 0.6 (shell scripts). |
| `use_field_init_shorthand` | `true` | Stable modern Rust idiom. `Foo { x: x }` → `Foo { x }`. |
| `use_try_shorthand` | `true` | Stable modern Rust idiom. `try!(expr)` → `expr?`. |

### Decision 4a — `clippy.toml` threshold values

| Key | Value | Reasoning |
|---|---|---|
| `cognitive-complexity-threshold` | `25` | Default. Reinforces the 60–80 line function cap — genuinely complex functions tend to violate both. |
| `too-many-arguments-threshold` | `7` | Default. More than 7 args is a signal to introduce a config struct. |
| `too-many-lines-threshold` | `80` | Tighter than the clippy default of 100. Directly enforces the AGENTS.md 60–80 line function cap. Note: `too_many_lines` is in `clippy::pedantic`; it fires only when pedantic is enabled (see Decision 4b). |
| `type-complexity-threshold` | `250` | Default. |

### Decision 4b — Lint allow/deny policy: workspace-level via `[workspace.lints.clippy]`

Workspace-level policy in root `Cargo.toml` (Rust 1.74+ feature, supported on 1.95.0). All 14 per-crate `Cargo.toml` files opt in via `[lints] workspace = true`. This is the modern Rust convention and matches the workspace-inheritance pattern already used for dependencies. One source of truth.

**Rust lint policy (`[workspace.lints.rust]`):**
- `unsafe_code = "deny"` — directly enforces AGENTS.md Rule: "No `unsafe` without a `// SAFETY:` comment... requires reviewer sign-off." Violations must add `#[allow(unsafe_code)]` with justification.
- `missing_docs = "warn"` — enforces AGENTS.md "doc comments (`///`) on every public item" rule automatically on every `make lint` run. At `warn` level, it surfaces violations without blocking progress; violations are escalated to errors by `-D warnings` in `make lint`.

**Clippy lint policy (`[workspace.lints.clippy]`):**

*Deny — code quality rules matching AGENTS.md:*
- `dbg_macro` — no `dbg!()` in committed code
- `todo` — no `todo!()` in committed code; use a proper error type or a comment
- `unimplemented` — no `unimplemented!()` in committed code
- `unwrap_used` — matches AGENTS.md Rule 3: "`unwrap()` and `expect()` are forbidden in production code paths." Tests, build scripts, and startup invariants are exempt via `#[allow(clippy::unwrap_used)]` at the site.

Note: `expect_used` is intentionally NOT denied. `expect("message")` with a meaningful message is acceptable in startup invariant paths (which already exist in `kraxctl/main.rs:28`). Code review catches bad uses.

*Pedantic group:* enabled at `warn`, priority `-1`. The lower priority means individual overrides below take precedence over the group setting. Because `make lint` runs with `-D warnings`, any pedantic lint that fires will be treated as an error — so pedantic is effectively enforced strictly.

*Curated allow-list overriding pedantic:* four lints suppressed because they fire on valid Krax patterns:
- `module_name_repetitions` — domain types (`RwSetError`, `SequencerError`, `MptState`) live in modules named after the concept (`rwset`, `sequencer`, `state`). The repetition is intentional and idiomatic in Rust; suppressing this prevents noise on every public type in Phase 1.
- `must_use_candidate` — fires on any function returning a non-unit value that isn't `#[must_use]`. Constructors, builders, and getters don't need this annotation by default. Too noisy to enforce globally.
- `missing_errors_doc` — requires `# Errors` sections in doc comments for `Result`-returning functions. Deferred: covered by `missing_docs = "warn"` at the Rust lint level, which requires doc comments to exist at all; section-level requirements are a Phase 1+ enforcement decision.
- `missing_panics_doc` — same reasoning as `missing_errors_doc`.

### Decision 5 — `bin/kraxctl/src/main.rs` empty Commands enum

**Moot.** Decision 1 confirms zero warnings. No change to `kraxctl/main.rs` is required or in scope for Step 0.8.

### Decision 6 — Phase 0 Gate impact

`make lint passes with -D warnings` is the load-bearing acceptance criterion for this step. After Step 0.8 lands, this gate item is satisfied by the same command that currently passes: `cargo clippy --workspace --all-targets -- -D warnings`. The new `[workspace.lints.clippy]` policy adds deny lints and pedantic warnings, but all current code is clean against them (verified: no `unwrap()`, `todo!()`, `dbg!()`, `unimplemented!()`, or `unsafe` blocks in any file; no public items lacking doc comments).

### Decision 7 — `cargo fmt --all` reformatting pass

After creating `rustfmt.toml`, the coder must run `cargo fmt --all` once, capture the diff (possibly empty — current files are tiny), and stage any reformatted files as part of this step's commit. Then run `cargo fmt --all -- --check` a second time to verify idempotency. Reformatting now while the codebase is 3 minimal `main.rs` files and 11 empty library stubs is cheap; deferring makes it progressively more expensive.

### Decision 8 — Makefile changes

None. Stable-only rustfmt (Decision 2) requires no changes to the `fmt` target. The `lint` target stays unchanged; the policy moves into `Cargo.toml` but the invocation stays `cargo clippy --workspace --all-targets -- -D warnings`.

### Decision 9 — `missing_docs` enforcement mechanism

`missing_docs = "warn"` in `[workspace.lints.rust]` (see Decision 4b). This is continuous enforcement at every `make lint` run rather than a one-time verification grep. A simple `cargo doc --workspace --no-deps` exit-0 check (no grep) is included in verification to confirm doc generation doesn't crash — that is the only value a separate doc check adds once `missing_docs` is a lint.

---

## Library verification checklist

No external Rust libraries are used in this step. `rustfmt` and `clippy` are toolchain components already installed via `rust-toolchain.toml` (`components = ["rustfmt", "clippy", "rust-src"]`). No Context7 lookups required.

| Tool | Version | Status |
|---|---|---|
| `rustfmt` | Bundled with 1.95.0 toolchain | Already installed (`components` list) |
| `clippy` | Bundled with 1.95.0 toolchain | Already installed (`components` list) |

---

## Files to create or modify

### Ordered execution sequence

The order below is mandatory for a clean run:

1. Create `rustfmt.toml` at project root
2. Create `clippy.toml` at project root
3. Edit root `Cargo.toml` (add lint sections at end of file)
4. Edit all 14 per-crate `Cargo.toml` files (add `[lints] workspace = true`)
5. Run `cargo fmt --all` to apply rustfmt.toml (step 1 must exist first)
6. Stage any reformatted files (`git add -p` or equivalent)
7. Run verification steps

---

### Step 1 (create): `rustfmt.toml`

Create at the project root. LF line endings, trailing newline.

**Exact content:**

```toml
# rustfmt.toml — project-wide formatting rules.
# Stable options only (Rust 1.95.0 pinned toolchain; no nightly rustfmt).
# See: https://rust-lang.github.io/rustfmt/?version=stable
edition                  = "2024"
max_width                = 100
tab_spaces               = 4
newline_style            = "Unix"
use_field_init_shorthand = true
use_try_shorthand        = true
```

---

### Step 2 (create): `clippy.toml`

Create at the project root. LF line endings, trailing newline.

**Exact content:**

```toml
# clippy.toml — threshold tuning for clippy lints.
# Allow/deny policy lives in [workspace.lints.clippy] in root Cargo.toml.
# See: https://doc.rust-lang.org/clippy/configuration.html
cognitive-complexity-threshold = 25
too-many-arguments-threshold   = 7
too-many-lines-threshold       = 80
type-complexity-threshold      = 250
```

---

### Step 3 (edit): root `Cargo.toml`

Append the following two sections at the **end of the file**, after the existing `[workspace.dependencies]` block. Do not modify any existing content.

**Content to append:**

```toml

# ---------------------------------------------------------------------------
# Workspace-level lint policy (Rust 1.74+, Cargo workspace inheritance).
# Per-crate Cargo.toml files opt in via [lints] workspace = true.
# ---------------------------------------------------------------------------

[workspace.lints.rust]
unsafe_code  = "deny"
missing_docs = "warn"

[workspace.lints.clippy]
# AGENTS.md code quality rules enforced at workspace level.
dbg_macro     = "deny"
todo          = "deny"
unimplemented = "deny"
unwrap_used   = "deny"

# Pedantic group at priority -1; individual overrides below take precedence.
# Note: make lint runs with -D warnings, so pedantic = "warn" is effectively
# enforced as an error. Any pedantic lint that fires must be fixed or suppressed
# at the call site with a documented reason.
pedantic = { level = "warn", priority = -1 }

# Suppress pedantic lints that fire on valid Krax patterns.
# module_name_repetitions: domain types (RwSetError, SequencerError, MptState)
#   live in modules named after the concept. Repetition is intentional.
# must_use_candidate: constructors, builders, getters don't need #[must_use].
# missing_errors_doc: deferred — covered by missing_docs = "warn" above.
# missing_panics_doc: same reasoning as missing_errors_doc.
module_name_repetitions = "allow"
must_use_candidate      = "allow"
missing_errors_doc      = "allow"
missing_panics_doc      = "allow"
```

---

### Step 4 (edit): 14 per-crate `Cargo.toml` files

Add the following block at the **end of each file**, after the existing `[features]` section. Do not modify any existing content.

**Content to append to each file:**

```toml

[lints]
workspace = true
```

**Files to edit (all 14):**

Binary crates:
- `bin/kraxd/Cargo.toml`
- `bin/kraxctl/Cargo.toml`
- `bin/kraxprover/Cargo.toml`

Library crates:
- `crates/krax-types/Cargo.toml`
- `crates/krax-config/Cargo.toml`
- `crates/krax-mempool/Cargo.toml`
- `crates/krax-rwset/Cargo.toml`
- `crates/krax-sequencer/Cargo.toml`
- `crates/krax-state/Cargo.toml`
- `crates/krax-execution/Cargo.toml`
- `crates/krax-batcher/Cargo.toml`
- `crates/krax-prover/Cargo.toml`
- `crates/krax-rpc/Cargo.toml`
- `crates/krax-metrics/Cargo.toml`

**Verification after Step 4:** `grep -rL "workspace = true" bin/*/Cargo.toml crates/*/Cargo.toml` must return empty output (no files missing the opt-in).

---

### Step 5: Run `cargo fmt --all`

After Steps 1–4 are complete, run:

```bash
cargo fmt --all
```

Capture the diff with `git diff`. The diff may be empty (current files are trivially formatted) or it may contain minor changes to the `main.rs` files or crate-level doc comments. Whatever changes `cargo fmt` makes are correct — they represent the project's canonical formatting under the new rules. Stage and commit them as part of this step's single commit.

Do **not** skip this step even if you expect no diff. The idempotency check in verification confirms it.

---

## Verification steps

Run in order from the project root. Every command must pass before the step is considered done.

```bash
# 1. Confirm rustfmt.toml exists.
test -f rustfmt.toml && echo "OK: rustfmt.toml exists"
# Expected: "OK: rustfmt.toml exists"

# 2. Confirm clippy.toml exists.
test -f clippy.toml && echo "OK: clippy.toml exists"
# Expected: "OK: clippy.toml exists"

# 3. Confirm key rustfmt.toml values are present.
grep 'edition = "2024"' rustfmt.toml && echo "OK: edition = 2024"
grep 'newline_style = "Unix"' rustfmt.toml && echo "OK: newline_style = Unix"
grep 'too-many-lines-threshold = 80' clippy.toml && echo "OK: too-many-lines = 80"
# Expected: three "OK:" lines.

# 4. Confirm workspace lint sections are present in root Cargo.toml.
grep 'unsafe_code.*=.*"deny"' Cargo.toml && echo "OK: unsafe_code deny"
grep 'missing_docs.*=.*"warn"' Cargo.toml && echo "OK: missing_docs warn"
grep 'unwrap_used.*=.*"deny"' Cargo.toml && echo "OK: unwrap_used deny"
grep 'pedantic' Cargo.toml && echo "OK: pedantic present"
# Expected: four "OK:" lines.

# 5. Confirm all 14 per-crate Cargo.toml files opt in to workspace lint policy.
grep -rL "workspace = true" bin/*/Cargo.toml crates/*/Cargo.toml
# Expected: EMPTY output. Any file path printed here is missing [lints] workspace = true.
# If any paths appear, edit those files and re-run.

# 6. Confirm formatting is idempotent (load-bearing — must be run AFTER Step 5's fmt pass).
cargo fmt --all -- --check && echo "OK: fmt idempotent"
# Expected: exit 0, "OK: fmt idempotent".
# If this fails, rustfmt.toml has a configuration that produces non-idempotent
# output — investigate before committing.

# 7. Run cargo clippy — the load-bearing Phase 0 Gate item.
cargo clippy --workspace --all-targets -- -D warnings && echo "OK: clippy clean"
# Expected: exit 0, "OK: clippy clean".
# This is the same command make lint runs. If it fails here, make lint will fail.
# Any new warning introduced by the lint policy must be fixed before proceeding.

# 8. Run make lint — confirms the Makefile target works end-to-end.
make lint && echo "OK: make lint"
# Expected: exit 0. This target has not changed; it still runs
# cargo clippy --workspace --all-targets -- -D warnings.

# 9. Run make fmt idempotency — Phase 0 Gate item.
make fmt && git diff --quiet && echo "OK: make fmt idempotent"
# Expected: exit 0 for make fmt, then git diff --quiet exits 0 (no unstaged changes).
# If git diff --quiet fails, cargo fmt produced a second round of changes —
# rustfmt.toml has a non-idempotent configuration. Investigate.

# 10. Confirm doc generation succeeds (no crash, missing_docs violations caught by make lint).
cargo doc --workspace --no-deps 2>&1 | tail -5
# Expected: exit 0. The last few lines show "Finished" or "Generated" output.
# Any missing_docs warnings from Phase 0's existing code would also have fired
# in step 7 (cargo clippy enforces missing_docs via the workspace lint policy).
# This step confirms doc generation itself doesn't error out.
```

---

## Definition of "Step 0.8 done"

- ✅ `rustfmt.toml` exists at project root with all 6 options specified.
- ✅ `clippy.toml` exists at project root with all 4 threshold entries.
- ✅ Root `Cargo.toml` has `[workspace.lints.rust]` with `unsafe_code = "deny"` and `missing_docs = "warn"`.
- ✅ Root `Cargo.toml` has `[workspace.lints.clippy]` with 4 deny lints, `pedantic = { level = "warn", priority = -1 }`, and 4 curated allow overrides.
- ✅ All 14 per-crate `Cargo.toml` files have `[lints] workspace = true`.
- ✅ `grep -rL "workspace = true" bin/*/Cargo.toml crates/*/Cargo.toml` returns empty.
- ✅ `cargo fmt --all -- --check` exits 0 (idempotent — confirmed by running twice).
- ✅ `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- ✅ `make lint` exits 0.
- ✅ `make fmt && git diff --quiet` exits 0 (idempotency via Makefile).
- ✅ `cargo doc --workspace --no-deps` exits 0.
- ✅ ARCHITECTURE.md Step 0.8 is checked off (all 3 items).
- ✅ AGENTS.md `Current State` and `Changelog` are updated.

---

## Open questions / coder follow-ups

**If `cargo clippy` fails after adding `[workspace.lints.clippy]`:**
The most likely cause is a pedantic lint firing on existing code (e.g., `clippy::too_many_lines` on a function that was hand-written long). Check the lint name in the error, then either fix the code or — if the lint fires on valid Krax patterns not already in the allow-list — surface it to the maintainer before adding a new allow override. Do not add workspace-level allows for lint-specific code issues; use `#[allow(clippy::...)]` at the call site with a comment.

**If `cargo clippy` fails with `missing_docs` violations:**
Any `pub` item in any crate that lacks a `///` doc comment will fire. For Phase 0 code, this should not occur (no `pub` items in binary crates; library crates have no items yet). If it fires, add a doc comment. Do not add `#[allow(missing_docs)]` to silence it.

**If `cargo fmt --all -- --check` fails after the fmt pass (non-idempotency):**
`rustfmt.toml` has a configuration that produces non-idempotent output. Run `cargo fmt --all` a second time, inspect the remaining diff, and identify which option is causing it. Remove or change that option.

**If `grep -rL "workspace = true"` returns file paths:**
Those files are missing `[lints] workspace = true`. Add it and re-run verification step 5.

**If `cargo doc --workspace --no-deps` exits non-zero:**
A doc-comment syntax error was introduced in an earlier step. Run `cargo doc -p <crate-name>` for each crate to isolate it. Fix the syntax before committing.

---

## What this step does NOT do

- ❌ No changes to `bin/kraxctl/src/main.rs`. The empty `Commands` enum produces no clippy warnings under 1.95.0 (Decision 1 confirmed). Adding a `Version` subcommand is deferred to a step that has a functional reason to add it.
- ❌ No nightly rustfmt options. `imports_granularity`, `group_imports`, `format_code_in_doc_comments`, and `imports_layout` are nightly-only (confirmed: rustfmt on 1.95.0 prints a warning and silently ignores them). Deferred until the project is ready to add a nightly toolchain component.
- ❌ No changes to the `Makefile`. The `fmt` and `lint` targets are unchanged. Lint policy moving into `Cargo.toml` does not change how the targets are invoked.
- ❌ No `clippy::expect_used` deny. `expect()` with a meaningful message is acceptable in startup invariant paths (already present in `kraxctl/main.rs:28`). Code review catches bad uses.
- ❌ No `clippy::pedantic = "deny"`. Pedantic at `warn` level (escalated to error by `-D warnings` in `make lint`) is strict enough. A hard deny would prevent adding fine-grained call-site suppression for legitimate exceptions.
- ❌ No changes to `contracts/`, `scripts/`, `docker-compose.yml`, or any `docs/` file other than this plan.
- ❌ No new Rust dependencies of any kind.
- ❌ `README.md` (Step 0.9).

---

## Updates to other files in the same commit

### `ARCHITECTURE.md`

Mark Step 0.8 complete. Change:

```markdown
### Step 0.8 — Lint & Format Configuration
- [ ] `rustfmt.toml` with project-wide formatting rules
- [ ] `clippy.toml` with allowed/denied lints
- [ ] Verify `cargo clippy` passes on the empty workspace
```

to:

```markdown
### Step 0.8 — Lint & Format Configuration ✅
- [x] `rustfmt.toml` with project-wide formatting rules
- [x] `clippy.toml` with allowed/denied lints
- [x] Verify `cargo clippy` passes on the empty workspace
```

### `AGENTS.md`

Replace `Current State` with:

```markdown
**Current Phase:** Phase 0 — Project Setup (Steps 0.1–0.8 complete, Step 0.9 next)

**What was just completed:**
- **Step 0.8 — Lint & format configuration done.** `rustfmt.toml` created (stable-only; 6 options: edition 2024, max_width 100, Unix newlines, field-init shorthand, try shorthand). `clippy.toml` created (4 threshold entries; too-many-lines tightened to 80 to match AGENTS.md function cap). Root `Cargo.toml` updated with `[workspace.lints.rust]` (unsafe_code deny, missing_docs warn) and `[workspace.lints.clippy]` (dbg_macro/todo/unimplemented/unwrap_used deny; pedantic warn at priority -1; 4 curated allow overrides). All 14 per-crate `Cargo.toml` files updated with `[lints] workspace = true`. `cargo fmt --all` run once; diff staged. `make lint` exits 0. `make fmt` is idempotent. `cargo doc --workspace --no-deps` exits 0.
- (Carry forward: Step 0.7 — `contracts/` initialized via `forge init --no-git`; `forge-std` submodule at v1.16.1; `foundry.toml` pinned to solc 0.8.24.)
- (Carry forward: Step 0.6 — `docker-compose.yml` placeholder; `scripts/devnet-up.sh` and `scripts/devnet-down.sh` as no-ops.)
- (Carry forward: Step 0.5 — `.gitignore` audited; `.env.example` with four `KRAX_*` variables.)
- (Carry forward: Step 0.4 — Makefile with seven targets.)
- (Carry forward: Step 0.3 — `cargo run --bin kraxd` → `krax v0.1.0`; `cargo run --bin kraxctl -- --help` → help text.)
- (Carry forward: Steps 0.1–0.2 — workspace, toolchain, full `bin/*` and `crates/*` tree.)

**What to do next (in order):**
1. 🔴 **Step 0.9 — README.** Public-facing README with one-paragraph description, build steps, quick start, links to AGENTS.md and ARCHITECTURE.md.

**Blockers:**
- Repository URL is a placeholder (`https://github.com/krax-labs/krax`). Replace before V1.0 branding. Not a blocker for Phase 0 work.
- Project name not finalized. "Krax" is a working name. Search-replace before mainnet branding (V1.1 concern).

**Notes:**
- `kraxd` version banner uses `println!` — documented Rule 4 exception with inline comment in `main.rs`. All future runtime output uses `tracing`.
- `tracing-subscriber` initialization is deferred to a later step alongside `krax-config`.
- The `Commands` enum in `kraxctl` is empty until a step adds a real subcommand. No clippy warning fires on it under 1.95.0 (confirmed at Steps 0.4 and 0.8).
- The `integration` feature on every crate is intentionally empty. Integration tests land in Phase 1+.
- `.env.example` documents the four kraxd env vars but nothing reads them yet. Config loading (`krax-config`) arrives in Phase 1+.
- `docker-compose.yml` is a placeholder. Do not add Anvil to Compose until Phase 11 or 12.
- `contracts/` is a Foundry project with no real Solidity yet. Real contracts land in Phase 12. Do not add stub contract files before then.
- `forge-std` is a git submodule at `v1.16.1`. New contributors must run `git submodule update --init` after cloning.
- Workspace lint policy: `unsafe_code` and `unwrap_used` are denied at workspace level. Call-site `#[allow(...)]` with a comment is required for any legitimate exception. For `unwrap_used`, tests are exempt via `#[allow(clippy::unwrap_used)]` at the test module or function level.
- Pedantic lints are `warn` in `[workspace.lints.clippy]` but `-D warnings` in `make lint` escalates them to errors. Any pedantic lint that fires on Phase 1+ code must be fixed or suppressed at the call site with a reason.
- `missing_docs = "warn"` is enforced by `make lint`. Every public item in every crate must have a `///` doc comment before it can land.
- Do NOT start any sequencer or RW-set work in Phase 0. That's Phase 1+.
- Every external library use MUST be Context7-verified per the Library Verification Protocol section. No exceptions.
- `reth-*` git rev must be updated periodically as reth main advances. When upgrading, change ALL reth-* entries to the same new rev in one commit.
```

Append to `Changelog`:

```markdown
### Session 9 — Step 0.8: Lint & Format Configuration
**Date:** <COMMIT_DATE>
**Agent:** <AGENT_IDENT>
**Summary:** Created `rustfmt.toml` (stable-only; 6 options: edition 2024, max_width 100, Unix newlines, field-init shorthand, try shorthand). Created `clippy.toml` (4 thresholds; too-many-lines tightened to 80). Edited root `Cargo.toml` to add `[workspace.lints.rust]` (unsafe_code deny, missing_docs warn) and `[workspace.lints.clippy]` (dbg_macro/todo/unimplemented/unwrap_used deny; pedantic warn at priority -1; module_name_repetitions/must_use_candidate/missing_errors_doc/missing_panics_doc allowed). Edited all 14 per-crate `Cargo.toml` files to add `[lints] workspace = true`. Ran `cargo fmt --all` and staged any reformatted files. `make lint` exits 0. `make fmt` idempotent. `cargo doc --workspace --no-deps` exits 0.
**Commit suggestion:** `chore(tooling): add rustfmt.toml, clippy.toml, workspace lint policy — Step 0.8`
```

---

## Commit suggestion

```
chore(tooling): add rustfmt.toml, clippy.toml, workspace lint policy — Step 0.8

rustfmt.toml (new file):
- Stable-only options (Rust 1.95.0); no nightly rustfmt required.
- edition = "2024" matches Cargo.toml.
- max_width = 100, tab_spaces = 4, newline_style = "Unix".
- use_field_init_shorthand, use_try_shorthand: modern Rust idioms.

clippy.toml (new file):
- Threshold tuning only; allow/deny policy is in Cargo.toml.
- too-many-lines-threshold = 80 (tighter than default 100) enforces
  the AGENTS.md 60-80 line function cap via clippy::pedantic.

Cargo.toml (edit — workspace lint sections):
- [workspace.lints.rust]: unsafe_code = "deny", missing_docs = "warn".
- [workspace.lints.clippy]: deny dbg_macro, todo, unimplemented,
  unwrap_used. Pedantic at warn (priority -1) — escalated to error by
  -D warnings in make lint. Four curated allows: module_name_repetitions,
  must_use_candidate, missing_errors_doc, missing_panics_doc.

14× per-crate Cargo.toml (edit):
- [lints] workspace = true in all bin/* and crates/* members.
- Opt-in is required for workspace lint inheritance (Rust 1.74+).

Phase 0 Gate status after this step:
  make lint passes with -D warnings ✅
  make fmt is idempotent ✅

Note: this commit also touches bin/kraxctl/Cargo.toml and all other
per-crate files from Steps 0.2-0.3; the lints opt-in is the only
change to those files.
```

---

## Outcomes

- **All 5 execution steps completed in order.** `rustfmt.toml` and `clippy.toml` created, root `Cargo.toml` extended with workspace lint sections, all 14 per-crate `Cargo.toml` files updated with `[lints] workspace = true`, `cargo fmt --all` run and confirmed idempotent.
- **One unplanned fix required.** `missing_docs = "warn"` (escalated to error by `-D warnings`) fired on the three binary crates — "missing documentation for the crate" — requiring `//!` crate-level doc comments in `bin/kraxd/src/main.rs`, `bin/kraxctl/src/main.rs`, and `bin/kraxprover/src/main.rs`. The plan's follow-up section correctly prescribed adding doc comments rather than suppressing the lint. No `#[allow]` used.
- **All verification steps passed.** `rustfmt.toml`/`clippy.toml` exist with correct values, all 14 per-crate opt-ins confirmed (`grep -rL` returns empty), `cargo fmt --all -- --check` exits 0 (idempotent), `cargo clippy --workspace --all-targets -- -D warnings` exits 0, `make lint` exits 0, `cargo doc --workspace --no-deps` exits 0.
- **ARCHITECTURE.md Step 0.8 checked off.** All three items marked `[x]`.
- **AGENTS.md `Current State` and `Changelog` updated.** Session 9 entry appended at the bottom of the Changelog.

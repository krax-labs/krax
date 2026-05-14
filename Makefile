SHELL := bash

.PHONY: help build test test-integration lint run fmt clean coverage

help:
	@echo "Krax — available make targets:"
	@echo ""
	@echo "  build            cargo build --workspace --release"
	@echo "  test             cargo test --workspace"
	@echo "  test-integration cargo test --workspace --features integration"
	@echo "  lint             cargo clippy --workspace --all-targets -- -D warnings"
	@echo "  coverage         cargo llvm-cov --workspace --features integration --fail-under-lines 85 (HTML at target/llvm-cov/html/)"
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

coverage:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || { \
		echo "cargo-llvm-cov is not installed."; \
		echo "Install with: cargo install cargo-llvm-cov   (or, on macOS: brew install cargo-llvm-cov)"; \
		exit 1; \
	}
	@$(MAKE) build
	@cargo llvm-cov --workspace --features integration --html --ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs' --fail-under-lines 85
	@cargo llvm-cov report --ignore-filename-regex 'crates/krax-types/src/(block|tx|journal_entry)\.rs|crates/krax-state/src/mpt/slots\.rs'
	@echo "HTML report: target/llvm-cov/html/index.html"

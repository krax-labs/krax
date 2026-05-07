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

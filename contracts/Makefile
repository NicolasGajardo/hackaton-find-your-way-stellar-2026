.PHONY: check build test fmt fmt-check lint ci clean

export CARGO_TARGET_DIR := $(CURDIR)/target

check:
	cargo check

build:
	stellar contract build

test:
	cargo test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets -- -D warnings

ci: fmt-check lint test build

clean:
	cargo clean

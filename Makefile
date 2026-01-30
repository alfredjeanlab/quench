.PHONY: check build test install clean bench bench-ci bench-baseline bench-check license

# Run all CI checks
check:
	cargo fmt --all
	cargo clippy --all-targets --all-features -- -D warnings
	cargo build --all
	cargo test --all
	./target/debug/quench check
	cargo audit
	cargo deny check licenses bans sources
	@if [ -d tests/fixtures/bench-rust ] && [ -f target/release/quench ]; then \
		timeout 5s ./target/release/quench check tests/fixtures/bench-rust >/dev/null 2>&1 \
		|| (echo "Performance smoke test failed"; exit 1); \
	fi

# Build release binary
build:
	cargo build --release

# Run tests
test:
	cargo test --all

# Install to ~/.local/bin
install:
	@./scripts/install

# Clean build artifacts
clean:
	cargo clean

# Run benchmarks
bench:
	cargo bench --bench baseline
	cargo bench --bench file_walking
	cargo bench --bench check
	cargo bench --bench tests

# Run benchmarks with CI tracking
bench-ci:
	./scripts/bench-ci

# Save benchmark baseline for regression detection
bench-baseline:
	cargo bench --bench adapter -- --save-baseline main
	cargo bench --bench stress -- --save-baseline main

# Compare benchmarks against baseline
bench-check:
	cargo bench --bench adapter -- --baseline main --noplot
	cargo bench --bench stress -- --baseline main --noplot

# Check and fix license headers
license:
	cargo build --release
	./target/release/quench check --ci --license --fix

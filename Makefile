.PHONY: check build test install clean bench bench-ci bench-baseline bench-check

# Run all CI checks
check:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all
	cargo build --all
	./scripts/bootstrap
	./target/debug/quench check
	cargo audit
	cargo deny check licenses bans sources

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

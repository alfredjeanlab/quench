.PHONY: check build test install clean bench bench-ci

# Run all CI checks
check:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all
	cargo build --all
	./scripts/bootstrap
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

.PHONY: check build test install clean

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

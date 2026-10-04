fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged

check:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test

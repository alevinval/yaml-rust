
lint:
	cargo +nightly fmt
	cargo check
	cargo clippy

test:
	cargo test

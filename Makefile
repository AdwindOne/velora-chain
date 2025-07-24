test:
	cd velora-chain && cargo test --test integration -- --nocapture

build:
	cd velora-chain && cargo build --release

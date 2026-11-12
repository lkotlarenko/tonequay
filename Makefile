.PHONY: studio studio-release studio-test studio-appimage
studio:
	cargo run --locked -p fm1-studio
studio-release:
	cargo build --locked --release -p fm1-studio
studio-test:
	cargo fmt -p fm1-studio --check
	cargo clippy -p fm1-studio --all-targets --no-deps -- -D warnings
	cargo test --locked --workspace
	node --test web/*.test.mjs
studio-appimage:
	./tools/build_studio_appimage.sh

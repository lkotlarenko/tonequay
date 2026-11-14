.PHONY: studio studio-release studio-test studio-appimage
studio:
	cargo run --locked -p tonequay
studio-release:
	cargo build --locked --release -p tonequay
studio-test:
	cargo fmt -p tonequay --check
	cargo clippy -p tonequay --all-targets --no-deps -- -D warnings
	cargo test --locked --workspace
	node --test web/*.test.mjs
studio-appimage:
	./tools/build_appimage.sh

.PHONY: run release test appimage
run: studio
release: studio-release
test: studio-test
appimage: studio-appimage

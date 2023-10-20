# RUSTFLAGS=--cfg=web_sys_unstable_apis cargo r --target wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown

wasm-bindgen --out-dir ./web/bin/ --target web ~/.cargo/target/wasm32-unknown-unknown/debug/pumpkin-game.wasm

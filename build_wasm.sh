cargo build --target wasm32-unknown-unknown

echo "Running wasm-bindgen..."
wasm-bindgen --out-dir ./web/play/pumpkin-game/bin/ --target web ~/.cargo/target/wasm32-unknown-unknown/debug/pumpkin-game.wasm

echo "Copying files from ./src/web/..."
cp -r ./src/web/* ./web-build/

echo "Find/Replacing placeholder bits..."
VERSION=$(date +%s)

sed -i 's/pumpkin-game_bg\.wasm/pumpkin-game_opt.wasm/g' ./web-build/play/pumpkin-game/bin/pumpkin-game.js
sed -i "s/##VERSION##/$VERSION/g" ./web-build/play/pumpkin-game/sw.js
sed -i "s/##VERSION##/$VERSION/g" ./web-build/play/pumpkin-game/index.html

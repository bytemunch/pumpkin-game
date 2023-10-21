cargo build --release --target wasm32-unknown-unknown

echo "Running wasm-bindgen..."
wasm-bindgen --out-dir ./web-build/play/pumpkin-game/bin/ --target web ~/.cargo/target/wasm32-unknown-unknown/release/pumpkin-game.wasm

echo "Optimising (-O3)..."
wasm-opt -O3 ./web-build/play/pumpkin-game/bin/pumpkin-game_bg.wasm -o ./web-build/play/pumpkin-game/bin/pumpkin-game_opt.wasm

echo "Removing unneeded files..."
rm ./web-build/play/pumpkin-game/bin/pumpkin-game.d.ts
rm ./web-build/play/pumpkin-game/bin/pumpkin-game_bg.wasm
rm ./web-build/play/pumpkin-game/bin/pumpkin-game_bg.wasm.d.ts

echo "Copying files from ./src/web/..."
cp -r ./src/web/* ./web-build/

echo "Find/Replacing placeholder bits..."

VERSION=$(grep -oP "version = \K\".+?\"$" ./Cargo.toml)

sed -i 's/pumpkin-game_bg\.wasm/pumpkin-game_opt.wasm/g' ./web-build/play/pumpkin-game/bin/pumpkin-game.js
sed -i "s/##VERSION##/$VERSION/g" ./web-build/play/pumpkin-game/sw.js
sed -i "s/##VERSION##/$VERSION/g" ./web-build/play/pumpkin-game/index.html

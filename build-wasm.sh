rm -rf out
cargo build --release --target wasm32-unknown-unknown
mkdir out
cp assets/index.html out/
cp -r assets out/
wasm-bindgen --no-typescript --out-name teahouse --out-dir ./out/ --target web ./target/wasm32-unknown-unknown/release/teahouse.wasm

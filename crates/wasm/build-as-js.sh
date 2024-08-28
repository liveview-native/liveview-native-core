#!/bin/sh

set -ex

# Compile our wasm module and run `wasm-bindgen`
#cargo build --target wasm32-unknown-unknown -p liveview_native_core_wasm
#wasm-bindgen  --out-dir ./pkg --no-typescript --debug ../../target/wasm32-unknown-unknown/debug/liveview_native_core_wasm.wasm
if true; then
  wasm-pack build --no-typescript --target nodejs
else
  wasm-pack build --no-typescript
  wasm2es6js --base64 pkg/liveview_native_core_wasm_bg.wasm -o pkg/liveview_native_core_wasm_bg.wasm.js

  # Update our JS shim to require the JS file instead
  gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' pkg/liveview_native_core_wasm.js
  gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' pkg/liveview_native_core_wasm_bg.wasm.js
  gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' pkg/package.json
fi

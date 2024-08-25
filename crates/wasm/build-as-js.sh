#!/bin/sh

set -ex

# Compile our wasm module and run `wasm-bindgen`
wasm-pack build --out-dir ./pkg/

# Run the `wasm2js` tool from `binaryen`
#wasm2js pkg/liveview_native_core_wasm_bg.wasm -o pkg/liveview_native_core_wasm_bg.wasm.js
wasm2es6js --base64 pkg/liveview_native_core_wasm_bg.wasm -o pkg/liveview_native_core_wasm_bg.wasm.js

# Update our JS shim to require the JS file instead
gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' pkg/liveview_native_core_wasm.js
gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' pkg/liveview_native_core_wasm_bg.wasm.js
gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' pkg/package.json

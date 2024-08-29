#!/bin/sh

set -ex
TARGET=$1

if [ $TARGET = "web" ] ; then
  wasm-pack build --no-typescript --out-dir ./liveview-native-core-wasm-web
  wasm2es6js --base64 liveview-native-core-wasm-web/liveview_native_core_wasm_bg.wasm -o ./liveview-native-core-wasm-web/liveview_native_core_wasm_bg.wasm.js

  # Update our JS shim to require the JS file instead
  gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' liveview-native-core-wasm-web/liveview_native_core_wasm.js
  gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' liveview-native-core-wasm-web/liveview_native_core_wasm_bg.wasm.js
  gsed -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' liveview-native-core-wasm-web/package.json
fi
if [ $TARGET = "nodejs" ] ; then
  wasm-pack build --no-typescript --target nodejs --out-dir ./liveview-native-core-wasm-nodejs
fi

#!/bin/sh

set -ex
TARGET=$1

SED=sed
if [ $(uname) = "Darwin" ]; then
    SED=gsed
fi

if [ $TARGET = "web" ] ; then
    wasm-pack build --no-typescript --out-dir ./liveview-native-core-wasm-web
    wasm2es6js --base64 liveview-native-core-wasm-web/liveview_native_core_wasm_bg.wasm -o ./liveview-native-core-wasm-web/liveview_native_core_wasm_bg.wasm.js

    # Update our JS shim to require the JS file instead
    ${SED} -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' liveview-native-core-wasm-web/liveview_native_core_wasm.js
    ${SED} -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' liveview-native-core-wasm-web/liveview_native_core_wasm_bg.wasm.js
    ${SED} -i 's/liveview_native_core_wasm_bg.wasm/liveview_native_core_wasm_bg.wasm.js/' liveview-native-core-wasm-web/package.json
    jq '.files += ["snippets/*"]' liveview-native-core-wasm-web/package.json > tmp.json && mv tmp.json ./liveview-native-core-wasm-web/package.json
    npm pack ./liveview-native-core-wasm-web
    mv liveview_native_core_wasm*tgz ./liveview-native-core-wasm-web.tgz
elif [ $TARGET = "nodejs" ] ; then
    wasm-pack build --no-typescript --target nodejs --out-dir ./liveview-native-core-wasm-nodejs
    jq '.files += ["snippets/*"]' liveview-native-core-wasm-nodejs/package.json > tmp.json && mv tmp.json ./liveview-native-core-wasm-nodejs/package.json
    npm pack ./liveview-native-core-wasm-nodejs/
    mv liveview_native_core_wasm*tgz ./liveview-native-core-wasm-nodejs.tgz
else
    echo "Either `web` or `nodejs` must be specified as the first argument"
    exit 1
fi

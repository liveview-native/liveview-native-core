#!/bin/sh
set -euo pipefail

initial_dir=$(pwd)
cleanup() {
  cd "$initial_dir"
  echo "Cleaning up..."
  #rm -rf temp_test
}

checkout_latest_tag() {
    git fetch --tags
    git checkout "$(git describe --tags "$(git rev-list --tags --max-count=1)")"
}

trap cleanup ERR

rm -rf temp_test
mkdir -p temp_test
cd temp_test

git clone https://github.com/phoenixframework/phoenix_live_view
cd phoenix_live_view/assets && checkout_latest_tag
npm install ../../../liveview-native-core-wasm-nodejs

# shim our classes into the jest tests
cp ../../../npm_shims/jest_mock.js .
#npm test -- --setupFilesAfterEnv='./jest_mock.js'

# run playwright tests
# TODO: Shim our wasm into the playwright build
cd ..
cp ../../npm_shims/mock.esbuild.mjs .
mix deps.get
# This script produces the esm module, which is the module used in the playwright
# tests but subs out the client classes for our WASM based ones.
npm install esbuild
node mock.esbuild.mjs
cd test/e2e && npx playwright install && npx playwright test

cleanup

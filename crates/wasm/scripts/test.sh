#!/bin/sh
set -euo pipefail

initial_dir=$(pwd)
cleanup() {
  cd "$initial_dir"
  echo "Cleaning up..."
  rm -rf temp_test
}

trap cleanup ERR

rm -rf temp_test
mkdir -p temp_test
cd temp_test

git clone https://github.com/phoenixframework/phoenix_live_view
cd phoenix_live_view/assets && npm install ../../../liveview-native-core-wasm-nodejs

# shim our classes into the jest tests
cp ../../../npm_scripts/jest_mock.js .
npm test -- --setupFilesAfterEnv='./jest_mock.js'

# run playwright tests
# TODO: Shim our wasm into the playwright build
cd ..
npm run setup
npm run e2e:test

cleanup

#!/bin/sh
set -euo pipefail


initial_dir=$(pwd)
cleanup() {
  cd "$initial_dir"
  echo "Cleaning up..."
  # CODEREVIEW: this should not make it to the PR, uncomment below
  rm -rf phoenix_live_view
}

checkout_latest_tag() {
    git fetch --tags
    git checkout "$(git describe --tags "$(git rev-list --tags --max-count=1)")"
}

trap cleanup ERR

if [ ! -d "phoenix_live_view" ]; then
    git clone https://github.com/phoenixframework/phoenix_live_view
fi

cd phoenix_live_view/assets && checkout_latest_tag
npm install ../../liveview-native-core-wasm-nodejs
cp ../../npm_shims/jest_mock.js .

# shim our classes into the jest tests
npm test -- --setupFilesAfterEnv='./jest_mock.js'
cleanup

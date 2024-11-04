#!/bin/sh
set -e


initial_dir=$(pwd)
cleanup() {
  cd "$initial_dir"
  echo "Cleaning up..."
  rm -rf phoenix_live_view
}

checkout_latest_tag() {
    git fetch --tags
    git checkout "$(git describe --tags "$(git rev-list --tags --max-count=1)")"
}

if [ $(uname) == "Darwin" ]; then
    trap cleanup ERR
fi

if [ ! -d "phoenix_live_view" ]; then
    git clone https://github.com/phoenixframework/phoenix_live_view
fi

cd phoenix_live_view/assets && checkout_latest_tag
npm install ../../liveview-native-core-wasm-nodejs
cp ../../npm_shims/jest_mock.js .

# shim our classes into the jest tests
# if you need to filter tests for iteration you can add the -t argument.
npm test -- --setupFilesAfterEnv='./jest_mock.js' -t "merges the latter"
# npm test -- --setupFilesAfterEnv='./jest_mock.js'
cleanup

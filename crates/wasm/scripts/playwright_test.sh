#!/bin/sh
set -euo pipefail


# CODEREVIEW: this should not make it to the PR
# Make this test use fewer relative paths

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

trap cleanup ERR

if [ ! -d "phoenix_live_view" ]; then
    git clone https://github.com/phoenixframework/phoenix_live_view
fi

cd phoenix_live_view && checkout_latest_tag

cp ../npm_shims/mock_esbuild.mjs js/phoenix_live_view
cd js/phoenix_live_view
npm install ../../../liveview-native-core-wasm-web

npm install esbuild
# This script produces the esm module, which is the module used in the playwright
# tests but subs out the client classes for our WASM based ones.
node mock_esbuild.mjs
cd ..
mix deps.get
cd test/e2e && npx playwright install && npx playwright test

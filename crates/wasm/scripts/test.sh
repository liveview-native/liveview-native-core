#!/bin/sh
set -euo pipefail


# CODEREVIEW: this should not make it to the PR
# we need to make this script cd less, use fewer
# relative directory paths.

initial_dir=$(pwd)
cleanup() {
  cd "$initial_dir"
  echo "Cleaning up..."
  # CODEREVIEW: this should not make it to the PR, uncomment below
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
# CODEREVIEW: this should not make it to the PR, uncomment below
#npm test -- --setupFilesAfterEnv='./jest_mock.js'

# run playwright tests
cd js/phoenix_live_view
cp ../../../../../npm_shims/mock_esbuild.mjs .
# This script produces the esm module, which is the module used in the playwright
# tests but subs out the client classes for our WASM based ones.
npm install esbuild
node mock_esbuild.mjs
cd ..
mix deps.get
cd test/e2e && npx playwright install && npx playwright test

cleanup

#!/bin/sh
set -e

# move to the root of the wasm directory
script_dir=$(dirname "$0")
cd "$script_dir/.."

# The first argument is interpreted as a quoted jest filter
# for example: "merges the latter"
if [ -z "$1" ]; then
  filter_arg="-t .*"
else
  filter_arg="-t $1"
fi

# set up a deferred cleanup hook
initial_dir=$(pwd)
cleanup() {
  cd "$initial_dir"
  echo "Cleaning up..."
  #rm -rf phoenix_live_view
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
# npm test -- --setupFilesAfterEnv='./jest_mock.js' -t "merges the latter"
npm test -- --setupFilesAfterEnv='./jest_mock.js' "$filter_arg"
cleanup

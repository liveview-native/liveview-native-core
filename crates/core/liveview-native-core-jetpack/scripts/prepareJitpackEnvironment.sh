# Installing Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# After install Rust, need to the environment variables
source "$HOME/.cargo/env"
# Rust code is using nigthly channel
rustup install nightly
rustup default nightly
# Adding the target architectures to compile Rust code
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add aarch64-linux-android
rustup target add x86_64-linux-android
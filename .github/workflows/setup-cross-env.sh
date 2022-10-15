#!/bin/sh -eux
setup_cross_env_linux() {
  RUST_TARGET="$1"
  DEB_ARCH="$2"
  GCC_ARCH="$3"

  dpkg --add-architecture "$DEB_ARCH"
  apt-get update
  apt-get install -y \
      "libc6-dev:$DEB_ARCH" \
      "libdbus-1-dev:$DEB_ARCH"

  mkdir -p "$HOME/.cargo"
  cat >> "$HOME/.cargo/config" <<EOF
[target.${RUST_TARGET}]
linker = "${GCC_ARCH}-gcc"

[target.${RUST_ARCH}.dbus]
rustc-link-lib = ["dbus-1"]
rustc-link-search = ["/lib/${GCC_ARCH}", "/usr/lib/${GCC_ARCH}"]
EOF
}

case "$1" in
  "aarch64-unknown-linux-gnu") setup_cross_env_linux "$1" arm64 aarch64-linux-gnu;;
  "armv7-unknown-linux-gnueabihf") setup_cross_env_linux "$1" armhf armv7-linux-gnueabihf;;
  "x86_64-unknown-linux-gnu")
    apt-get install -y libdbus-1-dev;;
  "x86_64-pc-windows-gnu")
    apt-get install -y gcc-mingw-w64-x86-64;;
esac

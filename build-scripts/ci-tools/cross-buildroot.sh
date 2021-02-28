#!/bin/bash
set -eu

DEB_ARCH=$1
RUST_ARCH=$2

dpkg --add-architecture $DEB_ARCH
apt-get update
apt-get install -y \
    libc6-dev:$DEB_ARCH \
    libgcc-8-dev:$DEB_ARCH \
    libdbus-1-dev:$DEB_ARCH

mkdir -p /.cargo
cat >> /.cargo/config <<EOF
[target.${RUST_ARCH}.dbus]
rustc-link-lib = ["dbus-1"]

EOF

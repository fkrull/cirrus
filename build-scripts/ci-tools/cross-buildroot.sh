#!/bin/bash
set -eu

DEB_ARCH=$1
GCC_ARCH=$2
RUST_ARCH=$3

dpkg --add-architecture $DEB_ARCH
apt-get update
apt-get install -y \
    libc6-dev:$DEB_ARCH \
    libgcc-8-dev:$DEB_ARCH \
    libdbus-1-dev:$DEB_ARCH

ln -s libgcc_s.so.1 /lib/$GCC_ARCH/libgcc_s.so

mkdir -p /.cargo
cat >> /.cargo/config <<EOF
[target.${RUST_ARCH}]
linker = "${GCC_ARCH}-gcc"

[target.${RUST_ARCH}.dbus]
rustc-link-lib = ["dbus-1"]
rustc-link-search = ["/lib/${GCC_ARCH}", "/usr/lib/${GCC_ARCH}"]

EOF

#!/bin/sh
set -eu

tmp=$(mktemp -d)
cleanup() {
  cd /
  rm -rf $tmp
}
trap cleanup EXIT

QEMU_URL=$1
QEMU_SHA256=$2
QEMU_OUT_BIN=$3

cd $tmp
curl -L -o qemu.tgz $QEMU_URL
echo "$QEMU_SHA256 *qemu.tgz" | sha256sum -c
tar xaf qemu.tgz
install qemu-*/qemu-*-static $QEMU_OUT_BIN

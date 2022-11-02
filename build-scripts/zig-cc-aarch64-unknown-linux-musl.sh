#!/bin/sh -eu
exec zig cc -target aarch64-linux-musl "$@"

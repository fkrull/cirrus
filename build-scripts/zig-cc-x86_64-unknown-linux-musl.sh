#!/bin/sh -eu
exec zig cc -target x86_64-linux-musl "$@"

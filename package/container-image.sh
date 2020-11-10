#!/bin/sh
set -eu

BASE_IMAGE=$1
QEMU_USER=${2-}

IMAGE_NAME=cirrus-container-build

function buildah_run {
  if [ -n "$QEMU_USER" ]; then
    buildah run -v $PWD:/qemu $1 -- /qemu/$QEMU_USER --execve /bin/sh -c "$2"
  else
    buildah run $1 -- /bin/sh -c "$2"
  fi
}

ctr=$(buildah from "$BASE_IMAGE")

chmod 0755 restic cirrus
buildah copy $ctr restic cirrus /usr/bin/

buildah_run $ctr "apk add --no-cache ca-certificates openssh-client"
buildah_run $ctr "mkdir -p /cache /config/cirrus"
buildah config --env XDG_CONFIG_HOME=/config $ctr
buildah config --env XDG_CACHE_HOME=/cache $ctr
buildah config --entrypoint /usr/bin/cirrus $ctr
buildah config --volume /cache $ctr

buildah commit $ctr $IMAGE_NAME

echo $IMAGE_NAME

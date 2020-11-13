#!/bin/sh
set -eu

tempdir=$(mktemp -d)
function cleanup {
  rm -rf $tempdir
}
trap cleanup EXIT

export PACKAGEDIR=$1
export APPX_ARCH=$2
export APPX_VERSION=$3
export CERTIFICATE=$4

envsubst < package/windows/AppxManifest.xml > $PACKAGEDIR/AppxManifest.xml
makemsix pack -d $PACKAGEDIR -p $tempdir/validate.appx
appx -o Cirrus.appx -c $CERTIFICATE -9 $PACKAGEDIR

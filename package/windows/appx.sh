#!/bin/sh
set -eu

tempdir=$(mktemp -d)
function cleanup {
  rm -rf $tempdir
}
trap cleanup EXIT

PACKAGEDIR=$1
APPX_ARCH=$2
APPX_VERSION=$3
CERTIFICATE=$4

envsubst package/windows/AppxManifest.xml | $PACKAGEDIR/AppxManifest.xml
makemsix -d $PACKAGEDIR -p $tempdir/validate.appx
appx -o Cirrus.appx -c $CERTIFICATE -9 $PACKAGEDIR

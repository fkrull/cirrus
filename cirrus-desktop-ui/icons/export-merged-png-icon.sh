#!/bin/bash
set -eu

TMP=$(mktemp --directory)
function cleanup {
  rm -rf "$TMP"
}
trap cleanup EXIT

SVG=$1
PNG=$2
SIZE=$3
OBJECTS=$4

mkdir -p "$(dirname $PNG)"

png_files=""

for object in $OBJECTS; do
  filename="$TMP/$object.png"
  inkscape \
    "$SVG" \
    --export-filename="$filename" \
    --export-width="$SIZE" --export-height="$SIZE" \
    --export-area-page \
    --export-id="$object" --export-id-only
  png_files="$png_files $filename"
done

convert -background transparent $png_files -layers flatten "$PNG"

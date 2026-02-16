#!/bin/bash
# tfl wrapper for xdg-desktop-portal-termfilechooser
# Args: $1=multiple $2=directory $3=save $4=path $5=out_file $6=debug
set -euo pipefail

multiple="$1"
directory="$2"
save="$3"
path="$4"
out="$5"

if [ "$save" = "1" ]; then
  dir="$(dirname "$path")"
  tfl --chooser-file="$out" "$dir"
elif [ "$directory" = "1" ]; then
  tfl --chooser-file="$out" "${path:-.}"
else
  tfl --chooser-file="$out" "${path:-.}"
fi

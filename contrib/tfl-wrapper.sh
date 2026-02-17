#!/bin/bash
# tfl wrapper for xdg-desktop-portal-termfilechooser
#
# NOTE: This is a template. The `tfl --install-portal` command generates a
# version with absolute paths for your system. If installing manually, replace
# the TFL and TERM_EMU variables with absolute paths (e.g. /usr/bin/ghostty,
# /home/you/.cargo/bin/tfl) — systemd services don't inherit your shell PATH.
#
# Args: $1=multiple $2=directory $3=save $4=path $5=out_file $6=debug
set -euo pipefail

multiple="$1"
directory="$2"
save="$3"
path="$4"
out="$5"

TFL="/path/to/tfl"
TERM_EMU="/path/to/terminal"

if [ "$save" = "1" ]; then
  dir="$(dirname "$path")"
  $TERM_EMU -e "$TFL" --chooser-file="$out" "$dir"
elif [ "$directory" = "1" ]; then
  $TERM_EMU -e "$TFL" --chooser-file="$out" "${path:-.}"
else
  $TERM_EMU -e "$TFL" --chooser-file="$out" "${path:-.}"
fi

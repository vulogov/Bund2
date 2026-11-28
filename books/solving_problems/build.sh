#!/usr/bin/env bash
# Build "Bund, Problem by Problem".
#
#   solving_problems.pdf       — print master (full-resolution images)
#   solving_problems-web.pdf   — web/email build (150-DPI images), with --web
#
# Usage:  ./build.sh            re-record the screens, then typeset
#         ./build.sh --fast     typeset only, from the screens already recorded
#         ./build.sh --web      also write the web-optimized copy
# Fonts:  the text face, Kyiv Machine, is read from the first book's assets
# Needs:  typst, python3, a built target/release/bund2 (or $BUND2);
#         ghostscript (gs) for --web

set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd ../.. && pwd)"

SRC="books/solving_problems/solving_problems.typ"
PRINT="books/solving_problems/solving_problems.pdf"
WEB="books/solving_problems/solving_problems-web.pdf"
DPI="${DPI:-150}"

fast=0; web=0
for arg in "$@"; do
  case "$arg" in
    --fast) fast=1 ;;
    --web)  web=1 ;;
    *) echo "usage: ./build.sh [--fast] [--web]" >&2; exit 2 ;;
  esac
done

command -v typst >/dev/null || { echo "error: typst not found on PATH" >&2; exit 1; }

if [ "$fast" -eq 0 ]; then
  echo "→ recording the screens …"
  python3 tools/shoot.py >/dev/null
fi

echo "→ compiling $SRC …"
( cd "$ROOT" && typst compile --root . --font-path books/Introduction/assets "$SRC" "$PRINT" )

if [ "$web" -eq 1 ]; then
  command -v gs >/dev/null || { echo "error: ghostscript (gs) not found on PATH" >&2; exit 1; }
  echo "→ web-optimizing → $WEB (images @ ${DPI} DPI) …"
  ( cd "$ROOT" && gs -sDEVICE=pdfwrite -dCompatibilityLevel=1.6 -dNOPAUSE -dBATCH -dQUIET \
     -dPDFSETTINGS=/ebook \
     -dDownsampleColorImages=true -dColorImageResolution="$DPI" -dColorImageDownsampleType=/Bicubic \
     -dDownsampleGrayImages=true  -dGrayImageResolution="$DPI"  -dGrayImageDownsampleType=/Bicubic \
     -dDetectDuplicateImages=true -dCompressFonts=true -dSubsetFonts=true \
     -o "$WEB" "$PRINT" )
fi

pages=$(pdfinfo "$ROOT/$PRINT" 2>/dev/null | awk '/Pages/{print $2}' || echo "?")
printf '\n✓ done — %s pages\n' "$pages"

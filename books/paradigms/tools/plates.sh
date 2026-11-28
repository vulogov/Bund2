#!/usr/bin/env bash
# Turn the engravings in assets/*.jpg into transparent PNGs.
#
#   tools/plates.sh              all plates
#   tools/plates.sh 08-manifest  one plate
#
# The paper becomes transparent and the ink stays: each pixel's darkness
# becomes its opacity, over a single near-black ink colour. The white point
# (88%) sits just under the paper tone of the scans, so the paper clears.
set -euo pipefail
cd "$(dirname "$0")/../assets"

if [ $# -gt 0 ]; then set -- "${@/%/.jpg}"; else set -- *.jpg; fi

for f in "$@"; do
  size=$(magick identify -format '%wx%h' "$f")
  magick "$f" -colorspace Gray -level 5%,88% -negate -write mpr:ink +delete \
    -size "$size" xc:'#111111' mpr:ink -alpha off \
    -compose CopyOpacity -composite \
    -define png:compression-level=9 "${f%.jpg}.png"
  echo "✓ ${f%.jpg}.png"
done

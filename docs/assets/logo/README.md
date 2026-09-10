# The Bund mark

| file | what it is |
|---|---|
| `bund2-logo-source.jpg` | what the repository owner supplied, 1241×1201, untouched |
| `bund2-logo.png` | the same image, lossless, for anywhere a JPEG is awkward |
| `bund2-logo-transparent.png` | black ink on alpha, for dark and coloured backgrounds |
| `bund2-logo.svg` | vector, 41 KB, `fill="currentColor"` so it inherits text colour |
| `trace.py` | regenerates the SVG from the transparent PNG |

## The two derivatives, and why they are not the obvious thing

**Transparency is taken from luminance, not by keying out white.** The source is
a JPEG, so its flat white is not uniformly `#ffffff` and its edges carry ringing;
`white → transparent` leaves a grey halo that only shows up once someone puts the
mark on a dark background. Instead `alpha = 1 − luminance`, levelled so that
below 0.06 is fully clear and above 0.55 fully opaque. The ink is pure black at
every pixel, and 0.6% of the image is partial alpha — that 0.6% is the
antialiased edge, which is what it should be.

**The SVG was traced, then checked against the raster rather than by eye.** There
is no `potrace` on the machine this was made on, so `trace.py` does marching
squares for subpixel contours, splits each contour at genuine corners, and fits
cubic Béziers to the runs between them with a 0.25 px error bound.

The bound is the point. Two earlier attempts looked plausible and were not:

| attempt | disagreeing pixels | thicker than a hairline |
|---|---|---|
| Catmull-Rom through simplified points | 0.56% | 1371 px, worst blob 383 px |
| least squares, control points unclamped | 21.73% | 292012 px |
| **least squares, clamped, 0.25 px bound** | **0.27%** | **0 px** |

"0 px thicker than a hairline" is the check that matters: rasterise the SVG at
1241×1201, XOR it against the source mask, erode by one pixel, and count what
survives. Nothing does, so the two shapes differ only where any vectoriser
differs — along the antialiased boundary itself.

To regenerate:

    python3 docs/assets/logo/trace.py     # needs numpy, scikit-image, Pillow

## Using it

The SVG paints with `currentColor`, so it takes the surrounding text colour and
needs no light and dark variants:

```html
<span style="color:#111"><!-- inline the svg here --></span>
```

Both PNGs are black ink. On a dark background use `bund2-logo-transparent.png`
and invert, or prefer the SVG.

# Shape Effects and Meta Props

Three files work together:

- **shapes-effects.sh** — Shell script that builds the deck (generates a sample PNG inline via Python heredoc, no external image needed).
- **shapes-effects.pptx** — The generated 5-slide deck.
- **shapes-effects.md** — This file.

This trio fills in everything that `shapes-basic` and `shapes-connectors`
didn't touch: text overflow behavior, mirror flags, image-as-fill, full
3D scene, soft edges, click hyperlinks on the shape itself, and z-order.

## Regenerate

```bash
cd examples/ppt
bash shapes/shapes-effects.sh
# → shapes/shapes-effects.pptx
```

## Slides

### Slide 1 — `autoFit` (text overflow behavior)

Three identical boxes with the same long text, same size, same width.
Only `autoFit=` differs:

| Value | What happens |
|---|---|
| `none` | Text overflows the box vertically |
| `normal` | Text shrinks until it fits |
| `shape` | Box grows until text fits |

```bash
--prop autoFit=normal   # shrink-to-fit
--prop autoFit=shape    # grow-to-fit
--prop autoFit=none     # overflow
```

Set-only aliases: `true`/`shrink` → `normal`, `resize` → `shape`,
`false` → `none`.

### Slide 2 — `flipH` / `flipV` (mirror)

Four `geometry=rightArrow` shapes: original, `flipH`, `flipV`, both.
Flip flags are independent of `rotation=` — combining them composes
predictably (`flipH=true + rotation=90` rotates the mirrored arrow,
not a non-mirrored one).

```bash
--prop flipH=true     # alias: flipHorizontal
--prop flipV=true     # alias: flipVertical
```

### Slide 3 — `image=` (picture as shape fill)

Three preset geometries (`ellipse`, `star5`, `diamond`) all filled with
the same PNG. The geometry **clips** the image; the bitmap doesn't drive
the bounding box.

```bash
officecli add file.pptx /slide[3] --type shape --prop geometry=star5 \
  --prop x=4.5in --prop y=1.5in --prop width=3.5in --prop height=3.5in \
  --prop image=/path/to/photo.png
```

> 📌 This is **different from `--type picture`**:
> - `--type picture` embeds the image with its native aspect inside a
>   rectangular DrawingML picture frame.
> - `--type shape --prop image=...` uses a shape preset's geometry as
>   the clipping outline (DrawingML blipFill on `<p:sp>`). Outline,
>   bevel, shadow, etc. all apply to the shape silhouette, not the
>   image bounds.

Aliases: `imagefill`. Get readback surfaces as `image: "true"` (presence
flag, not the path).

### Slide 4 — 3-D (bevel / depth / lighting / material)

Six shapes showing the four 3D props composed:

| Spec | Effect |
|---|---|
| `bevel=circle` | top bevel, default size (6×6 pt) |
| `bevel=angle-8-4` | top bevel, 8pt wide × 4pt high |
| `bevelBottom=circle-4-4` | bottom-face bevel |
| `depth=14pt` | extrusion height — flat shape becomes a 3D solid |
| `lighting=threePt` | scene light rig preset |
| `material=metal` | surface material preset |

Available `bevel` presets: `angle`, `artDeco`, `circle`, `convex`,
`coolSlant`, `cross`, `divot`, `hardEdge`, `relaxedInset`, `riblet`,
`slope`, `softRound`.

Available `lighting` rigs: `threePt`, `balanced`, `soft`, `harsh`,
`flood`, `contrasting`, `morning`, `sunrise`, `sunset`, `chilly`,
`freezing`, `flat`, `twoPt`, `glow`, `brightRoom`.

Available `material` presets: `clear`, `darkEdge` (alias `dkEdge`),
`flat`, `matte`, `metal`, `plastic`, `powder`, `softEdge`, `softMetal`,
`translucentPowder`, `warmMatte`, `wireframe` (alias `wire`).

### Slide 5 — `softEdge`, `link` + `tooltip`, `name`, `zorder`

**`softEdge=`** — feathered/blurred edge in points (`0` = sharp,
larger = heavier feather):

```bash
--prop softEdge=8pt    # bare number also accepted (interpreted as pt)
--prop softEdge=none   # clear
```

**`link=` + `tooltip=` on a shape** — the entire shape becomes
clickable (just like the picture demo, but on a shape):

```bash
--prop link=https://example.com --prop tooltip="Open homepage"
```

Same target grammar as picture: URLs, `slide[N]`, named actions
(`nextslide`, `firstslide`, …), `mailto:`.

**`name=`** — overrides the auto-generated `Shape {id}` label on
`cNvPr@name`. Useful for `--prop name="cta-button"` so a later
`query` or `set` can target the shape by `@name=cta-button` instead
of guessing the index.

**`zorder=`** — three overlapping rectangles with explicit stack
position (1 = back). Aliases: `z-order`, `order`. On Add/Set the
shape is moved within the slide's shape tree.

**Features covered:** `autoFit`, `flipH`, `flipV`, `image` (blipFill on
shape), `bevel`, `bevelBottom`, `depth`, `lighting`, `material`,
`softEdge`, `link`, `tooltip`, `name`, `zorder`.

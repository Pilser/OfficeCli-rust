# Basic PPT Shapes

Three files work together:

- **shapes-basic.sh** — Shell script that calls `officecli` to build the deck.
- **shapes-basic.pptx** — The generated 5-slide deck.
- **shapes-basic.md** — This file.

## Regenerate

```bash
cd examples/ppt
bash shapes/shapes-basic.sh
# → shapes/shapes-basic.pptx
```

## Slides

### Slide 1 — Geometry preset gallery

One row of 8 shapes, each using a different `geometry=` preset that the
PPT schema declares: `rect`, `roundRect`, `ellipse`, `triangle`,
`diamond`, `parallelogram`, `rightArrow`, `star5`.

```bash
officecli add file.pptx /slide[1] --type shape \
  --prop geometry=ellipse \
  --prop x=0.5in --prop y=1.5in --prop width=1.3in --prop height=1.3in \
  --prop fill=4472C4 --prop color=FFFFFF --prop bold=true \
  --prop text="ellipse"
```

`geometry=` aliases: `preset` and `shape`. Without it, the default is `rect`.

### Slide 2 — Fill variations

Seven shapes, all with the same `geometry=roundRect`, demonstrating every
fill form on one slide:

| Fill form | Spec |
|---|---|
| Solid hex | `fill=E63946` |
| Theme color | `fill=accent2` (follows deck theme) |
| Linear gradient | `gradient="FF6B6B-4ECDC4-45"` (`C1-C2-ANGLE`) |
| Radial gradient | `gradient="radial:FFE66D-FF6B35-center"` |
| Pattern | `pattern="diagBrick:1D3557:F1FAEE"` (`preset:fg:bg`) |
| Solid + opacity | `fill=2A9D8F --prop opacity=0.4` |
| Outline only | `fill=none --prop line="264653:2.5:solid"` |
| Gradient per-stop positions | `gradient="FF0000@0-FFD700@40-0000FF@100"` |

> ⚠ `opacity` **requires a fill to attach to**. `opacity=0.4` without
> `fill=` has no effect — see `schemas/help/pptx/shape.json` →
> `opacity.requires: ["fill"]`.

### Slide 3 — Outline styling

Two forms for outlines, both supported simultaneously:

**Compound form** (one string, three parts):

```bash
--prop line="E63946:3:solid"        # color:widthPt:dash
--prop line="1D3557:2:dash"
--prop line="2A9D8F:2.5:dashDot"
```

**Per-attribute form**:

```bash
--prop lineColor=E63946 --prop lineWidth=4pt --prop lineDash=solid
```

`cmpd` adds double / triple strokes for one outline:

```bash
--prop cmpd=dbl     # double
--prop cmpd=tri     # triple
```

`headEnd` / `tailEnd` work on **any** outlined shape, not just connectors —
the two skinny horizontal rectangles at the bottom of slide 3 have arrow
caps on a regular rect.

### Slide 4 — Rotation + effects

8 right-arrows showing `rotation=0..270` in 30°/45° steps. Plus three
demo shapes showing effects compound forms supported by the schema:

```bash
--prop shadow=000000        # outer shadow, default geometry
--prop glow=FFD700          # color glow
--prop reflection=tight     # tight | half | full
```

`shadow`/`glow` accept a color, or `true` (defaults), or `none` to clear.
Get readback returns rich compound `'#RRGGBBAA-blur-angle-dist-opacity'`
form — see `officecli help pptx get shape | grep -A3 shadow`.

### Slide 5 — Stroke geometry details

Three stroke-level props that are easy to miss but matter for precise
diagrams:

**`lineCap=`** (`flat` | `round` | `square`) — how the stroke
terminates at line endpoints. Shows up most clearly with a thick
dashed line: `round` and `square` both extend past the path's endpoint,
`flat` cuts off exactly at it.

**`lineJoin=`** (`round` | `bevel` | `miter`) — how corners are
rendered on a stroked shape. Demoed on a thick-outlined triangle so
each value's corner geometry is unambiguous.

**`lineAlign=`** (`ctr` | `in`) — stroke alignment relative to the
path. `ctr` centers the stroke on the path (default), `in` insets it
fully inside the shape boundary. With a 12pt stroke the difference in
the shape's outer bounds is `~12pt`.

```bash
--prop lineCap=round       # aliases: linecap, line.cap
--prop lineJoin=miter      # aliases: linejoin, line.join
--prop lineAlign=in        # aliases: linealign, line.align
```

**Features:** all 8 schema-declared `geometry` presets, every fill type
(solid/theme/gradient/radial/pattern/opacity/none/per-stop), both line forms
(`line="c:w:d"` compound and per-attribute `lineColor`/`lineWidth`/`lineDash`),
compound strokes (`cmpd=dbl|tri`), arrowheads on shape outlines,
rotation, shadow/glow/reflection effects, full stroke-geometry control
(`lineCap`, `lineJoin`, `lineAlign`).

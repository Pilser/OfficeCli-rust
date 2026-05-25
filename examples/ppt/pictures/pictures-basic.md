# Basic PPT Pictures

Three files work together:

- **pictures-basic.py** — Python script that generates 3 sample PNGs then builds the deck.
- **pictures-basic.pptx** — The generated 5-slide deck.
- **pictures-basic.md** — This file.

This example is Python (not shell) because it needs Pillow to generate
the sample images. Once you have your own images on disk, the
underlying `officecli add ... --type picture` calls are identical.

## Regenerate

```bash
cd examples/ppt
pip install Pillow
python3 pictures/pictures-basic.py
# → pictures/pictures-basic.pptx
```

## Slides

### Slide 1 — Three forms of `src=`

`src=` (alias `path=`) accepts:

| Form | Example |
|---|---|
| File path | `--prop src=/path/to/image.png` |
| URL | `--prop src=https://example.com/logo.png` |
| Data URI | `--prop src="data:image/png;base64,iVBOR..."` |
| Raw bytes | (programmatic — pass via base64 data-URI) |

The slide shows file path → data-URI → file-with-name. The `name=`
prop overrides the auto-generated `Picture {id}` label on `cNvPr@name`,
useful if you want to look it up by `@name=` later.

```bash
officecli add file.pptx /slide[1] --type picture \
  --prop src=/path/to/image.png \
  --prop x=0.5in --prop y=1.3in \
  --prop width=3.5in --prop height=2.6in \
  --prop alt="Image description for screen readers"
```

`alt=` (aliases: `altText`, `description`) sets accessibility text;
defaults to the source filename if you omit it.

### Slide 2 — Crop variants

Five copies of the same image, each cropped differently:

| Spec | Meaning |
|---|---|
| (no crop) | original aspect |
| `crop=20` | symmetric, 20% off all four edges |
| `crop=10,30` | 10% off top/bot, 30% off left/right (`V,H`) |
| `crop=5,10,40,20` | per-edge: `L,T,R,B` |
| `cropLeft=25 cropTop=25` | individual edges |

> 💡 `cropLeft=0.1` and `cropLeft=10` both mean 10% — the implementation
> auto-detects fraction (≤1) vs percent (>1). Values 0-100 are
> interpreted as percent; >0..1 as fraction.

The container `width=`/`height=` stays the same — the visible
fraction of the source image changes.

### Slide 3 — Rotation

Six copies arranged in a grid, one per `rotation=`:

```bash
--prop rotation=0      # default
--prop rotation=30
--prop rotation=90
--prop rotation=180
--prop rotation=270
--prop rotation=-45    # negative = counter-clockwise
```

Alias: `rotate=`. Values stored as `degrees × 60000` in `a:xfrm/@rot`.

### Slide 4 — Clickable pictures

`link=` makes the whole picture a hyperlink:

| Target | Example |
|---|---|
| URL | `--prop link=https://example.com` |
| Slide jump | `--prop link=slide[3]` |
| Named action | `--prop link=nextslide` |
| Other actions | `firstslide`, `lastslide`, `previousslide` |
| Email | `--prop link=mailto:hi@example.com` |
| Clear | `--prop link=none` |

`tooltip=` sets hover text — must be passed **together** with `link=`
in the same call (standalone tooltip update without link is not
supported by Set).

```bash
officecli add file.pptx /slide[4] --type picture \
  --prop src=/path/to/logo.png \
  --prop link=https://example.com \
  --prop tooltip="Open homepage"
```

### Slide 5 — Set-only effects: brightness / contrast / glow / shadow

Four props are schema-declared `add: false`, `set: true` — they live
on the picture but can only be applied **after** the picture is
created. Pattern: capture the picture's `@id` path from the Add
response, then Set the effect:

```bash
PIC=$(officecli add file.pptx /slide[5] --type picture \
        --prop src=/path/to/photo.png ... | awk '/Added/ {print $NF}')

officecli set file.pptx "$PIC" --prop brightness=40
officecli set file.pptx "$PIC" --prop contrast=-30
officecli set file.pptx "$PIC" --prop glow=FFD700-12-75
officecli set file.pptx "$PIC" --prop shadow=000000-10-45-6-50
```

| Prop | Range / Grammar | Effect |
|---|---|---|
| `brightness` | -100 .. 100 | luminance offset on the blip |
| `contrast` | -100 .. 100 | luminance modulation on the blip |
| `glow` | `color-radius-opacity` (e.g. `FFD700-12-75`) | outer glow |
| `shadow` | `color-blur-angle-dist-opacity` (e.g. `000000-10-45-6-50`) | outer shadow |

Brightness and contrast can be Set in the **same** call — they share
a single `<a:lum>` element under the blip:

```bash
officecli set file.pptx "$PIC" --prop brightness=15 --prop contrast=20
```

This slide also exercises `cropBottom=` / `cropRight=` directly by
name (slide 2 only reached them via the four-value `crop=L,T,R,B`
shorthand).

**Features:** three `src=` forms (file path / URL / data-URI),
`alt=`/`name=` for accessibility and lookup, every `crop=` form
(symmetric, `V,H`, `L,T,R,B`, per-edge `cropLeft`/`cropTop`/`cropRight`/`cropBottom`),
`rotation=` with positive and negative degrees, `link=` to URLs /
slides / named actions, `tooltip=` hover text, Set-only effects
`brightness`/`contrast`/`glow`/`shadow`.

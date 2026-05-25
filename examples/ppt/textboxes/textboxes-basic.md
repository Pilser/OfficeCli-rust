# Basic PPT Textboxes

Three files work together:

- **textboxes-basic.sh** — Shell script that calls `officecli` to build the deck.
- **textboxes-basic.pptx** — The generated 4-slide deck.
- **textboxes-basic.md** — This file.

## Regenerate

```bash
cd examples/ppt
bash textboxes/textboxes-basic.sh
# → textboxes/textboxes-basic.pptx
```

> 📌 `textbox` is an **alias** for `shape` in PPT (no separate
> element). Both route to `AddShape` — the only difference is that
> `--type textbox` skips the default `geometry=rect` outline and goes
> straight to a text-body container. The full property surface
> documented under `pptx/shape.json` applies.

## Slides

### Slide 1 — Horizontal alignment

Four full-width textboxes, one per `align=` value:

| Value | Effect |
|---|---|
| `left` | flush-left (default) |
| `center` | centered |
| `right` | flush-right |
| `justify` | full-justified (last line ragged) |

```bash
officecli add file.pptx /slide[1] --type textbox \
  --prop x=0.5in --prop y=1.3in --prop width=12in --prop height=1.3in \
  --prop text="..." --prop align=center
```

Aliases: `alignment`, `halign` both accepted on input.

### Slide 2 — Lists and multi-paragraph

Two patterns shown side-by-side:

**Bullet list** — start with one paragraph at Add time, then append
the rest via `--type paragraph` on the shape, then turn on bullets in
one shot at the shape level:

```bash
officecli add file.pptx /slide[2] --type textbox \
  --prop x=0.5in --prop y=1.2in --prop width=6in --prop height=4in \
  --prop text="Coffee preparation steps"

officecli add file.pptx '/slide[2]/shape[1]' --type paragraph \
  --prop text="Grind beans to medium-fine"
officecli add file.pptx '/slide[2]/shape[1]' --type paragraph \
  --prop text="Heat water to 93°C"
# ... repeat for each line

officecli set file.pptx '/slide[2]/shape[1]' --prop list=bullet
```

**Numbered list** — same pattern, `list=numbered` instead of `bullet`:

| `list=` value | Output |
|---|---|
| `bullet` | • dots |
| `numbered` | 1. 2. 3. ... |
| `alpha` | a. b. c. ... |
| `roman` | i. ii. iii. ... |
| `none` | strip all list formatting |
| any single char | use that glyph as bullet |

Nested levels via `--prop level=N` (0..8) on the paragraph — see the
last appended paragraph on slide 2's numbered list.

### Slide 3 — Rich text via runs

A textbox's `text=` is one big run. To mix styles inside one
paragraph, append `--type run` to the paragraph instead:

```bash
# Empty textbox first (text="")
officecli add file.pptx /slide[3] --type textbox \
  --prop text="" --prop size=20 ...

# Then build paragraph 1 run-by-run
officecli add file.pptx '/slide[3]/shape[1]/p[1]' --type run --prop text="The "
officecli add file.pptx '/slide[3]/shape[1]/p[1]' --type run \
  --prop text="quick " --prop bold=true --prop color=E63946
officecli add file.pptx '/slide[3]/shape[1]/p[1]' --type run \
  --prop text="brown " --prop italic=true --prop color=A0522D
officecli add file.pptx '/slide[3]/shape[1]/p[1]' --type run --prop text="fox..."
```

Also demonstrated:

- **Superscript / subscript**: `--prop baseline=super` / `--prop baseline=sub`
  (also accepts signed integer percent: `baseline=-25`)
- **Strikethrough**: `--prop strike=single` (also `double`)
- **Per-run color / size / bold / italic / underline** — all standard
  run-level rPr attributes

> ⚠ `cap=all` and `cap=small` are accepted on **shape** Add/Set, but
> not on **run** Add (different code paths). For all-caps in a run,
> upper-case the source text instead.

### Slide 4 — Multilingual + layout

**Per-script fonts** — `font.latin`, `font.ea`, `font.cs` target
separate rPr font slots. PowerPoint uses one per script automatically:

```bash
officecli add file.pptx /slide[4] --type textbox \
  --prop text="Hello, 世界! こんにちは、世界。" \
  --prop font.latin="Georgia" --prop font.ea="Yu Mincho"
```

ASCII characters use Georgia; Japanese/Chinese characters use Yu Mincho.
Aliases: `font.eastasia`, `font.eastasian`, `font.complexscript`,
`font.complex`. A bare `font=` sets both Latin and EA slots at once.

For Arabic / Hebrew layouts add `--prop direction=rtl`.

**Vertical alignment + padding** — three tall blue boxes show
`valign=top|middle|bottom`. The `margin=` prop sets uniform inner
padding (a.k.a. text-body inset):

```bash
--prop valign=middle --prop margin=0.15in --prop align=center
```

**Features:** every `align=` value, bullet/numbered lists via shape-level
`list=`, paragraph `level=` for nested indents, run-by-run styling (bold,
italic, underline, color, strike, baseline=super/sub), per-script fonts
(`font.latin`/`font.ea`), `valign=top|middle|bottom`, `margin=` inner
padding.

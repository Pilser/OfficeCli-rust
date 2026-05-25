# Advanced PPT Textbox Typography

Three files work together:

- **textboxes-advanced.sh** — Shell script that builds the deck.
- **textboxes-advanced.pptx** — The generated 5-slide deck.
- **textboxes-advanced.md** — This file.

Fills in the **child-element** properties (paragraph + run) that
`textboxes-basic` didn't reach. The two scripts together exhaust the
text-body property surface for PPT.

## Regenerate

```bash
cd examples/ppt
bash textboxes/textboxes-advanced.sh
# → textboxes/textboxes-advanced.pptx
```

## Slides

### Slide 1 — Per-paragraph overrides

One textbox; each paragraph carries its own `align=` and `lineSpacing=`.
The shape's defaults (`align=left`, single-spaced) apply only to
paragraphs that don't override them.

```bash
officecli add file.pptx '/slide[1]/shape[2]' --type paragraph \
  --prop text="..." --prop align=center

officecli add file.pptx '/slide[1]/shape[2]' --type paragraph \
  --prop text="..." --prop align=justify --prop lineSpacing=2x

officecli add file.pptx '/slide[1]/shape[2]' --type paragraph \
  --prop text="..." --prop lineSpacing=18pt
```

Use this pattern any time a single text box needs mixed paragraph
formatting (titles + body, quoted block + commentary, etc.).

### Slide 2 — Paragraph indents

Five variations on the same long text:

| Spec | Result |
|---|---|
| (default) | flush left, no indent |
| `marginLeft=1in` | whole paragraph shifted 1in right |
| `indent=0.5in` | first-line indent (book-style paragraph) |
| `marginLeft=0.6in indent=-0.5in` | hanging indent (bibliography-style) |
| `marginRight=2in` | text narrowed from the right edge |

`indent=` accepts negative values, which is how you build a hanging
indent — combine with a positive `marginLeft=` so the first line
"hangs" outside the body block.

Aliases: `leftindent`, `leftIndent`, `indentleft`. Both `indent` and
`marginLeft`/`marginRight` route through `SpacingConverter` so they
accept any length form (`0.5in`, `1.27cm`, `36pt`, or bare twips).

### Slide 3 — Per-paragraph styling (no runs needed)

When a whole paragraph shares the same styling, set `bold` / `italic` /
`color` / `size` / `lang` directly on the paragraph instead of
wrapping the text in a run. It's cheaper and the OOXML is cleaner.

```bash
officecli add file.pptx '/slide[3]/shape[2]' --type paragraph \
  --prop text="Whole paragraph is bold." --prop bold=true

officecli add file.pptx '/slide[3]/shape[2]' --type paragraph \
  --prop text="Whole paragraph is red." --prop color=E63946

officecli add file.pptx '/slide[3]/shape[2]' --type paragraph \
  --prop text="Whole paragraph is 22pt." --prop size=22

officecli add file.pptx '/slide[3]/shape[2]' --type paragraph \
  --prop text="French paragraph." --prop lang=fr-FR
```

The paragraph emits one implicit run with the merged rPr properties.

### Slide 4 — Per-run typography inside one paragraph

When **part** of a paragraph needs different styling, switch to runs.
This slide demonstrates the four run-only props not reached by the
basic example:

**`font=` per run** — mixed fonts in one line:

```bash
officecli add file.pptx '/slide[4]/shape[1]/p[1]' --type run \
  --prop text="Times " --prop font="Times New Roman" --prop size=24
officecli add file.pptx '/slide[4]/shape[1]/p[1]' --type run \
  --prop text="Courier " --prop font="Courier New" --prop size=18
```

**`spacing=` per run** — tracking variation:

```bash
--prop spacing=-1   # tighter
--prop spacing=4    # looser
--prop spacing=8    # very loose
```

Run-level `spacing=` is in **points** (decimal), shape-level `spacing=`
is in 1/100 pt (integer). Two different units for the same OOXML
attribute — handler emits whichever scope you asked for.

**`kern=` per run** — kerning threshold:

```bash
--prop kern=0       # disabled (no kerning at any size)
--prop kern=1       # enabled at all sizes (0.01pt threshold)
--prop kern=1200    # only kern from 12pt up
```

**`lang=` per run** — BCP-47 tag scoped to one run:

```bash
--prop lang=en-US   # color
--prop lang=en-GB   # colour
--prop lang=fr-FR   # couleur
```

PowerPoint's spellcheck honors per-run lang — useful for prose that
quotes terms from another language.

### Slide 5 — `subscript` / `superscript` aliases vs `baseline=`

Two equivalent ways to express the same OOXML attribute:

```bash
# Convenience aliases — boolean
--prop subscript=true        # ≡ baseline=sub
--prop superscript=true      # ≡ baseline=super

# Canonical — signed integer percent
--prop baseline=sub          # ≡ -25
--prop baseline=super        # ≡ +30
--prop baseline=50           # custom: 50% raise
--prop baseline=-40          # custom: 40% drop
```

Get readback always uses canonical `baseline` (signed integer
percent). `subscript` / `superscript` are mutually exclusive — set one
to clear the other.

Slide 5 also covers per-run `cap` directly on Add:

```bash
--prop cap=small        # canonical
--prop cap=all
--prop cap=none
--prop allCaps=true     # boolean alias → cap=all
--prop smallCaps=true   # boolean alias → cap=small
--prop allCaps=false    # → cap=none
```

**Features covered:** per-paragraph `align`/`lineSpacing` overrides
inside one shape, `indent` (positive & negative for hanging-indent),
`marginLeft`, `marginRight`, per-paragraph `bold`/`italic`/`color`/`size`/`lang`,
per-run `font`, per-run `spacing` (points), per-run `kern`, per-run `lang`,
per-run `cap` (with `allCaps` / `smallCaps` aliases),
`subscript`/`superscript` aliases, custom `baseline=` percent.

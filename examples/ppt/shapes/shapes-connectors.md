# Connectors and Groups

Three files work together:

- **shapes-connectors.sh** — Shell script that calls `officecli` to build the deck.
- **shapes-connectors.pptx** — The generated 3-slide deck.
- **shapes-connectors.md** — This file.

## Regenerate

```bash
cd examples/ppt
bash shapes/shapes-connectors.sh
# → shapes/shapes-connectors.pptx
```

## Slides

### Slide 1 — Connector geometry presets

Three side-by-side examples, one per preset. Each adds two anchor
ellipses then ties them together with one `--type connector`:

```bash
# Capture the added shape's @id path
A=$(officecli add file.pptx /slide[1] --type shape --prop geometry=ellipse \
      --prop x=0.5in --prop y=1.5in --prop width=2in --prop height=1.2in \
      --prop fill=4472C4 | awk '/Added/ {print $NF}')
B=$(officecli add file.pptx /slide[1] --type shape --prop geometry=ellipse \
      --prop x=4.5in --prop y=1.5in --prop width=2in --prop height=1.2in \
      --prop fill=E63946 | awk '/Added/ {print $NF}')

officecli add file.pptx /slide[1] --type connector \
  --prop shape=straight --prop from="$A" --prop to="$B" \
  --prop color=1D3557 --prop lineWidth=2pt --prop tailEnd=triangle
```

`shape=` accepts the short presets `straight`, `elbow`, `curve` (or the
OOXML full names: `straightConnector1`, `bentConnector3`,
`curvedConnector3` — see the schema note).

> 💡 Capture the path Add prints — every `officecli add` echoes
> `Added shape at /slide[N]/shape[@id=M]`. The `@id` form is **stable
> across re-numbering**: when you later `add group ... shapes=...`,
> existing positional indices may shift, but `@id` paths keep working.
> Positional `/slide[N]/shape[M]` works too, but only for read-only
> follow-up commands that don't restructure the slide.

### Slide 2 — Mini flowchart with attached connectors

Three boxes (Start → Valid? → End) plus a Retry box. Four connectors:

- `Start → Valid?` and `Valid? → End` — straight, black, solid
- `Valid? → Retry` and `Retry → Start` — elbow, red, dashed (the "no" loop)

This is the canonical "flowchart" workflow: shapes hold the boxes,
connectors with `from=` / `to=` hold the relationships. The connector
auto-routes its endpoints based on the source/target bounding boxes,
which is why `elbow` looks correct without any waypoint math.

Branch labels (`yes` / `no`) are added as plain `--type textbox` with
no fill or outline — same trick used elsewhere when you need bare text
floating over the canvas.

### Slide 3 — Grouping shapes

Three overlapping ellipses turned into one group:

```bash
G1=$(officecli add file.pptx /slide[3] --type shape --prop geometry=ellipse \
      --prop fill=E63946 ... | awk '/Added/ {print $NF}')
G2=$(officecli add ... fill=F4A261 ... | awk '/Added/ {print $NF}')
G3=$(officecli add ... fill=2A9D8F ... | awk '/Added/ {print $NF}')

officecli add file.pptx /slide[3] --type group \
  --prop shapes="$G1,$G2,$G3" --prop name="Logo"
```

The right side of the slide has three independent rectangles in the
same layout for visual comparison — they're addressable as separate
shapes, while the group on the left moves/scales/rotates as one unit
in the PowerPoint UI.

> ⚠ After `add group`, the message confirms `N shapes moved into
> group. Remaining shape count: K. Shape indices have been
> re-numbered.` This is exactly the trap that makes positional indices
> brittle and `@id` paths essential — see the note on slide 1.

**Features:** all 3 connector presets (straight/elbow/curve), attached
endpoints via `from=` / `to=` with `@id` paths, dashed connector lines
for branching/loopback semantics, arrowhead styling (`tailEnd=triangle`,
`tailEnd=arrow`), `--type group` with comma-separated `shapes=` paths.

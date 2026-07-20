# Phase 2 — Features Needing New Dependencies

These features require additional Rust crates beyond what the project currently uses. Each is evaluated for:
- **Maturity**: Is the crate well-maintained?
- **Footprint**: How much does it add to compile time / binary size?
- **Alternative**: Can we do it without the dependency?
- **Value**: What does it truly unlock for AI agents?

---

## 1. SVG Manipulation — `usvg` + `svgtypes`

### What it enables
- Parse SVG files into a structured element tree
- Walk/modify SVG elements: change `fill`, `stroke`, `opacity`, gradients
- Apply CSS `style` attributes to SVG elements
- Resolve SVG transforms, paths, and shapes
- Export modified SVG back to text

### Agent use cases
```bash
# Change icon color to match theme
officecli edit svg icon.svg --css 'fill: #1F4E79; stroke: none'

# Add shadow to SVG shape
officecli edit svg logo.svg 'shadow(2px, 2px, 4px, rgba(0,0,0,0.3))'

# Generate a chart as SVG, then import to docx
officecli create chart.svg --type bar --data 'Q1=100,Q2=150,Q3=200'
officecli add report.docx --parent /body/p[1] --type-name image --properties 'file=chart.svg'
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`usvg`](https://crates.io/crates/usvg) | 0.44 | **Very mature** — used in multiple renderers | Many (xmlparser, roxmltree, svgtypes, etc) | ~500KB binary |
| [`svgtypes`](https://crates.io/crates/svgtypes) | 0.15 | **Mature** — standalone SVG type parsers | Minimal | ~100KB |
| [`roxmltree`](https://crates.io/crates/roxmltree) | 0.20 | **Very mature** — read-only XML DOM | None | Minimal |

**Alternative without deps**: We can manipulate SVG as raw XML with `quick-xml` (already in project). Change `<path fill="red">` → `<path fill="blue">` is trivial. The dependency is needed for:
- Parsing SVG paths (`M10 20 L30 40 Z`)
- Resolving `transform="matrix(...)"` calculations
- Understanding SVG-specific CSS (`fill`, `stroke`, `clip-path`)
- Gradient resolution across SVG elements

**Recommendation**: Start with raw XML manipulation (no dep), add `svgtypes` later when path/transform parsing is needed.

---

## 2. Image Processing — `image` crate

### What it enables
- Resize, crop, rotate images before embedding
- Convert between image formats (PNG, JPEG, WebP, etc.)
- Generate thumbnails, apply watermarks, overlay text/images
- Get image dimensions (width, height, DPI)
- Compress/re-encode images for file size reduction

### Agent use cases
```bash
# Resize logo before embedding
officecli add doc.docx --parent /body/p[1] --type-name image \
  --properties 'file=logo.png' --properties 'resize=200x200' --properties 'crop=10,10,100,100'

# Add watermark to all images
officecli edit-images doc.docx --watermark 'CONFIDENTIAL' --opacity 0.3

# Convert everything to JPEG with quality 85
officecli optimize doc.docx --image-quality 85
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`image`](https://crates.io/crates/image) | 0.25 | **Very mature** — the standard Rust image crate | Many (png, jpeg, gif, webp decoders) | ~1MB binary |
| [`resize`](https://crates.io/crates/resize) | 0.7 | Mature — dedicated image resizing | Minimal | ~50KB |

**Alternative without deps**: We can skip image processing entirely (current behavior). No pure-Rust alternative exists for pixel manipulation.

**Recommendation**: Add `image` crate. It's the defacto standard, actively maintained, and enables a whole category of agent requests.

---

## 3. CSS Parsing — `cssparser`

### What it enables
- Robust parsing of CSS property declarations
- Color parsing: `#rgb`, `#rrggbb`, `rgb(r,g,b)`, `rgba(r,g,b,a)`, `hsl()`, `hsla()`, named colors
- CSS value parsing: lengths with units, percentages, numbers, strings, URLs
- CSS function parsing: `calc()`, `var()`, `linear-gradient()`, etc.
- Error recovery — malformed CSS doesn't crash, just skips the bad property

### Agent use cases
```bash
# Complex CSS with gradients
officecli add doc.docx --parent /body --type-name paragraph \
  --css 'background: linear-gradient(to right, #1F4E79, #2E75B6); font-weight: bold; color: white'

# CSS with calc and variables
officecli add slide.pptx --parent /slide[1] --type-name shape \
  --css 'width: calc(100% - 40px); fill: rgba(31, 78, 121, 0.8)'
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`cssparser`](https://crates.io/crates/cssparser) | 0.37 | **Very mature** — Mozilla/Servo, used in Firefox | Lightweight (itoa, dtoa-short, smallvec) | ~100KB |

**Alternative without deps**: Simple CSS parsing (`key: value;` split) is ~50 lines. Full CSS color parsing is ~80 more lines. The dep is only needed for:
- Complex CSS values (`linear-gradient(...)`)
- CSS function parsing (`calc()`, `rgb()`, `hsl()`)
- Robust error recovery for AI-generated CSS

**Recommendation**: Start with simple CSS parsing (no dep). Add `cssparser` when agents need gradient/calc/rgba support.

---

## 4. Font Embedding — `fontdb` + `ttf-parser`

### What it enables
- List available system fonts
- Embed fonts into documents for cross-platform portability
- Determine font metrics for layout calculations
- Subset fonts (only include used characters) for file size

### Agent use cases
```bash
# Use a custom font
officecli add doc.docx --parent /body --type-name paragraph \
  --properties 'text=Welcome' --properties 'font=Inter' --properties 'embedFont=true'

# Get font metrics for precise layout
officecli layout doc.docx --font-metrics 'font=Inter,size=12'
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`fontdb`](https://crates.io/crates/fontdb) | 0.18 | **Mature** — font database, used in Servo | Minimal | ~100KB |
| [`ttf-parser`](https://crates.io/crates/ttf-parser) | 0.24 | **Very mature** — TrueType/OpenType parser | None (no_std) | ~200KB |
| [`owned_ttf_parser`](https://crates.io/crates/owned_ttf_parser) | 0.24 | Mature — owned version | Depends on ttf-parser | ~50KB |

**Alternative without deps**: Font embedding can be done manually (read font bytes, write to docx parts, add relationship, update content type). Font metrics require actual font parsing.

**Recommendation**: Lower priority. Font embedding works manually. Only add when cross-platform font rendering becomes critical.

---

## 5. Advanced Color — `palette` crate

### What it enables
- Color space conversion (RGB, HSL, LAB, LCH)
- Color manipulation: lighten, darken, saturate, desaturate, mix/blend
- Palette generation: complementary, analogous, triadic, monochromatic
- Gradient interpolation between colors
- WCAG contrast ratio calculation

### Agent use cases
```bash
# Generate a blue theme
officecli palette --base 1F4E79 --scheme complementary
# → Returns: #1F4E79 (base), #79B84E (complementary), #2E75B6 (lighter), ...

# Apply a color scheme to whole document
officecli theme doc.docx --palette 'primary=1F4E79,secondary=2E75B6,accent=79B84E'

# Lighten color for background
officecli color lighten 1F4E79 40%
# → Returns: #E8EEF4
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`palette`](https://crates.io/crates/palette) | 0.7 | **Mature** | Linear algebra deps | ~300KB |
| [`colored`](https://crates.io/crates/colored) | 2.1 | Mature but terminal-focused | Minimal | ~30KB |

**Alternative without deps**: Color lighten/darken can be done with simple RGB math (20 lines). Named color lookup is a 150-entry map.

**Recommendation**: Skip. RGB math and a lookup table cover 95% of agent needs.

---

## 6. Markdown Parsing — `pulldown-cmark`

### What it enables
- Robust, spec-compliant Markdown parsing
- Support for CommonMark + GitHub Flavored Markdown extensions
- Parse tables, code blocks, headings, lists, emphasis, links, images, strikethrough, task lists
- Event-based streaming parser (low memory)

### Agent use cases
```bash
# Agent writes markdown, we convert to formatted docx
officecli import report.docx --markdown '
# Q3 Financial Report

## Revenue Breakdown

| Quarter | Amount  | Growth |
|---------|---------|--------|
| Q1      | $1.2M   | +15%   |
| Q2      | $1.4M   | +17%   |
| Q3      | $1.6M   | +14%   |

**Key Insight**: _Revenue grew 15% YoY_

> This is a significant achievement
'

# Result: Formatted docx with headings, table, bold/italic text, blockquote
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) | 0.11 | **Very mature** — used in mdBook, zola, etc. | Zero deps | ~200KB |
| [`markdown`](https://crates.io/crates/markdown) | 1.0 | **Very mature** | Zero deps | ~150KB |

**Alternative without deps**: Simple line-by-line Markdown parser is ~400 lines (as described in PHASE-2-NO-DEPS). However, it won't handle:
- Inline formatting within tables (bold in one cell, italic in another)
- Nested emphasis (`**bold *and italic***`)
- HTML in markdown
- Edge cases in CommonMark spec

**Recommendation**: Add `pulldown-cmark`. It's zero-dependency, extremely well-tested, and CommonMark compliant. A hand-rolled parser will miss edge cases that agents will inevitably encounter.

---

## 7. Data Visualization — `plotters`

### What it enables
- Render charts as PNG/SVG from data
- Bar charts, line charts, scatter plots, pie charts, histograms
- Axis labels, legends, grid lines, annotations
- Embedded into Office documents as images

### Agent use cases
```bash
# Create a chart from data and embed
officecli chart --type bar --data 'Q1=100,Q2=150,Q3=200' --title 'Quarterly Revenue' \
  --output chart.png
officecli add report.docx --parent /body/p[3] --type-name image --properties 'file=chart.png'

# Create a chart directly in a document
officecli add report.docx --parent /body --type-name chart \
  --properties 'type=bar' --properties 'data=Q1=100,Q2=150,Q3=200' \
  --properties 'title=Revenue'
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`plotters`](https://crates.io/crates/plotters) | 0.3 | **Mature** | Many (image, font, etc.) | ~1.5MB binary |
| [`plotly`](https://crates.io/crates/plotly) | 0.8 | Mature but produces HTML (Plotly.js) | Minimal Rust deps | Requires browser |

**Alternative without deps**: OOXML charts already exist (basic bar/column/line/pie via raw XML). plotters is for raster/vector rendering of custom charts.

**Recommendation**: Low priority. Existing OOXML chart support covers basic needs. Add only when agents need custom chart rendering or non-standard chart types.

---

## 8. Date/Time Parsing — `chrono`

### What it enables
- Parse date/time strings in various formats
- Calculate date differences, add durations
- Generate formatted dates for document fields
- Locale-aware date formatting

### Agent use cases
```bash
# Insert today's date
officecli add doc.docx --parent /body --type-name field --properties 'type=date' --properties 'format=MMMM DD, YYYY'

# Calculate due date
officecli set doc.docx '/body/p[3]' text='{{date_add today 30 days}}'
```

### Crate analysis

| Crate | Version | Maturity | Deps | Size Impact |
|-------|---------|----------|------|-------------|
| [`chrono`](https://crates.io/crates/chrono) | 0.4 | **Very mature** — de facto standard | time, iana-time-zone | ~300KB |
| [`time`](https://crates.io/crates/time) | 0.3 | Very mature — alternative | Minimal | ~200KB |

**Alternative without deps**: Simple date formatting (today's date, date+N days) can use `std::time::SystemTime`. Complex parsing needs chrono.

**Recommendation**: Add `chrono`. It's the standard, and date manipulation is a common agent request.

---

## Summary: Dependencies Decision Matrix

| Feature | Crate | Priority | Can Skip? | Skip Cost |
|---------|-------|----------|-----------|-----------|
| SVG manipulation | `usvg` + `svgtypes` | Medium | Yes — raw XML first | Can't resolve paths/transforms |
| Image processing | `image` | Medium | Yes — skip entirely | No resize/crop/watermark |
| CSS parsing | `cssparser` | Medium | Yes — simple parser first | Gradients, calc, error recovery |
| Font embedding | `fontdb` + `ttf-parser` | Low | Yes — manual | No font metrics |
| Color manipulation | `palette` | Low | Yes — RGB math | Advanced color schemes |
| Markdown parsing | `pulldown-cmark` | **High** | Partially — 400 lines hand-rolled | Edge cases, table formatting |
| Chart rendering | `plotters` | Low | Yes — OOXML charts exist | Custom chart rendering |
| Date/time | `chrono` | Medium | Partially — std::time | Complex date parsing |

## Recommended Dependency Additions

### Add now (high value, low risk):
1. **`pulldown-cmark`** (0 deps) — Markdown→Office conversion is a killer feature for AI agents

### Add when needed:
2. **`cssparser`** (lightweight) — When agents need gradient/calc/rgba in CSS
3. **`image`** (standard) — When agents request resize/crop/watermark

### Consider later:
4. **`chrono`** — When date math becomes frequent
5. **`svgtypes`** — When SVG manipulation needs path parsing
6. **`usvg`** — When SVG manipulation needs full tree parsing
7. **`fontdb` + `ttf-parser`** — When font embedding becomes critical
8. **`palette`** — When agents start designing color themes
9. **`plotters`** — When agents need custom chart rendering beyond OOXML charts

## Estimated Binary Size Impact

| Scenario | Binary Size |
|----------|-------------|
| Current (no new deps) | ~15MB |
| + pulldown-cmark | ~15.2MB |
| + pulldown-cmark + cssparser | ~15.3MB |
| + pulldown-cmark + cssparser + image | ~16.5MB |
| + all above + usvg + chrono | ~17.5MB |

Most dependencies are small. `image` and `usvg` are the only ones that add significant binary size.

# Phase 2 — Features Needing NO New Dependencies

These features can be implemented using **pure Rust** + the existing `quick-xml`/`zip` crates. They manipulate OOXML at the XML level, same as the existing codebase.

## Shared Infrastructure (handler-common)

### CSS Styling Module — `shared/css.rs`

**What**: Unified CSS property → OOXML mapping across all formats. Agents write CSS, tool translates.

| CSS Property | docx | xlsx | pptx | Effort |
|-------------|------|------|------|--------|
| `font-weight` | `w:b` | `<font><b/>` | `a:bold` | 2 lines |
| `font-style` | `w:i` | `<font><i/>` | `a:italic` | 2 lines |
| `font-size` | `w:sz` (×2) | `<sz val=N>` | `a:sz` | 3 lines |
| `color` | `w:color` | `<color rgb=...>` | `a:srgbClr` | 5 lines |
| `font-family` | `w:rFonts` | `<name val=...>` | `a:latin` | 3 lines |
| `text-decoration` | `w:u`/`w:strike` | underline/strike | `a:u`/`a:strike` | 5 lines |
| `text-align` | `w:jc` | alignment | `a:algn` | 3 lines |
| `background` | `w:shd` | fill | fill | 5 lines |
| `border` | `w:pBdr`/`w:tblBorders` | border | `a:ln` | 15 lines |
| `padding` | `w:tcMar` | — | — | 10 lines |
| `width` | `w:tblW`/drawing | col width | shape width | 5 lines |
| `opacity` | — | — | `a:alpha` | 3 lines |
| `vertical-align` | `w:vAlign` | valign | `a:anchor` | 3 lines |

**CSS color parser** (no deps needed):
- `#hex` → strip `#`, parse hex
- `rgb(r,g,b)` → split parens, parse ints
- `named colors` → 150-entry lookup table (red→FF0000, etc.)
- Total: ~80 lines

**New file**: `crates/handler-common/src/css.rs` — ~400 lines total

### Color Name Resolution — `shared/color.rs`

**What**: Named color → hex mapping + color manipulation (lighten/darken).

- 150 named CSS colors (W3C standard)
- `hex_to_rgb` / `rgb_to_hex` utilities
- No dependencies needed

**New file**: `crates/handler-common/src/color.rs` — ~200 lines

---

## docx-handler

### Run Properties (helpers.rs)

| Feature | OOXML | Lines | File |
|---------|-------|-------|------|
| Superscript/Subscript | `w:vertAlign val="superscript"` | 6 | `helpers.rs:290` |
| Double strikethrough | `w:dstrike` | 3 | `helpers.rs:285` |
| Raised/lowered text | `w:position val=N` (half-points) | 6 | `helpers.rs:295` |
| No-proof | `w:noProof` | 3 | `helpers.rs:300` |

### Paragraph Properties (helpers.rs)

| Feature | OOXML | Lines | File |
|---------|-------|-------|------|
| Tab stops | `w:tabs` with `<w:tab val=.. pos=.. leader=..>` | 40 | `helpers.rs:500` |
| Text alignment (vertical) | `w:textAlignment` | 6 | `helpers.rs:520` |
| Suppress line numbers | `w:suppressLineNumbers` | 3 | `helpers.rs:525` |
| Contextual spacing | `w:contextualSpacing` | 3 | `helpers.rs:530` |

### Headers/Footers — New Module `headers.rs`

**What**: Create header/footer parts with page numbers, dates, document info.

```rust
// Agent calls:
officecli add doc.docx --parent /body --type-name header --properties 'text=Page ' --properties 'pageNumber=true'
officecli add doc.docx --parent /body --type-name footer --properties 'text=Confidential'
```

**Architecture**:
1. Create `word/header1.xml` part with content
2. Create `word/_rels/document.xml.rels` relationship
3. Update `[Content_Types].xml` with Override
4. Add `w:headerReference` to sectPr in document.xml

**New file**: `crates/docx-handler/src/headers.rs` — ~250 lines

### Section Properties (mutations.rs)

| Feature | OOXML | Lines |
|---------|-------|-------|
| Section break type | `w:type val="nextPage"` | 10 |
| Page number format | `w:pgNumType w:fmt="decimal" w:start="1"` | 15 |
| Different first page | `w:titlePg` | 5 |
| Column widths (per-col) | `w:col w:w="..." w:space="..."` | 20 |

### Cell Properties (mutations.rs)

| Feature | OOXML | Lines |
|---------|-------|-------|
| Cell margins | `w:tcMar` with top/bottom/left/right | 25 |
| Horizontal merge | `w:hMerge val="restart"` / `"continue"` | 8 |

### Table Properties (mutations.rs)

| Feature | OOXML | Lines |
|---------|-------|-------|
| Default cell margins | `w:tblCellMar` | 20 |
| Width type | `w:tblW w:type="pct"` option | 5 |

### Anchored Images — Floating/Text Wrapping

**What**: `wp:anchor` template alongside existing `wp:inline`.

- `positionH`/`positionV` for absolute/relative positioning
- `wrapSquare`/`wrapTight`/`wrapThrough`/`wrapTopAndBottom` for text wrapping
- `behindDoc`/`inFrontOfText` for layering

**File**: `mutations.rs:2993` — new template function, ~80 lines

### Nested Tables (add.rs)

**What**: Allow `add_table` with parent path inside a table cell.

- Remove `/body`-only restriction in path validation
- Insert table inside cell's content

**File**: `add.rs:1802` — ~30 lines change

---

## xlsx-handler

### Styles Registry — New Module `styles.rs` (CRITICAL)

**What**: Full `xl/styles.xml` read/write module. The single most impactful fix.

```
crates/xlsx-handler/src/styles.rs
├── StylesModel (fonts, fills, borders, cellXfs, numFmts, dxfs)
├── fn parse_styles_xml(xml: &str) -> StylesModel
├── fn serialize_styles_xml(model: &StylesModel) -> String
├── fn register_style(styles: &mut StylesModel, props: &HashMap) -> u32
│   ├── find_or_create_font()   → fontId
│   ├── find_or_create_fill()   → fillId
│   ├── find_or_create_border() → borderId
│   ├── find_or_create_numfmt() → numFmtId
│   └── find_or_create_xf()     → xfId (combines all above + alignment)
├── fn update_cell_style(package, sheet_name, cell_ref, xf_id)
```

**Dependencies**: None — pure XML string building.
**Effort**: ~500 lines
**Files changed**:
- `styles.rs` (new) — 500 lines
- `mutations.rs` — replace hash stub with real registry call (~20 lines)
- `handler.rs` — wire up styles save on close (~10 lines)

### Layout Operations — New Module `layout.rs`

| Feature | OOXML | Lines |
|---------|-------|-------|
| Set column width | `<col min="1" max="1" width="N" customWidth="1"/>` | 40 |
| Set row height | `<row r="N" ht="N" customHeight="1"/>` | 30 |
| Hide/unhide column | `<col hidden="1"/>` | 10 |
| Hide/unhide row | `<row hidden="1"/>` | 10 |
| Freeze panes | `<pane ySplit="1" activePane="bottomLeft" state="frozen"/>` | 40 |
| Auto-filter | `<autoFilter ref="A1:C10"/>` | 25 |

**New file**: `crates/xlsx-handler/src/layout.rs` — ~200 lines

### Sheet Operations — New Module `sheets.rs`

| Feature | Implementation | Lines |
|---------|---------------|-------|
| Rename | Update `<sheet name="...">` in workbook.xml | 20 |
| Reorder | Reorder `<sheet>` elements in workbook.xml | 30 |
| Copy | Duplicate sheet XML + workbook entry + rels + content types | 80 |
| Hide/veryHidden | Set/unset `state` attribute on `<sheet>` | 15 |
| Tab color | Add/update `<sheetPr><tabColor rgb="..."/>` | 20 |

**New file**: `crates/xlsx-handler/src/sheets.rs` — ~200 lines

### Merged Cells — Add to `add.rs`

**What**: Allow merging cells during or after creation.

```rust
officecli add sheet.xlsx --parent Sheet1 --type-name merge --properties 'range=A1:C3'
```

**File**: `add.rs` — ~40 lines

### Named Ranges — Add to `helpers.rs` + `query.rs`

**What**: Parse `<definedName>` in workbook.xml, make queryable and creatable.

**Files**: `helpers.rs:parse_workbook`, `query.rs` — ~60 lines

### Conditional Formatting Reading — Add to `helpers.rs`

**What**: Parse existing `<conditionalFormatting>` elements from worksheet XML.

**File**: `helpers.rs:parse_sheet` — ~40 lines

---

## pptx-handler

### Table L2 Support — New Module `table_ops.rs`

**What**: Make tables addressable in the DOM so agents can `get`/`set`/`query` table cells.

1. Parse `<a:tbl>` elements in shape tree
2. Create `TableNode` DOM entries with cell text
3. Support `/slide[N]/shape[M]/tbl[1]/row[1]/cell[1]` paths
4. Support `set` for cell text

**New file**: `crates/pptx-handler/src/table_ops.rs` — ~200 lines
**Changes**: `navigation.rs`, `mutations.rs`, `dom_types.rs`

### Shape Run Formatting — Expand `view.rs`

| Feature | OOXML in pptx | Lines |
|---------|---------------|-------|
| Underline | `a:u val="sng"` | 5 |
| Strikethrough | `a:strike` | 5 |
| Font family | `a:latin typeface="Arial"` | 5 |
| Superscript/subscript | `a:baseline val="30000"` | 8 |
| Character spacing | `a:spc` | 5 |
| Highlight | `a:highlight` | 5 |
| Capitalization | `a:caps` / `a:smallCaps` | 6 |

**File**: `view.rs:710-775` — ~50 lines total additions

### Shape Outline/Effects — Expand `view.rs`

| Feature | Lines |
|---------|-------|
| Dashed lines (`prstDash`) | 10 |
| Line cap (round/flat) | 5 |
| Line join (round/miter) | 5 |
| Shadow (OuterShdw) | 25 |
| Glow | 15 |
| Reflection | 10 |

**File**: `view.rs` — ~80 lines total

### Gradient Fills — Add to `view.rs`

**What**: Support `fill="gradient(angle, color1, color2)"` syntax.

- `<a:gradFill>` with `<a:gsLst>` gradient stops
- Linear gradients with angle
- Path/radial gradients

**File**: `view.rs` — ~60 lines

---

## Cross-Cutting

### Markdown Import — New `shared/markdown.rs`

**What**: Convert Markdown text to OfficeCLI command sequences. No external parser needed for basic Markdown.

```rust
// Input: "# Heading\n\n**bold text** and *italic*\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |"
// Output: Vec of (element_type, properties) that can be fed to add()
```

**Parsing approach**: Simple line-by-line parser (no external dep):
- `# heading` → heading paragraph
- `**bold**` → run with bold
- `*italic*` → run with italic
- `| table |` → table creation
- `` `code` `` → font-family monospace
- `- list` → bullet list
- `1. list` → numbered list
- `> blockquote` → indented paragraph
- `---` → horizontal rule
- `[link](url)` → hyperlink
- `![alt](src)` → image

**Agent usage**:
```bash
officecli import doc.docx --markdown '# Report
## Q3 Results
**Revenue**: $1.2M
*Growth*: 15%
| Metric | Value |
|--------|-------|
| Users  | 5000  |'
```

This is **huge** for agent productivity. Agents naturally write markdown.

**New file**: `crates/handler-common/src/markdown.rs` — ~400 lines
**New command**: `officecli import file.docx --markdown '...'`

### Template System — New `shared/template.rs`

**What**: Variable substitution in document templates.

```rust
// Agent creates template with {{company_name}}, {{date}}, {{total}}
// Then: officecli render template.docx --var 'company_name=Acme' --var 'date=2026-07-19'
```

**Implementation**: Simple `{{var}}` → `value` replacement using `replace_in_string`. No template engine needed.

**New file**: `crates/handler-common/src/template.rs` — ~100 lines

---

## File Modularization Plan

### xlsx-handler (mutations.rs: 1462 lines → split)

| Current State | Split Into |
|---------------|------------|
| `mutations.rs:1-41` → | `sheets.rs` — sheet remove |
| `mutations.rs:43-53` → | `cells.rs` — cell remove |
| `mutations.rs:325-450` → | `styles.rs` — style props collection |
| `mutations.rs:453-590` → | `cells.rs` — cell properties set |
| `mutations.rs:591-600` → | `styles.rs` — modify_style_in_cell |
| `mutations.rs:709-802` → | `styles.rs` — style id extraction |
| `mutations.rs:804-900` → | `cells.rs` — insert/remove cells |
| `mutations.rs:901-1065` → | `cells.rs` — formula handling |
| `mutations.rs:1066-1200` → | `cells.rs` — range operations |
| `mutations.rs:1201-1462` → | `sheets.rs` — sheet-level operations |
| — | `layout.rs` — NEW (col widths, row heights, freeze) |

**Result**: `mutations.rs` shrinks to ~200 lines (orchestration only)

### docx-handler

| Current File | Lines | Split |
|-------------|-------|--------|
| `mutations.rs` | 4170 | Extract `headers.rs`, keep rest |
| `add.rs` | 2530 | Extract `images.rs` (add_image + chart), keep rest |
| `helpers.rs` | 950 | Could split into `run_props.rs` + `para_props.rs` |

---

## Implementation Order (Recommended)

### Tier 1: Critical Fixes (week 1)
1. `xlsx/styles.rs` — Real styles registry (fixes broken formatting)
2. `docx/headers.rs` — Headers/footers (fundamental missing feature)
3. `docx/helpers.rs` — vertAlign, tabs (high-usage text props)

### Tier 2: High-Impact Additions (week 2)
4. `xlsx/layout.rs` — Column widths, row heights, freeze panes
5. `xlsx/sheets.rs` — Sheet rename/move/copy/hide
6. `docx/mutations.rs` — Anchored images, section types, page numbers
7. `pptx/table_ops.rs` — Table L2 support

### Tier 3: Shared Infrastructure (week 3)
8. `handler-common/css.rs` — CSS-to-OOXML mapping
9. `handler-common/color.rs` — Color name resolution
10. `handler-common/markdown.rs` — Markdown import
11. `handler-common/template.rs` — Variable substitution

### Tier 4: Nice-to-Have (week 4)
12. `pptx/view.rs` — Run formatting, shadows, gradients
13. `docx/mutations.rs` — Cell margins, image effects
14. File modularization cleanup

---

## Total Effort Estimate (No-Deps Features)

| Category | Files | Estimated Lines |
|----------|-------|----------------|
| New modules | ~15 | ~3,000 |
| Modifications to existing files | ~10 | ~600 |
| **Total** | **~25 files** | **~3,600 lines** |

All using pure Rust + existing dependencies. No new crates.

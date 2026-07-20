# Feature Gaps — What Prevents OfficeCLI from Being Fully Flexible

This document catalogs every significant feature gap across all four handlers (docx, xlsx, pptx, pdf) that limits what an AI agent can accomplish through the tool.

## Legend

- **Critical**: Feature doesn't work at all or silently produces broken output
- **High**: Commonly needed for professional documents, no workaround
- **Medium**: Useful but has manual workarounds via L3 (raw XML)
- **Low**: Niche, rarely needed in AI agent workflows

---

## docx-handler

### Critical

| Gap | Details | Location |
|-----|---------|----------|
| Headers/footers | No high-level API to create header/footer parts with page numbers, dates, or document info | No `add` element type; no sectPr `headerReference`/`footerReference` in `mutations.rs` |
| Nested tables | `add_table` only works under `/body` — no support for tables inside table cells | `add.rs:1802` — path check rejects non-body parents |

### High

| Gap | Details | Location |
|-----|---------|----------|
| Superscript/subscript | `vertAlign` property (`superscript`, `subscript`) not implemented in run properties | `helpers.rs:255-466` — `build_run_properties` missing `w:vertAlign` |
| Tab stops | `tabs` (`w:tabs` with tab positions, alignments, leaders) not implemented in paragraph properties | `helpers.rs:473-661` — `build_paragraph_properties` missing |
| Raised/lowered text | `position` (w:position, half-points offset from baseline) not implemented | `helpers.rs` — run properties missing |
| Double strikethrough | `dstrike` not implemented (only `strike` exists) | `helpers.rs` — run properties missing |
| Anchored/floating images | Only inline images (`wp:inline`) supported. No `wp:anchor` for absolute positioning, text wrapping, behind/in front of text | `mutations.rs:2993` — drawing template hardcoded to `wp:inline` |
| Section break type | `w:type` (nextPage, continuous, oddPage, evenPage) not implemented in sectPr | `mutations.rs:1703-1809` — `set_section_properties` missing |
| Page number format | `w:pgNumType` (format, start value) not implemented in sectPr | `mutations.rs:1703-1809` |
| Different first-page header | `w:titlePg` in sectPr not implemented | `mutations.rs` |
| Cell margins | `w:tcMar` per-cell margins not implemented | `mutations.rs:1057-1175` — `set_cell_properties` missing |
| Table default cell margins | `w:tblCellMar` not implemented | `mutations.rs:780-907` — `set_table_properties` missing |
| Image borders/effects | `pic:spPr` only covers position/size — no `a:ln` for borders, no effects | `mutations.rs:2993` — drawing template minimal |

### Medium

| Gap | Details | Location |
|-----|---------|----------|
| `noProof` run property | Disable spell/grammar check on runs | `helpers.rs` |
| `textAlignment` paragraph property | Vertical alignment within line (auto, baseline, top, bottom, center) | `helpers.rs` |
| `suppressLineNumbers` | Suppress line numbers on specific paragraphs | `helpers.rs` |
| Per-column column widths | `w:cols` supports `num` and `space` but not individual column widths | `mutations.rs:1703-1809` |
| `hMerge` cell property | Horizontal merge (restart/continue) — `gridSpan` covers most cases | `mutations.rs:1057-1175` |
| `w:tblW` type attribute | Width type hardcoded to `dxa` — no `pct`, `auto`, `nil` | `mutations.rs` |
| Image cropping | `a:srcRect` not implemented | `mutations.rs` |
| Image rotation | No rotation attribute on drawing | `mutations.rs` |
| SVG proper support | SVG recognized but may not render correctly | `add.rs` |

### Low

| Gap | Details |
|-----|---------|
| `w:outline`, `w:shadow`, `w:emboss`, `w:imprint` text effects | Rarely used in business documents |
| `w:effect` text animation | Not for print documents |
| `w14:textFill`, `w14:textOutline` gradient text | Post-2007 feature |
| `w:bidi` run/paragraph bidi | RTL via `rightToLeft` property exists |
| Footnotes/endnotes content creation | Reference exists but no content body API |
| Math equations (OMML) | Rarely needed |
| Watermarks | Can be done manually |
| Digital signatures | Out of scope |
| Charts | Out of scope for docx |
| ActiveX / OLE objects | Out of scope |

---

## xlsx-handler

### Critical

| Gap | Details | Location |
|-----|---------|----------|
| **Styles registry non-functional** | Formatting properties (bold, color, fill, border, alignment) accepted but silently produce **fake hash-based style IDs** that don't match any real `cellXfs` entry in `styles.xml`. All cell formatting is **invisible in Excel**. | `mutations.rs:709-802` — `modify_style_in_cell` uses hash placeholders; `extract_style_id_from_spec` returns synthetic IDs (comment: "A future PR will register the style") |
| **No `styles.xml` for new files** | `add_sheet()` creates worksheet XML but **never creates `xl/styles.xml`** for brand-new workbooks | `add.rs` — no stylesheet generation path |
| Styles not registered on `set` | `set_cell_properties` collects style props into `style_parts` vector but never writes them to `styles.xml` — only injects fake IDs into cell XML | `mutations.rs:325-450` |

### High

| Gap | Details | Location |
|-----|---------|----------|
| Column widths — set | No `set` property for column width | `mutations.rs` |
| Row heights — set | No `set` property for row height | `mutations.rs` |
| Hide/unhide columns | `col` element's `hidden` attribute not writeable | `mutations.rs` |
| Hide/unhide rows | Row `hidden` attribute not writeable | `mutations.rs` |
| Insert/delete rows | No operation to insert empty rows or delete entire rows with renumbering | `mutations.rs` |
| Insert/delete columns | No column insertion/deletion that shifts cells | `mutations.rs` |
| Freeze panes | `<pane>` element not createable | `mutations.rs` |
| Auto-filter | Only created during CSV import — no general API | `import.rs` |
| Sheet rename | No `set` to change `<sheet name="...">` | `mutations.rs` |
| Sheet move/reorder | No operation to reorder sheets | `mutations.rs` |
| Sheet copy | No duplicate sheet operation | `mutations.rs` |
| Sheet hide/very hidden | `state` attribute not writeable | `mutations.rs` |
| Merged cells — create | Merges **read** for HTML preview but **not createable** via any `add` operation | `add.rs` — no merge facility |
| Named ranges / defined names | `<definedName>` not parsed, not queryable, not creatable | `helpers.rs:parse_workbook` |

### Medium

| Gap | Details | Location |
|-----|---------|----------|
| Chart reading/modification | Charts parsed for HTML preview but not in DOM — cannot `get`/`set`/`query` | No model representation |
| Chart types limited | Only 4 types (bar/column/line/pie) — no scatter, area, radar, bubble, doughnut, surface | `add.rs:742` |
| Chart multi-series | Chart XML only produces single `<c:ser>` block | `add.rs` |
| Chart styling | No customization for colors, line styles, markers, 3D | Hardcoded templates |
| Sparklines | Not supported | — |
| Conditional formatting — read | Can `add` conditional formats but cannot read existing ones | `helpers.rs` |
| Conditional formatting — types limited | Only `cellIs` rules — no data bars, color scales, icon sets, top/bottom | `add.rs:914-921` |
| Data validation — read | Existing validations not parsed | `helpers.rs:parse_sheet` |
| Comments | Not parsed, not viewable, not creatable | — |
| Array formulas | `<f t="array">` not supported | `mutations.rs:541` |
| Shared formulas | `<f t="shared">` not supported | `helpers.rs` / `mutations.rs` |
| Print areas | `<printArea>` not parsed or settable | — |
| Page setup | `<pageSetup>` not supported | — |
| Headers/footers | `<headerFooter>` not supported | — |
| Pivot table creation | Parsed and queryable but not createable | `add.rs` |
| Table styling | `add_table` hardcodes `TableStyleMedium2` — no style property | `add.rs:418-420` |
| Gradient fills | Only `patternFill` — no `gradientFill` | `helpers.rs` / `html_preview.rs` |
| Rich text in cells | Multi-run inline strings lose per-run formatting on parse | `helpers.rs:parse_sheet` |
| Theme color resolution on write | Named color map exists but can't resolve theme references when writing | `mutations.rs:1132-1153` |

### Low

| Gap | Details |
|-----|---------|
| Worksheet protection | `<sheetProtection>` not parsed or settable |
| Data sorting | No API to sort rows by columns |
| Outline/summary rows | SUBTOTAL outline rows, collapse/expand not supported |
| 3D formula references | Formula evaluator doesn't support `Sheet1:Sheet3!A1` |
| Document properties | Extended props partially read, no write support |
| Cell notes / threaded comments | Newer Excel feature |
| VBA/Macros | Out of scope |
| Digital signatures | Out of scope |
| Custom XML data parts | Out of scope |

---

## pptx-handler

### Critical

| Gap | Details | Location |
|-----|---------|----------|
| **Tables not addressable via L2** | Tables are created (add.rs) and rendered (html_preview.rs) but **not in the DOM** — cannot `get`, `set`, `query`, or `remove` table cells/text | `navigation.rs:160-162` — DOM skips non-text elements |
| **Shape fills limited to solid** | L2 `set` only handles solid fill colors — no gradient, pattern, picture fills | `view.rs:680-703` |
| **Run formatting limited** | `apply_text_format` only handles bold, italic, size, color, alignment. No strike, underline, superscript, subscript, font family, spacing, highlight, caps | `view.rs:710-775` |

### High

| Gap | Details | Location |
|-----|---------|----------|
| Shadow effects | Cannot set shape shadows via L2 — `view.rs` has no shadow property handler | `view.rs:396-428` |
| Glow / soft edges | No L2 set support | `view.rs` |
| 3D effects | No bevel, material, lighting, rotation | `view.rs` |
| Reflection | No L2 support (html_preview renders it for display only) | `view.rs` |
| Outline dashing/cap/join | `set_shape_line` supports width + solid color only — no `prstDash`, cap, join | `view.rs:826-863` |
| Flip (horizontal/vertical) | No flip property in L2 | `view.rs` |
| Chart types limited | Only 4 types (bar/column/line/pie) — matches xlsx limitation | `add.rs:1075-1167` |
| Chart not in DOM | Charts created but not addressable via L2 paths | Same as xlsx |
| SmartArt | Not supported at all | — |
| Slide sections | Not supported | — |
| Slide size/orientation | Read-only, cannot be changed | `html_preview.rs:1872-1887` |
| Master slides | Not addressable via L2 paths | — |
| Layout slides | Not addressable via L2 paths | — |

### Medium

| Gap | Details | Location |
|-----|---------|----------|
| Picture fills on shapes | `blipFill` in spPr rendered in HTML but not settable via L2 | `view.rs` |
| Image cropping | `a:srcRect` read in html_preview, not settable | `view.rs` |
| Image transparency | `alphaModFix` rendered, not settable | `view.rs` |
| Hyperlink listing | Can add hyperlinks but cannot list/query existing ones | `add.rs:1706-1807` |
| Speaker notes — read | Can add notes but cannot read them via L1/L2 API | `add.rs:1664-1703` |
| Audio icon | `add_audio` incorrectly delegates to `add_picture` | `add.rs:955-957` |
| OLE objects | Placeholder rendered in HTML, no L2 support | — |
| Ink / handwriting | Not supported | — |
| Slide number / date/time fields | `a:fld` recognized in HTML but no create API | — |

### Low

| Gap | Details |
|-----|---------|
| VBA/Macros | Out of scope |
| ActiveX controls | Out of scope |
| Custom XML parts | Out of scope |
| Digital signatures | Out of scope |
| Accessibility tags | Out of scope |

---

## pdf-handler

### High

| Gap | Details | Location |
|-----|---------|----------|
| Rich text formatting | PDF text is basic — no bold/italic/color/font selection in created text | Unclear without full audit |
| Table creation | No table layout support in PDF output | — |
| Image embedding | May not support image embedding in created PDFs | — |

### Medium

| Gap | Details |
|-----|---------|
| Hyperlinks | No link support in PDF creation |
| Headers/footers | No page numbers, headers/footers in PDF output |
| Font embedding | May not embed fonts, affecting portability |

---

## Cross-Cutting Gaps

| Gap | Handlers Affected | Details |
|-----|-------------------|---------|
| **CSS styling** | All | No unified CSS-to-OOXML mapping — agents must learn format-specific property names |
| **Color name resolution** | All | No `color: "red"`→`FF0000` mapping — agents must know hex values |
| **Gradient fills** | docx, xlsx, pptx | No gradient support in any format |
| **SVG manipulation** | docx, pptx | SVG added as opaque binary — cannot edit colors, strokes, or text inside SVG |
| **Image processing** | docx, xlsx, pptx | No crop, resize, rotate, watermark, or overlay operations |
| **Template system** | All | No variable substitution or template rendering — agents must build documents line by line |
| **Markdown import** | docx (primary) | No markdown-to-docx conversion — agents write in markdown but must translate to OfficeCLI commands |

---

## Summary by Priority Tier

| Tier | Count | Examples |
|------|-------|---------|
| **Critical** | 5 | xlsx styles broken, no headers/footers, pptx tables not addressable, pptx fills limited, style registry non-functional |
| **High** | ~45 | vertAlign, tabs, anchored images, column widths, freeze panes, merged cells, sheet operations, shadows, 3D, runs formatting |
| **Medium** | ~30 | Chart types, conditional formatting, image effects, gradients, SVG, sparklines |
| **Low** | ~20 | VBA, digital signatures, watermarks, equations |

**Total: ~100 gaps** preventing the tool from being "fully flexible" for professional use.

# Phase 1.2 — Bug Fixes & Feature Gaps

> Status: 4 of 12 items fixed in this phase. See ✅ for completed fixes.

---

## ✅ Fixed in This Phase

### 1. `add image` with `file=` property now works
**File:** `crates/docx-handler/src/mutations.rs:2921`, `crates/pptx-handler/src/add.rs:642`, `crates/docx-handler/src/add.rs:2141`

The code only checked for `src` or `path` property keys, but the CLI passes `file=`.
Added `.or_else(|| properties.get("file"))` as fallback in all three image add paths.

### 2. `--properties 'border=all'` now creates visible borders
**File:** `crates/docx-handler/src/mutations.rs:902`

`"all"` didn't match `"all="` (prefix), `"single"`, or `"thin"`. Added `|| value == "all"` so `border=all` behaves like `border=all=single`.

### 3. `set <xlsx> cell text=` now accepted
**File:** `crates/xlsx-handler/src/mutations.rs:376`

Added `| "text"` alongside `"value"` match arm so `text=Hello` works for xlsx cells.

### 4. Multi-row table `rNcN` text fixed
**File:** `crates/docx-handler/src/add.rs:1871-1874`

Outer loop had `for _ in 0..rows` (no row counter) and format string used `format!("r{}c{}", col_idx + 1, 1)` (hardcoded column). Changed to `for row in 0..rows` and `format!("r{}c{}", row + 1, col_idx + 1)`.

---

## Remaining Issues

> Identified by building a real-world laptop shop quotation (.docx) and testing
> all formats (docx, xlsx, pptx, pdf) with the `officecli-v1` binary built from
> Phase 1 changes.

---

## 🔴 Critical (Docx Corruption)

### 1. `add image` produces 0-byte media files — docx invalid

The ZIP entry `word/media/image1.png` is created but **empty (0 bytes)**. The
drawing XML correctly references `r:embed="rId1"` and the relationship points
to `media/image1.png`, but no binary pixel data is ever written.

```bash
# Reproduction
officecli create test.docx
officecli add test.docx --parent /body/p[1] --type-name image \
  --properties "file=/path/to/image.png" --properties "width=100" --properties "height=100"
unzip -l test.docx word/media/image1.png
# → 0 bytes  ← CORRUPTED
```

**Effect:** Any docx with an image added cannot be opened in Word/LibreOffice.
Schema validators pass because the ZIP structure is correct — only the binary
payload is missing.

**Fix scope:** `crates/oxml/` or `crates/docx-handler/src/add.rs` — the image
data must be read from disk and written into the ZIP entry.

### 2. Image dimensions ignored — always 100×100 EMU

The drawing XML always emits `cx="100" cy="100"` regardless of the
`--properties 'width=120' --properties 'height=120'` values. 100 EMU ≈ 0.01
inches — effectively invisible.

**Fix scope:** `crates/docx-handler/src/add.rs` — convert pt→EMU and pass to
`<wp:extent>` / `<a:ext>`.

---

## 🔴 Critical (Xlsx Cell Set Broken)

### 3. `set <xlsx> /Sheet1/A1 text=value` rejected

Despite `officecli help xlsx cell` listing `text` as a supported property, the
actual handler returns:

```
OK (UNSUPPORTED props: text (did you mean: next?).
```

```bash
# Reproduction
officecli create test.xlsx
officecli set test.xlsx '/Sheet1/A1' text=Hello
# → UNSUPPORTED props: text
```

**Effect:** No way to write cell values in xlsx. CSV import (`officecli import
--file-source`) works, but individual cell writes are broken.

**Fix scope:** `crates/xlsx-handler/src/handler.rs` or `crates/xlsx-handler/src/mutations.rs`
— the `set` handler doesn't dispatch `text` property to the cell writer.

---

## 🟡 Table Borders Invisible

### 4. `--properties 'border=all'` produces empty XML

The property creates `<w:tblBorders />` with **no child elements**. Proper
border XML requires:

```xml
<w:tblBorders>
  <w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/>
  <w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/>
  <w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/>
  <w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/>
  <w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/>
  <w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/>
</w:tblBorders>
```

Current output:

```xml
<w:tblBorders />
```

**Effect:** Tables have no visible gridlines in Word/LibreOffice. Data is
correct and rows/columns are structured, but appear as a floating text block.

**Fix scope:** `crates/docx-handler/src/add.rs` — populate tblBorders with
default border children when `border=all` is specified.

---

## 🟡 Table Features Missing

### 5. No table title / caption

OCX `w:tblCaption` / `w:tblDescription` not supported. There is no `--property
'title=...'` for tables.

**Fix scope:** `crates/docx-handler/src/add.rs` — accept `title` / `caption`
property and inject `<w:tblPr><w:tblCaption w:val="..."/></w:tblPr>`.

### 6. No column width control

All table columns get equal width. `--properties 'width=...'` on a table or
column has no effect.

**Fix scope:** `crates/docx-handler/src/add.rs` — parse column widths from
properties and emit `<w:tblGrid><w:gridCol w:w="..."/></w:tblGrid>` entries.

### 7. No colspan / rowspan

Cell merging not supported. `--properties 'span=2'` on cell exists in schema
but has no observable effect in the generated XML.

### 8. Adding cells one-by-one is tedious

Creating a 5×6 table requires ~25 CLI calls (1 table + 5 rows + 20 cells). No
`--rows N --cols N` shorthand that works end-to-end.

---

## 🟡 Formatting Gaps

### 9. No paragraph background / shading

Only cell-level shading works (`set ... shading=1F4E79`). Paragraph-level
`<w:shd>` not exposed.

### 10. No page break control

No way to insert `<w:br w:type="page"/>` before a section.

### 11. No column set widths for tables in `set` command

Even after table creation, `set` on a column/cell doesn't accept `width=`.

---

## 🟢 Minor

### 12. Validate passes corrupted docs

`officecli validate` reports "No validation errors" on docs with 0-byte images
— because the ZIP structure is technically valid. Should detect referenced
media files that are empty.

### 13. `--range` and `--grid` flags accepted but untested

These flags parse correctly but no comprehensive test verifies they produce
correct cropped/grid screenshots.

---

## Summary

| Priority | Count | Key Items |
|----------|-------|-----------|
| 🔴 Critical | 3 | Image 0-byte, Image dimensions, Xlsx cell set broken |
| 🟡 Important | 7 | Borders empty, No caption, No column widths, No merge, No page break, No paragraph bg |
| 🟢 Minor | 2 | Validate misses empty media, Range/grid untested |

**Total gaps identified: 12 items across 4 formats (docx, xlsx, pptx, pdf)**

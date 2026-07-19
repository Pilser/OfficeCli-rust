# Phase 1.2 — Bug Fixes & Feature Gaps

> Status: 10 of 12 items addressed. See ✅ for completed fixes.

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

### 5. Image dimensions: bare numbers treated as points
**File:** `crates/docx-handler/src/mutations.rs:3470-3473`

`parse_emu()` treated bare numbers as raw EMU (e.g., `width=200` = 200 EMU ≈ 0.016pt).
Changed else branch to multiply by 12700, treating bare numbers as points.

### 6. Table caption/title support
**File:** `crates/docx-handler/src/add.rs:1840-1846`

`--properties 'title=My Table'` now creates `<w:tblCaption w:val="My Table"/>` in table properties.

### 7. Column widths for tables
**File:** `crates/docx-handler/src/add.rs:1899-1915`

`--properties 'colWidths=100,200,150'` now sets column widths (in points → twips) on `<w:gridCol>` elements.

### 8. Page break support
**File:** `crates/docx-handler/src/mutations.rs:245-279`

`officecli set docx '/body/p[1]' pageBreak=true` inserts `<w:r><w:br w:type="page"/></w:r>` at paragraph start.

### 9. Validate detects empty media files
**File:** `crates/docx-handler/src/handler.rs:771-788`

`officecli validate` now reports an error if any file under `word/media/` has 0 bytes.

### 10. Paragraph background shading
Already existed in the codebase via `shading`/`shd` property on paragraph `set`.

---

## Remaining Issues (Phase 2 candidates)

> Identified by building a real-world laptop shop quotation (.docx) and testing
> all formats (docx, xlsx, pptx, pdf) with the `officecli-v1` binary built from
> Phase 1 changes.

---

## 🟡 Phase 2 Candidates

### 1. Colspan / rowspan (cell merging)

`--properties 'span=2'` on cell exists in the schema but has no observable
effect. Need to verify `<w:gridSpan>` and `<w:vMerge>` work through the DOM
serialization path.

### 2. Tedious cell-by-cell table creation

A 5×6 table requires ~25 CLI calls (1 table + 5 rows + 20 cells). Could add
`--cells 'r1c1=val,r1c2=val,...'` shorthand.

### 3. Column widths in `set` command

`officecli set docx '/body/tbl[1]/col[1]' width=100` is not supported. Column
widths only work at table creation time via `colWidths`.

### 4. `--range` and `--grid` screenshots (verify)

These flags parse correctly but haven't been verified with actual screenshot
capture in this phase.

---

## Summary

| Priority | Count | Key Items |
|----------|-------|-----------|
| ✅ Fixed | 10 | Image 0-byte, Image dims (pt), Xlsx text, Border all, Multi-row text, Caption, Col widths, Page break, Validate media, Paragraph bg |
| 🟡 Phase 2 | 4 | Colspan/rowspan, Cell shorthand, Col set-widths, Screenshot verify |

**Total gaps identified: 14 items — 10 fixed, 4 deferred to Phase 2**

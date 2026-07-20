# L2 Gap Analysis — Properties Missing from the DOM/Query Layer

**Status as of 19 July 2026**: All known gaps have been resolved. A professional quotation document can now be created using only L2 commands (`create`, `add`, `set`, `--css`) with **zero raw XML edits**.

---

## Resolved Gaps ✓

| # | Feature | Fixed In | Status |
|---|---------|----------|--------|
| 1 | Paragraph borders (`borderBottom`, `borderTop`, `borderLeft`, `borderRight`, `borderAround`) | `mutations.rs` + `helpers.rs` | ✅ Working |
| 2 | Table cell run properties (bold, color, size, font on cell runs) | `mutations.rs` | ✅ Working |
| 3 | Paragraph background shading (`shading`/`bgColor`) | `mutations.rs` | ✅ Working |
| 4 | Table cell background (`shading`/`bgColor`) | `mutations.rs` | ✅ Working |
| 5 | HTML preview table borders | `html_preview.rs` | ✅ Fixed (`border: 1px solid #ccc`) |
| 6 | HTML preview header/footer content | `html_preview.rs` | ✅ Fallback reader added |
| 7 | Image count in stats | `handler.rs` + `dom_types.rs` | ✅ Working |
| 8 | CSS property mappings (`background-color`, `border`, `vertical-align`, `font-weight`, `line-height`, `text-decoration`, `padding`, `border-*`) | `css.rs` | ✅ Working |

---

## How to Use the New Features

### Paragraph borders
```bash
officecli set document.docx '/body/p[4]' 'borderBottom=color=1F4E79;size=8;space=1'
officecli set document.docx '/body/p[5]' 'borderAround=color=FF0000;size=4;space=2'
```

### Table cell styling
```bash
officecli set document.docx '/body/tbl[1]/row[1]/cell[1]/p[1]/r[1]' bold=true color=FFFFFF size=10
officecli set document.docx '/body/tbl[1]/row[4]/cell[2]/p[1]/r[1]' bold=true color=1F4E79 size=12
```

### CSS flag (works with add)
```bash
officecli add document.docx --parent /body --type-name paragraph \
  --css 'font-weight: bold; color: #FF0000; font-size: 18pt; text-align: center' \
  --properties 'text=Styled Text'
```

### Stats with image count
```bash
officecli view document.docx -m stats
# Now shows: Images: 1 (previously always 0)
```

---

## Remaining Minor Notes (not blockers)

| Note | Details |
|------|---------|
| `headerBg`/`headerColor` on `add table` | Works in docx XML but LibreOffice may not render it. Apply bold/color white manually via `set` on header row cells as workaround. |
| HTML preview fidelity | The HTML output is a preview, not a pixel-perfect replica. Table borders are now visible. |
| `--css border` shorthand | The `border` shorthand maps to all 4 sides. For fine control, use `--properties` with `borderTop`, `borderBottom`, etc. |

---

## Priority for Future Work

All critical gaps are closed. Future improvements are enhancements, not blockers:

| Priority | Item | Effort | Reason |
|----------|------|--------|--------|
| Low | Nested list support | ~40 lines | Lists work but nested lists may need refinement |
| Low | Image resize via `set` | ~15 lines | Currently only `add` supports image dimensions |
| Low | Page margins via L2 | ~10 lines | Currently set in template only |

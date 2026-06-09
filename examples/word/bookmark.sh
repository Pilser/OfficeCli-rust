#!/bin/bash
# Bookmark test document — positional, --wrap, and --range-paths modes
# Usage: ./bookmark.sh [officecli path]

set -e
CLI="${1:-officecli}"
OUT="$(dirname "$0")/bookmark.docx"

rm -f "$OUT"
$CLI create "$OUT"

# ==================== Setup: add paragraphs ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Bookmark Test Document" style=Heading1
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="First paragraph for bookmark testing."
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Second paragraph with some text content."
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Third paragraph for cross-para bookmark."

# ==================== Scenario 1: Positional bookmark (empty pair) ====================
echo "=== Scenario 1: Positional bookmark ==="
$CLI add "$OUT" --parent '/body/p[2]' --type-name bookmark --properties name=pos1
echo "  Added: positional bookmark 'pos1' at p[2]"
$CLI get "$OUT" '/body/p[2]/bookmarkStart[@name=pos1]' --json

# ==================== Scenario 2: Bookmark with text ====================
echo "=== Scenario 2: Bookmark with text ==="
$CLI add "$OUT" --parent '/body/p[3]' --type-name bookmark --properties name=withText text="Bookmarked content"
echo "  Added: bookmark 'withText' with text at p[3]"
$CLI get "$OUT" '/body/p[3]/bookmarkStart[@name=withText]' --json

# ==================== Scenario 3: Bookmark with custom id ====================
echo "=== Scenario 3: Bookmark with custom id ==="
$CLI add "$OUT" --parent '/body/p[4]' --type-name bookmark --properties name=customId id=42
echo "  Added: bookmark 'customId' with id=42"
$CLI get "$OUT" '/body/p[4]/bookmarkStart[@name=customId]' --json

# ==================== Scenario 4: SET — rename bookmark ====================
echo "=== Scenario 4: SET rename bookmark ==="
$CLI set "$OUT" '/body/p[2]/bookmarkStart[@name=pos1]' name=renamed1
echo "  Renamed 'pos1' to 'renamed1'"
$CLI get "$OUT" '/body/p[2]/bookmarkStart[@name=renamed1]' --json

# ==================== Scenario 5: SET — update bookmark id ====================
echo "=== Scenario 5: SET update bookmark id ==="
$CLI set "$OUT" '/body/p[4]/bookmarkStart[@name=customId]' id=99
echo "  Updated 'customId' id from 42 to 99"
$CLI get "$OUT" '/body/p[4]/bookmarkStart[@name=customId]' --json

# ==================== Scenario 6: --wrap a run ====================
echo "=== Scenario 6: --wrap a run ==="
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Paragraph for wrap testing."
$CLI add "$OUT" --parent '/body/p[5]' --type-name bookmark --wrap '/body/p[5]/r[1]' --properties name=wrap1
echo "  Wrapped r[1] in p[5] with bookmark 'wrap1'"
$CLI get "$OUT" '/body/p[5]' --json

# ==================== Scenario 7: --range-paths (split + bookmark) ====================
echo "=== Scenario 7: --range-paths ==="
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Paragraph for range-paths bookmark testing with enough characters."
$CLI add "$OUT" --parent '/body/p[6]' --type-name bookmark --range-paths '/body/p[6][5..20]' --properties name=range1 color=FF0000
echo "  Added range-paths bookmark 'range1' at chars 5..20 in p[6]"
$CLI get "$OUT" '/body/p[6]' --json

# ==================== Scenario 8: Validate ====================
echo "=== Scenario 8: Validate ==="
$CLI validate "$OUT"

echo ""
echo "All bookmark scenarios completed successfully."
echo "Generated: $OUT"
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

# ==================== Scenario 7: --range-paths (split + bookmark + font color) ====================
echo "=== Scenario 7: --range-paths with font color ==="
RANGE_PARA=$($CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Paragraph for range-paths bookmark testing with enough characters." | sed 's/Created: //')
$CLI add "$OUT" --parent "$RANGE_PARA" --type-name bookmark --range-paths "${RANGE_PARA}[5..20]" --properties name=range1 color=FF0000
echo "  Added range-paths bookmark 'range1' at chars 5..20 with red font color"
$CLI get "$OUT" "$RANGE_PARA" --json

# ==================== Scenario 7b: --range-paths with font color + shading ===
echo "=== Scenario 7b: --range-paths with color + shading ==="
RANGE2_PARA=$($CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Another paragraph demonstrating range bookmark with both font color and background shading applied." | sed 's/Created: //')
$CLI add "$OUT" --parent "$RANGE2_PARA" --type-name bookmark --range-paths "${RANGE2_PARA}[10..30]" --properties name=range2 color=FF0000 shading=FFFF00
echo "  Added range-paths bookmark 'range2' at chars 10..30 with red font + yellow background"
$CLI get "$OUT" "$RANGE2_PARA" --json

# ==================== Scenario 8: Validate ====================
echo "=== Scenario 8: Validate ==="
$CLI validate "$OUT"

# ==================== Scenario 9: Batch — add bookmark + set color/shading ====================
echo "=== Scenario 9: Batch bookmark + set color/shading ==="
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Batch test paragraph one."
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Batch test paragraph two."
BATCH_PARA=$($CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Batch test paragraph three." | sed 's/Created: //')
BATCH_JSON="[{\"command\":\"add\",\"parent\":\"${BATCH_PARA}\",\"type\":\"bookmark\",\"properties\":{\"name\":\"batchBk1\",\"text\":\"three\"}},{\"command\":\"set\",\"path\":\"/body/p[9]/r[1]\",\"props\":{\"color\":\"0000FF\",\"shading\":\"CCFFCC\"}}]"
$CLI batch "$OUT" "$BATCH_JSON"
echo "  Batch: added bookmark 'batchBk1' + set color/shading in one call"
$CLI get "$OUT" "${BATCH_PARA}" --json

# ==================== Scenario 10: SET — modify font color and background on bookmark range runs ====================
echo "=== Scenario 10: SET font color + shading on range bookmark runs ==="
$CLI set "$OUT" '/body/p[7]/r[2]' color=0000FF shading=CCFFCC
echo "  Set p[7]/r[2] (range1 bookmarked run) to blue font + green background"
$CLI get "$OUT" '/body/p[7]/r[2]' --json

# ==================== Scenario 11: Batch — range-paths bookmark with color + shading ====================
echo "=== Scenario 11: Batch range-paths bookmark + set ==="
BATCH_RANGE_PARA=$($CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Batch range-paths bookmark paragraph with enough text content for testing." | sed 's/Created: //')
BATCH_RANGE_JSON="[{\"command\":\"add\",\"parent\":\"${BATCH_RANGE_PARA}\",\"type\":\"bookmark\",\"range_paths\":\"${BATCH_RANGE_PARA}[6..25]\",\"properties\":{\"name\":\"batchRange1\",\"color\":\"FF0000\",\"shading\":\"FFFF00\"}},{\"command\":\"set\",\"path\":\"/body/p[10]/r[1]\",\"props\":{\"color\":\"9900CC\",\"shading\":\"FFCC99\"}}]"
$CLI batch "$OUT" "$BATCH_RANGE_JSON"
echo "  Batch: range-paths bookmark chars 6..25 with red+yellow + set p[10]/r[1] purple+peach"
$CLI get "$OUT" "${BATCH_RANGE_PARA}" --json

echo ""
echo "All bookmark scenarios completed successfully."
echo "Generated: $OUT"
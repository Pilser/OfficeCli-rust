#!/bin/bash
# Generate complex table test documents (Word + Excel + PowerPoint)
# Includes merged cells, multi-level headers, formulas, charts, and other complex scenarios
# For testing officecli's table processing capabilities

set -e

echo "Using CLI: officecli"

DIR="$(dirname "$0")"

###############################################################################
# 1. Word Complex Table Document
###############################################################################
DOCX="$DIR/tables.docx"
echo ""
echo "=========================================="
echo "Generating Word complex table document: $DOCX"
echo "=========================================="

rm -f "$DOCX"
officecli create "$DOCX"
officecli add "$DOCX" --parent '/body' --type-name paragraph text="Complex Table Examples" style=Heading1 align=center
officecli add "$DOCX" --parent '/body' --type-name paragraph text=""

# -- Table 1: Project Progress Tracker (vertical merge vmerge) --
echo "  -> Table 1: Project Progress Tracker"
officecli add "$DOCX" --parent '/body' --type-name paragraph text="1. Project Progress Tracker" style=Heading2
officecli add "$DOCX" --parent '/body' --type-name table rows=7 cols=6

# Header
officecli set "$DOCX" '/body/tbl[1]/tr[1]/tc[1]' text="Project Name" bold=true shd=4472C4 color=FFFFFF valign=center
officecli set "$DOCX" '/body/tbl[1]/tr[1]/tc[2]' text="Phase" bold=true shd=4472C4 color=FFFFFF
officecli set "$DOCX" '/body/tbl[1]/tr[1]/tc[3]' text="Owner" bold=true shd=4472C4 color=FFFFFF
officecli set "$DOCX" '/body/tbl[1]/tr[1]/tc[4]' text="Start Date" bold=true shd=4472C4 color=FFFFFF
officecli set "$DOCX" '/body/tbl[1]/tr[1]/tc[5]' text="End Date" bold=true shd=4472C4 color=FFFFFF
officecli set "$DOCX" '/body/tbl[1]/tr[1]/tc[6]' text="Progress" bold=true shd=4472C4 color=FFFFFF

# Project A - Smart Office System (merge 3 rows)
officecli set "$DOCX" '/body/tbl[1]/tr[2]/tc[1]' text="Smart Office System" vmerge=restart valign=center shd=D9E2F3
officecli set "$DOCX" '/body/tbl[1]/tr[2]/tc[2]' text="Requirements"
officecli set "$DOCX" '/body/tbl[1]/tr[2]/tc[3]' text="John"
officecli set "$DOCX" '/body/tbl[1]/tr[2]/tc[4]' text="2025-01-05"
officecli set "$DOCX" '/body/tbl[1]/tr[2]/tc[5]' text="2025-02-15"
officecli set "$DOCX" '/body/tbl[1]/tr[2]/tc[6]' text="100%" color=00B050

officecli set "$DOCX" '/body/tbl[1]/tr[3]/tc[1]' text="" vmerge=continue shd=D9E2F3
officecli set "$DOCX" '/body/tbl[1]/tr[3]/tc[2]' text="Development"
officecli set "$DOCX" '/body/tbl[1]/tr[3]/tc[3]' text="Sarah"
officecli set "$DOCX" '/body/tbl[1]/tr[3]/tc[4]' text="2025-02-16"
officecli set "$DOCX" '/body/tbl[1]/tr[3]/tc[5]' text="2025-06-30"
officecli set "$DOCX" '/body/tbl[1]/tr[3]/tc[6]' text="75%" color=FFC000

officecli set "$DOCX" '/body/tbl[1]/tr[4]/tc[1]' text="" vmerge=continue shd=D9E2F3
officecli set "$DOCX" '/body/tbl[1]/tr[4]/tc[2]' text="Testing"
officecli set "$DOCX" '/body/tbl[1]/tr[4]/tc[3]' text="Mike"
officecli set "$DOCX" '/body/tbl[1]/tr[4]/tc[4]' text="2025-07-01"
officecli set "$DOCX" '/body/tbl[1]/tr[4]/tc[5]' text="2025-08-31"
officecli set "$DOCX" '/body/tbl[1]/tr[4]/tc[6]' text="0%" color=FF0000

# Project B - Data Platform Upgrade (merge 3 rows)
officecli set "$DOCX" '/body/tbl[1]/tr[5]/tc[1]' text="Data Platform Upgrade" vmerge=restart valign=center shd=E2EFDA
officecli set "$DOCX" '/body/tbl[1]/tr[5]/tc[2]' text="Architecture"
officecli set "$DOCX" '/body/tbl[1]/tr[5]/tc[3]' text="Emily"
officecli set "$DOCX" '/body/tbl[1]/tr[5]/tc[4]' text="2025-03-01"
officecli set "$DOCX" '/body/tbl[1]/tr[5]/tc[5]' text="2025-04-15"
officecli set "$DOCX" '/body/tbl[1]/tr[5]/tc[6]' text="100%" color=00B050

officecli set "$DOCX" '/body/tbl[1]/tr[6]/tc[1]' text="" vmerge=continue shd=E2EFDA
officecli set "$DOCX" '/body/tbl[1]/tr[6]/tc[2]' text="Migration"
officecli set "$DOCX" '/body/tbl[1]/tr[6]/tc[3]' text="David"
officecli set "$DOCX" '/body/tbl[1]/tr[6]/tc[4]' text="2025-04-16"
officecli set "$DOCX" '/body/tbl[1]/tr[6]/tc[5]' text="2025-07-31"
officecli set "$DOCX" '/body/tbl[1]/tr[6]/tc[6]' text="40%" color=FFC000

officecli set "$DOCX" '/body/tbl[1]/tr[7]/tc[1]' text="" vmerge=continue shd=E2EFDA
officecli set "$DOCX" '/body/tbl[1]/tr[7]/tc[2]' text="Acceptance"
officecli set "$DOCX" '/body/tbl[1]/tr[7]/tc[3]' text="Lisa"
officecli set "$DOCX" '/body/tbl[1]/tr[7]/tc[4]' text="2025-08-01"
officecli set "$DOCX" '/body/tbl[1]/tr[7]/tc[5]' text="2025-09-30"
officecli set "$DOCX" '/body/tbl[1]/tr[7]/tc[6]' text="0%" color=FF0000

# -- Table 2: Financial Statement (gridspan horizontal merge + vmerge vertical merge) --
echo "  -> Table 2: Financial Statement"
officecli add "$DOCX" --parent '/body' --type-name paragraph text=""
officecli add "$DOCX" --parent '/body' --type-name paragraph text="2. Financial Statement" style=Heading2
officecli add "$DOCX" --parent '/body' --type-name table rows=8 cols=5

# Header row 1 - gridspan=2 automatically removes merged tc
officecli set "$DOCX" '/body/tbl[2]/tr[1]/tc[1]' text="Category" bold=true shd=2E75B6 color=FFFFFF vmerge=restart valign=center
officecli set "$DOCX" '/body/tbl[2]/tr[1]/tc[2]' text="Line Item" bold=true shd=2E75B6 color=FFFFFF vmerge=restart valign=center
officecli set "$DOCX" '/body/tbl[2]/tr[1]/tc[3]' text="Amount (10K USD)" bold=true shd=2E75B6 color=FFFFFF gridspan=2 align=center
# gridspan=2 removed original tc[4], original tc[5] becomes tc[4]
officecli set "$DOCX" '/body/tbl[2]/tr[1]/tc[4]' text="Notes" bold=true shd=2E75B6 color=FFFFFF vmerge=restart valign=center

# Header row 2
officecli set "$DOCX" '/body/tbl[2]/tr[2]/tc[1]' text="" vmerge=continue shd=2E75B6
officecli set "$DOCX" '/body/tbl[2]/tr[2]/tc[2]' text="" vmerge=continue shd=2E75B6
officecli set "$DOCX" '/body/tbl[2]/tr[2]/tc[3]' text="Budget" bold=true shd=5B9BD5 color=FFFFFF align=center
officecli set "$DOCX" '/body/tbl[2]/tr[2]/tc[4]' text="Actual" bold=true shd=5B9BD5 color=FFFFFF align=center
officecli set "$DOCX" '/body/tbl[2]/tr[2]/tc[5]' text="" vmerge=continue shd=2E75B6

# Revenue (merge 3 rows)
officecli set "$DOCX" '/body/tbl[2]/tr[3]/tc[1]' text="Revenue" vmerge=restart valign=center shd=DEEAF6 bold=true
officecli set "$DOCX" '/body/tbl[2]/tr[3]/tc[2]' text="Product Sales"
officecli set "$DOCX" '/body/tbl[2]/tr[3]/tc[3]' text="500.00" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[3]/tc[4]' text="523.50" align=right color=00B050
officecli set "$DOCX" '/body/tbl[2]/tr[3]/tc[5]' text="Exceeded"

officecli set "$DOCX" '/body/tbl[2]/tr[4]/tc[1]' text="" vmerge=continue shd=DEEAF6
officecli set "$DOCX" '/body/tbl[2]/tr[4]/tc[2]' text="Consulting Services"
officecli set "$DOCX" '/body/tbl[2]/tr[4]/tc[3]' text="200.00" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[4]/tc[4]' text="185.30" align=right color=FF0000
officecli set "$DOCX" '/body/tbl[2]/tr[4]/tc[5]' text="Below target"

officecli set "$DOCX" '/body/tbl[2]/tr[5]/tc[1]' text="" vmerge=continue shd=DEEAF6
officecli set "$DOCX" '/body/tbl[2]/tr[5]/tc[2]' text="Tech Licensing"
officecli set "$DOCX" '/body/tbl[2]/tr[5]/tc[3]' text="80.00" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[5]/tc[4]' text="92.00" align=right color=00B050
officecli set "$DOCX" '/body/tbl[2]/tr[5]/tc[5]' text="New partners"

# Expenses (merge 3 rows)
officecli set "$DOCX" '/body/tbl[2]/tr[6]/tc[1]' text="Expenses" vmerge=restart valign=center shd=FFF2CC bold=true
officecli set "$DOCX" '/body/tbl[2]/tr[6]/tc[2]' text="Labor Cost"
officecli set "$DOCX" '/body/tbl[2]/tr[6]/tc[3]' text="320.00" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[6]/tc[4]' text="335.00" align=right color=FF0000
officecli set "$DOCX" '/body/tbl[2]/tr[6]/tc[5]' text="New hires"

officecli set "$DOCX" '/body/tbl[2]/tr[7]/tc[1]' text="" vmerge=continue shd=FFF2CC
officecli set "$DOCX" '/body/tbl[2]/tr[7]/tc[2]' text="Operating Expenses"
officecli set "$DOCX" '/body/tbl[2]/tr[7]/tc[3]' text="150.00" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[7]/tc[4]' text="142.80" align=right color=00B050
officecli set "$DOCX" '/body/tbl[2]/tr[7]/tc[5]' text="Cost savings"

officecli set "$DOCX" '/body/tbl[2]/tr[8]/tc[1]' text="" vmerge=continue shd=FFF2CC
officecli set "$DOCX" '/body/tbl[2]/tr[8]/tc[2]' text="R&D Investment"
officecli set "$DOCX" '/body/tbl[2]/tr[8]/tc[3]' text="180.00" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[8]/tc[4]' text="195.50" align=right
officecli set "$DOCX" '/body/tbl[2]/tr[8]/tc[5]' text="Strategic investment"

# -- Table 3: Skill Assessment Matrix (color heatmap) --
echo "  -> Table 3: Skill Assessment Matrix"
officecli add "$DOCX" --parent '/body' --type-name paragraph text=""
officecli add "$DOCX" --parent '/body' --type-name paragraph text="3. Skill Assessment Matrix" style=Heading2
officecli add "$DOCX" --parent '/body' --type-name table rows=6 cols=7

# Header
officecli set "$DOCX" '/body/tbl[3]/tr[1]/tc[1]' text="Name/Skill" bold=true shd=002060 color=FFFFFF align=center
for col_data in "2:Python" "3:Java" "4:Frontend" "5:Database" "6:DevOps" "7:AI/ML"; do
    col="${col_data%%:*}"; name="${col_data#*:}"
    officecli set "$DOCX" "/body/tbl[3]/tr[1]/tc[$col]" text="$name" bold=true shd=002060 color=FFFFFF align=center
done

# Colors: Expert=00B050(dark green) Proficient=92D050(light green) Familiar=FFC000(yellow) Beginner=FF0000(red)
fill_skill_row() {
    local row=$1 person=$2; shift 2
    officecli set "$DOCX" "/body/tbl[3]/tr[$row]/tc[1]" text="$person" bold=true shd=D6DCE4 align=center
    local col=2
    for cell in "$@"; do
        local text="${cell%%:*}" color="${cell#*:}"
        officecli set "$DOCX" "/body/tbl[3]/tr[$row]/tc[$col]" text="$text" shd="$color" color=FFFFFF align=center bold=true
        ((col++))
    done
}
fill_skill_row 2 John   Expert:00B050 Proficient:92D050 Familiar:FFC000 Expert:00B050 Familiar:FFC000 Expert:00B050
fill_skill_row 3 Sarah  Proficient:92D050 Expert:00B050 Expert:00B050 Proficient:92D050 Familiar:FFC000 Beginner:FF0000
fill_skill_row 4 Mike   Familiar:FFC000 Familiar:FFC000 Expert:00B050 Familiar:FFC000 Expert:00B050 Proficient:92D050
fill_skill_row 5 Emily  Expert:00B050 Beginner:FF0000 Familiar:FFC000 Expert:00B050 Proficient:92D050 Familiar:FFC000
fill_skill_row 6 David  Proficient:92D050 Proficient:92D050 Proficient:92D050 Expert:00B050 Expert:00B050 Expert:00B050

echo "  Done: Word document: $DOCX"

###############################################################################
# 2. Excel Sales Report
###############################################################################
XLSX="$DIR/tables.xlsx"
echo ""
echo "=========================================="
echo "Generating Excel sales report: $XLSX"
echo "=========================================="

rm -f "$XLSX"
officecli create "$XLSX"

# Sheet1: Sales Data
echo "  -> Sheet1: Sales Data"
officecli set "$XLSX" '/Sheet1/A1' value="2025 Annual Sales Report"
officecli set "$XLSX" '/Sheet1/A2' value="Department"
officecli set "$XLSX" '/Sheet1/B2' value="Q1"
officecli set "$XLSX" '/Sheet1/C2' value="Q2"
officecli set "$XLSX" '/Sheet1/D2' value="Q3"
officecli set "$XLSX" '/Sheet1/E2' value="Q4"
officecli set "$XLSX" '/Sheet1/F2' value="Annual Total"

for entry in "3:Engineering:128000:156000:189000:210000" \
             "4:Marketing:95000:112000:138000:165000" \
             "5:Operations:76000:89000:102000:118000" \
             "6:Sales:230000:275000:310000:356000" \
             "7:HR:45000:48000:52000:55000"; do
    IFS=':' read -r row dept q1 q2 q3 q4 <<< "$entry"
    officecli set "$XLSX" "/Sheet1/A$row" value="$dept"
    officecli set "$XLSX" "/Sheet1/B$row" value="$q1"
    officecli set "$XLSX" "/Sheet1/C$row" value="$q2"
    officecli set "$XLSX" "/Sheet1/D$row" value="$q3"
    officecli set "$XLSX" "/Sheet1/E$row" value="$q4"
    officecli set "$XLSX" "/Sheet1/F$row" formula="SUM(B${row}:E${row})"
done

# Total row
officecli set "$XLSX" '/Sheet1/A8' value="Total"
for col in B C D E F; do
    officecli set "$XLSX" "/Sheet1/${col}8" formula="SUM(${col}3:${col}7)"
done

# Growth rate
officecli set "$XLSX" '/Sheet1/A9' value="Quarterly Growth Rate"
officecli set "$XLSX" '/Sheet1/C9' formula="(C8-B8)/B8"
officecli set "$XLSX" '/Sheet1/D9' formula="(D8-C8)/C8"
officecli set "$XLSX" '/Sheet1/E9' formula="(E8-D8)/D8"

# Sheet2: Employee Performance
echo "  -> Sheet2: Performance"
officecli add "$XLSX" / --type-name sheet name="Performance"

officecli set "$XLSX" '/Performance/A1' value="Employee Performance Review"
officecli set "$XLSX" '/Performance/A2' value="Name"
officecli set "$XLSX" '/Performance/B2' value="Department"
officecli set "$XLSX" '/Performance/C2' value="Performance Score"
officecli set "$XLSX" '/Performance/D2' value="Capability Score"
officecli set "$XLSX" '/Performance/E2' value="Attitude Score"
officecli set "$XLSX" '/Performance/F2' value="Total Score"
officecli set "$XLSX" '/Performance/G2' value="Grade"

declare -a EMP_DATA=(
    "3:John:Engineering:92:88:95"
    "4:Sarah:Marketing:85:90:78"
    "5:Mike:Operations:78:82:90"
    "6:Emily:Sales:96:75:88"
    "7:David:Engineering:88:92:85"
    "8:Lisa:HR:72:85:92"
    "9:Tom:Sales:91:78:80"
    "10:Amy:Marketing:65:70:88"
    "11:Chris:Engineering:95:93:90"
    "12:Kate:Operations:80:86:75"
)

for emp in "${EMP_DATA[@]}"; do
    IFS=':' read -r row name dept s1 s2 s3 <<< "$emp"
    officecli set "$XLSX" "/Performance/A$row" value="$name"
    officecli set "$XLSX" "/Performance/B$row" value="$dept"
    officecli set "$XLSX" "/Performance/C$row" value="$s1"
    officecli set "$XLSX" "/Performance/D$row" value="$s2"
    officecli set "$XLSX" "/Performance/E$row" value="$s3"
    officecli set "$XLSX" "/Performance/F$row" formula="C${row}*0.4+D${row}*0.35+E${row}*0.25"
    officecli set "$XLSX" "/Performance/G$row" formula="IF(F${row}>=90,\"A\",IF(F${row}>=80,\"B\",IF(F${row}>=70,\"C\",\"D\")))"
done

# Sheet3: Summary
echo "  -> Sheet3: Summary"
officecli add "$XLSX" / --type-name sheet name="Summary"

officecli set "$XLSX" '/Summary/A1' value="Metric"
officecli set "$XLSX" '/Summary/B1' value="Value"
officecli set "$XLSX" '/Summary/A2' value="Highest Score"
officecli set "$XLSX" '/Summary/B2' formula="MAX(Performance!F3:F12)"
officecli set "$XLSX" '/Summary/A3' value="Lowest Score"
officecli set "$XLSX" '/Summary/B3' formula="MIN(Performance!F3:F12)"
officecli set "$XLSX" '/Summary/A4' value="Average Score"
officecli set "$XLSX" '/Summary/B4' formula="AVERAGE(Performance!F3:F12)"
officecli set "$XLSX" '/Summary/A5' value="Grade A Count"
officecli set "$XLSX" '/Summary/B5' formula="COUNTIF(Performance!G3:G12,\"A\")"
officecli set "$XLSX" '/Summary/A6' value="Annual Total Sales"
officecli set "$XLSX" '/Summary/B6' formula="Sheet1!F8"

echo "  Done: Excel document: $XLSX"

###############################################################################
# 3. PowerPoint Data Report
###############################################################################
PPTX="$DIR/tables.pptx"
echo ""
echo "=========================================="
echo "Generating PowerPoint data report: $PPTX"
echo "=========================================="

rm -f "$PPTX"
officecli create "$PPTX"

# Slide 1: Title Page
echo "  -> Slide 1: Title Page"
officecli add "$PPTX" /presentation/slides --type-name slide
officecli raw-set "$PPTX" '/slide[1]' --xpath "/p:sld" --action replace --xml '<p:sld>
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="1F3864"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="1500000" y="2000000"/><a:ext cx="9192000" cy="1200000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
        <p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="4000" b="1" dirty="0"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>2025 Annual Data Analysis Report</a:t></a:r></a:p></p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Subtitle"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="2500000" y="3500000"/><a:ext cx="7192000" cy="800000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
        <p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="2000" dirty="0"><a:solidFill><a:srgbClr val="BDD7EE"/></a:solidFill></a:rPr><a:t>Dept Comparison | Performance Overview | Financial Summary</a:t></a:r></a:p></p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>'

# Slide 2: Data Table
echo "  -> Slide 2: Data Table"
officecli add "$PPTX" /presentation/slides --type-name slide
officecli raw-set "$PPTX" '/slide[2]' --xpath "/p:sld" --action replace --xml '<p:sld>
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="500000" y="200000"/><a:ext cx="11192000" cy="600000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
        <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="l"/><a:r><a:rPr lang="en-US" sz="2800" b="1" dirty="0"><a:solidFill><a:srgbClr val="1F3864"/></a:solidFill></a:rPr><a:t>Quarterly Sales by Department</a:t></a:r></a:p></p:txBody>
      </p:sp>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="4" name="Table"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="500000" y="1000000"/><a:ext cx="11192000" cy="4500000"/></p:xfrm>
        <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
          <a:tbl>
            <a:tblPr firstRow="1" bandRow="1"/>
            <a:tblGrid><a:gridCol w="2238400"/><a:gridCol w="2238400"/><a:gridCol w="2238400"/><a:gridCol w="2238400"/><a:gridCol w="2238400"/></a:tblGrid>
            <a:tr h="700000">
              <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1600" b="1" dirty="0"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>Department</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="2E75B6"/></a:solidFill></a:tcPr></a:tc>
              <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1600" b="1" dirty="0"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>Q1</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="2E75B6"/></a:solidFill></a:tcPr></a:tc>
              <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1600" b="1" dirty="0"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>Q2</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="2E75B6"/></a:solidFill></a:tcPr></a:tc>
              <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1600" b="1" dirty="0"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>Q3</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="2E75B6"/></a:solidFill></a:tcPr></a:tc>
              <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1600" b="1" dirty="0"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr><a:t>Q4</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="2E75B6"/></a:solidFill></a:tcPr></a:tc>
            </a:tr>
            <a:tr h="700000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>Engineering</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="DEEAF6"/></a:solidFill></a:tcPr></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>128,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>156,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>189,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>210,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
            <a:tr h="700000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>Marketing</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="DEEAF6"/></a:solidFill></a:tcPr></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>95,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>112,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>138,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>165,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
            <a:tr h="700000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>Operations</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="DEEAF6"/></a:solidFill></a:tcPr></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>76,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>89,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>102,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>118,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
            <a:tr h="700000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>Sales</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="DEEAF6"/></a:solidFill></a:tcPr></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>230,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>275,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>310,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>356,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
            <a:tr h="700000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>HR</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="DEEAF6"/></a:solidFill></a:tcPr></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>45,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>48,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>52,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1400" dirty="0"/><a:t>55,000</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
          </a:tbl>
        </a:graphicData></a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>'

# Slide 3: Pie Chart Analysis
echo "  -> Slide 3: Pie Chart Analysis"
officecli add "$PPTX" /presentation/slides --type-name slide
officecli add "$PPTX" '/slide[3]' --type-name shape text="Annual Sales Share by Department" size=28 bold=true x=500000 y=200000 width=11192000 height=600000
officecli add "$PPTX" '/slide[3]' --type-name shape text="Engineering 683,000 (24.4%)" x=1000000 y=1200000 width=10000000 height=500000
officecli add "$PPTX" '/slide[3]' --type-name shape text="Marketing 510,000 (18.2%)" x=1000000 y=1900000 width=10000000 height=500000
officecli add "$PPTX" '/slide[3]' --type-name shape text="Operations 385,000 (13.7%)" x=1000000 y=2600000 width=10000000 height=500000
officecli add "$PPTX" '/slide[3]' --type-name shape text="Sales 1,171,000 (41.8%)" x=1000000 y=3300000 width=10000000 height=500000
officecli add "$PPTX" '/slide[3]' --type-name shape text="HR 200,000 (7.1%)" x=1000000 y=4000000 width=10000000 height=500000

echo "  Done: PowerPoint document: $PPTX"

###############################################################################
# Verification
###############################################################################
echo ""
echo "=========================================="
echo "Verifying all files"
echo "=========================================="
officecli view "$DOCX" outline
echo ""
officecli view "$XLSX" outline
echo ""
officecli view "$PPTX" outline
echo ""
ls -lh "$DOCX" "$XLSX" "$PPTX"
echo ""
echo "All done!"

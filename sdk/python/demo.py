#!/usr/bin/env python3
"""
Demo: build a small sales report .xlsx using the officecli Python SDK.

Run:  python3 demo.py [path-to-officecli-binary]

Shows the whole loop over a single resident:
  create -> writes applied as one batch -> read back -> save -> close -> reopen.
"""

import os
import sys
import officecli   # the client

BIN = sys.argv[1] if len(sys.argv) > 1 else "officecli"
OUT = os.path.abspath("sales_report.xlsx")

ROWS = [
    ("North", 120, 9.5),
    ("South", 95, 11.0),
    ("East",  140, 8.75),
    ("West",  60, 12.5),
    ("Central", 110, 10.0),
]

COL = "ABCDE"


def cell(c, r):
    return f"/Sheet1/{c}{r}"


def main():
    with officecli.create(OUT, "--force", binary=BIN) as doc:
        items = []

        # Header row
        for j, title in enumerate(["Region", "Units", "Price", "Revenue"]):
            items.append({"command": "set", "path": cell(COL[j], 1),
                          "props": {"text": title, "bold": "true"}})

        # Data rows + a live formula for Revenue (=Units*Price)
        for i, (region, units, price) in enumerate(ROWS, start=2):
            items.append({"command": "set", "path": cell("A", i), "props": {"text": region}})
            items.append({"command": "set", "path": cell("B", i), "props": {"text": str(units)}})
            items.append({"command": "set", "path": cell("C", i), "props": {"text": str(price)}})
            items.append({"command": "set", "path": cell("D", i), "props": {"formula": f"=B{i}*C{i}"}})

        # Totals row
        last = len(ROWS) + 1
        items.append({"command": "set", "path": cell("A", last + 1),
                      "props": {"text": "TOTAL", "bold": "true"}})
        items.append({"command": "set", "path": cell("B", last + 1),
                      "props": {"formula": f"=SUM(B2:B{last})"}})
        items.append({"command": "set", "path": cell("D", last + 1),
                      "props": {"formula": f"=SUM(D2:D{last})"}})

        doc.batch(items)

        # Read one cell back
        node = doc.send({"command": "get", "path": cell("A", 1)})
        print("A1 reads back as:", node)

        # Validate in-session
        v = doc.send({"command": "validate"})
        print("validate (in-session):", "OK" if v else v)

        doc.send({"command": "save"})

    # Round-trip proof: reopen and confirm
    with officecli.open(OUT, binary=BIN) as doc:
        v = doc.send({"command": "validate"})
        print("validate (reopened):", "OK" if v else v)
        a1 = doc.send({"command": "get", "path": cell("A", 1)})
        print("A1 after reopen:", a1)

    print(f"wrote {OUT} ({os.path.getsize(OUT)} bytes)")


if __name__ == "__main__":
    main()

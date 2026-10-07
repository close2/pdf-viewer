#!/usr/bin/env python3
"""Seed the `aform` fuzz target with one-call scripts, every function and every argument shape.

    python3 fuzz/seed_aform.py fuzz/corpus/aform

**Why a generator.** The target's input is a field's `/JS` text, its value and a keystroke's
change, NUL-separated (`fuzz/fuzz_targets/aform.rs`). A mutator handed nothing will not spell
`AFNumber_Format` before it finds a parenthesis, so every one of the twenty-one names is written
here, under each argument shape the grammar takes — numbers with a sign and an exponent, both
quote characters with each escape, `true` and `false`, an array of strings, no arguments, fewer
and more than the function declares — and each against a value of the kind the function reads:
a number, a date in its picture, a phone number, a mask's filling. Then the malformed: a string
that never ends, a number of four hundred digits and one past the double range, quotes nested
inside the other kind, an escape at the end, ten thousand arguments, a script at the grammar's
64 KiB bound and one past it. Each seed is named by its content, so a re-run adds nothing new.
"""

import hashlib
import os
import sys

FUNCTIONS = [
    ("AFNumber_Format", ["2, 0, 0, 0, \"$\", true", "0, 1, 1, 0, \"€\", false",
                         "3, 2, 2, 0, '', true", "2, 3, 3, 0, \"kr\", false", "2, 4, 0, 0, \"\", 1",
                         "-1, 0, 0, 0, \"\", true", "1e3, 9, 9, 0, \"\", true"], "-1234.5678"),
    ("AFNumber_Keystroke", ["2, 0, 0, 0, \"$\", true", "2, 2, 0, 0, \"\", false"], "12,5"),
    ("AFPercent_Format", ["2, 0", "0, 1, true", "4, 3"], "0.12345"),
    ("AFPercent_Keystroke", ["2, 0", "1, 2"], "45"),
    ("AFDate_Format", ["0", "13", "\"mm/dd/yyyy\"", "14", "-1", "2.5"], "02/29/2024"),
    ("AFDate_FormatEx", ["\"mm/dd/yyyy\"", "\"dddd, mmmm d, yyyy\"", "'yy-mm-dd HH:MM:ss tt'",
                         "\"\\\"quoted\\\" mmm\""], "2024-02-29"),
    ("AFDate_Keystroke", ["2", "\"m/d/yy h:MM tt\""], "2/29/24 11:05 pm"),
    ("AFDate_KeystrokeEx", ["\"mm/dd/yyyy\"", "\"d-mmm-yy\""], "29-Feb-24"),
    ("AFTime_Format", ["0", "1", "2", "3", "4"], "23:05:09"),
    ("AFTime_FormatEx", ["\"HH:MM\"", "\"h:MM:ss tt\""], "11:05 PM"),
    ("AFTime_Keystroke", ["0", "3"], "11:05:09 pm"),
    ("AFTime_KeystrokeEx", ["\"HH:MM:ss\""], "23:05:09"),
    ("AFSpecial_Format", ["0", "1", "2", "3", "4", "\"2\""], "5551234567"),
    ("AFSpecial_Keystroke", ["0", "1", "2", "3"], "123-45-6789"),
    ("AFSpecial_KeystrokeEx", ["\"(999) 999-9999\"", "\"AAA-XXX-OOO*\"", "\"\\\\9999\"",
                               "\"\""], "(555) 123-4567"),
    ("AFSimple_Calculate", ["\"SUM\", new Array(\"a\")", "\"SUM\", [\"Line1\", \"Line2\"]",
                            "\"AVG\", \"a, b, c\"", "\"PRD\", ['x']", "\"MIN\", \"\"",
                            "\"MAX\", [\"a\", \"b\"]", "\"MED\", \"a\""], "3.5"),
    ("AFRange_Validate", ["true, 0, true, 100", "false, 0, true, -1e308",
                          "true, \"5\", false, 0"], "50"),
    ("AFMakeNumber", ["\"1,5\""], "1,5"),
    ("AFExtractNums", ["\"a1b22c333\""], "a1b22c333"),
    ("AFMergeChange", ["event"], "abc"),
    ("AFParseDateEx", ["\"2/29/2024\", \"mm/dd/yyyy\""], "2/29/2024"),
]

MALFORMED = [
    "AFNumber_Format(2, 0, 0, 0, \"$",
    "AFNumber_Format(2, 0, 0, 0, '$\", true)",
    "AFDate_FormatEx(\"mm/dd/yyyy",
    "AFDate_FormatEx('mm\\')",
    "AFDate_FormatEx(\"it's \\\"x\\\" 'y'\")",
    "AFDate_FormatEx('say \"hi\" \\'there\\'')",
    "AFNumber_Format(" + "9" * 400 + ", 0, 0, 0, \"\", true)",
    "AFNumber_Format(1e309, 0, 0, 0, \"\", true)",
    "AFNumber_Format(-1e-400, 0, 0, 0, \"\", true)",
    "AFPercent_Format(4294967296, 0)",
    "AFSpecial_KeystrokeEx(\"\\u00e9\\x41\\0\\n\\t\\\\\")",
    "AFSpecial_KeystrokeEx(\"\\u{1F600}\\uD800\")",
    "AFSimple_Calculate(\"SUM\", [\"a\", ])",
    "AFSimple_Calculate(\"SUM\", [[\"a\"]])",
    "AFNumber_Format(2,,0)",
    "AFNumber_Format(2 0)",
    "AFNumber_Format(2, 0);;",
    "AFNumber_Format(2, 0); AFNumber_Format(2, 0)",
    "AFNumber_Format (  2  ,  0  )  ;  ",
    "AFNumber_Format(- 2, +0)",
    "AFFoo_Bar(1)",
    "AFNumber_Format",
    "AFNumber_Format(",
    "AFNumber_Format)",
    "// AFNumber_Format(2)",
    "AFNumber_Format(2) /* c */",
    "AFNumber_Format(0x10, 0b1, 1_000)",
    "AFNumber_Format(.5, 5., -.e1)",
    "AFNumber_Format(" + ", ".join(str(n) for n in range(10000)) + ")",
    "AFSimple_Calculate(\"SUM\", [" + ", ".join(f"\"f{n}\"" for n in range(10000)) + "])",
    "AFDate_FormatEx(\"" + "mm" * 32000 + "\")",
    "AFDate_FormatEx(\"" + "y" * 70000 + "\")",
    "AFSpecial_KeystrokeEx(\"" + "9" * 30000 + "\")",
]

VALUES = ["", "0", "-0", "1e308", "-1e-308", "NaN", "Infinity", "1.005", "  42  ", "1,234.56",
          "1.234,56", "12/31/9999", "2/30/2024", "00:00:00", "12:00 am", "١٢",
          "１２", "\U0001f600", "9" * 400]


def write(out, script, value, change=""):
    data = script.encode("utf-8", "surrogatepass") + b"\0" + value.encode("utf-8") + b"\0" + \
        change.encode("utf-8")
    path = os.path.join(out, hashlib.sha256(data).hexdigest()[:40])
    if os.path.exists(path):
        return 0
    with open(path, "wb") as handle:
        handle.write(data)
    return 1


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "fuzz/corpus/aform"
    os.makedirs(out, exist_ok=True)
    added = 0
    for name, shapes, value in FUNCTIONS:
        added += write(out, f"{name}()", value)
        for shape in shapes:
            added += write(out, f"{name}({shape});", value, value[-1:])
            added += write(out, f" {name} ( {shape} ) ", "", value)
        for other in VALUES[:6]:
            added += write(out, f"{name}({shapes[0]})", other, "mm/dd/yyyy")
    for script in MALFORMED:
        added += write(out, script, "1234.5", "9")
    for value in VALUES:
        added += write(out, "AFNumber_Keystroke(2, 0, 0, 0, \"\", true)", value, value)
        added += write(out, "AFDate_KeystrokeEx(\"mm/dd/yyyy\")", value, "dddd mmmm")
    print(f"seed_aform: {added} new seeds in {out}")


if __name__ == "__main__":
    main()

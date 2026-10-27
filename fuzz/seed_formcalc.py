#!/usr/bin/env python3
"""Seed the `formcalc` fuzz target with scripts that reach the grammar and the evaluator.

    python3 fuzz/seed_formcalc.py fuzz/corpus/formcalc

The target's framing: the script, then after a NUL byte the values of the form's fields `A`, `B`
and `C` separated by further NUL bytes. The seeds are written for this file — the shapes of XFA
3.3 chapter 25's examples (declarations and their blocks, the three loops, user-defined
functions, the operators' promotions) and of the calculations a form usually carries (a sum of
two fields, a field rewritten in place, a name joined from its parts) — and none is a document's
script.
"""

import hashlib
import os
import sys

SCRIPTS = [
    "A + B * C",
    "(A - 1) / (B + 1)",
    "A / 0 + 1",
    '"10" + "abc" - null',
    "5 + 1 \"Hello\" 10 * 3 + 5 * 4",
    "if (A > B) then A else B endif",
    'if ("abc") then 10 else 20 endif',
    "var i = 0 while (i < 10) do i = i + 1 endwhile i",
    "var s = 0 for i = 1 upto 100 step 3 do s = s + i endfor s",
    "var s = 0 for i = 10 downto 1 do s = s + i endfor s",
    "var n = 0 foreach v in (A, B, C, 7) do n = n + v endfor n",
    "var x = 3 do var x = 1 end x",
    "func Square(p) do p * p endfunc Square(A) + Square(B)",
    "func Down(p) do if (p > 0) then Down(p - 1) else 0 endif endfunc Down(5)",
    "func Loop(p) do Loop(p) endfunc Loop(1)",
    "Sum(A, B, C) * 2",
    "Sum(Row[*])",
    "Sum(Row[1], Row[2])",
    'Concat("a", A, null, "b")',
    'Replace(A, "x", "yy")',
    'Replace(A, "", "z")',
    "HasValue(A) and HasValue(B)",
    'HasValue(" ")',
    "Out = A + B",
    "$ = Out",
    "Out = Out + 1; Out",
    "Row[0] = 9",
    "Row[-1]",
    "Row[+1]",
    "A.B.C",
    "A..B",
    "A.#C",
    "A.*",
    "Abs(A)",
    "Sqrt(4)",
    "A.method()",
    "break",
    "var a = 1; var a = 2; a",
    '"unterminated',
    '"\\u00e9" & "\\uD800"',
    "1e308 * 10",
    "0.1 + 0.2",
    ".5 + 5.",
    "A <> B or A == B and not A",
    "A lt B ge C eq 1 ne 2",
    "- - - A",
    "((((((((((A))))))))))",
    "// a comment\nA ; another\nB",
    "A\x00\x00B\x00C",
    "",
]

FORMS = [b"", b"\x001\x002\x003", b"\x00x\x00y\x00z", b"\x00-1.5\x001e3\x000", b"\x00\x00\x00"]


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    written = 0
    for script in SCRIPTS:
        for form in FORMS[:2] if script else FORMS[:1]:
            data = script.encode("utf-8", "surrogatepass") + form
            name = os.path.join(directory, hashlib.sha256(data).hexdigest())
            if not os.path.exists(name):
                with open(name, "wb") as handle:
                    handle.write(data)
                written += 1
    for form in FORMS[2:]:
        data = b"A + B + C" + form
        name = os.path.join(directory, hashlib.sha256(data).hexdigest())
        if not os.path.exists(name):
            with open(name, "wb") as handle:
                handle.write(data)
            written += 1
    print(f"seed_formcalc.py: {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)

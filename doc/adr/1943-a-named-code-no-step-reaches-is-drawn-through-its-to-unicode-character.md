# 1943 — A named code no step reaches is drawn through its `/ToUnicode` character

Session 1553. Status: **accepted** and **built** in `pdf-font`. Context: ADRs 0015, 0520, 1942.
ISO 32000-2 §9.6.5.4, Annex D.2.

## 1. What was wrong

§9.6.5.4 ends: "If a character cannot be mapped in any of the ways described previously, a PDF
processor may supply a mapping of its choosing." This reader's first choice offered the code to
every `cmap` subtable in the font's order. For a non-symbolic font with an `/Encoding` the code is
a position in that named encoding, so offering it to a (1, 0) subtable reads a WinAnsi code as a
Mac OS Roman one. Pol.pdf's OpenPDF subsets name code 240 (octal) `space` — the duplicate Annex
D.2's note says Code Page 1252 makes U+00A0 — and carry no glyph for U+0020: the (1, 0) subtable
listed first drew a superscript two in "50 %" and a ž in the bold face. mupdf draws
`nonbreakingspace`, which is evidence about our reading and not the reason for the change.

## 2. Decision

For a non-symbolic font with an `/Encoding`, the first mapping of this processor's choosing asks
the (3, 1) subtable for the one character the font's own `/ToUnicode` (§9.10.3) states for the
code. The code is still recorded as reached by a reader's choice, so ISO 19005-2 section
6.2.11.6 still reports it (ADR 1942 answers it). Symbolic and unencoded fonts keep the old order,
whose own witnesses are codes that really are characters of the font's subtable.

## 3. What it changes

A mark, on pages whose named TrueType font shows a code no step of the clause reaches and whose
`/ToUnicode` names a character the (3, 1) subtable covers; elsewhere nothing. Pol.pdf page 8 now
draws the space and agrees with its archived copy pixel for pixel.

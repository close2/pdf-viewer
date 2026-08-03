# ADR 0157 — A font program that draws nothing

Status: accepted, 2026-08-03. Session 193. Found by opening the third page on the ambiguous
ranking.

## The page

`issue13316_reduced.pdf` is 200×50 points and its whole content stream is
`BT 10 20 TD /F1 20 Tf (ABCBDBEBF) Tj ET` through an embedded `TrueType` program. `poppler` and
`hayro` draw 开票通知单; `mupdf` and `ghostscript` draw `A C  E F`; **we drew nothing, with `0
commands` and `unsupported: []`.** It sat third on §3a's printed ranking at 11.44 bounds from
the nearest reference, inside a verdict that cannot tell a blank page from a grainy one.

## What was wrong, and it was not the drawing

`show_text` asks the program for a code's outline and, when there is none, decides whether that
is worth recording. The three branches were:

```rust
if let Some(outline) = program.outline(code) { show(outline) }          // no tally at all
else if program.uncovered_character(code).is_some() { tally(missed) }   // §9.10.2's case
else { tally(drawn) }                                                   // "fine"
```

The last arm counts a code that drew **nothing** as a code that drew, and the first counts a
code that drew as nothing at all. So the rule the code applied was *"this font misses one of
§9.10.2's characters and shows no blank glyph"* — in practice, "and shows no space" — while ADR
0152's own words, and this tree's ledger and handover, say the rule is **"the face drew none of
what it was asked for"**. A comment is not a test, and no gate compares one with the code
beneath it.

An embedded program answering every code with no outline therefore fell into the last arm and
was recorded as having drawn its text.

## The rule now

One tally per font resource with three outcomes — an outline, no outline, or no outline *and*
§9.10.2 says why — and one condition: **no code reached an outline, and at least one wanted
one.** The message says which of the two it is, because "the substitute face has no glyph for
this character" and "the program has no outline for this code" are different things to be told.

**A code that reads back as whitespace is not tallied at all**, and that is the whole of what
keeps the report honest. A space is *meant* to be blank. Counting one took the corpus's
incomplete documents from 79 to **109**, twenty-two of the thirty new reports naming a single
code — trap 11 exactly: print what a condition matched before trusting its count. Excluding
whitespace takes it to **80**, and every one of the four documents that report has text a
person cannot see:

| document | codes | what it is |
|---|---|---|
| `issue13316_reduced.pdf` | 5 | the page above |
| `issue12963.pdf` | 39 | a whole run of text |
| `recursiveCompositGlyf.pdf` | 10 | composite glyphs that recurse |
| `issue20232.pdf` | 1 | **the ⌀ this tree has been quietly dropping**, recorded in the handover for thirty sessions as "draws 56 where three references draw ⌀56" |

## What it costs, and it is a report *leaving*

Correcting the condition takes the substituted-face report from two documents to none.
`issue11555.pdf` and `issue2128r.pdf` draw most of their text and miss a character or two, so
under the stated rule they do not qualify — and **both pages agree with the reference
consensus**, which is what says the report they carried was an overstatement rather than a
warning. The oracle's agreeing count rises 847 → **849** because they rejoin the judged set.

What is still not reported is a font that draws *some* of its characters and misses others.
That is the trade ADR 0152 argued (reporting every uncovered code named 13 documents that
mostly draw fine, and each report costs the oracle a judged page) and the gap the handover
already names: "a font is reported as a whole, and that is not fine-grained enough".

## The gates

Corpus 79 → **80** incomplete, which is a new report and not a regression (trap 5). Oracle:
`agrees` 847 → **849**, contradicted **70** unchanged, ambiguous 751 → 748 as three pages leave
the judged set for the honest reason. Text readback **98.2%** unchanged, 36 below the floor
unchanged. `silent_fonts.rs` holds both halves — the page that must report and a page of
ordinary text that must not.

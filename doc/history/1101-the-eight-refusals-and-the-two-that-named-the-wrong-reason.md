# 1101 — The eight refusals, and the two that named the wrong reason

Date: 2026-09-15. Branch: `batch-1098-1103`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADR: [1115](../adr/1115-the-kind-of-a-font-decides-what-a-refusal-may-say.md).

## The census, and the question the tree had already answered
`archive_corpus` re-run: PDF/A-2u refuses eight, all on the two Unicode rows — five
`fonts/to-unicode-present` (`…6-2-11-7-2-t01-fail-a`,`-b`,`-c`,`-d`,`-e`) and three
`fonts/to-unicode-values-are-usable` (`…-t02-fail-a`,`-b`,`-c`). 4f and 4e refuse on embedded files,
appearances and a 3D stream, not on Unicode.
**The remedy is `Mechanical` and was built in session 951** — `Answer::Mechanical(Rewrite::ToUnicode)`
over §9.10.2's second method — so no Q66 and no amendment. The brief's fixture would have proved
nothing either: ISO 19005-2 section 6.2.11.7.2's **first** exemption excuses a `/WinAnsiEncoding`
font outright, so the row never binds it. All eight refusals are the clause's own outcome: its
closing sentence says that where the three methods fail there is no way to determine what a code
represents and what is left is a character of the processor's *choosing*, which `A48` declines.

## What was wrong: two of the eight were told a reason that is not theirs
`t01-fail-d` is an `Identity-H` `CIDFontType2`. The composite check tested a **code's width**, and a
Type 0 font showing small CIDs walks past it — so the document was told one of its codes "selects its
glyph without a name", about a font that has no names to miss. The kind is asked of the font now,
the width check staying behind it as documented defence.
`t01-fail-c` is a Type 3 font naming `/square` and `/triangle`. It was told its font "could not be
loaded" — true of `pdf_font`, which refuses a Type 3 because §9.6.4 makes its glyphs `/CharProcs`
streams, and silent about the file. §9.6.4 requires that font's `/Differences` array to describe its
whole encoding, and that array *is* §9.10.2's second method's glyph selection — which is why section
6.2.11.7.2's second exemption names Type 3 beside Type 1. `Naming` holds both routes and reads the
array through `pdf_archive::type3_encoding`, made public so the validator that judges the exemption
and the converter that derives from it cannot read it two ways.

## Calibration, witnesses, and what moved
Trap 13 on both. Planted back (`if false &&`), the Type 3 tests fail with the old "could not load"
sentence and the composite test with the glyph-name one — the corpus defect reproduced in a fixture.
Through `quorra-transform archive --to 2u`, `t01-fail-c` now refuses because `/square` is in neither
list and `t01-fail-d` because the font is composite; the other six are unchanged. The third test is
the capability the Type 3 arm adds, which no corpus document is shaped for: a Type 3 naming `/A`
with a `<0000>` placeholder derives `<41> <0041>`, converts, and conforms when held to 2u again.
**Every target's counts are unchanged** — 2b 450/160, 2u 1/8, 2a 4/12, 4 177/182, 4f 5/2, 4e 5/3 —
which is the point: nothing converts that did not, and each refusal now names the absence in the
file rather than one in this program. `over` 0 and `unconsidered` 0 at all six targets.

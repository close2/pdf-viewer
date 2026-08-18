# 586 — The press that asked a display list it had just thrown away

The first instrument in this tree that **clicks** ran over the corpus, and 78 of 1017 dragged words
selected nothing at all: `Command::Pointer` sets §12.5.5's appearance state before it decides where
the press landed, and changing that state calls `Open::stale`, so the anchor was taken from an
interpretation that had just been discarded — every drag beginning on text that lies over an
annotation selected nothing, on 44 corpus documents. One line moved; 92.33% → 98.91%.

Date: 2026-08-18.
Argued by: [ADR 0421](../adr/0421-the-press-that-asked-a-display-list-it-had-just-thrown-away.md).

Touched: `crates/viewer-core/tests/selection_census.rs` (new), `crates/viewer-core/src/viewer.rs`,
`crates/viewer-core/tests/headless.rs`, `crates/viewer-core/Cargo.toml`,
`crates/pdf-model/tests/text_extraction.rs`, `crates/pdf-model/tests/text_state.rs`,
`crates/pdf-model/src/content/run.rs`, `doc/conformance/ledger.toml`, `tools/state.sh`,
`doc/todo/{02,05}.md`, `doc/{HANDOVER,verify}.md`.

## The demand half

`doc/todo/05`'s first item owed two things and they were the same gap. The geometry half (ADR 0333)
judges where the text layer *says* the words are, in the page's own points; the journey from there
— device pixels in, selected text out — had one test on one document at the page's own point size,
where the magnification is 1 and the origin is 0. Trap 12a's defect lived for seventy-five sessions
in exactly that stretch because no gate clicks.

`crates/viewer-core/tests/selection_census.rs` is that test's population: every corpus document at a
*fitted* magnification, the drag's endpoints taken from `pdftotext -bbox -cropbox` and nothing about
them from this tree's own geometry, with the two self-inverse properties ADR 0323 asked for beside
it. It shares the geometry half's cached invocation, so it asks poppler nothing new.

The first run's finding is in the title. What made it visible rather than a shrug is the classes: a
drag that missed was either a word set down the page (a horizontal drag is then the wrong gesture,
not a wrong answer), or one of 78 that selected the empty string — and the empty string is not a
near miss. Fixing it left eleven, in four classes the gate names.

The caret property needed a second look before it could be asserted at all. `Query::Caret` is not
injective: where a glyph's advance is zero, several offsets are the same point and the point names
the last of them, which is one corpus document and 26 offsets. So the property is
`caret(offset(caret(o))) == caret(o)` — the round trip lands on the same *place* — and the shared
points are counted apart rather than tolerated.

And the geometry half's verdict gates now, on `doc/todo/05`'s own rule rather than despite it: the
figures held from session 498 to this one, so the gate carries a named list of the documents with a
word out of bounds, both directions, plus a floor under the judged set — a refusal that grew would
otherwise shrink the denominator and leave the verdict looking unmoved.

Six deliberate breakages, each reverted, are in the ADR's table. The one worth repeating here is
that a 1 pt shift of every glyph box is **fatal** to the geometry gate and **invisible** to the drag
census, while a mirrored y flip is the other way about: neither instrument is the other's
approximation, and that was measured rather than claimed.

## The spec half

`spec-errata emit` over clause 9, run before writing rather than `check` afterwards, found that
Errata Collection 3 **adds a requirement to §9.4.2** — within a text object, `q` and `Q` "shall
additionally push and pop Tm and Tlm as part of the graphics state stack" (issue #368,
`/State` `Review` `Completed`), with §7.8.2 pointed at §9.4.1 for it in the same collection. §9.4.1's
ledger row said the opposite in as many words. The stack entry is now the graphics state and the two
matrices together.

The corpus cannot show it, and that was measured too: 13 of the 974 documents put a `q` or a `Q`
inside a text object and not one moves `Tm` between the two, so no page, no oracle verdict and no
word box moves either way. It is pinned by a pair of streams differing only in the `q`, which is
`cross_references.rs`'s construction for the same reason.

## What is still owed

The drag fraction's own ratchet, once it has held across rounds — the rule this round kept for the
geometry verdict is the rule that keeps it printed for now. The eleven remaining misses are a
reading list rather than a number: one is an end glyph whose advance we and poppler disagree about,
three are reversed text, and three are a form-heavy page where the word poppler reads out of a
widget's appearance is not the word our layer has at that point.

# 513 — The witness needs glyphs before it needs a shaper, and the binary has none

**Finding.** The complex-script question behind `freetext_no_appearance.pdf` — the one corpus
document §12.7.4.3's construction refuses whole — taken to its measurements, and the premise it
arrived with reversed by the first of them: the compiled-in Liberation Sans carries **no Arabic
at all** — its `(3,1)` format-4 `cmap` maps every code point tried in U+0600–U+06FF and both
presentation-form blocks to glyph 0, its `GSUB` script list is `DFLT`/`cyrl`/`grek`/`latn`, and
`fc-scan` agrees from the other side — so no shaping subset, however derivable from Unicode's
own data, can draw one mark of that value from this binary. The witness was decoded and read:
36 distinct Arabic characters, one combining mark once, nine mandatory lam-alef ligatures, and
every line one RTL run — so it needs a glyph source, joining-form selection and right-to-left
ordering **together or not at all**, because glyphs without the other two draw isolated forms
left-to-right, a plausible wrong page that reports nothing. `pdftoppm` was looked at and draws
exactly the construction ADR 0112 rejected: the value's full stops scattered on an empty page,
one "found character that the font can't represent" per character on stderr. The refusal
stands, now priced (an OFL Arabic face as a fifteenth compiled-in program; ArabicShaping and
Bidi_Class statics under the Unicode licence `doc/third-party-data.md` would have to accept;
either presentation-form addressing or `GSUB` type-1/type-4 execution over the `read-fonts`
already in the tree — not `rustybuzz`, which would bring `ttf-parser`, a second sfnt stack).
Two measurements make the refusal machine-independent and are why it can be pinned in a gate:
the value has 36 distinct missing characters against the invented `/Differences`'s 31 free
codes, and `read-fonts`' Adobe Glyph List holds no name for any Arabic character (zero `afii`
entries), so `named_glyphs_reach_more` reaches no installed face on any machine.

**Date.** 2026-08-14.
**ADR.** [0348](../adr/0348-the-witness-needs-glyphs-before-it-needs-a-shaper.md).

**Code.** One test: `crates/pdf-model/tests/variable_text.rs::`
`the_arabic_free_text_declines_whole_and_names_both_halves` — opens the witness itself, asserts
no ink and one report naming `/Helv` and the wholeness, and is the guard against the
wrong-but-plausible page: a later change that draws this value partially or in logical order
fails a gate instead of shipping.

**Touched.** `doc/adr/0348-*` (new), `doc/todo/21-font-substitution.md` (§1 is not this
witness's owner — the fallback alone would draw it wrong), `doc/todo/22-variable-text-edges.md`
(the one remaining item's entry now carries the reading and the pin),
`doc/stack.md` (the `rustybuzz` entry sharpened: if shaping returns for generated text it sits
on `read-fonts`, and what blocks it today is glyphs, not machinery),
`doc/conformance/ledger.toml` (§12.7.4.3's note and test list), `doc/todo/README.md` (row 22's
claim of a composite edge had been stale since the five-hundred-and-second closed it), this
file.

**Witness pictures.** Ours before and after: the same blank page with the same report — the
round's outcome is that the blank is now argued, priced and pinned rather than only standing.
`pdftoppm`'s render of the same page: six-and-some full stops on white, looked at side by side.

**Gates, watched print.** `cargo fmt --all --check` clean. `cargo clippy --workspace
--all-targets` silent of lints (the `viewer-qt@0.1.0:` `-Wmaybe-uninitialized` lines are the
documented cold-build gcc output from the cxx bridge). `cargo nextest run --workspace`:
1861 passed, 15 skipped. `cargo test --workspace --doc`: every crate ok, 1 doctest passed.
Corpus gate: 974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
62 incomplete, 0 slow; silence lines 5 over 2, 57 over 9, 1228 over 43 — unchanged. Oracle:
905 agree / 68 contradicted / 786 ambiguous, 99.8% cache hit against the shared
`pdfref-cache` — unchanged, as a round whose only code change is a test should expect and this
one verified rather than assumed. `cargo test -p conformance -- --nocapture`: 5 checks ok.
The text, dates, XMP, JPEG 2000 and quorra gates were not owed: no edit reaches a raster or a
readback.

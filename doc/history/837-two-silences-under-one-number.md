# 837 — Two silences under one number, and a census that could not see its own witness

Date: 2026-09-01. On `main` directly, from `f928da88`.

ADR: 0764 — two silences under one number, and the field that separated them.

Touched: `crates/pdf-font/src/vertical.rs` (`Form`, and `Downward::read` no longer refusing a face
with no `vert`), `crates/pdf-font/src/loading.rs` (`substituted_glyph`, `unsupplied_vertical_form`),
`crates/pdf-model/src/content.rs`, `content/report.rs` (the fourth count and
`Shortfall::without_a_vertical_form`), `content/text.rs` (the counting site and its trace),
`content/transparency.rs` (the rewind mark), `crates/pdf-model/tests/corpus.rs` (a fourth silence
line, and the four moved into `the_silences`), `crates/pdf-model/tests/vertical_forms.rs` (the
trichotomy), `crates/pdf-model/examples/vertical_form_census.rs` (new),
`crates/viewer-core/src/query.rs`, `crates/viewer-confined/src/protocol.rs`,
`crates/viewer-ffi/src/kinds.rs`, `crates/viewer-ffi/include/pdf_viewer.h`,
`crates/viewer-ffi/tests/header_and_library_agree.rs`, `tools/pdf-retrieve/src/main.rs`,
`doc/conformance/ledger.toml` (§9.7.5.1, §9.7.4.2), `doc/todo/21-font-substitution.md`,
`doc/verify.md`, `doc/adr/0764-*`, this file.

## The primary item

ADR 0763 declined to *report* a substituted face with no vertical form and wrote that it "stays
counted-by-nothing … exactly as ADR 0270 left its neighbours". The refusal is right; the last
clause is not. ADR 0270 left its neighbours **counted**, and this one was counted by nothing — so a
face with no glyph for a character and a face with no glyph for that character's *vertical form*
were one silence with one number, and the number was about the other one. Every instrument in the
tree read zero on a page drawn in the wrong shape.

So the field: `Interpretation::codes_without_a_vertical_form`, `Shortfall::without_a_vertical_form`,
and the four consumers ADR 0422 built — `Query::Readback`, the confined pipe (nine numbers now, each
distinct in the round-trip test), `pdfv_readback_count`'s tenth kind, and `pdf-retrieve`'s
`readback` object, where it is `upright_vertical_forms` beside `missing_glyphs` and `blank_glyphs`.
The corpus gate prints it as a fourth silence line.

`Shortfall::is_whole()` deliberately does **not** move: that code was named and was drawn, and the
one consumer of `is_whole` that speaks is `viewer-accessibility`'s status group — a reader who
cannot see the page loses nothing at all to a bracket drawn upright, and telling them characters
are missing would be false.

## Calibration, which is the part that matters (trap 13)

On this machine the honest count is **zero**: the face this catalogue offers for Adobe-Japan1
states the forms. A zero from an uncalibrated instrument is worth nothing, so the defect was
planted — `VerticalForms::read` made to return an empty map, which is what every Latin face is —
and watched three ways. As committed the trichotomy test passes on its *form supplied* arm and the
census reports nothing; with the defect planted the test passes on its *counted* arm and the
curated census reports 15 codes on `VerticalText.pdf`; with the defect planted **and**
`unsupplied_vertical_form` made to answer `None`, the test **fails** on CID 7911. Without the third
run the first two prove nothing.

The test itself needs no skip, which is what makes it different from ADR 0763's: for each vertical
CID the witness shows, exactly one of three holds on any machine — the face cannot draw the
character (which is `uncovered_character`, the other silence), or it draws the form, or it draws it
upright and is counted for it.

## The census, and the defect it had

`examples/vertical_form_census` prints two populations side by side, because a census derived from
the clause is not a census of the defect: the *files'* — a `Type0` in writing mode 1, embedding no
program, in a collection Table 116 publishes a vertical `CMap` for — and *this machine's*, how many
codes those documents then draw upright.

**Its first version could not see `issue11555.pdf`, which is the corpus's own witness.** The walk
was `xref().object_numbers()`, copied from `hollow_glyph_census`, and that document writes its whole
`Type0` — `/Encoding /90ms-RKSJ-V`, no embedded program — *inline inside the page's `/Resources`*,
so it is not an object the table names. The run said the pdf.js corpus states no substituted
vertical font at all, which is trap 25 exactly: a population that misses what is there prints the
same thing as a clean corpus. `collect_type0` recurses now, and `--pdfjs` goes from finding nothing
to naming it. **`hollow_glyph_census` still has the hole** and §9.7.4.2's row says so rather than
being re-run this round.

## What the census said

Three runs, the last two after the fix, with the crawl run before the gate sequence rather than
beside it:

- **curated (1251)**: two documents in the clause's population — `VerticalText.pdf` and
  `issue11555.pdf` — and nothing lost on either.
- **pdf.js (974)**: 488 `Type0` dictionaries, 4 in writing mode 1, one of them substituted; nothing
  lost. The corpus gate's fourth silence line agrees, at 0 over 0.
- **crawl (65 944)**: 194 272 `Type0` dictionaries in the 65 703 that open, 1312 in writing mode 1,
  98 substituted in a collection with a published pair, over **42 documents** — of which **one**,
  `7311602.pdf`, loses **33 codes**. `PDFVIEWER_TRACE_VERTICAL_FORM=1` names them: Adobe-Japan1's
  CIDs 7923 (っ) and 7939 (ヶ), both small kana. So the shortfall is **per glyph rather than per
  face** — the same machine supplies the witness's brackets from the same feature — and a design
  gated on "does this face state `vert`" would have counted nothing.

That is what ADR 0152's arithmetic could not have produced: it was taken over 974 documents, and the
population that matters here has one member there and forty-two on the crawl.

## The second track

Two ledger rows, both `implemented`, both moved to what is true:

- **§9.7.5.1** gains the count and its instrument, and its "13 corpus fonts" sentence is
  denominated — it means `doc/pdf.js`, this tree now holds five populations, and what the
  population is *today* is a command rather than a sentence.
- **§9.7.4.2** gains the honest limit on `hollow_glyph_census`'s figures: its population is the
  objects the cross-reference table names, so a font written directly inside its parent is outside
  it, and its numbers are a floor.

The `undenominated` sweep is what found the first of those; `quotations`, `pointers` and `parts`
ran after the sequence and name nothing this round added.

## Gates

The full §2 sequence on a quiet machine, after the censuses rather than during them: both
formatting lines, `clippy --workspace --all-targets` and the `fuzz/` manifest under
`RUSTFLAGS="-D warnings"`, the workspace tests, the doctests, the sandbox worker, the corpus gate,
`pdfref-hayro`, the oracle, the three extraction gates, both censuses, `dates`, `xmp`, `jpeg2000`,
`render-quorra`'s corpus gate, `fixed_documents` and `cargo test -p conformance`. All green, and
§5's binaries were rebuilt and installed.

Two lint findings on the way, both this round's own: a verbatim NOTE written inline rather than as
a blockquote (`doc_markdown` is what makes the tree's own quoting convention enforceable), and the
corpus gate's headline function passing 100 lines, which is why the four silence lines are now
`the_silences`.

## What the next round might take

`doc/todo/21` §7's remaining three are a half-width vertical pair with no witness, a `GSUB` lookup
type nothing on this machine states, and two collections Adobe publishes no vertical `CMap` for at
all — the last closed rather than owed. The sharper thing this round leaves is elsewhere: **the
same population defect it found in its own census is still in `hollow_glyph_census`**, whose
figures §9.7.4.2's row quotes, and it is one recursion and one crawl run to settle. And
`7311602.pdf` is now a named witness for a shortfall nothing draws attention to — whether a face
that states `vert` for brackets and not for small kana is worth a second lookup (`vkna`, `vrtr`) is
a measurement before it is a decision.

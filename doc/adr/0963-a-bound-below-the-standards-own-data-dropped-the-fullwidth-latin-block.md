# 0963 — A bound below the standard's own data dropped the fullwidth Latin block, in silence

Status: accepted. Session 960.
Context: `crates/pdf-font/src/cmap.rs`, `crates/pdf-font/src/loading.rs`,
`crates/pdf-model/src/content/font.rs`, ISO 32000-2 §9.7.5.2, §9.7.6.2 and §9.7.6.3,
`doc/traps/instruments-and-reports.md` traps 5, 11 and 13, ADR 0961.

## How it was found

ADR 0961 fixed one bomb bound — `mesh::MAX_TRIANGLES`, which stopped a mesh part-way and
reported the page complete — and left the obvious follow-up: **every other bound on the
render and interpret path, asked the same question.** Does hitting it produce a report, or a
silently short answer?

Most of them survive the question. `content.rs`'s four and the four apiece in `pattern.rs` and
`run.rs` raise `Unsupported::LimitReached` already; `pdf_render::MAX_GROUP_DEPTH` and
`MAX_EXTENT` are `BackendError`s; `function.rs` refuses a chain with a sentence;
`optional_content.rs` has a `Visibility::TooDeep` whose own doc comment says why a bound
reached in silence would be wrong; `image.rs`'s bounds are `ImageError`s. The sweep found
`pdf-font`'s `CMap` bounds, which are four `if len < MAX { push }` with no `else` at all — and
then found something worse than silence.

## The finding: the bound sat below what the standard's own data states

§9.7.5.2 Table 116 names the predefined `CMap`s a reader resolves by name. Adobe publishes the
files; this binary has carried all 239 of them since the hundred-and-fifty-sixth session
(`data/cmaps/`, ADR 0140), which is what makes §9.7.5.2's row `implemented`.

`MAX_RANGES` was `1 << 14`, which is 16 384 ranges per code length. Counted over every carried
file, by code length:

```text
UniCNS-UCS2-H  RANGES len=2  16418
```

and nothing else anywhere near it. One registered `CMap`, 34 entries over — so the parse
discarded its last 34 `cidrange` lines, which are contiguous at the end of Adobe's file:

```text
<ff02> … <ff0f>   fullwidth quote, hash, dollar, percent, ampersand, apostrophe,
                  parentheses, asterisk, plus, comma, hyphen, full stop, solidus
<ff10> <ff19> 333 the fullwidth digits
<ff1a> … <ff20>   colon, semicolon, less-than, equals, greater-than, question, at
<ff21> <ff3a> 365 the fullwidth capitals
<ff3b> … <ff3f>   brackets, reverse solidus, circumflex, low line
<ff41> <ff5a> 391 the fullwidth lower case
<ff5b> … <ffe4>   braces, vertical line, halfwidth ideographic comma,
                  fullwidth not sign, fullwidth broken bar
```

That is the fullwidth ASCII block of Adobe-CNS1: what Traditional Chinese typesetting sets
Latin letters, digits and most punctuation with. Measured on the shipping binary before the
fix, `predefined::cmap("UniCNS-UCS2-H")` answered `None` for `<ff21>`, `<ff10>`, `<ff41>` and
`<ff08>` and `Some(108)` for `<ff01>` — the entry that is number 16 384, one inside the bound,
and the control that says the file was being read at all.

§9.7.6.2 makes that lookup a `shall`:

> The code extracted from the string shall be looked up in the character code mappings for
> codes of that length.

and §9.7.6.3 says what a failed lookup costs:

> If the CMap does not contain either a character mapping or a notdef mapping for the code,
> descendant 0 shall be selected and the glyph for CID 0 shall be substituted.

So a document naming `UniCNS-UCS2-H` — or `UniCNS-UCS2-V`, which `usecmap`s it — drew `.notdef`
for every fullwidth letter, digit and bracket on the page, and `Interpretation::is_complete`
returned true.

**No corpus finds this.** No document in `doc/pdf.js`, `doc/corpora` or `corpus-cache` names
that `CMap`; the corpus gate's figures do not move, and after the fix not one document on this
disk reports any of the four bounds. It was found by counting the standard's own data against a
constant in a source file, which is trap 8 stated the other way round: a corpus finds what
documents contain, not what the specification says.

## Two things were wrong and they are separate

1. **The bound was below the floor the standard sets.** A resource bound is this program's to
   choose — Annex C.1, "this PDF standard does not restrict the size or quantity of things
   described in the PDF file format" — but a bound that cuts a file §9.7.5.2 requires a reader
   to resolve is not a resource decision, it is a wrong answer. `MAX_RANGES` is now `1 << 15`:
   the measured figure with a little under a factor of two to spare.
2. **It cut in silence.** All four bounds did, and raising one changes nothing about the other
   three or about a hostile embedded `CMap` that reaches the raised one.

## Decision

- `CMap` carries `truncated: Option<&'static str>`, set by `cut_by` **with the discarded entry
  already in hand** rather than on reaching a count — so a file whose last entry lands exactly
  on a bound lost nothing and says nothing. That is trap 11, and the same property ADR 0961
  gave the mesh bound.
- Four bounds set it, each with the name a report carries: `max_cmap_ranges`,
  `max_cmap_singles`, `max_cmap_codespace`, `max_cmap_operands`. `absorb` carries a
  `usecmap`ped map's truncation into the map that builds on it, because the two are consulted
  as one.
- `LoadedFont::cmap_truncated` exposes it for both `CodeMapping::Composite` and
  `CodeMapping::Substituted`, and `pdf_model::content::font` raises
  `Unsupported::LimitReached { limit }` — at the load, and again where the cross-page font
  cache serves a font, because a report is about *this page*. That second call is
  `note_char_procs_damage`'s own precedent, one line above it.
- `MAX_RANGES` goes to `1 << 15`, with the census in its doc comment and the cost priced: a
  range is three `u32`s, so a mapping at the bound holds 384 KiB and a `CMap` whose eight
  mappings all do holds 3 MiB, twice what the old constant admitted. `Mapping::get` scans
  linearly, so a lookup on such a mapping costs twice what it did; that is a property of the
  representation rather than of the bound, and a file able to pay it at `1 << 14` already was.
- `viewer-core`'s sentence for `LimitReached` said the page "was not drawn to the end", which
  was true of `MAX_OPERATIONS` and false of `max_clips` before this round and of
  `max_cmap_ranges` after it. It now says part of what the document asked for is not on the
  page, which is what every one of them means.

## Calibration

`no_registered_cmap_is_cut_by_these_bounds` walks all 239 carried files and asserts
`truncated() == None`, then checks the three fullwidth runs against the CIDs
`data/cmaps/UniCNS-UCS2-H` gives them — so a bound raised only far enough to stop the flag
firing, while the parse still lost the mappings, fails the second half. Calibrated per trap 13
by restoring `MAX_RANGES` to `1 << 14`: it fails naming `/UniCNS-UCS2-H`, and passes when the
constant is put back.

`a_cmap_past_the_range_bound_is_reported_by_name` builds a Type 0 font over an embedded `CMap`
of 32 769 one-code `cidrange` entries and asserts the report;
`a_cmap_exactly_on_the_range_bound_reports_nothing_about_it` is the same fixture at 32 768 and
asserts its absence. Both assert the page draws, so neither can pass by the font failing to
load — which the first draft did, with `/Ordering (Identity)` and a refusal wearing the bound's
name. Calibrated by planting `cut_by` as a no-op: the first fails, the second still passes.

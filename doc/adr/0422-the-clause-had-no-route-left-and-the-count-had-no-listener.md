# ADR 0422 — The clause had no route left, and the count had no listener

Status: accepted, 2026-08-18. Session 587. Amends §9.10's, §9.10.1's, §9.10.2's and §9.10.3's
ledger rows. Settles the open question `doc/HANDOVER.md` carried and `doc/todo/21` §5 held; leaves
ADR 0152's trade, ADR 0270's split and ADR 0318's census exactly where they are.

## The question, and it was asked in two terms when there were three

`doc/HANDOVER.md` had carried this for four rounds:

> **None of the three is a report**, deliberately, on ADR 0152's arithmetic: a report takes a page
> off the oracle's judged set, and these are shortfalls in the readback of pages that mostly draw
> perfectly. The volume is measured; what to do with it is not settled.

Three counts, one decision, and the decision had been posed as *report or leave it*. Both are bad:
reporting costs the oracle a judged page apiece for a shortfall that is not a defect, and leaving
it costs trap 5's whole subject — a shortfall nobody can see. The third option is what this round
took, and the reason it took two steps to get there is that the first step had to establish there
was no *fourth*: a method of §9.10.2 this tree does not walk would have made the whole question
moot, because then the band would be a defect and the answer would be to fix it.

## Step one: the clause, method by method, against the code

The census first, so that what is being argued about is a population and not a memory:

```sh
cargo run --profile gates -p pdf-model --example unnamed_code_census -- doc/pdf.js/test/pdfs/*.pdf
```

**1226 codes over 41 documents**, of the 892 corpus page ones that report nothing — the corpus
gate's third silence line prints the same pair, which is what says the census's population is the
gate's own and not a second measurement. By cause, largest first: 761 an `Identity` ordering with
no `/ToUnicode`, 210 a `/ToUnicode` that omits the code, 131 a glyph name neither published list
holds, 123 a glyph selected by code that nothing names, 1 a registered collection with nothing for
the CID, 0 a mapping answering with no characters.

Trap 11 says to read what a count matched before trusting it, and `PDFVIEWER_TRACE_UNNAMED_CODE=1`
was pointed at the head of each band. The largest contributor of the third band is
`font_ascent_descent.pdf`'s 70 codes, and every one of them is a dvips `/GNN` — `G1` through `G30`
— which is ADR 0318's `G14` a whole document at a time. `standard_fonts.pdf`'s 32 are codes 0 to 31
shown through one simple font, `issue20489.pdf`'s 61 are code 10 shown through three non-embedded
`TrueType`s stating `/Encoding /WinAnsiEncoding`, and `issue18059.pdf`'s 26 are code 0 in
`/Helvetica`. Annex D leaves every one of those codes unencoded in all four Latin encodings, and
Table D.2's note 3 — "[i]n WinAnsiEncoding, all unused codes greater than 40 map to the bullet
character" — is **octal 40**, so it reaches none of them; `encoding.rs` implements that note and a
test pins `BaseEncoding::WinAnsi.glyph_name(127) == "bullet"`.

`spec-errata emit` was run over clause 9 before any of this was written, because session 586 found
an erratum adding a requirement to §9.4.2 that the ledger denied. Errata Collection 3 annotates
§9.3.4, §9.3.8, §9.4.2, §9.4.4, §9.5, §9.6.2.1, §9.6.2.3, §9.6.4, §9.7.3, §9.7.4.2, §9.7.5.3,
§9.7.6.2, §9.8.1, §9.8.2, §9.8.3.3 and §9.10.3 — **and lands nothing at all on §9.10.2**. Its
methods, their order and its closing sentence are as published.

Then the three methods, each against `LoadedFont::text`:

**Method 1, `/ToUnicode`.** Walked first, and its failure falls through **per code** rather than
per font — which is the distinction that matters, because §9.10.3 requires no completeness and 210
of the 1226 are a producer's table with a hole in it. A reader that gave up on the font when its
table missed one code would lose the rest of the page.

**Method 2, a simple font's glyph name through the Adobe Glyph List and the Adobe Glyph List for
New Fonts.** Walked, with Annex D.6's `CHAR` column behind it for `ZapfDingbats` (ADR 0318) and the
Adobe Glyph List for New Fonts' composition and variant rules in `encoding::text_for`.

**Method 3, a composite font's character collection.** Walked — and this is where the trap-5 shape
would have been, because the clause's condition has two disjuncts and the exception governs only
the first:

> If the font is a composite font that uses one of the predefined CMaps listed in "Table 116
> -Predefined CJK CMap names" (except Identity -H and Identity -V ) or whose descendant CIDFont
> uses the Adobe-GB1, Adobe-CNS1, Adobe-Japan1, Adobe-Korea1 (deprecated in PDF 2.0 (2020)) or
> Adobe-KR (added in PDF 2.0 (2020)) character collection

So an `Identity-H` font *whose descendant states one of the five collections* is inside the method,
and reading the exception as a property of the font rather than of the encoding name would have
excluded it. `composite::collection_meaning` reads the descendant's own `/CIDSystemInfo` and never
consults the encoding's name, and all five `registry-ordering-UCS2` files the clause names are in
`data/cmaps/`. `NamingGap::UnaddressableCid` is therefore what its doc comment says it is: a
composite font for which *no* collection resolved.

**And the order between methods 2 and 3 cannot matter, which is an argument rather than a hope.**
`text` tries 3 before 2, which is not the clause's order — and the two are conditioned on "a simple
font" and "a composite font", which no font satisfies together. In this tree that is structural:
`load_simple` sets `collection: None`, `load_composite` sets `glyph_names: None` and
`symbolic_set: None`. A font can reach at most one of the two branches, so the order between them
is unobservable. (The order that *is* observable — the closing permission after all three — is the
clause's, and `text_from_program` is the last thing tried.)

**Conclusion of step one: no method is skipped, none is tried out of order in any way a document
can see, and each falls through to the next per code.** The 1226 are the clause's own sentence:
"there is no way to determine what the character code represents".

## Step two: the decision, which is about the voice

So the band is not a defect, and the two terms the question had been posed in are both wrong.

**A report is wrong** because nothing failed. ADR 0152's arithmetic prices it and this round
re-measured the price: 41 of 892 silent page ones would leave the oracle's judged set to say
something ISO 32000-2 has already said, about pages that draw correctly. Worse than the cost is the
claim — an `Unsupported` entry is this program stating what it could not do, and putting the
standard's answer in that list would be a false statement about our own coverage.

**Silence is wrong** because the shortfall is invisible in every channel a consumer has.
`Interpretation::is_complete()` is `true`, `unsupported` is empty, and the text is short. The
sharpest form of it is `french_diacritics.pdf`: 29 glyphs drawn, a readback of `1`, and nothing
anywhere saying the other 28 characters exist. `pdf-retrieve` handed a caller that string with
`"complete": true` beside it.

**The third option is that the count crosses.** It was already computed and already correct; what
it had was no listener. Four things were built:

- **`pdf_model::content::Shortfall`** — the three per-code counts as one value, with
  `Interpretation::shortfall()`. One type rather than three fields because a *consumer* cannot use
  one without the others: ADR 0270's whole point is that a code reaching a blank glyph is a space
  and not a loss, so a host told only `codes_without_a_glyph` would be reading a fraction of a
  fraction. It is the same reason `Answer::Field` and `Answer::Fields` carry one type (ADR 0247).
- **`viewer_core::Query::Readback`**, beside `Query::Reports`. It passes `doc/ui-boundary.md`'s
  test — a question no host can answer for itself, because the counts come from the font programs'
  own tables during interpretation and a host holds neither the fonts nor the codes. Every `Query`
  crosses `viewer-confined`'s pipe, so it does; the eight numbers are written and read field by
  field from a destructured struct, so a field added later fails to compile rather than crossing as
  a zero, and the round trip uses eight *different* numbers because an encoder with two fields
  swapped would pass with any two of them equal.
- **`viewer-accessibility`'s status group** carries a sentence for it, which is the one consumer
  that is not a convenience. §14.7's tree already carries what the page could not *draw*, on the
  argument that the person who cannot see the page is the one for whom a count in the title bar is
  no answer; a page whose codes cannot be named is spoken short and every other channel says the
  page is fine. **The two are worded apart on purpose**: "not drawn as the document specifies" is a
  refusal of ours, and a code §9.10.2 ends at gets "cannot be read: nothing in the document says
  which characters their codes stand for". Calling the second a drawing fault would tell a person
  the picture is wrong when it is not, and a test asserts that the sentence does not say so.
- **`pdf-retrieve`'s `readback` object**, per page and summed per section, beside `unsupported` and
  never inside it. The default `text` is still `Interpretation::text` byte for byte, which that
  tool's own test asserts (ADR 0257) — a shortfall reported *beside* the string is the only shape
  that does not put this tool between a caller and the gate that measures it.

`french_diacritics.pdf` through `pdf-retrieve page` now answers `"complete": true`,
`"unsupported": []`, 28 unnamed codes all under `unlisted_name`, and the same 27 spaces and a `1`
it always did. Nothing about the answer changed; what changed is that the answer says what it is.

## What was deliberately not done

- **No `Unsupported` variant, and no widening of `is_complete()`.** Both are above, and the oracle's
  judged set is the number that decides it.
- **No C ABI entry point.** `viewer-ffi`'s header states two structs passed by value and calls them
  "the boundary's most expensive kind of change"; a third carrying eight counts would be that
  change for a number no C consumer has asked for. The Rust boundary carries it, `pdf-retrieve`
  carries it, and adding the entry points later costs a compiled caller nothing (a `Query` is not a
  counted kind). Recorded here so that it is a decision rather than an oversight.
- **No change to any count.** The corpus gate's three silence lines, the oracle's verdicts and both
  text gates are what they were: this round added a reader for a number, not a number.
- **No new census.** ADR 0318's is the instrument and it needed nothing; what it needed was to be
  run and read, which is step one.

## The gates

Everything `doc/todo/02` §2 runs, on the tree as committed. `cargo fmt --check` clean,
`clippy --workspace --all-targets` silent, **2162 tests** run and 16 skipped under `nextest`, the
doctests clean. Corpus: 974 documents, 66 incomplete, and the three silence lines **5 over 2**,
**57 over 9** and **1226 over 41** — the last of which is the census's own total, which is the
check that the two instruments count one population. Oracle: 1794 pages, 1690 complete, **907
agrees, 786 ambiguous, 66 contradicted**, 18 no-render — unchanged, and it has to be, because
nothing this round touched runs during interpretation. Text extraction: **99.2%** of `pdftotext`'s
words over the corpus with 22 below the floor, **99.8%** against PDFBox's frozen extraction over
its 40 documents with 4 below, and the word-box gate 98.26% with 486 of 508 documents fully in
bounds. `selection_census`, dates, XMP, JPEG 2000, quorra (932 of 957 agree) and `conformance` all
as they were.

**One latent flake was fixed on the way past**, and it is worth a sentence because it looked like a
fixture builder that emitted nothing: `pdf-retrieve`'s test fixture named its temporary file after
the process alone, two tests in one binary asked for one on two threads, and one of them read a
file the other had just truncated — `NoHeader { searched: 0 }`. It carries a counter now. Adding a
test is what surfaced it, which is the ordinary way a shared-path race is found.

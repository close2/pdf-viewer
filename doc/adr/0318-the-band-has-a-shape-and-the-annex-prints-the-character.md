# ADR 0318 — The band has a shape, and the annex prints the character

Status: accepted, 2026-08-13. Session 483. Amends §9.10.2's and Annex D.6's ledger rows. Extends
ADR 0311's count into a census by cause, and leaves ADR 0152's trade and ADR 0270's split exactly
where they are: nothing here becomes a report.

## The question

ADR 0311 gave the readback's refusal a voice and left it as one number — 1342 codes over 45 corpus
documents that §9.10.2 could not name — with `doc/todo/21` §5 asking for its **shape**, and a
hypothesis about what would be in it. This round was to derive the shape from what the fonts
actually are, and then close whatever part of it the standard reaches.

## The instrument

`pdf_font::NamingGap` answers, for a code no method named, **which of the clause's methods was the
highest-priority one that font could have answered with**. The clause ranks its own methods — "[a]
PDF processor can use these methods, in the priority given" — so a font carrying a `/ToUnicode`
that omits the code has failed at the first method whatever its glyph names say. Reporting the
*last* thing tried would describe every gap identically, because every route ends at the same
declined permission.

It is derived by running `LoadedFont::text` and classifying the failure, so the census cannot
drift from the extraction it measures. `Interpretation::codes_without_a_character` is now
`UnnamedCodes` — one counter per variant, with `total()` equal to the number ADR 0311 added — and
`examples/unnamed_code_census` is the command:

```sh
cargo run --profile gates -p pdf-model --example unnamed_code_census -- doc/pdf.js/test/pdfs/*.pdf
```

`PDFVIEWER_TRACE_UNNAMED_CODE=1` names each code and font on stderr, in the idiom
`PDFVIEWER_TRACE_MISSING_GLYPH` already had; it is what makes the `UnlistedName` population
legible, because there the *name* decides whose gap it is.

## The shape, and it is not the hypothesis

The census's own output, before this round's change. `doc/todo/21` §5 guessed three populations;
the two largest are the first two of them and the third is a *quarter* of what the guess implied,
because most of the `aNNN` names in the corpus are not dingbats at all:

| codes | cause | what it is |
|---|---|---|
| 761 | an `Identity` ordering and no `/ToUnicode` | §9.10.2 excludes the third method **by name**; §9.7.4.2 says why there is nothing else to ask. `complex_ttf_font.pdf` is 616 of it |
| 247 | a glyph name neither published list holds | pdfTeX's and dvips' `/aNNN`, `/GNN`, TeX's `circlecopyrt`, and **`ZapfDingbats`** |
| 210 | a `/ToUnicode` that omits the code | a producer's own table, incomplete. `issue5874.pdf` 130, `bug911034.pdf` 72 |
| 123 | a glyph selected by code that nothing names | codes 0–31 and code 10 shown in a simple font, and code 0 reaching `.notdef` |
| 1 | a registered collection with nothing for the CID | one document |
| 0 | a mapping answering with no characters | the branch exists because `Readback::Nothing` includes it; no corpus page does it |

Read by opening the files rather than by reasoning about the counts, which is what the todo asked
for and what made the difference: three of the four large populations turn out to be the standard's
own "there is no way", and the fourth contains one sub-population the standard *does* answer.

## The clause reading: `ZapfDingbats`, and Annex D.6 prints the character

**`a192` is not the route and the standard says so.** ADR 0311 established that following the one
published list that holds the name — Adobe's `zapfdingbats.txt`, which §9.10.2 sends a reader to
nowhere — would read pdfTeX's `À` as an ornament. That stands. Annex D.1 puts it more strongly
than this project had:

> The characters for ZapfDingbats are ordered by code instead of by name, since the names in that
> font are meaningless.

**But the annex answers the question the name cannot.** Three sentences, and the third is the one
nobody in this tree had looked at:

- D.1: "This annex lists the character sets and encodings that shall be predefined in any PDF
  processor."
- §9.6.5.1: "The standard 14 fonts include two symbolic fonts, Symbol and ZapfDingbats, whose
  encodings and character sets are documented in Annex D".
- Table D.6 itself has three columns, and the first is **`CHAR`** — the character, printed beside
  the name and the octal code, for all 188 encoded codes.

So a processor that predefines this set predefines what each of its codes represents, which is
exactly what §9.10.2 asks. The route is Annex D's, not the Adobe Glyph List's, and the standard is
its own source: `ZAPF_DINGBATS_CHARACTERS` is transcribed from Table D.6 beside the name table that
has been in `encoding.rs` for hundreds of sessions, and a test asserts the two describe the same
188 codes — a slip that left them out of step would hand a code its neighbour's character, which is
the one error a reader could not see.

**The `CHAR` column is the standard's text and not a picture of a glyph**, which is worth one
sentence because it decides the whole argument. `pdffonts` over the two pages of
`ISO_32000-2_sponsored_EC3.pdf` that carry Table D.6 lists Cambria, Calibri, Arial and MS-Gothic
and **no `ZapfDingbats`**: ISO typeset that column in Unicode text fonts, so the characters in it
are code points the standard chose, not shapes a dingbat font drew. Trap 9's habit — read the
file's own tables rather than take a vote — settled in two minutes a question that could have been
argued for a page.

**And it is keyed by code within this font's own encoding, never by name globally.** That is what
makes it safe against the coincidence the eighth session paid for on the drawing side, when a
Type 3 font was substituted and the substitute drew dingbats: `SymbolicEncoding::character_for`
answers only for a font this crate resolved *as* §9.6.2.2's `ZapfDingbats`, and it finds the
character by locating the name in that font's own encoding. `french_diacritics.pdf`'s Type 3
`/a192` is not that font, gets nothing, and stays counted — which the corpus confirms rather than
the reasoning: its 28 codes are still in the census after the change.

`Symbol` needs none of this and the asymmetry is the standard's: Table D.5 names its glyphs
`alpha`, `universal`, `weierstrass`, which the Adobe Glyph List holds, so §9.10.2's second method
already answers. Asserted rather than described — the test walks all 256 codes of both sets and
finds 0 unlisted names in one and 187 in the other.

## What that closed, including a defect no count could see

**114 codes, and the census total goes 1342 → 1228 over 45 → 43 documents**: `ZapfDingbats.pdf`'s
50 and `issue15716.pdf`'s 64, the latter a `/BaseFont /ZapfDingbats` whose `/Differences` names
`a109`–`a112`.

**The second half was invisible to every instrument in the tree, and it is the more serious of the
two.** A dingbat at code 0x21 was not unnamed — it read back as `!`. §9.10.2's closing permission
is taken in `text_from_the_code` for printable ASCII on a stated argument: 0x21 to 0x7E "is the
range in which a byte and a Unicode code point mean the same character under every encoding §9.6.5
states". That argument is **false for a symbolic font whose built-in encoding Annex D documents**,
and this is where it was being applied anyway. `ZapfDingbats.pdf` is a specimen sheet that prints,
in Helvetica beside each dingbat, the name and the Unicode value that dingbat has; the page used to
read `! a1 [x2701]` and reads `✁ a1 [x2701]` now. The document stating its own expected answer is
evidence about our reading of Annex D.6 and not the source of it (principle 5), and it is what
`silent_fonts.rs`'s new test asserts against.

**Neither text gate moves, and the reason is worth more than the flatness.** It is not the usual
"both strip whitespace" (session 464): `reference_words` trims every reference token to its
alphanumeric core and drops what is left under three characters, so a `pdftotext` line holding one
dingbat is not a word at all — and `issue15716.pdf` is a document `pdftotext` extracts nothing
from. Both documents are therefore invisible to the instrument that would have scored them, which
is exactly the shape of trap 1 one directory over: a gate that cannot see a population is not a
gate saying the population is fine. `pdftotext` does read those characters, and its agreement is
evidence about the reading rather than its source.

Two things follow that are not readback: `LoadedFont::code_for` builds its character-to-code table
by *running* `text`, so a §12.7.4.3 appearance in `ZapfDingbats` could previously encode `!` to a
dingbat's code and now encodes the dingbat; and nothing about the picture changes, because the
route touched is the one that says what a code *means* rather than which glyph it selects.

## What stays counted, and why that is the answer rather than a backlog

- **761 `Identity`-ordering codes with no `/ToUnicode`.** The clause excludes them from the third
  method by name and §9.7.4.2 states the reason: a CID indexes the glyphs of the font that defined
  it. This is "there is no way to determine what the character code represents", in the standard's
  own words, and no reading closes it.
- **210 codes a producer's `/ToUnicode` omits.** §9.10.3 requires no completeness, and the
  program's own `post` and `cmap` have already been asked (ADR 0259's route). What is missing is in
  the file.
- **133 unlisted names.** `G14`, `LW010000`, `c128`, `circlecopyrt` — 14 corpus documents lose
  exactly one code to that last one, a Computer Modern name for © that no list §9.10.2 names holds
  — and pdfTeX's `/aNNN`, refused on ADR 0311's reading, which this round re-examined and did not
  weaken.
- **123 codes that select a glyph by code and name nothing**: codes 0–31 and code 10 in a simple
  font, code 0 reaching `.notdef`. Annex D leaves the control range unencoded in every table, so
  there is nothing to look up.

A count that does not fall is the honest answer where the clause says there is none, and this ADR
is the argument that the remaining 1228 are that.

## What was deliberately not done

- **No report.** ADR 0152's arithmetic is unchanged: 43 documents that mostly draw perfectly would
  cost the oracle 43 judged pages for a shortfall in the readback.
- **No `zapfdingbats.txt`.** The characters come from ISO 32000-2's own Table D.6. Adobe's list
  would have produced the same table and would have been the wrong source — and would have offered
  no principle for excluding pdfTeX's `/a192`, which is precisely what the annex's own key supplies.
- **No Annex D.6 route for an *embedded* font that happens to use those names.** §9.6.5.1 gives an
  embedded program its own built-in encoding and the annex documents the `ZapfDingbats` font
  program's; a document that embeds something else under those names has said nothing this table
  can be applied to. `symbolic_set` is `Some` only where this crate resolved the font as one of
  §9.6.2.2's fourteen.
- **No change to `text_from_the_code`'s bound.** The printable-ASCII permission is untouched for
  every font whose encoding is Latin; what changed is that a font whose character set the standard
  documents now answers before that permission is reached.

# 0433 — The head of the ink sweep is a sentence that forbids the file

Status: accepted.
Context: `doc/todo/00` step 7, the ambiguous bucket's ink sweep.

## What was measured

`doc/todo/00` step 7 is our ink minus the lightest reference's, over every ambiguous page, from
the artefacts an oracle run leaves on disk. Re-run whole in the five-hundred-and-ninety-eighth
session over all 786: **19 at or past −1, 16 of them documents this tree already calls
incomplete**, head

```
-19.447  issue12418_reduced.pdf   ours 0.000 vs hayro       19.447   [incomplete]
-13.810  issue4722.pdf            ours 0.000 vs mupdf       13.810   [incomplete]
-12.927  issue15977_reduced.pdf   ours 0.000 vs poppler     12.927   [incomplete]
-11.272  bug1050040.pdf           ours 0.000 vs hayro       11.272   [incomplete]
 -8.991  issue5801.pdf            ours 0.000 vs ghostscript  8.991   [incomplete]
```

`ours 0.000` is literal: this tree lays down no ink at all on those pages, which is the one
thing a distance ranking cannot see and the reason step 7 exists.

**Eleven of the sixteen incomplete names in the negative tail are one cause**, and it is not the
one a group name would have guessed. `examples/open_one` on each prints the same report —

> `font /F1 uses unsupported encoding neither a /ToUnicode nor a registered character
> collection, so a substitute cannot be addressed (§9.10.2)`

— on `issue12418_reduced`, `issue4722`, `issue15977_reduced`, `issue15443`, `issue15441`,
`issue19695`, `issue15594_reduced`, `issue5801`, `issue11242_reduced`, `issue11578_reduced` and
`issue13916`. Every one of the eleven states the same construction, read out of the file's own
bytes: a Type 0 font with `/Encoding /Identity-H`, a `CIDFontType2` descendant with a
`/FontDescriptor` carrying **no** `/FontFile2`, `/CIDSystemInfo` `(Adobe) (Identity) 0`, and no
`/ToUnicode` anywhere.

## The clause

§9.7.5.2, verbatim, and it is about the *file* rather than about the reader:

> The Identity-H and Identity-V CMaps shall not be used with a non-embedded font. Only
> standardized character sets may be used.

§9.7.4.2 says why, from the reader's side:

> If the TrueType font program is not embedded but is referenced by name, and the Type 2 CIDFont
> dictionary contains a CIDToGIDMap entry, the CIDToGIDMap entry shall be ignored, since it is
> not meaningful to refer to glyph indices in an external font program. In this case, CIDs shall
> not participate in glyph selection, and only predefined CMaps may be used with this CIDFont
> (see 9.7.5, "CMaps"). The PDF processor shall select glyphs by translating characters from the
> encoding specified by the predefined CMap to one of the encodings in the TrueType font's "cmap"
> table. The means by which this is accomplished are implementation-dependent.

The route the clause defines starts from **characters**, and Table 116 gives `Identity-H` none:
it "maps 2-byte character codes ranging from 0 to 65,535 to the same 2-byte CID value". A CID is
an index into a font program that is not here. So the clause's own glyph-selection route has no
input, `/ToUnicode` is absent, and `/Ordering (Identity)` is not a registered character
collection — §9.10.2's three methods are exhausted, which is exactly what the refusal says.

## The references are the prohibition's evidence, not a case against it

Four programs, four different readings, measured off the side-by-side strips and the artefacts:

- `issue15977_reduced.pdf`, codes `35 28 26 32 30 28 27 24 26 2c cf 31` —
  one reference reads each code as a Unicode scalar and draws `5(&20('$&,Ï1`; two read it as the
  standard Macintosh glyph ordering a `post` format 1.0 table defines (53=R, 40=E, 38=C, …) and
  draw `RECOMEDACIÓN`; the fourth draws something else again.
- `issue12418_reduced.pdf` — `Uvolnrn² vinkulaceï`, `Uvolnění vinkulace –`,
  `r ½ ⌐ ⌐ …`, `Uvoln ní vinkulace –`. Four panels, four strings.
- `issue19695.pdf` — **two of the four references draw nothing either**, ink 0.000 for both,
  which is this tree's answer arrived at independently twice.

Pairwise mean absolute difference between the references, in levels of 255, closest *voting*
pair per page: `issue11578_reduced` 4.63, `issue11242_reduced` 6.18, `issue19695` 6.22,
`issue15594_reduced` 7.85, `issue5801` 8.05, `issue4722` 12.05, `issue15977_reduced` 12.27,
`issue13916` 12.84, `issue12418_reduced` 16.15, `issue15443` 16.41, `issue15441` 16.51 — against
a text tolerance whose mean bound is 5.00.

## Decision

**The eleven pages are the report doing its job and no code changes.** Adopting the
standard-Macintosh-ordering guess would be curve-fitting to two renderers against a clause that
says CIDs shall not participate in glyph selection, which `CLAUDE.md`'s principle 5 forbids
outright. What changes is that §9.7.5.2's `shall not` is now quoted beside the refusal in
`pdf-font/src/loading.rs` and in the clause's ledger row, so the next reader of that message
learns that the *file* is what broke a rule.

## The price, and it is zero

`doc/traps/` trap 11's arithmetic is that a report takes a page off the oracle's judged set, and
nineteen pages would be a large price. It is not being paid here, and the reason is read off
`tools/pdfref/src/lib.rs::decide` rather than assumed: `Outcome::Ambiguous` is returned when no
mutually-agreeing subset of two or more references exists, **before our own comparison is
consulted at all**. The eleven pages are `ambiguous` because the references cannot agree with
each other — measured above, 4.63 to 16.51 of 255 at the closest voting pair — and would stay
`ambiguous` whatever this tree drew.

What a report *does* cost is a place in `oracle.rs::check_the_ratchets`, whose `named` closure
filters on `e.complete`; the incomplete pages are held instead by the corpus gate's own list,
where this population is the `Text` row's "27 with no `/ToUnicode` so a substitute cannot be
addressed". Two ratchets, one page each, and neither is blind.

## What would change the answer

A `/ToUnicode` appearing in one of these files, or the clause being amended. Neither is this
tree's to arrange. A future round that finds this head again should read this file rather than
re-derive it: the head has been `issue12418_reduced.pdf −19.447` since at least ADR 0237's run,
and it has been unexplained for every one of those sessions.

## One clause of this corrected, in the thousand-and-eighth (ADR 1027)

The description above says each of the eleven states "no `/ToUnicode` anywhere". One of them does
state one: `issue12418_reduced.pdf`, this ranking's own head, writes `/ToUnicode /Identity-H` — a
**name**, where §9.10.1 says the value "shall be a stream object" and Table 119 types it `stream`.
So the entry is there, it is not a `CMap`, and `read_to_unicode` rightly takes nothing from it;
what was wrong was the *refusal's* sentence, which said the file had written none.
`examples/to_unicode_kind_census` counts the construction at **16 font dictionaries over 10
documents** in 1 239 opened files, every one of them that same name.

Nothing else here moves. §9.7.5.2 is still what decides these eleven pages, the decision is still
that no code changes for them, and the refusal is still right — the name is not a `/ToUnicode`
`CMap` and §9.10.2's first method is still unavailable. ADR 1027 §"Why the construction is not
recovered" says on which clause that stands.

## The population has a control in it, found in the thousand-and-thirtieth

The eleven above are the *negative tail's* members, not the clause's population: the corpus gate
counts **eighteen documents** under "an Identity CMap over a font the file did not embed
(§9.7.5.2)", and this file has never named the other seven. One of them is the sharpest witness
the corpus holds for this decision, and it is below the alarm rather than at its head —
`issue6127.pdf` pages 1 and 2, −0.797 and −0.792.

**It breaks the sentence and keeps it, inside one text object.** Page 1's attestation paragraph
runs in `/C2_2` — `DPOKGC+TimesNewRoman`, `/Encoding /Identity-H`, descendant with a `/FontFile2` —
as far as *"…Assurance Maladie Maternité et "*, and the very next operator selects `/C2_14`:
`TimesNewRoman`, the same `/Encoding`, no program and no `/ToUnicode`. Same face, same `CMap`, same
sentence, adjacent operators. So this page states the counterfactual the eleven could only be
argued about: what the *reader* does is identical on both halves and only the file differs.

That is why the decision above is now held by a test rather than by an argument alone —
`pdf-model/tests/silent_fonts.rs::an_identity_cmap_over_a_font_the_file_did_not_embed_is_refused_beside_one_it_did`,
whose drawn half is what stops it passing on a blank page. `doc/todo/00` carries the measurement,
including the four references' readings written as arithmetic off the file's own CIDs.

## The class is derived from the files, checked whole in the thousand-and-fifty-fourth

The corpus gate counts eighteen documents under this clause and `whose_defect` places each there by
matching the refusal's own wording, so membership was a statement about this reader's control flow
rather than about the document — a class that can only confirm itself. Read out of the eighteen
files instead, every one states `/Encoding /Identity-H` on a Type 0 whose descendant's
`/FontDescriptor` holds none of Table 120's three keys; the two documents that also carry embedded
`/Identity-H` fonts (`issue6127.pdf` nineteen, `ThuluthFeatures.pdf` four) report only the
unembedded ones. Nothing is hiding behind the class.

The one member this tree had never named is `issue19550.pdf` — `/BaseFont /Arial`, one `Tj` of
nineteen CIDs on 200 × 50 points, no `/ToUnicode`, a descriptor with metrics and no program. Ours
is blank; `poppler`, `ghostscript`, `hayro` and `mupdf` draw four different strings, and the first
two log the prohibition in their own words. `doc/todo/00` step 7 carries the reading.

**And the premise the refusal stands on is now the guard's.** `collection_gap`'s first branch
asserts that the *file* embedded no program; what entitled it was the call site, which admitted
`FontError::UnsupportedProgram` beside `NotEmbedded`. Nothing has constructed that variant since
§9.9's `/FontFile` was implemented (ADR 0040), so no message was ever wrong — but a second error
kind routed to substitution would have carried this clause's `shall not` to a file that did embed a
program, which is a gap of this reader's wearing the file's name. The variant is deleted and both
guards name `NotEmbedded` alone.

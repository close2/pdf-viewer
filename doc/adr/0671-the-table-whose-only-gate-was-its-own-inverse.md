# 0671 — The table whose only gate was its own inverse

Status: accepted.
Context: the successor selection rule's fourth step, written by the round that could not run it —
and the head it produces once it runs.

## The rule, corrected

ADR 0627's rule, with ADR 0637's repair to its second step, ADR 0653's tie-break, and the fourth
step ADR 0660 recorded as owed:

> Rank each ledger row by the errata annotations that fall on it whose issue number this tree names
> nowhere. Rank once over the live rows and once over **every** row, take the head of the two, and
> prefer the settled row where they tie. Reassemble the issue from every clause `emit` files it
> under, and read the issue whole.

Two things were owed and both are done here. `doc/todo/01`'s recipe carries the fourth step as an
argued ordering rather than as a note, and its second step carries the writing rule ADR 0660 named:
**an issue number written outside `doc/errata-read.md` carries the `Issue #` prefix**, because the
bolded bare form `**#214**` is invisible to both of step 2's greps and a third grep is not available
— `doc/HAYRO_ISSUES.md` lists another project's issues under live errata numbers.

**Why the settled rows belong in the ranking, and why they are not simply ranked above the live
ones.** A live row's count ranks a debt the ledger already declares; the round that pays it off will
re-read the clause anyway. A settled row's count ranks a *claim* — `implemented` says every
requirement in the clause is executed, `inapplicable` says none reaches this program — and
`CLAUDE.md` says both kinds decay, with its own §10.5 entry as the standing example of an
`inapplicable` that was wrong. An unread erratum on such a row is the only signal this project has
that one has decayed. But ordering the settled rows above the live ones outright would throw away
the count, which is the whole instrument; so it is one ranking over every row, with the tie going to
the row that asserts more.

## The head, and it is not close

Over live rows: **§14.8.5.3, seven annotations** — the plateau ADR 0660 left standing, §7.7.4 having
left the ranking because ADR 0660 read it. Over every row: **§D.3, fifteen annotations under Issue
#285, Issue #461 and Issue #562**, more than twice the live head, on a row that is `implemented`.
§9.6.4 and §7.4.1, the two ADR 0660 measured from outside, are second and third at 11 and 8.

Of the 302 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` carrying a strike or a caret, 126
are named nowhere in this tree at this base.

## What the three errata turned out to be

`doc/errata-read.md` has all three with the rectangle that places each, against `pdftotext -bbox`.
Every one of them corrects Table D.3's own *presentation* and not one moves a byte this program
decodes:

- **Issue #562** strikes the alias `(END OF TEXT)` off code 0x04 — where it is already 0x03's — and
  writes *END OF TRANSMISSION*, and strikes 0x05's `(END OF TRANSMISSION)` and writes *ENQUIRY*.
  Both are codes the annex marks `U`. Its other half is in the `Character` column: the `u` and `v`
  printed for 0x18 and 0x19, where a breve and a caron belong.
- **Issue #285** corrects code 0x16's `Unicode` cell from U+0017 — printed twice, on 0x16 and 0x17 —
  to U+0016, and a spelling in its alias. 0x16 is a `U` code under both printings.
- **Issue #461** strikes the `Š` printed in code **0x8a**'s `Character` column, whose `Unicode` cell
  says U+2212 and whose name cell is empty, and writes `−` and *MINUS*.

**The three glyph corrections are written in `PDFDocEncoding` itself**, which is worth knowing
before trusting a tool that prints them: each caret's `/Contents` is the single byte 0x18, 0x19 or
0x8a, so `spec-errata emit` renders the erratum's replacement *through the table it corrects*. The
annex's `Unicode` column is the independent side of that circle.

## What reading them made this round look at

**§D.3 was `implemented` on a test that cannot see a wrong transcription.** Its cited gate was
`text_string.rs::every_text_string_survives_the_round_trip`, and a round trip is a statement about
an encoder and a decoder that share one array: `encode_text_string` searches the array
`text_string` indexes, so a code transcribed with the wrong character round-trips perfectly and
draws a document's text wrong. 232 of the annex's mappings had no other assertion than seven
spot-checked codes.

**Issue #461 names the exact mistake that gate would have missed**, which is why this is a finding
rather than a tidy-up. The standard prints `Š` in the `Character` column of the row whose code is
0x8a and whose `Unicode` column says U+2212; `Š` is 0x97's character. A transcription taken from the
column the erratum corrects would put U+0160 at 0x8a — a minus sign decoding as a capital S with
caron, on any file that writes one. Planted, **all ten of the module's tests stay green**, including
the round trip the row cited.

`crates/pdf-syntax/tests/pdf_doc_encoding.rs` closes it: all 256 rows read out of `doc/md/` and
compared in both directions, decode and encode stated separately because they fail differently — a
character transcribed onto two codes decodes differently and encodes alike, and only the reverse
direction sees it. Calibrated per trap 13 with three plants, each removed: 0x8a as `Š`, which fails
both tests and no other; codes 0x12 and 0x13 given the `Unicode` cell of a `U` row, which fails the
decode test alone and which **no pre-existing test sees at all**; and the pair swap at 0x18/0x19,
which the module's own `the_table_is_not_latin_1` does catch.

The parse is positionally independent, because the conversion mangles this table's columns from one
page block to the next: a row's single `0xNN` is its code and its single `U+NNNN` is its character.
The one place that needs care is telling the annex's `U` note from a character that *is* `U`, and
code 0x55 is why — so the cell showing exactly the character the `Unicode` column names is dropped
before the row is searched for a bare `U`, which identifies the `Character` column by what it is for
rather than by where the conversion put it.

**And the row named the wrong table for its whole life.** Its note said the code was "the fourth
column of Table D.2" — a font encoding keyed by glyph name, which is `pdf-font`'s — where
`text_string.rs` holds Table D.3's code-to-Unicode column. Nothing could print it: the ninth sweep
reads `Table NNN` citations, and an annex table's number is outside its population. The parent §D
row's claim that all five tables are "transcribed from `doc/md/` and gated" was an overstatement of
this member's evidence and is true now.

## Consequences

- The rule has a fourth step that has been run, and `doc/todo/01` argues its ordering rather than
  stating it.
- `PDFDocEncoding` is gated against the annex rather than against itself. No behaviour moves: the
  transcription was right at all 256 codes, which is what the round set out to check rather than
  what it assumed.
- The three errata leave the population, and §D.3 leaves the head of the full ranking. §9.6.4 and
  §7.4.1 are what the sixth use inherits.

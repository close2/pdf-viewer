# 1027 — A second statement about the same bytes belongs in the refusal: Table 125 beside RFC 1950's check value, and a `/ToUnicode` the file *did* write

Status: accepted. Session 1008.
Clauses: ISO 32000-2 §9.9 Table 125 (`/Length1`, an embedded program's extent), §7.4.4.1 (the two
RFCs `FlateDecode` makes normative), §9.10.1 (a `/ToUnicode`'s value "shall be a stream object"),
§9.10.2 (the ranked methods for reading a code as a character), §9.10.3 (what a `/ToUnicode` `CMap`
is), Table 119 (the Type 0 font dictionary), §9.7.5.2 (the prohibition ADR 0433 read).
Code: `crates/pdf-font/src/program.rs` (`whole_program`'s `stated` and `extent_corroboration`),
`crates/pdf-font/src/composite.rs` (`collection_gap`'s fifth fact and `to_unicode_gap`),
`crates/pdf-font/src/loading.rs` (the one call site).
Tests: `crates/pdf-font/src/program.rs::a_check_value_refusal_says_so_when_the_stated_extent_disagrees`,
`::a_check_value_refusal_says_the_extent_agrees_when_it_does`,
`::a_compact_font_program_states_no_extent_to_corroborate_a_check_value_with`;
`crates/pdf-font/src/composite.rs::a_to_unicode_that_is_not_a_stream_is_not_an_absent_one`, and the
row added to `::four_facts_about_a_file_reach_four_different_refusals`.
Measurement: `crates/pdf-font/examples/to_unicode_kind_census.rs`.
Documents: `doc/todo/00` §7, §9.9's and §9.10.1's ledger rows.
Predecessors: ADR 0836 (the check-value refusal), ADR 0433 (§9.7.5.2's population), ADR 0343 and
ADR 0459 (why a font program of the wrong content is refused), ADR 1022 §5 (session 1002's head
read, which left the first half of this owed).

## The shape both halves have

A refusal is a sentence about a file, and a sentence about a file can be checked against the file.
Twice in this round a refusal that was **right** was printed beside a *second* statement the
document makes about the very same bytes — and in each case the code had the second statement one
call away and did not make it.

| | what refuses | the second statement | what the refusal said instead |
|---|---|---|---|
| a damaged `/FontFile2` | RFC 1950's Adler-32 (§7.4.4.1) | §9.9 Table 125's `/Length1` | nothing about the extent |
| a substituted composite font | §9.7.5.2's `shall not` | §9.10.1's "shall be a stream object" | "it states no `/ToUnicode`" |

The first is a *missing* corroboration. The second is worse and is ADR 0836's own lesson arriving
one crate over: the sentence was **false of ten corpus documents**, because a `/ToUnicode` a reader
could not use is not a `/ToUnicode` the file did not write.

## Half one: Table 125 is asked on a check-value failure too

`pdf_font::program::whole_program` consulted [`stated_extent`] on `Damage::Truncated` and on
nothing else. ADR 0836's own paragraph is why — on a check-value failure there is "no shortfall
against Table 125's extent", so the clause had nothing to *decide*. That is true and it is not the
only thing a clause can do.

**Session 1002 proved it by hand.** Opening `bug1050040.pdf` page 1 — the ink sweep's fourth head
row at −11.272, where this tree draws nothing and four references draw a legible line — it found
ADR 0836's refusal right on two witnesses the code had never been shown:

- **Table 125 states 59212 and the filter delivers 59211.** The clause states the extent "after it
  has been decoded using the filters specified by the stream's Filter entry, if any", which is a
  statement the filter cannot influence, so it is independent of the checksum and here it says the
  same damage.
- **The font's own per-table `checkSum` fields** put the missing byte inside `glyf`: five tables
  check in place, seven check exactly one byte early, and `glyf` checks at no offset at all.

The first of those two is a dictionary lookup. It is now made, on the damaged path only, and it
reaches the refusal as `extent_corroboration`'s sentence. What `bug1050040.pdf` prints today:

```text
/FontFile2 decoded whole and its check value disagrees (59211 bytes): RFC 1950's Adler-32 says
these are not the bytes that were compressed, and a font program whose content may not be its own
draws glyphs in place of the producer's, and §9.9's Table 125 states 59212 against the 59211 that
arrived — 1 byte short of the extent the file states, so the document says the same damage a
second time and independently of the filter
```

**The agreeing answer is printed too, and that is the half a corroboration is worthless without.**
`issue13316_reduced.pdf` — ADR 0836's other witness, and the document that declined admitting this
damage — decodes to 168 808 bytes under a `/Length1` of 168 808:

```text
… and §9.9's Table 125 states 168808, which is exactly what arrived — so the file's own extent
corroborates nothing here and the check value stands alone
```

A reader told nothing cannot tell that case from the case where nobody looked. A `/FontFile3`
states no extent at all by §9.9's own sentence — the three lengths "are not needed in that case and
shall not be present" — and prints nothing, because a line saying the clause is silent would appear
on every bare CFF in every corpus.

**The refusal is unchanged in all three.** RFC 1950's compliance clause decides it alone, and a
checksum over 168 808 bytes says what a checksum over 59 211 says with the same confidence. What
changed is what the reader is told — and a round that ever wants to soften the check-value refusal
now finds `/Length1`'s answer printed rather than owed.

## Half two: a `/ToUnicode` the file wrote and this reader could not use

The round's second job was to read another head off `doc/todo/00`'s rankings. **Every name at or
past the ink sweep's −1.00 alarm is read** (§7 below), so the name taken was the first one under it
that is named nowhere in `oracle.rs`: `issue11915.pdf` page 1 at **−0.636**, ours 0.6542 against
`mupdf`'s 1.2900.

### What the document states

It is a **font specimen**. Five lines, each naming a face, each shown in that face:

```text
( A r i a l) Tj      /Type0TTF0   — /BaseFont /Arial
( A r i a l   I t a l i c) Tj     — /Arial-ItalicMT,Italic
( C a l i b r i) Tj               — /Calibri
( C o u r i e r   N e w) Tj       — /CourierNewPSMT
( T i m e s   N e w   R o m a n)  — /TimesNewRomanPSMT
```

The spaces are the giveaway: the strings are two-byte codes, and `00 41 00 72 00 69 00 61 00 6C` is
`Arial` in UTF-16BE. Each of the five is a Type 0 font over `/Encoding /Identity-H` whose
`CIDFontType2` descendant has **no** `/FontFile2`, states `/CIDSystemInfo` `(Adobe) (Identity) 0`,
and carries a `/CIDToGIDMap` **stream**. The page's other marks are a stroked rectangle.

### What each renderer did

Off the gate's own artefacts, each panel opened rather than inferred from its position (trap 1 —
this paragraph named the wrong two renderers until they were). Ours draws the rectangle and nothing
else. `mupdf` and `hayro` set all five lines correctly; `poppler` sets four and garbles the
*Calibri* line into `/1Ʈ⁄₆ρA`; `ghostscript` sets five lines of `^¸®±` and `q®§¹k§¾o´²`.

**Four references, three pictures, one machine.** The CIDs are glyph indices into a program the
file does not carry, so what each reference draws is an index into whatever face *it* found —
correct where the face's glyph order happens to match the producer's, one garbled line where one
face out of five does not, and five garbled lines where none of them does. That is not an
implementation difference; it is the same file rendering differently according to what is installed,
which is exactly what §9.7.5.2's `shall not` exists to forbid. A stronger statement than ADR 0433's
"four programs, four readings", and it came off looking at the panels.

### Which clause decides it

§9.7.5.2, about the file rather than about the reader:

> The Identity-H and Identity-V CMaps shall not be used with a non-embedded font. Only
> standardized character sets may be used.

and §9.7.4.2 from the reader's side, which also disposes of the `/CIDToGIDMap` stream: it "shall be
ignored, since it is not meaningful to refer to glyph indices in an external font program", and
"CIDs shall not participate in glyph selection". So the clause's glyph-selection route starts from
*characters* and this file gives it none. **We are right**, and this page is ADR 0433's population
with one file more in it.

### The defect the page did have, and it is in the report

The refusal this tree prints ended, for every one of `collection_gap`'s four facts, with

> — and it states no `/ToUnicode` to read the codes by instead (§9.10.2)

and **`issue11915.pdf` states five of them**. Each of its Type 0 dictionaries writes

```text
/ToUnicode /Identity-H
```

— a *name*. §9.10.1 says of that entry that its

> value shall be a stream object containing a special kind of CMap file that maps character codes
> to Unicode values

and Table 119 types it `stream`. So `read_to_unicode` is right to take nothing from it, and the
refusal was wrong to say the file wrote nothing. That is exactly ADR 0836's finding one crate over
and about a *report* rather than about a decoder: **what a reader could not use is not the same
statement as what a file did not state.**

`collection_gap`'s own doc comment is where this stings, because it is the comment that says a
report firing on four conditions has named none of them — and it was carrying a fifth unseparated
the whole time. There are now five, and the fifth has three answers: no entry; an entry that is not
a stream, with the value's own kind and a name's own spelling; and a stream that produced no
mapping, which `read_to_unicode` also answers empty for.

### The population, counted rather than guessed

`crates/pdf-font/examples/to_unicode_kind_census.rs` walks every object the cross-reference table
names and every dictionary nested inside one — trap 25's rule, because `issue11555.pdf` writes a
whole Type 0 inline in a page's `/Resources` — and asks every `/Type /Font` dictionary what kind of
thing its `/ToUnicode` is. Over `doc/pdf.js` and `doc/corpora`, **1 251 files of which 1 239
open, 3 941 font dictionaries**:

```text
968 state a /ToUnicode, 952 of those as the stream §9.10.1 requires and 16 as something else
  the name /Identity-H: 16 font dictionaries over 10 documents
    IdentityToUnicodeMap_charCodeOf.pdf 1   bug1650302_reduced.pdf 1   bug920426.pdf 1
    issue11915.pdf 5                        issue12418_reduced.pdf 1   issue3323.pdf 1
    issue4402_reduced.pdf 1                 issue7200.pdf 2            javauninstall-7r.pdf 1
    sample_fonts_solidconvertor.pdf 2
and 55 of the 952 streams state no bf mapping at all, over 27 documents
```

Three things in that table are worth more than the count. **Every one of the sixteen is the same
value** — not one array, dictionary, string or number anywhere in two corpora — so producers that
get this wrong get it wrong in exactly one way, and a report that names the value is naming
something a reader can act on. **`issue12418_reduced.pdf` is the ink sweep's *head*** at −19.447,
the top row of ADR 0433's own eleven: the refusal printed on the single most-looked-at page of this
bucket was saying the false half of this sentence, and had been since the sentence was written.

**And the third answer is the commonest and was nearly written off, which the golden caught.** The
doc comment for it said no corpus document reaches it. `issue5801.pdf` — a *fourth* of ADR 0433's
eleven — does: its `/ToUnicode` is a stream, and the stream is a copy of the **`Identity-H` CID**
`CMap`, every mapping a `begincidrange` and not one `beginbfchar`, so a `/ToUnicode` reader finds
nothing in it. The last line of the census counts that shape at **55 streams over 27 documents**,
more than three times the name population. Trap 11 from both ends in one round: a report that fires
on a condition it does not name, and a comment asserting a population nobody had counted. The
number is in the code's own comment now, beside the arm.

**ADR 0433's reading is corrected by one clause and otherwise stands.** Its description of the
eleven says each has "no `/ToUnicode` anywhere"; one of the eleven has one, of the wrong type. The
clause that decides those pages is §9.7.5.2 and nothing here touches it.

### Why the construction is not recovered, stated rather than assumed

Reading `/ToUnicode /Identity-H` as "the code is already the Unicode value" would draw
`issue11915.pdf`'s five lines and `issue12418_reduced.pdf`'s, and it is declined. §9.10.3 defines a
`/ToUnicode` `CMap` as the *contents of a stream*; Table 116's predefined `CMap`s map codes to
**CIDs**, not to Unicode; and no clause anywhere says what a name in that position would mean. That
the codes on this page happen to be UTF-16BE is a fact about one producer, and adopting it because
three references land on the right letters is curve-fitting to other implementations, which
`CLAUDE.md` principle 5 forbids outright. What the round owes such a construction is a report that
says what the file did, which it now has; a round that wants to recover it has an ADR to argue
against and a census that prices it at ten documents.

## What moved, and what did not

**No pixel.** Both halves change the text of a report and nothing else: the refusals are the same
refusals, on the same conditions, for the same documents — and `raster_golden` is the instrument
that says so rather than the claim. Over 974 tracked documents it held **969 and moved 5**, every
one of the five labelled `reports only (a change in the diagnosis, trap 37)`:

| page | which half |
|---|---|
| `bug1050040.pdf` p1 | Table 125 corroborates: 59212 against 59211 |
| `issue13316_reduced.pdf` p1 | Table 125 agrees: 168808 against 168808 |
| `issue11915.pdf` p1 | `/ToUnicode` is the name `/Identity-H` |
| `issue12418_reduced.pdf` p1 | the same, on the ink sweep's head |
| `issue5801.pdf` p1 | `/ToUnicode` is a stream that states no `bf` mapping |

**The fifth is the one worth having run the gate for.** Nothing in this round predicted it: the
third arm of `to_unicode_gap` was written for a condition its own comment said no document reached,
and the golden named a document reaching it before the ADR was finished. `doc/checks/fixed-documents.toml`'s
one row that asserts on this text matches a substring that survives.

## The general lesson

**A refusal that is right is still a measurement, and a measurement with one input is the weakest
one available.** Both halves of this round had a second, independent statement about the same bytes
sitting in the same dictionary — Table 125's extent beside RFC 1950's checksum, §9.10.1's type
beside a reader's inability to use the value — and in both cases the code had reached for it in
*some* other branch and not in this one. The rule that falls out is narrow enough to apply:

> When a refusal names a fact about a file, ask whether the file states that fact twice. Where it
> does, print both — including when the second one agrees, because a reader cannot tell a
> corroboration that was not found from one that was not sought.

# 0988 — The last owed rewrite, and the file its writer was not in

Session 976. Status: **accepted**. It builds the twenty-second and last of
`doc/pdf-a-mitigations.md` §13.3's *owed, not optional* rewrites —
`fonts/vertical-metrics-agree-with-the-program` — and records a third correction to the same
catalogue entry, which had now been wrong about the direction, about what it waited on, and about
where the writer it waited on lives.

Context: `crates/pdf-font/src/restate.rs`,
`crates/pdf-transform/src/archive/{fonts,decision,prepare,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-mitigations.md` §5, §13.3, §13.3.1,
`doc/pdf-a-conversion-limits.md` §4.9, ISO 19005-4 6.2.10.5, ISO 32000-2 §9.2.4, §9.7.4.3,
§9.9.1, ISO/IEC 14496-22 (`vhea`, `vmtx`, `head`).

## 1. The clause that settles the direction, and the one beside it that was never read

ISO 19005-4 6.2.10.5's third paragraph asks of a composite font shown in writing mode 1 what its
first paragraph asks of every font across the page: the dictionary's numbers and the program's
shall agree. Part 2 states no such rule at all.

Two rounds have argued about which side may move. The entry first proposed restating `/DW2` and
`/W2` from the program; ADR 0965 overturned that on §9.2.4 — the dictionary's numbers are what
positions a glyph *without looking inside the program*, and §9.7.4.3 gives `/DW2` and `/W2` that
role going down the page, so restating them moves every glyph on a vertical line.

**§9.9.1 says it outright, and is stronger than the inference:**

> The "vhea" and "vmtx" tables that specify vertical metrics shall never be used by a PDF
> processor. The only way to specify vertical metrics in PDF shall be by means of the DW2 and W2
> entries in a CIDFont dictionary.

So the direction is not a judgement about which statement is authoritative. A conforming processor
is *forbidden* to read the table this rewrite touches, which makes the rewrite unobservable by
construction, and rewriting the dictionary instead would move marks on the authority of a table
nothing may consult.

**The sentence before that one was not read either, and it is the round's own finding.** §9.9.1's
preceding paragraph lists the TrueType tables that "shall always be present if present in the
original TrueType font program" — `head`, `hhea`, `loca`, `maxp`, `cvt `, `prep`, `glyf`, `hmtx`
and `fpgm` — and `vhea` and `vmtx` are not among them. Read with the sentence that follows, the
standard both declines to require their preservation and forbids their use, which means **removing
them is as lossless as restating them**, and is the `/CIDSet` and `/CharSet` *remove* route one
clause family over. It is not the route taken here, and the reason is the opposite of the usual
one: removal is the *harder* build, because a table directory cannot lose a record without being
rebuilt, while an advance already has a field to be overwritten in. The reading is recorded because
a second reader of this row should not have to find it again.

## 2. Where the writer actually was, and what that cost

The catalogue said the row waited on the writer rather than the reader, and named it:
"`pdf_font::restate` rewrites an sfnt's `hmtx` and nothing vertical". That sentence is true about
the module's *subject* and wrong about its *code*: `restate.rs` owns the units, the tolerance and
the format dispatch, and the byte surgery it dispatches to — the table splice, the directory
update and the checksums — is `sfnt.rs`'s `with_advances`, `rewritten_sfnt` and `checksummed`,
none of which is reachable from outside that module.

This round held `restate.rs` and not `sfnt.rs`. The consequence is a design decision that would
otherwise not have been forced, and it turns out to be the better one:

- **The horizontal restatement must lengthen `hmtx`**, because a font that states one advance for
  its whole tail cannot state a different one for a single glyph of it — so the table moves to the
  end of the file and its record follows.
- **The vertical restatement need not**, because every advance it writes already has a field. The
  glyphs that do *not* have one — the tail past `numOfLongVerMetrics` — are refused by name
  (`VERTICAL_NOT_RESTATABLE`), since giving one of them an advance means lengthening the table and
  thereby restating every other glyph in the tail, and none of those was asked for.

So `with_vertical_advances` overwrites `uint16` fields where they already are, and every other byte
of the program — every offset, every length, every outline — is the producer's.

**The checksums are adjusted rather than recomputed, and that is a claim rather than a shortcut.**
Overwriting a `uint16` at an even offset in a four-byte-aligned table changes exactly one 32-bit
word, so the table's stated checksum moves by that word's own difference and the whole-file sum by
twice it, the record's `checkSum` field being itself a word of the file. Both adjustments are
exact. For a program whose checksums were true, adjusting and recomputing produce the same bytes —
`checksums_hold_across_a_vertical_restatement` recomputes independently and compares. For a program
whose producer's checksums were already wrong, the adjustment carries the error across unchanged,
which is `doc/adr/0947`'s first rule at the level of a byte: a checksum is not what any failed
requirement asked to be changed.

## 3. Two requirements, one stream, and the rule that forced them apart

The two agreements are separate requirements and part 2 states only one of them, so a PDF/A-2
target must leave a disagreeing `vmtx` exactly where its producer left it. That makes
`Rewrite::RestateVerticalFontMetrics` a rewrite of its own rather than a second reason for the
existing one: `wanted_by` is what asks whether a *failed* requirement called for a change, and one
enum variant cannot answer two questions.

But both rewrites replace the **same stream object**, and a font whose dictionary disagrees with
its program in both directions must be restated twice into one set of bytes. Two maps keyed by the
same object would have let the second replacement be dropped in silence. So `Metrics` keeps one
replacement per program object and two sets saying which requirement each answers; the rewriter
writes the stream once and counts it under each requirement that asked.

**The generalisation is worth carrying, and it is this round's habit for the ADR:** when a new
rewrite reaches an object an existing rewrite already replaces, the question is not "which
rewrite wins" but "the object is replaced once, by both". A `BTreeMap<ObjectId, Object>` per
rewrite makes that impossible to express and makes the loss silent.

## 4. Proving the placement, which session 971 made a rule

`proves` reads the rewritten program back — through `restate::advance` and
`restate::vertical_advance`, the two readers a caller holding nothing but bytes has — and refuses
the conversion unless every glyph the restatement named now states the number it was given. A
rewrite that silently missed a site would hand back a document that still fails the requirement it
was converted for and say nothing about it.

The unit test goes one step further and reads the restated program through **`skrifa`'s `vmtx`**,
which is what `LoadedFont::program_vertical_advance` uses and therefore what `pdf_archive`'s own
rule will ask after the conversion. A rewrite the writer's reader agreed with and the validator's
did not would be the same failure wearing a green test.

## 5. A silent clamp found next door

`with_widths` converted a rounded design-unit advance to the `uint16` an sfnt states it in with
`u16::try_from(...).unwrap_or(0)`. A width past 65 535 design units — pathological, and reachable
from a `/Widths` array a document controls — became an advance of **zero**, written into a program
with nothing said about it. It is now `RestateError::OutOfRange`, named and refused. Principle 1's
"no silent error swallowing" is the rule; what made it findable was writing the same conversion a
second time for the vertical side and asking what its upper bound was.

## 6. What the corpus said, which is nothing

The veraPDF corpus's conversion figures are byte-for-byte unchanged at all six targets. No document
in it is a composite font set vertically whose `vmtx` disagrees with its `/DW2` or `/W2`, so the
sweep cannot rank this row and never could have. `CLAUDE.md`'s two denominators are the whole of
the answer: this is a coverage question, the specification is its denominator, and a round that had
waited for the corpus to ask for it would have waited forever. What moved is the census —
`fonts/vertical-metrics-agree-with-the-program` leaves `refused` for `remedy` at the three part 4
targets — and `Unconsidered` stays 0 at all six.

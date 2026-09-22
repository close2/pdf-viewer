# 1197 — A rich text string is a construct this tree already reads, and its characters are the field's

Status: **accepted**.
Context: `crates/pdf-model/src/popup.rs` (`rich_text`, `rich_text_characters`,
`RichTextCharacters`), `crates/pdf-model/src/appearance.rs` (`rich_text_value`,
`rich_text_unformatted`, `text_field_text`), `crates/pdf-model/src/variable_text.rs`
(`Owed::RichTextFormatting`), `crates/pdf-model/src/forms_data.rs` (`read_field`'s owed list),
`crates/pdf-model/examples/field_flag_census.rs`,
`crates/pdf-model/tests/variable_text.rs`.
Amends: ADR 1122 (its report's condition and the claim under it) and ADR 1186 section 1's last
bullet.
Builds: ADR 0224 (the same construct read for §12.5.6.6), ADR 0199, ADR 0111.
Clauses: ISO 32000-2 §12.7.4.3 (Table 228), §12.7.5.3 (Tables 231, 232), §12.5.6.6 (Table 177),
§12.5.6.2 (Table 172), §K.1.

## 1. The claim this tree carried, and what the standard says instead

Two ledger rows, one todo file and two ADRs said the same sentence about Table 228's `/RV` and
`/DS`: *XFA rich text, excluded by `CLAUDE.md` principle 5*. The claim has two halves and neither
survives reading.

**The exclusion does not reach this.** `CLAUDE.md`'s closed list excludes XFA on a permission the
standard grants in §K.1, and the permission is about one thing: "[t]he implementation of such a
schema driven page generation involves considerable effort beyond that for a simple PDF viewer and
therefore a PDF processor may choose to not implement this feature". The feature is Annex K's
schema-driven page generation, reached by the interactive form dictionary's `/XFA` entry. Table
228's `/RV` is not that feature; it is an entry of an AcroForm field dictionary whose *format* the
standard defines by pointing at the XFA specification, the way it points at CSS for a style string
and at XHTML in its own bibliography.

**And this tree already reads the construct.** Table 172's `/RC` and Table 177's `/RC` carry the
same words — a rich text string, see the XFA specification version 3.3 — and `popup::rich_text` has
taken their character data since ADR 0224, on Table 177's own `shall` that `/RC` "shall be used to
generate the appearance of the annotation". One construct, named identically in three tables, was
being read in two of them and declared out of scope in the third. That is not a scope decision; it
is a sentence that was never re-read.

## 2. What is owed, and what is not

§12.7.5.3 says where a text field's text lives and what a processor does with it:

> The contents of this text string or stream shall be used to construct an appearance stream for
> displaying the field

and Table 231 bit 26 says what that text *is* when the flag is set: "the value of this field shall
be a rich text string". Put together, a conforming PDF 2.0 rich-text field holds markup in `/V`,
and the contents a processor constructs the appearance from are that string's character data.
Before this ADR the whole of the markup went to §12.7.4.3's layout, so such a field drew its own
angle brackets, attributes and namespace URI across the page.

**The characters are taken; the formatting is not.** A face, a size, a colour and an alignment are
stated in the markup and in Table 228's `/DS`, in a specification this tree does not hold, and
§12.7.4.3's "the following conventions are not used" sets aside the construction that would place
them. That half stays a departure and stays reported, which is ADR 1122's decision unchanged.
Reading the XFA formatting is still refused for ADR 1122's reason: a plausible appearance in the
wrong place fails worse than an absence with a sentence.

## 3. The condition the report fires on, corrected by a count

ADR 1122 chose Table 228's `/RV` as the condition, on trap 11's rule that a report is worth what
its condition is, and it looked at one of the two entries in the table. `/DS` is the other, and it
is the one producers state. Over the 90 763 documents reachable from `corpus-cache`, `doc/corpora`
and `doc/pdf.js/test/pdfs`, `examples/field_flag_census` counts **451** widgets setting bit 26, of
which **60** state `/RV` and **411** state `/DS` — so the report as ADR 1122 left it was silent on
the large majority of the fields that state formatting this program does not apply. The condition
is now any of the three things that put formatting in the file: `/RV`, `/DS`, or a `/V` that is
itself markup.

**And the defect the extraction fixes has no witness on this disk: 0 of the 451 state a `/V` that
parses as markup.** Producers keep flat characters in `/V` and the markup in `/RV`, which is the
convention ISO 32000-1 asked for in words ISO 32000-2 no longer carries. So no page on this disk
moves, the corpus is silent about the requirement, and the fixtures are the whole defence — which
is `CLAUDE.md`'s two denominators exactly: a corpus cannot rank a requirement no document
exercises.

## 4. Where the extraction stops

A value the flag calls rich text and the parser cannot is drawn as it stands. Bit 26's `shall`
binds the file; a file that sets the flag over plain characters — the shape every producer on this
disk writes — has still stated the characters the field shows, and reading a LESS-THAN SIGN in such
a value as the start of an element would lose everything after it. The test is the same one
[`popup::rich_text_characters`] answers for the annotation side and is made of two facts about the
walk rather than a guess about the text: every token parsed, and at least one element opened. This
is ADR 0111's rule in this clause's terms — a malformed declaration may not erase what the clause
states.

**The flag decides, not the text.** A value that looks like markup under a clear bit 26 is drawn as
its own characters; sniffing it would read a producer's text as a producer's instruction. Both
directions are fixtures, each planted back (trap 13), along with the walk's guard and each of the
report's three conditions.

## 5. What this does not decide

§12.7.8.3.2's Table 249 `/RV`, which ADR 1186 declined to import on the claim corrected here. The
entry's message in `forms_data::read_field` now says what is true — a rich text string whose
formatting §12.7.4.3 does not apply, so importing it would change nothing a reader sees — but
whether it should cross is that clause's row, not this one's. Nor does this decide §12.5.6.6's
free text annotation, whose `/RC` characters are drawn with no report beside them; the entry that
would carry the same departure there is Table 177's `/DS`, and that is §12.5.6.6's row to answer.

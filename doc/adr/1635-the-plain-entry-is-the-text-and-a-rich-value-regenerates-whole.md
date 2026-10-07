# 1635 — The plain entry is the text, and a changed rich value regenerates the whole appearance

Session 1399. Status: **accepted**. Amends ADR 0224 (`/Contents` no longer outranks an `/RC` stating
the same characters) and ADR 1122 (its regeneration `shall` is carried out); none is edited.
Context: `crates/pdf-model/src/rich_text.rs` (`for_field`, `for_free_text`, `written`, `write_beside`),
`crates/pdf-model/src/appearance.rs` (`regenerate`, `regenerated_whole`), one call in
`crates/pdf-model/src/view.rs`'s save; ADR 1634. ISO 32000-2 §12.7.4.3, §12.7.5.3 (Table 231),
§12.5.6.2, §12.5.6.6 (Table 177), §12.5.5; XFA 3.3 section 27, *Version Specification*, page 1222.

## 1. Which entry wins, and where the standard says so

The brief for this build read §12.7.4.3 as saying which of `/RV` and `/V` wins. It does not; two
other places do. §12.7.5.3:

> The field's text shall be held in a text string (or, beginning with PDF 1.5, a stre am) in the V
> (value) entry of the field dictionary. The contents of this text string or stream shall be used to
> construct an appearance stream for displaying the field

and Table 231 bit 26: "If the field has a value, the RV entry of the field dictionary ("Table 228 -
Additional entries common to all fields containing variable text") shall specify the rich text
string." So `/V` holds the field's text and `/RV` specifies that text's rich form. **`/RV` is drawn
where its characters are `/V`'s**, compared with white space compressed as chapter 27 lays it out;
**where they differ, `/V` wins**, in the field's `/DS`, and `Owed::RichTextDisagrees` says so: a rich
text string with other characters describes another value. A `/V` that is itself markup is the rich
text string bit 26's first sentence says it is (ADR 1197). Without the flag neither entry is read:
for such a field "The text shall be presented in a single style (font, size, colour, and so forth),
as specified by the DA (default appearance) string."

A free text annotation's pair is decided the same way by §12.5.6.2: "The annotation's Contents entry
specifies the displayed text", and NOTE 1 expects the two "textually equivalent". `/RC` is drawn where
it states `/Contents`'s characters, or where there is no `/Contents`; otherwise `/Contents`, in `/DS`.

## 2. A changed value regenerates the whole appearance

§12.7.4.3: "For these fields, the following conventions are not used, and the entire annotation
appearance shall be regenerated each time the value is changed." The splice into `/Tx BMC` … `EMC` is
one of those conventions, so for a rich text field `regenerate` builds the whole appearance again —
Table 192's background and border, then the text — in the stored stream's own `/BBox`, which §12.5.5
places with the stored `/Matrix` as it placed the old stream. Table 192's `/R` is left to that
`/Matrix`, which is where a producer states the turn. The same path serves the drawing after an edit,
a reset or an import, `/NeedAppearances`, and the stream a save writes. A plain field keeps its splice.

## 3. What a value this program set is, and what the save writes

A person typing sets plain characters, and no interface here states formatting, so the new value is
those characters in the field's `/DS` — a choice, and the one the entry's name describes. The save
writes `/V` as the characters (§12.7.5.3's "field's text"), and `/RV` beside it on the same dictionary
as bit 26 requires: one `p` per line, a run of spaces kept by `xfa-spacerun:yes`, the three markup
characters escaped, XHTML's and XFA's namespaces on the `body`, `xfa:spec` naming 3.3, and no
`xfa:APIVersion`, which *Version Specification* reads as the latest revision. A cleared value removes
`/RV`: bit 26's `shall` is conditioned on the field having a value. The string reads back as the value
(`the_rv_a_save_writes_reads_back_as_the_value`).

## 4. What this costs

A rich value a person edits loses the runs' own formatting and keeps the default style: what was bold
is not bold after a keystroke. The alternative — carrying the old runs over new characters — would
invent where each old style begins and ends in a string it never described. An editor of formatted
text is the build that removes the cost.

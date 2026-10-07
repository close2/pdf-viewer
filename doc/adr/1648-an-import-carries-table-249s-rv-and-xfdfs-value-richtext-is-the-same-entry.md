# 1648 — An import carries Table 249's `/RV`, and XFDF's `<value-richtext>` is the same entry

Session 1406. Status: **accepted**. Closes what ADR 1635 left for §12.7.8.3.2; amends ADR 1297's
reading of XFDF, whose field half named `<value-richtext>` and applied nothing of it; none is edited.
Context: `crates/pdf-model/src/forms_data.rs` (`FdfField::rich_value`, `Import::rich_value`,
`read_field`), `crates/pdf-model/src/xfdf.rs` (`Reader::rich_value`), `crates/pdf-model/src/view.rs`
(`FieldValue::Imported`'s `rich`), `crates/pdf-model/src/appearance.rs` (`Field`, `RichSource`),
`crates/pdf-model/src/rich_text.rs` (`Stated`, `for_field`, `characters`). ISO 32000-2 §12.7.8.3.2
(Table 249), §12.7.5.3 (Table 231 bit 26), Table 228; XFDF 3.0 (`doc/XFDF_Spec_3.0.pdf`), *The value and
value-richtext elements in fields* (page 31) and *value-richtext* (page 36), cited and never quoted.

## 1. What crosses, and what it replaces

§12.7.8.3.2's one load-bearing sentence is "importing a field causes the values of the entries in the
FDF field dictionary to replace those of the corresponding entries in the field with the same fully
qualified name in the target document", and Table 249's `/RV` is "[a] rich text string, as in Adobe
XML Architecture, XML Forms Architecture (XFA) Specification, version 3.3" — Table 228's entry on the
target field is the corresponding one. A rich text string is a text string: it names no object of the
FDF file, so it crosses whole, the way ADR 1186 let `/IF` cross. `read_field` decodes it as `/V` is
decoded — §7.9.2.2's text string or §7.9.3's text stream — and a string in a registered character set
this program has no table for is refused at the entry and named, as the value is.

The imported `/RV` reaches the layout beside the value it describes (`FieldValue::Imported`'s `rich`),
and **it is drawn by the rule a stored `/RV` is drawn by** (ADR 1635): where its characters are the
imported value's, in its formatting; where they differ, the value in the field's `/DS`, with
`Owed::RichTextDisagrees` saying so. An imported value is therefore no longer treated as a value no
`/RV` describes: it is the file's statement about the field, as `/V` is.

## 2. An FDF field that states no `/RV`

The replacing sentence is about the entries the FDF field *states*. One stating `/V` and no `/RV`
replaces the value and leaves the target's own `/RV` standing — which then describes the old value,
so the comparison above draws the imported characters in the default style and says why. The other
reading — an import clears every entry it does not state — would make the sentence's "corresponding
entries" mean all of Table 228, and nothing in the clause says that. An FDF field stating `/RV` and no
`/V` remains a field whose value is removed, which is how this tree already reads a missing `/V`
(ADR 0090); Table 231 bit 26 conditions the `/RV` on the field having a value.

## 3. XFDF's element

ISO 19444-1's preview stops before section 6.3.4, so the field half reads `<value-richtext>` from the
text ISO 19444-1 was made from, as ADR 1297 read `<annots>`. XFDF 3.0 says the element is the field's
value formatted as a rich text string and corresponds to the variable text field's `/RV`, and that
plain text inside it maps to the `/V` entry. So markup inside the element becomes Table 249's `/RV`,
taken whole from the file as `<contents-richtext>` is; plain characters become the value. **Where a
file states the element and no `<value>` beside it** — the shape the same section says one exporter
writes when a field has both entries — the value is the string's characters, as `rich_text` reads them
(paragraphs as carriage returns, white space as chapter 27 compresses it). That is what the
element holding the field's value means, and it is what lets §12.7.5.3's `/V` hold the field's text at all; a
`<value>` stated beside the element stands, and §1's comparison decides what is drawn.

## 4. What this does not do

`ViewState::save` writes the values a person entered, and an imported value is not among them; the
imported `/RV` follows the imported `/V` there rather than leading it, so neither reaches the file.

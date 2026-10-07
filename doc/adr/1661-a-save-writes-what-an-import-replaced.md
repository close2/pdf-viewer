# 1661 — A save writes what an import replaced

Session 1412. Status: **accepted**. Amends ADR 1648 section 4, which kept both entries out of the file, and the
sentence `ViewState::import` carried about §12.7.6.3's reset; none is edited.
Context: `crates/pdf-model/src/view.rs` (`ViewState::save`, `ViewState::write_imported_values`,
`ViewState::import`); `crates/pdf-model/tests/rich_text.rs`
(`a_save_writes_the_imported_value_and_its_rich_text_string`). ISO 32000-2 §12.7.8.3.2 (Table 249),
§12.7.2, §12.7.4.1, §12.7.4.2, §12.7.5.3 (Table 231 bits 14 and 26), §7.5.6; ADRs 0090, 0120, 1223,
1635, 1648.

## 1. Why the value is written

§12.7.8.3.2: "importing a field causes the values of the entries in the FDF field dictionary to
replace those of the corresponding entries in the field with the same fully qualified name in the
target document". ADR 1223 already wrote one of those entries — Table 249's `/AP` — into the save,
on `CLAUDE.md`'s provenance test: the marks are the FDF producer's, carried unreinterpreted. The
value is the same producer's statement under the same sentence, and a save that wrote the imported
appearance beside the target's old `/V` wrote a widget whose value and appearance §12.7.2 requires
to agree and which did not. A save that wrote neither left the reader with a file that draws what
the screen did not show. So the import reaches §7.5.6's update with the edits.

## 2. What is written, and where

- Table 249's `/V`, on the dictionary `holder` finds — the one a typed value goes on — as the name
  a toggling button's value is, with `/AS` following; an FDF field stating no `/V` removes it
  (ADR 0090).
- Table 249's `/RV` beside it where the FDF field states one; where it states none the target's own
  stands (ADR 1648 section 2).
- `/Ff` as the import changed it, on the field §12.7.4.2 names, which is Table 249's "the form's
  corresponding field dictionary"; `/F` on the widget.
- Table 234's `/I` is removed, as for a typed value: an index beside a value it does not describe
  is the file contradicting itself.
- The appearance constructed from the imported value, unless the import carried its own `/AP`,
  which ADR 1223 writes and which stands.

A field a person typed into after the import is written by the edits and not here — ADR 0120's
latest statement — and a password field's imported value is withheld by the predicate a typed one
is (Table 231 bit 14's NOTE, ADR 0247).

## 3. What this does not do

A reset (§12.7.6.3) is still a viewer's state and is not written; it is the same shape, and the
next build of this path. `/IF`, `/Opt`, `/A` and `/AA` from an import are drawn or acted on and not
written; each is an entry Table 249 names, and each is a later build of this function.

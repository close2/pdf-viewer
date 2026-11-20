# 3312 — A field is exported by its export value, in every format and in XFA data

Session 2152. Status: accepted. Context: audit 2148-B01, B02 and its closing note on `xfa_change`;
XFA 3.3 Template Reference, `items`; Chapter 14, "Data Associated with Choice List Selections".

## Decision

Two clauses give a value an export form apart from `/V`: §12.7.5.4's two-element `/Opt` entry, whose
first element is "a text string representing the export value that shall be used when exporting
interactive form field data from the document", and Table 230's `/Opt` for buttons, "an array of text
strings representing the export value of each annotation in the field". `submission::choice_export`
and `submission::button_export` are the one mapping, applied by `submission::exported` to every entry a
submission carries — HTML Form, FDF and XFDF alike — and by `view/writing.rs`'s `xfa_change`.

- A choice's label maps to the option Table 234's `/I` names where its label is the value's (two
  options sharing a label export their own values), else to the first option with that label; text no
  option carries is an editable combo box's and exports itself.
- A button that is on exports the `/Opt` element at its widget's position among the field's widgets;
  for a submission, the widget the view shows on.

**XFA data takes the export value.** XFA 3.3's `items` element: of two lists, the one with `save="1"`
holds the values "entered into the field", and "the value in the first column with it set is saved";
Chapter 14's example stores `0` for the displayed `apples`. XFA 3.3 names no correspondence with a
two-element `/Opt` entry, so reading its export value as the saved column is this program's reading of
a hybrid form, and it matches what `xfa/datasets.rs` already documented ("a choice list's is the
export value").

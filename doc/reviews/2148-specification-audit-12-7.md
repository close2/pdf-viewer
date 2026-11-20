# Specification audit of ISO 32000-2 §12.7 (interactive forms) — round 2148

The project's fifth specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128,
2131, 2136 and 2142 ([`2142-specification-audit-7-4.md`](2142-specification-audit-7-4.md)).
Baseline: main at `dd12e73b`. Evidence: [`2148-specification-audit-12-7/`](2148-specification-audit-12-7/)
— `findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing** but one documentation slip (D01). A fix round takes the
findings by the groups at the end.

## Method

1. Every `shall` and `should` of §12.7.1 to §12.7.7 (Tables 224 to 243) was read in
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 12393 to 12925; §12.7.8's FDF was read only where
   a submission writes one, because 2038's findings (B75, D07, T15, T59) and its fixes cover the
   import side.
2. 2038's §12.7 findings (B13, B26, B58, B60–B67, B69, B75 and the D and T rows beside them) were
   read first and are not repeated: the inheritance of `/TM`, `/AA`, `/DS`, `/TI`, `/I` and a
   choice `/Opt`, the empty `/Fields` array, the live `charLimit`, the reset of a shared ancestor.
3. The code was traced through `pdf-model`: `inheritance.rs` (`READ_UP_THE_CHAIN`, `Inherited`),
   `appearance.rs`'s field section (`Field::read`, `text_shape`, `is_on`, `replacement_state`,
   `an_earlier_button_answers_to`, `regenerates`), `form.rs` (`fields`, `selected`, `Choice`),
   `view/field_tree.rs` (`walk`, `qualified_name`, `ancestry`, `widgets_under`), `view/fields.rs`
   (`set_field`, `chosen`, `held_to_max_len`), `view/actions.rs` (`reached_by_reset`),
   `view/writing.rs` (`save`, `holder`, `write_state`, `xfa_change`) and `submission.rs`
   (`selected_names`, `chosen`, `html_body`, `urlencoded`, `text_bytes`); and `viewer-host`'s
   `toggling` and `clicked`, which decide what a host sends for a click on a button.
4. The lens was the four habits of the earlier audits — what binds to what; a rule keyed on a name
   applies wherever the name is read; a measure has an origin on the way out as on the way in; a
   parameter has a type as well as a place — turned to the place §12.7 is where the reader
   *writes*: what a value is exported as, which widget a value turns on, what a save writes, and
   which values a field type may take.
5. Each B finding has a failing `*_spec_audit` test in
   `crates/pdf-model/tests/forms_values_spec_audit.rs` (a module of `forms_suite`, ADR 3086), built
   from bytes the test writes, with a control that passes on main; each failure was read and is
   the finding's own assertion.

What holds is not listed one by one. Among it: the inheritable set and its terminal-field limit;
`/V` written on the terminal field and not on a nonterminal ancestor (`holder`); `/MaxLen` cut in
characters on every programmatic path A834 names; `/AS` over `/V` for a stored state and the value
over a stale `/AS` once the reader replaced it; a choice's `/V` written as labels with `/I` for a
repeated label, and `/I` yielding to `/V`; Table 241/242's reference and name forms with their
descendants and the Exclude flag; Table 240's NoExport precedence and `IncludeNoValueFields`; the
`/Tx BMC` splice with `/DR` resources added under the existing names; `/NeedAppearances` written
only for what the save could not construct; the password value withheld from the save.

## Findings

Eight behaviour gaps and one documentation slip. In order of how much a person would see:

- **2148-B03** — a saved radio set can state two buttons on. With `RadiosInUnison` clear and two
  buttons sharing an on-state name, the view turns the first on and the save's `write_state`
  writes `/AS /Yes` on both, so the reopened file draws both. A value written back that this
  reader reads differently.
- **2148-B04** — the second of two such buttons cannot be selected. A host sends a button's
  `on_state` to `set_field`, which takes no widget, and the first goes on; `viewer_host::clicked`
  promises the one under the pointer. The first-kid rule `radios_in_unison.rs` documents is sound
  for a file's `/V`, which names no widget, and not for a person's click, which does.
- **2148-B01** — a submission carries a choice field's label, not its export value. §12.7.5.4
  makes `/V` the second element of a two-element `/Opt` entry and the first element the value
  "that shall be used when exporting interactive form field data". `Choice::export` is
  documented as what §12.7.6.2 exports, and nothing reads it.
- **2148-B02** — a check box or radio button with Table 230's `/Opt` is submitted by its state
  name (`/0`), not its export value; `xfa_change` already makes this mapping for XFA data.
- **2148-B08** — a root field named by the empty string merges its child `a` with a top-level
  field `a`: `qualified_name` drops the PERIOD after an empty prefix, though its own comment holds
  that an empty `/T` names a field.
- **2148-B05** — typed text is taken as the value of a list box or a non-editable combo box.
- **2148-B06** — a push-button takes a value and the save writes it as `/V`, which §12.7.5.2.2
  forbids.
- **2148-B07** — a Comb field with FileSelect set is laid out and described as a comb; Table 231
  bit 25 names three flags and `text_shape` reads two.
- **2148-D01** (fixed) — `inheritance.rs` named Tables 229 and 231 for `/MaxLen`, `/TI` and `/I`
  (they are Tables 232 and 234).

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each:
`12.7.4.2` (B08), `12.7.5.2.2` (B06), `12.7.5.2.3` (B02, B03), `12.7.5.2.4` (B02, B03, B04),
`12.7.5.3` (B07), `12.7.5.4` (B01, B05), `12.7.6.2` (B01, B02).

## Suggested fix groups

1. **`submission.rs`** — B01 and B02: one export-value mapping (a choice's label to its `/Opt`
   export, by `/I` where labels repeat; a button's state to `/Opt[index in /Kids]`), shared with
   `writing.rs`'s `xfa_change`, applied in every format (HTML Form, FDF, XFDF).
2. **`view/writing.rs` with `appearance.rs`** — B03: `write_state` writes the `/AS` the view's
   `Field::is_on` reads, not the `/AP` key test alone.
3. **`view/fields.rs` and `view.rs` (`Entered`), with the hosts** — B04, B05, B06: an edit that can
   name the widget a person selected, and `set_field` refusing a value a field type cannot take
   (typed text for a choice without Combo and Edit; any value for a push-button or a signature
   field). B04 touches `viewer-host`'s `clicked`/`toggling` and the three hosts that send it.
4. **`appearance.rs`** — B07, one flag in `text_shape`.
5. **`view/field_tree.rs`** — B08, with any other name builder that strips an empty prefix.

One observation outside §12.7 for a later round: `writing.rs`'s `xfa_change` hands a choice's
labels to XFA data where `xfa/datasets.rs` documents the export value (Annex K), which wants a
reading against XFA 3.3 before it is called a finding.

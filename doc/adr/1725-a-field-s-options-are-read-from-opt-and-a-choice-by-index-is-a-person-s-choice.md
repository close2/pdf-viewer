# 1725 — A field's options are read from `/Opt`, and a choice by index is a person's choice

Status: accepted and **built**. Session 1444. Builds on ADR 1700's rule that a script writes what a
reader's edit reaches, and ADR 1664's widget order; RFC 0008 section 4.2's `Field` row.
Code: `crates/pdf-script/src/engine/choices.rs` (the members), `crates/pdf-model/src/view/script_model.rs`
(`FieldState::options` and `selected`, read through `form::options` and `form::selected`;
`ScriptEdit::Choose`), `crates/pdf-model/src/view/scripts.rs` (`choose`, through `view::chosen`),
`crates/pdf-script/src/surface.rs` (`REFUSED`'s `Field` row), `crates/pdf-script/src/wire.rs`
(version 10, with ADR 1724), `crates/pdf-script-worker/examples/wire_seeds.rs` (the new shapes).
Tests: `crates/pdf-script/tests/pages_and_choices.rs` (the items, a multiple selection, a value off
the list, a choice by index, Table 230's export values and `setItems` refused),
`crates/pdf-script/tests/wire.rs`.

## 1. The census

ADR 1724 section 1's count: `currentValueIndices` in 9 documents, `numItems` 6, `getItemAt` 6
(as a method on a result), `setItems` 2, `insertItemAt` 2, `deleteItemAt` 1, `clearItems` 1,
`exportValues` none.

## 2. What is read, and from where

Adobe's *JavaScript for Acrobat API Reference* at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`,
"Field properties" and "Field methods", is cited and never quoted; each answer is a documented
choice under principle 5.

- **A field's realm state carries its inherited `/Opt`** in both of Table 234's forms, each entry an
  export value where it is a pair and the text shown, and the selected items' indices read as
  §12.7.5.4 reads `/V` and `/I` — the reading the appearance and the form's controls already make
  (`form::options`, `form::selected`), so a script and the page agree. At most 32 768 entries.
- **`numItems`** counts the entries; **`getItemAt(nIdx, bExportValue)`** answers an entry's export
  value where `bExportValue` is true (the default) and the entry is a pair, its text otherwise, with
  `-1` the last entry as the reference has it; an index naming no entry is a `RangeError`. Both are
  a combo box's and a list box's, and on any other field are refused by name.
- **`currentValueIndices`** answers one selected index as a number, several as an ascending array,
  and `-1` where nothing is selected or the value is no entry's text — an editable combo box's own.
- **`exportValues`** is a check box's or a radio button's: each widget's Table 230 entry where the
  field states `/Opt` — "one entry for each widget annotation in the Kids array" — and otherwise the
  name its on state is selected by (ADR 1689), empty for a widget naming no single on state.

## 3. What is written

- **`currentValueIndices = n` or `[n, …]` is a person's choice**: `ScriptEdit::Choose`, applied
  through the very function a host's `Entered::Chosen` goes through (`view::chosen`), so the value is
  written as `/V` naming the items and `/I` listing them, and the save writes both. Several indices
  on a field whose Table 233 `MultiSelect` flag is clear are cut to the first, as a person's are; an
  index naming no entry chooses nothing and is a `RangeError`.
- **`setItems`, `insertItemAt`, `deleteItemAt`, `clearItems` and a write of `exportValues` are
  refused by name** (`surface::REFUSED`): each rewrites `/Opt`, the field's own list of options,
  which no reader's edit changes — a person chooses among the options and never rewrites them. This
  is ADR 1700's line for an annotation's properties drawn for a field's, and it departs from RFC
  0008 section 4.2, which admits "the choice methods"; bridging them needs an `/Opt` override that
  the appearance, the form's controls and the save all read, which is its own round's argument.

## 4. Unfinished

- The four rewriting methods, above: at most six census documents use one (the counts overlap).
- `/Opt` past 32 768 entries is cut silently at the realm; no corpus field comes near it.

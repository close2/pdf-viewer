# 1737 — A script's rewrite of `/Opt` is one property the page, the control and the save read

Status: accepted and **built**. Session 1450. Supersedes ADR 1725's refusal of `setItems`,
`insertItemAt`, `deleteItemAt` and `clearItems` (`surface::REFUSED`'s last row).
Code: `crates/pdf-model/src/view/script_model.rs` (`Property::Options`),
`crates/pdf-model/src/view/scripts.rs` (the drawn arm, `choose`), `crates/pdf-model/src/appearance.rs`
(`scripted_entries`'s `/Opt`, `options_entry`, `with_rewritten_options`), `crates/pdf-model/src/view.rs`
(`chosen`, named hunk), `crates/pdf-model/src/form.rs` (`read`, named hunk),
`crates/pdf-script/src/engine/choices.rs` (the four methods, `rewrite`), `crates/pdf-script/src/wire.rs`.
Tests: `crates/pdf-script/tests/pages_and_choices.rs`
(`set_items_rewrites_opt_for_the_realm_the_control_a_choice_and_the_save`,
`inserting_and_deleting_keep_the_selection_with_its_item_until_it_is_deleted`), `tests/wire.rs`.

## 1. Why an override and not a refusal

ADR 1725 refused the four because "no reader's edit changes" `/Opt`. RFC 0008 section 4.2 admits
them; ADR 1725's census counts `setItems` in 2 documents, `insertItemAt` 2, `deleteItemAt` 1 and
`clearItems` 1, at most six with the overlaps; and the tree already has the shape a rewrite needs: a script's property is kept per widget beside the edit log (`scripting.drawn`), drawn by
reading the widget as the entries it writes (`appearance::with_scripted`, ADR 1617) and saved as
those entries on the field §12.7.4.2 names (`write_scripted`). `/Opt` is one more such entry, so
the rewrite is `Property::Options` — the whole list as the script left it, field-level, the latest
replacing the earlier — and every reader of the options reads it the one way:

- **the page**: a list box's construction reads `/Opt` off the scripted widget;
- **a host's control** (`form::fields`), **a person's choice by index** (`view::chosen`) and **the
  realm's next telling** each read the widget through `with_rewritten_options`, so an index names
  an item of the list the page draws;
- **the save** writes Table 234's array on the named field: a text string, or "an array
  consisting of two text strings: the option's export value and the text that shall be displayed".

## 2. The members, documented choices under principle 5

Adobe's "Field methods" at `ab3b42a7`, cited and never quoted. `setItems` takes an element that
converts to a string as an option whose text and export value are that string, and a two-element
array as text then export value — the reverse of Table 234's pair, turned when written; an export
value equal to its text is written as Table 234's plain string. `insertItemAt(cName, cExport,
nIdx)` inserts at the top for 0 or no index and at the end for -1; another place outside the list
is a `RangeError`. `deleteItemAt(nIdx)` deletes the item named, or with no index the first item
selected; with no index and nothing selected nothing is deleted, noted. `clearItems` deletes all.
A list past `MAX_PAGES` items is a `RangeError`. A field that is not a combo box or a list box is
refused by name, as ADR 1725's members are.

**The selection follows §12.7.5.4's `/V`**, which names an item by its text: an item a rewrite
moves stays selected where it now stands, and one a rewrite takes away leaves the field with no
selection, as the reference says of `deleteItemAt` — carried as an empty `ScriptEdit::Choose`, a
person's empty choice. A stale `/I` is harmless by the table's own rule: "[i]f the items
identified by this entry differ from those in the V entry …, the V entry shall be used".

## 3. What it costs

A rewritten list is held whole for each of the field's widgets until the document closes — at
most 32 768 options a field, the bound the realm already reads under. The save writes `/Opt` on
the field once per widget it touches, the same entry each time.

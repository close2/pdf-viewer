# 1603 — `/V` and `/C` run at the commit, `event.rc` is honoured, and a script's write to a field is an edit

Status: accepted and **built**. Session 1383. Builds on RFC 0008 sections 4.2, 6.4, 6.5 and 11 item
3 (b), accepted by the owner in `doc/questions/A193`; extends ADRs 1579 and 1591; ADR 1602 is the
realm every run here happens in.
Context: ISO 32000-2 §12.6.3 (Table 199), §12.7.3 (Table 224's `/CO`), §12.7.4.3, §12.7.6.3,
§12.6.4.11; `CLAUDE.md` principle 1's immutable `Document`.
Code: `crates/pdf-model/src/view/scripts.rs` (`commit_field`, `walk_calculations`,
`refresh_formatted`, `apply_edits`), `view/script_model.rs`, `view.rs` (the `scripting` field,
`set_field`'s read-only, the reset's walk, the save's display), `appearance.rs` (`Format`),
`crates/pdf-script/src/engine/bridge.rs`, `src/surface.rs`. Tests: `crates/pdf-script/tests/hook.rs`,
`realm.rs`, `fixtures.rs`.

## 1. The commit, in the reference's order

`ViewState::commit_field` runs `/K`'s commit form, then `/V`, then every `/CO` entry, then `/F` on
every field a runner formats — Adobe's "Form event processing", which RFC 0008 section 6.5 adopted.
Where a field's script is one `AF*` call Tier 0 runs it as before (ADR 1579); where it is any other
script and a runner is supplied, the runner runs it in the document's realm.

- **`/V`**: `event.value` the committed text; `event.rc` false leaves the field as it was before the
  typing began and the answer says so (`Committed::Refused`, "the field's validate script refused
  the value"). The reference says both that the event does not listen to `rc` and that false
  invalidates; the second is the sentence with an effect, RFC 0008 section 6.5's choice. A value the
  script writes into `event.value` is not taken: the event validates a value.
- **`/C`**: Table 199's "recalculate the value of this field when that of another field changes",
  in Table 224's order, once. `event.value` is the field's value, `event.source` the field whose
  commit started the walk; the script's `event.value` is the value taken, and `rc` false leaves the
  field's value. `AFSimple_Calculate` called from a script reads the realm's table, so a sum written
  as one call and a sum written inside a script agree. **A script's calculation runs at the commit
  alone**, where the reference puts it — a script may do more than compute — while a one-call `/C`
  still runs after every keystroke as ADR 1579 has it; a reset (§12.7.6.3, or a script's
  `resetForm`) and `calculateNow` walk the order with scripts too.
- **A runaway is named by its place.** Every sentence a calculation's run owes is reported as
  *Total (entry 2 of 3 in the calculation order): …*, so a stop names where in the chain it was;
  the walk goes on past a stopped entry, each entry under its own budget, and the walk as a whole is
  held to one second (`MAX_SEQUENCE_TIME`), past which the remaining entries are named as not
  recalculated.
- **`/F`**: after the walk, every field whose `/F` a runner runs and whose value changed since its
  display was made is formatted, and what it displayed is kept beside the edit log
  (`view::Displayed`).

## 2. The drawn appearance asks the runner

`AnnotationView::displayed` carries what the runner's format displayed for a widget's value, and
`appearance`'s format step draws it where Tier 0 does not run the format itself and the value is the
one it was made from; a value that has moved on since is drawn as it stands and reported as before.
The save draws through the same view, so a saved file shows what the viewer showed. The display is
made at a commit, after the open sequence and the sites of ADR 1602, and after a reset.

## 3. What a script reaches on a field, and how a write lands

The bridge carries `Field`'s `value`, `valueAsString`, `name`, `type`, `display`, `hidden`,
`readonly`, `required`, `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment`,
`charLimit`, reads `multiline`, `password`, `comb` and `doNotScroll`, and answers `page`, `rect`
and `doc`; `this.getField` of any field the realm holds, of a name with fields below it (one
`Field` for the subtree, as the reference lets a script hide a group) or `null`;
`getNthFieldName` in the sorted order of §12.7.4.2's names, which the reference leaves open;
`numFields`, `numPages`, `pageNum`; `calculateNow`; `resetForm`; and the reference's `display`,
`border` and `color` constants. `value` reads as a number where a text or combo-box field's text is
one, which is the reference's behaviour and why its library reads through `AFMakeNumber`. Every
other admitted member stays a `NotAllowedError` by name (`surface.rs`). Each spelling is a
documented choice read from the reference.

**A write is an edit a person could have made** (RFC 0008 section 6.4): it changes the realm's table
and lands in the outcome as a `ScriptEdit`, which the view state applies — a value into the edit log
beside a typed one, never into a field a person is typing into; `display` and `hidden` into the
hide sets beside §12.6.4.11's; `readonly` where `set_field` refuses a user; `resetForm` as
§12.7.6.3's reset. `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `alignment`, `charLimit`
and `required` are kept by field name (`ViewState::script_properties`), read back by the realm, and
**not drawn**: each is reported once, *Total: a script set Field.textColor; this view state keeps it
and the drawn appearance does not carry it*. A keystroke, format or validate script changes its own
field through `event.value` only. A run that does not finish hands back no edit and leaves the
realm's table as it was. A save writes values; a script's properties are the session's, as a hide
action's are.

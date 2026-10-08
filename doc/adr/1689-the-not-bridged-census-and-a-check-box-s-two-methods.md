# 1689 — The not-bridged census, a check box's two methods, and the tail ranked

Status: accepted and **built**. Session 1426. Builds on ADR 1625's Tier 1 column, ADR 1664's widget
addressing and A193's rule that the census ranks the work and never caps it.
Code: `crates/pdf-script/tests/script_corpus.rs` (`not_bridged`, the `Refused` tally),
`crates/pdf-script/src/engine/members.rs` (`is_box_checked`, `check_this_box`, `widget_argument`),
`crates/pdf-script/src/surface.rs` (`NOT_BRIDGED`, two members fewer), `crates/pdf-model/src/view/script_model.rs`
(`WidgetState::on_state`), `crates/pdf-model/src/appearance.rs` (`on_state`, now crate-visible),
`crates/pdf-script/src/wire.rs` (the on state crosses; version 7, shared with ADR 1688).
Tests: `crates/pdf-script/tests/realm.rs`
(`a_box_is_checked_where_the_value_names_its_on_state_and_checking_it_sets_that_value`),
`crates/pdf-model/tests/script_properties.rs`
(`a_check_box_s_on_state_is_told_and_checking_it_saves_that_state`), `crates/pdf-script/tests/wire.rs`.

## 1. The instrument

The two `NotBridged` sites of `bridge.rs` were `setFocus`'s refusal (gone, ADR 1688) and
`refuser`, which every member of `surface::NOT_BRIDGED` reaches. The Tier 1 column now prints one
line per listed member — runs, documents and the first document — with a zero printed, a count of
how many listed members a run reached, and any `NotBridged` refusal of a member outside the list.
The population is the list itself (trap 25), and the column already named `app.setInterval` from a
real run, so a reached member is seen to be named (trap 13).

## 2. What it found, and what it cannot see

Of 52 listed members, runs reached one: `app.setInterval`, 2 runs in 2 documents
(`GHOSTSCRIPT-688384-0.zip-2.pdf` and `-3.pdf`, one form's clock, `app.setInterval("pdshowtime()",
1000)` in a `/F`). The column walks the open sequence, each field's `/K` `/F` `/V` `/C` and Table
200's five, so a member called only from Table 197's events — a button's `/U` — is never reached;
and 9 177 of its 9 317 throws are `ReferenceError`s that stop a script before any later member. So
the static census was read beside it (`examples/javascript_census.rs` over the same 90 763 files,
929 scripted): `this.getAnnots` in 34 documents, `app.beep` in 24, `checkThisBox` in 20 (12 as a
method on a result, 8 bare), `isBoxChecked` in 14. No other listed member is on its printed lists,
whose floors are five documents for a property and two for an unknown name.

## 3. Bridged: `isBoxChecked` and `checkThisBox`

Adobe's reference ("Field methods", commit `ab3b42a7`) gives both a widget index from zero, which
here is the field table's (ADR 1664). Each is a documented choice under principle 5:

- **A toggling widget's on state crosses to the realm** as `WidgetState::on_state`: the name
  §12.7.5.2.3's on state is selected by, read by the same rule the appearance uses
  (`appearance::on_state` — the one non-`Off` state of `/AP /N`, `None` where the file states two).
- **`isBoxChecked(n)` is whether the field's value is widget n's on state.** Widgets sharing an on
  state are checked together, the reference's own note on radio buttons. A widget the field does
  not have, or one with no on state, is not checked.
- **`checkThisBox(n, bCheckIt)` sets the value as `field.value` does**: widget n's on state, or
  `Off` to uncheck a check box whose widget n is the one on. The reference lets no radio button be
  unchecked this way, so `false` on one changes nothing. A field that does not toggle, or a widget
  with no on state to set, is refused by name; a negative index is a `RangeError`.
- The value is written as a person's click writes it: the view state's toggling-button rule makes
  the text a name, and the save writes `/V` and `/AS` both naming the state.

## 4. The tail, ranked by documents

1. `this.getAnnots` (34): an `Annotation` object of its own (Adobe's "Annotation" page), whose
   properties are a model the realm does not hold; most uses follow Adobe's `syncAnnotScan()`.
2. `app.beep` (24): a sound is a host's act, so it is an event across the wire that a face plays,
   and the faces are another slot's this batch.
3. `app.setInterval`, `setTimeOut`, `clearInterval`, `clearTimeOut` (2 reached): a timer needs a
   clock that runs while a person reads, and `viewer_core::Command::Tick` is sent only by a host
   showing a presentation. Built, it is a request the view state holds, a tick every host sends
   while a timer is due, and the timer's script run as a later run; A193 (10) admits it bounded to
   the document.
4. Every other listed member: no document on the census's printed lists. A193 makes each owed all
   the same; the order above is the order a round takes them in.

# 1626 — The census's remaining members are carried, and a script's depth is bounded before it is parsed

Status: accepted and **built** in `pdf-script` and `pdf-model`. Session 1395. Builds on RFC 0008
section 4.2 (accepted, `doc/questions/A193`), ADR 1602 (one realm per document), ADR 1603 (a
script's write is an edit), ADR 1617 (a drawn property is the standard's entry). `app.alert` and
`app.response` are ADR 1627's.
Context: ISO 32000-2 Table 99 (`/Locked`), §8.11.2.3, Table 192 (`/CA`, `/AC`, `/RC`), Table 232
(`/MaxLen`), Table 231 (`DoNotScroll`), Table 349; Adobe's *JavaScript for Acrobat API Reference*
at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`, its "event properties", "Doc properties", "Doc
methods", "OCG", "util methods", "Field methods" and "global" pages, cited by name, never quoted.
Code: `crates/pdf-script/src/engine/members.rs`, `depth.rs`, `engine/mod.rs` (`too_deep`, the
optimizer, `ensure_can_compile_strings`), `engine/bridge.rs` (`begin`'s three event properties),
`surface.rs`, `wire.rs` (version 4); `crates/pdf-model/src/aform/printf.rs`,
`view/script_model.rs` (`CommitKey`, `Face`, `DocumentState`, `Property::Caption`,
`ScriptEdit::Layer`), `view/scripts.rs` (`commit_field_by`, `Typed`, `mark_saved`, `switch_layer`),
`appearance.rs` (`scripted_entries`' caption arm).
Tests: `crates/pdf-script/tests/census_members.rs`, `view_members.rs`; `src/depth.rs`'s and
`aform/printf.rs`'s units.

## 1. The members, each the reference's meaning or the standard's entry

- **`global`** is an ordinary object per realm: what a document stores lives as long as its realm
  and reaches no other document. `setPersistent` and `subscribe` are RFC 0008 section 4.3's, a
  channel between files, refused by name.
- **`event.commitKey`, `fieldFull`, `changeEx`** are read-only, as the reference has them.
  `commitKey` is 1–3 at a commit's keystroke, validation and its field's format, 0 elsewhere;
  `ViewState::commit_field_by` takes the key, and `commit_field` is it with Enter, the documented
  choice for a host that does not say. `fieldFull` is true where a text field's whole value passes
  Table 232's `/MaxLen` or the room Table 231's `DoNotScroll` leaves (the prefix `set_field`
  accepts); then `change` is what fits, `changeEx` all that was typed, and the field takes `change`.
  A choice field's `changeEx` is the export value of the `/Opt` pair its change shows; a text field
  that is not full has `changeEx` equal to `change`, the page defining it only for a full one.
- **`this.dirty`** is the view state's unsaved work since `ViewState::mark_saved` (the host calls it
  after a save), or since the open. A script's write is read back for the rest of its run and noted,
  and moves nothing else: **a document does not declare its reader's typing saved**, since a host
  that believed it could lose that typing at a close.
- **`this.info`** is the trailer's `/Info`: every text-string entry under its key, the reference's
  nine standard keys under their lower-case spelling as well, the two dates `Date`s where §7.9.4
  parses them, `/Trapped`'s name as text. Read-only, as in a reader: a write is refused by name and
  the object is sealed.
- **`this.getOCGs()`** answers Table 98's groups a person's switch can change, by name (the
  reference's order without a page); with a page it is refused, since a page's groups are a walk of
  its resources the realm is not handed. `OCG.state = …` is `ScriptEdit::Layer`, made through
  `ViewState::set_group` — **a script flips only the switch the person reading could flip**: Table
  99 permits more ("such as ECMAScript") and this program declines it, so a locked group stays and
  the run says why. `name`, `initState` and `locked` read; their writes are refused.
- **`util.printf`** is `pdf_model::aform::printf`, on the library's own `digits_with` and
  separator styles, so a script and `AFNumber_Format` write the same digits. Its choices are in the
  module: six places for a bare `%f`, fifteen significant digits for `%s` of a number (the page's
  own example), 32-bit two's complement for a negative `%x`, `%%`, and a count of arguments that
  differs from the conversions refused.
- **`buttonGetCaption` and `buttonSetCaption`** read and write Table 192's `/CA`, `/AC` and `/RC`
  (`nFace` 0, 1, 2) as `Property::Caption`, drawn and saved on ADR 1617's route; `/CA` is any
  button's and the other two a push-button's, and anything else is refused by name.

## 2. A script's depth is bounded before it is parsed

Boa 0.22's parser and bytecompiler recurse per level of the expression tree and state no limit.
Measured (habit 71) on an optimised build, threads of 2, 4 and 8 MiB, the stack per level is 22.8–
25.6 KiB per bracket, 51.8 KiB per nested function, 3.4 KiB per prefix operator, 4.4 KiB per arrow,
2.7 KiB per `if`, `else if` and `?:`, 1.5 KiB per assignment or `**`, 0.75–0.81 KiB per binary
operator, call or member access; the `1+1+…` chain the brief named overflows 8 MiB at 11 136 terms.
`pdf_script::depth::estimate` gives each token the dearest of these it can begin and sums them along
the path from the start of its statement through every open bracket, the sum returning to its base
where the grammar makes things siblings — a `;`, a line break between operands, a comma between
elements, properties or arguments. `Budget::depth` is 4 MiB, half the worker's thread; the deepest
script of the census population estimates 670 KiB (`PDFBOX-4323-0.pdf`), and no run of the column
exceeds it. A string compiled at run time — `eval`, `Function` — meets the same two bounds through
Boa's `ensure_can_compile_strings` hook, which the bracket count alone had never reached. The scan's
one reading of its own is a slash after `yield`, `await` or `let` used as a name; a script built on
that is the worker's process to contain, as every script was before (ADR 1609).

## 3. Boa's optimizer is off

Re-measured here on the largest library (`evince-LINK-46-2.pdf`'s 167 433 bytes), fifteen runs
each: parse 8.71 ms (median 9.04) with the optimizer, 2.91 ms (2.94) without; evaluation 1.33 ms
against 1.12 ms. A 2 500-term chain of constants: 0.77 + 0.00 ms with it, 0.65 + 1.84 ms without.
Round 1392 had the optimizer at 171.6 M of the library's 242.5 M instructions and three other
libraries alike within their spread. So it costs every document with a library about 6 ms at its
first trigger and pays only on folding constants, which no form's script is; and it is a second
recursive walk over what a hostile script nests. **Off**, in `Realm::with_asker`.

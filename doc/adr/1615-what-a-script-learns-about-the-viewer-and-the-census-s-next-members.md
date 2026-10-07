# 1615 — What a script learns about the viewer it runs in, and the census's next members

Status: accepted and **built** in `pdf-script` and `pdf-model`; the host's half of `setFocus` is
round 1390's. Session 1389. Builds on RFC 0008 sections 3.4, 4.2 and 10 question 11, answered by the
owner in `doc/questions/A193` (answer 11: the truth, this program's name and version); extends ADR
1591 (the bridge) and ADR 1603 (a script's write is an edit, kept where it is not drawn).
Context: ISO 32000-2 §7.9.4 (dates), §12.7.4.3 (Table 231's `Comb`); Adobe's *JavaScript for Acrobat
API Reference* at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`, its "app properties", "util
methods", "Field methods" and "Field properties" pages, cited by name and paraphrased.
Code: `crates/pdf-script/src/viewer.rs`, `engine/bridge.rs` (`identity`, `print_mask`,
`print_date`, `write_flag`, `get_array`, `set_focus`), `engine/guard.rs` (`string_units`),
`surface.rs`, `wire.rs`; `crates/pdf-model/src/view/script_model.rs` (`TextFlag`,
`Property::TextFlag`, `ScriptEdit::Focus`), `view/scripts.rs` (`take_focus_request`).
Tests: `crates/pdf-script/tests/members.rs`; the columns `crates/pdf-script/tests/script_corpus.rs`
and `crates/pdf-script-worker/tests/script_column.rs`.

## 1. The viewer's identity is this program's

The reference's values for `viewerType` (Reader, Exchange, Exchange-Pro) and `viewerVariation`
(Reader, Fill-In, Business Tools, Full) each name a product or a packaging of Adobe's, and this
program is none of them. A193 settled it: **`viewerType` and `viewerVariation` answer `quorra`;
`viewerVersion` and `formsVersion` answer the release's major and minor as a number** (`0.1.0` is
`0.1`), read from the crate's manifest, the two being one software. A script that compares the
version with 7 or 9 takes its old-viewer branch, which RFC 0008 section 3.4 priced: a dialog the
document's author wrote for a viewer that is not Adobe's, under the one-dialog cap, with the level
as the lever for quiet. **`platform` and `language` answer from the reference's own lists**, because
those name facts rather than products: `UNIX` for every build that is neither Windows (`WIN`) nor
macOS (`MAC`), decided at compile time; `ENU`, English, the one language this program's interface
is written in — a translated host states its own. All six are read-only, and a write is refused by
name (`app.viewerVersion=`).

## 2. `util.printx` and `util.printd`, through the same functions Tier 0 runs

`printx` is `pdf_model::aform::print_mask`, the function `AFSpecial_Format` writes through (ADR
1578 section 7); its output never exceeds the two strings the script already holds, so it asks no
budget. The reference's telephone-number example is the fixture.

`printd` writes a `Date` read in local time at the request's offset through
`pdf_model::aform::print_date`, the picture language `AFDate_FormatEx` writes. The reference also
numbers three formats, each a choice here:

- **0** is §7.9.4's date string, local time with its offset — `D:20000801145605+07'00`. The
  reference's example ends in a further apostrophe, PDF 1.7's spelling; §7.9.4 writes none, and its
  NOTE 2 is what keeps the older form readable, so the standard's spelling is written.
- **1**, which the reference calls universal, is the same moment in Universal Time with §7.9.4's
  `Z`. Its printed example repeats format 0's, so the word is what decides.
- **2** is the reference's example's own shape, `yyyy/mm/dd HH:MM:ss`, because this program has one
  locale.

An XFA picture clause (the third argument true) is XFA's, which Annex K permits a processor not to
implement, and is refused by name, as are a second argument that is not a `Date`, a `Date` holding
no time value, and any other number. A picture's output is held to the string budget before it is
built, at nine characters for every four of the picture (`mmmm` writes `September`).

## 3. A field's `getArray`, `setFocus` and text flags

- **`getArray`** answers the terminal fields below a name, each a `Field`; for a terminal field it
  answers the field itself — the reference speaks only of a parent's terminal children, and a
  choice is made.
- **`setFocus`** is a `ScriptEdit::Focus` for the first terminal field. The focus is the host's —
  which widget a key reaches, and the page turned or scrolled to show it — so the view state holds
  the latest request and a host takes it with `ViewState::take_focus_request` after any call that
  ran scripts, and moves the focus as a press would, raising Table 197's `/Bl` and `/Fo`. That call
  is round 1390's to make.
- **`multiline`, `password`, `doNotScroll` and `comb`** are writable, as `Property::TextFlag`. Each
  is a text field's flag and is refused on any other field. `comb` is held to Table 231's own
  sentence — it may be set only where `/MaxLen` is present and Multiline, Password and FileSelect
  are clear — and setting it sets `doNotScroll` too, the side effect the reference states. Like
  every property beyond value, display and read-only, the flags are kept and reported as not drawn
  (ADR 1603).

`pdf_script::wire::VERSION` is 3 for these and ADR 1614's sites: property tag 9, edit tag 4.

## 4. What the columns said

The figures are the round's record's, `doc/history/1389-*.md`, which prints them as they were
measured; this section states what was decided from them. Both columns now also run Table 200's
five after the commits. The Tier 1 column through the worker (`script_column`) runs every script
of the in-process column in `pdf-script-worker` under `Profile::Script`, one worker per document,
and counts its deaths by cause; a death by `SIGSYS` would be a call the bridged model reached and
the profile does not admit, and the column holds that count at zero. **It was zero, with no worker
lost to any cause, so `Profile::Script` gains nothing and the model loses no member**: the six
identity answers are constants, and `printd`'s local time is the realm's own hook over the
request's offset, so nothing here asks the kernel for a clock zone, a locale or a file. The run
ended in the in-process column's five columns count for count, which is the evidence that the process
boundary changes no outcome.

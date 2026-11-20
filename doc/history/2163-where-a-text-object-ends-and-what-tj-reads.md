# Round 2163 — where a text object ends, what `TJ` reads, and a glyph's text kept off the page's

A fix round in the `gwg` worktree for audit 2161's five findings (§9.2 to §9.4); all five audit
tests in `crates/pdf-model/tests/text_objects_spec_audit.rs` pass.

## What changed

- B01 + B04 (ADR 3353): a `BT` inside an open text object ends it as `ET` would — §9.3.6's clip,
  §11.7.4.4's and §9.3.8's groups — then begins its own, reported as `TextObjectNested`; `ET`
  resets the `TextObject`, discarding Tm and Tlm; a positioning or showing operator outside a text
  object, and a stray `ET`, are reported (`TextOperatorOutsideATextObject`); the operator acts on
  the identity matrices, so a stream with no `BT` still draws where its `Td` says.
- B02 (ADR 3354): `FlatArray::last_of` in `run/operands.rs` reads the last bracketed run; `TJ` and
  `d` share it. What stands outside is `OperandOutsideItsArray`; a `TJ` element neither string nor
  number is `TextArrayElementOfTheWrongType` and is not shown. `set_dash` returns `DashFaults`.
- B03 (ADR 3355): a text state operand of the wrong type is `TextStateOperandOfTheWrongType`;
  `Tc`/`Tw`/`Tz`/`TL`/`Ts`/`Tr` keep their value; `Tf` leaves unset (no font, size nought) what it
  does not state, a shortfall included, rather than keeping the earlier size.
- B05 (ADR 3356): `separate_text` returns inside a glyph description and the readback cursor moves
  only through `move_text_cursor`, both gated on `glyph_depth` as `read_back` is.
- `stream_faults_by_name::a_text_objects_faults_are_faults_of_the_stream` names each new fault.

## Manifest and ledger

The five 2161 entries left `expected-failures-current.json` (2148-B04 stays); 2161's findings are
`fixed`. Rows 9.3.1, 9.3.6, 9.4.1, 9.4.3, 11.7.4.4 stay `implemented`, their notes corrected (9.4.1's
"restarts the text object" is gone) and the audit tests added to their `test` lists.

**Gates.** fmt 0; clippy `-D warnings --all-targets -p pdf-model -p render-cpu -p viewer-qt` 0 (101 on the
first run: one `match` that wanted `if let`); `pdf-sandbox --bins` 0; nextest of the three crates
(`xvfb-run -a`): 2 failed — 2148-B04 (manifest) and `viewer-qt`'s `unsafe_position` (Q979); rustdoc
`pdf-model` 0; `fuzz cargo check --bins` 0 (`StreamSyntaxFault` gained five variants);
`cargo test -p conformance --no-fail-fast` 0. `raster_golden`: 11 moved, none by a pixel of this round's; its four are reports only — a stray `ET` (bug1065245, issue9458), `TJ`/`Tm` with no `BT` (issue10388_reduced, poppler-90-0-fuzzed).

## Habit

Choose a malformed construct's reading by the sentences that are there: Table 105 states none for a
nested `BT`, but `BT`'s own "initializing" is possible only once the open object has ended.

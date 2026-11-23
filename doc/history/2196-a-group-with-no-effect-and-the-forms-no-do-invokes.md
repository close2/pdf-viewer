# Round 2196 — a group with no effect, and the forms no `Do` invokes: audit 2195's fixes

A fix round in the `audit` worktree for review 2195's three B and two D findings (§8.9 to §8.11).

## What changed

- **B01, D01, D02** (`optional_content.rs`): a group with §8.11.2.3's "no effect on visibility"
  (outside the configuration's `/Intent`, or unlisted) drops out of its `/VE` operator; an
  expression with nothing left shows the content, as the `/OCGs` route's empty states do. A `/VE`
  with no operator name, an operator other than the three, or a `/Not` with no operand is
  `Visibility::Malformed`, drawn and reported in its own sentence, never as the depth bound. Both
  comments restated. ADR 3485.
- **B02** (`xobject.rs` `enter_imported`): the imported page's groups are the `ViewState` built
  there, `/Print` event, magnification and now the audience carried — not a fresh read. ADR 3487.
- **B03** (`annotations.rs` `draw_annotation`): a stored appearance stream's own `/OC` (Table 93,
  §12.5.5) is asked beside the annotation's; a hidden one takes its drawing, text span and `/H`
  highlight, not a playing sound's background. Row 12.5.5's `departed` note (Table 166's `/ca`,
  `/CA`) is untouched. ADR 3486.
- Two tests of this round's in `tests/optional_content.rs`: the malformed sentences, and a
  nested non-participating group with a two-operand `/Not`.
- The four audit tests pass; their entries left `expected-failures-current.json` (now empty), and
  2195's five findings are `fixed`.

## Ledger

8.11.2.3, 8.11.2.2, 8.11.3.3, 8.10.2, 8.11.4.5, 8.10.4.1: `implemented` before and after, each
re-read against `doc/md/`, its note given the sentence that now holds and its test list the test.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p viewer-qt`
exit 0; `pdf-sandbox` 0; nextest `--lib --bins --tests` of the three (`xvfb-run -a`) exit 0, 3688
passed, 26 skipped, `audit-diff.py` "failed 0, expected 0"; rustdoc `-D warnings
--document-private-items -p pdf-model` exit 0; fuzz `cargo check --bins` exit 0 (`Visibility`
gained `Malformed`); `cargo test -p conformance --no-fail-fast` exit 0. `raster_golden`: held 974,
moved 0; the strips test 0. No golden page moved, so none was looked at; the fixtures' pixels
are what the tests assert, both ways.

## Habit

A rule keyed on an object's kind binds every route that draws one (audit 2195's habit) held again:
`draw_appearance` is a form XObject's second entry point, and `/OC` had to be asked there too.

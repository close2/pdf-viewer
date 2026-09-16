# 1120 — The annex that was already eleven shalls done, and the malformed page end to end

2026-09-16. Files: `crates/viewer-core/tests/fragments.rs`, `doc/conformance/ledger.toml` (clause
O.2.1's `test` array), this file. No ADR: nothing about the fragment-to-view mapping was decided —
ADR 0209 already states it and ADRs 0250/0310/0357/0431 the four parameters that came off the
refused list. Every other path in `git status` is a sibling's.

**The contract was Annex O, and Annex O was done before this round opened.** `tools/state.sh
annex-o` prints, unchanged before and after, *carried out: Page NamedDestination StructureElement
Comment EmbeddedFile Zoom View ViewRect Highlight Search Fdf* and *reported: none* — all eleven of
Table Annex O.3's five and Table Annex O.4's six, since the five-hundred-and-twenty-second session
for the last two (highlight, fdf) and the five-hundred-and-ninety-sixth for `ef`'s trailing
parameters. `doc/todo/39` is closed. Rows O, O.1, O.2, O.2.1, O.2.2 are all `implemented`; none
moved, because there is no status past it and no `shall` unbuilt. So the round was trap 8's
measurement and trap 13's calibration, not an implementation.

## Calibration — the four the contract named, each already asserting the view it names

- `#page=3` → index 2 (`a_fragment_opens_the_page_it_names`).
- `#nameddest=page.2` → index 1 via §12.3.2.4's name tree, `vertical.pdf`'s own `/Names`
  (`a_named_destination_opens_the_page_the_document_files_it_under`).
- `#zoom=200` → scale 2.0, a percentage against `view=XYZ,,,2`'s factor
  (`a_zoom_is_a_percentage_and_a_view_is_a_factor`).
- Malformed → page one and *named*, not silent: covered at the parser by
  `fragment.rs::a_malformed_number_is_refused_rather_than_salvaged`.

## The one gap that was evidence, not behaviour

The malformed-number calibration existed only at the parser (`Reason::Arguments` on `page=12pt`),
never end to end — the same thinness session 939 charged the aggregate O row with. Added
`a_malformed_page_number_opens_at_page_one_and_is_named`: `Command::Open` with `#page=12pt` leaves
`page(&viewer) == 0` and reports *its arguments are not what Annex O states for it*, which is
`apply_fragment` naming an `unread` entry rather than opening page one in silence (CLAUDE.md's no
silent swallowing, §O.2's left-to-right). Cited in clause O.2.1's `test` array; note untouched.

`launch_path` not run — the open path did not grow (a test and a citation only). Gates:
`cargo test -p viewer-core` 51 lib + 161 integration (fragments 19) ok, exit 0; `cargo test
-p pdf-model --test corpus` 4 ok 1 ignored, exit 0; `cargo test -p conformance` all ok (conformance
7, records 1), exit 0; `cargo clippy -p viewer-core --tests` clean, exit 0; `tools/batch.sh check`
clean, exit 0.

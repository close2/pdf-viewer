# 1725 — The small leftovers of batches 109 and 110, each read against its clause

Session 1725, slot 6 of batch one hundred and eleven, a clause round; 2026-10-23. ADR 2284 (2285
unused), no `Q`, no ledger row moved.

**1. The popup list.** §12.5.6.14's popup "is associated with a markup annotation", and §12.5.6.2
says Table 172, `/Popup`'s table, lists entries "that apply to all markup annotations"; §12.5.6.2's
groups are not the list, since `Redact` is in none of them. So `linked_popups.rs` follows Table
171's Markup column less Sound ("Sound annotations do not have a popup window"), keeps `FreeText`
and `Redact`, and gains `Projection`, whose §12.5.6.24 gives it Table 172 (ADR 2284). The hypothesis
(two names lost) did not hold. Slot 3 was not editing the file, so this round edited it.
`a_popup_pairs_with_every_markup_subtype_but_sound` holds the list; with `Projection` planted out it
fails on `Projection`.

**2.** `pdf-font/src/metrics.rs`'s `DEFAULT_WIDTH` now gives only the current reason (ADR 0039).
**3.** The query is `LogicalSelection`. It has no entry point because `quorra_copy`'s
`Event::Copied` carries the same answer in `logical`, and `quorra_selection_copy_text` reads it back
with its order (ADR 0519). `tools/state.sh hosts` checks each such `Variant:Event` pair against
`viewer-ffi`'s code and prints "36 of 36" with the event named. With the event planted wrong, it
prints "35 of 36" and the query again.

**4.** `doc/todo/75`'s counts are now commands: `zoom_frame`'s `command(s) encoded` and `lanes: line`
(the step's walk is their difference) and `sub_pixel_width_census`'s `§8.5.3.2 dots`. Both ran on the
witness and printed the counts the file held.

**5.** `tools/state.sh batches` prints a line under each batch commit that counts the paths it names
under `/home/AI/` and `scratchpad/`, word by word as `evidence.rs` counts them. `--outside-paths` is
the function, and a planted body in `state_sections.rs` holds it. **6.** `doc/todo/02` section 8 step
1 and the template's section 3 say "against HEAD's six arms (`tools/batch.sh arms`)" and name no path
(ADR 2260).

**Gates.** The session worked in `scratchpad/r1725/`. Bare in the worktree: `cargo fmt --check` exits 0
for `pdf-font` and `conformance`. For `pdf-transform` it exits 1 on `archive/fonts.rs` alone, a
sibling's file; `rustfmt --check` on this round's four `.rs` files exits 0. `RUSTFLAGS="-D warnings"
cargo clippy --all-targets` exits 0 for `pdf-font`, `pdf-transform` and `conformance`. `cargo nextest
run` exits 0 for `pdf-font` (294 passed) and `pdf-transform` (664 passed, 14 skipped). `cargo doc -p
pdf-font` with warnings denied exits 0. `cargo test -p conformance` passes but `names` and `teaching`, both
the text interpreter's split (a sibling's); `evidence`, `adr_status` and `records` pass. No arm is owed:
`cargo tree -i pdf-transform` reaches no `render-raster` (habit 119), and `pdf-font`'s edit is a doc
comment. Duration 3 600 s.

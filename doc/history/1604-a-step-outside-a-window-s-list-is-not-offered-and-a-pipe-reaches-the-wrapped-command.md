# 1604 — A step outside a window's list is not offered there, a pipe reaches the wrapped command, and tier 1's crate checks are printed

Slot 6 of batch ninety-five, 2026-10-19, a docs tools round. ADRs 2044 and 2045; no question.

**Premise and hypothesis.** The three premises held where the brief put them. `tools/bounded.sh`
started its command in `( … ) &`, which bash hands `/dev/null` where job control is off:
`printf 'piped\n' | tools/bounded.sh --tree 1 -- cat` printed nothing at HEAD. `--step` ran a step
in every window: HEAD's `--step accessibility` in `quorra-confined` said `wrong` after 20.2 s. The
batch-92 merge's gates line names the Sonnet round's `teaching.rs` failing fmt and clippy. **One
reading did not hold**: a step outside a window's list is not always one the window cannot do.
HEAD's `--step rendition_click`, in the confined list alone, said `works` in the other three
windows, which hand the clip over through the same `viewer_host::handed_media`. The hypothesis held:
a few lines each, nothing under `crates/`.

**Built.** `bounded.sh`: the subshell is `( … ) <&0 &`, a `--build` reads `/dev/null`, and
self-test case 15 pipes two lines through `cat` (ADR 2044). `drive-windows.sh`: the loop's two lists
are the arrays `CONFINED_STEPS` and `WINDOW_STEPS`, 65 names in HEAD's order (a diff of the old
loop's names against the arrays is empty), beside `OUTSIDE_STEPS`; under `--step` a step the
window's list does not hold says `not offered` and is not run, a name in no list is refused with
exit 2, a listed name that is no function stops the drive, `rendition_click` joins the three
windows' list, and the summary counts `not offered` (ADR 2045). `round.sh` prints `cargo fmt --check
-p <crate>` and the `-D warnings` clippy line under "the round" for every kind; `doc/todo/02` §0
item 3 names both.

**Rows.** None; no clause is touched. Stale sentences: none found beyond trap 207's row, which is
not this round's to edit (the report proposes its words).

**Gates.** `tools/bounded.sh --self-test`: exit 0 twice, the second at a load average of 29.93;
planted with `<&0` taken out, exit 1 on case 15. `cargo test -p conformance --no-fail-fast`: 442
passed, 0 failed; after the last edit `--test bounded --test records --test traps`: 24, 3, 2 passed.
`RUSTFLAGS=-D warnings cargo clippy -p conformance --all-targets` and `cargo fmt --check -p
conformance`: exit 0. `--bin pointers`, `--bin quotations`: exit 0, no absent pointer of this
round's; `tools/state.sh --round 1604 docs`: exit 0. The drive on the worktree's release binaries copied to scratch, display `:164`: `--step
confined --step rendition_click --step accessibility --step zoomed_window_processor` in the four
windows, 10 works, 0 wrong, 7 not offered; `archive_chosen` refused with exit 2.

**Unfinished.** Nothing. Duration: 1374 s.

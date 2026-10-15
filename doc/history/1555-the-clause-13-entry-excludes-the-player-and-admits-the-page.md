# 1555 — The clause 13 entry excludes the player and admits the page, and two stale comments say what is

Slot 4 of batch eighty-eight, 2026-10-14, a docs round. ADR 1946; no question.

**(a) Built.** `CLAUDE.md`'s exclusion now reads RFC 0009 §11 question 2's sentence, the one A254
adopted as written, with its ratification beside it. **The hypothesis held for §13.4, and it was
narrower than the change**: §13.4 goes `departed` → `implemented`, because the playing and the
boolean `/Poster` are now the closed list's ("[i]f it is the boolean value true , the poster image
shall be retrieved from the movie file"). But the entry now admits viability and the hand-off by
name, so rows that rested on "clause 13" alone had to be read again (ADR 1946 §2). §13.6.2 and
§13.7.2.1 go `out-of-scope` → `implemented` ("Inactive (the default initial state): the annotation
displays the annotation's normal appearance."), and §12.5.6.25, their pointer, follows them.
§13.2.2 ("In making these decisions, the viewer shall determine the viability of the objects
used."), §13.2.4.2 and §14.9.2.4 go `out-of-scope` → `reported`: they are owed by RFC 0009 §12 item
3, and the rendition action's refusal is said where playing begins. By ADR 1599, §13.2, §13.2.4,
§14.9.2 and §14.9 become `partial`. The other 77 notes name the amended entry; `doc/todo/65`,
`state-of-play`, HANDOVER's RFC 0009 line and `Exclusion::Multimedia`'s doc comment follow.

**(b) Built; the premise was off by one site.** `worth_combining` lives in `image/mask.rs` (ADR
1912). `absence_audit.rs:186` and `:776` named `image.rs`; `:435` named only the local copy and now
names the original too. `compute.rs`'s module comment was stale twice: its pass list had no scan,
and it described a mid-frame readback. Both now describe ADR 0095's one submission and its eight
bytes read at the frame's end.

**The proof that nothing moved**: every changed line in the three `.rs` files is a `//`, `///` or
`//!` line, checked over `git diff -U0`. **Unfinished.** RFC 0009 §12 item 3 itself; Table 285's
form-XObject clip, unread (ADR 1946 §3); `action.rs`'s "Rendition: …, excluded" still says nothing
of viability, because changing it waits on that report.

**Files.** `CLAUDE.md`, `doc/conformance/ledger.toml`, `doc/todo/65-the-remaining-frontier.md`,
`doc/state-of-play.md`, `doc/HANDOVER.md`, `crates/pdf-model/examples/absence_audit.rs`,
`raster/crates/raster-gpu/src/compute.rs`, `tools/conformance/src/ledger.rs`, ADR 1946, this record.

**Gates.** `rustfmt --check`, 3 files: 0. Clippy `-D warnings --all-targets`: `conformance` 0,
`raster-gpu` 0, `pdf-model` 101 only on a neighbour's `examples/r1552_probe.rs` (8 lints), and
`--example absence_audit` 0. `cargo nextest run -p raster-gpu` 0 (723 passed). `tools/state.sh
docs` 0. `cargo test -p conformance` 101: 435 passed, and `round_numbers` fails on that same probe.
`--bin pointers` 0 and `--bin quotations` 0, neither naming this round's lines. `tools/batch.sh
check` 1, on `cargo fmt --all` over two neighbours' examples. About 4 300 s.

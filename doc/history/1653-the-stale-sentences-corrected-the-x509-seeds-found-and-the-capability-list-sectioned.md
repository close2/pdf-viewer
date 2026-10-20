# 1653 — The stale sentences corrected, the x509 seeds found again, and the capability list sectioned

Session 1653, slot 7 of batch one hundred and one, a docs round; 2026-10-20. ADR 2142 (2143 unused).
What rounds 1637 and 1645 (ADRs 2110, 2126) found false outside their slots, each claim checked
against the tree first and corrected where it stands (ADR 1023).

**Built.** `fuzz/seeds.sh`'s `x509` arm handed the seeder `crates/pdf-model/src/*.rs`, where no
certificate is, so a re-seed reached none of the twelve fixture certificates that alone reach
`dsa::verify` and the curve arms. It now feeds every `.rs` under `crates/` beside the documents;
`tools/conformance/tests/fuzz_seeds.rs` runs the arm from a copy of `fuzz/` with no documents and
holds the summary's three routes to the constants it counts itself (fixture 12, the others 0);
`doc/state-of-play.md` has 26 `###` sections under `## The state of play`, words unchanged.

**The premises that did not hold.** "Five routes": twelve certificates, from five files. "Re-point
`tools/round.sh`'s pointers": it names no section of `state-of-play.md`, so `doc/HANDOVER.md`'s row
says to read the section instead. "Words unchanged" bounds the sections: one sentence runs 150
lines (password to save) and stays one section.

**Found false, corrected** (ADR 2142 §3): the toolkit windows "draw no page through a device"; todo
15's vanished quotation; `pdffs`; `NOTICE`'s `pdf-viewer`; SHAKE256's "no second reading" and
cms.rs's "all ten"; todo README row 57; `RestrictKind::On` "the default"; `pages_corpus`'s absent
structure tree; `foreign_corpus`'s "five writers exist"; `spelled_ordinals`' "the three"; a history
sentence in `bridge.rs` and the three-crate list there and in `doc/PLAN.md`; two ordinals in
`quorra.h`; `only_the_three_named_crates…` asserting six, renamed; `gesture.rs`'s header and
command; `press_cost.rs`'s `static`; `awkward_classes.rs`'s ignore reason and `--bins` line.

**Found, not fixed** (other slots' files): `crates/viewer-ffi/c/open_a_page.c` line 606 and
`doc/running-the-viewer.md` line 328 spell a session by ordinal; `quorra-confined` shows no
third-party notice; the drive's step 27 does not read the toolkit windows' refusal.

**Gates.** `cargo test -p conformance --no-fail-fast`: 101, on `names` (`render-raster`'s
`examples/image_residual.rs` naming `scene::image`, slot 8's split) and `worker_features`
(`bytemuck`'s features, another slot's manifest) alone; `fuzz_seeds` 1 passed, failing with the old
glob planted back; `records`, `round_numbers`, `spelled_ordinals`, `traps` 0. `--bin quotations` 0,
`--bin pointers` 0. `cargo fmt --check -p conformance` 0, clippy 0. Habit 146: every ADR in a
deleted line is in an added one. `tools/state.sh --round 1653 docs` 0 after 224 s. `bash -n` 0 on
`fuzz/seeds.sh` and `tools/drive-windows.sh`. `cargo fmt --check` 0 on the nine crates touched;
clippy with dependencies 101 on seven, all on a sibling's `render-cpu/src/lib.rs:409`, so `--no-deps`:
0 on six, 101 on `viewer-device` at a sibling's `renderer/thread.rs` alone. `cargo test` of
`viewer-qt`'s and `viewer-ffi`'s `unsafe_position`, both header tests and `pdf-fuse`: 0. About 2 300 s.

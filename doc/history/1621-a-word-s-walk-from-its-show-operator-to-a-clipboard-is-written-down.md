# 1621 — A word's walk from its show operator to a clipboard is written down

Slot 7 of batch ninety-seven, 2026-10-20, a docs round: ADR 2078 (2079 unused), no question, no `crates/` file.

**Premise.** Held in substance: every path the walk names exists after batch ninety-six's splits.
One detail did not: `logical_range` is in the root `crates/pdf-model/src/structure.rs`, not in
`structure/logical.rs`, which ADR 2046 gives the whole-page order. **Hypothesis.** Five stages held,
in a changed order: `/ActualText` and the soft hyphen come before the drag, because a replacement
is applied at the `EMC` inside the interpretation (ADR 2078 §2.3).

**Built.** `doc/teaching/how-text-is-selected-and-copied.md` follows §14.9.4's EXAMPLE word
*Drucker* on a small tagged page through five stages, six Mermaid diagrams with a note each. Every
string in it was read off the tree: the readback with `crates/pdf-model/examples/readback.rs`, the
drag and both copies through the C ABI from a C probe in this round's scratch.

**Census** over batch 96's span (habit 110: eight transcripts, rounds 1607–1614, first record 23:40Z,
after the 23:09Z open): `--since 2026-10-20T01:09:15+02:00 --until 2026-10-20T05:59:44+02:00
--buckets`, 1 518 requests, 3.1 M tokens written, 376.1 M read, 2 gaps over 300 s and 1 miss —
round 1611's 6 406 s after its `SubagentHandback`, a resumed agent and not a wait (0.4 M written).
The 318 s gap after a `tools/wait.sh` hit, as did all 49 requests in [240, 270) s; cache cost 22.7 M
equivalents. Control, round 1565's transcript: 14 of 14 gaps past 400 s missed.

**Gates.** `cargo test -p conformance --test teaching`: exit 0, 3 passed; three faults planted in
the new walk (a file, a name, ADR 9997): 3 reported, exit 101, then taken out. `cargo test -p
conformance --no-fail-fast`: exit 0, 59 binaries. `--bin quotations`: exit 0, none of mine
diverging, a planted misquote of §14.8.3.3 reported (9 of 11 words). `--bin pointers`: exit 0, none
of mine absent. `rustfmt --check`, `cargo fmt --check -p conformance`, clippy `-D warnings
--all-targets`: exit 0. Mermaid 11.4.1: 21 of 21 diagrams over three walks, 2 of 2 planted errors
caught. `tools/state.sh --round 1621 docs`: exit 0.

**Found, not fixed.** (1) `crates/viewer-core/src/viewer/geometry.rs::device_lines` appends each
code's slice and `push_replacement` gives every code of an `/ActualText` section the whole
replacement, so the example's first paragraph line is `Der Drucc` in the accessibility tree, against
`TextLine`'s invariant. (2) `crates/viewer-ffi/src/session.rs::copy_text` (`quorra_selection_copy_text`)
gives a C host copy text without `Command::Copy` — Table 22 bit 5's level is never asked — and
unrejoined: `Seiten\u{ad}\nzahl.` where `Event::Copied` says `Seiten\u{ad}zahl.`. (3)
`crates/viewer-ffi/src/events.rs::copied` restates `viewer_host::copied` rather than calling it.
**Files.** The new walk, `teaching.rs`, `doc/HANDOVER.md` (one row), `doc/crate-map.md` (lines
20–23), ADR 2078, this record. 1 985 s.

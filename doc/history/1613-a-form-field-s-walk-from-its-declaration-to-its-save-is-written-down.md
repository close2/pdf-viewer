# 1613 — A form field's walk from its declaration to its save is written down

Slot 7 of batch ninety-six, 2026-10-20, a docs round: ADR 2062 (2063 unused), no question, no `crates/` file.

**Premise.** Held in substance: every path the walk names exists, and the census shows no miss.
Three details did not hold. ADR 1954 is the window's PDF/A conversion, not a form step. `quorra`
has no typing card: it draws the field's own appearance and asks the core for the caret, and its
card shows a script's question or a restriction's ask. `viewer_host::keys` is asked only for a key
that reached the page. **Hypothesis.** Six stages held, in a changed order: the scripts come before
the appearance, because `/K` runs inside `set_field` and the commit's `/V` and `/CO` run before
the next frame (ADR 2062 §2.2).

**Built.** `doc/teaching/how-a-form-field-is-filled.md` follows §12.7.5.3's EXAMPLE field
(`Silly prose`) through six stages, then the reopen. It has seven Mermaid diagrams, a maintenance
note under each, and fifteen quotations of ISO 32000-2. `teaching.rs` holds both walks from one
`WALKS` table (eight and seven diagrams), and its pointer test checks every walk. Slot 2's
move to `crates/viewer-device` left the first walk naming three absent files, so its stage 7 is
re-pointed and its sentence rewritten in the present tense.

**Census** over batch 95's span (habit 110: eight transcripts, rounds 1599–1606, first record at
20:15Z, after the 20:07Z open): `--since 2026-10-19T22:07:16+02:00 --until
2026-10-20T01:09:15+02:00 --buckets`, 1 057 requests, 1.9 M tokens written, 183.0 M read, 0 gaps
over 300 s, 0 misses. The 46 requests in [240, 270) s were all hits; cache cost 11.5 M
equivalents. Control, round 1565's transcript: 14 of 14 gaps past 400 s missed.

**Gates.** `cargo test -p conformance --test teaching`: exit 0, 3 passed. Three faults planted in
the new walk (a file, a name, ADR 9998): 3 reported, exit 101, then mended. `cargo test -p
conformance --no-fail-fast`: exit 101, five binaries failing on siblings' work in progress —
`records` (1614's record), `state_sections` and `names` (the renderer's move), `conformance`
(§11.3.6's `software.rs` test), and `teaching` until stage 7 was re-pointed. `--bin quotations`:
exit 0, none of mine diverging, a planted misquote reported (35 of 38 words). `--bin pointers`:
exit 0, none of mine absent. `rustfmt --check`, `cargo fmt --check -p conformance`, and clippy
`-D warnings --all-targets`: exit 0. Mermaid 11.4.1's parser: 15 of 15 diagrams, 2 of 2 planted
errors caught. `tools/state.sh --round 1613 docs`: exit 101, on `viewer-ui`'s rustdoc
(`software.rs`'s unresolved `TargetSpec` link, slot 2's file mid-edit).

**Found, not fixed.** `crates/pdf-model/src/view/writing.rs:132` says what the line "used to"
justify, a history comment ADR 1023 retires. **Files.** Both walks under `doc/teaching/`,
`tools/conformance/tests/teaching.rs`, `doc/HANDOVER.md` (one row), `doc/crate-map.md` (lines
19–22), ADR 2062, this record. 2 300 s.

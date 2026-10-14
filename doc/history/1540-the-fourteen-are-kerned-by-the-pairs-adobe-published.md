# 1540 — The fourteen are kerned by the pairs Adobe published for them

Slot 1 of batch eighty-six, 2026-10-14, a clause round building `doc/questions/A308`'s "Owes" line.
ADR 1916; ADR 1917 and Q435 not needed. Rows: §12.7.4.3, §12.7.5.3, §12.5.6.6 `partial` →
`implemented`, and their headings §12.7.4, §12.7.5, §12.5.6, §12.7, §12.5 with them (ADR 1599);
§9.4.3 and §9.6.2.2 stay `implemented`, their notes and paths extended.

**Premise.** Held: `pairs.rs` already read an embedded program's `GPOS` and `kern` pairs and the
layout already wrote them as `TJ` numbers (ADR 1682), so the `kern` half and the `TJ` half were
built; what was owed was the fourteen's table. `NOTICE` and `PROVENANCE.md` existed. **Hypothesis.**
Held: Adobe's own server still serves `Core14_AFMs.zip` (download.macromedia.com; the old
partners.adobe.com URL is 404, the Internet Archive's copy has the same digest), `MustRead.html`'s
terms are A308's reading, Helvetica 2 705 and Times-Roman 2 073 `KPX` lines.

**Built.** `data/standard-fonts/afm/`: the fifteen files byte for byte, `SHA256SUMS`. `build.rs`
writes their `KPX` lines into a relocation-free table marked at its head as a modification, with
each file's notice; `pairs::published` reads it; a non-embedded simple font whose `/BaseFont` is one
of §9.6.2.2's strings is kerned by its face's file, by the glyph name each code selects. `NOTICE`
§1c carries the paragraph unmodified and every file's Notice line; `viewer-ui/tests/notices.rs`
checks the paragraph against the file, each name and the digests. The batch check admits the files
by path. Looked at: `AVAWAY To Wa` in Helvetica and Times, kerned and not, at 3× — tighter where the
files say, nothing else moved. **Unfinished**: none; clones such as Arial stay said (ADR 1916 §3.1).

**Files.** `data/standard-fonts/afm/` (15 + `SHA256SUMS`), `data/standard-fonts/PROVENANCE.md`,
`NOTICE`, `crates/pdf-font/build.rs`, `crates/pdf-font/src/{pairs.rs,pairs/published.rs,loading.rs}`,
`crates/pdf-model/src/rich_text/layout.rs` (a doc comment), `crates/pdf-model/tests/rich_text.rs`,
`crates/viewer-ui/tests/notices.rs`, `tools/batch.sh` (one admission), `doc/conformance/ledger.toml`,
`doc/todo/01`, `doc/todo/65`, `doc/state-of-play.md`, ADR 1916.

**Gates.** `rustfmt --check` on the seven Rust files: 0. `clippy -D warnings --all-targets`
`pdf-font`, `pdf-model`: exit 0; `viewer-ui --test notices`: exit 0. `nextest -p pdf-font` 285
passed; `-p pdf-model` 2 052 passed; `viewer-ui --test notices` 7 passed. `cargo test -p
conformance` in the worktree: one failure, §12.10.2's test name, slot 2's mid-edit; in the export
`tests/conformance.rs` 8 of 8 (the eight others failing there read git, which an archive is not).
`tools/batch.sh check`: this round's lines clean. One hold on the export (HEAD + this change, the
ledger as a patch; queued 1 763 s, held 1 582 s, exit 0): `t2-corpus` exit 0, ratchets at their
ceilings; `t2-raster_golden` exit 0, held 974, moved 0; the six arms exit 0 against
`/home/AI/arms-1540/`: 968, 968, 968, 965, 965, 965 pages, 0 digests and 0 mean errors differ (the
comparison tells HEAD's `cpu-1x` from its `gpu-1x` on 106 pages). Duration 6 100 s.

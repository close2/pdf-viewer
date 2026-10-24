# 1729 — e-sRGB and e-sYCC are drawn from PIMA 7667

Session 1729, slot 2 of batch 112, a clause round; 2026-10-23. ADRs 2292, 2293; no `Q`.

**Premise.** Held, with one correction: PIMA 7667:2001 is on disk at `doc/pima7667-2001.pdf`
(SHA-256 as A348 records), and T.801 Table M.25 gives e-sRGB 20 and e-sYCC 24 — the brief's 21 is
ROMM-RGB, already drawn. Not held: "the row moves to `implemented`" — CIEJab (19) still owes, so
§7.4.9 stays `partial` with its reason narrowed to 19 alone, as A348 asked. The hypothesis of two
spaces under two hundred lines each held: one module of 244 lines before its tests.

**Built.** `pdf_colour::colour::ExtendedSrgb`: section 4.5's equations 5, 6a–6c and 7, Annex B.2's
B7–B9, D65 onto D50 by Bradford, painted as `Lab` under D50 so an extended colour is not clipped
before its destination; e-sRGB at section 4.1's 10, 12 and 16 bits, e-sYCC at any depth (ADR
2292). `jpx.rs` frees every `colr` box for the two and reads the integers itself; `jpx_samples`
refuses them to a writer by name (ADR 2293). The first e-sYCC fixture held two colours outside
section 4.3.4's luminance bound and drew them through `Lab`'s lightness clamp; it was replaced by
four valid colours rather than tuned. Each new test was seen red with the two codes taken back.

**Files.** New: `crates/pdf-colour/src/colour/extended_srgb.rs`, ADRs 2292 and 2293, this record.
Edited: `crates/pdf-colour/src/colour.rs`, `crates/pdf-model/src/image/jpx.rs`,
`crates/pdf-model/tests/jpx_enumerated_spaces.rs`, `crates/pdf-model/examples/jpx_colour_census.rs`
(its doc comment), §7.4.9's row, `doc/todo/01`, `doc/todo/65`, `doc/third-party-data.md` (the
PIMA 7667 entry, with the conversion's command, trap 187). I worked in `scratchpad/r1729/`.

**Gates.** Worktree: `rustfmt --check --edition 2024` on the five Rust files 0; `cargo fmt --check -p
pdf-colour` 0 and `-p pdf-model` 0; `-D warnings clippy -p pdf-colour -p pdf-model --all-targets`
0; nextest over both 2 341 of 2 341; `jpx_enumerated_spaces` 16 of 16 and the module's 9 of 9;
rustdoc `-D warnings --document-private-items` on both crates 0; `cargo test -p conformance`
exit 101 on two targets not this round's — `documents` on HEAD's own `Q570` and `records` on
siblings' records being written — and `--test evidence --test adr_status --test records` 0 for
this record and both ADRs. Export, one hold: the six arms against HEAD's six arms, 0 of 5 799
digests moved; `raster_golden` 974 held, 0 moved. No `t2-turn_path`: the one loop touched is
`JPXDecode`'s per-image conversion, which no page of the turn gate states.

Duration 7034 s, about 90 tool uses.

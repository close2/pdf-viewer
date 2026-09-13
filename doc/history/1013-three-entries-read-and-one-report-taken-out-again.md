# 1013 — Three entries read, and one report written and taken out again

## §8.4.5 — Table 57's `/FL`, `partial` → `implemented`

`content/ext_gstate.rs` reads the entry in Table 57's own order and discards it, which §10.7.2
permits outright — the answer `i` has had since the interpreter's first commit, now had by the
`gs` route too. `tests/line_parameters.rs::flatness_is_read_and_reaches_nothing` holds that `/FL`
disturbs none of Table 57's other entries, calibrated by making it set the line width (4.0 against
0.5). §10.7.2's row had a sentence saying `/FL` is not read; corrected.

## §9.9.1 — Table 125's `/Length3`, `partial` → `implemented`

`pdf_font::program::with_fixed_content` appends the 512 zeros and `cleartomark` to a `/FontFile`
whose stream states `/Length3 0`, and to nothing else — an absent entry is a malformed file rather
than a statement, a positive one names a portion already carried. Two tests, both calibrated: the
append disabled (0 against 512), and the condition widened to fire on a silence. Eight lines of
sixty-four zeros is recorded as this tree's choice, the clause stating only a count and an
operator while PostScript needs a separator between them.

## §8.7.3 and §8.7.3.1 — Table 74's `/TilingType`, read, counted, **not** reported

`content/pattern.rs` reads the entry and states, under the clause's own sentences, that every value
gets Table 74's value 2. The report the brief asked for was written — an `Unsupported::TilingType`
on codes 1 and 3 — run over `doc/pdf.js`, and taken out again, because the walk measured what it
costs: seven documents on `corpus.rs`'s incomplete list, every one a tiling document, and
`tests/oracle.rs` filters its ratchets over `e.complete`, so `tiling-pattern-box.pdf page 1` and
`tiling_patterns_variations.pdf page 1` would have left its ambiguous buckets with their diagnoses.
ADR 0563's shape one clause over. `examples/tiling_type_census.rs` went in instead: over
`doc/pdf.js`, 955 Type 1 pattern dictionaries, **392 asking for 1, 557 for 3 and 5 for 2** — the
value this tree performs is half of one per cent of them. Both rows stay `partial`; ADR 1031 is
the decision and says what would close them, which is a rasteriser that snaps a lattice.

## Gates

`rustfmt --edition 2024 --check` on the five files touched: 0. `cargo test -p conformance`: 0.
`cargo nextest run -p pdf-model -p pdf-font --lib --tests`: 0, 1497 passed — `--lib --tests`
because a neighbour's example did not compile.
`cargo test --profile gates -p pdf-model --test corpus -- --ignored`: 0, run with the report in
place (the measurement above) and again after it came out.
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-font --all-targets`: 0; `-p pdf-model`: 101, on two
`doc_markdown` lints in a neighbour's `tests/annotations.rs` and nothing of this round's.

# Round 2135 — page one is decided without the embedded file pass

A measure round in the `gwg` worktree on ADR 3234's cost: in an encrypted document whose `/EFF`
differs from `/StmF`, page one's content stream paid a parse of every indirect object.

## What changed

- ADR 3241: `Document::resolve_as`, `get_key_as` and `reached_as` take a `StreamRole`
  (`PageContents`, `Resource`, `FontProgram`, `Appearance`), routes Table 43 never gives an
  embedded file; `stream_filter` gives a stream so reached `/StmF` without the pass. Callers:
  `ContentReader::for_page`, `resource_entry`, `pdf_font::program`, `annotation.rs`.
  `Document::embedded_file_streams_collected` reports whether the pass ran.
- ADR 3242: `linearize::graph_of` groups §F.3.10's embedded file streams by what `/EF` and `/RF`
  name (indirect ones too), no longer by `/Type /EmbeddedFile`.

## Numbers

`cargo test --release -p pdf-model --test syntax_suite embedded_file_scan_launch --
--include-ignored --nocapture`, open plus `pdf_model::interpret` of page one, median of 31, one
sitting, A with the role check disabled: 5 pages 117.1 to 78.4 us, 500 pages 1371.9 to 199.8 us,
5000 pages 12737.2 to 1552.7 us. What still grows is the open's cross-reference table.

## Ledger

7.6.6 and F.3.10 stay `implemented`; their notes state the construction and list the new tests.

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-model -p pdf-transform -p viewer-ui`: fmt exit
0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib --bins
--tests` 5163 run, 5163 passed, difference against the manifest 0 and 0; `cargo test -p
pdf-syntax` 407 passed; rustdoc exit 0; `cd fuzz && cargo check --bins` exit 0; `cargo test -p
conformance` 478 passed, 0 failed. `tools/state.sh --round 2135 launch` exit 101 with 10 figures outside;
HEAD in the same sitting exit 101 with 9, the same instruction figures within 0.02%, and the
tenth `xfa_filled_imm1344e.pdf`'s `peak_anon_mib`, 3.039 against a ceiling of 3.0, inside on HEAD.

## Habit

A cost "paid once" is on the launch path if the first object loaded can trigger it; ask whether
the route to that object already answers the question.

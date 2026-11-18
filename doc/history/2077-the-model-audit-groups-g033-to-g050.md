# 2077 — The model's audit groups G033 to G050 are answered

Session 2077, an audit-fix round in `pdf-viewer-lane4`. Groups G033 to G050 of
`doc/reviews/2038-specification-audit/fixes.md`: 57 findings (0 B, 47 T, 10 D), and the carried B49 and B73.

- **Fixed 56, owner 1 (D15, Q899), argued 0, open 0** in the groups; B49 fixed, B73 left `open`.
- B49, ADR 2988: a rendition's Table 218 `/JS` is an ECMAScript action at every script site. Its `/OP` is
  the fallback only where no runner is supplied, read as the processor's ability, not one script's outcome.
  `pdf-model` `rendition_scripts.rs` is new. `Rendition` gains `script`; `ViewState::runs_scripts` is new.
- D15: §8.7.4.3's note no longer says `/AntiAlias` "owes nothing"; Q899 asks whether a true value is honoured.
- D findings answered in notes: 7.1, 10.4, 10.4.2, 10.8.3 (its partial-status history removed), 13.2.7.4,
  13.2.7.4.4, 14.1, 14.11, I.2, 12.6.4.14 (B49). Comments: `page.rs` (Table 31 token boundaries),
  `appearance.rs` (3D/RichMedia scope), `viewer-core` `notes.rs` (the report comes after the first frame).
- D05's premise no longer holds: ADR 2946's `said_after_drawing` is the viewer's call site.
- Every T test was plant-checked by a Sonnet helper on a scratch copy: each fails on its plant and passes
  restored. T66's partial-tie half, T44's Normal and Multiply modes and T82's BE half were not planted.
- B73 is still `open`. Its line in `fixes.md` gives the design: an image's CTM at `Do` maps Table 269's
  unit square, and the interpreter has to report it.
- No owner audit file was edited. No ledger status changed.

**Gates.** fmt exit 1 on one file not this round's (`named_property_actualtext_spec_audit.rs`, as on
main); clippy `-D warnings --all-targets` 0 after one rerun; nextest of pdf-model and viewer-core 45
failed: the manifest's 46 less B49, none new; rustdoc 0; fuzz `cargo check --bins` 0; conformance as below.

Trap: `cargo fmt -p pdf-model` rewrote another round's owner audit file. Run `rustfmt --check` on your own
files and `cargo fmt --check` for the gate, never `cargo fmt -p` on a shared crate.
Habit: `json.dump` of a hand-kept manifest rewrites every line. Remove an entry textually.

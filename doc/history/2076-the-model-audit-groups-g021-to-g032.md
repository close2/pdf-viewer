# 2076 — The model's audit groups G021 to G032 are answered

Session 2076, an audit-fix round in `pdf-viewer-lane4`. Groups G021 to G032 of
`doc/reviews/2038-specification-audit/fixes.md`: 35 findings (12 B, 19 T, 4 D).

- **Fixed 34, open 1, argued 0, owner 0.** Each line of `fixes.md` names its ADR and tests. The
  carried B49 (rendition `/JS`) and B73 (image `/Measure`) were not reached and stay `open`.
- ADR 2982: an annotation without a handler draws `/N` whatever the pointer does (§12.5.5).
- ADR 2983: FDF `utf_8` and `utf_16` read unmarked strings in that form; `/Status` beside
  `/Pages` is named (Table 246).
- ADR 2984: the `DPart` walk has no depth bound; Issue #612's flat `/DParts` is the normative shape.
- ADR 2985 supersedes ADR 0054 on order: a string destination asks the name tree first.
- ADR 2986 supersedes ADR 2814 section 3: `/PV` and `/PI` scripts run where the arrangement
  gains or loses a page; the open sequence runs the presented page's.
- ADR 2987 amends ADR 1080: a constant-spacing tiling rounds its steps where the matrix snap
  would distort the cell past one pixel (`pdf_render::snap_steps`).
- Without an ADR, each on its clause: page labels inverted per range (`PageLabels::index_of`, no
  4 096-page bound); Table 156's boolean `/A` versus a short array; a JP2 `cdef` kind-2 channel
  is opacity; a page-level output intent without a readable profile no longer borrows the
  catalog's; a field's `/AA` is the terminal field's own (Table 226).
- `inheritance.rs` (G019's file) lost `Inherited::action` and its `actions` map, dead once B62's
  lookup moved to `aform::site::terminal_action`.
- B20 stays `open`: a zero-height pattern `/BBox` must clip and still paint a pixel, which is a
  degenerate-clip rule in both rasterisers, not in `pdf-model`.
- Ledger notes moved: 8.7.3.1, 12.5.5, 12.6.3, 12.7.8.3.1, 14.11.5; no status changed.
- 16 manifest failures pass and left `expected-failures-current.json`; none appeared.

**Gates.** fmt exit 1 on one file not this round's (`named_property_actualtext_spec_audit.rs`, as on
main); clippy `-D warnings --all-targets` 0; nextest of the four crates 50 failed, the manifest's 66
less the 16 this round fixed and none new; rustdoc 0; fuzz `cargo check --bins` 0; conformance 101 on
one of main's five, §14.9.2.2's row (G017); `records` passes with this paragraph written.

Habit: a function a fix leaves dead in another group's file is removed there and named in the record,
because `-D warnings` will not let it stand and a shim to keep it alive would be the clever construction.

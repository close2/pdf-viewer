# 2080 — The renderer and script audit groups G092 to G099 are answered

Session 2080, an audit-fix round in `pdf-viewer-lane4`. Groups G092 to G099 of
`doc/reviews/2038-specification-audit/fixes.md`: 13 findings (5 B, 7 T, 1 D), and the carried B20.

- **Fixed 13, argued 0, owner 0, open 0** in the groups; B20 fixed. The manifest loses 11 entries.
- B39, ADR 3006 (§11.6.2): a stroke's rest and the §8.5.3.2 dots `split_degenerate` takes out of it meet the
  backdrop once. `render-cpu` draws both in white onto a surface-sized buffer and fills once through its
  alpha met with the clip (`scan::own_coverage`); `render-raster` makes them elements of an isolated knockout
  group (`Encoder::one_object`); `render-gpu` draws them in a layer, the dots knocking out the rest. An opaque
  solid colour under Normal keeps the two draws, which composite identically.
- B59, ADR 3007 (§8.4.3.2): `render-raster`'s scalar width needs `path_width · (max − min) ≤ 0.25` device
  pixels besides the 1.01 ratio, and a scalar scene under a non-similarity reads the view (ADR 0702).
- B70/B71, ADR 3008 (Tables 79, 80): `pdf_render::Shading::paints_nothing`, asked by `render-cpu`'s shaded
  fill and stroke.
- B20, ADR 3009 (Table 74 NOTE 1, §10.7.4): no renderer change was owed. Since ADRs 1060/1064 every
  backend's clip region scan-converts a zero-height rectangle as one pixel row, so `tiling.rs` now keeps any
  finite box as the cell clip. ADR 0056's reading is superseded on the point.
- B52, ADR 3010 (A193's reference, `Doc.addLink` Example 2): `Matrix2D` is defined in every realm
  (`engine/matrix2d.js`), each member a stated choice; `fromRotated` uses `getPageBox`'s rotated user space.
  New test `pdf-script/tests/matrix2d.rs`.
- The T findings and D54 were already closed by the audit's own tests and comment changes. Planted and
  failed: T111 (every stroke on the scalar route: 3 fail), T65 (no `/JS` stream read: the stream test
  fails). T53, T37, T104, T126 and T128 were checked by reading.
- No owner audit file was edited. No ledger status changed; the touched notes stay true.

**Gates.** fmt `--check` exit 0; clippy `-D warnings --all-targets` exit 0 after three fixes; nextest of the
six crates 3830 run, 4 failed: the manifest's 3 and `pdf-script::hook`'s
`a_page_shown_and_left_runs_each_annotation_s_visibility_with_its_open_and_close`, which fails with and without
this round's change and is not in the manifest; rustdoc exit 101 on two links in `pdf-script` this round did not
write (`engine/mod.rs:7`, `bridge.rs:1513`), the other five crates clean; conformance 1 failed, this record
before this paragraph, the five audit-file failures gone. Fuzz not owed: no public type changed.

Trap: a `cargo clippy` with `RUSTFLAGS="-D warnings"` after a build without it rebuilds every dependency;
under four lanes that is half an hour. Build the gate's flags once and keep them.

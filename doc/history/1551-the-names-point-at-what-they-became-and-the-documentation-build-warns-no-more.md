# 1551 — The names point at what they became, six tests sit beside their subjects, and the documentation build warns no more

Slot 6 of batch eighty-seven, 2026-10-14, a docs round. ADR 1938; no question. No ledger row moves
status; three notes are re-pointed and two test paths follow their tests (§7.4.6, §7.4.8).

**(a) Premise held; the class was wider.** `knockout_group_elements` became `implicit_knockout_group`
(ADR 1000, `0dde6224`): §11.4.6's note names it, and §11.6.2's says the report is kept where that
function cannot build the group (`Unsupported::CompositedInParts`). `Decode::raw_of` went when the
`DCTDecode` matte began to go through `unpack` (ADR 1268, `ebfe2642`): §11.6.5.2's note says two
routes and names `image::dispatch::matte_through_unpack`. `render-cpu/src/image.rs:85` says
`reduced_images`. "§12.4.2's selection" stood in nine comments, not four — both hosts' `input.rs`,
`viewer-host/src/keys.rs`, `viewer-qt/src/host/tests.rs`, `quorra/window.rs` twice and
`viewer-qt/cpp/window.cpp`; selection has no clause (`select.rs`), so each says "the text selection".

**(b) Built.** The six one-module tests moved into a `tests` module of `ccitt`, `soft_mask`,
`dispatch`, `dct`, `device_scale` and `convert`; ten stay in `image/tests.rs`. Seven functions only
the tests had widened are private again. **The proof**: the old `tests.rs`'s lines against the new
one plus the six modules differ by paths (`super::dct::` → `super::`, `super::decode` →
`crate::image::decode`), imports, the six wrappers, one rustfmt join, one rewrap and one reference
definition; all six pass under their new paths.

**(c) Built** (ADR 1938): 105 rustdoc warnings → 0 — 21 `render-cpu`, 59 `raster-gpu`, 15
`viewer-ui`'s library, 10 `quorra`. **The hypothesis held in part**: `pub(crate)` resolves no
private-item link, since rustdoc's lint reads it as private, so all ten are code spans. 33 lowered
letters escaped `\[t]`, 23 ambiguous names, 13 links to `#[cfg(test)]` items, 15 moved, 4 stale,
7 redundant. `tools/state.sh docs` is the build, in tier 1's block and section 0. **The proof that
nothing moved**: every changed line in the 54 files is `///` or `//!`, but (a)'s three `//` lines.

**Files.** `doc/conformance/ledger.toml`; `crates/pdf-model/src/image/{ccitt,soft_mask,dispatch,dct,
device_scale,convert,tests}.rs`; doc comments in 54 files of `render-cpu` (10), `raster-gpu` (33)
and `viewer-ui` (11); (a)'s nine comments; `tools/state.sh`; `doc/todo/02`; ADR 1938; this record.

**Gates.** `rustfmt --check`, 65 files: 0. Clippy `-D warnings --all-targets` on `pdf-model`,
`render-cpu`, `raster-gpu`, `viewer-ui`, `viewer-host`, `viewer-gtk`, `viewer-qt`: 0. `cargo
nextest run`: `pdf-model` 0 (2 060 passed); `render-cpu`, `viewer-host`, `raster-gpu` 0 (1 193);
`viewer-ui`, `viewer-gtk`, `viewer-qt` 0 (241). `tools/state.sh docs` 0. `cargo test -p
conformance` 0 (436 passed). `--bin pointers` 0, `--bin quotations` 0, neither naming this round's
lines. `tools/batch.sh check` 0. `docs` fails on a `[t]` put back (101); a misquotation planted
in an inline `"\[t]he …"` was unseen, the quotation gate reading blockquotes only. 6 792 s.

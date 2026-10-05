# 1523 — Annex O is eleven parameters, and a `zoom` beyond this reader's reach lands on the bound and is named

Status: accepted. Session 1344. Amends nothing; builds on ADRs 0209, 0250, 0357, 0431 and 0919.
Context: ISO 32000-2 Annex O (§O.2, §O.2.1, §O.2.2); `CLAUDE.md` principles 1, 3 and 5; trap 5.
Code: `crates/viewer-core/src/open.rs` (`Open::apply_parameter`'s `Parameter::Zoom` arm).
Tests: `crates/viewer-core/tests/fragments.rs`'s
`a_zoom_beyond_the_readers_range_lands_on_the_bound_and_is_named` and
`the_xfdf_a_fragment_names_is_imported_like_an_fdf`.

## 1. The annex's parameters are the ones its two tables print

The round's brief listed seventeen fragment parameters. Table Annex O.3 prints five (`page`,
`nameddest`, `structelem`, `comment`, `ef`) and Table Annex O.4 six (`zoom`, `view`, `viewrect`,
`highlight`, `search`, `fdf`). `pagemode`, `toolbar`, `statusbar`, `scrollbar`, `navpanes`,
`messages` and `collab` are another vendor's open parameters and appear nowhere in `doc/md/`'s
ISO 32000-2; `xfdf` is not a parameter, since `fdf` names "an FDF or XFDF file". Under principle 5
those seven are evidence about what readers do, not requirements, and **they are not built**. A
fragment carrying one is not silent: `Fragment::parse` names what it could not read and the rest
of the fragment runs, which §O.2's "processed and (if required) executed from left to right" asks
and nothing in it lets one parameter cancel another. A later round that wants them builds them as a
documented choice of its own, against Table 29's `/PageMode` and Table 147's viewer preferences, and
writes the ADR that says so; it does not cite this annex for them.

## 2. A `zoom` outside 2% to 6400% lands on the nearer bound, and the fragment is told

Table Annex O.4 gives "the percentage to which the document should be zoomed" and states no
bound. `ZOOM_RANGE` is the magnification a person can reach from the keys, and it is a bound for
the pixel budget's and an `f32` transform's sake rather than taste. A fragment's `zoom` is clamped
by the same bound — one range whoever asks — and the clamp is now **reported**: a URI that asked
for 10000% and opened at 6400% has otherwise been answered with a different number in silence,
which trap 5 forbids. The note names the asked percentage and the range. Zero stays "leave the
magnification alone", Table 149's reading for `/XYZ`.

## 3. An absolute `fdf` URI is refused out loud until a level decides it

Both formats `fdf` names are read and imported (ADR 1108's XFDF reader, tested end to end through
the fragment). An absolute URI is refused by `viewer_host::resolve_import` with a sentence; that
refusal is not yet a level, which principle 3 calls the thing to avoid. The build is designed in
`doc/todo/39` — `Submissions`' four levels, a `may_fetch_import` beside `may_submit`, the submit
client's GET — and not taken here. The row stays `implemented`: the annex's row describes the
import, and every URI a host can resolve without the network is imported.

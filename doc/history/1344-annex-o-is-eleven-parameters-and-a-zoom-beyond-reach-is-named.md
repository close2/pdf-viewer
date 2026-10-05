# 1344 — Annex O is eleven parameters, a zoom beyond reach is named, and the XFDF is imported

Batch fifty-three, the ledger slot. ADR 1523 (ADR 1524 not needed). No question written. No row
changed status.

**The premise was stale twice.** `tools/state.sh annex-o` before: all eleven parameters carried
out, none reported; all five Annex O rows `implemented` (since ADRs 0209 to 0431 and 0919). And
the brief's seventeen parameters are not the annex's: Tables Annex O.3 and O.4 print eleven, and
`pagemode`, `toolbar`, `statusbar`, `scrollbar`, `navpanes`, `messages` and `collab` appear nowhere
in ISO 32000-2. `xfdf` is no parameter: `fdf` names "an FDF or XFDF file". Under principle 5 the
seven were not built. A fragment that carries one names it, and the rest runs (ADR 1523 section 1).

**What was really left.** Two sentences were false. The O.2.2 note and `doc/todo/39` both said XFDF
was declined for want of an XML parser, but `pdf_model::xfdf` has read it since ADR 1108.
Both now say what is. No test imported an XFDF through the fragment, so
`the_xfdf_a_fragment_names_is_imported_like_an_fdf` now does. A `zoom` outside `ZOOM_RANGE` was
clamped in silence (trap 5). It is now named, and
`a_zoom_beyond_the_readers_range_lands_on_the_bound_and_is_named` checks 10000% landing at 64.0 and
1% landing at 0.02 (ADR 1523 section 2).

**`fdf` over the network**, design only: `doc/todo/39` lays it out under `Submissions`' four levels.
A `may_fetch_import` on `may_submit`'s shape, the submit client's GET, the bytes crossing as
`Command::Supply`. The fragment's origin is not consulted, and the confined window refuses.

**The drive.** Two steps were added to `tools/drive-windows.sh`, which is 1340's script (13 lines,
before the Arabic find): `30-fragment-page` (`drive.pdf#page=3`, title `page 3 of 3`) and
`30-fragment-search` (`#page=3&search=%22drive%22`, the trace's found line). Driven on `:144`: all three windows `works` on both steps, and the whole run is 97 works, 0 wrong, 4 not offered.

**Gates.** fmt 0. clippy `-p viewer-core -p viewer-ui` 0. nextest on the same two crates: 450/450,
exit 0. `fragments.rs`: 21/21. `cargo test -p conformance` 365 pass, 1 fail: `state_sections`
names `render-raster/tests/turn_path.rs` as ignored with no gate line, which is a sibling's. Tier 2
behind the lock: `selection_census` exit 0, 974 documents in 9.3 s, find 997/1009.
`accessibility_census` exit 101: "elements a caret reaches, whole population" is 121427 against a
floor of 121463. The census opens with `fragment: None`, so my arm is unreached, and the drop is in
the uncommitted `pdf-model/src/content/` diff. `launch_path` exit 0, 26 figures banded, 0 outside.

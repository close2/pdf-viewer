# 2006 — A font the window drew as nothing is embedded drawing nothing

Session 2006, a single round, PDF/A lane's worktree. `doc/pdfa-blocking-issues.md` section 3, R09 and the
cut-program paragraph; ADR 2830; Q837; trap 334 (round 2005's, by the brief).

**The premise.** Held. At round 2005's binary `issue11651` refused under the six profiles withholding
`machine-face` at both parts, and its `as-if-printed` and `state-every-loss` outputs (CPU, 72 dpi,
`/OutputIntents` renamed) drew "Hello world Test" in Liberation Sans where the window drew none (618 px). ADR
2636's route drops a cut or unparsed program and embeds the shipped face; every document it reaches drew text
the window did not (`178360`: all 24 pages). No clause forbids the window's nothing: §9.1 and §9.8.1 permit a
substitute for an unavailable program and require none (Q837 asks whether the window should draw one).

**Built.** ADR 2830: where the window cannot load a cut or unparsed program, a simple font keeps ADR 2636's
route and every glyph a shown code reaches in the face written is emptied (`pdf_font::embed::
with_glyphs_emptied`: `glyf` given no bytes, `CFF ` rewritten as width and `endchar`); a composite font goes
to ADR 2824's empty glyphs under a `/CIDToGIDMap` stream, which `fonts/cid-to-gid-map-present`'s `Identity` no
longer overwrites (it read the new stream through the source as absent; veraPDF failed that output, this
tree's checker passed it). Proved: no outline, no substitute, the window's advance bit for bit. The loss
`unreadable-program`: `as-shown` by a per-loss key, `as-if-printed` and `state-every-loss` by the bundle, the
other five stop (a first build missed bare `CFF ` faces, whose header `sfnt_tables` read as a directory).

**Freed** under `as-shown` at 2b and 4: `issue11651`. **Changed**, now drawing the window's nothing: `178360`,
`435321`, `504076`, `bug1050040` (2b, 4), `268346`, `020747`, `477139` (4) under `as-if-printed` and
`state-every-loss`, `bug1050040` under `as-shown` too; `no-data-loss` now refuses `bug1050040`, whose output
drew text the window did not under no loss word. `issue13316_reduced` moves from R09 to R08 and R11: the route
still asks the shipped face for every code it then empties. R09 12 to 10 at each part. **Left of section 8**:
line 7 frees no document (its witness a plant); the `described` fonts also lose `layout-from-this-viewer`.
**Pixels**: `issue11651` 0 px (was 618); `178360` 23 of 24 pages 0 px, page 24 a photograph as before;
`268346` page 3 0 (was 267); `020747` pages 1-3 0 (were up to 10 336); `504076` pages 2-3 0; `bug1050040` 0
(was 873); `435321` page 1 70 079 to 25 279 px; the rest is photographs and other fonts' stretch, as before.

**Gates.** fmt 0 (`pdf-font`, `pdf-transform`); clippy `-D warnings --all-targets` 0 (101 first: `row` at 101
lines); nextest `pdf-font`, `pdf-transform`, `pdf-archive` 0 (1905 run, 25 skipped), `pdf-transform` again
after the map fix 0 (1174); seven tests each fail under a plant; `fuzz` `cargo check --bins` 0; archive tier 0
and 0; censuses against round 2005 (`cmp.py`): shipped 2 NEW, 4 GONE, 45 CHANGED, stating 6 GONE, 42 CHANGED,
of which this round's are the lines above and the rest carry no trace of it (HEAD's page tree and annotation
merges); veraPDF 1.30.3: the 16 outputs and 29 census outputs PASS; `rank.py --table`: sections 2 and 9
replaced; `cargo test -p conformance` 0 after `git add -N`; `tools/state.sh --round 2006 docs` 0; tier 2
`corpus` 101 and `raster_golden` 101, each failing as round 2008's runs at HEAD do (the same 23 first pages
moved; `corpus` joined by `issue15053`, `mixedfonts`): no difference.

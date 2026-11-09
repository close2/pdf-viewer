# 1971 — A TrueType code on no glyph is given a glyph that draws nothing

Session 1971, a single round, PDF/A lane's worktree. R03a, one construction (ADR 2753).

**The premise.** Half. R03a's four documents freed under `as-if-printed` were three it alone
refuses — `issue14821`, `issue17333`, `issue11768_reduced` — and `507676` (with R08 and R11). The
first two are no new program kind: simple TrueType fonts ADR 2543 already extends, refused because
the code that fails section 6.2.11.4.1 reaches *no* glyph (§9.6.5.4's route: `issue17333`'s code 0
under `MacRomanEncoding` and a (1, 0) subtable starting at 165; `issue14821`'s subset under an
`/Encoding` that is a content stream), not glyph 0. `issue11768_reduced` is an `Adobe-Japan1` font
whose own `UniJIS-UTF8-H` reads `e0e0e0` as invalid (§9.7.6.3) and sends it to CID 0, with no
`ToUnicode`. `027613`, `268346`, `477139` and `504076` (as-shown) carry other refusals.

**Built.** ADR 2753: a shown code (the survey's, complete) that a simple TrueType program reaches no
glyph for, and that this reader draws no mark for, is given a glyph with no outline — a `loca` entry
spanning no bytes — at its width, one per width, beside ADR 2543's copies of glyph 0; the `cmap` is
restated to it. `notdef_glyph::with_glyphs_given` appends either kind (`NewOutline`); a shown code on
no glyph drawn with a mark keeps the font out of `notdef_given`. The stated sentence now names the
other reader's loss: one that drew `.notdef` for such a code draws nothing.

**Freed** at both parts under all nine profiles but `factur-x` (which declines every one of these
sources): `issue14821`, `issue17333`. **Pixels** (CPU, 72 dpi): `issue17333` 0; `issue14821` five
pixels one level apart, the same with `/OutputIntents` renamed, in rows the substituted Helvetica
draws; `text1` draws `t   t` in source and output alike.

**Tests.** `archive/truetype_no_glyph.rs` (two: one width, two widths; a code on glyph 0 beside),
each asserting `settled` empty; `notdef_glyph::a_glyph_given_nothing_spans_no_bytes_at_its_own_advance`.
Plants: the empty glyph written as glyph 0's outline fails all three; no code on no glyph given one
fails both integration tests (the output still fails three rows).

**Census** (against round 1970): shipped and stating each 8 NEW (the two on all four lines), 0 GONE,
0 CHANGED. R03a frees 2 / 2, stops 6 / 3. veraPDF 1.30.3 passes 4 CLI and 16 census outputs.

**Left of R03a**: `issue11768_reduced` (a fresh CID needs a codespace range the CMap does not state,
and §9.10.2 reads it as a Japan1 character: ADR 2622 section 2); the rest are refused elsewhere too.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-transform; nextest pdf-transform 0
(1127 passed, after an incremental-compilation ICE re-run with `CARGO_INCREMENTAL=0`); archive tier
0; both censuses 0; `cargo test -p conformance` 0 (471 passed) after `git add -N`; `tools/state.sh --round 1971
docs` 0.

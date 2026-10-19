# 1592 — An `xmp:Rating` is held to the 2005 Integer, and a destination profile's header is restated

Slot 1 of batch ninety-four, 2026-10-19, an archive round. ADRs 2020, 2021; no question.

**Premises and hypotheses.** The `over` premise held: one veraPDF pass file at 2b, section
6.6.2.3.1 — `6-6-2-3-1-t07-pass-m`, named once `tests/corpus.rs` printed `over`'s names as it
prints `missed`'s. Its hypothesis did not: no schema was missed. The file states `xmp:Rating` `1.0`;
the 2005 edition types the property a closed choice of Integer and its Integer admits no decimal
point (its Real does), and TechNote 0010's A020 holds a value to its type alone. The validator
stands; the corpus row is `SpecAgainstTheCorpus` (ADR 2020). The census premise held: the two ICC
rows share one witness, `6-2-3-t01-fail-d` (a 2b and a 4 copy), and so did its hypothesis — a v2
Adobe RGB (1998) display profile under a version 5 header, the header alone failing both rows.

**Built.** ADR 2021: `pdf_archive::icc_written_in_the_1998_edition` (ICC.1:1998-09 clause 6.4's
forty-five tags with clause 6.5's type signatures, from `doc/ICC-1_1998-09.pdf`; private tags
passed over; clause 6.3's required tags); `archive/destination.rs`'s `DestinationProfileRestated`
answers both rows as a statement — version 5 written 2.2.0, bytes 84 to 99 cleared, where the
profile is in 1998's terms; a stale `Profile ID` written as the digest — and refuses by name a tag
short under its own edition, later types, and CIELAB tables (the version decides their encoding).
`prepare::the_destination_object` is ADR 2007's one-object rule, extracted for both rewrites. A448:
the site's sentence is in `decision.rs`, the catalogue and `doc/todo/72` §3.

**Pixels.** `6-2-3-t01-fail-d`: 2b and 4 outputs, page byte-identical at 72 dpi before and after
(looked at: the green invader), veraPDF 1.30.3 compliant where the sources fail 6.2.3. Fixtures:
the shipped sRGB under a v5 header converts with the display list equal.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-archive -p
pdf-transform --all-targets` exit 0; their nextest 828 passed, 0 failed; `cargo test -p
conformance --no-fail-fast` exit 0, 400 passed (`state.sh docs` not owed: render-cpu, raster-gpu,
viewer-ui). `tools/state.sh --round 1592 archive` exit 0: `--remedy-sites` 2b 28 of 71 not built (30
of 73), 4 26 of 61; `t3-archive-val` 2b `over` 0, `settled` 8 (7); `archive_corpus` 2b 489
converted, 122 refused (488/123), 4 229/130 (228/131); `cross_check` 0 disagreements; pdf.js and
corpora census 1 450 of 1 477, no `not-built-yet` site reached at 2b. veraPDF census `--tree 6` exit
0: 2b 28 unbuilt (30). One `--clock` hold, 154 s: `t3-foreign_corpus` exit 0, 0 panicked.

**Unfinished.** The census's next site over the veraPDF corpus unfiltered is
`graphics/halftone-type-is-one-or-five` (2, `6-2-5-t03-fail-a`'s two copies: types 6, 10 and 16),
then `no-halftone-phase-in-a-graphics-state` and `pdfa-output-intent-states-a-destination-profile`
(1 each); the first is ADR 1965's question for a whole halftone and needs its own ADR. 5 340 s.

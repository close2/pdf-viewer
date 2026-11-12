# 2000 — Every finding of the specification audit is answered

Session 2000, a single round in two parts. The owner, 2026-11-11: "Please use a round to fix all findings" of
`doc/reviews/1983-specification-audit.md`. ADRs 2811-2820, Q834; detail in its directory's `fixes.md`.

**The subject.** The audit branch merged with `--no-ff` (one conflict, both sides kept): 72 failing tests in
`expected-failures.json` and 112 prose findings, numbered in `fixes.md` section 3. The first part split them across
six helpers by area, each checking its findings against `doc/md/` and the errata; the second fixed prose items 14,
21, 55, 100 and 111 and the open problems, and ran the gates. One audit test did not compile on the merged tree
(`viewer-ui/tests/outline_count_audit.rs`: main's 8116c887 added `Content::external_files`); it gained that field.

**Decisions.** ADR 2811 (bitshift zero fill, stitching's degenerate bound), 2812 and 2817 (model core: Type 1 in
OpenType, knockout mask groups, d1 glyph colour, `/Decode` length), 2813 and 2818 (press defaults to invert, printed
sheet scale, annotation geometry, written appearances, the struck §12.5.6.18 sentence), 2814 (views, triggers and
click targets), 2815 (Annex I warns of saving, insertion raises the version, whole dates), 2816 (MaxLen and commits
in every control; FDF options, pages, scripts; no-`/V` as a choice with §12.7.6.2's reason), 2819 (an integer
`/Count` other than the leaves is restated by the converter and refused by `optimize` and `redact`), 2820 (pattern
resources, associated-file dates, Annex Q read at the painting operator).

**Owner's (Q834).** The model-side MaxLen cut, `/Enforce`'s default level, Redact `/RO`, Annex I's first error.
**Left.** In `fixes.md` section 4: Table 196's mixed order beyond a link's `/U`; `/PV` and `/PI` scripts; run-time
§7.10.5.3 faults below a report; §9.9.2 subset tags across one file's writers; FDF templates naming another file;
prose items 30 (ADR 1106) and 52 (ADR 0304) as readings.

**Tier 2 against HEAD.** `corpus`: 61 incomplete where HEAD's run had 62; `issue19389.pdf` left the list (its
widget's stored `/Resources` now name the `/DA` font, §12.7.4.3, ADR 2818 section 5); the two font-face joiners are
HEAD's. `raster_golden`: 23 first pages move, HEAD's 16 and seven more: `annotation-squiggly-` and
`annotation-underline-without-appearance` (§12.5.6.10's QuadPoints corners, ADR 2818 section 4), `issue19634` (Table
111 ignores a `d1` glyph's colour parameters, ADR 2812), `issue19389` (above), `bug1703683_page2_reduced` (§7.10.4's
stitching), and `bug1538111` and `issue14565` (the underline's centring, ADR 2818 §4's choice).

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 over the sixteen crates touched; nextest of the nine 0
(5726 of 5726, so `expected-failures-after-2000.json` lists none); nextest of the other seven touched 1656 of
1657, the one `render-raster::gpu_lock` timing test at a load of 250, 7 of 7 alone; `xvfb-run -a cargo test -p
viewer-gtk --lib` 0 (20 of 20); `cd fuzz && cargo check --bins` 0; `cargo test -p conformance` 0;
archive tier: `pdf-archive` corpus 0 missed and 0 over at all six targets, `archive_corpus` 0, the archive walk 0;
`script_corpus_model`, selection and accessibility censuses 0; `state.sh --round 2000 docs` 0; the transform gate
101 as HEAD's (24.0 pages/s warm against HEAD's 19.4–20.6 at the same load, floor 40).

**Habit.** A merged branch older than main can fail to compile its own tests (a struct literal meets a new field).

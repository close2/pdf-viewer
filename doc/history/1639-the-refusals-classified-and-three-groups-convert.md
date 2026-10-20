# 1639 — The refusals the corpus still meets, classified, and three groups convert

Slot 1 of batch one hundred, 2026-10-20, an archive round. ADRs 2114, 2115; no question.

**Instrument.** `archive_corpus`'s `the_shipped_profiles_list_every_refusal_by_site_and_reason`: per
profile and target, each refusing requirement, its kind of *no* (`output-fails`: the second verdict).

**Refused documents under `as-if-printed` (`as-shown`), by the lowest group each reaches**; 2u 8, 2a
8, 4f 2 and 4e 3 unmoved under both, and `doc/todo/72` section 4 names each group's sites.

| group | 2b before → after | 4 before → after |
|---|---|---|
| 1 colour family beside the profile in hand | 25 → 7 | 37 → 15 |
| 2 a remedy RFC 0007's shapes can build | 17 (21) → 2 (6) | 0 (1) → 0 (1) |
| 3 converter defect | 1 → 0 | 1 → 1 |
| 4 the standard forces it (fence, 6.1.13, external data, header) | 49 → 49 | 48 → 48 |
| 5 a loss both profiles leave at `stop` | 8 (6) → 8 (6) | 6 (8) → 6 (8) |
| 6 `not-built-yet` sites of one to three witnesses | 21 → 21 | 24 → 24 |
| total | 121 (123) → 87 (89) | 116 (119) → 94 (97) |

**Premises and hypotheses.** 119 and 116 held, under every loss authorised; the hypothesis did not:
none is unreadable or locked — veraPDF's built-to-fail files, the fence the largest group.

**Built.** ADR 2114: `DeviceRGB` — or a group, base or alternate in it — beside a CMYK destination
profile takes a `/DefaultRGB` over the shipped sRGB, both parts; ADR 1153 had counted no second RGB
licence. ADR 2115: an invoked PostScript XObject becomes an empty form, its name kept; the regression
net is tested in `archive::tests`. The catalogue's `discard` for a container missing a field the file
states nowhere: it goes with the properties it described, `metadata-property`, each named and recorded.

**Pixels and veraPDF.** The 56 witnesses that moved: veraPDF 1.30.3 fails 55 sources and passes all
56 outputs (`6-6-2-3-3-t05-pass-a` lacks `pdfaSchema:property`, which ISO 19005-2 section 6.6.2.3.3
requires and A029 does not relax). At 72 and 300 dpi, 42 byte-identical, 14 within one level.

**Gates.** `t3-foreign_corpus` `--clock` exit 0, 140 s; `cargo fmt --check -p pdf-transform` exit 0;
`-D warnings` clippy `--all-targets` exit 0; nextest 587/0 failed; conformance 420/0 failed.
`tools/state.sh --round 1639 archive` exit 0: `--remedy-sites` 2b 23 of 68, 4 17 of 56; validator
`missed` 0, `over` 0 every target; `archive_corpus` 2b 525/86, 4 265/94; `cross_check` 0 disagreements.

**Unfinished.** Group 1's CMYK half (ADR 2114 §4); the part 4 action the removal misses
(`6-6-1-t01-fail-m`); the container's narrower cut; pdf.js and `doc/corpora` unswept. 6 100 s.

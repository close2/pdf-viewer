# 1157 — The second profile, and the page one intent cannot license

2026-09-21. Files: `data/icc/GRACoL2006_Coated1v2.icc` and `PROVENANCE.md`, `NOTICE`,
`doc/third-party-data.md`, `doc/pdf-a-conversion-limits.md`, `pdf-transform/src/archive/`
(`prepare.rs`, `decision.rs`, `mod.rs`), `bin/quorra-transform.rs`, `tests/archive.rs`,
`pdf-colour/src/icc.rs`, `pdf-archive/src/table/graphics.rs`, ledger §14.11.5 and §8.6.5.5, ADR 1153.

## What the owner decided and what was left to decide
"perfect. use it." ships `GRACoL2006_Coated1v2.icc` as the default CMYK output-intent profile in
`A18`'s shape. What that does not settle is **when**: ISO 19005-2 and -4 section 6.2.3 allow one
destination profile object per `OutputIntents` array and section 6.2.4.3 of each licenses a device
space only through a profile of its own family, so a page painting in `DeviceRGB` and `DeviceCMYK`
cannot be licensed by output intents at all. **The rule taken**: the CMYK profile where the document's unlicensed device colour is CMYK and
none of it is RGB, the RGB profile otherwise, read off the validator's own four rows. The tie goes
to RGB because only CMYK has a **second** licence — part 2's `DeviceN` `/DefaultCMYK`, NOTE 2 — so
under part 2 this order licenses a mixed page completely and the reverse cannot. Part 4 states no
second licence, so a mixed page keeps one refusal and gets `WRONG_FAMILY`'s sentence naming the
flag. Different documents are different conversions; a form XObject makes its *page* mixed
(§8.10.1, and the survey attributes its colour to that page). Part 4's page-level intents are
declined with the clause: available only with no document-level intent, then required on every
device-dependent page, and still leaving the mixed page refused.

## Measured, before and after, under the lock
`archive_corpus`, at the base commit and again after.
`graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent` at PDF/A-4:
**19 refused → 8**. PDF/A-4 overall 177/182 converted/refused → 188/171; at default authorisations 150/209 →
161/198. Two cross-checks: the part 4 **RGB** row is 16 both times, so nothing lost its RGB licence
to the new profile; and **no part 2 target moved at all** — ADR 1105's precedent was that an RGB
intent converted none of its witnesses because each already held a profile, and the part 2 CMYK row
is quiet here for a different reason, that the `DeviceN` default already answered it. What changes
under part 2 is the answer's quality: a CMYK-only document now takes the profile instead of
§10.4.2.5's transform, which §10.4.2.1 calls a crude approximation.

## Calibration, and one brief error
The decision was planted both ways (trap 13): forced to sRGB the two new CMYK tests fail, forced to
CMYK four others do including the mixed-page one. The shipped file's own edition test carries a
plant inside it — `A2B0`'s signature struck — so `missing_required_tags` has to name what is missing
rather than pass a profile with neither of clause 6.3.3's transforms. And `doc/pdf-a-mitigations.md`
has **no** entry for the device-colour site: it catalogues `not-built-yet` rows and that one is
built, so the "operator must supply a profile" claim lived in the limits document alone.

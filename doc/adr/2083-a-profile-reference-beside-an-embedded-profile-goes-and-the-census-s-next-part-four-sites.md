# 2083 — A profile reference beside an embedded profile goes, and what the census ranks next at part 4

Session 1623, the archive slot of batch ninety-eight. Status: **accepted**, **built** in
`pdf-transform`. Context: ADRs 0948, 0965, 0982, 2021, 2035; `doc/pdf-a-mitigations.md` section 4.1.
Code: `crates/pdf-transform/src/archive/sites.rs` (`profile_references`),
`Rewrite::DestinationProfileReferenceRemoved`, the two `REMEDIES` rows in `decision.rs`.

## 1. The census, read before the build

`examples/archive_unbuilt` over `doc/veraPDF-corpus`, `doc/pdf.js/test/pdfs` and `doc/corpora`
(4 358 of 4 385 documents opened) ranks **no `not-built-yet` site with a document at 2b, 2u or 2a**:
every site it counts there is `the-fence` or `not-this-target`. At part 4 it ranks four:
`graphics/no-icc-space-duplicating-the-output-intent-profile` (6, veraPDF's `6-2-4-2-t03-fail-a`
to `-f`), `graphics/icc-profiles-conform-to-the-base-standard` (2, outside the veraPDF part 4
folders), `graphics/separation-alternate-space-does-not-duplicate-a-current-profile` (2,
`6-2-4-4-t01-fail-i`, `-j`) and `graphics/no-destination-profile-reference` (1,
`6-2-3-t04-fail-a`). The brief's hypothesis — small sites, each a loss with a `preserve` answer —
held for none of them.

## 2. The duplicate-profile rows are not taken here, and why

The two duplicate-profile rows are the catalogue's *mitigation none*, argued in ADR 0965 and held in
ADR 0982: writing `DeviceCMYK` over an `ICCBased` space whose profile is the output intent's settles
§8.6.5.7's implicit conversion, which the base standard leaves to the processor, and §8.6.7's
overprint mode turns on exactly that. A conditional statement in ADR 1978's shape — the
substitution where no mark in the space is painted with overprinting on — may be available now that
`DeviceCMYK` inside an `ICCBased` CMYK group is that group's space, but it is a reversal of two ADRs
and is its own round's argument, not a site to build on the way past. **Measured for that round, by
hand and not built**: `6-2-4-2-t03-fail-a` (the output intent's profile) and `-c` (the group's
blending space) with the one resource array written `/DeviceCMYK`, padded so no offset moves, draw
byte-identically to the source at 72 and 720 dpi and pass veraPDF 1.30.3 at PDF/A-4; neither paints
with overprinting on. The siting half stands as ADR 0965 put it — the findings name a page — so the
build owes a population that is not a second content walk. The base-standard row needs its two
witnesses named first. All three stay `not-built-yet`.

## 3. `DestOutputProfileRef` beside an embedded profile is a statement

ISO 19005-4 section 6.2.3 forbids the key in every output intent; ISO 19005-2 section 6.2.3 in a
PDF/X one. Where the same intent embeds its `DestOutputProfile`, the profile its colours are
referred to is in the file; ISO 32000-2 §14.11.5 says the data in an output intent "shall be for
informational purposes only", and Table 401 asks nothing of a processor about the reference. This
reader reads no reference. So the key is removed under both rows as one `Stated` rewrite, and what
goes is the producer's description of where further copies of a profile could be found. Refused by
name: an intent whose reference is its only profile (the external-data case, not built for an
output intent), a reference whose `URLs` lists an embedded file specification (Table 402 admits one,
and removing the reference would orphan that file), and an intent written as a direct dictionary.

## 4. Evidence

`6-2-3-t04-fail-a` converts to 4 and 4e, veraPDF 1.30.3 compliant (failing 6.2.3 test 3 before),
its page byte-identical at 72 and 720 dpi; 4f refuses it at Annex A.2 (no embedded file), as
before. Three `pdf-transform` tests: the removal, the reference that is the only profile, the
reference listing an embedded file.

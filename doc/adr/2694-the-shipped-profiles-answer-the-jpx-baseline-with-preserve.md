# 2694 — `as-if-printed`, `as-shown` and `only-metadata-loss` answer the JPX baseline with `preserve`

Session 1940, a single archive round.
Status: accepted and built: `doc/profiles/as-if-printed.toml`, `as-shown.toml` and
`only-metadata-loss.toml` (four entries each); the sentences in `pdf-transform`'s
`archive/config.rs` (`in_place_preservation`), `archive/transcode.rs` (`TRANSCODED`) and
`archive/decision/colour_and_graphics.rs` (`JPEG2000_BASELINE_NOT_REACHED`).
Context: ADR 1400 section 4 (amended here), ADR 1412, ADR 2484, ADR 2490 (R13), ADR 2695.
Clauses: ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3; ISO 32000-2 §7.4.9.

## The premise, checked

ADR 2490's rank called R13 "unbuilt". It was built: ADR 1400's `preserve` transcodes a JPEG 2000
image outside the baseline whose dictionary states its colour space, and `no-data-loss`,
`state-every-loss` and `keep-everything` answer all four of its sites so. Round 1939's census
converts R13's three documents under both stating profiles at 2b and 4. What refused them was the
two shipped profiles, which listed `graphics/jpeg2000-*` among the sites they leave at `stop`
("the image printed", "the image shown"), following ADR 1400 section 4's reason for making the
transcode an operator's answer.

## Decision

1. `as-if-printed`, `as-shown` and `only-metadata-loss` answer the four sites the transcode answers
   (`jpeg2000-uses-the-baseline-feature-set`, `-bit-depth`, `-channel-count`,
   `-no-ciejab-colour-space`) with `preserve`.
2. ADR 1400 section 4 is amended. Its two costs are not losses: every sample decodes as it did, so
   this reader draws the page as it did, and no profile's loss vocabulary has a word for either.
   The first, the file's growth, is the optimiser's opposite and no profile counts size. The
   second, this tree's decoder's output standing for the producer's codestream, is exactly what
   `as-if-printed` and `as-shown` are named for — the page as this reader prints it and shows it —
   and for a bare codestream it no longer arises (ADR 2695).
3. `refuse-any-loss` keeps the refusal: it answers no site at all, which is its statement. A
   dictionary stating no `/ColorSpace` keeps the refusal under every profile (ADR 1400 section 2).

## What it costs

Under the three profiles a document whose JPEG 2000 image is outside the baseline in another way
than a bare codestream is written larger, by as much as its samples are uncompressible; the report
names each image with its size before and after.

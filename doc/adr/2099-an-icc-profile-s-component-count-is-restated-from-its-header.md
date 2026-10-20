# 2099 — An `ICCBased` profile's component count is restated from its header

Session 1631, the archive slot of batch ninety-nine. Status: **accepted**, **built** in
`pdf-transform`. Context: ADRs 0948, 1990, 2006, 2083 §1; `doc/pdf-a-mitigations.md` section 4.1.
Code: `crates/pdf-transform/src/archive/component_counts.rs`, `Rewrite::IccComponentCountRestated`,
`pdf_archive::icc_profile_component_count`.

## 1. The census, read first

`examples/archive_unbuilt --who graphics/icc-profiles-conform-to-the-base-standard` over
`doc/veraPDF-corpus`, `doc/pdf.js/test/pdfs` and `doc/corpora` (4 358 of 4 385 opened) names two
documents at 4, 4f and 4e, both outside the veraPDF part 4 folders: `ISO 32000-1/6-2-3-2-t01-fail-b`
(an RGB profile under `/N 4`, painted with four operands) and `Undefined/6-2-3-2-t01-undefined-a`
(a profile stream with no `N` at all). Both are the requirement's `N` sentence; neither is its
device class or data colour space sentence.

## 2. Decision

ISO 19005-4 section 6.2.4.2 holds the profile to ISO 32000-2 §8.6.5.5, whose Table 66 makes `N`
"[t]he number of colour components in the colour space described by the ICC profile data" and
says "[t]his number shall match the number of components actually in the ICC profile". The number
is in the file: the header's data colour space, counted by §8.6.5.5's table of ranges — the
requirement's own test, now `pdf_archive::icc_profile_component_count`. And it is what this reader
drew: `pdf_colour` draws an `ICCBased` space through its profile wherever the stream decodes whole
and the profile parses, and reads no `N` there. So `Rewrite::IccComponentCountRestated` writes it,
a `Stated` rewrite: a processor that took the stated `N` over the profile no longer has one to take.

Refused by name: a profile this reader does not draw through (it drew Table 65's `Alternate` or the
device space `N` names, which a new `N` could change), and a profile whose device class or data
colour space the clause does not admit, or that has no header — no dictionary entry corrects what
the profile states. Each refuses the file, since a rewrite of the others would leave the row failing.

## 3. Evidence

Both witnesses convert to PDF/A-4, veraPDF 1.30.3 compliant, each page byte-identical to its source
at 72 and 720 dpi (`doc/history/1631`). Three `pdf-transform` tests: no `N`, `/N 4` over an RGB
profile (display lists equal), and a device link profile refused.

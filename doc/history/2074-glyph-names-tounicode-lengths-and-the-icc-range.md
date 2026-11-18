# 2074 — Glyph names by the whole AGL algorithm, `/ToUnicode` code lengths, and the ICC range before the alternate

Session 2074, an audit-fix round in `pdf-viewer-lane3`. Review 2038's groups G009 to G016 (`pdf-colour`, `pdf-font`):
18 findings, 6 B, 11 T, 1 D. Per-finding detail is in `doc/reviews/2038-specification-audit/fixes.md`.

**The premises.** Every B finding held on HEAD: its audit test failed as the manifest says. ADRs 2948 and 2956 were
read first for B50; they are about a name-keyed program's 256-slot vector and do not meet §9.6.5.4's TrueType sentence.

**B15, B16 (ADR 2970).** `text_for` now runs the Adobe Glyph List Specification's four steps: a row of several scalar
values and a `uni` name of several groups keep all of them, and a component no rule maps contributes the empty string.
That supersedes the comment's whole-name refusal; the unit test asserting `f_f_notaglyph` → None now asserts `ff`.
**B50 (ADR 2971).** A nonsymbolic TrueType font's dictionary `/Encoding` fills its undefined slots from
`StandardEncoding`. **B27, B28 (ADR 2972).** `/ToUnicode` statements keep their source code's length, read exactly
where the map writes that length and by value where it writes none. A `bfrange` with a sequence destination
increments the last byte per code, up to 255. **B25 (ADR 2973).** An `ICCBased` space drawn in its `/Alternate` is
`ColourSpace::IccAlternate`, which clamps to Table 65's `/Range` before the alternate clamps to its own.

**T findings, by plant.** Each audit test was run against a plant of the defect it names, in two builds, and failed.
The plants: Separation initial tint 0; `entry_of` floor; palette bytes unscaled; vertex `align` removed; flag 2's edge
reversed; patch order reversed; 32-bit `decode_at` halved; Table 113 row moved; `/Widths` and `/W` members
unresolved; `/Differences` references unresolved; the absent-base fallback as WinAnsi; a direct CID descriptor
ignored; an Encoding CMap parsed without its base. T30 was the exception: its audit test passed with a stream
`/UseCMap` ignored, since its base maps CID 36 to the `A` the program gives anyway. The new
`tounicode_usecmap_stream_base` maps it to U+0416 and fails on that plant. **D43**: `standard.rs` and `substitute.rs`
now name `Courier-BoldOblique` as the clause's (Issue #384), the other spelling as an alias.

**Ledger.** Notes corrected on 8.6.5.5 (the `/Alternate` path), 9.10.2, 9.10.3 and 9.6.2.2; statuses unchanged.
**Gates.** Over `pdf-colour`, `pdf-font`, `pdf-model`, `pdf-transform`, `viewer-core`: fmt exit 1, only on the audit's
`named_property_actualtext_spec_audit.rs` (as on main; not edited); clippy `-D warnings --all-targets` exit 0;
nextest `--lib --bins --tests` 5279 of 5362 passed, 83 failed, exit 100: the manifest's entries less the nine this
round fixed, none new (`expected-failures-current.json` updated). Conformance exit 101 on main's five. Fuzz
`cargo check --bins` exit 0. Rustdoc `-D warnings --document-private-items` of the three crates exit 0.
`raster_golden` was queued 69 minutes behind a clock run and stopped before its grant: not run, so no page is
named as moved; the train owes it (a page with an unsupported `ICCBased` profile or a `MacExpertEncoding` TrueType base).

**Left.** Nothing in G009 to G016.

# 1151 — A twentieth empty head, and §14.7.5.4's route is dead on 69% of the tagged corpus

2026-09-16. Files: `crates/viewer-core/tests/accessibility_census.rs` (the silence diagnosis),
`crates/pdf-model/src/structure.rs` (one doc comment), ADR 1151, this record. Every other
`git status` path is a sibling's, as are two `-D warnings` errors (`content/pattern.rs:1288`,
`file_spec.rs:575`) and three `conformance` failures (`command.rs:200`, `restriction_levels.rs:6`,
a §11.7.5.2 ledger row). **Oracle head empty a twentieth round** — `1962 pages in 55.1s`,
`agrees 1012, contradicted 46, ambiguous 835` (807 complete), nothing under the undiagnosed head.
So the contract was `accessibility_census`, and its non-reaching and refused documents.

## The 3 refused and the 61 silent, classified
`REFUSED_OPEN` unchanged, all three the document's: `issue21579.pdf` and `encrypted-attachment.pdf`
need a password nobody has recorded (§7.6.4.1); `PDFBOX-4352-0.pdf` states `/Encrypt 6 0 R`
resolving to null where §7.6.2 puts the dictionary. The 61 silent pages were **all reported as "no `/StructParents`" and 2 of them state one**.
`Tree::elements_on_page` answers `None` for two different files and the census asserted the first
cause for both; it now asks the page and says which, 59 / 2. The 2 are the document's:
`bug1365930.pdf p1` roots its tree at a `Document` element with no `/K`, orphaning every real
element; `bug1978317.pdf p1` states 32 768 `Link` elements, each named by its annotation's `/StructParent`
and none stating the `/K` `/OBJR` §14.7.5.3 requires.

## The defect behind the false label — read, measured, **not** shipped (ADR 1151)
§14.7.5.4's parent-tree value for a content stream "shall be an array"; §7.3.10 lets any value be
"labelled as an indirect object", and §14.7.5.4's own EXAMPLE 2 writes it that way. Both array call
sites match `Object::Array` on the **unresolved** entry, so an indirectly-written array reads as
nothing — and **75 of the 109 documents with a structure tree write it that way**,
`structure_simple.pdf` among them, so ADR 0325's page-scoped walk has never run on one of them.
Fixed in a scratch arm, A/B back to back: whole-population elements 351 324 → 227 207, every
content-carrying count unchanged (own marks 191 815, caret 113 098, lines 199 568, characters
5 354 660), the whole drop in *placeless* ones, 130 171 → 8 634 — one tree counted once per page
(`ISO-19444-1-2019-preview.pdf`, 131 884 → 8 179 over 19 pages). But `prefilled_f1040.pdf` states no
`/StructParent` on any of its 242 widgets, which Table 359 requires, so the page-scoped walk loses
them: tracked controls 272 → 46, annotations 415 → 189, elements 4 060 → 3 648. Shipping the right
parse alone makes a real file worse, so it is named for a round that can argue ADR 0325's walk
against that completeness. Code reverted; no ratchet moved.

## Gates, under the lock, one at a time
`accessibility_census` exit 0 (61 silent, 2 stating the key; 0 named-but-silent, 0 at bound, every
floor slack 0). `render-raster --test corpus` exit 0: `958 pages compared, 943 agree, 8 differ, 7
refused, 16 not comparable` — **8+23 unchanged**. `raster_golden` exit 0 (held 974, moved 0).
`oracle` exit 0. `tools/batch.sh check` flags only 1150's 45-line record.

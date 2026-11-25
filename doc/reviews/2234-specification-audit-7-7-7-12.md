# Specification audit of ISO 32000-2 §7.7 to §7.12 (document structure, content streams and resources, common data structures, functions, file specifications, extensions) — round 2234

The project's fifteenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2229 ([`2229-specification-audit-14-6-14-9.md`](2229-specification-audit-14-6-14-9.md)).
Baseline: main at `6116e221`. Evidence:
[`2234-specification-audit-7-7-7-12/`](2234-specification-audit-7-7-7-12/) — `findings.json`
(2038's schema) and `expected-failures.json` (the failing tests of each behaviour finding). The
same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing** but one documentation slip (D01). A fix round takes the
findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §7.7 to §7.12 (Tables 29 to 49 as `doc/md/` numbers them) were
   read from `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 3037 to 4090, and set against the code
   that reads the entry or performs the step. Most of Table 29's rows delegate to the clause that
   defines the entry and were left to those clauses' audits.
2. 2038's thirty-nine findings on these clauses were read first and are not repeated (among them
   B09 and B10 on functions, B22 on the open action, B45 on Type 4 NUL separators, B48 on page
   tree depth, D41 on the tree depth limit). Errata: `doc/errata/site/docs/32000-2-2020/clause07.md`,
   H7.7 to H7.12 — Issues #439, #242, #106, #105, #348, #614 and #271 (Tables 29 to 31), #214 and
   #307 (name and number trees), #302 (§7.8.2's nesting), #128 (§7.8.3's Type 3 search order),
   #251 (§7.9.4's missing offset), #30 (Type 2's domain), #481, #155 and #156 (§7.11.4), #732,
   #399 and #239 (§7.12). Issue #128 is the clause B01 rests on; #161 is D01's; none settles a
   finding below.
3. The readers traced: `pdf-model`'s `page.rs` (the page tree walk, `Inherited::overlay`,
   `build_page`'s boxes, `/Rotate` and `/UserUnit`), `type3.rs` and `content/text/type3.rs` (the
   glyph's resources), `colourants.rs`, `content/ext_gstate.rs`'s `resources_blend`,
   `optional_content.rs`'s `groups_referenced_by`, `file_spec.rs`, `attachment.rs` (`read`,
   `related`), `action.rs`'s file targets and `requirements.rs`'s `Extensions`; `pdf-syntax`'s
   `tree.rs`, `text_string.rs`, `date.rs` and `version.rs`; `pdf-colour`'s `function.rs` by its
   tests' names and its quotations of §7.10.
4. The lens was the earlier audits' habits, chiefly *every route to a thing* and *every reader of
   a dictionary is a route to it*: §7.8.2 lists four kinds of content stream besides a page's
   (forms, patterns, Type 3 glyphs, annotation appearances), §7.8.3 gives each its own resource
   dictionary, and Issue #128 puts a glyph stream's own `/Resources` first. The interpreter takes
   every one of those routes; the walks that read a page's resources *ahead* of interpretation
   were read against the same list.
5. Each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/resources_7_8_3_spec_audit.rs` (a module of `content_suite`) and
   `pdf-syntax/tests/text_string_prefix_spec_audit.rs`.

What holds, probed by reading, is not listed one by one. Among it: `/Resources`, `/MediaBox`,
`/CropBox` and `/Rotate` inherited whole through `/Parent` and nothing else inherited (`/BleedBox`,
`/TrimBox`, `/ArtBox` and `/UserUnit` read from the page alone, and the transform writers flatten
the same four keys); a null `/Resources` read as absent; `/Rotate` normalised by `rem_euclid(360)`
with a non-multiple of 90 logged; rectangles normalised corner by corner; name-tree keys compared
byte by byte with a shorter key first, `/Limits` narrowing a lookup only below the root, a cycle
and a depth bound both terminating; the text-string decoder's three forms, supplementary
characters through UTF-16 surrogates, and §7.9.2.2.2's escape in both Unicode forms and not in
PDFDocEncoding; §7.9.4's defaults (month and day 1), its ranges, NOTE 2's trailing apostrophe and
a missing offset compared as GMT (Issue #251); `/UF` before `/F` for a name and in `/EF` and
`/RF`; §7.11.2.1's escaped SOLIDUS and §7.11.2.2's `..` resolution; the catalog `/Version` read as
the latest of header and every revision (Issue #399); and PDF 2.0's array of developer extension
dictionaries under one prefix.

Readings left, each a question rather than a defect found: `FileSpec::components` and
`resolve_against` split `/F`'s bytes even where `/UF` is present, which the module's own test pins
as a choice (§7.11.2.1 says the components "shall be passed to the operating system without
interpretation"); no production caller uses either, and the host's import policy works from the
`/UF`-first name. A zero-width or zero-height crop box is read as absent and logged, where §7.9.5's
NOTE says "[r]ectangles can have a width of zero or height of zero" — the silent-repair campaign's
question (ADR 3477), not a clause's.

## Findings

Three behaviour gaps. In order of how much a person would see:

- **2234-B02** — the layers panel's `VisiblePages` list (`groups_referenced_by`) follows a page's
  resources into forms only: a group marked inside a tiling pattern's cell or inside an
  annotation's normal appearance is not offered, though the page paints it.
- **2234-B01** — a Type 3 glyph description's own `/Resources` (Issue #128's first step) is walked
  by the interpreter and by neither of the two walks that read a page's resources ahead of it:
  `spot_colourants` gives a `Separation` space stated there no plane, and `resources_blend` cannot
  see a blend mode stated there.
- **2234-B03** — `encode_text_string` writes a text beginning `þÿ` or `ï»¿` in PDFDocEncoding,
  whose bytes are then the UTF-16BE or UTF-8 prefix; the text reads back as something else.

One documentation finding, fixed: D01, `text_string.rs`'s module documentation spelled NOTE 4's
prefix `¨»¿` where byte 239 is `ï` (Issue #161).

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `7.8.3` (B01, B02),
`8.11.4.3` (B02), `10.8.3` and `11.6.3` (B01), `7.9.2.2.1` (B03).

## Suggested fix groups

1. **`pdf-model/src/colourants.rs`'s `Role::Font` arm and `content/ext_gstate.rs`'s
   `blends_under`** — B01: beneath a Type 3 font, the `/Resources` of each `/CharProcs` stream
   that states one, under each walk's existing bounds.
2. **`pdf-model/src/optional_content.rs`'s `gather_from_resources` and `groups_referenced_by`** —
   B02: a tiling pattern's resources, a Type 3 font's and its glyphs', and each annotation's
   normal appearance streams', under the existing depth bound and visited set. Groups 1 and 2 are
   one walk written three times; a fix round may prefer one shared "every content stream's
   resources beneath these" iterator, which would close both at once.
3. **`pdf-syntax/src/text_string.rs`'s `encode_text_string`** — B03: the PDFDocEncoding form only
   where it does not begin with a Unicode prefix.

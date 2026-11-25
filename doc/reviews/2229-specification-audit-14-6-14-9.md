# Specification audit of ISO 32000-2 §14.6 to §14.9 (marked content, logical structure, tagged PDF, accessibility) — round 2229

The project's fourteenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2197 ([`2197-specification-audit-12-3-12-4-12-6.md`](2197-specification-audit-12-3-12-4-12-6.md)).
Baseline: main at `62782620`. Evidence:
[`2229-specification-audit-14-6-14-9/`](2229-specification-audit-14-6-14-9/) — `findings.json`
(2038's schema) and `expected-failures.json` (the failing tests of each behaviour finding). The
same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §14.6 to §14.9 (Tables 352 to 385 as `doc/md/` numbers them)
   were read from `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 16375 to 17975, and set against the
   code that reads the entry or performs the step. Most are constraints on the file (Table 359's
   "at most one of these two entries", §14.7.5.1.1's nesting restrictions, `ParentTreeNextKey`'s
   increment); those were read for what a reader does when a file keeps them.
2. 2038's thirty findings on these clauses were read first and are not repeated (among them B03
   and B04, the traversal bounds; B12, `/ActualText` once over an element; B30, the empty `/Lang`;
   B43, mixed `/K` order; B56, Warichu). Errata: `doc/errata/site/docs/32000-2-2020/clause14.md`
   for §14.6.1 (Issue #334: NOTE 3 on role-mapping tags deleted, second-class tag names), §14.7.2
   (Issues #93, #396), §14.7.5 (Issue #452's renumbering, 2038-D01), §14.8.4.7 (Issue #133's Link
   subclause), Table 385 (Issue #346's "not inheritable") and §14.8.6; round 2225's restatement of
   §14.7.6 (#226). None settles a finding below.
3. The readers traced: `pdf-model`'s `structure.rs` and its modules (`roles`, `attributes`,
   `owners`, `logical`, `tables`), `content/marked.rs` and `run/marked_ops.rs` (the property list
   read per `BDC`, `/Span`'s entries and the element fallback), `content/text/show.rs` and
   `readback.rs` (`ReversedChars`, `/ActualText`'s cut and word breaks), `accessibility.rs`
   (`speech`), `media.rs`'s §14.9.2.4 search, `optional_content.rs`'s language match and
   `submission.rs`; `viewer-core`'s `accessibility.rs` (the page's walk, `/OBJR`, `/StmOwn`) and
   `scripting.rs`; `viewer-host`'s `panel.rs`.
4. The lens was the earlier audits' habits: every route to a thing (§14.7.5 gives a content item
   three — an `/MCID` sequence, an `/MCR` with `/Stm`, an `/OBJR` — and §14.7.5.4's Table 359 names
   an `XObject`'s own `/StructParent` beside a page's and an annotation's); a rule keyed on a name
   applies wherever the name is read (`/TU` wherever a field is named to a person, `/Lang` wherever
   an outline title or a group name is shown); a NOTE or an exception that defines a scope is a list
   of routes (§14.9.2.3's "except for nested marked-content that is contained in the structure
   hierarchy").
5. Each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/structure_14_6_14_9_spec_audit.rs` (a module of `navigation_suite`),
   `viewer-core/tests/structure_objr_xobject_spec_audit.rs` and
   `viewer-host/tests/outline_language_spec_audit.rs`.

What holds, probed with fixtures, is not listed one by one. Among it: two consecutive `Span`
`/ActualText` sequences with a positional gap read back with no word break; `/ReversedChars` over
the clause's own EXAMPLE and over a right-to-left placement; a `BDC` in the first stream of a
`/Contents` array closed in the second, with its replacement applied across; a role map through two
namespaces' `/RoleMapNS` to a standard type; a standard type remapped by the root's `/RoleMap`
(§14.7.3's "even if the original name is one of the standard types"); `Note` read as no standard
type in the PDF 2.0 namespace; the later of two attribute objects of one owner winning, and `/A`
over `/C`; `/E` separated by a word break from the text after it; text clipped away, drawn in
render mode 3, or placed outside the crop box kept as content (§14.8.2.4); the language of an
element inheriting its parent element's `/Lang` over an enclosing `Span`'s; §14.9.2.4's exact then
prefix match; and the optional content `Language` match ignoring case.

Readings left, each a question rather than a defect found: an extracted show string ending in a
stated SPACE followed by a `TJ` gap reads back with two spaces (`(Hello ) -200 (world.)`), which is
the word-gap inference adding a break a document stated, and is the silent-repair campaign's
question (ADR 3477) rather than a clause's; a role-map chain is followed past the first standard
type it reaches (`Chapter → Sect → Div` answers `Div`), which §14.7.3's "shall always be mapped"
supports and NOTE 2's "until it either finds a structure type it recognises" does not; a PDF 2.0
type with no `/NS` is read as PDF 2.0's although §14.8.6.1 puts it in PDF 1.7's namespace, a choice
ADR 2014 section 3 records; §14.7.6.4's user properties are read and shown by no panel, where
Table 362 says processors "should display text, number and boolean values".

## Findings

Five behaviour gaps. In order of how much a person would see:

- **2229-B01** — an element whose content is a whole form `XObject` (§14.7.5.3; §14.7.5.4's
  EXAMPLE 1) gets none of its text. The form's text is reported as an artifact by absence, missing
  from `Tree::logical_text`, and the element's accessibility node has an empty name; the same form
  drawn inside the element's `/MCID` sequence is real content on all three routes.
- **2229-B03** — a field is named to a person by its qualified name although it states `/TU`:
  every `field {name}: …` sentence `submission::compose` owes (ten sites, Table 227 bit 2's Required
  report among them), the script question's subject and the edit log's notes. `FieldName::shown`
  is the right answer and the accessibility tree uses it.
- **2229-B02** — a structured sequence inside a `Span` stating `/Lang` takes the `Span`'s language
  when no element above it states one; the clause withholds a `Span`'s language from "nested
  marked-content that is contained in the structure hierarchy", so the document default applies.
- **2229-B04** — the outline panel's rows (and the layers panel's) carry no language, though
  §14.9.2.1 makes the catalog's `/Lang` the language of "outline entries and names for optional
  content groups".
- **2229-B05** — `Interpretation::speech`, the public vocalisation of a page, speaks a structure
  element's `/Alt` once per sequence it owns and not at all over a child element's content.
  `viewer-core`'s tree reads the element's own `/Alt` and is not affected; no production caller uses
  `speech` today.

One documentation finding: D01, `by_absence`'s and `artifacts_by_absence`'s docs give three routes
into the structure tree and omit Table 359's form or image `XObject` (B01's cause).

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `14.7.5.3`,
`14.7.5.4`, `14.8.2.2.2` and `14.8.2.5.1` (B01, D01), `14.9.2.3` (B02), `14.9.3` (B03, B05),
`14.9.2.1` (B04).

## Suggested fix groups

1. **`pdf-model/src/content.rs`'s `by_absence`, the interpreter's object spans, and
   `structure/logical.rs`'s `logical_text`, with `viewer-core/src/accessibility.rs`'s
   `Child::Object` arm** — B01 and D01: the readback range of an `XObject` stating `/StructParent`
   recorded beside the annotations', subtracted from the artifacts, and attributed to the element an
   `/OBJR` names.
2. **`pdf-model/src/content/marked.rs`'s `accessibility` and `accessibility.rs`'s `speech`** — B02
   and B05: a structured sequence's language that stops an enclosing `Span`'s, and `/Alt` and `/E`
   read through the walk up `/P` that `structure::enclosing` already makes for `/ActualText`.
3. **`pdf-model/src/submission.rs`, `view/fields.rs`'s `field_name_of` and
   `viewer-core/src/scripting.rs`'s `subject`** — B03: `FieldName::shown` in every sentence meant
   for a person.
4. **`viewer-host/src/panel.rs`'s `PanelRow`, `outline_rows` and `layer_rows`, and each toolkit's
   tree** — B04: the document's language carried and set on the accessible items.

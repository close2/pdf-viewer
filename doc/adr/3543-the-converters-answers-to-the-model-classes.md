# 3543 — The converter's answers to `pdf-model`'s classes: content under the fence, page entries to the rewrites and the output's check

Session 2210. Status: **accepted**; extends ADR 3521. Context: ADR 3521 (section 5.1 answered by what
the repair was), ADR 3541 (the classes), ADR 3542 (the grammar); ISO 32000-2 §7.7.3.3 Table 31;
ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

ADR 3521's converter answers a repair outside content with the whole-file rewrite, refuses one in
content by name, and leaves undecodable data to the output's own check. `pdf-model`'s classes fall
on both sides: the operand and inline-object classes are in content; the page tree's are in page
objects and nodes, whose values the rewrite mostly carries as written.

## Decision

- **The content classes** (`model/inline-…`, the five operand classes) join `IN_CONTENT_BY_CLASS`:
  answered where the page is photographed, refused under the fence everywhere else; the run rewrite
  writes none of them out. A unit test holds every sentence to a class `pdf_syntax` or `pdf_model`
  declares, under its clause.
- **A page entry is not refused up front.** A first cut refused every page-object class by name
  before writing; it pre-empted the rewrites that already restate such a page (a box with no width
  written as the box this viewer draws, a recovered page given the values its reader resolved, a
  `/Count` restated from its leaves, ADR 2456). So a page-entry class is left to those rewrites and
  to the output's own check, whose refusal names each class the output still holds (ADR 3521's
  `declined_output`).
- **A page whose ancestry states no `/Resources` is written with one.** Table 31: "If the page
  requires no resources, the value of this entry shall be an empty dictionary." The rewrite that
  gives a page its resources for section 6.2.2 (`Rewrite::PageResources`) now also runs for each
  page the reader logged `model/resources-absent` at, writing that empty dictionary; and the
  rebuild (`recovered`), which wrote such a page without the entry and dropped an inherited empty
  dictionary, writes it too.
- **Fixtures held to the new reading.** Five tests whose sources broke a rule the validator now reads
  are changed with the rule named: a page with no `/Resources` (two fixtures), a `/Length` one byte
  short, `sc` given one operand under a two-colourant `DeviceN`, and a `Tj` given no operand after a
  lone delimiter (two). Two tests' expectations change: a lone `Q`'s page and a page whose box lies
  past the rebuild's bound are refused by section 5.1, by the repair's name, rather than written.

## Consequences

- A page the reader drew on its own box, or with a `/Rotate` of 45, converts only where a rewrite
  restates it; otherwise the output's check refuses it, naming the class. Building a restatement
  for each (a `/Rotate` written as the multiple of 90 the reader drew) would write this reader's
  reading as the producer's, and is left unbuilt by name.

# ADR 0017 — A layer the document turns off is not drawn

Status: accepted, 2026-07-28.

## Context

ISO 32000-2 §6.3.2.2 places three obligations on a processor that renders a page: render the
page contents, draw the appearance stream of every annotation whose flags call for one, and
**respect the default or user-specified optional content configuration**. Two of the three
were built in earlier sessions. The third was not implemented, not reported, and not ranked
anywhere near the top of anything, because the instrument the project had been steering by
was a corpus and the corpus ranks optional content **seventh**, tied at five documents with
three other items.

The corpus was not wrong. It was answering a different question — what share of the files that
exist do we draw correctly — and by that measure five pages is five pages. Clause 6 answers
the other one, and by that measure this was the only one of a rendering processor's three
stated obligations that we failed. Both orderings are legitimate; running only one of them is
what `doc/HANDOVER.md` now calls the demand curve, and it is why this session took the item.

What it cost, meanwhile, was not subtle. `issue12007_reduced.pdf` drew a whole hidden
screenshot — a cat in a plastic cone, with a caption — over a page that all four reference
renderers leave nearly blank. `unsupported: []`.

## The decision

Implement §8.11 as far as it decides what is *drawn*, in a new `pdf-model` module, and record
the rest in the conformance ledger rather than leaving it unstated.

Three things decide visibility, and the middle one is what makes this more than reading a list
of groups that are off:

- **The default configuration** (§8.11.4.3, §8.11.4.5). `/BaseState` reaches every group in
  `/OCProperties /OCGs`, then the array opposite to it adjusts what it names. §8.11.4.5 calls
  the result "the initial state used by all PDF processors", which is exactly the state a
  renderer with no layer panel is in.
- **Membership** (§8.11.2.2). Content usually points not at a group but at a membership
  dictionary, which combines groups under `AnyOn`, `AllOn`, `AnyOff` or `AllOff`, or under a
  visibility expression — a boolean tree of `/And`, `/Or` and `/Not`. **A reader that
  implements "skip what `/OFF` lists" gets `/AllOff` exactly backwards**, and `/AllOff` is
  what a document writes when content should appear precisely while a layer is hidden.
- **Intent** (§8.11.2.3). A group states what it is for and a configuration states which
  intents it considers; a group outside them "shall have no effect on visibility" — neither on
  nor off, simply not consulted. Every group in `issue12007_reduced.pdf` carries
  `/Intent [/View /Design]`, so getting this wrong would have changed that page.

Both entry points are implemented, and both are load-bearing: `BDC /OC` … `EMC` spans
(§8.11.3.2) and `/OC` on a form or image XObject or an annotation (§8.11.3.3).
`issue12007_reduced.pdf` hides its layers through the *second*, so an implementation of only
the first would have passed every fixture and changed nothing on the page that motivated the
work.

**Hiding suppresses marking and nothing else.** §8.11.3.1 is explicit that colour, transform
and clip "shall still be applied" and that the text position advances "even for text wrapped
in optional content" — the state after a hidden section must be what it would have been. The
implementation gets that by construction rather than by care: a counter on the interpreter,
consulted where a command would enter the display list, and nowhere else. Form XObjects and
annotations are skipped whole, which the same clause permits because their state changes do
not escape them, and which also keeps an undecodable image inside a hidden layer from being
reported as a gap.

## What was decided against

**Guessing when a group is not declared.** §8.11.3.2 says content is optional content only if
the operand "is a valid optional content group that is included in the OCGs array of the
optional content properties dictionary … or a valid optional content membership dictionary".
So `/OFF` naming a group that `/OCGs` does not is adjusted by nothing, and the content draws.
A file like that is malformed, and the alternative — treating any group we meet as real —
would hide content on the strength of a guess.

**Treating a missing `/OCProperties` as "no layers are off".** §8.11.4.2 makes it decisive
rather than a default: with the dictionary absent "a PDF processor shall ignore any optional
content structures in the document". A stray `/OC` in such a file is not optional content, and
nothing has to be inferred.

**Implementing the interactive half.** Usage dictionaries (`/Usage`, §8.11.4.4) and usage
application dictionaries (`/AS`, §8.11.4.5) switch groups by zoom level, language or print
state; `/RBGroups`, `/Locked`, `/Order` and `/ListMode` describe a layer panel; `/Configs`
holds alternates for someone to choose between. None affects the initial state. They are
recorded in the ledger, and §8.11.4.4 is recorded as **`silent`** rather than deferred,
because this viewer does have a window: a layer that should switch itself off is drawn, and
nothing says so.

**A tolerant visibility expression.** `/VE` is a tree the document supplies, so it is
untrusted input with a natural recursion. Deeper than 32 is refused, and refusing *reports* —
`Unsupported::OptionalContent` — rather than silently answering "visible". The content is
still drawn, which is the deliberate choice between the two ways to be wrong: content that
should be hidden is on the page where a reader can see it, and content that should be visible
would be missing without a trace.

## Consequences

The oracle's contradicted count fell from 108 to 106, and the corpus's incomplete count did
not move — no document started or stopped reporting, which is what it should do when a feature
lands that only ever *removes* marks.

**One page stayed contradicted, and we are the ones who are right.**
`visibility_expressions.pdf` is pdf.js's own test for `/VE` (issue #12097, PR #13243). With
group C off and A and B on, two of its five expressions are false and two lines stay pale.
We draw them pale. So does `poppler`. `mupdf` and `ghostscript` draw all five dark, and their
source says why:

- `mupdf`, `source/pdf/pdf-layer.c`: `if (pdf_is_array(ctx, obj)) { /* FIXME: Calculate
  visibility from array */ return 0; }`
- `ghostscript`, `pdf/pdf_optcontent.c`: `WARNING: OCMD contains VE, which is not supported
  (ignoring)`
- `poppler` does implement it — the installed library exports
  `OCGs::evalOCVisibilityExpr(Object const*, int) const`.
- `pdf.js` implements it and prefers it over `/OCGs` and `/P`, in `src/core/evaluator_utils.js`.

**This generalises trap 9.** That trap was two references sharing a *decoder*; this is two
references sharing a *gap*. An unimplemented feature almost always falls through to "draw it",
so the same silence produces the same picture in two unrelated programs, and the oracle sees
agreement. Agreement is evidence only where the implementations can fail independently, and a
missing feature is not an independent failure. The page stays listed rather than excused.

**A defect this module shipped for an afternoon is worth recording**, because it was invisible
to every test written from the clause. `/OCGs` may be "a dictionary or array of dictionaries"
(Table 97), and a group's identity is its indirect reference — so reading the entry
*resolved*, to find out which shape it is, turns the single-group form into a dictionary with
no reference left on it, matching nothing in `/OCProperties /OCGs`. Every layer of
`issue12007_reduced.pdf` is written `<< /Type /OCMD /OCGs 38 0 R >>`, so the page drew in full
while eight fixtures passed. The oracle found it; `tests/optional_content.rs` now pins it.

# Retrieving the standard from the standard, and an API a machine can drive

Status: **stated by the project owner on 2026-08-10.** In their words: *"I do also think, that we
shouldn't need the specification markdown files any longer (it might be acceptable to keep them, if
we reference line numbers). Our viewer is now stable enough, that we can trust it. (There should of
course a test, which extracts some text). (That's like a compiler starting to compile itself; at
some point you have to trust it)."*
Priority: 36 — capability. It is the last consumer `viewer-core` was built for and has never had:
not a person, not a toolkit, but a program asking a document questions.
Corpus: —, the first consumer is the fourteen documents under `doc/`
Code: `crates/viewer-core`, `crates/viewer-ffi`, `tools/spec-errata`, `tools/conformance`

## What `doc/md/` is actually used for, checked rather than assumed

Two things, and **neither is line numbers**. `conformance::citation::Citation::line` is the
1-based line *in our own source* where a citation appears; nothing in this tree cites a line of
`doc/md/`. So the owner's escape clause — "acceptable to keep them, if we reference line numbers" —
does not apply, and the dependence is narrower than it looks:

1. **Clause existence.** A citation `ISO 32000-2 §9.6.5.4` is checked against the clause numbers the
   conversion contains. 6115 citations, 217 tables.
2. **Verbatim quotation.** 579 rustdoc blockquotes are compared with the conversion's text through
   `quote::normalise`.

Both are answerable from the PDF directly, and this tree already reads more of it than the
conversion does: session 416 established that `doc/md/` **drops every annotation**, and sessions 417
to 419 found some thirty stale quotations because of it, three of them quoting sentences an erratum
had struck.

## The bootstrapping hazard, and the owner's decision on it

`doc/todo/48` recorded the objection: a gate that checks our quotations against a copy **we
generated** puts this project between the specification and the check, and a defect in our extractor
becomes a defect in the standard we hold ourselves to. The owner has answered it, and the analogy is
theirs: a compiler that compiles itself. The answer is not to pretend the hazard is absent but to
keep the two things it needs:

- **A test that extracts text**, which the owner asked for in the same sentence. The tree already
  has the strongest form of it: `tests/text_extraction.rs` compares against `pdftotext` on 974
  documents at 99.2%, and the fourteen specification PDFs are at **100% of its words**. That is an
  *independent* second opinion and it must stay independent — it is what makes trusting our own
  extraction different from asserting it.
- **A second opinion where a quotation lands.** If our extraction becomes the thing quotations are
  checked against, then any span a quotation falls on should still agree with a foreign extractor.
  That is cheap (it is the same `pdftotext` already in the gate) and it is what stops a silent
  extractor defect from validating a wrong quotation.

**And one thing changes for the better**: an errata-aware check becomes possible. `tools/spec-errata`
already reads the strikeouts and their replacement carets; a quotation checked against the PDF
*with* its errata applied is checked against the current standard, which `doc/md/` cannot do at all.

## Is the API good enough? The honest inventory

The question the owner asked. What exists today, all of it reachable from `viewer-core`:

| want | today |
|---|---|
| a page's text | `Interpretation::text`, one `Placed` per character code in `text_layer` |
| text in reading order | `Tree::logical_range` / `logical_text` (§14.8.2.5) — **truncated on ISO 32000-2**, see below |
| the outline | `Query::Outline`, owned since session 411, with destinations |
| the structure tree | `Query::AccessibilityTree` — §14.7 elements, §14.9's spoken form, quadrilaterals |
| annotations | read completely: §12.5.6's subtypes, `/Contents`, popups, `/RC`, `/State` |
| search | `Command::Find` (document) and `Query::Find` (page) |
| a rendered page | `Rendered::Raster`, and `pdfv_render_request_rasterise` from C |

So the *readers* are there. **Three gaps stand between them and "retrieve a section":**

1. **No addressing by section.** An outline item names a destination; a destination names a page and
   a point. Nothing turns "§9.6.5.4" into a range of text. The structure tree and the outline both
   contain the answer and neither is joined to the readback.
2. **`Tree::walk`'s `MAX_CHILDREN` is 65 536 and ISO 32000-2's structure tree is 71 371 items**
   (session 416), so logical order sees only the front of the one document this is for.
   `ParentTree::for_page` is the route that works. This is `doc/todo/49`'s item 5 and it blocks this.
3. **No text-with-annotations join.** `spec-errata` proves the pieces fit; nothing offers "this
   section's text, with or without the annotation text attached to it" as one answer.

## What a round taking this should build, and the shape question

The owner asked for *"An API (arguments to call the app with AND/OR pipe/http-rest commands) which
is suitable for llms to retrieve data (retrieve a page, a section, with/without annotation
texts…)"*. Three shapes, and they are not equivalent:

- **A CLI with structured output** — subcommands over the existing readers, JSON on stdout. Cheapest,
  composes with everything, no new surface to secure, and it is what the two in-tree tools
  (`conformance`, `spec-errata`) already are. **Start here.**
- **A pipe protocol** — one process, many requests. `viewer-confined` already *is* this: a length-
  prefixed request/reply transport over stdio carrying all twenty-five questions, fuzzed at 13 M
  executions. A machine-facing front end over that transport is mostly wiring, and it inherits the
  confinement for free.
- **HTTP/REST** — and this one has a principle-3 argument attached. The renderer has no network by
  design and the confined worker's seccomp filter has no socket call; a listener therefore lives in
  a *host* process outside the confinement, talking to the worker over the existing pipe. That is
  buildable and is the right layering, but it is a network service and wants its own security
  argument (bind address, no filesystem paths from the wire, request bounds). **Do not add a socket
  to anything inside the sandbox.**

## What would make this round successful

Not "an API exists" but: **`tools/conformance` stops needing `doc/md/`** — clause existence and
quotation text both answered from the PDF, with the `pdftotext` second opinion retained — and a
machine can ask for §9.6.5.4's text, with or without its errata annotations, in one call. The
migration cost is real and is `doc/todo/48`'s: 6115 citations and 579 quotations were verified
against the conversion's whitespace, and a new substrate moves all of it. Doing the retrieval API
first and the substitution second is what keeps that cost separable.

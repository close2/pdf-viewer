# ADR 0257 — The joins that were missing, and a walk that saw half a tree

Status: accepted, 2026-08-10 (session 421).

## Context

The project owner asked two things in one breath:

> Is our API already good enough? Does it allow retrieving text / annotations using for instance
> outlines… An API (arguments to call the app with AND/OR pipe commands) which is suitable for llms
> to retrieve data (retrieve a page, a section, with/without annotation texts…)

and, of the Markdown conversion of the standard the conformance gate reads,

> I do also think, that we shouldn't need the specification markdown files any longer. Our viewer
> is now stable enough, that we can trust it. (There should of course a test, which extracts some
> text). (That's like a compiler starting to compile itself; at some point you have to trust it)

`doc/todo/36` had already answered the first question honestly: the *readers* are all there — a
page's text, the outline with its destinations, the structure tree, every annotation subtype, the
search — and **three joins between them were missing**. This round builds the joins, measures what
the substitution would cost, and does not perform it. The owner's note at the top of that file —
*"ignore the rest-api!"* — closes the third of the three shapes it listed, so what is left is a
command line and a pipe.

## Decision 1 — a CLI, because two of these already exist

`tools/pdf-retrieve` is a library and a binary over `pdf-model`'s readers, printing JSON on stdout.
That is the shape `tools/conformance` and `tools/spec-errata` already have; it composes with
everything, it adds no listener, and it puts nothing new inside the sandbox principle 3 defends.

```
pdf-retrieve <document|outline|sections|page|section> <file.pdf> [<n>|<address>]
             [--annotations] [--subtype <Name,Name>] [--no-artifacts] [--logical]
```

The one design rule worth stating, because everything else follows from it: **the default answer is
`pdf_model::Interpretation::text` byte for byte.** That is the string
`pdf-model/tests/text_extraction.rs` compares against `pdftotext` over 974 documents at 99.2%, and
over the fourteen specification PDFs at 100% of its words. A tool that tidied the text on the way
out would put itself between a caller and the only *independent* measurement this project has of
its own extraction — which is precisely what the owner's compiler analogy needs to stay honest. A
test asserts the equality rather than the intention.

Every departure from that string is asked for and is reported in the answer: §14.8.2.2's artifacts
dropped, §14.8.2.5's order taken instead of the stream's (`"order": "content"` when the document has
no structure tree to give it), a section trimmed at its headings (`trimmed_start`, `trimmed_end`).

There is no serialisation dependency. `json.rs` is a hundred lines and the output is a fixed shape
this tool writes and never parses; what a crate would buy is derive macros for six kinds of value.
The one part that is dangerous to get wrong is escaping, and it has a test: a form feed is legal in a
content stream's text and illegal raw in JSON.

## Decision 2 — a section is addressed through §12.3.3, and two halves of that are choices

Half of the addressing is the document's own statement and half is not, and principle 5 makes the
difference worth writing down.

**Derived.** §12.3.3 makes an outline "a hierarchy of outline items … which serve as a visual table
of contents", and each item's destination is §12.3.2.2's. So the page a section begins on is a fact
the file states. `pdf_model::retrieval::sections` resolves all 988 of ISO 32000-2's items in 23 ms,
through `Pages::indices` for the reason `Outline::section_at` already gives.

**Chosen, and ISO 32000-2 states nothing about either:**

- **A section number is the leading token of the title** where that token is digits, full stops and
  capitals with at least one digit among them. Table 151 makes `/Title` "[t]he text that shall be
  displayed on the screen for this item" and says nothing about its shape. 946 of the 988 items
  qualify; `Foreword`, `Contents` and `ISO 32000-2:2020 front page` do not, which is the outcome
  that matters — an invented number is an address that answers the wrong clause.
- **A section ends where the next outline item that is *not* one of its descendants begins.** The
  alternative is the next item of any kind, which would make asking for §9.6 give the paragraph
  above §9.6.1 and nothing else. Asking for a section and being given its subsections is what a
  reader means by the word.

The *text* is then cut at the two headings, found by squeezing the spaces out of both sides — ADR
0253's comparison, for its reason: two extractions of one heading do not agree about the spaces
between its words, and `9.6.5.4 Encodings for TrueType fonts` in the outline is
`9.6.5.4  Encodings for TrueType fonts` on the page. Each heading is looked for in *its own page's*
stretch of the assembled text and nowhere else, because ISO 32000-2's body is full of
cross-references that would otherwise end a section early.

## Decision 3 — an annotation belongs to a section by where its text is, not by its page

§12.5.6.10's Table 182 states that each `/QuadPoints` quadrilateral "shall encompasses a word or
group of contiguous words in the text underlying the annotation", so the text under a markup is a
fact the file states. `pdf_model::retrieval::text_under` reads it back; it was `spec-errata`'s
private function for five sessions and is now the model's, with two callers.

The join rule: **an annotation that covers text belongs to the section any of that text is in; one
that covers none is kept for its page.** The first half is what makes asking for §9.6.5.4 give the
errata on §9.6.5.4 rather than every mark on the three pages it touches. The second half is the
honest limit and it is visible in the demonstration below: a `Caret` carries the *replacement* text
and no quadrilaterals, so a caret belonging to the clause above is kept.

**And there are two coordinate systems, which cost a defect before they were noticed.** A
`/QuadPoints` span is a range of the *raw* readback. Dropping artifacts or taking logical order
changes which characters are where, so an offset computed in one is meaningless in the other:
§12.5.2 came back with 24 annotations one way and 23 the other. `Retrieval::section` now assembles
its pages twice — once raw, where the edges are found for deciding what is *inside*, and once as
asked for, where they are found for *cutting* — and a test asserts the two answers agree.

## Decision 4 — the structure-tree walk grows, reports, and stops being quadratic

`doc/todo/49`'s item 5 said `Tree::walk`'s bound "silently truncates the largest document this
project owns" and that the walk "should say so or grow". Measured, it was worse than recorded in two
ways.

**The tree is bigger than anybody knew.** The bound was 65 536 *items* over the whole tree, applied
at the top of each recursion, so it overshot to 71 371 and stopped — and session 416 wrote that
number down as the size of ISO 32000-2's structure tree. It is **129 389**. `logical_order` walks
the whole tree once per page, so §14.8.2.5's reading order for any page of the document this project
checks itself against was a *prefix* of the tree, with nothing said.

**And the walk was quadratic.** Its visited set was a `Vec<Dictionary>` searched linearly and
comparing whole dictionaries at each step. Measured on ISO 32000-2 in this session:

| | items reached | time |
|---|---|---|
| before | 71 371 (truncated, 44 651 elements) | **16.8 s** |
| after | 129 389 (78 468 elements) | **151 ms** |

Three changes, and the second is the one that matters more than the speed:

- the walk's own bound is `MAX_ELEMENTS`, 2²⁰ — eight times that tree — separate from
  `MAX_CHILDREN`, which stays 65 536 and bounds one `/K` array;
- `Tree::walk` answers a `Reading` carrying `truncated`, and `logical_text` and `logical_range`
  refuse on it rather than returning a prefix. `logical_text`'s return type became `Option<String>`
  for that, which is the shape `logical_range` already had for the same reason: a partial reading of
  a page is the one failure a caller cannot see;
- the visited set is keyed by `ObjectId`. An element reached other than through a reference has no
  identity to remember and is always descended into, which loses nothing — a dictionary written
  inline in its parent's `/K` is contained by that parent, so it cannot close a cycle.

The tool routes around none of this: it asks for content order by default, which needs no walk at
all, and `--logical` now gets the whole tree when it asks for one.

## The demonstration

Asked of the release binary, on the document this project checks itself against. **66 ms**, cold.

```console
$ target/pdf-retrieve section doc/ISO_32000-2_sponsored_EC3.pdf 9.6.5.4 --no-artifacts
  number: 9.6.5.4
  title: 9.6.5.4 Encodings for TrueType fonts
  pages: [339, 340, 341]        ends_at: 9.7 Composite fonts
  trimmed_start: true           trimmed_end: true
  complete: true                unsupported: []
  words: 1077
  text: "9.6.5.4  Encodings for TrueType fonts \nA TrueType or OpenType font program's …"
```

and the errata attached to it:

```console
$ target/pdf-retrieve section doc/ISO_32000-2_sponsored_EC3.pdf 9.6.5.4 --subtype StrikeOut,Caret
  "annotations": []
```

**Which is a real answer and a weak demonstration**, so the same question of a clause Errata
Collection 3 did change — and it is the one ADR 0253 found this tree implementing from retired text:

```console
$ target/pdf-retrieve section doc/ISO_32000-2_sponsored_EC3.pdf 12.5.2 --subtype StrikeOut,Caret
  pages [481, 482, 483, 484]   words 1613   annotations 23
    p484 Caret     Issue #23 and #34  says="When rendering the appearance dictionary, a PDF reader"
    p484 StrikeOut Issue #23 and #34  struck="BM, "
    p484 Caret     Issue #56          says="MK, "
    p484 StrikeOut Issue #34          struck="NOTE"
    p484 Caret     Issue #34          says="NOTE 2"
```

Without `--subtype` the same clause answers 161 annotations — 55 links, 53 popups, 30 `Text` replies carrying §12.5.6.4 states, and only 23 errata — and §9.6.5.4 answers 11, all of them
§12.5.6.5 links: the filter is what turns "every mark on these pages" into "the errata on this
clause".

## What the substitution would cost, measured rather than estimated

`doc/todo/36`'s stated success condition is that `tools/conformance` stops needing `doc/md/`, and
`doc/todo/48`'s item 5 is the migration. **It is deliberately not done in the same round as the
API** — session 413 declined a 417-span migration for less — but it now has a number.
`tools/pdf-retrieve/examples/substitution_cost.rs` asks the gate's own two questions of both
substrates:

| | `doc/md/` | the PDF, through this reader |
|---|---|---|
| clause existence | 1034 headings, 8 ms to parse 24 MB | 946 numbered outline items, **23 ms**, nothing interpreted |
| **506 distinct clauses cited** | 0 missing | **0 missing** |
| whole-document extraction | — | 4.30 s, 2 658 697 bytes |
| **582 blockquotes** | **582** verbatim | 40 by the gate's comparison |
| … with the spaces taken out | | 523 |
| … and the dashes folded together | | **553** |
| **left to re-verify by hand** | | **29**, 5.0% |

Two things follow, and neither was known before the measurement.

**Clause existence is free.** Every clause this tree cites is an outline item of the standard, so
that half of the gate could move today at no cost in verification.

**The quotation half costs 29 readings, and the reason is typography rather than words.** The gate's
own comparison finds 40 of 582, which sounds like a wall and is an artefact: `doc/md/` and this
reader are two programs extracting the same glyphs, and PDF positions glyphs rather than words.
Squeezing the spaces takes it to 523 — ADR 0253's finding, one substrate over. Folding the dashes
takes it to 553, and *that* is a finding about this tree rather than about the comparison:
`doc/md/` writes `Table 87 -Additional entries` where the standard prints `Table 87 — Additional
entries`, and a quotation copied out of the conversion carries the conversion's dash. **59 of our
blockquotes quote the converter's typography.** They are still the standard's words; they are not
the standard's characters.

So the migration is not 582 spans of work. It is one comparison decision — spaces and dashes folded,
which the errata sweep already argues for — and 29 readings.

## The pipe, and what was *not* built

`doc/todo/36` put the pipe second and said it "mostly exists": `viewer-confined`'s transport carries
all twenty-eight questions over stdio, length-prefixed, fuzzed at 13 M executions. **No second
transport was built, and none should be.** What the round found instead is that the missing piece is
not a transport but a *vocabulary*: `viewer_core::Query` has no "this page's text". The readback
crosses that boundary only as a *selection* — which needs a drag — or as §14.9's spoken accessibility
nodes. So a retrieval consumer on the pipe costs one `Query` variant, its encoding on both ends and
one number in the C ABI's count, and it costs no new surface at all. That is recorded in
`doc/todo/36` with its size rather than done here, because `viewer-core`'s vocabulary is a person's
at a window and adding to it is a decision about that boundary rather than about this tool.

## Consequences

- A program can ask this project for a page, a section, and the annotations over either, in one
  call and in a shape a machine reads. The first consumer is the retrieval of the standard itself.
- `pdf_model::structure`'s walk is 111 times faster on the largest document here and no longer lies
  about how much of it it saw. Every consumer of §14.8.2.5's order is affected and none had to
  change except to unwrap an `Option`.
- `tools/spec-errata` lost 104 lines to `pdf_model::retrieval::text_under`, which is now where the
  §12.5.6.10 join lives. `conformance` still depends on neither, deliberately (ADR 0252): nothing
  this project generates may become what the gate checks the standard against while the question of
  switching is open.
- **The bootstrapping stays honest.** `tests/text_extraction.rs` is untouched and is still the
  foreign second opinion; this tool's default answer is asserted to be the string it measures.
- The tool's *output*, run against the documents under `doc/`, is the standard's text, and ADR 0187
  covers it exactly as it covers `doc/md/`. The substitution-cost example prints counts and clause
  numbers and no sentence of the standard, which is why its numbers can be written down here.

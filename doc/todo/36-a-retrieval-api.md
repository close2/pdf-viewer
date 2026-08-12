# Retrieving the standard from the standard, and an API a machine can drive

Note from the human after this file had been created: ignore the rest-api!

Status: **the CLI is built and the three joins are closed** — session 421, ADR 0257. What is left is
one message on the pipe and the substitution itself, both below with their measured sizes.
Priority: 36 — capability. It was the last consumer `viewer-core` was built for and had never had:
not a person, not a toolkit, but a program asking a document questions.
Corpus: —, the first consumer is the fourteen documents under `doc/`
Code: `tools/pdf-retrieve` (built), `crates/pdf-model/src/retrieval.rs` (built),
`crates/viewer-core`, `tools/spec-errata`, `tools/conformance`

## What is built

`tools/pdf-retrieve`, a library and a binary, JSON on stdout and nothing else there:

```sh
pdf-retrieve <document|outline|sections|page|section> <file.pdf> [<n>|<address>] \
             [--annotations] [--subtype <Name,Name>] [--no-artifacts] [--logical]
```

**The three gaps this file named are closed**, each with its argument in ADR 0257:

1. **Addressing by section.** `pdf_model::retrieval::sections` turns §12.3.3's outline into one
   `Section` per item carrying the pages its text occupies — from its own destination to that of the
   next item which is *not* one of its descendants. All 988 of ISO 32000-2's items resolve in 23 ms;
   946 carry a clause number. The text is cut at the two headings with the spaces squeezed out of
   both sides, which is ADR 0253's comparison and for its reason. Two halves of this are **choices**
   and are documented as choices: that a number is the title's leading token, and where a section
   ends.
2. **`Tree::walk`'s bound: fixed, not routed around.** It was worse than this file recorded — 65 536
   *items* over the whole tree, so session 416's 71 371 was the bound rather than the tree, which is
   **129 389** — and the walk was quadratic besides, at **16.8 s** for ISO 32000-2. It is 151 ms,
   bounded at 2²⁰, and `Reading::truncated` says when the bound is reached; `logical_text` (now
   `Option<String>`) and `logical_range` refuse rather than answer a prefix. `doc/todo/49` item 5.
3. **Text with annotations.** §12.5.6.10's `/QuadPoints` read back through
   `pdf_model::retrieval::text_under` — moved out of `spec-errata`, 104 lines, two callers — and an
   annotation belongs to the section **any of the text it covers is in**, rather than to the pages
   the section touches. `--subtype` narrows it to the errata.

Demonstration, on the release binary, 66 ms: §9.6.5.4 comes back as pages 339–341, trimmed at both
headings, 1077 words, and **no erratum touches it**; §12.5.2 comes back with 23 strikeouts and
carets including the `BM, ` that ADR 0253 found this tree implementing from retired text.

## The pipe: one message, not a transport

`viewer-confined`'s length-prefixed stdio transport carries all twenty-nine questions and is fuzzed
at 13 M executions, so **no second transport was built and none should be**. What is missing is a
*vocabulary* entry rather than a channel: `viewer_core::Query` has no "this page's text". The
readback crosses that boundary only as a `Selection` — which needs a drag — or as §14.9's spoken
accessibility nodes.

The size, so that whoever takes it is not guessing: one `Query` variant and its `Answer`, its two
`match` arms in `viewer-confined/src/protocol.rs` (both exhaustive, so the compiler names them), and
one number in `viewer-ffi`'s `PDFV_*_COUNT`. It is left undone deliberately: `viewer-core`'s
vocabulary is a *person at a window's*, and adding to it is a decision about that boundary rather
than about this tool. Take it when a host wants a page's text, not because the list looks short.

## The substitution: measured, and deliberately not done here

The success condition this file used to state is that `tools/conformance` stops needing `doc/md/`.
It now has a number instead of an estimate — `tools/pdf-retrieve/examples/substitution_cost.rs`,
which asks the gate's own two questions of both substrates:

| | `doc/md/` | the PDF, through this reader |
|---|---|---|
| clause existence, 506 distinct clauses cited | 0 missing | **0 missing**, out of 946 numbered outline items, 23 ms |
| 582 blockquotes | **582** verbatim | 40 by the gate's own comparison |
| … with the spaces taken out | | 523 |
| … and the dashes folded together | | **553** |
| **left to re-verify by hand** | | **29** |

So the migration is not 582 spans of work: it is **one comparison decision** — fold spaces and
dashes, which the errata sweep already argues for — and **29 readings**. Clause existence could move
today at no cost in verification at all.

And the 59 that need the dash fold are a finding about *this tree*: `doc/md/` writes
`Table 87 -Additional entries` where the standard prints `Table 87 — Additional entries`, so those
quotations carry the converter's typography. They are the standard's words and not its characters.

**Doing it is still a separate round**, and the reason is unchanged: session 413 declined a 417-span
migration for less, and a gate that changes what it compares against on the same day the comparison
is written has nothing independent left to check it.

## The bootstrapping hazard, and what keeps it honest

The owner's answer to it is a compiler compiling itself, and what that needs is two things this
round did not touch and the next must not either:

- **`pdf-model/tests/text_extraction.rs`**, which compares this reader's readback against
  `pdftotext` over 974 documents at 99.2% and over the fourteen specification PDFs at 100% of its
  words. It is the *foreign* second opinion and it is what makes trusting our extraction different
  from asserting it.
- **`pdf-retrieve`'s default answer being that same string, byte for byte.** A test asserts it. Any
  helpfulness added here — smart quotes, joined hyphens, dropped headers by default — would put this
  tool between a caller and that measurement, silently.

## What would make this item done

The pipe's one message, and the substitution with its 29 readings. Neither is blocked and both have
a size. When both land, this file goes.

# ADR 0097 — A table that parses is not a table that works

Status: accepted, 2026-08-01.

## Context

`corpus.rs`'s own comment had named this and priced it, four sessions before anybody took it:

> **Three are recovered by at least one reference and not by us**, which is a robustness gap
> rather than a clause … All three fail here with `/Root` missing or not a dictionary, and two of
> them **without the cross-reference table having been rebuilt at all** — `was_recovered()` is
> false, so the scan that exists was never reached. That is the cheapest of these to take on.

This is the project's second question — *what share of the files that actually exist render
correctly* — where the corpus is the only instrument and no clause coverage answers anything. It
is also the sentence-that-admits-ignorance habit paying again: the diagnosis was written down and
sat there.

## Two rules, and neither comes from another reader

**§7.5.5: a cross-reference table that leads to no catalog has been disproved by the file
itself.** `/Root` is "[t]he catalog dictionary for the PDF file". `xref::read` scans for
objects only when the table is *absent, unreadable or empty* — which leaves exactly the case a
hand edit produces: a complete, self-consistent table whose offsets all point a few bytes wrong.
It parses; it works for nothing.

`Document::open` now checks `catalog()` once after opening, and where it fails and the table was
not already a scan, rebuilds by scanning and tries again. The rebuilt document is kept only if it
does better, so a caller's error message is unchanged wherever this changes nothing, and a
well-formed document pays one dictionary lookup.

**§7.7.3.3 Table 31: a page is an object that says it is one.** `/Type` is "(Required) … shall be
`Page` for a page object". §7.7.3.2's tree is a walk *downwards* from the catalog, and a document
whose tree is broken has no page one — but every page in it is still declaring itself. So where
the tree yields nothing, `Pages::new` asks each object instead, and applies §7.7.3.4's
inheritance up each recovered page's own `/Parent` chain: the tree failing downwards does not
stop `/Parent` working upwards, and that is where a recovered page's `/MediaBox` and `/Resources`
come from.

Both rules are recoveries from **the file's own declarations**, which is what keeps them inside
principle 5. Neither was arrived at by looking at what `ghostscript` does; the evidence that
`ghostscript` opens these documents is what made the gap worth pricing, and that is the direction
of inference the principle allows.

Page *order* is the one thing a scan cannot recover, and the choice is written down: ascending
object number, because §7.7.3.2's tree is where order lives and a file whose tree is gone has not
stated one.

## What moved

- **Documents with no page one: 11 → 5.** `issue18986.pdf`, `issue9418.pdf`,
  `operator_list_cycle.pdf`, `issue19484_1.pdf`, `issue19484_2.pdf` and `poppler-395-0-fuzzed.pdf`
  now reach page one.
- **The oracle's `no render` count: 25 → 19**, and `issue18986.pdf` joins the **agreeing** set.
  Agreeing 839 → 840, contradicted 65 unchanged.
- **`issue9418.pdf` draws completely**, which the inheritance half is responsible for: its
  recovered page's `/Resources` are on its `/Parent`, and without them two fonts were missing.
- **Corpus documents drawing incompletely: 86 → 90, and the rise is the point.** Four of the five
  remaining new pages report something — a form-depth cycle the file is *named* for, two whose
  content is ciphertext this reader derives the wrong key for (a known pair, ADR 0031), and a
  fuzzed file whose content stream does not inflate. Trap 5: a rise is not a regression when it is
  a new report, and these are not even that — they are pages that were not being counted at all.

## Looking at the pages, because that is the rule

Trap 1 says every page a change makes drawable is a page nobody has looked at, so all three
recovered pages with content were rendered and read.

`issue18986.pdf` draws **nothing** — no commands, no text — and the reference consensus agrees,
which is why it lands in the agreeing set rather than anywhere interesting.

`operator_list_cycle.pdf` draws nothing and reports `MAX_FORM_DEPTH`, which is what the file is
built to do. Its page size disagrees with all three references — ours 596×842, theirs 612×792 —
and that is *our documented default* meeting theirs: the recovered page states no `/MediaBox`
anywhere in its chain, so `Page::DEFAULT_MEDIA_BOX` applies, and it is A4 "because this project's
corpus and locale are metric" while every reference defaults to US Letter. A documented choice
showing itself, on an incomplete page the gate does not judge.

`issue9418.pdf` is the one worth having looked at. It draws an architectural title block —
correctly, with "General Notes", "Revision/Issue", "Firm Name and Address", "Project", "Sheet",
"Date", "Scale" all legible — and across the sheet, four enormous Arabic glyphs. They read back
as **سلام**, twice, which is a word rather than garbage, and a giant one is exactly what a test
file filed under an Arabic-rendering issue would contain. No reference can open the document at
all — `poppler` reports zero pages — so there is no oracle, and the readback is the evidence.

## Consequences

No clause changed status; two rows gained a paragraph. 838 tests, and the fixtures for this are
the corpus itself, which is the honest place for a robustness rule: a synthetic file with a
deliberately broken table would only test what I imagined a broken table looks like.

What is left of the pageless list is five documents and every one of them is refused by every
reference too, or is not a PDF defect at all: `Brotli-Prototype-FileA.pdf` prototypes a filter
the PDF Association is still standardising, `bug1020226.pdf` is a Firefox worker-shutdown bug,
and the other three are fuzzer crashers kept as regression fixtures.

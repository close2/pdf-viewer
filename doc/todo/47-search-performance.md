# A document-wide search takes 6.19 s on 1023 pages

Status: **raised by the project owner on 2026-08-09**, on reading session 414's report:
*"Is the search implemented single threaded? 6.19s doesn't sound that fast. Can we easily improve
this? … Any improvement must be reasonable. Improving the search speed is a goal, but not a
requirement if the cost is too high (for instance in code quality or possibly also memory usage)."*
Priority: 47 — performance, measured, and explicitly **not** a requirement: the owner has priced
the trade in advance and code quality and memory both outrank the seconds
Corpus: every document; the cost scales with the *document*, not with the needle
Code: `crates/viewer-core/src/viewer.rs` (`find_step`), `crates/viewer-core/src/search.rs`,
`crates/viewer-core/src/open.rs` (`interpret`), `crates/pdf-model/src/content.rs`

## Yes, single-threaded — and by rule rather than by oversight

`Command::Find` reads **one page per step**; the host pumps `Find::Continue` and
`Event::Searched { found, remaining, wrapped }` says how many are left. `viewer-core`'s rule 4
(`doc/ui-boundary.md`) is *"no threads the core was not handed, and no blocking"*, and rule 3 leaves
it no clock to budget with — which is why the design is a pump and not a loop. ADR 0250 argued that
shape and it is not the thing to undo casually: it is what lets a host keep its window alive during
a search, and it is why the confined worker and the C ABI can both drive one.

**Where the 6.19 s goes is not the searching.** `find_step`'s own comment names it: `interpret` is
the expensive half of this program, measured at **5.7 ms a page** over ISO 32000-2's 1023, and the
readback it produces is **thrown away** afterwards. The needle comparison is `select::find` over a
`String` and is not on anybody's list of costs.

**And the first measurement of it was wrong about its own subject**, which is worth keeping: a
full-document miss took **19.25 s** until session 414 noticed `viewer-ui` was repainting on every
step — 1024 windows through `lavapipe` to move a progress digit. Throttled to one step in sixteen it
was 6.19 s. Before optimising the search, check that the *host* is not the thing being measured.

## The four candidate answers, and what each costs

None is free, and the owner's constraint decides between them rather than the seconds do.

1. **Interpret less.** A search needs `Interpretation::text` and nothing else — not the display
   list, not decoded images, not shadings. If a text-only path is a *narrowing* of the same walk
   (a flag that stops commands being emitted) it is cheap and safe; if it is a **second extraction
   path**, it can diverge from what the page draws, and then a search finds words the reader cannot
   see. **This is the one to measure first**, and the discriminator is whether it can be built
   without a second traversal to maintain. `examples/readback` already exists and is the instrument.
2. **Parallelism, which rule 4 puts in the host rather than the core.** A host may pump several
   searches — but `pdf_syntax::Document` caches behind `RefCell` and **is not `Sync`**, so N threads
   means N documents, i.e. N parses and N caches. That is memory the owner has named as a possible
   veto, and it re-reads the file N times. Measure the per-document cost before believing this is a
   win: `Document::open` is 10–13 ms on 101 318 objects and the caches are what make page 900 cheap.
3. **A readback cache.** Priced and deliberately absent in ADR 0250, for the memory reason — 2.66 MB
   of readback for ISO 32000-2, which is not much, but it is per document and unbounded in principle.
   Cheap to reconsider *with a bound*; the argument to beat is the one already written down.
4. **Skipping pages cheaply.** Tempting and probably unsound: a content stream's bytes are not the
   page's characters — a `/ToUnicode` CMap, a composite font's codes and §9.10.2's three methods all
   stand between them, so a byte scan for the needle would miss text this tree can read. Do not take
   this one without an argument that survives `doc/todo/21`'s populations.

## What a round taking this owes

- **Measure before choosing**, and measure the right thing: `find_step` alone, not a host's loop.
  Session 391's finding is the standing warning — the "2 ms accessibility cost" was two X11 round
  trips, and the "bimodal scene" was per *sample* rather than per command.
- **State what each option costs in memory and in lines**, because those are what the owner said may
  veto it. A 2× speed-up that adds a second text path is probably the wrong trade; a 2× that deletes
  work is obviously the right one.
- **Do not undo the pump.** Whatever is done stays a step a host drives; rules 3 and 4 are not
  negotiable and a blocking search would cost the confined host and the C ABI their responsiveness.
- **A search that returns different results after the change is a defect, not a speed-up.** The gate
  is `tests/text_extraction.rs` at 99.2% and the 14 specification PDFs at 100% of `pdftotext`'s
  words; a text-only path must reproduce both exactly.

## What is explicitly not owed

A match count ("3 of 17"), which needs the whole document read before the first answer. No host has
asked, and nothing in the vocabulary prevents one doing it.

# ADR 0701 — A font outlives the page that loaded it

Status: accepted, 2026-08-28 (session 770).

## Context

ADR 0694 re-took `callgrind_interpret`'s composition and left one sentence precisely: 21.4% of
interpreting page 101 of ISO 32000-2 is `Interpreter::font`, almost all of it seven
`pdf_font::LoadedFont::load`s that an `Interpreter` living for one page pays again on the next.
Whether a font cache can outlive an interpretation is a question about `Document`'s immutability
and about memory, and 766 did not take it.

Two sentences of `CLAUDE.md` bind the answer. `pdf_syntax::Document` stays immutable so that
`interpret` remains a pure function of what the file says, the viewer state and what the user did
— the oracle's whole comparison rests on it. And principle 3 requires an explicit memory budget,
derived rather than picked.

## What a second page actually re-does, measured before anything was built

A temporary counter in `Interpreter::load_font` recorded every key an actual load ran under
(reverted the same day; nothing of it is in the tree). Over the pdf.js corpus at up to ten pages
per document:

- **62.4% of all font loads in multi-page documents are re-loads** of a font an earlier page of
  the same document had already loaded (893 of 1432); over the whole corpus including one-page
  documents, 35.6%.
- ISO 32000-2's first forty pages: 240 loads, **27 distinct fonts**, 213 repeats.
- The median corpus document names **1** distinct font, p99 names 22, the maximum 51
  (`issue6127.pdf` — on one page, so nothing there repeats).
- 47 loads in the whole corpus are keyless — a resource dictionary stating its font *directly*
  rather than by reference (`bug946506.pdf` is 19 of them). Those have no identity to key on and
  stay uncached, exactly as they were.

So the population the briefing asked about is real and the cache's shape follows from it: what
repeats is *the same object across pages of one document*; what a second *document* shares is
nothing, because an `ObjectId` means nothing across files.

## Decision

### Where it lives: beside the document in `viewer-core`, passed into `pdf-model` by reference

`pdf_model::FontCache`, held by `viewer_core::Open` beside the document — the place ADR 0256 put
the readback cache and for the same reason — and threaded through a new public entry point
`interpret_with_fonts(document, page, state, &cache)`. `interpret` and `interpret_with` are that
function with a fresh cache per call, so every existing caller — the oracle, the corpus gates,
every example — computes what it computed before through the same code path.

Not inside `pdf-font`: a global or thread-local cache there would have no per-document lifetime
and would put the binding question (below) where no host can see it. Not inside `Document`:
`pdf-syntax` cannot name a `LoadedFont` (the dependency runs the other way), and the cache is
about *interpretation*, which is `pdf-model`'s layer.

One caller inside `pdf-model` shares it without being handed one: §11.4.7's subtractive pair
interprets one page twice, and `interpret_with` gives both runs one cache, so a `DeviceCMYK`-
blended page loads each font once instead of twice.

`Open::stale` deliberately does **not** empty it: everything that makes a display list stale is a
move of the view state, and a loaded font is a function of the document alone.

### What the key is: the font dictionary's `ObjectId`, bound to the document's own bytes

The per-interpretation key (`FontKey::Referenced`, session 127) is already the right identity
*within* a document: `LoadedFont::load` is a function of the document and the dictionary — the
`name` argument reaches only error wording — so one object is one loaded font, whatever names
pages use for it. What a cache that outlives the interpretation adds is the *document* half of
the key, and it is held rather than remembered: the cache keeps the `Arc<[u8]>` of the document
it was filled from and empties itself when a different allocation arrives. Holding the
allocation is what makes the address a name — `doc/todo/41`'s lesson at 4 KB, ADR 0317's
liveness invariant one crate down.

A *failed* load is kept per page (as before) and never across pages: keeping one would make the
second page's `Interpretation::unsupported` depend on interpretation order and word the report
with the first page's resource name — a change to the answer, not the cost, so it is declined.

### What it is bounded by: 2 MiB of font program, least-recently-used, derived twice over

`FONT_BUDGET`'s doc comment carries both derivations in `DECODED_BUDGET`'s form; the instrument
is `examples/font_cache_budget`, which runs the real cache at each budget and prints the
process's high-water resident memory beside each row. The floor: 1 MiB thrashes on ISO 32000-2's
first hundred pages (251 evictions), 2 MiB takes 94% of what an unbounded cache gives. The
ceiling: the owner's band (ADR 0256) less the readback's 4 MiB and the decoded streams' 4 MiB —
and, sharper than the arithmetic, the charge's own honesty: over all 1023 pages a 2 MiB budget
costs **+2.0 MB** of peak resident memory (the budget and nothing else, because an embedded
program's bytes are the same allocation the decoded-stream cache already holds), while 4 MiB
costs **+6.3 MB** — the uncharged tables beside the programs overtake the charge. A bound whose
accounting stops being true above it is a bound to stay below.

### What had to change in `pdf-font`: `Rc` → `Arc`, cells → locks

`Open` crosses a thread once (ADR 0182 opens the document beside the window and moves the viewer
back), so everything held beside a document must be `Send`. `Font`'s `Rc`s became `Arc`s and
`LoadedFont`'s four memos became `Mutex`/`OnceLock`. Priced in isolation before being taken:
**+0.468%** on fifty interpretations of page 101, **+0.140%** over twenty distinct pages.

## The numbers

`examples/callgrind_pages` is the instrument the briefing said would have to be built:
`callgrind_interpret` repeats one page and therefore *contains* the repetition a cross-page cache
removes, so it cannot measure the motivating case. The new example walks distinct pages, and both
arms are one binary — `fresh` builds a cache per page, `kept` passes one for the run — sharing
one `ViewState`, so the difference is the cache and nothing else. All counts callgrind, one
sitting, display-list command totals identical in every pair:

| workload | fresh | kept | |
|---|---:|---:|---:|
| ISO 32000-2, pages 1–20 | 574 165 171 | 488 822 640 | **−14.86%** |
| ISO 32000-2, pages 101–150 | 2 120 905 451 | 1 716 780 379 | −19.05% |
| ISO 32000-2, page 101 × 50 (the `stale` population) | 1 208 416 582 | 829 880 750 | **−31.32%** |
| `tracemonkey…8.pdf`, 14 pages | 957 904 327 | 846 040 865 | −11.68% |
| `alphatrans.pdf`, 1 page (miss-only) | 6 263 848 | 6 263 863 | +0.0002% |
| `issue6127.pdf`, 1 page, 51 fonts, no reuse | 105 928 196 | 105 928 417 | +0.0002% |

The revisit row is larger than the load share because a kept font brings its built memos —
outlines, AGL cells — which are memos of pure functions of glyph and code.

What a caller that keeps nothing pays for all of this — the `Send` conversion plus the per-load
insertion — is `callgrind_interpret` before this round against after: 1 195 249 573 →
1 201 660 678, **+0.536%** on the arm the oracle and the gates run in. Taken, because the
reader's workloads above are 12–31% and the gates interpret one page per document.

## The purity argument is tested against its own defects, not asserted

`content::tests::a_kept_font_changes_what_a_page_costs_and_not_what_it_says` interprets five
corpus documents' pages twice — alone, and through one shared cache walked across all of them —
and compares the `Debug` rendering of the whole interpretation (list, reports, text, glyph
count). Trap 13, both directions:

- a `get` stubbed to answer with *any* held font fails the comparison on page 2 of the first
  document — the equality catches key confusion within a document;
- a `bind` stubbed never to rebind **passes** the comparison — four documents' font dictionaries
  simply do not collide on object numbers — and only the test's direct `rebound > 0` assertion
  fails. Which is exactly why the binding is asserted rather than trusted to show up as a wrong
  glyph, and why the comparison alone would be false confidence about the cross-document case.

A third assertion (`hits > 0`) stops the whole test passing with the cache never consulted, and a
second test runs a 64-byte budget to exercise eviction-to-empty and assert a starved cache
changes no answer.

## What was declined

- **Caching in `pdf-font` or globally** — no per-document lifetime, no host-visible binding.
- **Keeping failures across pages** — changes reports, not costs (above).
- **A 4 MiB budget** — the charge's accounting breaks above 2 MiB (above).
- **Keying by allocation instead of holding the bytes** — the 4 KB lesson; an address is only a
  name while something holds it.
- **Sharing across documents** — nothing to share; an `ObjectId` is scoped to a file.

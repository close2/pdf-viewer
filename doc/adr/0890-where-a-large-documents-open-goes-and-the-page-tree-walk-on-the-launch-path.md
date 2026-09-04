# 0890 — Where a large document's open goes, and the page-tree walk that was on the launch path

Session 925. Status: **accepted**. The first of this round's two records; the second,
[ADR 0891](0891-the-open-is-linear-in-the-objects-the-table-names.md), is what follows for
`CLAUDE.md` principle 2's sentence about the five-page document.

## Context

[ADR 0885](0885-what-the-launch-path-costs-and-which-of-principle-2s-claims-hold.md) measured
principle 2's claims and found one false: *a 500-page document must open no slower than a 5-page
one* is false of the open by about thirty times. It did not say **where** that time goes, and it
recorded two neighbouring claims as holding — *no full page-tree walk* among them — on evidence
that turns out to have covered only part of the path.

This round profiled the open. The instruction was to start from where the time goes rather than
from the sentence, and that is what produced both findings below.

## The instrument, and the hole in it

`crates/pdf-model/examples/open_cost.rs` says in its own module comment that it measures
"everything `viewer_core::Open::around` and `viewer_core::notes::about` do before a window
exists". It did not: `viewer_core::Viewer::open` ends with `announce_page`, which is neither of
those two, and which is where the largest item turned out to be. **A step named by an example's
comment and absent from its output is invisible exactly the way a missing gate line is** — the
example was believed, by ADR 0885 among others, to be the whole path.

It now carries `announce_page`'s three steps. That is how the finding below was made, and it is
the reusable half of this round.

## Finding 1 — a full page-tree walk is on the launch path, and principle 2 forbids one by name

`Viewer::announce_page` raises `Event::PageChanged`, whose `section` is §12.3.3's innermost
outline item covering the page being shown — the window caption's second half. It reaches it
through `Outline::section_at`, which resolves every item's destination to a page number; and
resolving many destinations one at a time is a tree walk apiece (ADR 0058, session 141, 344 ms a
page turn), so `section_at` builds `Pages::indices()` first — **a map of every object in §7.7.3's
tree to its page number, which resolves every node of it.**

That is the "full page-tree walk" principle 2's first startup bullet forbids by name, and it is on
the launch path of every document that has an outline. ADR 0885 read the bullet against
`Pages::new`, which takes §7.7.3.2's `/Count` and does *not* walk — true, and about a step two
functions earlier.

**It is the largest single item in a large document's open.** Pinned to this machine's fast cores,
minimum of nine fresh processes, `--release`, warm (`doc/habits.md`'s *Measuring*):

| step | 5 pages | 57 pages | 1023 pages | scales with |
|---|---|---|---|---|
| `Document::open` (§7.5) | 0.099 ms | 0.195 ms | 4.184 ms | objects the table names |
| `Pages::new` (§7.7.3) | 0.031 | 0.045 | 0.245 | — (`/Count`) |
| `PageLabels::read` (§12.4.2) | 0.011 | 0.011 | 0.125 | label ranges |
| `Outline::read` (§12.3.3) | 0.047 | 0.355 | 2.981 | outline items |
| **`Pages::indices`** (in `section_at`) | **0.033** | **0.544** | **4.657** | **pages** |
| `section_at`'s own walk | 0.001 | 0.021 | 0.031 | outline items |
| everything else together | < 0.06 | < 0.06 | < 0.06 | — |

The three that scale are 96% of the 1023-page document's open and three different populations:
112 269 cross-reference entries, 1023 pages, 988 outline items. `Pages::indices` alone is 41% of
it.

Callgrind agrees and is not subject to this machine's two classes of core: of the open's
instructions, `pdf_model::outline::level` (which contains `Outline::read`) is 57.5 M inclusive and
`pdf_syntax::xref::read_section` 42.8 M, with `Document::get` — object parsing — 49.1 M spread
across both.

## Finding 2 — the walk was also being paid on every page turn, and that one is free to fix

`announce_page` runs on every `go_to`, and it built **both** a `Pages` and the whole index map
each time. `pdf_syntax::Document` is immutable for as long as it is open (rule 1), so that map is
a function of the file and is the same map every time.

So `Open` keeps it in a `OnceCell`, `Outline::section_at_with` takes a prepared map, and
`Destination::page_index_with` lost its `Pages` parameter — it never used it once a map was given,
and removing it is what lets `announce_page` construct no page tree at all. Measured as an A/B in
one binary (`open_cost` prints both), 1023 pages, warm:

| what a page turn pays for its caption | before | after |
|---|---|---|
| `Pages::new` | 0.245 ms | — |
| the section | 0.555 ms | 0.031 ms |

**0.77 ms off every arrow key on ISO 32000-2**, against `doc/checks/launch-path.toml`'s turn band
for that document — about a seventh of a page turn, and it is the part of the turn that scales
with the document rather than with the page.

**The open is unchanged by this**, deliberately: the map is built once either way, and the first
build is at open. What removing it from the open would cost is ADR 0891's subject.

## Consequences

- `open_cost` measures the path it claims to. A round that trusts an example's comment over its
  output repeats ADR 0885's mistake.
- The page-index map is `Open`'s, built once per document. A second consumer that wants it —
  §12.3.2's links resolve destinations the same way — has it already.
- Principle 2's *no full page-tree walk* is false as a flat sentence and true as the bullet
  beneath it means it: the walk is needed to answer a question about the page being shown. That is
  the second sentence of that bullet in the same shape as ADR 0885's font finding, and both are in
  `doc/questions/Q25`.
- No clause's reading moved. §12.3.3, §12.3.2 and §7.7.3 answer exactly what they answered; this
  is where the answer is computed, not what it is.

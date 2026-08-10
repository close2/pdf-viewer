# Which of this project's restrictions are load-bearing, and which are habit

Status: **raised by the project owner on 2026-08-10**, asking why search is single-threaded,
whether that is an over-restrictive *security* decision, which rules could be relaxed to simplify
the code or make the program faster, which are worth putting behind flags, and how much memory a
viewer may reasonably spend. This file is the audit, with each restriction's warrant separated from
its habit.
**Two of the five are settled in the four-hundred-and-twentieth session** (ADR 0256): item 2, the
readback cache, is built with a bound and eviction, and item 4 is subsumed by it exactly as this
file predicted. The flag `--cache-text` is **declined**, with the condition that would revive it
written into its entry below.
Priority: 49 — the project's own decisions, the band `43`–`48` already occupy. Nothing here is a
defect; several entries are cheap wins and one is decided outright by a number the owner supplied.
Code: `CLAUDE.md`, `doc/ui-boundary.md`, `crates/pdf-syntax/src/document.rs`,
`crates/viewer-core/src/viewer.rs`, `crates/viewer-core/src/search.rs`

## First, the question as asked: why is the search single-threaded?

**It is not a security decision, and it is not one decision.** It is two, and only the second is a
real constraint.

**1. `viewer-core` rule 4 — "no threads the core was not handed, and no blocking."** This is an
*embeddability* rule, not a security one. GTK, Qt, Win32 and AppKit all forbid touching the
interface from another thread and expect an application to return to its event loop promptly; a
core that spawned threads or blocked could not live inside one. It is also what lets the confined
worker and the C ABI drive a search with the same three verbs the window uses. **And note the
wording: "threads the core was not handed."** The rule already permits a core given a pool to use
it; what is missing is an API to hand one, not permission.

**2. The page loop is single-threaded; the work inside a page is not.** `interpret` already uses
rayon — `image.rs` bands §8.9.5's per-sample colour conversion across `rayon::current_num_threads()`
— so "single-threaded" was never true of the program, only of the loop over pages. What stops the
*loop* parallelising is neither rule: **`pdf_syntax::Document` caches parsed objects behind
`RefCell` and is therefore `!Sync`.** That is an implementation choice, not a principle, and it is
the actual blocker. Options, none free: `RwLock` or a sharded cache (lock traffic on the hottest
path in the program), or N documents in N threads (N parses, N caches — the memory the owner named).
**Measure the lock version before believing either.**

So: the honest answer is that the search is a pump for the hosts' sake, the work inside a page
already parallelises, and cross-page parallelism is blocked by a cache design that predates the
question.

## The memory question, and the owner's own number settles one item

> "I am also not against caching text for a search (and therefore using memory), it should just not
> use too much. (1 GB is definitely too much, below 10 MB is definitely ok. Not sure how this is
> usually handled. Should a user just expect a big PDF to use more memory? Should we have
> performance flags?"

**The readback of ISO 32000-2's 1023 pages is 2.66 MB** (ADR 0250 measured it). That is inside the
owner's "definitely ok" band by a factor of four, for the largest document this project owns — so
**the readback cache ADR 0250 declined is decided, and it was declined against a bound nobody had
been given.** What it needs is a *bound with eviction*, not an absence: a cap in megabytes,
least-recently-used, stated in one place, and observable.

On the general question, the honest answer: **yes, a reader should expect a large document to cost
memory, and every viewer works that way** — but the discipline that makes it acceptable is that the
cost is *bounded and legible*, not that it is small. This tree already has the pattern in three
places: `MASK_BUDGET` (32 MB), the confined worker's address-space ceiling (4 GiB), and
`MAX_SAMPLES` (halved on measurement in session 396). A fourth for the readback is the same shape.

## The audit: what is load-bearing, what is habit

**Keep, and they are not negotiable — these are the security decisions and they are cheap:**

- **`#![forbid(unsafe_code)]` in every crate that touches PDF bytes.** It has a real cost: session
  404 priced a `memfd` document hand-off and stopped on exactly this, and it forecloses SIMD
  intrinsics. Keep it anyway — hostile input reaching `unsafe` is the failure this project cannot
  recover from, and the escape hatch already exists in the right place (`viewer-qt` and
  `viewer-ffi` lift it; neither parses a document).
- **The sandbox, and `--no-sandbox` as the flag that trades it.** Already a flag, already the safe
  default, already prints what it gave up.
- **The resource budgets** (`MAX_OPERATIONS` 4 M, `MAX_FORM_DEPTH`, `MAX_TILES`, the decode
  deadline). These are what stand between a decompression bomb and the machine. A "trusted
  document" flag that lifted them is *plausible* but should be argued as a whole, not per constant.
- **`Document` immutable and `interpret` a pure function of (document, view state).** This looks
  like a restriction and is actually the foundation of the test strategy: the oracle's 1794-page
  comparison means something only because interpretation is reproducible. A **cache beside** it
  breaks nothing — purity is about the answer, not about how fast it is reached.

**Relax or re-examine — each of these is habit rather than warrant:**

1. **`Document`'s `!Sync`.** The blocker above. Worth one measurement round. **Still open**, and
   deliberately untouched by the four-hundred-and-twentieth, which was told to leave it alone.
2. **The readback cache.** ~~Decided by the owner's bound; needs eviction and a number.~~ **Done in
   the four-hundred-and-twentieth** (ADR 0256): `crates/viewer-core/src/readback.rs`, 4 MiB per open
   document, least-recently-used, one constant in one place, and readable through
   `Viewer::readback_cache` and `pdf-viewer --trace=search`. A repeated document-wide sweep of ISO
   32000-2 fell from 5.45 s to **7.27 ms** and the window's from about five seconds to **0.021 s**;
   the *first* search did not move, which is the honest half of the result. It lives beside
   `interpret` rather than inside it, for the reason the "keep" list above gives: purity is about
   the answer and not about how fast it is reached, and `pdf-model` gained nothing — not a `&mut`,
   not an interior mutability, not a lifetime.
3. **Rule 4's missing half.** The rule permits a handed pool and nothing hands one. If measurement
   says cross-page parallelism is worth it, the API shape is the question, not the permission.
   **Still open**, and it waits on item 1.
4. **`viewer-core` re-interprets a page per search step and throws the result away.** ~~Independent
   of threads: the same page interpreted for a search and then again to draw it is two
   interpretations. Item 2 subsumes it if the cache is keyed by page.~~ **Subsumed, as predicted**:
   the cache *is* keyed by page, and `settle` now puts the page it interpreted to draw into it, so
   a find bar's search no longer re-reads the page the person is looking at. The remaining half —
   the page a search *lands* on being interpreted again to draw it — cannot be subsumed, because
   drawing needs a display list and the cache deliberately holds only the readback.
5. **`MAX_CHILDREN` 65 536 in `Tree::walk`** — session 416 found ISO 32000-2's structure tree is
   larger (71 371), so `logical_order` sees only the front of that document. A bound that silently
   truncates the largest document this project owns is the wrong bound; `ParentTree::for_page` is
   the route that works and the walk should say so or grow.

**Worth a flag, and the tree already has the idiom** (`--no-sandbox`, `--cpu`, `--backend`,
`--ignore-restrictions`, `--trace=<topics>`):

- ~~`--cache-text[=MB]` or a general memory budget — the owner's question in flag form.~~
  **Declined in the four-hundred-and-twentieth, by measurement, and the condition to revive it is
  stated**: 4 MiB holds the whole readback of ISO 32000-2 — 2.66 MB, the largest document this
  project owns, against a corpus whose largest is `freeculture.pdf`'s 352 pages — with `evicted` at
  zero on every run. There is no document in reach whose owner would want to type a number, and the
  rule three bullets below says what a flag may not be. **Build it when a document arrives whose
  readback exceeds the budget**, which the report says out loud: LRU under a forward sweep is
  exactly the pathological case, and a non-zero `evicted` beside a search that is slow twice is the
  measurement that justifies the knob.
- `--threads=N`, if item 1 goes anywhere; also the honest place to expose "use one thread" for
  reproducing a bug.
- **What a flag may not be**: a way to avoid deciding. `CLAUDE.md` principle 1 is that a shortcut is
  documented as a deliberate decision with its cost, never taken silently — and a knob whose default
  is wrong is a decision deferred onto the user. Every flag here should have a right default and
  exist for the person who knows their document.

## What this file is not

It is not a licence to relax the four in the "keep" list because a round found them inconvenient.
Each has a written argument; `CLAUDE.md`'s own rule is that an exclusion is revisited **by argument,
never by attrition**, and that applies to the restrictions as much as to the scope.

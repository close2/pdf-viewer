# Which of this project's restrictions are load-bearing, and which are habit

Status: **raised by the project owner on 2026-08-10**, asking why search is single-threaded,
whether that is an over-restrictive *security* decision, which rules could be relaxed to simplify
the code or make the program faster, which are worth putting behind flags, and how much memory a
viewer may reasonably spend. This file is the audit, with each restriction's warrant separated from
its habit.
**Two of the five are settled in the four-hundred-and-twentieth session** (ADR 0256): item 2, the
readback cache, is built with a bound and eviction, and item 4 is subsumed by it exactly as this
file predicted. The flag `--cache-text` is **declined**, with the condition that would revive it
written into its entry below. **A third is settled in the four-hundred-and-twenty-first** (ADR 0257):
item 5's bound was not only wrong but hiding a quadratic walk, and both are fixed. **Item 1 is
measured and settled in the four-hundred-and-twenty-fourth** (ADR 0260), and the measurement moved
item 3 rather than unblocking it: what is left is item 3 alone, and it is now a question about
memory rather than about a lock.
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
~~**Measure the lock version before believing either.**~~ **Measured in the
four-hundred-and-twenty-fourth, and the sentence above is wrong in the one word that matters**: the
`RefCell`s were not *the* blocker, because **N documents in N threads needs nothing from
`pdf-syntax` and was available the whole time** — and on this machine it is the faster of the two,
1.18 s against 1.61 s over 1023 pages on 24 threads. What `!Sync` blocked was the *cheaper* of the
two in memory, not the faster one. The lock itself costs **0.021%** of a page interpretation's
instructions and nothing measurable in wall clock, so it shipped; the parallel search did not, at
625 to 966 MB of peak resident against 225. ADR 0260 has every table.

So: the honest answer is that the search is a pump for the hosts' sake, the work inside a page
already parallelises, and cross-page parallelism is not blocked by anything — it is **declined on
memory**, which is a different sentence and the one the measurement supports.

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
  **What they cost is measured, over 65 944 crawled documents** (ADR 0269): 48 reach `MAX_TILES`,
  31 `MAX_OPERATIONS`, 4 `MAX_FORM_DEPTH` and 1 `MAX_STATE_DEPTH` — **84 refusals over 83
  documents, 0.127% of the web** (the count of *documents* was written as 84 until the
  four-hundred-and-thirty-fifth session; `7680183.pdf` reports two of them), against 0.2% of
  session 430's 4000 and 0.105% of session 425's 1896, so the rate is stable at three sample sizes.
  ~~What is still owed is one of the 84 read with the bound lifted in a scratch build, to
  find out whether the constant costs a mark or stops a bomb.~~ **Done for all 83 in the
  four-hundred-and-thirty-fifth** (ADR 0271), one process apiece with the bound lifted, and the
  answer is different for each of the four:

  | bound | the population, lifted | why it stays |
  |---|---|---|
  | `MAX_FORM_DEPTH` 16 | **all four documents are cycles** — lifted to 256, all four reach 256 | the attack it exists for. Unbounded recursion exhausts the *stack*, which the confined worker's 4 GiB ceiling does not see and which Rust turns into an abort rather than a report |
  | `MAX_TILES` 4096 | all 48 terminate, 0.06–14.2 s, wanting 4104–895 500 tiles; **14 of 48 want under twice the bound** | 1 000 000 *empty* tiles interpret in 889 ms **reporting nothing** — an empty cell executes no operator, so `MAX_OPERATIONS` never sees it and this is the only bound on the loop |
  | `MAX_OPERATIONS` 4 M | all 31 terminate, wanting 4.1–53.6 M **lexer tokens** — the word was *operators* here and in ADR 0271 and the counter was counting tokens (ADR 0306) — 0.27–49.9 s; the worst peaks at 1.57 GB for 495 marks | **a count is not a cost** — one `sh` paints the page — so no larger number bounds the time either. The cancel does, at 0.83–1.97 ms |
  | `MAX_STATE_DEPTH` 256 | one document, wanting **337** | ISO 32000-2 §C.2's Table C.1 prints **28** as the depth a writer could rely on. 256 is nine times the standard's own figure and the document wants twelve times it |

  **And the two documents that are slow are still not among them**, which was true before this
  reading and is the shape of the answer: the bound stops the work inside the per-document budget
  rather than after it. Both slow documents were diagnosed in the same session and neither was a
  budget at all — see the new item below.
- **`Document` immutable and `interpret` a pure function of (document, view state).** This looks
  like a restriction and is actually the foundation of the test strategy: the oracle's 1794-page
  comparison means something only because interpretation is reproducible. A **cache beside** it
  breaks nothing — purity is about the answer, not about how fast it is reached.

**Relax or re-examine — each of these is habit rather than warrant:**

1. ~~**`Document`'s `!Sync`.** The blocker above. Worth one measurement round.~~ **Done in the
   four-hundred-and-twenty-fourth** (ADR 0260). The five `RefCell`s are `RwLock`s, `Document` is
   `Send + Sync` with a compile-time assertion saying so, and the `loading` set is **per thread**
   rather than per document — the one hazard the swap could have introduced, since a shared set
   would answer §7.3.10's null to the second of two threads that wanted one object at one moment.
   Cost: **+0.021%** instructions through `callgrind_interpret` (2 208 807 721 → 2 209 269 060),
   **−0.14%** through `callgrind_open`, and a cold sweep inside its own spread over seven
   interleaved samples apiece. What the counter build found on the way is the part worth keeping:
   `Document::get` is asked **829 times a page** and answers 92.7% of a cold sweep from the cache —
   **and a fully warm cache is worth 5.5% of the wall clock**, so the object cache is not where a
   sweep's seconds are.
2. **The readback cache.** ~~Decided by the owner's bound; needs eviction and a number.~~ **Done in
   the four-hundred-and-twentieth** (ADR 0256): `crates/viewer-core/src/readback.rs`, 4 MiB per open
   document, least-recently-used, one constant in one place, and readable through
   `Viewer::readback_cache` and `pdf-viewer --trace=search`. A repeated document-wide sweep of ISO
   32000-2 fell from 5.45 s to **7.27 ms** and the window's from about five seconds to **0.021 s**;
   the *first* search did not move, which is the honest half of the result. It lives beside
   `interpret` rather than inside it, for the reason the "keep" list above gives: purity is about
   the answer and not about how fast it is reached, and `pdf-model` gained nothing — not a `&mut`,
   not an interior mutability, not a lifetime.
3. **Rule 4's missing half.** The rule permits a handed pool and nothing hands one. **Still open,
   and item 1 changed what it is about.** It is no longer waiting on a lock — it is waiting on a
   memory argument. `parallel_sweep` puts a 1023-page sweep at 1.61 s shared or 1.18 s per-thread
   against 6.11 s on one thread, for **625 MB or 966 MB of peak resident against 225 MB**; the
   owner's own bar is that 1 GB is definitely too much. A round taking this owes three things:
   the API that hands a pool in (and whether the core takes a `&ThreadPool` or a
   `dyn Fn(&dyn Fn())` so that a host with no rayon can supply one), **a bound on how far ahead a
   search may read**, since a search stops at the first match in document order and N threads
   reading N pages ahead throw most of it away, and the arrangement chosen with its memory named
   — shared costs less and stops scaling past eight threads; per-thread costs more and does not.
   ADR 0260 §4.
4. **`viewer-core` re-interprets a page per search step and throws the result away.** ~~Independent
   of threads: the same page interpreted for a search and then again to draw it is two
   interpretations. Item 2 subsumes it if the cache is keyed by page.~~ **Subsumed, as predicted**:
   the cache *is* keyed by page, and `settle` now puts the page it interpreted to draw into it, so
   a find bar's search no longer re-reads the page the person is looking at. The remaining half —
   the page a search *lands* on being interpreted again to draw it — cannot be subsumed, because
   drawing needs a display list and the cache deliberately holds only the readback.
5. ~~**`MAX_CHILDREN` 65 536 in `Tree::walk`**~~ — **done in the four-hundred-and-twenty-first, and
   it was worse than this entry recorded** (ADR 0257). The bound was on *items over the whole tree*
   and it overshot, so the 71 371 session 416 wrote down was the bound rather than the tree: it is
   **129 389**, and `logical_order` walks the whole tree once per page, so §14.8.2.5's reading order
   for any page of ISO 32000-2 was a prefix. Two things were wrong and both are fixed. The bound is
   now `MAX_ELEMENTS` at 2²⁰ — eight times that tree — separate from `MAX_CHILDREN`, which stays and
   bounds one `/K` array; and it **reports**, through `Reading::truncated`, which `logical_text` (now
   `Option<String>`) and `logical_range` refuse on rather than answering a prefix. **And the walk was
   quadratic**, its visited set a `Vec<Dictionary>` searched linearly and compared whole: keyed by
   `ObjectId` it is **16.8 s → 151 ms**. `pdf-model/tests/structure.rs` holds the count and the flag,
   which is the assertion that did not exist to be made.

## What the census left open: two bounds count the wrong quantity

**Raised by ADR 0271 and not taken there**, because neither is a constant to move — each is a
change to *what* is bounded, and both need the argument before the code.

- **`MAX_TILES` bounds a count where it means to bound work.** `7680183.pdf` wants 42 282 tiles
  and takes 14.2 s; `2760154.pdf` wants 765 440 and takes 8.7. So the number that decides admits
  the expensive document and refuses the cheap one, and raising it would move an arbitrary line
  rather than a wrong one. What the bound is really for is the *loop* — an empty cell at 0.89 µs
  a tile is four days at the trip count a file may state — so the shape wanted is a budget over
  cells replayed *and* operators executed, checked as the loop runs, with the same refusal by
  name at the end. The empty-cell measurement is the one that says the count cannot simply be
  dropped in favour of `MAX_OPERATIONS`.
- **`MAX_OPERATIONS` has the same defect one layer up**, and its population says so: 30 of the 31
  documents it stops are legitimate drawings wanting 4.1–53.6 M tokens, and the thirty-first
  produces 495 marks from 53.6 M. A count cannot tell them apart because one operator's cost is
  unbounded. The honest instrument is a *deadline*, which this tree already has in the confined
  worker (ADR 0241, a kill at 0.83–1.97 ms) — so the question is whether `interpret` should carry
  one of its own for the unconfined path, and what a host that is not the viewer does with it.
- **And it was counting the wrong quantity as well, which is a different fault from the one above
  and is fixed** (ADR 0306). Every "operators" in this file's budget rows means *lexer tokens*: the
  one increment site was the token loop, and §7.8.2 puts an operator after its operands, so a `c`
  cost seven. The counter now counts operators, the value stays at four million, and the
  re-measurement is 48 pages of 926 680 past four million tokens against **8** past four million
  operators. The argument in the two bullets above survives it unchanged — a count is still not a
  cost — but the *rate* this file quotes was measured through the old unit and a round re-running
  the survey should expect the `MAX_OPERATIONS` row to fall.

Neither is a defect today: both bounds refuse loudly and both refuse 0.127% of the web. This is
here so that a round which wants to admit `MAX_TILES`' 48 knows the price is a new mechanism
rather than a bigger number.

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
- `--threads=N`, if item 3 goes anywhere; also the honest place to expose "use one thread" for
  reproducing a bug. **And it is now the flag with a right default to find**: the measurement in ADR
  0260 says the answer is not "as many as the machine has" — shared stops improving at about eight
  and the memory keeps climbing to 24.
- **What a flag may not be**: a way to avoid deciding. `CLAUDE.md` principle 1 is that a shortcut is
  documented as a deliberate decision with its cost, never taken silently — and a knob whose default
  is wrong is a decision deferred onto the user. Every flag here should have a right default and
  exist for the person who knows their document.

## What this file is not

It is not a licence to relax the four in the "keep" list because a round found them inconvenient.
Each has a written argument; `CLAUDE.md`'s own rule is that an exclusion is revisited **by argument,
never by attrition**, and that applies to the restrictions as much as to the scope.

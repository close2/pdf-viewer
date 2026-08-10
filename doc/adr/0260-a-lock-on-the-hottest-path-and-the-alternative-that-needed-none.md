# ADR 0260 — A lock on the hottest path, and the alternative that needed none

Status: accepted, 2026-08-10 (session 424).

## Context

The project owner asked why the search is single-threaded and whether that was an over-restrictive
security decision. `doc/todo/49` answered the *why* — `viewer-core`'s rule 4 is an embeddability
rule and not a security one, and the work *inside* a page already parallelises through rayon — and
named the real blocker:

> **`pdf_syntax::Document` caches parsed objects behind `RefCell` and is therefore `!Sync`.** That
> is an implementation choice, not a principle, and it is the actual blocker. Options, none free:
> `RwLock` or a sharded cache (lock traffic on the hottest path in the program), or N documents in
> N threads (N parses, N caches). **Measure the lock version before believing either.**

This round is that measurement. `CLAUDE.md` principle 2 says "genuinely" is decided by measurement
and never by assumption, and there were two assumptions in the sentence above: that the caches are
hot enough for a lock to matter, and that sharing a document is the way to parallelise a sweep.
**One of them is false and the other is smaller than it sounds.**

## 1. What the five caches actually do

`document.rs` holds five `RefCell`s — an object cache, expanded object streams, a loading set, the
file's own object headers, a misfiled set. Nobody had ever counted what they are asked. The
instrument was a **temporary counter build**: one `AtomicU64` per call site inside `document.rs`
and a `cache_census` example that resets them between phases. It is not in the tree, because a
counter on `Document::get` is a counter on the hottest path in the program; the patch is 136 lines
and the numbers below are what it printed. Every count is deterministic and reproduced exactly
across runs.

ISO 32000-2, 1023 pages, 101 318 objects, `--profile gates`:

| phase | `get` | of those, hits | `load` (misses) | `expand` | `resolve` | `get_key` | `decode` |
|---|---|---|---|---|---|---|---|
| `Document::open` | 1 | 0 | 1 | 0 | 1 | 0 | 0 |
| page 1, cold | 113 | 49 | 64 | 36 | 5 341 | 755 | 23 |
| page 2, a page turn | 133 | 80 | 53 | 41 | 3 881 | 813 | 17 |
| whole document, cold | 848 423 | 786 587 | 61 836 | 57 053 | 8 910 914 | 4 373 662 | 12 717 |
| per page | **829.3** | **768.9** | 60.4 | 55.8 | 8 710.6 | 4 275.3 | 12.4 |
| the same sweep again | 848 423 | **848 423** | **0** | **0** | 8 908 688 | 4 370 694 | 11 975 |

Four things follow, and three of them were not what the file expected.

**The object cache answers 92.7% of a cold sweep and 100% of a repeat, and that is worth 5.5% of
the wall clock.** 6.15 s cold against 5.81 s with every object already parsed (medians of three,
6.15/5.89/6.24 and 5.81/5.79/5.92). A cache that removes *all* of the parsing removes a twentieth
of the time — so whatever a sweep is spending its seconds on, it is not `Document::get`.

**`resolve` is called ten times as often as `get` and mostly touches no cache at all.** 8 710 a
page against 829: the rest are `Object`s that are not references, which `resolve` returns without
looking anything up. Only `get` takes a borrow, so the borrow count per page is about
829 reads + 60 writes on the object cache, 2 × 60 on the loading set and 56 on the expanded
streams — **roughly 1 070 borrows for 6 ms of work**, one every 5.6 µs.

**`headers` and `misfiled` are never touched on any of the 974 corpus documents' happy path**, and
were not touched once here. They are the repair path, and the field comments already said so.

**And a module doc comment was false.** "Objects are parsed when asked for, and decoded streams are
cached" — `decoded_stream_data` runs 12 717 times over one sweep and **11 975 times over the second
sweep of the same document**, which is a filter chain re-run and not a cache read. What is memoised
is §7.5.7's object streams, whose contents are objects. The sentence is corrected and
`doc/todo/47` carries the question of whether the rest should be memoised, which is a question
about a byte budget rather than about a map.

## 2. What `Sync` costs single-threaded

The five `RefCell`s become `std::sync::RwLock`. No dependency was added: a concurrent map crate
would bring `unsafe` into the crate hostile bytes reach first, which is ADR 0186's shape of
decision and not a detail, and the measurement below says nothing needed one.

Two things changed beyond the type name.

- **Poisoning is ignored, deliberately and in one place.** `read` and `write` are two four-line
  helpers that take `unwrap_or_else(PoisonError::into_inner)`, with the reason written above them:
  these locks hold no invariant across fields, every write is one `insert` into one collection, and
  a panic between two of them leaves a map that is merely smaller than it could have been.
  Propagating the error would turn a panic anywhere in the process into a document that can no
  longer be read.
- **The `loading` set became per thread**, and this is the one place where "make it `Sync`" was not
  mechanical. It is a guard on the *call stack* — `get` is re-entrant because loading an object can
  need an indirect `/Length`, an indirect `/Filter` or the object stream it lives in. A set shared
  between threads would answer §7.3.10's null to the second of two threads that happened to want
  one object at one moment: **a wrong answer produced by timing**, which is the one kind this
  program must not have. `the_recursion_guard_is_per_thread_rather_than_per_document` is the test
  that would have caught it.

What it costs, in instructions, which do not move with the machine's load:

| | `RefCell` | `RwLock` | |
|---|---|---|---|
| `examples/callgrind_interpret` (page 101 of ISO 32000-2, ×50) | 2 208 807 721 | 2 209 269 060 | **+0.021%** |
| `examples/callgrind_open` (ISO 32000-2, §7.5 alone) | 78 464 732 | 78 357 201 | **−0.14%** |

Both are exactly repeatable — two runs of each gave the same figure to the instruction. The second
is negative because the `loading` set is now a `HashMap` that opening never populates.

And in wall clock, `viewer-core/examples/find_cost` on the same document, **seven samples of each,
interleaved run by run** so that neither gets a quiet machine the other did not:

| a cold document-wide sweep, 1024 steps | median | range |
|---|---|---|
| `RefCell` | 5.69 s | 5.61 – 5.91 |
| `RwLock` | 5.78 s | 5.54 – 5.90 |

0.09 s of median against spreads of 0.30 and 0.36 s: **unchanged, and inside the spread**, which is
what the instruction counts predicted.

The launch path, `pdf-viewer --trace` under `Xvfb` with lavapipe, twelve samples of each binary
interleaved: **`document joined` is 5.46 ms against 4.95 ms**, both ranging over about 2 ms.
The whole-launch figure is not worth quoting against itself here and the reason is worth writing
down — `EventLoop::new` ran 28 to 55 ms across those twenty-four launches, **and which side of that
range a run landed on depended on whether it was the first launch after the X server went idle
rather than on which binary it was**. Reversing the order reversed the apparent difference. The
launch stage this change can touch at all is the document's, and that one did not move.

## 3. What parallelism buys, and against what

`pdf-model/examples/parallel_sweep` is the instrument and it is in the tree, because the
measurement is the whole justification for this ADR and has to be repeatable. It sweeps every page
three ways, adds up the text each read so that a run which skipped a page cannot look fast, and
runs every parallel section inside a pool built with exactly N threads — `interpret` bands
§8.9.5's colour conversion across `rayon::current_num_threads()` of its own, so splitting the pages
N ways inside the *global* pool would have measured N tasks on 24.

- **one** — one thread, one document, which is what the viewer does today;
- **shared** — N threads over one `&Document`, which is what this round made possible;
- **per-thread** — N documents opened from the same bytes, one to a worker: no shared state, no
  lock, N parses and N caches.

All three read the same 2 658 697 bytes at every thread count. Medians of three, on a 12-core /
24-thread Ryzen AI 9 HX 370 carrying a background load average of about 4:

| threads | one | shared | per-thread |
|---|---|---|---|
| 1 | 5.97 s | 6.02 s | 6.10 s |
| 2 | 5.91 s | 3.31 s | 3.39 s |
| 4 | 6.01 s | 1.93 s | 1.98 s |
| 8 | 6.08 s | 1.59 s | 1.50 s |
| 16 | 6.07 s | 1.48 s | 1.31 s |
| 24 | 6.11 s | 1.61 s | **1.18 s** |

And the same sweep repeated on warm caches, at 24 threads: **shared 1.11 s, per-thread 1.22 s** —
the one place the shared arrangement wins, and it wins for a legible reason. A cold shared sweep
takes 61 836 write locks; a warm one takes none, so the contention that costs it 1.61 s goes away
while the per-thread arrangement re-opens 24 documents and re-parses everything it had.

Memory, `VmHWM` from `/proc/self/status`, one arrangement per process, two sweeps each:

| | 1 thread | 8 threads | 24 threads |
|---|---|---|---|
| one | 225 MB | — | — |
| shared | — | 398 MB | 625 MB |
| per-thread | — | 488 MB | 966 MB |

## 4. What that decides

**Neither arrangement needs the other's permission, and the one that scales furthest needs no
`Sync` at all.** Up to eight threads the two are inside each other's spread; past that the shared
one stops improving because 61 836 cache misses are 61 836 exclusive locks, and the per-thread one
keeps going. So the honest headline is not "the lock is affordable, ship the parallel search". It
is: **cross-page parallelism was never blocked by `!Sync`** — N documents in N threads has been
available the whole time and is the faster of the two on this machine — and what `!Sync` blocked
was the arrangement that costs *less memory*, not the arrangement that goes fastest.

**The `RwLock` change ships**, on three grounds and not on the speedup:

1. It costs 0.021% of the instructions a page interpretation executes and nothing measurable in
   wall clock, measured rather than argued.
2. It makes the comparison in §3 repeatable. Without it `shared` cannot be built at all, and an ADR
   whose central table nobody can reproduce is an opinion.
3. It corrects `loading` from a document-wide set to what it always meant, which is a per-stack
   one.

**The parallel search itself is declined this round**, and the reasons are numbers rather than
taste:

- **Rule 4 is not the obstacle and is not being bent.** `viewer-core` spawns nothing; the pool
  would be *handed* to it, which rule 4 already permits and which `doc/todo/49` item 3 is the
  design of. That API does not exist and inventing it in the same round that measured the case for
  it would be deciding two things with one argument.
- **The memory is outside the band the owner stated.** 966 MB at 24 threads and 625 MB shared,
  against 225 MB for the loop the viewer runs today and against "1 GB is definitely too much". A
  4× speedup on a *first* search, bought with 2.8× to 4.3× the peak resident memory, is a trade
  somebody has to want; ADR 0256 already made every search after the first cost 7.27 ms for
  2.66 MB, which is the same problem answered inside the band.
- **A sweep is the wrong shape for a viewer's search anyway.** A search stops at the first match in
  document order, so N threads reading N pages ahead throw away most of what they read; the
  worst-case miss measured here is exactly the case where they do not. That is a design question
  and it belongs with the API, not with the lock.

**What is not declined and is not this round's**: `doc/todo/47`'s remaining half — a fifth of a
cold sweep is `Pages::get` walking §7.7.3.2's tree from the root once per page — and the decoded
stream chain found in §1, which runs 11 975 times on a *second* sweep of a document nothing had
changed. Both are single-threaded wins, and a single-threaded win costs no memory at all.

## 5. Cost, written down

- **`document.rs` is +174 / −34 lines**, and the split is the point: **66 of the additions are the
  two new tests** and about 45 more are doc comments carrying the arguments above. What is actually
  executed is four small functions — two lock helpers and two `loading` helpers — and five field
  types.
- **+2 tests**, and one of them is a compile-time assertion that `Document: Send + Sync`, which is
  the property the whole of §3 rests on and which a stray `Rc` anywhere in the object graph would
  take away silently.
- **+1 example**, `parallel_sweep`, which is the measurement.
- **The temporary counter build is not in the tree**, deliberately: `Document::get` is asked 829
  times a page and a permanent counter on it would be exactly the unmeasured cost this ADR was
  written to avoid. What replaces it is §1's table.

## What did not move

Every gate: the corpus's 974 with 70 incomplete, the oracle's 905/68/786 over 1794 pages with
1688/106 and the undiagnosed list empty, quorra's 912/36/9/17, the text gate's 99.2%
(24003/24187) and PDFBox's 99.8% (14257/14281), dates 1514/1545, XMP 318 read of 319, JPEG 2000's
14 byte-identical, and the ledger's 875 rows at 401 implemented / 251 partial. Tests 1542 → 1544.

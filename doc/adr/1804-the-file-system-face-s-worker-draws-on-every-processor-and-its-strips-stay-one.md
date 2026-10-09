# 1804 — The file-system face's worker draws on every processor, and its strips stay one

Session 1484. Status: **accepted** and **built**. Takes the speed question ADR 1774 section 5 left:
whether `pdf-vfs-worker`'s pool should be as wide as the machine now that `serve::STRIPS` is one
and `MALLOC_ARENA_MAX` covers every worker. Amends ADR 0218 section 2 for this worker as ADR 1554
did for `viewer-confined`. Context: ADRs 0218, 0865, 1554, 1742, 1774; `CLAUDE.md` principle 2.
Code: `crates/pdf-vfs/src/serve.rs` (`ARENA_LIMIT_VARIABLE`, `RASTERISING_STACK`, `confine`,
`message_budget`, `WorkerLimits::threads`). Tests: `tests/confined.rs`'s
`a_pool_built_inside_the_confinement_under_an_arena_limit_is_as_wide_as_the_machine`; `serve.rs`'s
`a_message_budget_leaves_room_for_two_copies_a_page_and_the_pools_stacks`. Instrument:
`scratchpad/r1484/` (`ab.sh`, `cost.py`, `sample.sh`).

## 1. Measured

One frozen binary set (the gates profile, the worker's width chosen at spawn by a variable that
was never committed), arms interleaved, under one `--clock` hold each, 2026-10-09.

- **The walk does not shorten.** `--test read_corpus`, 1 159 documents, pinned to the eight fast
  CPUs (`taskset -c 0-3,12-15`, derived from `cpuinfo_max_freq`), load average 3.6 to 4.4: one
  thread 147.3 s and 163.0 s, eight threads 155.1 s and 163.0 s. Unpinned, load average 3.6 to
  19.2: one thread 245.6 s and 180.0 s, twenty-four 160.0 s and 237.8 s — the machine, not the
  width. Every one of the eight runs exits 0 and prints the same count lines: 3 217 renders their
  generator's bytes, 0 not, 0 killed.
- **Where the walk's time goes.** The test process's own processor time and its reaped children's
  (`/proc/<pid>/stat`, sampled each second): pinned, 267–274 s in the test and 295–319 s in the
  workers, against 147–163 s of wall. That is the walk's four `rayon` lanes busy 3.5 to 3.8 at a
  time: each lane asks its mount, waits, then computes the same file itself, so the walk is about
  (test + workers) ÷ 4, and a wider worker shortens only the part of an answer that has parallel
  work in it — little over a corpus most of whose pages have no image. The wider pool spent 4–8%
  more worker processor time doing it.
- **The question a face's user waits on does shorten.** A mount's worker answers one question at
  a time (`confined::Confined`), so `cp renders/300dpi/0001.png` waits for exactly one answer.
  `examples/vfs_cost`, its confined column, pinned as above, the minimum of three processes of
  five asks each: `issue19971.pdf`, page one's text 74.1 → 35.5 ms, its 150 dpi render 116.0 →
  78.2 ms, its 300 dpi render 202.3 → 164.8 ms; `bug1815476.pdf`'s images 9.87 → 7.63 ms. Of the
  twenty questions over five documents, none is slower by more than 3%, on answers under a
  millisecond. Inside each run, confined ÷ in process (the in-process column is the run's own
  control on a fixed pool) on `issue19971.pdf` is 1.47–1.83 at one thread and 0.94–0.99 at eight.

## 2. Decision

**`confine` builds the pool as wide as `available_parallelism` answered before the confinement,
where `MALLOC_ARENA_MAX` is a positive number, and one thread where it is not** — `viewer-confined`'s
arrangement (ADR 1554), and for the same reason it is safe. The brief's fallback was "if the walk
does not shorten, the width stays one". It is not taken, because the walk is the wrong figure to
decide this on: it measures a test's throughput over four mounts at once, and `CLAUDE.md` principle 2
ranks the latency a person waits on above throughput. That latency falls by up to half on a page with
an image and rises nowhere, and the walk is unchanged within its own spread.

- **`STRIPS` stays one**, ADR 1774's rule and reason: the face's gate holds bytes. What the pool is
  wide for is the parallel work inside a page that is not the strip split — an image's bands, a
  shading's rows, `render-cpu`'s per-pixel pass — each a function of its own rows, so no band boundary
  changes a byte. The walk at width eight and twenty-four is the evidence; ADR 1774 section 3's plant
  is the calibration that the walk sees a byte the strip count moves.
- **The pool's stacks are stated (2 MiB) and are a term of the message budget**, because the pool is
  built after the address space in use was read. `message_budget` takes the width; `WorkerLimits`
  carries it, so the budget re-derived when a document opens counts the same stacks.
- **The pool is built after the confinement**, so every thread inherits the Landlock domain and the
  seccomp filter, as before.

## 3. What holds it

The new probe confines itself under `MALLOC_ARENA_MAX=1`, holds the pool to the width its unconfined
parent read, extracts the two images of the Tagged-PDF guide's page 60 (ADR 0865's killing page),
draws that page at 300 dpi, and broadcasts twenty rounds of 4 MiB allocations to every thread.
Calibrated (trap 13): with the arena check forced false, the probe exits 17 (`REFUSED`) and the test
fails.

## 4. Cost, and what is left

- 4–8% more processor time in the worker over the walk, and its peak over the process tree 1.99–2.08
  GiB pinned against 1.96–1.98 at one thread.
- `confined_transport::Host::start` said both workers rasterise on one thread; that sentence was false
  for `viewer-confined` since ADR 1554 and now for this worker, and it says what is.
- The walk's own wall would shorten by giving the test more lanes, not the worker more threads; that
  is the walk's lane count, held at four by the lock's measured ceilings (ADR 1766), and not changed.

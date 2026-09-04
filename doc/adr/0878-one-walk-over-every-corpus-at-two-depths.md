# 0878 — One walk over every corpus, at two depths, and what one document cost to learn it

Session 919. Status: **accepted**. The first of this round's two records: two instruments that each
asked half a question become one, and the population that made them two becomes a crate.

## Context

Session 914 built `crates/pdf-vfs/tests/read_corpus.rs` (ADR 0871), which asks whether the two
transports **agree** — every file of RFC 0003 section 4's layout, over `doc/pdf.js`'s 974
documents, held byte for byte against the generator `crate::layout` names. Session 917 built
`crates/pdf-vfs/tests/awkward_classes.rs` (ADR 0877), which asks whether the confined worker
**survives** — ten document classes, over a stride sample of every corpus root on the disk.

ADR 0877 said plainly what should happen to them: "one instrument should eventually do both, and
the merge is `read_corpus.rs`'s: widen its population beyond `doc/pdf.js` and its byte comparison
covers these classes too. What stops that today is cost … and cost is a reason to keep two
instruments, not to keep two designs." `doc/todo/58` §4 carried it. This is that round.

## Decision

**One walk, one population, two depths.**

- **The population is every corpus root on this disk**: `doc/pdf.js` whole, and every other root —
  the `doc/corpora` submodules, the `corpus-cache` collections — sampled at a fixed stride,
  classified, and the first `PER_CLASS` documents of each class walked beside it. Every document of
  the population is classified, pdf.js's included, so the matrix the run prints is over the whole
  walk rather than over its widening.
- **What came over from the deleted instrument is the half a byte comparison does not have.** A
  **death** — a worker killed by a signal, which `confined-transport` words as `killed by signal N`
  — is told from a refusal by that sentence, wherever it appears: an open, a listing, a read, or a
  comparison against a generator that succeeded. Any death fails the run. Each mount is then asked
  one more question after its walk, so that session 902's recovery of a dead worker is measured
  rather than claimed, and the matrix is printed per class because that is the only thing that can
  say *which* property killed a worker (ADR 0877's own finding).
- **Two depths, and only the reads are bounded.** A `doc/pdf.js` document is walked exactly as it
  was — sixteen pages, every entry of every directory — which is what keeps the figures printed
  since session 914 comparable with the sessions that wrote them down. A widened document is walked
  to two pages and four entries of a directory. Listings are whole on both sides, held to the
  layout's own names in order; what the bound changes is how many files are `stat`ed and read, and
  the run prints how many entries and pages it listed and did not read.
- **The classes are `crates/corpus-classes`**, a crate rather than a helper in the test file,
  because `viewer-confined` sweeps the *other* confined program over the same population (ADR 0879)
  and two copies of a population are two populations. `test-scenes` is the precedent and the reason
  is the same one: a comparison between two instruments that build their own inputs is a comparison
  of the inputs.

## What one document cost, which is why the second bound exists

The widening was written first with one depth, and it ran into
`corpus-cache/tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf`. The walk was still inside that
**one document after twenty-five minutes**, twice — at sixteen pages and, after the first bound was
added, at two — with the whole rest of the population finished and one worker at 100% of a core.
Every individual question of it is fast: `examples/vfs_cost` answers its page count in 131 ms, a
page out in 214 ms, its images in 200 ms and a 300 dpi render in 305 ms.

What it states is **10 084 images on one page**, each two pixels by one, `DCTDecode`. So
`/images/0001/` is a directory of ten thousand files, and this walk `stat`s and reads every entry
it lists — twenty thousand questions. A `stat` generates (RFC 0003 section 5.5); a read puts a
whole extraction run in the cache at once; and **a run too large for the cache's budget is put
nowhere at all**, which is round 911's own finding about `Cache::put` (ADR 0865 §3). So each of
those twenty thousand questions re-ran an extraction of ten thousand images.

Bounded to four entries, the same document takes **7.6 s** and the 10 077 entries it did not read
are counted and printed.

**That is a fact about the mount before it is a fact about the walk**, and it is the sharpest
witness `doc/todo/58` §5's "a `stat` generates, and on a large document that is minutes" has had:
the shortfall was recorded for a *long* document (1023 pages, 2 min 45 s) and it is worse for a
*wide* directory, where the cache's own budget turns it from linear into quadratic. The todo file
carries it.

## What it prints

```
vfs-read: 1132 documents in 315.5s, 24 threads, confined transport, 16 pages a doc/pdf.js document and 2 of every other root's
vfs-read:   pdf.js: 974 classified, 974 walked          safedocs: 1200 classified, 37 walked
vfs-read:   format-corpus: 167 classified, 25 walked     tika-issue-tracker: 1200 classified, 42 walked
vfs-read:   pdf-differences: 37 classified, 10 walked    openpreserve: 267 classified, 32 walked
vfs-read:   pdf20examples: 7 classified, 6 walked        pdfbox: 64 classified, 6 walked
vfs-read:   directories listed: 11664, entries stat'd: 20976, files read: 14274 (784.8 MiB)
vfs-read:   not the generator's bytes: 0        the two transports disagree: 0
vfs-read:   killed: 0                           did not recover: 0
vfs-read:   encrypted 45 documents, locked 5, encryption unimplemented 2, pageless 9, damaged 60,
vfs-read:   unopenable 8, huge 30, jbig2 119, jpeg 2000 35, plain (control) 847
```

**It is faster than the walk it replaces**, which was not the expected result: 1132 documents in
315.5 s against 974 in 344.6 s on the same tree an hour earlier. The widening costs about a fifth
of the population and the `text/` fix pays for it — `text()` interpreted *every* page of a document
longer than the walk's depth in order to throw the text away, because `document.txt` is the joined
readback and a document past the depth has no expectation for it. Reading past what will be
compared is what made a long document cost what a long document costs.

## Trap 13: the merged instrument was run against the defect

`no_machine_fonts()` commented out of `pdf_vfs::serve::confine`, `pdf-vfs-worker` rebuilt, the same
1132 documents: the deaths reappear, in the classes and in the count, and the run fails on them.
The figures are in this round's history file, beside session 917's 76 over 258 documents.

## Consequences

- **One line of `doc/todo/02` §2 instead of two**, and `crates/pdf-vfs/tests/awkward_classes.rs` is
  deleted rather than inherited, which is what `doc/todo/58` §4 asked for.
- **A knob with a reason.** `PER_CLASS` is what the widening costs and it is set against this
  walk's wall clock rather than for coverage. A round that wants a class swept deeper raises it for
  a run and does not commit the raise.
- **`PDFVFS_READ_ONLY=<substring>`**, which the diagnosis above needed and could not have without
  it: the population is *derived*, so it cannot be narrowed by editing a list, and a walk over a
  thousand documents that has to be run for one of them has no other way in. The run prints the
  filter, so a figure taken under it can never be read as the whole walk's.
- **The two bounds are the honest limit.** A widened document's pages past two and entries past
  four are listed, held to the layout's names, and read by nobody; the run says how many. What
  would remove the bound is not a bigger budget but a generator that can state a length without
  writing it, or a time budget a face can refuse against — neither of which exists (`doc/todo/58`
  §5).

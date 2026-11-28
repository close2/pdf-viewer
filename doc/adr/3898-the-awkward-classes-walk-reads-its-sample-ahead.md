# 3898 — The awkward-classes walk reads its sample ahead of classifying it

Session 2299. Status: **accepted** and built in `crates/viewer-confined/tests/awkward_classes.rs`.
Context: ADR 3826 (the walk's median hold of 510 s against the 600 s a small walk is allowed, and
one hold of 1 279 s), ADR 0879 (the walk), principle 2's rule that an optimisation carries its
benchmark and its comment.

## Where the time went

The walk now prints where its time goes. It times each root's classification, each document's
sweep and every class's sum (`view-awkward: time:` lines, which `state.sh`'s filter does not keep).
Run once as it stood, under `tools/bounded.sh --lock` with four rayon threads:

| | seconds |
|---|---|
| choosing the population (classifying 4 170 documents of 8 roots) | 442.3 |
| sweeping the 232 chosen documents through `pdf-view-worker` (wall) | 19.8 |
| the walk | 462.2 |

The same walk again at once, with the page cache warm, chose the same population in 10.4 s. So
the walk's time is the corpus's cold reads, not classification or the confined viewer. Its holds
since 2026-11-20 run from 320 s to 1 279 s, and the walk's own figure from 89 s to 895 s. That
spread is how much of the sample the page cache still held. A hold also includes the `gates`
profile's build of the test, 191 s when it is not fresh.

The classifier reads each file object by object, four kibibytes at a time at the offsets the
cross-reference table names (`FileBytes`'s first window). `safedocs` and `tika-issue-tracker` sit on
the rotating disk. Their sample is 2 400 files and 3 000 MiB. Read whole from a cache emptied of
those files (`POSIX_FADV_DONTNEED`, which needs no privilege), it took more than 260 s with one
reader, 171 s with four and 104 s with sixteen.

## Decision

Before a root is classified, `read_ahead` reads that root's sample whole, once, on sixteen threads
that wait on the disk and compute nothing. The sample is `corpus_classes::sampled` of
`documents_under`, the files `population` will open. Nothing it does reaches an answer:
`population` is called unchanged, and the cache changes only how long its reads wait. If the two
lists ever drift apart, the read-ahead warms the wrong files, and the walk is slow again but no
less right. A file it cannot read is counted, and the classifier meets the same file and says what
it found. Calling `population` one root at a time, so that each root is timed, gives the same
population, because it draws each root's share apart from the others'.

Measured with the sample evicted first, every root, inside the lock, one run each:

| | before | after |
|---|---|---|
| population chosen | 448.6 s | 118.1 s |
| walk | 467.3 s | 133.6 s |
| bounded hold | 508 s | 136 s |
| peak resident | 1.99 GiB | 2.18 GiB |

The per-root and per-class lines of the two runs are identical (18 lines, `diff` empty). The cost
is the bytes the classifier would not have read, image data above all: 3 000 MiB is read from the
rotating disk where the scattered reads touched less. It is paid in sixteen sequential streams
rather than four scattered ones, and they finish sooner.

## Consequences

The walk stays `small`, now with room under ADR 1869's 600 s even cold. If its time grows again,
the `view-awkward: time:` lines name the root or the document that grew. The other walks that
classify through `corpus_classes::population` (`pdf-vfs`'s `read_corpus` among them) pay the same
cold reads. The construction is theirs to take, measured on their own run.

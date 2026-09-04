# 919 — One walk over every corpus, and the other confined worker swept

Date: 2026-09-04.
ADRs: [0878](../adr/0878-one-walk-over-every-corpus-at-two-depths.md),
[0879](../adr/0879-the-other-confined-program-is-swept-over-the-same-population.md).
Touched: the merge of `round-917`; then `crates/corpus-classes/` (new crate),
`crates/pdf-vfs/tests/read_corpus.rs`, `crates/pdf-vfs/tests/awkward_classes.rs` (deleted),
`crates/viewer-confined/tests/awkward_classes.rs` (new), the two manifests and the workspace's,
`doc/todo/02-every-round.md`, `doc/todo/58`, `doc/todo/59`, `doc/todo/61`, `doc/crate-map.md`,
`doc/verify.md`, `doc/conformance/ledger.toml` (one note moved between two rows, in the merge),
two ADRs, this file. **No pixel moves**: no crate that draws changed, and the ten corpus walks say
so.

A merge round of the file-system faces' second stream, and then the one item that stream left
owed which was small enough to take beside it.

## 1. The merge

**`round-917` (5d376a24) is on `main` as `23dd2b2f`**, `--no-ff`. It is a chain of four rounds:
913's KIO face (a crate of thirty-five C functions and a C++ `WorkerBase` plugin outside the cargo
workspace), 914's read-side corpus walk and the `no_machine_fonts()` fix the walk's first sixty
documents forced, 916's *ask* level crossing the confinement as a question and an answer rather
than as a dialogue on the wire, and 917's ten-class sweep.

**No conflict**, and `doc/conformance/ledger.toml` auto-merged trivially because main had not
touched it since the branch left. Checked **row by row** against the common ancestor rather than
off the diff: main moved no row, the branch moved five — §7.5.6, §7.6.4.2, §7.6.4.3, §9.6.2.2 and
§12.8.2.2 — no third row moved, 875 rows on all four versions, and `tomllib` parses the result.

**The row-by-row check found the thing the clean auto-merge would have kept**, which is the fourth
merge in a row to find one that way. Session 916's paragraph about the *ask* level was written into
**§7.6.4.3**, whose title is *File encryption key algorithm* and whose whole note is one sentence
about which algorithm each revision uses. The paragraph is about Table 22's permission bits, which
are **§7.6.4.2**'s — the row whose `code` and `test` lists the same session extended, and whose
note already carries session 906's paragraph about the four levels reaching a file system. That
round's own history file says it touched §7.6.4.2. Moved, appended after 906's so the two read in
session order; §7.6.4.3 is its own sentence again.

**The whole `doc/todo/02` §2 sequence ran on the merged result, all thirty lines exit 0**:
`nextest` 3296 tests in 69.2 s; the corpus, oracle, text (99.67% of words in bounds), both censuses,
dates, xmp, jpeg2000 and fixed-documents gates green; quorra 958 pages in 31.6 s, 929 agree, 22
differ, 7 refused; the transform gate 162.1 pages/s against a floor of 40; all six transform walks
green including `foreign_corpus`; `vfs-write` 974 documents in 43.4 s and `vfs-read` 974 in 344.6 s;
`awkward_classes` 258 documents, 0 killed, in 37.4 s; `cargo test -p conformance` 218 tests.

**Five worktrees closed** — r911, r913, r914, r916, r917, each checked with `git merge-base
--is-ancestor` and each its checkout, its branch and its build directory taken together. Rounds 920
and 921 hold their own; 920 had merged `round-917` into itself before this merge, so its history
keeps those commits whatever happens to the branch ref.

## 2. Two instruments become one

`doc/todo/58` §4 had said how, and ADR 0877 had said why it should be done: `read_corpus.rs` asks
whether the two transports agree, byte for byte, over `doc/pdf.js`; `awkward_classes.rs` asked
whether the confined worker survives, over every corpus root on the disk. ADR 0878 is the merge.

```
vfs-read: 1132 documents in 315.5s, 24 threads, confined transport, 16 pages a doc/pdf.js document and 2 of every other root's
vfs-read:   pdf.js 974 classified/974 walked, format-corpus 167/25, pdf-differences 37/10,
vfs-read:   pdf20examples 7/6, pdfbox 64/6, openpreserve 267/32, safedocs 1200/37, tika 1200/42
vfs-read:   directories listed: 11664, entries stat'd: 20976, files read: 14274 (784.8 MiB)
vfs-read:   not the generator's bytes: 0   the two transports disagree: 0   panicked: 0
vfs-read:   killed: 0   did not recover: 0
vfs-read:   encrypted 45, locked 5, encryption unimplemented 2, pageless 9, damaged 60,
vfs-read:   unopenable 8, huge 30, jbig2 119, jpeg 2000 35, plain (control) 847
```

**It is faster than the walk it replaces** — 1132 documents in 315.5 s against 974 in 344.6 s on
the same tree an hour earlier — and that was not expected. `text()` interpreted *every* page of a
document longer than the walk's depth in order to throw the readback away, because `document.txt`
is the joined text and a document past the depth has no expectation for it. Not reading past what
will be compared pays for the whole widening.

**Trap 13.** `no_machine_fonts()` commented out of `pdf_vfs::serve::confine`, the worker rebuilt,
the same 1132 documents: **666 deaths, in six of the ten classes** — plain (control) 509, huge 84,
damaged 54, jbig2 23, jpeg 2000 23, encrypted 20 — exit 101, `did not recover: 0`. Session 917's
finding at four times the scale, and its shape unchanged: the control class dies most.

## 3. The document that priced the second bound

The widening was written with one bound and met
`corpus-cache/tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf`, which held the walk for
**twenty-five minutes** — twice, at sixteen pages and at two — with the rest of the population
finished. Every individual question of it is fast (`vfs_cost`: page count 131 ms, a page out
214 ms, its images 200 ms, a 300 dpi render 305 ms). What it states is **10 084 images on one
page**, each two pixels by one: `/images/0001/` is a directory of ten thousand files, and the walk
`stat`s and reads every entry it lists. A `stat` generates, a read caches a whole extraction run at
once, and a run too large for the cache's budget is cached **not at all** (round 911's finding, ADR
0865 §3) — so twenty thousand questions each re-ran an extraction of ten thousand images.

Bounded to four entries a directory it takes **7.6 s**, and the 10 077 entries it did not read are
counted. The finding is the mount's before it is the walk's and is in `doc/todo/58` §5 beside the
long-document one.

`PDFVFS_READ_ONLY=<substring>` exists because that diagnosis needed it: the population is derived,
so it cannot be narrowed by editing a list, and the run prints the filter so a figure taken under
it cannot be read as the whole walk's.

## 4. The other confined program

`doc/todo/61` §2 had recorded that the same sweep had never been run through `pdf-view-worker`,
and that session 914's "the viewer loses the page rather than the glyph" was read rather than
measured. ADR 0879 is the sweep:

```
view-awkward: 8 root(s), 3916 document(s) classified, 198 swept, 24 threads
view-awkward:   encrypted 28 documents / 28 frames, damaged 39 / 32, huge 30 / 31, jbig2 21 / 21,
view-awkward:   jpeg 2000 28 / 30, plain (control) 48 / 48, locked 10 refused, unopenable 8 refused
view-awkward: killed: 0, in 13.5s
```

**Trap 13**: `no_machine_fonts()` out of `viewer_confined::worker::confine`, the worker rebuilt,
the same 198 documents — **28 deaths in six of the ten classes**: huge 10, damaged 5, plain 5,
jbig2 3, jpeg 2000 3, encrypted 2. Exit 101.

Two things in its output are worth keeping. The 16 342 reports on 39 damaged documents are the
viewer's own sentences crossing the confinement — a rebuilt cross-reference table, an unimplemented
operator, a font program whose Adler-32 disagrees with its bytes — rather than a defect. And a
locked document is *answered*: `Event::PasswordRequired` crosses as the event it is, so ten
refusals and no deaths where a person would be asked for a password.

It is in `doc/verify.md` rather than in `doc/todo/02` §2, with the trap 10 build line above it. The
reason is in ADR 0879: the read walk gates the same class of defect every round over a wider set of
questions, and a second corpus-scale line would cost every future round for a narrower one.

## 5. The population is a crate

`crates/corpus-classes` — the roots on this disk, the fixed stride, the ten classes, and the
sentence that tells a death from a refusal. A dev-dependency of `pdf-vfs` and `viewer-confined` and
a dependency of neither. The argument is `test-scenes`': two sweeps that built their own
populations would answer differently for a reason that is not their workers'.

## 6. Gates

The full §2 sequence on the merge (§1). After part 2, the lines the change can reach: the four core
lines and the two `fuzz/` lines, the `pdf-vfs` walks, the viewer sweep, and `cargo test -p
conformance`. The figures are in the round's report; nothing that draws changed, and the two walks'
own comparisons are what would have said otherwise.

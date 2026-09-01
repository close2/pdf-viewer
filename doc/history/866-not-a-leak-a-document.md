# 866 — Not a leak: a document that costs ten gibibytes to interpret, met 192 at a time

Date: 2026-09-01.
ADR: [0798](../adr/0798-a-walks-cost-is-what-is-in-flight.md).
Touched: `tools/bounded.sh` (new), `doc/environment.md`, `doc/todo/03-more-corpora.md`,
`doc/todo/17-a-mebibyte-per-image-xobject.md` (new), `tools/safedocs/src/main.rs`,
`crates/pdf-model/examples/standing_count_census.rs`.

## The question

The owner asked whether yesterday's 90 GB working set and soft lockup was a leak in the viewer,
and for the memory to be limited if it was not. It was not. One document in the GHOSTSCRIPT
directory — `GHOSTSCRIPT-688117-0.zip-0.pdf`, ten thousand one-row image XObjects — costs
10.59 GiB to *interpret*, at any scale, and eight shards each running a rayon pool of one thread
per core had 192 documents in flight over a directory of fuzzed files. The whole-versus-halves
and one-thread-versus-twenty-four measurements are in the ADR; the shape of the answer is that a
slice's peak is its worst document's, not its length's.

## The bound

`tools/bounded.sh`: `RLIMIT_DATA` and a rayon thread count divided between `--shards N`, nice 19,
a once-a-second sample of the tree, a `--tree` ceiling for builds, and a last line that names the
bound when the bound is what ended the run. Its first version read every status 134 as the limit
and reported an 82-byte pageless file as an out-of-memory; the fix is in the script and the
lesson is trap 11's. The agreement — 32 GiB a walk, four shards, one walk at a time — is in
`doc/environment.md`.

## The aborts in the crash journal

No core was stored for any of them (`Storage: none`), so the journal holds the signal and not
the message. `SIGABRT` with `si_code SI_TKILL` from a `release` binary is `panic = "abort"`
ending a panic whose message went to the round's own standard error. `owed` exits 0 on this tree;
`render_at` on `PDFBOX-4623-1.pdf` exits 0 here. Both are diagnostic programs whose `expect`s are
meant to stop them loudly on a bad argument or a file they cannot open, and nothing in the journal
says otherwise — but nothing in it can confirm it either, and that is the honest limit.

## Gates

The core, `doc/todo/02` §2's first six lines plus `cargo test -p conformance`, all through the
wrapper; figures are in the run. No crate in the first row of the map was changed, so the full
sequence is not owed, and the interpreter defect is left for a round that can run it.

## For the next round

`doc/todo/17`. The cost is `RasterCache::parts` cloning the page's resource dictionary into every
cache entry and charging the budget the samples alone — massif puts 82 % of the peak on that clone.
Hold what the decode read rather than the dictionary, or charge the clone, and re-run
the second half of `batch2/GHOSTSCRIPT`'s first 680 under `tools/bounded.sh --data 2` per document
until nothing runs out — then the whole `doc/todo/02` §2 sequence.

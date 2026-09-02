# 873 — A pool with no next name says so on the line

Date: 2026-09-02.
ADR: [0805](../adr/0805-a-pool-with-no-next-name-says-so-on-the-line.md).
Touched: `crates/pdf-model/tests/oracle.rs`, `doc/conformance/ledger.toml` (§10.7.4's row),
`doc/oracle-and-corpus.md` §3b, `doc/todo/00-ambiguous-bucket.md` (a closing section).

**This round was two sittings.** The first did the work below and was killed before it could
commit: the whole Claude session went down under `systemd-oomd` when a neighbouring round's probe
reserved about 20 GB per process. What it left was the uncommitted worktree — the code, the four
document edits, the ADR and a draft of this file — and its logs under `tmp/`. The second sitting
read all of it, checked the reading against §10.7.4 in `doc/md/`, re-ran the whole of
`doc/todo/02` §2, corrected one sentence, and committed.

## The item

Take the worst-ranked contradicted page whose cause is not already diagnosed and held by name.
The oracle in a fresh worktree, every reference re-rendered by the installed `pdftoppm` 26.08.0,
`mutool` 1.28.0 and `gs` 10.07.1 (cache hit rate 0.1%, the first sitting's run, log kept):
**60 contradicted, 980 agreeing**, the ratchet green — and every one of the sixty held by a
`CONTRADICTED_*` group. There was no page to take.

`unpriced`, run over the same log, finds all 89 failing bounds on the 60 pages named by the note
that holds each page. **The first sitting wrote that `quoted` found "one figure" and `overtaken`
"no page", and that is not what they print**: re-run by the second sitting over its own log they
name ten and twelve `CONTRADICTED_*` notes, and each says on its last line that a hit is a reading
list rather than a verdict. Neither asks whether a page is held; the sentence in the ADR and in
`doc/todo/00` now says what they say.

The head of the by-the-bound ranking was opened rather than trusted, side-by-sides and logs: the
`bitmap-*` halftone composite (ours and `hayro` the family's drawing, both Artifex programs
`jbig2dec`'s garbled region), the CMYK shadings (ours and `poppler` on §10.4.2.5, the three on a
SWOP characterisation together), `xobject-image.pdf` (a self-contradicting file, our choice
reported). The notes hold.

## What was declined, with the clause read

The highest row whose note names a departure of *ours* is `issue4436r.pdf`, §10.7.4's image
paragraph: the clause paints only the pixels whose centres lie inside the image's region, the
references paint the whole row, ours paints its coverage. The sentence is verbatim in `doc/md/`,
and the second sitting re-read it there. Carrying the paragraph out is one line
(`anti_alias = false` on the image's fill) and it was not written: it is a change to departure (1),
which `doc/todo/11` §5 prices and keeps for every mark; it moves the page's verdict nowhere, the
differing fraction being a threshold count; and the three backends would owe it together. Said so
on §10.7.4's ledger row.

## What was built

The hour the round spent mapping sixty pages to twelve groups by hand is now a column: the
by-the-bound ranking prints `held by <group>` on each row, a line under it counts the pages no group
holds — the population ADR 0349 found outside every diagnosis, counted every run — and
`CONTRADICTED_GROUPS` is the one table the ratchet and the ranking both read, checked against the
file's own declarations by `every_contradicted_group_is_in_the_table`.

## Gates

No pixel moved; a change in `pdf-model` runs the whole of `doc/todo/02` §2. The first sitting ran
it once (its logs are the worktree's `tmp/gates-873/`); the second sitting ran it again from the
top, in this worktree with round 874 running on `main` beside it, and every line was green:
`fmt` and the `fuzz/` `fmt` clean; `clippy` under `RUSTFLAGS="-D warnings"` silent on both
workspaces (the only `warning:` lines are gcc's `-Wmaybe-uninitialized` on `cxx-qt`'s generated
bridge, which §2 names as not lints); `nextest` 2936 passed, 19 skipped; doctests green; the
sandbox and `pdfref-hayro` built. The corpus walks ran under `tools/bounded.sh --data 4`, one at a
time — corpus 1.54 GiB peak, oracle 1.91, quorra 0.99, fixed documents 0.97 — with the oracle's
counts the same as the first sitting's, 60 contradicted and 980 agreeing, at a 100% cache hit rate
this time; text extraction, both censuses, dates, xmp, jpeg2000, the transform gate and
`cargo test -p conformance` (218 rows) green.

## For the next round

The contradicted list is not where the next defect is. `doc/todo/00`'s closing section says where
to be sent instead.

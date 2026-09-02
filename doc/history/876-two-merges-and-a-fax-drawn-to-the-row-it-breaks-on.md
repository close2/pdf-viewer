# 876 — Two merges, and a fax drawn to the row it breaks on: rounds 873 and 875 on `main`, `batch5/REDHAT` walked, and a damaged `CCITTFaxDecode` stream delivers the scan lines before the damage, leaves the rest unpainted and says so

Date: 2026-09-03.
ADR: [0794](../adr/0794-a-damaged-fax-is-drawn-to-the-row-the-error-occurs-on.md).
Touched: `crates/pdf-sandbox/src/decode.rs`, `crates/pdf-sandbox/src/protocol.rs`,
`crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/thumbnail.rs`, `crates/pdf-model/tests/ccitt_bound.rs`,
`crates/pdf-model/tests/image_reuse.rs`, `crates/pdf-transform/src/images.rs`,
`doc/checks/fixed-documents.toml`, `doc/conformance/ledger.toml`, `doc/todo/03-more-corpora.md`;
and the two merge commits before this round's own.

## The merges

`round-873` (5c218c91, ADR 0805: the oracle's `CONTRADICTED_GROUPS` table, §10.7.4's image edge
read and declined) and then `round-867`'s round 875 (935105d2, ADR 0804: `--format pgm`, the
`/Names`-indirect holder fixture, the writer over the corpus as a §2 line and `tools/state.sh
writer`) went into `main` with `--no-ff`. Neither merge had a conflict: 873's `oracle.rs` and one
ledger row, 875's `pdf-transform`, `tools/state.sh` and the todo index, and 874's `pdf-model`
touched no common line. The whole `doc/todo/02` §2 sequence — twenty-one lines now, the writer
among them — ran on the merged `main` under `tools/bounded.sh` (`--data 8`, `--tree 8` for a
build and `12` for a walk, one walk at a time, the script checking for another round's walk before
each) and every line was green; the figures are in the round's logs. Then `tools/worktree.sh close
866 873` took both checkouts and their build directories away, 866 having been merged by 869;
`r867` stays open as the transform stream's branch and `r877` is a neighbour's.

## The chunk, and the finding

`doc/todo/00`'s closing section sent a round looking for a defect to `doc/todo/03`, whose owed
list began with `batch5`'s two dozen unwalked trackers. `batch5/REDHAT`, 1712 documents, was
surveyed whole under the four rules of 2026-09-02 (twelve rayon threads, 22 s, 2.4 GiB peak);
`doc/todo/03` §40 has its line and its reports. The ranking — ours flattened on white against
`pdftoppm -cropbox` and `mutool draw` over the 104 incomplete pages — had one head by a factor of
three: `REDHAT-229174-0.pdf`, a Photoshop 4.0 Group 4 scan of a textbook page, ours 0 against
`poppler` 8.9 and `mupdf` 74.3, reporting `CCITTFaxDecode: arithmetic overflow in position
calculation`.

Two instrument mistakes were made and caught before a number was believed, and both are habits
this tree already wrote down: the first ranking ran `pdftoppm` without `-cropbox`, and the first
ink figures read our alpha channel as ink and put every page at half of `poppler`'s. And one
process mistake of the round's own: a `kill $(pgrep -f 876-rank.sh)` matched the shell running
it, which is `doc/environment.md`'s paragraph about `pgrep -f` exactly, and cost a restart.

The file's `stream\r` — §7.3.8.1's forbidden CARRIAGE RETURN alone — was checked first, because
an off-by-one at the stream's start produces the same sentence; `pdf-syntax` tolerates it. Then a
scratch probe against the pinned `hayro-ccitt` decoded 756 whole scan lines of 2244 and stopped
inside the 757th, which is the row `poppler`'s text stops at; its black band below and `mupdf`'s
two further lines of text are what each does after the damage. §7.4.6 — "The filter shall not
perform any error correction or resynchronization" beyond `/DamagedRowsBeforeError`, whose Table
11 row defaults to zero — makes the decoder right to stop and `mupdf`'s continuation the thing the
sentence forbids. The only question was this tree's: `pdf_sandbox::decode::ccitt` refused the
whole picture on any decoder error, on a comment arguing that drawing the rows before it "would be
a page that is silently missing its bottom half". The word was *silently*.

ADR 0794: the rows the filter delivered are drawn, the rest are left unpainted, and the shortfall
is reported beside the drawing. `Bilevel` carries `delivered` and `stopped_by` out of the worker;
`image::Parts` carries the sentence beside the picture through the raster cache, so a second `Do`
says it too; the page's image, a `/Mask`, an `/SMask` and the transform's `images` verb all say
it in the same words; not one whole row delivered stays a refusal; a thumbnail's shortfall stays a
refusal because its type crosses two wire protocols, recorded as a residue. **The first build of
the change drew the lower two thirds of the page solid black** — the worker's padding is the
filter's white, which under `/BlackIs1 true` with no `/Decode` is the page's black — and only
looking at the page (trap 1) caught it; the rows are cleared after unpacking now, which is ADR
0356's choice for §7.3.8.2's short image and the same clause's error. `tests/ccitt_bound.rs` pins
the pair over a grey page so that unpainted and white read differently, and the refusal where the
damage falls inside the first line; the document is a row in `doc/checks/fixed-documents.toml`,
its band the gate's own reading and not the round's `magick` figure, which was a different
instrument and half the number.

`batch5/poppler` was walked twice and surveyed neither time: one document asks for 6 001 925 632
bytes in one allocation and takes the survey down under `--data 8` at twelve threads and at four.
Which document was not chased; §40 says what the next round of this chunk starts with.

## Gates

The full §2 sequence ran twice on `main`: after the merges, and again after this round's change,
which touched two first-row crates. Both runs were under `tools/bounded.sh` at `--data 8`, `--tree
8` for a build and `12` for a walk, the second run's later walks waiting on round 877's per-document
walk of the crawl before starting. Second run, from the logs: formatting and `clippy` under `-D
warnings` silent for the workspace and for `fuzz/`; **2979 tests passed, 20 skipped**; doctests
green; the corpus gate at **974 documents, 64 incomplete**; the oracle at **1945 pages, 1841
complete, 104 incomplete**; the three text gates green (99.67% of matched words in bounds, 493 of
503 documents fully in); the two censuses green; dates 1514 of 1545; XMP and JPEG 2000 green;
quorra at **958 pages, 932 agree, 22 differ, 4 refused**; fixed documents **57 of 57** — after one
failure of this round's own, the new row's band written from the wrong instrument and corrected to
the gate's 4.918; the transform gate at 196.4 pages/s over a floor of 40; the writer over 974
documents with nothing failed; conformance green; `--bin quotations` and `--bin pointers` with
nothing of this round's among their hits. Not a fifth round, so §5's binaries were not rebuilt.

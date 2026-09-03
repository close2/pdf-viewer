# 891 — A merge, and a note that does not say it: round 887 on `main`, §8.7.3.1's NOTE 2 is about `/XStep`, and the lattice paint is priced and declined

Date: 2026-09-03.
ADRs: [0827](../adr/0827-the-note-that-says-a-cell-is-evaluated-once-is-not-the-one-three-places-cited.md),
[0828](../adr/0828-the-display-list-gains-no-lattice-paint-and-the-two-witnesses-say-why.md).
Touched: `crates/pdf-model/src/content/pattern.rs` (a doc comment),
`doc/checks/fixed-documents.toml` (two rows), `doc/conformance/ledger.toml` (§8.7.3.1, §11.6.7),
`doc/todo/49-restrictions-worth-re-examining.md`, `doc/todo/11-shapes-that-still-disappear.md`,
`doc/adr/0810` (a correction line); and the merge commit before this round's own.

## The merge

**`round-887` (44958f77) is on `main` as `76e782b1`**, `--no-ff`, on top of round 889 and the
merges of 883 and 885. It carries two things the owner asked for: the licence is Apache-2.0 in
every place this tree names its own — `LICENSE`, the workspace `license` field, `NOTICE` as
section 4(d)'s attribution file, `deny.toml`, the CI packaging comment, `doc/third-party-data.md`
and `doc/HAYRO_MERGE.md` (ADR 0819) — and revision 5 of the standard security handler is read
rather than refused (ADR 0820).

**One conflict, in §7.6.4.2's ledger row, which both sides rewrote.** 885 had added
`viewer-core`'s two headless tests and the paragraph deciding that a file filed by §12.5.6.15's
annotation is Table 22 bit 6's rather than bit 4's; 887 had replaced the row's "/R 5 is refused
here rather than guessed at" sentence with the reading that Table 21's "Shall not be used"
binds a *writer* and points at the Adobe Supplement's Algorithm 3.2a. Resolved with both intents:
885's `code` and `test` lists, which are the superset, and 887's revision-5 paragraph in the
refusal sentence's place, ahead of 885's own closing paragraph. §7.6.4's parent row merged
without conflict and already names the change.

**The two conflicts this round was told to expect in the corpus gate did not appear**, and the
reason is worth the line: `main` has not touched `crates/pdf-model/tests/corpus.rs` or
`tests/oracle.rs` since 839a659a, the branch point, so 887's `MAX_LOCKED` 8 → 9,
`MAX_UNREADABLE_ENCRYPTION` 2 → 1 and the two oracle arrays applied unchanged. The corpus gate on
the merged tree confirms them rather than the merge asserting them: **974 documents, 0
unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete**, with `issue21579.pdf`
among the nine locked and `PDFBOX-4352-0.pdf` the one encryption not implemented.

`tools/worktree.sh close 887` took the checkout and its build directory once the branch was an
ancestor of `main`. `r867` and `r890` are neighbours' and untouched.

## The finding: three places cited a note that does not say it

`doc/todo/49`'s one open tiling item rested on a citation — "a cell rendered *once* and
replicated by the rasteriser — §8.7.3.1's NOTE 2's own suggestion". **§8.7.3.1 has no note about
replication.** The two printed under that heading are Table 74's: NOTE 1, that a `/BBox` of zero
height or width still paints one pixel, and NOTE 2, that `/XStep` and `/YStep` can differ from
the dimensions the `/BBox` implies. The sentence about a cell "evaluated once and then
replicated" is **§11.6.7's NOTE 1**, and read whole it says that of the *opaque* imaging model,
giving the optimisation back in the transparent one only "in the common case in which the pattern
consists entirely of objects painted with the Normal blend mode". §11.6.7's NOTE 2 — the one ADR
0810 quoted accurately — is about treating all tiles as one transparency group against seam
artefacts, which `Interpreter::tile` has done since the hundred-and-seventeenth session.

`doc/todo/11` had made the same mistake and corrected *itself* four sessions later, in a
parenthesis; the correction reached that file and none of the three that had copied it — ADR
0810 twice, `pattern.rs`'s `MAX_TILE_COPIES` doc comment, and the §8.7.3.1 ledger row. ADR 0827
has the reading and the grep that finds the shape.

What binds instead was already being met: "the PDF processor shall paint the cell on the current
page as many times as necessary to fill an area", with the order "unspecified and unpredictable",
and the lattice "displaced by multiples of XStep and YStep". None of that says where the
replication happens.

## What the two witnesses actually cost

Both arms built in one sitting, the second with `MAX_TILE_COPIES` and `MAX_OPERATIONS` raised to
10⁹ at the constants themselves (trap 29), alternated on an idle machine through
`examples/open_one` at scale 1, four repetitions each:

| | commands | interpret | total | peak | ink |
|---|---|---|---|---|---|
| `2760154.pdf` cut | 67 676 | 43 ms | **0.33 s** | 0.02 GiB | 33.583 |
| `2760154.pdf` whole | 765 191 | 378 ms | **2.08 s** | 0.42 GiB | 34.670 |
| `PDFIUM-1497-2.pdf` cut | 276 157 | 103 ms | **1.87 s** | 0.19 GiB | 11.9049 |
| `PDFIUM-1497-2.pdf` whole | 276 157 | 601 ms | **10.53 s** | 0.93 GiB | 11.9049 |

**`PDFIUM-1497-2.pdf` is byte-identical either way** — the same MD5, and `compare_rasters` at mean
0.0000, differing 0.0000%, SSIM 1.00000 — although the budget draws 16 384 of the 448 632 sites
its largest tiling states. Eight and a half seconds and three quarters of a gibibyte buy nothing
on it. `2760154.pdf` is cut of a mean of 1.087 of 255, all of it the pale wash behind a poster's
title, at a maximum of 11 levels over 7.73% of the pixels.

**And nine tenths of both gaps is rasterisation**: interpretation, which is the only part a
lattice paint removes, is 19% and 6% of them. So ADR 0828 declines the paint. The version that
fits this architecture leaves `PDFIUM-1497-2.pdf` at about ten seconds and both pages looking as
they do; the version that would buy the rest is a device-resolution tile blitted per site, which
bakes a flattening resolution into a display list that has none and composites tile edges with
one another where §11.6.2 forbids it — the 13% loss `pdf_render::repeat` exists to remove — and
which two of the three backends are other repositories with no notion of a pattern.

The item is closed by argument rather than left open by neglect, and the close is checkable: both
witnesses now have rows in `doc/checks/fixed-documents.toml`. `2760154.pdf`'s ±1.0 band
discriminates by 0.087 of a level, so a paint that drew its wash whole would fail the row;
`PDFIUM-1497-2.pdf`'s cannot, says so in its `why`, and pins instead that the page draws and that
the refusal is still `MAX_TILE_COPIES` rather than `MAX_OPERATIONS`.

## Gates

The whole `doc/todo/02` §2 sequence ran twice on `main` under `tools/bounded.sh` (`--tree 8` for a
build, `12` for a walk, one walk at a time): once on the merged tree before this round's own
change, and once after it, with `--bin quotations` and `--bin pointers` added because documents
moved. **Every line exit 0 on both runs.** From the second: formatting and `clippy` under
`RUSTFLAGS="-D warnings"` silent for the workspace and for `fuzz/`; **3072 tests passed, 22
skipped**; doctests green; the corpus gate at 974 documents and 64 incomplete; the oracle at 61
contradicted pages, every one held by a group by name; the three text gates green (99.67% of
matched words in bounds, 493 of 503 documents fully in); the two censuses green (877 of 877
untagged pages answering the empty tree); dates 1514 of 1545 (97.99%); XMP 318 of 319 streams
read; JPEG 2000 green; quorra at **958 pages, 929 agree, 22 differ, 7 refused, 16 not
comparable**; fixed documents **63 of 63, 0 absent**, the two new rows among them; the transform
gate at 192.7 pages/s over a floor of 40; the writer over 974 documents with 941 attached into the
tree and 940 onto page 1, read back and removed; conformance, quotations and pointers green.

**One re-run of the corpus gate failed and the failure was the machine.** Asked for a headline
figure the sequence's `tail` had cut off, a second `--test corpus` run exited 101 on
`a document must not take longer than 30s to open and draw:
[("ContentStreamCycleType3insideType3.pdf", 32.234686503s)]` — the known `MAX_FORM_DEPTH` cycle,
3.8 s in ADR 0810. `ps` found a round 892 building and running `cargo nextest run --workspace`
against `/home/AI/cargo-target/pdfv-r892` beside it, at a load of 26. The same gate on the same
tree exits 0 quiet, twice in this session. §2's "run the sequence on a quiet machine" has a wall
clock inside `tests/corpus.rs` as well as around the reference renderers, and this is that
threshold's first recorded false positive: it is 8× the margin on this document and it went
under load.

§5's binaries were rebuilt and installed, which this round owed twice over — a fifth round, and a
measurement was taken.

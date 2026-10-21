# 1672 — Font scales and declared width ranges

Session 1672, slot 2 of batch 104, a pixels round; 2026-10-21. ADRs 2180 and 2181.

**CFF.** Read the original Top DICT matrix before the dependency rounds its reciprocal
scale to integer units per em. The outline reader's normalized horizontal factor remains;
program advances and outlines use the same fractional scale. A 649-unit advance under
0.00049 becomes 0.31801, and nonuniform scaling retains its horizontal factor. The real
wp315.pdf font111/F4 now exposes its PDF/program width disagreement rather than hiding it.

**Widths.** Table 109's LastChar bounds the declared array. Trailing entries cannot replace
MissingWidth beyond the range; a short array invents no widths. An absent LastChar keeps
the previous malformed-file recovery. The archive remedy is separately owned by 1671.

**Gates.** Scoped formatting and warnings-denied all-targets clippy exit 0. Final nextest
passes 291 tests in 1.296s. HEAD plus only the five new tests gives four failures and one
pass (absent-matrix default), exit 100; the fixes pass all five. Frozen font-only six arms
exit 0: 968 pages each at 1x, 965 each at 4x; 3276s hold, peak 3.85GiB. Baseline and
candidate worker hashes agree. Full current-tree conformance exits 0 via integration's
archive check (136s). Integrated CPU golden update passes both tests (75s hold, 1.62GiB);
Final text extraction passes four tests (43s hold); fixed documents passes one (17s hold).

**Pixels.** At 1x CPU/GPU digests do not move; issue4801's mean error improves .0794 to
.0793. Compute's issue4801 digest moves with mean error .0084 unchanged. At 4x CPU/GPU
move issue4550, issue4801 and issue5751; compute moves only issue4801. Every 4x mean error
is unchanged. Shipped-quantum zoom_frame raw RGBA pairs for every moved page and lane
show at most six changed pixels per image, each by one component level. Full image pairs
were inspected: accented letters, umlauted letters and Open Access retain shapes/positions.
The isolated exports and complete differences are recorded under scratchpad/r1672/.

**Unfinished.** The separate Type 1 reader also exposes integer units per em; a fractional
Type 1 witness and comparison are needed before changing it. Integration regenerated once:
974 entries, three moved, no additions/removals. All report hashes stay identical; issue4550
and issue5751 move only their lists. issue4801's old/new raster hashes exactly match the
isolated font-only CPU images: pixel (73,16), RGB25→26. List attribution is inferred from scope.

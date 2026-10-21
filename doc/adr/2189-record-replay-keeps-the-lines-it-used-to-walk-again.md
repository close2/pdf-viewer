# 2189 — Record replay keeps the lines it used to walk again

Session 1676, slot 6 of batch 1671–1678. Context: ADRs 0087, 2056, 2134 and 2188;
`doc/todo/75` item 3. Deferred: the measured prototype was removed; selection remains. No clause or ledger status changes:
this changes how an unchanged scene is rendered at another view.

## The measured population

The `render-raster` example `replay_line_census` draws each document's first page at scales 1 and
1.25, on fresh backends using Compute coverage, then reads the actual second frame's encode source
and line-lane count. A generated line reports `(true, 1)` and an empty page zero lines before the
corpus is asked. PDF operators or display-list shapes are not used as proxies for replay admission.

The 968 paths are exactly the successful first-page names from batch 1671's compute-1x arm.
**957 drew, 850 record-replayed, and 61 of those had line instances.** Those 61 pages contain 17,834
observed line instances; `bug1978317.pdf` accounts for 16,384, `issue1249.pdf` for 384,
`issue11473.pdf` for 242 and `issue16038.pdf` for 200. The census held the bounded long lane for
487 seconds, exit 0, peak 1.15 GiB. This is a population measurement, not a timing comparison.

**Eleven inputs remain uncensused**, not negative: ten need passwords (`bug1782186.pdf`,
`encrypted-attachment.pdf`, `issue15893_reduced.pdf`, `issue21579.pdf`, `issue3371.pdf`,
`issue6010_1.pdf`, `issue6010_2.pdf`, `pr6531_1.pdf`, `print_protection.pdf`, `saslprep-r6.pdf`);
`issue19517.pdf` exceeds the adapter's 16,384-pixel side limit at the moved view. Thus 61 establishes
a real population, without asserting that the refused pages contain none.

The executable's SHA-256 was `1e4c85a18fbfe95e44f9338d1587f7e01172d382555c4b7a5099140a9c787171`;
the path manifest's was `5fa90d0686c04c7785dd6f1ccc03331561a9f51bd8da9ab5175c2652e36e707e`.
It was built from the shared batch working tree based on `5ded4916304afc8b9dd931fa8b21a86db6644ef8`,
with the selection candidate but before this replay change. The instrument is retained in
`crates/render-raster/examples/replay_line_census.rs`; its command accepts explicit PDF paths or
`@paths.txt`. Runtime refusals are printed separately, never counted as successful negatives.

## Prototype design (removed)

The Records route carries the previous `Encoded` alongside its `ReplayList`. Rect and SolidFill
records retain their existing fast replay. A sequence of Slow records for a complete closed-form
line run uses the same kept-run placement and conservative selection as the scene walk. The run
still resolves its clip and mask at its original position; line instances and drawing order stay
unchanged. The existing retained handle adopts the same resident line buffer, so no line byte is
uploaded again.

A closed-form line writes exactly one Slow record before any cull. Consecutive line commands
therefore occupy consecutive records in the same private encode. The retained walk checks the
span's far boundary and its clip correspondence. On a failed match it discards the partial encoder
and restarts ordinary record replay. Non-axis views select ordinary replay directly. Neither case
continues into a half-built plan or appends a second copy of earlier marks.

The old veto on idle whole builds for replayable pages is removed: a page opened zoomed can now
obtain a complete line stream while idle, and its later record replay can use that stream. The
existing whole-build admission, residency refusal and once-declined behavior remain authoritative.
Optional bounds refusal still draws the original complete runs and reports that choice.

## Prototype validation (archived)

`/home/AI/r1676-deferred-replay/candidate-files/raster/crates/raster-gpu/tests/kept_record_replay.rs` compares mixed Rect/SolidFill/Slow-line sequences against
fresh devices through zoom, non-axis rotation, return to fit and reflection. The archived private test
`a_failed_run_match_restarts_without_partial_marks` breaks a run's clip correspondence after an
already replayed rectangle (both clip and record-span mismatches), and compares the restarted plan
and streams with ordinary replay.
The archived candidate version of `crates/render-raster/tests/idle_build.rs` exercises a replayable page opened zoomed, checking idle and
fresh-frame bytes, kept counts and zero line upload. The optional-bounds refusal test runs both CPU
and Compute coverage. `/home/AI/r1676-deferred-replay/candidate-files/raster/crates/raster-gpu/tests/kept_record_whole.rs` uses Compute coverage
and 257/259-line runs separated by translucent rectangles and a triangle: runs share and cross
global selection blocks, while the zoom-opened idle build and five subsequent views must preserve
fresh-frame pixels, painter order, kept counts and zero line upload. All named prototype witnesses passed before deferral: formatting, warnings-denied Clippy and
876 GPU/render-raster tests passed, five skipped, exit 0. The replay-only performance verdict and
combined pixel arms still belong in the round history; passing correctness does not establish a
worthwhile speedup.

## Cache-free replay comparison

The initially prepared same-cache prototype exports were never measured. Fresh cache-free exports
compare selection only with selection plus replay, preserving HEAD font/model/alpha code on both
sides. ADR 2188 records source provenance, the two rejected activity samples, the symmetric fixed
pre-start rest, and the final exclusive nice-0 hold: exit 0 after 235 seconds, all 34 GPU activity
means below 20%, no sampled CPU load above 12. No owner GPU loop was used.

The positive probe and all three paired processes actually record-replayed `bug1978317.pdf` page 1,
base scale 1, views 1/1.125/1.25, Compute coverage, idle build disabled, 24 Rayon/encode threads.
Each instrument reports the minimum of three rounds. Each after process kept 16,384 lines at the
two changed views and sent zero line bytes; before sent 983,040 line bytes per changed view.
All nine digest/size pairs matched in every process pair. No selection blocks were omitted on this
witness, so this comparison measures replay reuse rather than viewport-block omission.

| Pair/view | Whole frame before → after (ms) | Encode before → after (ms) |
| --- | --- | --- |
| 1 / 1.125× | 31.1 → 31.5 | 2.5 → 3.0 |
| 1 / 1.25× | 52.8 → 31.8 | 5.0 → 2.7 |
| 2 / 1.125× | 31.0 → 31.6 | 3.6 → 2.9 |
| 2 / 1.25× | 30.8 → 30.5 | 2.7 → 2.4 |
| 3 / 1.125× | 30.8 → 52.2 | 2.8 → 4.1 |
| 3 / 1.25× | 31.5 → 51.2 | 2.7 → 4.8 |

First-frame totals before → after were 66.6 → 66.7, 66.7 → 65.7, and 65.0 → 66.7 ms. The
engineering controls and idle construction costs are fully listed in ADR 2188. Earlier valid
prefixes were mixed as well: the first attempt's replay totals improved by 1.4 and 5.9 ms, while
the next attempt included an unfavorable 31.6 → 52.1 ms at 1.25×. No outlier is silently removed.

**The transport saving is demonstrated; a consistent whole-frame or encode speedup is not.**
The extra replay reuse is **deferred and removed**: inconsistent latency results do not justify
its additional state while stability is the priority. Selection is retained independently. The
nine-path rollback restores the previous Records route and the replay-page idle-build veto, and
removes only replay-specific tests. The complete prototype, those tests, exact reapplication patch,
source/binary manifests, exports and all measurement attempts are preserved outside the closing
tree at `/home/AI/r1676-deferred-replay/`; `replay-only.patch` applies to the selection-only source
and passed `git apply --check`. A future attempt must explain the observed whole-frame variance
and demonstrate benefit before reintroducing the extra reuse. The future GPU prediction question
in todo 75 section 8 remains separate.

Before removal, the cache-free scoped suite passed 868 tests (five skipped), the whole workspace
7,519 (53 skipped), and all six pixel arms passed without an error increase. These prove correctness under the tested
sequences, not a performance benefit. The final timing logs and summary are preserved separately
under `/home/AI/r1676-deferred-replay/cachefree-rested`; rejected attempts are not merged into that set.
Final-tree tests and arms are repeated after removing the prototype; launch/turn/toolkit gates
remain conditions of acceptance for the retained selection change.

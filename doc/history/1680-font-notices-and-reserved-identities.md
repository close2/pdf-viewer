# 1680 — Font notices and reserved identities

Archive round, batch one hundred and five, 2026-10-21. ADRs 2194, 2195; owner A550.
**Implemented; batch validation pending.** ISO 32000-2 §14.3.2 Tables 347–348 and
PDF/A-2 §6.6.2.1/PDF/A-4 §6.7.2.1: newly embedded, substituted, or rewritten bundled
Foxit programs carry LICENSE_FOXIT in their own Metadata/XML stream's UsageTerms.
One x-default language alternative preserves every character, including the final newline.
Existing unrelated metadata survives; exact canonical terms can be reused alongside it.
Parsed shape plus namespace-aware scratch removal proves the selected property's text.
Different terms, duplicates, comment decoys, wrong namespace bindings and trimmed text refuse.
Untouched pre-existing programs are not retroactively annotated; conforming input stays unchanged.

Modified Liberation programs replace active reserved name-table identities with QuorraText.
Appended string storage preserves legal records even when they alias an identity's bytes.
The sfnt writer repairs table checksums and checkSumAdjustment; glyph tables stay unchanged.
Both metric-rewrite routes and the CID cmap-removal route are covered. Unmodified copies
remain byte-identical, and malformed identity offsets refuse. PDF dictionary widths stay fixed.
No new owner question; the descriptorless route still refuses metric changes.

**Evidence.** Fixed baseline 020a86f3 with only test/helper paths compiles, then fails all five
new end-to-end assertions: absent notices, missing conflict refusal, and reserved names.
Expected exit 101, 100 s held, 0.99 GiB peak; emitter hashes equal the fixed baseline.

**Gates.** Candidate nextest: 925 passed, 13 skipped, 4.203 s tests. All six new archive witnesses and
name-storage/outline/checksum witnesses pass. A test initially assumed output object7;
following the actual resource/descendant/descriptor fixes that test, not production behavior.
Scoped rustfmt and diff checks pass; warnings-denied clippy passes (24 s, 1.24 GiB).
The strengthened namespace witness passes all four notice tests (6 s, 0.73 GiB).
Archive catalogue/census and validator pass; converter corpus: six tests pass, 186 s,
2.31 GiB. Resource cross-check passes, 46 s, 1.02 GiB. Remaining phases ran in separate
bounded holds after stopping the outer section between populations to free short-job capacity.
veraPDF 1.30.3 accepts 27 outputs: all ten Foxit faces, unchanged Liberation, and two
metric-restatement cases at 2b/4, plus real issue6410 at 2b (18 s, 0.22 GiB retry).
An initial unembedded synthetic metric case hit the existing Q566 fitting refusal;
embedded metric witnesses exercise the intended restatement route without relaxing it.
Five baseline/current first-page rasters at 144 dpi are byte-identical; metric witnesses
and issue6410 were inspected. Final workspace conformance and warnings-denied docs passed.

Owned population: scratchpad/r1680/owned-paths.txt; detailed logs/report: /home/AI/report-1680.md.
Duration: 9762 s including bounded queues, from 2026-10-21T22:50:19+02:00.

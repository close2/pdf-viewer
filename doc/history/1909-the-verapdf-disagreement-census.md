# 1909 — the veraPDF disagreement census

Batch 131, slot 9, a "clause" round (reading). ADR 2652; ADR 2653 and Q785 unused. I worked in
`scratchpad/r1909/`.

**What.** The census of real producers' files keeps every output it writes when
`QUORRA_CENSUS_OUTPUTS` names an absent or empty directory (`Kept` in
`crates/pdf-transform/tests/archive_corpus.rs`), with a `.unchecked.tsv` beside each naming what this
tree's validator did not check. `tools/verapdf-census.py` puts every output through veraPDF at its own
flavour, bounded by chunk, process count, heap and a per-file time limit, and prints the rules veraPDF
fails on an output this tree wrote as conforming, by rule and by document. `tools/state.sh archive`
keeps the outputs of its converter walk in a scratch directory and prints the tool's two count lines.
`doc/pdfa-blocking-issues.md` section 1 names both. No fix.

**Found.** Against HEAD's census (counts identical line for line): 4997 outputs, 4850 compliant to
veraPDF, 113 failing on 32 rules (17 clauses) over 35 documents, 34 outputs of 9 documents veraPDF
did not finish. First reading: seven clauses for veraPDF, two for this tree, eight undecided; the
table is ADR 2652 section 2. The hypothesis held by clause, not by rule (18 of 32 rules on fonts).

**Plant.** `helloworld.pdf`'s 2b output with `pdfaid:conformance` set to `Z` was listed alone at
ISO 19005-2 6.6.4-3. A per-file time limit of one millisecond sent all 29 sample outputs to "did not
finish" and none to compliant; `--jobs 17` is refused.

**Measures.** The veraPDF run took 460 s under the lock at a load of 15 to 33, 4.11 GiB peak over the
tree; the census keeping its outputs peaked at 2.52 GiB beside HEAD's 2.40 GiB.

**Gates.** `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform --test archive_corpus` exit 0;
`rustfmt --check --edition 2024` on the test exit 0; the census with outputs kept exit 0;
`tools/verapdf-census.py` exit 0; `cargo test -p conformance --test commands --test state_sections`
exit 0 (2 and 6 passed); `--test batch` exit 0 (24 passed) after the orchestrator named the Python
line's missing `PYTHONDONTWRITEBYTECODE=1`; `--test read_only` exit 0 (2 passed) once the section's
scratch directory became a `mktemp` one; `--bin cited` exit 0; `--bin quotations` exit 0 (3 diverging,
none in a file of mine); `cargo test -p conformance --no-fail-fast` exit 0 on the final tree. The confirmation run, `tools/state.sh --round
1909 archive` in an export of the tree before the two section fixes, exit 0 after 6918 s, most of it
queued for the lock's second lane: `veraPDF census: 4997 outputs this tree wrote as conforming, 4850
compliant to veraPDF, 113 not, 34 veraPDF did not finish` and `disagreements: 32 rules over 113
outputs of 35 documents; 17 rules on a clause this tree checks whole, 15 only on a clause it checks in
part`, the tool's own run less the plant (4998, 114, 33 rules over 36 documents, 16 checked in part)
figure for figure; the section unchanged, so its tests not re-run.

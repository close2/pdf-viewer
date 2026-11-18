# 2082 — The documentation audit groups G123 to G143 are answered

Session 2082, an audit-fix round in `pdf-viewer-audit`. Groups G123 to G143 of
`doc/reviews/2038-specification-audit/fixes.md`: 30 findings, all D.

- **Fixed 23, argued 4 (D01, D46, D57, D68), owner 0, open 3 (D02, D55, D59: viewer-crate builds, each
  with its ADR).** D03's doc comment in `page_label.rs` is G063's file and waits for round 2079's merge.
- Errata read against the sponsored PDF's own annotations (`mutool show`, `pdftotext -bbox`), never the
  audit's word: #193 is §7.2.2's and makes binary transport unconditional; #309 is about `None`, not
  `/Colorants`; #444 makes Ink's Path-over-InkList a rule; #453 is §9.8.1's; #218 is Table 263's; #10 is
  should→shall in Table 5's `/F` cell; #179's overlay contradicts its own issue (ADR 3018 §2).
- pdf-issues checked by `gh api`: #432 closed as #593's duplicate and #593 completed on 2025-11-10, so
  Table 161's repeated letters are the amended reading and ADR 0601's conflict is gone (ADR 3018 §1).
- ADR 3018: four premises the errata and a table number had moved (0601, Table 285, 0803, 0473).
  ADR 3019: five rationales qualified (2811's 32 bits, raster 0030's `min`, 2356/0171 under 3008, 0914's
  singular matrix, 1080 under 2987). ADR 3020: a collection's initial document is presented, not marked.
  ADR 3021: the file-select read at commit, and the confined window's restriction control. ADR 3022: the
  archive's dynamic-XFA stop keeps the flag.
- Ledger notes corrected in place: 7.2.2, 7.3.8.2, 7.5.4, 7.7.2, 8.9.7, 9.3.2 (and test links on 9.3.2,
  9.4.1, 9.4.2), 9.6.4, 9.8.1, 10.7, 12.4.2, 12.5.1, 12.5.6.13, 13.2.4.2, O.2.2. No status moved.
- Comments: `write.rs`, `function.rs`, `appearance.rs`, `type3.rs` (and its error message),
  `archive/sites.rs`, and two test comments that quoted §12.5.6.18's struck sentence.
- No owner audit file was edited. `expected-failures-current.json` is unchanged; no finding here had a test.

**Gates.** Over `-p pdf-syntax -p pdf-colour -p pdf-model -p pdf-transform`: fmt `--check` exit 0; clippy
`-D warnings --all-targets` exit 0; nextest `--lib --bins --tests` exit 100, 6 failed of 4722, exactly the
manifest's 6 for these crates, none new and none fixed; rustdoc `-D warnings --document-private-items` exit 0;
`cargo test -p conformance` exit 0 once this record stated its gates. No public type changed, so no fuzz check.

Trap: `pkill -f "<pattern>"` run from a shell whose own command line holds the pattern kills that
shell (exit 144). Name the process by a bracketed pattern, as `pgrep` waiters already do.
Habit: an erratum's overlay is a snapshot. Before recording two errata as in conflict, read the issue
tracker's current state of each; the sponsored PDF's `/State` can be a year behind it.

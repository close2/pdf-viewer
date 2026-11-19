# Round 2126 — a portable conformance suite, designed and a first slice built

A docs round in the `gwg` worktree. RFC 0016 designs a suite of tests whose expected outcomes come
from ISO 32000-2 alone and that any processor runs through an adapter. The round builds a first slice
of 17 tests and asks Q948. No ledger row moves.

## What it built and found

- `portable-conformance/`: 17 TOML manifests (40 checks: 9 pixels, 3 text, 3 structure, 2 refusal)
  over §§7.3.7, 7.3.10, 8.4.3.3, 8.4.4, 8.5.3.3.2, 8.5.3.3.3, 8.5.4, 9.10.2, 9.10.3, 10.4.2.2,
  12.3.2.4, 12.4.2, 12.5.5, 12.7.4.1. There is a stdlib generator (byte-reproducible; `qpdf --check`
  finds no syntax error), a stdlib runner, and four adapters. `tools/portable-adapter` answers text
  and questions for this program, and pages come from `quorra-transform render`.
- Measured: quorra 17 of 17; poppler 15 (2 `n/a`); MuPDF 14 (3 `n/a`); Ghostscript 10.05.1 12, with
  3 `n/a` and 2 failures. Ghostscript reads a page's `/Rotate null` as a value (a control file without
  the entry turns), and it writes the ToUnicode surrogate pair `<D840DC3E>` as two CESU-8 units. Both
  §7.3.7 and §9.10.3 decide against it.
- Calibration (trap 13): one defect planted per test, and all 17 failed under this program.
- `portable_quotes.rs` holds each manifest's quotations to its own clause. On its first run it caught
  a §7.3.10 manifest quoting §7.3.7.
- ADRs 3205 (a top-level directory of readable source), 3206 (analytic expectations, no image), 3207
  (the runner, and the adapter as an executable), 3208 (the quotation gate). Q948 asks about the
  location, the licence and the adapters' scope.

## Ledger

None touched. The slice's rows are `implemented`, except §12.5.5, which stays `departed` (its
departure is `ca`/`CA`, not `/AS`).

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` over `-p portable-adapter -p conformance`
exit 0 on the second run (the first found an `expect` outside a test function); `pdf-sandbox --bins`
exit 0; nextest `--lib --bins --tests` of both 478 of 478 passed, and the failures equal
`expected-failures-current.json` (0 and 0); rustdoc `-D warnings --document-private-items` exit 0;
`cargo test -p conformance --no-fail-fast` exit 0. No public type changed, so fuzz is not owed.

## Habit

A quotation is held to the clause its file names. A whole-text search passes a verbatim sentence
filed under the wrong clause.

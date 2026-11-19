# Q948 — Where does the portable conformance suite live, under which licence, and how far do its adapters go?

Source: round 2126, RFC 0016 (`doc/rfc/0016-a-portable-conformance-suite-expected-outcomes-from-the-clause-alone.md`),
the owner's request of 2026-11-19 for "a portable conformance test suite (still in parallel to the
other tasks)". The first slice is built: 17 tests, 40 checks, under `portable-conformance/`.

## What the RFC found

- No public suite has expected values a machine checks that come from the standard rather than from
  a processor. pdf.js's 1282 `eq` tests compare against pdf.js's own snapshots. `pdf-differences`
  states its expectations in prose and screenshots. veraPDF's corpus asks whether a *file* conforms.
  The Ghent suite is judged by eye and cannot be redistributed.
- Under this program all 17 tests pass. poppler, MuPDF and Ghostscript 10.05.1 pass every test they
  can be asked, except two under Ghostscript: it reads a page's `/Rotate null` as a value rather than
  as absent (§7.3.7), and it writes a UTF-16 surrogate pair from a ToUnicode CMap as two separate
  code units (§9.10.3). In both cases the clause's text decides against Ghostscript.
- Every check fails when one defect is planted in its file (17 of 17 under this program).

## The questions

1. **Where does the suite live?** Recommendation: the top-level `portable-conformance/` now, and its
   own repository once it holds about a hundred tests, pinned here by version as `doc/corpora/`
   pins its submodules. *Meanwhile*: it stays in this tree, and the next slice builds the ledger tie
   (RFC 0016 section 6) against the in-tree directory.
2. **Which licence?** Recommendation: CC0 1.0 for the manifests and the PDFs they generate, and
   Apache-2.0 for the generator, runner and adapters. ISO's sentences are quoted as citation, a
   sentence or two per test, as the source comments already quote them. The alternative is CC BY 4.0,
   which veraPDF's corpus and `pdf-differences` use. *Meanwhile*: `suite.toml` and the README carry
   CC0's SPDX identifier, and nothing is published outside this repository.
3. **How far do the adapters go?** Recommendation: keep this program's CPU adapter, and add a shipped
   command that draws a page to a file through `render-raster`, so that a `quorra-gpu` adapter can
   judge the device's readings: the product draws page one on the device. poppler, MuPDF and
   Ghostscript stay as evidence. A pdf.js adapter waits on a Node runtime, which is a system package
   only the owner installs. *Meanwhile*: the four built adapters stand, and no evidence adapter's
   verdict changes an expected value.

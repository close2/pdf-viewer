# 3313 — The portable suite states its licences per file kind, in REUSE form

Session 2153. Status: **implemented.** Context: RFC 0016 section 4.4; the owner's A948 (2026-11-20):
CC0-1.0 for the manifests and the generated PDFs, Apache-2.0 for the generator, the runner and the
adapters, and the suite in this tree until the owner says otherwise.

## Decision

- **Where the licences are stated.** `portable-conformance/REUSE.toml` maps every path to its licence
  (REUSE specification 3.2), `portable-conformance/LICENSES/` holds `CC0-1.0.txt` (Debian's
  `common-licenses` copy of the Creative Commons text) and `Apache-2.0.txt` (this repository's own
  `LICENSE`), and each file states its identifier in its first comment. The tree has no other SPDX
  line: the repository is under one licence, stated once by the root `LICENSE` and each crate's
  `license.workspace`. The suite is the first part of it under two, and a file of it is meant to be
  copied out alone, so it is the part where a file has to say what it is under.
- **Per file kind.** CC0-1.0: `tests/*.toml`, `suite.toml`, `README.md`, `REUSE.toml`. A copied suite
  needs its format's description as much as its manifests, so the README is dedicated with them.
  Apache-2.0: `tools/` and `adapters/`.
- **A generated PDF carries `% SPDX-License-Identifier: CC0-1.0` as its third line**, after §7.5.2's
  header and binary comment. §7.2.4: comments "have no semantics defined by this document", and a
  processor "should ignore comments", so the line reaches a person and no processor. Not `/Info` and
  not XMP, because either is an object the test did not write: the generator adds only what a person
  should not compute by hand (ADR 3205), and a structure test's file would carry a dictionary its
  check never asked for. §7.2.4's NOTE 1 discourages comments that communicate *to processors*; this
  one communicates to people. The line shifts every offset by 35 bytes, which the generator computes.
- **ISO's sentences are not dedicated.** They are quoted as citation, as in the tree's comments; the
  README says that CC0 covers what the suite adds around them.
- **The suite stays in-tree**, laid out as a REUSE root of its own (`LICENSES/` beside `REUSE.toml`)
  so that a later move changes nothing in it.

## Cost

Two copies of the Apache text (root and suite) and one SPDX line per file. The copy is the price of a
directory that is complete when lifted out.

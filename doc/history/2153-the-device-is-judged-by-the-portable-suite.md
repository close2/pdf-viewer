# Round 2153 — the portable suite's licences, a page drawn on the device, and its adapter

An instruments round in the `gwg` worktree, on the owner's A948 (2026-11-20): the suite stays in
this tree until the owner says otherwise; CC0-1.0 for the manifests and generated PDFs, Apache-2.0 for
the generator, runner and adapters; a shipped `render-raster` page-drawing command and a
`quorra-gpu` adapter. The pdf.js adapter waits. No ledger row moves.

## What it built

- **Licences** (ADR 3313): `portable-conformance/REUSE.toml` maps each path to its licence in REUSE
  3.2 form, `LICENSES/` holds both texts, and every file names its identifier in its first comment.
  CC0-1.0: `tests/*.toml`, `suite.toml`, `README.md`, `REUSE.toml`; Apache-2.0: `tools/`, `adapters/`.
  A generated PDF's third line is `% SPDX-License-Identifier: CC0-1.0`: §7.2.4 gives a comment no
  semantics, where `/Info` or XMP would add an object the test did not write. `qpdf --check` clean.
- **`quorra-draw`** (ADR 3314, `crates/quorra-draw`, in `tools/batch.sh`'s `install_binaries`):
  `quorra-draw <pdf> [--page n] [--dpi d] -o <png>`, the transform's contract (one-based page, 150 dpi,
  `TargetSpec::for_page` at d/72, the 2^28-pixel budget, RGBA PNG, exits 0/1/2/4), in the window's
  view state, on the device the environment selects, never falling back to the processor. It installs
  the GPU lock hook itself: one queued hold of 77 ms per page, with a ticket.
- **`adapters/quorra-gpu`** (ADR 3315): pages from `quorra-draw`, text and questions from
  `portable-adapter`. A disagreement with `quorra` is read against the clause, not the other backend.

## Measured

- RX 580 (RADV POLARIS10): quorra 17 of 17, quorra-gpu 17 of 17; lavapipe: quorra-gpu 17 of 17. No
  disagreement, so no clause had to decide one. Ten of the eleven rendered PNGs are byte-identical
  between the backends (axis-aligned fills); the round cap's antialiased edge differs and still
  passes. With no Vulkan driver the command exits 1 naming no adapter, rather than drawing elsewhere.

**Gates.** Over `-p portable-adapter -p render-raster -p conformance -p quorra-draw`: fmt exit 0;
clippy `-D warnings --all-targets` exit 0 on the second run (the first found a bare `- 1` and a test
file without its lint expectation); `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` 637 of
637 passed (7 skipped), and the failures equal `expected-failures-current.json` (0 and 0); `cargo test
-p conformance` exit 0 on the second run (the first found this record's gates unstated); `cargo deny
check` exit 0; rustdoc `-D warnings --document-private-items` exit 0. No public type changed.

## Habit

A shipped command that draws on the shared device installs the lock hook the tests use, before
its device exists, and drops the device before it encodes: the hold is the device's life, no longer.

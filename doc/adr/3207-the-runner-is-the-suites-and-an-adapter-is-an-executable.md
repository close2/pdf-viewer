# 3207 — The runner is the suite's own, and an adapter is any executable

Session 2126. Status: accepted for the first slice; the adapters' scope is the owner's
(`doc/questions/Q948` question 3). Context: RFC 0016 section 5; ADR 3205.

## Decision

`portable-conformance/tools/run.py` (Python's standard library alone, including a 60-line PNG
reader) runs every manifest against one or more adapters and prints one table row per test, ordered
by clause. An adapter is any executable that answers `render <pdf> <page> <dpi> <out.png>`,
`text <pdf> <page>` and `query <pdf> <question ...>`, and exits 3 for a request it cannot be asked.
This program's adapter (`adapters/quorra`) draws through `quorra-transform render`, the command a
person runs. Its text and questions go through `portable-adapter`, a binary of nine
`pdf_model` calls in `tools/portable-adapter`. `adapters/poppler` and `adapters/mupdf` drive those
projects' command-line programs, and they are evidence only.

## Why

- **The runner must run where this tree does not build.** A Rust runner would ask a processor's
  authors for a Rust toolchain and this workspace. Python's standard library is already on every
  machine that runs poppler's or pdf.js's test suites.
- **An executable is the smallest contract a processor can meet.** A library interface would tie
  the suite to one language. A shell script around a command-line program is a few lines, as
  `adapters/poppler` shows.
- **This program's pixels come from the shipped command**, not from a test harness's own
  `interpret` and rasterise. A verdict is then about what a person gets from `quorra-transform`.
  The text and question routes have no shipped command that prints them as a line, so
  `portable-adapter` makes the same `pdf_model` calls the viewer does and adds no reading of its own.
- **Exit 3 keeps "cannot" apart from "wrong".** poppler's programs print no page label or field
  value, so those cells read `n/a`. A failure would have charged poppler with a reading it never
  offered.

## Cost

Python joins Rust and shell as a language the tree ships for a consumer to run. `tools/` already
holds Python for the tree's own use, but this is the first Python a consumer runs. The runner judges
nothing a manifest does not state, so a check type it lacks is an edit to the runner and the README,
and the suite's minor version changes with it (README.md, "Version").

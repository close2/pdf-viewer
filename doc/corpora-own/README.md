# Documents this project owns

**The first tracked PDF bytes in this repository, and the rule that keeps it small.** Everything
else this tree tests against is either a submodule under [`corpora/`](corpora) — which carries no
bytes into this history — or a fixture *built in code*, which is the tree's default and stays the
default: `crates/test-scenes` writes PDFs from Rust so that a fixture can be read, diffed and
varied one entry at a time (trap 8 wants pairs, and a pair of binaries is two opaque blobs).

A file belongs here only when **the artefact itself is the evidence** and rebuilding it in code
would lose what it witnesses. That is a narrow test, and each entry below says why it passes.

| file | bytes | who made it | why it is here |
|---|---|---|---|
| `type4_pi.pdf` | 2357 | the project owner, 2026-08-14 | A §7.10.5 PostScript calculator function that computes π by the BBP series and paints digits as rectangles — **written by hand, with comments**, which is the construction this program refused until ADR 0361. Reproducing it in code would reproduce a *reading* of it; the file is the thing that found the defect. Named by `pdf-model`'s `shadings::the_owners_pi_file_paints_the_digits_its_program_computes`, which asks it for the picture: five digit strokes black, three background points white, so a page that is blank, inverted, mirrored or shifted fails. |
| `pi_seven_segment.pdf` | 3325 | the project owner, 2026-08-15 | The second §7.10.5 function, and the one with a *number* attached: 2580 bytes of PostScript driving a seven-segment display through `{ } ifelse` branches, where the first file's program is straight-line arithmetic. It is the witness for `doc/QUORRA_FUNCTION_PAINT.md`'s measurement — 1142.8 ms in `scene` against `mutool`'s 15 ms — so a round that makes a sampled shading cheap has a document to prove it on, and a round that makes one *wrong* has branches to fail on. |


**Licence and provenance.** These are the project's own work, made by the project owner and
committed with their permission, so `doc/third-party-data.md`'s questions do not arise for them —
which is also why an outside document may not be added here. A file from anywhere else goes
through that document's licence question and, if it passes, through a submodule under `corpora/`.

**Nothing here is a corpus.** The corpus gates walk `doc/pdf.js` and `doc/corpora/`; these files
are witnesses named by the tests that use them, and a test naming one says what it is asking of it.

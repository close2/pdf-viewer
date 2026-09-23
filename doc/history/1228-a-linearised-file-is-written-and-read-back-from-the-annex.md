# Session 1228 — a linearised file is written, and read back from the annex

Batch thirty-five, parallel. Contract: `doc/questions/A03` — Annex F ratified on 2026-09-22.

**Built.** `pdf_syntax::linearize::serialize_linearized` writes Annex F's eleven parts (part 5
before part 4, no part 10) from the serializer's `Assembly`, with the page offset and shared object
hint tables and every table Table F.2 makes required of the document (`/T /O /A /E /V /I /C /R /B`,
plus `/L`). Every offset is computed to a fixed point before a byte is written. `state` is F.1's
reader half. `optimize --linearize` drives it; object streams and encryption with it are refused
by name at exit 4. ADR 1293 is the design; `doc/questions/Q131` the one question left.

**Read back from the annex.** `tests/support/linearized.rs` is a second reader of Tables F.1 to
F.12, and `faults()` asks a file everything the annex lets a reader check against its
cross-reference chain; a planted change to header item 2 is named. `tests/linearize.rs` has
thirteen tests over five fixture shapes, and `tests/optimize_corpus.rs` has a linearised arm.

**Corpus (pdf.js, 974 documents).** 959 linearised and read back with their pages and the
producer's decoded content, 958 page ones drawn bit-identically (the other draws on neither side),
0 Annex F faults, 0 not idempotent, 1 refused (`Pages-tree-refs.pdf`, whose page tree is a cycle).
The arm found three defects on the way, all fixed: `Pages::indices` names interior nodes too, so
pages come from `Pages::get`; an empty `/Threads` array is no thread; and `issue15590.pdf`'s
`/OpenAction` names the page object itself, which part 4 had taken from part 6.

**qpdf, read and run as evidence.** Read for F.3.7's walk (`updateObjectMaps`): agreement. Its
checker refused nodes still stating inheritable attributes, which led to reading F.3.10's "pushed
down" as moved: agreement. Two disagreements, each taken to the annex, which stands: qpdf pads
every item's run to a byte where F.4.1 says fields run "without regard to byte boundaries"
(padding the runs in a scratch build cleared its warnings, then reverted), and qpdf takes page 0 as
the first page where F.3.7 lets `/OpenAction` choose. `qpdf --check` on ten corpus outputs: two
with no warning (`freeculture.pdf`, whose 352 pages make every run a whole byte, and
`labelled_pages.pdf`), seven with only the packing difference, and `issue15590.pdf` exit 2 on a
`/Pages` naming a page, which its source has too.

**Ledger.** F, F.3, F.3.1, F.3.5, F.3.7 `partial`; the other eighteen `implemented`; all left
`out-of-scope`. `doc/todo/65` places F.3.1, F.3.5 and F.3.7 in bucket 4 and F, F.3 in the aggregates.
**CLAUDE.md**: the "Annex F stays excluded" sentence now says it is in scope since the ratification.

**Owed.** Object streams inside a linearised file (about a round), encryption inside one (under a
round), F.3.7 (b)'s bead arrays (about a day) — ADR 1293 section 5.

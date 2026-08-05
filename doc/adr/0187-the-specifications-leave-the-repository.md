# ADR 0187 — The specifications leave the repository, and its history

Status: accepted, 2026-08-05 (session 311).

## Context

Fourteen documents in `doc/` — ISO 32000-2:2020 including Errata Collection 3, ISO 14289-1 and -2,
ISO/TS 32001 to 32005, and five PDF Association notes and guides — are what `CLAUDE.md` principle 5
calls the only source of truth. Beside them, `doc/md/` held all fourteen converted to Markdown,
which is what `conformance::clause` reads to build the standard's clause index and what
`conformance::quote` checks 416 verbatim quotations against. 460 tracked files, 105 MB, most of a
64 MiB pack.

**The project owner is not licensed to redistribute them.** They are free to obtain — the PDF
Association hosts sponsored copies at no charge — and *free to download* is not *free to pass on*.
A git repository carrying them passes them on to everyone who clones it, and this repository has a
remote. `NOTICE` section 3 said the true thing in the wrong tense: "not redistributed by any
**build** of this program". The build was never the distribution. The repository is.

The Markdown is not a way out of it. It is a conversion of the same text, verbatim enough that the
conformance gate checks quotations against it, which makes it a derived work rather than an index.
Both go.

## Decision

**Stop tracking them, ignore them, and remove them from every commit in the history.**

- `git rm -r --cached doc/md 'doc/*.pdf'`, with `.gitignore` covering both in the same commit, so
  the files stay on the disk of whoever has the right to have them and are invisible to git.
- `git filter-branch --index-filter` over every ref, dropping the same two paths from all 436
  commits, then `git push --force`. The 436 hashes all change; there is one author in the whole
  history, which makes this the cheapest it will ever be and is why the item was ranked above
  every band of engineering work in `doc/todo/`.
- `NOTICE` section 3 rewritten to describe what is then true: the documents are not here, they
  come from <https://pdfa.org/>, a developer's own copies go in `doc/` under the names the code
  opens, and `doc/md/` is produced from them by docling.

**What is deliberately *not* done, decided by the project owner in this session.** The todo file
that had carried this item asked for a bootstrap written first and for every gate, test and
example that opens one of these files to grow a skip path and a printed sentence. Neither is
taken. The references stay exactly as they are — `expect("the specification is committed in
doc/")` and all — and it is the developer's job to put their own copies where the code looks. The
cost of that is legible and small: four tests and eleven measurement examples fail loudly on a
checkout without the documents, and a failure that names the missing file is not a mystery. The
cost of the alternative was fifteen edits to working code and a permanent second path through it,
paid so that a clone nobody has yet made would be quieter. Should that clone arrive and the
loudness matter, the skip paths are still one afternoon away.

## Consequences

- **The repository may be published**, which it could not before. Nothing else in `doc/todo/` had
  to be true before that.
- **Clone cost falls by most of the pack.** This was also the largest single thing available to do
  about it, and the only item whose price rose with every commit.
- **A fresh clone builds and runs the program, and cannot check a citation.** `cargo test -p
  conformance` needs `doc/md/ISO_32000-2_sponsored_EC3.md`; without it the gate that verifies 4033
  citations and 416 quotations does not run at all. That is the load-bearing consequence and it is
  the reason this ADR exists rather than a commit message.
- **Anyone holding a clone must re-clone.** Every hash changed.

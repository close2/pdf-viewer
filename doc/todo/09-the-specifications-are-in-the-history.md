# The specifications are in the repository, and in its history

Status: **owed, and it blocks publishing this repository anywhere.**
Priority: 09 — not a standing task, and placed above every band of engineering work below it
because nothing else in this directory has to be true *before the repository is pushed*.
Corpus: —
Clauses: none. This is a licensing question, not a reading of the standard.
Code: `NOTICE` §3, `tools/conformance`, `doc/PLAN.md` §5a, ten `examples/` and two tests.

## The problem, stated by the project owner

**The project owner is not permitted to redistribute these documents**, and 28 tracked files in
`doc/` are them:

| what | files | size |
|---|---|---|
| `doc/*.pdf` | 14 — ISO 32000-2 itself, ISO 14289-1 and -2, ISO/TS 32001 to 32005, and five PDF Association notes | 30 MB |
| `doc/md/**` | the same fourteen documents converted to Markdown, and a `processed/` copy | 75 MB |

They are **free to obtain** — the PDF Association hosts sponsored copies of ISO 32000-2 and the
technical specifications at no charge, which is where these came from — but *free to download* is
not *free to redistribute*, and a git repository that carries them is redistributing them. Putting
this tree on a public forge with those objects in it publishes them.

`NOTICE` §3 already says the right thing and says it in the wrong tense:

> `doc/*.pdf` and `doc/md/*` are ISO 32000-2 and the PDF Association's application notes … They
> are ISO's and the PDF Association's, under their own terms, and are not redistributed by any
> build of this program.

"Not redistributed by any **build**" is true and is not the claim that matters. The repository is
the distribution.

## The Markdown is not a way out

`doc/md/` is a *conversion* of the same text — this file's own gates depend on it being verbatim,
which is exactly what makes it a derived work rather than an index. Removing the PDFs and keeping
the Markdown would keep the whole problem and lose the ability to check a quotation against the
original (`tools/conformance/tests/conformance.rs`'s own comment says to check `doc/`'s PDF before
editing a quote). **Both go.**

## How they come back, for a person who has the right to have them

The documents are downloaded once, by hand, from the PDF Association. The Markdown is then
**generated rather than fetched**, with [`docling`](https://github.com/docling-project/docling):

```sh
# once, per developer, into a directory git ignores
pip install --user docling
for pdf in doc/*.pdf; do
  docling --to md --output doc/md/ "$pdf"
done
```

That is the whole bootstrap, and it is why this is a deletion rather than a loss: the *inputs* are
a download anybody may make and the *artefact* is reproducible from them. What has to be written
down beside it is which docling version produced the committed conversions, because a different
converter will move line numbers and `conformance::clause` reads them.

## What has to happen, in order

1. **Write the bootstrap first**, in `doc/PLAN.md` §5a and in `doc/HANDOVER.md`'s "Verify it":
   where the fourteen documents come from, the docling invocation, and the version. A history
   rewrite that leaves the tree unbuildable for the next person is worse than the problem.
2. **Make every gate say so rather than fail.** `tools/conformance` reads
   `doc/md/ISO_32000-2_sponsored_EC3.md` and asserts against it; the ten examples and two tests
   that open `doc/ISO_32000-2_sponsored_EC3.pdf` do the same. Each must skip with a printed
   sentence when the file is absent — the pattern `corpus()` already uses for the pdf.js
   submodule, which prints "the pdf.js submodule is not checked out; skipping". **This is the step
   that makes the repository honest**: a clone without the specifications must be a clone that
   builds, tests what it can, and says what it could not.
3. **`.gitignore` `doc/*.pdf` and `doc/md/`**, in the same commit that stops tracking them, so the
   files can stay on a developer's disk.
4. **Rewrite the history.** `git filter-repo --path doc/md --path-glob 'doc/*.pdf' --invert-paths`
   over every ref, then a force push. `filter-branch` also works and is slower and worse; the
   repository is 64 MiB packed and most of it is these objects, so this is also the largest single
   thing that could be done about clone time.
5. **Correct `NOTICE` §3** to describe what is then true: the documents are not in the repository,
   here is where they come from, here is how the Markdown is produced.
6. **Tell anyone with a clone**, because every commit hash changes. At the time of writing there
   is one author in the whole history, which makes this the cheapest it will ever be.

## Why it is priority 09 and not 50

`50`–`59` is "blocked on a dependency, a decision or an infrastructure this program does not
have". Nothing blocks this. It is one decision the owner has already taken, one afternoon of
work, and it is the only item in this directory whose cost *rises* with every commit — the objects
are in the pack for ever until the history is rewritten, and every commit added before the rewrite
is one more hash that changes.

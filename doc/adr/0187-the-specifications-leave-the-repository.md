# ADR 0187 — The specifications leave the repository in the clear, and come back encrypted

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

But the gates cannot do without them. `cargo test --workspace` runs the conformance gate over 4033
citations and 416 quotations, and four tests open a PDF; CI runs that on every push. So the
requirement is not "remove them", it is **remove them from what a clone publishes while leaving
them where a build can reach them.**

## Decision

**Untracked in the clear; tracked encrypted; and the same two paths removed from the history.**

- `git rm -r --cached doc/md 'doc/*.pdf'`, with `.gitignore` covering both in the same commit, so
  the unpacked copies stay on the disk of whoever has the right to have them and are invisible to
  git.
- **`doc/specifications.zip`**, tracked: the fourteen PDFs and the fourteen Markdown conversions,
  ZipCrypto, 37 MB. `unzip -P <password> doc/specifications.zip` from the workspace root puts
  every file back exactly where the code looks — the entries carry their `doc/` prefix, and a
  round trip is byte-identical, checked by hash.
- **CI opens it from a repository secret**, `SPEC_ZIP_PASSWORD`, in a step before the tests. The
  project owner's framing is the right one and is worth recording: **CI is a developer here**, and
  a developer has to provide the documents before the gates that read them will run. The secret is
  passed through the environment rather than interpolated into the command line, and the step
  fails with a sentence naming the secret when it is absent — which is what a pull request from a
  fork sees, since a fork does not receive secrets.
- `git filter-branch --index-filter` over every ref, dropping the same two paths from all 436
  commits, then `git push --force`. The 436 hashes all change; there is one author in the whole
  history, which makes this the cheapest it will ever be and is why the item was ranked above
  every band of engineering work in `doc/todo/`. **This half is the project owner's to run**, and
  as of this ADR it has not been: the working tree and the index are clean of the documents in the
  clear, and every commit before this one still carries them, so the repository is not yet
  publishable. The command is in `doc/HANDOVER.md`'s "Verify it", and every commit added before it
  runs is one more hash that will change.
- `NOTICE` section 3 rewritten to describe what is then true.

**The password is not a security measure and is not treated as one.** Stated by the project owner:
the ISO text is free and hundreds of unprotected copies of it are on the internet, so the archive
is not defending a secret. What it does is make a clone of this repository not *be* a
redistribution — the bytes are here, and reading them takes a permission the owner grants rather
than an act anybody performs by typing `git clone`. ZipCrypto is chosen over AES for exactly that
reason: it is the format every `unzip` on every runner already opens, and there is nothing to
defend that would justify a stronger one.

**What is deliberately *not* done, decided by the project owner in this session.** The todo file
that had carried this item asked for a bootstrap written first and for every gate, test and
example that opens one of these files to grow a skip path and a printed sentence. Neither is
taken. The references stay exactly as they are — `expect("the specification is committed in
doc/")` and all — and it is the developer's job to unpack the archive. The cost of that is legible
and small: four tests and eleven measurement examples fail loudly on a checkout where nobody has,
and a failure that names the missing file is not a mystery. The cost of the alternative was
fifteen edits to working code and a permanent second path through it, paid so that a clone nobody
has yet made would be quieter. Should that clone arrive and the loudness matter, the skip paths
are still one afternoon away.

## Consequences

- **The repository may be published once the history rewrite has run**, which it could not before
  and cannot until then. Nothing else in `doc/todo/` has to be true before that.
- **Clone cost falls from 105 MB of documents to one 37 MB blob**, and that is a smaller win than
  the removal alone would have been. It is bought back honestly: these documents change
  approximately never, so there will be exactly one blob, and the alternative — the archive kept
  outside the repository and fetched by CI — was weighed and rejected because it puts a second
  thing between a clone and a green build without protecting anything more (a release asset on a
  public repository is a public download; the password is the only protection either way).
- **A fresh clone builds and runs the program, and cannot check a citation until it unpacks.**
  That is the load-bearing consequence and the reason this ADR exists rather than a commit
  message.
- **Anyone holding a clone must re-clone** once the history is rewritten. Every hash changes.
- **The password now has to live somewhere a person can find it**, which is a small permanent
  obligation this project did not have before: the GitHub secret, and the owner.

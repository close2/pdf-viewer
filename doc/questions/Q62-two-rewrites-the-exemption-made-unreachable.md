# Q62 — Two built rewrites the exemption leaves reachable by no document

Source: the merge of rounds 999–1004 (session 1005), reading session 1001's `Examination::reaches()`
and `reach::exemption_narrows` (ADR 1021) against session 947's and 962's rewrites.

## What happened

ISO 19005-2 section 6.2.2's last sentence withdraws a requirement from a **named resource no
content stream references**. Nothing in this tree could see that until session 1001 gave
`Examination` a reachability answer; `reach::exemption_narrows` now applies it to every requirement
except section 5.1's and the file-structure subclauses, which is what the clause and TechNote 0010's
A010 say between them.

Two rewrites the converter already carries are, in consequence, reachable by no document:

| rewrite | drawn | not drawn |
|---|---|---|
| `Rewrite::PostScriptXObject` (ISO 19005-2 6.2.9.3) | **refused** — the object must go, the `Do` that names it may not be edited (`doc/pdf-a-conversion-limits.md` section 5.2), and removing it leaves a named resource undefined, which 6.2.2 forbids | **exempt** — ISO 32000-1:2008, 8.8.2 (the edition part 2 delegates to, and the only one that has PostScript XObjects at all: ISO 32000-2 8.8.1 names two kinds and not this) says such fragments "shall have no effect either when viewing the document on-screen or when printing it to a non-PostScript device", so *no* page ever invokes one |
| `Rewrite::SymbolicTrueTypeEncodingRemoved` (6.2.11.6) | **refused** — the same subclause requires a rendered symbolic font's program to carry a `cmap` subtable of the kind the part names; the face this suite builds from carries the Microsoft Unicode one, and that rule is behind ADR 0816's fence | **exempt** — a font dictionary no mark shows is not judged against the font rules |

Both were built against tests whose fixtures declared a resource their page never drew. Those tests
now assert the exemption instead (`crates/pdf-transform/tests/archive.rs`), which is true and is
what the clause says — but it means neither rewrite has a document that reaches it.

## Why it needs the owner

Three answers are defensible and they are not the same project.

1. **Leave them.** Dead code with a passing neighbour is cheap, and a future part or a future
   fence amendment may reach them. Cost: two rewrites nothing exercises, which is the shape
   `CLAUDE.md` principle 1 calls a promise nothing keeps.
2. **Retire them**, with the argument recorded, and let the refusal stand in both cases. Cost: a
   later document that *is* reachable would need them written again — and the reachability rests on
   ADR 0816's fence, which has been amended three times.
3. **Reopen the fence for the drawn case.** The symbolic-font rewrite is refused because rendering
   the fixture's face needs a `cmap` subtable the part names and this face lacks; that is a
   *fixture* limitation as much as a fence one. A face that carries the right subtable would make
   the rewrite reachable and the test real.

## What the tree does meanwhile

Both rewrites stay, both are covered by tests asserting the exemption rather than the rewrite, and
`archive_corpus` over the veraPDF corpus is unmoved — conforming in, conforming out, on all six
targets. No document in any corpus on this disk reaches either rewrite, which is why nothing
noticed until the exemption was readable.

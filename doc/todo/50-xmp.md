# §14.3.2's XMP

Status: **not read, and the absence is named rather than silent.**
Priority: 50 — blocked on a dependency decision this tree has not taken
Corpus: 319 documents carry the stream
Clauses: §14.3.2, with §12.2's `/DisplayDocTitle` and §14.3.3
Code: `crates/pdf-model/src/metadata.rs`

XMP is RDF/XML, so reading it is an **XML parser over untrusted bytes** — a dependency decision
of the kind `zune-jpeg` and `skrifa` were, with a fuzz target owed alongside it, and this tree has
not taken it.

**What the absence costs is printed rather than hidden.** §12.2's `/DisplayDocTitle` asks for
`dc:title`; this program shows §14.3.3's `/Info /Title` instead, which Table 349's own NOTE 1
calls the same fact in the other place. That substitution is a *choice*, and it is printed once
for each of the 18 documents that set the flag and carry a stream:

```text
note: this document asks for its title in the title bar (§12.2's /DisplayDocTitle), which names
XMP's dc:title; this program reads no XMP and shows §14.3.3's /Info /Title instead
```

Before taking it, the questions ADR 0014 asked of every other dependency: which crate, what its
attack surface is, what it does on malformed input, and what the fuzz target looks like. A
streaming pull parser over a bounded input is the shape to want; a DOM builder over an untrusted
319-document population is not.

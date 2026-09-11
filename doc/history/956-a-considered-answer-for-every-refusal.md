# 956 — A considered answer for every refusal, and what writing them taught the format

Date: 2026-09-11. ADR: none of its own — the findings went into RFC 0007 §5b.
Files: `doc/pdf-a-mitigations.md` (new), `doc/profiles/{refuse-any-loss,as-if-printed,
only-metadata-loss,keep-everything}.toml` (new),
`doc/rfc/0007-a-refusal-is-a-question-somebody-can-answer-in-advance.md` §5b.

The owner asked for every refusal to be stepped back from and considered one at a time. This round
did that literally: **one entry per refused requirement**, each answering four questions — what is
lost, what remedies exist, which of them the selected target admits, and whether a human could
choose it from a configuration file.

Writing them is the first real test RFC 0007's format has had, and it failed five times. Those are
§5b and they are the round's actual output; the catalogue is the evidence.

**`supply` is a fifth remedy kind**, and it is the direct answer to the owner's question about
external tools. RFC 0007's four kinds all describe what happens to the *document's* content. A
whole class of refusals is unblocked instead by **the operator stating a fact the document does
not** — a role map for a producer's own structure types, the archive's language, which of two
conflicting `Separation` definitions wins, an attachment's media type, which write mode a CMap's
producer meant. No tool makes anything and nothing is inferred; it is a human putting their own
knowledge into the file, recorded as theirs. It is the commonest way a refusal in this catalogue
becomes a conversion, and it does not cross `A48`'s line, because `A48` binds *this program's*
guessing rather than an operator's statement.

**A site is finer than a requirement, and the format assumed they were the same.** At least seven
distinctions split one requirement into two shapes with different answers — a blend mode as an
array versus a bare name, an inline image filtered with `LZWDecode` versus `Crypt`, a resources-less
content stream that is a page versus one that is a form XObject, an appearance dictionary with an
`/AS` versus without. An operator who answers the safe shape must not be taken to have answered the
dangerous one, and a key of requirement-identifier alone cannot say that.

**And the sharpest one: a third of the refusals should not be configurable at all.** Twenty-two of
them refuse a requirement whose right answer **loses nothing** — they were waiting on code, not on a
decision, and offering an operator a `discard` to get past one would trade a permanent loss for a
missing afternoon's work. They are marked *owed, not optional*, their configuration answer is *none
by design*, and §13.3 is the list. That is the catalogue's most actionable output for the converter
itself: nearly a fifth of everything the converter refuses is a lossless rewrite nobody had written.

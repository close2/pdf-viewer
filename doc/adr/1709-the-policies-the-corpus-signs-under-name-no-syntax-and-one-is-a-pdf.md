# 1709 — The policies the corpus signs under name no syntax, and the one most of them use is a PDF

Session 1436. Status: **accepted**. Amends nothing; it measures the block ADR 1219 described as per
file and named at runtime. Context: ISO 32000-2 §12.8.3.4.4; ETSI EN 319 122-1 clauses 5.2.9.1,
5.2.9.2 and 5.2.10, held and cited by clause, never quoted (ADR 1085). Instrument:
`crates/pdf-signature/examples/signature_algorithm_census.rs`, run over every document on this disk
carrying a `/ByteRange` (1 706 of 90 765 under `corpus-cache`, `doc/corpora`, `doc/corpora-own` and
`doc/pdf.js/test/pdfs`), and a scratch reader of each policy attribute's qualifiers and digest.

## 1. What the signatures state

Of the signatures stating a `signature-policy-identifier`, 39 state the implied-policy alternative
clause 5.2.9.1 forbids, one is not the structure the clause defines, and **16 name an explicit policy
under four identifiers**: `2.16.724.1.3.1.1.2.1.9` (13 signatures, all from the `cc-main-2021-31`
crawl, SHA-1), `1.2.250.1.120.100.2.1.1`, `2.16.620.2.1.2.2` (each one, SHA-256) and
`2.16.76.1.7.1.13.1` (one, whose digest algorithm is a signature algorithm's identifier and whose
digest value is its own identifier's DER encoding). **None carries clause 5.2.9.2's specification
qualifier, and none carries clause 5.2.10's store.** Five carry the URL qualifier, each naming a PDF.
So no signature in reach *names* a syntax: where the qualifier is absent clause 5.2.9.1 leaves the
specification to the context of the signature.

## 2. What the context says, for thirteen of them

The URL three of the thirteen name no longer resolves; the Internet Archive's copy of it is a 26-page
PDF, and **its SHA-1 over the whole file is the digest the thirteen signatures signed**
(`1bbae8b9…02cd`). A digest does not agree by accident (ADR 1219), so that policy document is a PDF
and its digest's input is the file's bytes: a syntax this tree holds, and a human-readable one —
clause 5.2.9.2's note names human-readable as one of the three kinds. The other three identifiers'
documents were not reached: on 2026-10-08 the French URL gave no answer and the Portuguese one a
404, the archive holds neither, and the Brazilian signature names none.

## 3. Decision

**A human-readable policy is to be bound and shown, not enforced, and the row stays `partial` on
enforcement.** Its constraints are prose addressed to a person, so no comparison a program makes
against them is the policy's own, and calling the presentation of a document *enforcing* it would
claim what the program does not do. What the tree can do for such a policy is ADR 1219's binding over
the fetched bytes and the document opened for the reader; that build waits on a fetch through A98's
client under a reader's level, and it is the next round's to take, with
`signatures.rs::every_corpus_signature_is_asked_which_policy_it_was_made_under` (ten signatures,
none stating a policy) unchanged because none of the sixteen is in that population. Enforcing a
machine-readable policy stays blocked per file, as ADR 1219 has it; no signature in reach states
one.

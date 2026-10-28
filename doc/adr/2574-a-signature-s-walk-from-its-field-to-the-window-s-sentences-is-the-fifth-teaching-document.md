# 2574 — A signature's walk from its field to the window's sentences is the fifth teaching document

Session 1870, slot 10 of batch 127, a docs round. Status: **accepted** and **built** in
`doc/teaching/how-a-signature-is-verified.md` and `tools/conformance/tests/teaching.rs`. Context:
the owner's code-quality slot; ADR 2002 (the first walk and the diagram convention), ADR 2003 (what
the gate holds and what a note holds), ADRs 2062, 2078 and 2554 (the second to fourth walks); ADR
0215 (the three questions), ADR 1039 (anchors are an input), ADR 1076 (the word is a type), ADR
2560 (the policy's constraints enforced). Clauses: ISO 32000-2 §12.8.1, §12.8.3.3.1, §12.8.3.4.4,
§12.8.3.4.5, §12.8.4 and §12.8.5; ETSI texts cited by clause and never quoted (ADR 1085). ADR 2575
and Q736 are unused.

## 1. The gap

`pdf-signature` is thirty thousand lines in which almost every module comment argues one sentence
of §12.8.1, and the three walks that cross `viewer-core` leave it out. A new contributor met it
through `lib.rs` and the module comments, each of which says what a part is; none says what
happens *in order* to one signature, or where the word *valid* is withheld.

## 2. Decisions

1. **Drawn at one commit, with one round's work on it, and said so.** The walk describes commit
   2d54febf with round 1863's `policy_rules.rs` and its `notes.rs` sentences on it (ADR 2560),
   read from a snapshot of the shared tree taken once 1863 had reported. Its names were held by the
   gate in the shared tree and in an export of HEAD carrying those files.
2. **Five stages, in the order the code takes them**: the field, its byte range and the digest
   (`signature.rs`); the CMS container and the signer's key (`der.rs`, `cms.rs`, `ess.rs`,
   `x509.rs`, the five families); the path, the file's revocation material and the instant
   (`trust.rs`, `revocation.rs`, the security store, `timestamp.rs`); the policy (`policy.rs`,
   `policy_rules.rs`); the verdict and the sentences (`verdict.rs`, `notes.rs`, the report's
   deferral and the window). The brief's hypothesis — five stages, six diagrams — held.
3. **The digest is stage 1 although the CMS reader is stage 2.** `Signature::integrity` reads the
   recorded digest through `cms::signed_data`, so stage 1 names the call and stage 2 says what it
   does; the order follows §12.8.1's questions rather than the call graph.
4. **The policy is drawn beside the verdict, not before it**, because that is where the code puts
   it: `Verdict::reached` takes no policy finding, and `notes.rs` says the findings after the
   verdict's sentence. Whether a broken constraint should withhold the word is reported as a
   finding for a later batch rather than drawn as if it did.
5. **One running example, read off a build.** `bug854315.pdf` from the pdf.js corpus, opened in a
   `dev` build of `quorra` from the export under Xvfb with no anchors: an `adbe.pkcs7.detached`
   signature with no signed attributes, so stage 2's answer settles stage 1's and the report says one
   sentence where it would otherwise say two; its four lines are in the walk verbatim. It is named by its file name
   alone because the corpus is a submodule this tree links rather than holds.
6. **The gate holds five walks from one table.** `WALKS` gains the entry with six diagrams; the
   calibration is unchanged. A renamed path planted in the new walk was reported by name and then
   taken out.

## 3. Cost

About four hundred lines of prose held by the gate only for names, over a crate whose sentences are
its product. The notes are the price: each lists its files, and stage 4's names the two policy
syntaxes still owed as the change that redraws it.

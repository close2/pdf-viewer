# 2554 — A document's walk from its profile to a PDF/A file is the fourth teaching document

Session 1860, slot 10 of batch 126, a docs round. Status: **accepted** and **built** in
`doc/teaching/how-a-document-becomes-pdfa.md` and `tools/conformance/tests/teaching.rs`. Context:
the owner's code-quality slot (one per batch, on Opus for batch 126); ADR 2002 (the first walk and
the diagram convention), ADR 2003 (what the gate holds and what a note holds), ADRs 2062 and 2078
(the second and third walks); ADR 0947 (the converter's three stages), ADR 2534 (the settling pass);
RFC 0014 (what comes next). Clauses: ISO 32000-2 §12.5.5, §12.5.6.6 Table 177, §12.7.4.3; ISO
19005-2 sections 6.2.8.1, 6.3.3 and 6.6.6, cited by section and never quoted. ADR 2555 and Q726
are unused.

## 1. The gap

The converter is the largest subsystem a round of the last batches touched, and the one a new
contributor met only through its module comments, its RFCs and `doc/pdfa-remedies.md` — each of
which says *what* a part is, and none of which says what happens *in order* to one document. The
three walks before this one each follow one thing through the program; none follows a document to
PDF/A.

## 2. Decisions

1. **Drawn from the code at one commit, and said so.** Four slots of the same batch were changing
   `crates/pdf-transform/src/archive/` while the walk was drawn (RFC 0014's ordered answer, the
   statement page, `derive` widened, the operator's base). The walk describes commit 0c34fee4's
   converter, read as `git show HEAD:<path>` where the shared tree differed, names RFC 0014 in its
   opening as what comes next, and names in its last section the stages that RFC's build redraws.
   Its names were held by the gate in the shared tree and in an export of HEAD alike.
2. **Five stages, in the order the code takes them**: the profile (read by the caller, checked
   against one target, folded into `ArchivePlan`), the verdict and one `Decision` per failure (the
   operator's answer before the table's), the remedy in four shapes (a rewrite or statement, a
   removal under a `Loss`, a fact supplied, a page), the output held to the target and settled
   (ADR 2534), and the report in three forms (text, JSON, `xmpMM:History`) with the caller's tool
   loop. The brief's hypothesis — five stages, six diagrams — held.
3. **The `xmpMM:History` entry is drawn in stage 3, not stage 5.** The packet is composed in
   `prepare.rs::the_packet` before the rewrite and is one of the objects the rewrite inserts, and
   `unless_recorded` makes every recorded remedy conditional on it; drawing it with the report would
   put a condition of the remedy after the remedy. Stage 5 names it as the third reader.
4. **One running example that reaches the settling pass.** A one-page invoice: `DeviceRGB` marks, an
   unembedded `Helvetica`, an image with `/Interpolate true`, and a free-text annotation with a
   `/DA` and no `/AP`. Under `as-if-printed` at 2b it fails eight requirements, every decision
   proceeds, the constructed appearance's `/Helv` makes the first output fail
   `fonts/font-programs-embedded`, and the second pass embeds it; veraPDF passes the output. With no
   profile it is refused on `image-smoothing`. Every report string in the walk was read off those
   two runs of `quorra-transform` built from HEAD.
5. **The gate holds four walks from one table.** `WALKS` gains the entry with six diagrams; the
   calibration is unchanged. A renamed path planted in the new walk was reported by name and then
   taken out.

## 3. Cost

About five hundred lines of prose that the gate holds only for names, in a subsystem that changes
every batch. Its notes are the price: each lists the files its diagram depends on, and the RFC 0014
build is told by name which two sections it redraws.

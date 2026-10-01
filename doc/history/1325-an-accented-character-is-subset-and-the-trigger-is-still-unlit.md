# 1325 — An accented character is subset as the outline it draws, and §7.6.5's trigger is still unlit

Date: 2026-10-01. ADRs 1485, 1486. Batch fifty, the ledger slot.

**The public-key rows.** Briefed to build §7.6.5 if the owner's answers said so. `A66` ("Shared crate
below both. Wait for a real trigger.") and `A168` ("I agree with your recommendation (including the
extra thing).") say the PDFBox keystore documents are the witness set, not the trigger. Everything
`A168` owes was already in the tree: the ledger note, `doc/todo/65`'s fifth bucket, and
`public_key_witnesses.rs`, which passes. Nothing was built. §7.6.5–§7.6.5.3 stay `reported`, and
§7.6 and §7.6.6 stay `partial` (ADR 1485 section 1).

**Annex H.2.** `hayro-jbig2`'s MQ decoder is `pub(crate)`, and no public route decodes under one
fixed context, so the test sequence cannot be a `pdf-sandbox` fixture. It is a unit test inside the
codec instead, shipped as `doc/patches/hayro-jbig2-annex-h2-test.patch`. Before each of the 256
decisions it compares I, MPS, A, CT and the software-conventions C with Table H.1, then the
decision, and the decisions are cross-checked against the 32 packed bytes. It was applied to a
scratch clone of the fork at the pinned rev. It passes there, and one perturbed Qe entry makes it
fail at EC 95. The patch applies beside the two other patches. Why the numbers count as a test
vector and not ITU's prose: ADR 1485 section 2.

**seac.** TN #5177 Appendix C finds an accented `endchar`'s components by glyph name, and a
CID-keyed subset has no names, so keeping the two components (the brief's plan) would not work.
`embed::accented` writes the outline the form draws instead: the components are evaluated in 16.16,
the accent is moved to `(adx, ady)`, and the original width is kept, with no hints (the cost, ADR
1486). `reach` now reads the width under Note 4. No machine face uses the form (fontTools scan), so
the witness is a hand-built face: the moves are worked by hand, the advance is the stated one, and
the outlines match the whole face's. Dropping `ady` fails the test.

Files: `crates/pdf-font/src/embed.rs`, `embed/accented.rs` (new), `embed/cff.rs`, `embed/reach.rs`,
`doc/patches/hayro-jbig2-annex-h2-test.patch` (new), `doc/state-of-play.md`, the two ADRs, this file.

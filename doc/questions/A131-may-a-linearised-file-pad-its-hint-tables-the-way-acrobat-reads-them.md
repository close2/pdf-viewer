Status: complete
Given: 2026-09-23, in conversation — transcribed by the round
Owes: the padding written — `pdf_syntax::linearize`'s hint tables padding every item's run to a byte boundary, the way Acrobat, pdlin, qpdf and PDFium read them — with the F.4.1 ledger row and ADR 1293 recording the reading and its evidence: the exception sentence's whole-table reading restates the byte-offset stream-attribute rule and does no work, the operative reading is the padding, the format's author has shipped it since 1996, and no reader implementing the whole-table packing is known; revisited when an edition or corrigendum states the packing either way, and this tree follows the text then (acting round)

> I agree with option 2: pad each item-run.

Reading: the word follows the search the owner asked for, which is what changed the
recommendation it adopts. The research found that padding **is** expected: Acrobat and pdlin
by qpdf's source comment and its manual ("first tested against linearized files created by
external tools (Acrobat and pdlin)"), qpdf itself on both read and write, and PDFium —
Chrome's PDF engine, whose `ReadPageHintTable` byte-aligns after every item-run — while no
reader implementing the packing this tree had written is known to exist, and pdf.js, which
does not consult the tables at all, is no evidence either way. The word takes the padding as
F.4.1's **operative reading** rather than as a departure: the exception sentence is ambiguous
between a reading that restates the byte-offset attribute rule and does no work, and the
reading the format's author has implemented for twenty-nine years and every reader expects,
and the second is the one a normative sentence should not be denied. The cost is named with
it: a hypothetical reader that packs whole tables only would misread these files, none is
known, and the row and the ADR record that claim where it can decay visibly.
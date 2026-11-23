# Q1023 — Does PDF/A-2u follow A1011, so that preserved annotation marks may stand outside a tagged document's structure tree there too?

Asked by round 2202 (ADR 3509).

**The condition.** A1011 lets a tagged document's annotation marks, kept on an appended page under
`as-shown`, stand outside its structure tree as an artifact (ISO 32000-2 §14.8.2.2.2's last sentence)
"for PDF/A-2b and all PDF/A-4 flavours (keeping the refusal for Level A)", the loss
`tagging-of-preserved-marks`. It does not name PDF/A-2u. ISO 19005-2's Level U adds Unicode mapping
of text to Level B and asks nothing of the logical structure, so the reason Level A keeps the refusal
does not bind it; the marks set no text, so Level U's own requirement is not touched either. Round
2202 built the construction at 2b and part 4 and kept the refusal at 2u, because the answer's words
stop there.

**The question.** May 2u take the loss as 2b does?

**Recommendation**: yes — Level U's addition is about text, and the marks carry none. The
photograph's gate (`fonts::photographed::offered`) stops at 2b and part 4 too, but for its own reason:
a photograph turns text into pixels, which is what Level U forbids; a page of annotation marks turns
nothing into anything.

# Q1011 — May a tagged document's annotation marks, preserved on an appended page under `as-shown`, stand outside its structure tree?

Asked by round 2190 (ADR 3463), for R15 and R16 of `doc/pdfa-blocking-issues.md`, the rows whose
last `as-shown` stops are this one refusal.

**The condition.** Under `as-shown` an annotation the target forbids (a `FileAttachment` whose file
is not itself PDF/A, a subtype part 2 or part 4 does not define) is removed and its appearance
stream is placed on an appended page, so the marks the window showed are kept (ADRs 1014, 2051). In
a document with a structure tree, `tagged::MARKS_IN_A_TAGGED_DOCUMENT` refuses: §14.8.2.2.1 makes the
marks real content, which belongs in the tree, and Table 355 makes a structure element's `/S`
required, but what an annotation's appearance *is* was a fact only its producer held (ADR 1163).
At round 2186's census this refusal is the whole of what stops five documents under `as-shown`
(`PDF-Sample-Document-Fully-Featured-Layout_Redacted`, `embedded_video_avi`,
`embedded_video_quicktime`, `fileAttachment_fileAttachmentAnnotation`,
`annotation-fileattachment`); `as-if-printed` converts all five by removing the annotation.

**The question.** May the converter, under `as-shown` at 2b and every part 4 flavour (Level A keeps
the refusal), leave those preserved marks out of the structure tree — which §14.8.2.2.1's last
sentence then classifies as an artifact — under a loss of its own word, say
`tagging-of-preserved-marks`, recorded per page in the report and in `xmpMM:History`? This is A707's
answer for a photographed page, applied to an appended one. The alternatives are a structure type
this conversion chooses (`Figure`, Table 368's "item of graphical content", fits an attachment's
icon and not a `FreeText`'s text), or keeping the refusal.

**Recommendation**: yes, as A707 — the marks stay exactly as the window drew them, and what is lost
is named.

# 3463 — What the `builds only` rows leave under `as-shown`, each by its sentence

Session 2190. Status: **accepted**. Context: `doc/pdfa-blocking-issues.md` sections 2 and 9 at round
2186's census; ISO 19005-2 and ISO 19005-4 sections 6.3.1, 6.3.3, 6.6.2.1 and 6.9 (paraphrased);
ISO 32000-2 §7.4, §14.8.2.2.1, Table 355; ADRs 1014, 1163, 2490, 2682, 3141, 3461; A694, A707,
A778; Q1011.

After ADR 3461, what the `builds only` rows still stop under `as-if-printed` is `020747` (R03 and
R08: a page this reader does not draw whole, because a bound it sets stopped the drawing, and a
photograph would fix that state) and R01's sources, each refused by its own sentence. What they leave
under `as-shown`, most documents first, and why each is a refusal by name rather than a build:

1. **R16 and R15 — five tagged documents** (`PDF-Sample-Document-Fully-Featured-Layout_Redacted`,
   `embedded_video_avi`, `embedded_video_quicktime`, `fileAttachment_fileAttachmentAnnotation`,
   `annotation-fileattachment`). Each stops only at `tagged::MARKS_IN_A_TAGGED_DOCUMENT`: an annotation
   part 2 or 4 forbids (section 6.3.1, or a file attachment whose file is not itself PDF/A, section
   6.9) is removed and its marks kept on an appended page, and in a tagged file those marks are real
   content that Table 355 asks a structure type of, which only the producer knew. A707 answered the
   same shape for a photographed page; Q1011 asks it for this one. "Office files below 4f" are these
   documents' attachments at part 4's plain flavour: 4f is the target that admits them.
2. **R16 — `poppler-395-0-fuzzed`**: `stand-in-face`, unauthorised by `as-shown` (ADR 2858).
3. **R14 — two documents, each the file's own content.** `241573_ftp`'s object 67 is a metadata
   stream stating no `/Filter` whose bytes begin `78 9C`, deflate data: §7.4 gives a stream's bytes
   the filters its dictionary names, so read as stated the packet is not text, and inflating it would
   be choosing a filter the file does not state. `preserve::NOT_TEXT` stands. `balloon_a1b_jp2k`'s
   packet holds a NUL in its `begin` attribute (byte 17), where ISO 16684-1 puts a byte-order mark;
   no face has a glyph for a C0 control, and the step that leaves that character off the page and
   names it is `as-if-printed`'s (ADRs 2682, 3141, A778), so `as-shown` keeps `preserve::NO_FACE`.
4. **R09 — thirteen documents.** Twelve stop at `unauthorised`: their faces are the machine's or their
   layout is this viewer's, losses `as-shown` does not grant (ADR 2490); that is the profile's answer,
   not a missing build. One, `issue13316_reduced`, is `not-built-yet` at "the machine's face this
   viewer drew a font from has no glyph, by the route a reader of the embedded font takes", and R11
   refuses it first. The row's listed remainders (an empty glyph's width the face's em cannot state,
   a simple subset, a bare CFF program) are no document's stop in round 2186's census.

So R14, R15 and R16 hold no build that frees a document today; Q1011's answer is the one change that
would move five.

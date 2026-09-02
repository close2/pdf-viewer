# 57 — The transform suite: what RFC 0002 still owes after its first three landings

Status: **open**, on the long-lived branch the transform rounds share (`round-867` onward).
Priority: 50-band — the largest items are **blocked on the owner's answers** to RFC 0002 §13,
and the unblocked ones are listed first.
Corpus witnesses: `issue11124.pdf`, `bug1065245.pdf`, `images_1bit_grayscale.pdf` (inline
images, `--native`); `issue21570.pdf` (a JPEG under an `/SMask`); `issue2177.pdf` (a crop box a
quarter of its media box); `attachment.pdf` (an `/EmbeddedFiles` tree to attach into); the
suite's own gate runs over ISO 32000-2's PDF.
Clauses: §7.5.4, §7.5.5, §7.5.7, §7.5.8 and §14.4 on the way out (the serializer, item 2);
§7.6.4.2 Table 22 (item 3).
Code: `crates/pdf-transform/`, ADRs 0800, 0801, 0802, 0803, 0804.

## What is done

RFC 0002 §14's first landing — the seam, the range grammar, the name patterns, the report, the
exit statuses, and `render`, `images`, `attachments` (read) — ADR 0800, session 867. Session 868
(ADR 0801): the CPU-time question answered by measurement and the font cache per rayon split
fixed; the transform gate with RFC §12's perf floor; §8.9.7's inline images and `--native`;
§12.5.6.15's annotations as the third home `attachments` reads. Session 870 (ADR 0802): `images
--no-mask` and the mask beside every native JPEG; `render --page-box` over Table 31's five boxes
and `--no-annotations`; and **`attachments --attach`**, the suite's first writer consumer, on
§7.5.6's incremental update alone — the source's bytes intact, three objects and a rewritten
holder after them, deterministic unless `--date` is given. Session 872 (ADR 0803): the
restriction policy done once in `pdf_model::restriction` — every Table 22 bit named, the
transform's three operations beside the viewer's two, the four levels and an exhaustive verdict
asked once by `decide`, §12.8.2.2's certification read against all five; **`--attach
--to-page`**, §12.5.6.15's annotation on a page with this tree's own icon; and **`--remove`**,
the tree without the entry and the objects marked free by §7.5.4's second mechanism. Session
875 (ADR 0804): **`--format pgm`** for `render` and `images`, §10.4.2.2's grey through the one
statement of the NTSC weights the tree has, over the RGB the interpreter's own conversion
already produced, with the mask beside a netpbm image; the `/Names`-indirect holder fixture,
and the walk's census of every holder shape the corpus has; and **the writer over the corpus**
— `tests/writer_corpus.rs`, every corpus document the suite opens attached into, read back,
the file removed and filed on page 1, on `doc/todo/02` §2's sequence with its refusals counted
by reason. Everything below is what the RFC proposed and no round has taken, in the order the
next round should.

## 1. Small things that need no writer — unblocked

- JPEG output from `render` waits on §13 question 2, the DCT encoder.

## 2. The serializer and the writing verbs — blocked on RFC §13 question 1

`split`, `merge`, `pages` and `optimize` need RFC §10's structure-preserving serializer in
`pdf-syntax`, and that needs the owner to ratify RFC §11.1's redrawn authoring exclusion in
`CLAUDE.md`. Nothing here starts before that sentence.

## 3. One thing still without its dependency

- `pdf_transform::Operation` moved into `pdf_model::restriction` in session 872 (ADR 0803);
  nothing of the policy is in this crate any more but the words on stderr.
- `--password-prompt`: an interactive prompt that suppresses echo needs a terminal-mode
  dependency (`doc/stack.md` decides), or a host that owns a terminal. `--password-fd` is the
  scripted route and is what exists.

## 4. The confinement tranche — RFC §13 question 3, defaulted to in-process

ADR 0800 §6 states the cost. The worker split is a transport change on the `pdf-view-worker`
pattern — plan in, report out, sources and sinks as descriptors the broker opened — and the seam
was written so that it is one; `viewer-confined` is the precedent. Taken when the verbs settle,
or earlier if the owner requires it before the first release. `--attach` adds one thing to the
plan that crosses: the payload's bytes, which are a descriptor like a source's.

## 5. What the gates do not see

The transform gate's floor is a wall-clock number over 24 threads, and ADR 0801 §2's defect —
a cost that shows only where a CPU-second is worth what it was, at two or four threads — would
pass it. The instrument for that class is the thread curve in ADR 0801, taken by hand with
`RAYON_NUM_THREADS`; a round that touches `render`'s parallel shape re-takes it, and ADR 0804
has the one taken in session 875. The writer's walk judges every update with this tree's own
reader and nothing else (trap 8, said in its file): `tests/writer.rs` holds `qpdf --check` over
the committed fixtures, and a corpus-wide *foreign* readback of the transform's updates —
poppler and mupdf listing the file back, the shape `save_round_trip.rs` has for the viewer's
edits — is the instrument ADR 0334 priced and nobody has taken for this writer.

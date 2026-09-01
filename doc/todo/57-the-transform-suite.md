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
Code: `crates/pdf-transform/`, ADRs 0800, 0801, 0802.

## What is done

RFC 0002 §14's first landing — the seam, the range grammar, the name patterns, the report, the
exit statuses, and `render`, `images`, `attachments` (read) — ADR 0800, session 867. Session 868
(ADR 0801): the CPU-time question answered by measurement and the font cache per rayon split
fixed; the transform gate with RFC §12's perf floor; §8.9.7's inline images and `--native`;
§12.5.6.15's annotations as the third home `attachments` reads. Session 870 (ADR 0802): `images
--no-mask` and the mask beside every native JPEG; `render --page-box` over Table 31's five boxes
and `--no-annotations`; and **`attachments --attach`**, the suite's first writer consumer, on
§7.5.6's incremental update alone — the source's bytes intact, three objects and a rewritten
holder after them, deterministic unless `--date` is given. Everything below is what the RFC
proposed and no round has taken, in the order the next round should.

## 1. Small things that need no writer — unblocked

- `--format pgm` waits on a stated grey conversion; JPEG output on §13 question 2.
- `attachments --attach` has no fixture for the holder case where the catalog's `/Names`
  dictionary is indirect and the tree inside it is direct; the other two cases are tested. A
  corpus census for that shape, or a fixture, closes it.
- `--attach --to-page N` (RFC §6.6's example): a §12.5.6.15 file attachment annotation on a page
  rather than a name-tree entry. The same three objects plus an annotation dictionary and the
  page's `/Annots` rewritten; the icon's artwork is the same documented choice the viewer
  already makes.

## 2. The serializer and the writing verbs — blocked on RFC §13 question 1

`split`, `merge`, `pages` and `optimize` need RFC §10's structure-preserving serializer in
`pdf-syntax`, and that needs the owner to ratify RFC §11.1's redrawn authoring exclusion in
`CLAUDE.md`. Nothing here starts before that sentence.

## 3. Two things in the wrong crate, deferred for the same reason

- `pdf_transform::Operation` — `Print`, `Extract` and now `Modify` over Table 22's bits 3, 5
  and 4 — belongs in `pdf_model::restriction::Operation` beside `FillInForm` and `Annotate`, so
  that one module reads all six restriction sources for every operation this tree performs.
  First-row change: the natural content of a round that runs the whole gate sequence anyway.
- `--password-prompt`: an interactive prompt that suppresses echo needs a terminal-mode
  dependency (`doc/stack.md` decides), or a host that owns a terminal. `--password-fd` is the
  scripted route and is what exists.

## 4. The confinement tranche — RFC §13 question 3, defaulted to in-process

ADR 0800 §6 states the cost. The worker split is a transport change on the `pdf-view-worker`
pattern — plan in, report out, sources and sinks as descriptors the broker opened — and the seam
was written so that it is one; `viewer-confined` is the precedent. Taken when the verbs settle,
or earlier if the owner requires it before the first release. `--attach` adds one thing to the
plan that crosses: the payload's bytes, which are a descriptor like a source's.

## 5. What the gate does not see

The transform gate's floor is a wall-clock number over 24 threads, and ADR 0801 §2's defect —
a cost that shows only where a CPU-second is worth what it was, at two or four threads — would
pass it. The instrument for that class is the thread curve in ADR 0801, taken by hand with
`RAYON_NUM_THREADS`; a round that touches `render`'s parallel shape re-takes it. The gate also
holds no output of `--attach`: `tests/writer.rs` does, on two committed and corpus documents,
and a corpus-wide attach-and-read-back walk is the writer's equivalent of the render corpus gate
(RFC §9), owed when the serializer lands and worth taking before it.

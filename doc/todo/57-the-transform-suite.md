# 57 — The transform suite: what RFC 0002 still owes after its first landing

Status: **open**, on the long-lived branch the transform rounds share (`round-867` onward).
Priority: 50-band — the largest items are **blocked on the owner's answers** to RFC 0002 §13,
and the unblocked ones are listed first.
Corpus witnesses: `issue11124.pdf`, `bug1065245.pdf`, `images_1bit_grayscale.pdf` (inline
images, `--native`); the suite's own gate runs over ISO 32000-2's PDF.
Clauses: §7.5.4, §7.5.5, §7.5.7, §7.5.8 and §14.4 on the way out (the serializer, item 2);
§7.7.3 and §6.3.2.2 (item 1); §7.6.4.2 Table 22 (item 3).
Code: `crates/pdf-transform/`, ADRs 0800, 0801.

## What is done

RFC 0002 §14's first landing — the seam, the range grammar, the name patterns, the report, the
exit statuses, and `render`, `images`, `attachments` (read) — ADR 0800, session 867. Then, in
session 868 (ADR 0801): the CPU-time question answered by measurement and the one real defect in
it fixed (a font cache per rayon split, now one per run); the transform gate with RFC §12's perf
floor, on `doc/todo/02` §2's sequence; §8.9.7's inline images and `--native` in `images`; and
§12.5.6.15's annotations as the third home `attachments` reads. Everything below is what the RFC
proposed and no round has taken, in the order the next round should.

## 1. `images` and `render`, the flags that need no writer — unblocked, and first

- **`--no-mask`**, keeping a soft mask as `img-%d.mask.png` beside the image (RFC §6.3). Under
  `--native` a JPEG's `/SMask` is dropped today and the usage text says so; this flag is what
  would keep it.
- **`--page-box`** (§7.7.3's boxes, crop by default) and **`--no-annotations`** (§6.3.2.2's
  obligation, opted out of) are RFC §6.4's; `interpret` offers no knob for either, so adding one
  is a first-row change with the whole gate sequence behind it. `--format pgm` waits on a stated
  grey conversion; JPEG output on §13 question 2.

## 2. The serializer and the writing verbs — blocked on RFC §13 question 1

`split`, `merge`, `pages`, `optimize` and `attachments --attach` all need RFC §10's
structure-preserving serializer in `pdf-syntax`, and that needs the owner to ratify RFC §11.1's
redrawn authoring exclusion in `CLAUDE.md`. Nothing here starts before that sentence.
`attachments --attach` is the one verb §7.5.6's incremental writer could serve today, and would
be the smallest first consumer of any writer.

## 3. Two things in the wrong crate, deferred for the same reason

- `pdf_transform::Operation` — `Print` and `Extract` over Table 22's bits 3 and 5 — belongs in
  `pdf_model::restriction::Operation` beside `FillInForm` and `Annotate`, so that one module
  reads all six restriction sources for every operation this tree performs. First-row change.
- `--password-prompt`: an interactive prompt that suppresses echo needs a terminal-mode
  dependency (`doc/stack.md` decides), or a host that owns a terminal. `--password-fd` is the
  scripted route and is what exists.

## 4. The confinement tranche — RFC §13 question 3, defaulted to in-process

ADR 0800 §6 states the cost. The worker split is a transport change on the `pdf-view-worker`
pattern — plan in, report out, sources and sinks as descriptors the broker opened — and the seam
was written so that it is one; `viewer-confined` is the precedent. Taken when the verbs settle,
or earlier if the owner requires it before the first release.

## 5. What the gate does not see

The transform gate's floor is a wall-clock number over 24 threads, and ADR 0801 §2's defect —
a cost that shows only where a CPU-second is worth what it was, at two or four threads — would
pass it. The instrument for that class is the thread curve in ADR 0801, taken by hand with
`RAYON_NUM_THREADS`; a round that touches `render`'s parallel shape re-takes it.

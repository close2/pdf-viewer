# 2537 — A file attachment annotation whose file went to pages becomes a link to them

Status: accepted — session 1851, the first archive slot of batch 126. Context: ADR 2536 (the two
embedded-file rows answered per file, a text file appended as pages), ADR 2490 and
`doc/pdfa-blocking-issues.md`'s R15 entry ("the annotation or GoTo pointing at the first appended
page"), ADR 1014 (a page of the document's own content), ADR 2067 section 4.

## The problem

At PDF/A-2 ISO 19005-2 section 6.3.1 admits a `FileAttachment` annotation, so no removal takes it
off its page, and section 6.8 does not admit the non-PDF/A file its `/FS` names. ISO 32000-2
§12.5.6.15's Table 184 makes `/FS` required, so the annotation cannot stay one and name nothing.
Under ADR 2536 alone the embedded-file row therefore refused every such document at 2b: the file
could go to pages, but the annotation would still reach it.

## Decision

1. **Where the row answers the file with appended pages, the annotation becomes a §12.5.6.5 link.**
   `/Subtype` is written `/Link`, `/FS` and `/Name` (Table 184's icon name, meaningful only for a
   file attachment annotation) go, and `/Dest` is §12.3.2.2's `[page /Fit]` on the first page its
   file was laid out on. `/Rect`, `/F`, `/Contents` and the producer's `/AP` stay, so what the
   page shows is unchanged and no mark is composed; a reader who clicks the icon lands on the
   file's text. Where the annotation states neither `/Border` nor `/BS`, `/Border [0 0 0]` is
   written: Table 166's default is a one-unit solid border, a reader may draw a link's border
   over its appearance (poppler did, on `annotation-fileattachment.pdf`, which is evidence about
   readers and not a reading), and the producer's appearance is the whole of what it drew.
2. **Where the file was removed instead** (`on-failure = "discard"`), the annotation becomes a link
   with no destination: its appearance stays on the page, and it names no file. The outcome per
   file names the annotation and where it now points, in the report, its JSON (`annotations`) and
   `xmpMM:History`.
3. **An embedded `GoTo` action still refuses**: §12.6.4.4 names a destination *inside* the file,
   which a page of its text does not hold. So does a row with no appended pages (`discard` alone),
   which is the refusal ADR 2536 kept.

## Alternatives not taken

- Removing the annotation: a loss over the producer's marks that nothing in the clause asks for.
- Keeping `FileAttachment` and pointing `/FS` at a fresh PDF/A file: that file would be composed by
  this program, which is authoring.

## What it costs

The annotation's markup keys (`/T`, `/Popup`, `/CreationDate`) stay on a subtype that does not
define them. No clause of ISO 32000-2 gives them a meaning on a link and neither part of ISO
19005 forbids them there, so the producer's values are kept rather than judged.

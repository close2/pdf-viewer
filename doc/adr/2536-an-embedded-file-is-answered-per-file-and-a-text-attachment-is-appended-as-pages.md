# 2536 — An embedded file is answered per file, and a text attachment is appended as pages

Status: accepted — session 1851, the first archive slot of batch 126. Context: the owner's words of
2026-10-27 (a company archives every PDF it receives; a profile may accept a loss and append a page
stating what was removed), `doc/pdf-a-mitigations.md` section 14 point 10 ("the configuration is
per site and the outcome is per object", owed whatever Q706 answers), `doc/pdfa-remedies.md`
section 4.1, ADRs 1014 (a page of the document's own content), 2067 section 4 (a removed file
attachment annotation keeps its file), 2490 (R15 and the loss `attachment-as-pages`), A57 (one
alternative, never a chain). Q706 is open, so nothing here is RFC 0014's `try` list or its
statement page.

## Decision

1. **The outcome of the two embedded-file rows is per file.** ISO 19005-2 section 6.8 and ISO
   19005-4 section 6.9 are answered by one row, and the report, its JSON (`attachments`) and the
   output's `xmpMM:History` name every file the requirement failed at with its own answer:
   laid out on pages (loss `attachment-as-pages`) or removed (loss `embedded-file`), each with
   its object, its §7.11.3 name (`/UF` before `/F`, Table 43) and, for a removal, why.
2. **`remedy = "preserve"`, `placement = "append"` is built at both rows.** A file whose decoded
   §7.11.4 stream is UTF-8 text with no control character but tab, line feed, form feed and
   carriage return is laid out as text on appended pages by `super::preserve`'s composer, which
   is ADR 1014's permission exactly: the characters are the file's own. HTML and XML are set as
   written, not rendered.
3. **`on-failure = "discard"` is carried out at these two rows, per file**, which A57 permits
   (one alternative) and which the composer's other rows still refuse. A file this reader does not
   lay out — not text, not decodable, past 256 KiB, past the sixteenth, or on a page the composer
   refuses (no output intent, no face) — is removed and named with that reason; `stop`, the
   default, refuses the conversion instead. The two bounds are checked before any file is
   decoded or laid out.
4. **A file attachment annotation section 6.3.1's removal takes off a page no longer reaches its
   file.** The refusal "named by a file attachment annotation's FS entry" was raised at PDF/A-4
   for an annotation the same conversion removes, which is a defect of the reading, not a fact of
   the output.
5. **A file an embedded-file row answers is not filed again by the annotation row.** ADR 2067
   section 4's `keep-attachment = true` files a removed annotation's file in the name tree; where
   section 6.8 or 6.9 failed at that same specification, the embedded-file row decides its fate,
   and filing it again put back the file that row took out (the output then failed 6.9).
6. **Both shipped profiles carry R15**: the two rows become `preserve` by page with `on-failure =
   "discard"`; the 4f row still keeps the file unchanged (Annex A).

## What this does not decide

- A file attachment annotation PDF/A-2 admits that still names its file: the file is still
  reached, and the conversion is refused as before. Pointing the annotation at the appended
  page changes its subtype and is its own decision.
- The multimedia clip kept as an attachment at 4f and 4e (`doc/pdfa-remedies.md` 4.2).

## The ordered list and the statement page (`A706` items 1 and 2)

The owner adopted RFC 0014 sections 3 and 4 on 2026-10-28, and the third session of the round
built them on this ADR's per-file outcome.

- **`try = [...]` is resolved when the file is read** (`archive/config/steps.rs`): each step is
  made into the row it would be alone and asked `check_rows`'s questions for the target; a step
  refused by name there is passed over and listed (`Configuration::skipped`), a list with no step
  left is an error at read time, and a site the target does not bind is never resolved, so one
  line serves six targets. `remedy` with `on-failure` is the same two-step list, and an unknown
  `on-failure` word is now an error rather than a silent `stop`.
- **Two steps per object, because that is what the executor holds**: a keeping step, then `stop`,
  `discard` or `append-statement` for each object it could not keep, carried out where the outcome
  is per object — `preserve:append` at the two embedded-file rows. A second step anywhere else, a
  keeping step in second place, and a step after one that never hands an object on are refused by
  name. A third step needs an executor that hands an object from one keeping step to the next;
  that is RFC 0014 section 5's widened `derive`, and this does not build it.
- **`append-statement` is `discard` with a page** (`archive/statement.rs`), built at the two
  embedded-file rows only, because the page lists objects and only those rows report one outcome
  per object. Each listed file carries the requirement and its clause for the target, the loss word
  with one fixed line, the specification's object number, its `/UF` or `/F` name, decoded length,
  `/Subtype` media type and SHA-256, the step tried before with the sentence it failed on, and the
  steps the target did not admit. A name's characters outside printable ASCII are written
  `<U+XXXX>`. The page is ADR 1014's appended page set by `preserve::compose` — labels extended,
  structure entries at Level A, the face reported — and a page that cannot be composed refuses the
  site rather than letting the loss go unstated.
- **Bounds, each refused by name**: 32 objects (`MOST_STATED_OBJECTS`, before any stream is
  decoded or hashed), 64 KiB of text (`MOST_STATEMENT_BYTES`, before layout) and 16 pages
  (`MOST_STATEMENT_PAGES`). The object bound is the validator's own: `sites::objects_named` reads
  the places a finding lists, and a finding lists 32, so a document with more failing attachments
  is answered for the 32 it names — a limit of the site, not of the statement, named for a later
  round.

## A PDF or an image drawn as pages (R15)

A `preserve` row at either embedded-file site draws, beside the text files it sets, an attachment
that is a PDF (`%PDF-` at its first byte, §7.5.2) or an image this reader decodes: a PNG, or a JPEG
whose frame header names the baseline, extended sequential or progressive Huffman process at eight
bits with one or three components. Both end in the shape a derived file's pages already take
(`archive/drawn.rs`), so that one path composes, reports and links every drawn page:

- **a PDF is a derived file whose program is the identity.** It is converted by
  `derived::convert` to the outer target under the budget the derivations share, and its pages
  are imported by `derived::import_pages` — each a form XObject in a wrapper drawing its
  annotations' normal appearances. **One level**: the plan it is converted under has every
  preservation's `draws_documents` false, so a PDF inside it is that run's to set or remove and is
  never drawn — the depth bounded by the plan's shape rather than a counter, as ADR 2539's is;
- **an image is one image XObject on one page at one unit per sample** — a choice: the standard
  states no size for an attachment's page, and `pHYs` or a JFIF density speaks to a printer. Scaled
  uniformly into ISO 19005-2 section 6.1.13's 3 to 14 400 units (held at part 4 too), refused where
  no scale fits; a PNG's alpha becomes §11.6.5.2's soft mask, a JPEG's bytes are written unchanged
  under `DCTDecode`, and a four-component JPEG is refused because its polarity is APP14's
  convention, not the file's word. The pixel count is held to `Budget::max_pixels` before a sample
  is decoded.

**The output intent agrees by ADR 2539's construction, not a new one**: a drawn file names the device
families its pages paint in (a converted PDF's by its own intent, an image's by its file), and the
preparation asks for the archive's intent exactly as a page painting in them would, so the archive's
one intent licenses every drawn page. What `attachment-as-pages` loses for an image is its colour
chunks (`iCCP`, `gAMA`, `sRGB`): its samples are read under the archive's intent. Each file stays its
own answer: one that does not convert, decode or import takes `on-failure` with its own sentence,
and a tagged document's drawn file is refused per file as `tagged::MARKS_IN_A_TAGGED_DOCUMENT` says.
`tests/archive/drawn_attachments.rs` draws a plain PDF, a PNG and a JPEG at 2b and 4 (four pages
written for one, each output passing veraPDF, a planted copy without its intent failing three
rules) and names the plant, a PNG cut after its signature, as removed. **A drawn file whose page
cannot be composed takes `on-failure` too**: where the composition of every kept page fails and the
row says `discard`, the text and drawn files are removed with the composition's sentence and the
rest is composed again — two forty-page PDFs, past the 64 appended pages, are the test's case.

## A media clip kept as a file at PDF/A-4f and 4e

`keep-media = "attach"` on a `preserve` row at either section 6.3.1 subtype requirement keeps,
beside the poster the row keeps, the clip a removed Screen or Movie annotation played: the file
specification is filed in the `/EmbeddedFiles` tree as the producer wrote it, the way ADR 2067
section 4 files a `FileAttachment`'s. Only 4f and 4e, whose Annexes A.2 and B.4 admit a file of any
type; elsewhere, or on another row or remedy, the key is an error at read time. The walk is the
dictionaries ISO 32000-2 states — a Screen's §12.6.4.14 rendition action, its §13.2.3.2 media
rendition, the §13.2.4 clip (a section followed once to its data), the data's `/D`; a Movie's §13.4
`/F` — and files only a specification holding an `/EF`: a clip stored outside the file is not the
file's to keep. A filed clip answers Annex A.2's required tree. A Sound annotation's sound is a
stream of samples rather than a file and is not kept; a RichMedia annotation is admitted at 4e and
its assets at 4f are not walked. `tests/archive/media_clips.rs` keeps an MP4 at 4f and 4e (veraPDF
compliant at both, a copy with the tree's key renamed failing one rule) and files no external movie.

## What it moved

The census before and after, against HEAD, without the CJK pack: under `as-if-printed` at PDF/A-4,
`annotation-fileattachment.pdf` (its `Test.txt` laid out on an appended page, one page in the source
and two written) and `fileAttachment_fileAttachmentAnnotation.pdf` (its Lotus `PF.WK1` removed and
named) are written and pass veraPDF at 4, and nothing else moved. The other three R15 documents stay
refused: the Word file's document is tagged and its annotation's marks need a structure type, and
`issue17056.pdf` and `issue18030.pdf` at 4 wait on `associated-file-media-type`.

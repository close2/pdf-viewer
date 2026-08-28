# RFC 0001 — The survey: what PDF tools provide, what users ask for, and where the gaps are

Status: **proposed**
Round: 784, commissioned by the owner
Decides nothing. This is the *collect* half of a three-round arc: RFC 0002 (transform
suite, round 785) and RFCs 0003–0005 (file-system faces, print, and text editing, round
786) make the design arguments. This document gathers the evidence they argue from.

**Two registers, kept apart.** Everything below is evidence about *demand and convention*:
what products sell, what users file, what interfaces the field converged on. None of it is
evidence about what a page means — rendering correctness stays governed by the
specification alone (`CLAUDE.md` principle 5). A survey can say "everyone ships a merge
tool"; it cannot say what a merged outline is, and where a proposed feature has normative
content the clause is cited beside it.

**The owner's framing directive, which governs this whole series**: the survey and the
RFCs after it are *not* bound by the project's current rules. Where a standing rule is
relevant — the authoring exclusion, the immutability of `pdf_syntax::Document` — it is
named below as a **current restriction with its original rationale**, and the estimates
are for the **unconstrained design**, with the rule's cost noted as a second data point
where it would have mattered. The owner amends rules by argument, and is signalling
willingness to hear one; a survey that graded everything "hard because forbidden" would
have hidden the argument they asked for.

All URLs fetched 2026-08. Product feature sets move; the date is part of each claim.

---

## 1. Commercial editors — the ceiling of user expectation

### Adobe Acrobat (Standard / Pro)

Adobe's own comparison page (https://www.adobe.com/acrobat/pricing/compare-versions.html)
splits the line as: **Standard** covers everyday work — edit text and images, convert
to/from Office formats, organize pages (reorder, rotate, delete, insert), protect with
passwords, fill and sign, collect routine e-signatures. **Pro** adds the trust- and
scan-shaped work: OCR of scanned documents, redaction (permanent content removal),
comparing two versions with a change summary, preflight for print production, PDF/A and
accessibility checking, and heavier e-signature workflows. Secondary summaries agree on
the split (https://www.pcworld.com/article/397929/adobe-acrobat-standard-dc-vs-adobe-acrobat-pro-dc.html).

The read for us: Adobe's own market segmentation puts *page organisation, text editing,
conversion and protection* in the "everyone needs this" tier, and *OCR, redaction,
compare, preflight* in the professional tier. That ordering recurs in every product below.

### PDF-XChange Editor

Product page (https://www.pdf-xchange.com/product/pdf-xchange-editor). Windows-only, and
notable for how much is free: the vendor states roughly two hundred and fifty features
free against roughly a hundred and fifty licensed. Free tier: viewing, twenty-plus
annotation types, page cropping/rotation/extraction/merging/reordering, form filling,
export to images, bookmarks management, headers/footers, watermarks, barcodes. Licensed:
enhanced OCR, form *creation*, document comparison, digital signing with timestamping.
Its free-tier breadth is a useful signal of what Windows users treat as table stakes in a
*viewer* — including a lot of light editing.

### Foxit PDF Editor

Product page (https://www.foxit.com/pdf-editor/). Edit text/images/pages, merge and
split, organize pages, OCR to searchable PDF, compare versions, convert to/from Office,
form create/fill/sign, password protection and encryption, redaction (with an AI-assisted
tier), shared review and annotation. Editions differ mostly by platform and cloud
services, not by which document operations exist.

### Nitro PDF Pro

Product pages (https://www.gonitro.com/pdf-editor, https://www.gonitro.com/ocr). Edit
text and images, merge and split (split via page selection → new document), rotate, OCR
to editable/searchable text, redaction including AI-assisted PII removal, white-out,
digital signing plus signature-validity checking with certificate chain display,
convert to/from Office and images, headers/footers/page numbers/bookmarks, password
protection, fillable form creation.

### The common commercial core

Every one of the four ships: text+image editing, page organisation (merge, split,
reorder, rotate, delete), OCR, redaction, conversion to/from Office formats, form
fill+create, password protection, digital signing, watermarks/headers/footers, and
export to images. Compare-two-documents is in all four at some tier. That is the ceiling
of expectation a general-purpose "PDF program" is measured against.

---

## 2. Stirling-PDF — what ordinary users actually reach for

The owner's "quite successful recent open source PDF web service" is almost certainly
**Stirling-PDF** (https://github.com/Stirling-Tools/Stirling-PDF) — self-hosted web
service plus desktop app, about ninety-one thousand GitHub stars as of 2026-08, "55+
tools", REST API for nearly all of them, no-code pipelines. Nothing else recent matches
the description's shape or success. (Runners-up considered: none close in adoption.)

Its tool list (https://docs.stirlingpdf.com/functionality/) is the best available proxy
for what ordinary users want from a PDF utility, because every tool exists due to demand
on a self-hosted service rather than due to an enterprise checklist:

- **Its own "most popular" list**: text editor, multi-tool (visual page surgery),
  read & annotate, **merge, convert, compress, OCR, compare, redact**.
- **Page operations**: split, merge, rotate, extract pages, reorganize (arbitrary
  reorder), add page numbers, remove pages / remove blank pages.
- **Security**: add/remove password, change permissions, handwritten signature,
  certificate signing, validate signature, watermark, **sanitize** (strip JavaScript,
  embedded files, metadata), redact.
- **Convert**: to PDF from Word/Excel/PowerPoint/images/HTML/Markdown/email; from PDF
  to Word/PowerPoint/text/images/CSV/HTML/XML/PDF-A.
- **Content & editing**: add/extract images, stamp, edit metadata, remove annotations,
  replace colors, get PDF info.
- **Advanced**: overlay PDFs, booklet imposition, multi-page layout (n-up), scale
  pages, auto-rename (from content), show JavaScript, scanner effect.

The read: the centre of gravity is **document transforms** — merge, split, compress,
convert, OCR — exactly the class today's scope excludes. A web service has no "viewer"
advantage at all; people run Stirling for the operations, which is the demand signal the
owner asked this survey to weigh.

---

## 3. Open-source CLI utilities — operations and interface conventions

These are the tools whose *conventions* a `pdf-utils` CLI would be measured against.

| tool | operations | interface convention worth learning |
|---|---|---|
| **qpdf** (https://qpdf.readthedocs.io/en/stable/overview.html) | structural, content-preserving transforms: split/merge by page ranges, rotation, overlay/underlay, encryption add/remove, linearization, object-stream repacking, repair, JSON serialization of structure | describes itself as knowing nothing of content-stream semantics — the clean *structural/semantic* split. And **QPDFJob** (https://qpdf.readthedocs.io/en/stable/qpdf-job.html): CLI flags, a JSON job file and the C++ API are one synchronized vocabulary (`--some-option` ↔ `someOption` ↔ `config()->someOption()`), so automation never diverges from the CLI |
| **pdftk** (https://gitlab.com/pdftk-java/pdftk) | cat (ranges, rotation suffixes), shuffle, burst, fill_form from FDF, stamp/background, attach/unpack files, dump_data/update_info | *handles* (`A=in1.pdf B=in2.pdf cat A1-7 B8-end`) — a tiny algebra for multi-input page assembly that users still miss everywhere else |
| **mutool** (https://mupdf.com/docs/mutool.html) | draw (rasterize), convert, clean (garbage-collect/decompress/repair), merge, poster, extract (images+fonts), show, create, sign, trim | one binary, subcommand per verb — the git-shaped CLI; `clean`'s decompress mode as the debugging view of a file |
| **poppler-utils** (https://poppler.freedesktop.org/) | pdftoppm/pdftocairo (rasterize), pdfimages (extract images *as stored*), pdftotext, pdfseparate/pdfunite, pdfdetach, pdffonts, pdfinfo, pdfsig | one tool, one job, pipe-friendly; `pdfimages -list` prints what it *would* extract — the dry-run-as-report pattern; `pdftotext -layout` as the de-facto extraction reference (already this tree's oracle) |
| **ocrmypdf** (https://ocrmypdf.readthedocs.io/en/latest/introduction.html) | adds a text layer to scanned PDFs via Tesseract; deskew, rotate, optimize; PDF/A-2b output by default | strict Unix filter, `ocrmypdf in.pdf out.pdf`, **never modifies the input**; rasterizes only for recognition and grafts the text layer back into the *original* pages rather than re-rendering them — content preservation as a design principle |
| **ghostscript pdfwrite** (https://ghostscript.readthedocs.io/en/latest/VectorDevices.html) | re-distill to PDF: compression/downsampling presets (`-dPDFSETTINGS=/screen /ebook /printer /prepress`), PDF/A and PDF/X output | the *preset* interface for compression — four named quality points instead of thirty knobs; also the cautionary tale, since re-distilling re-interprets content and loses what it does not understand |

Convention synthesis for RFC 0002: one binary with subcommands (mutool), a JSON job form
kept mechanically in sync with the flags (qpdf), inputs never modified in place
(ocrmypdf), page-range algebra with multi-input handles (pdftk), named quality presets
for compression (ghostscript), and a dry-run report mode (pdfimages -list). This tree
already has one CLI citizen to be consistent with: `tools/pdf-retrieve`, JSON on stdout
(ADR 0257).

---

## 4. Tracker mining — what users of open-source viewers actually file

Specific, citable issues; long-standing ones weighted over recent ones.

- **Okular** (KDE bugzilla): sanitize + redact requested outright (bug 452403); insert
  and replace-text annotation, i.e. light text editing (bug 332835, open since 2014);
  typewriter annotation (bug 353401); copy/paste annotations (bug 440405); highlighter
  auto-filling its popup with the highlighted text (bug 321992); session restore
  (bug 364680); search across all open files (bug 426133). Print is a recurring sore
  spot rather than one bug: force-rasterization is the default workaround path and
  users report shifted/cut-off and low-quality output
  (https://forums.opensuse.org/t/okular-fit-to-printable-area-is-there-an-option-somewhere/118839),
  and print *preview* has broken outright on some stacks
  (https://bbs.archlinux.org/viewtopic.php?id=259287).
- **Evince** (GNOME gitlab): application-wide night mode — the current one is
  per-document (issue 1256); tab support (issue 1331); horizontal continuous view
  (issue 1018); annotation tooltips (issue 882); scrolling lag on heavily annotated
  pages (issue 1201). Print quality: low-resolution rasterized output is a
  long-standing complaint (https://bugs.launchpad.net/bugs/984082).
- **pdf.js** (github.com/mozilla/pdf.js): the heavily-requested line is annotation
  *editing* — highlight/comment support (issue 14975), editing annotations that already
  exist in the file, including other producers' (issue 15403), editable popup/reply
  annotations (issue 18982), and honouring annotations other tools wrote (issue 9552).
  pdf.js has been adding editor modes (freetext, ink, highlight, stamp) in response —
  the demand was strong enough to move a *browser* viewer into editing.
- **SumatraPDF** (github.com/sumatrapdfreader/sumatrapdf): dark mode (issue 628, one of
  the oldest and most reacted; theming follow-ups 743, 5646); annotation editing gaps
  (issue 2485). Sumatra's answer — it grew annotation editing and themes — is again the
  viewer-grows-editing pattern.
- **Zathura** (git.pwmt.org/pwmt/zathura): annotations support is the standing
  wishlist item (issue 7), unresolved for years for architectural reasons.
- **MuPDF**: tracker mining is thin (Artifex runs bugs.ghostscript.com plus a bounty
  programme); the visible pattern is that mupdf-gl *added* full annotation
  create/edit/delete with appearance-stream synthesis — the same direction of travel.

The pattern across every tracker: **users push viewers toward light editing** —
annotations first, then text — plus **comfort features** (night mode, tabs, session
restore) and **printing that does not degrade the page**. Notably, almost nobody files
"render this clause correctly" — correctness is assumed; features are what get filed.

**The owner's interests against this evidence**: print + preview lands on demonstrated
pain (both major Linux viewers degrade output; preview breaks); basic text editing
without reflow matches the single longest-standing Okular annotation wish and the
commercial table stakes; compression, CLI parity and the transforms match Stirling's
most-popular list. KIO and FUSE appear in no tracker mined here — they are
owner-originated differentiators, not demand-backed items, and the survey says so
honestly; their value argument is integration (section 5), not vote counts.

---

## 5. KDE precedent — KIO workers, and what a kio_pdf could model itself on

Prior art is real and old: **kio_archive** in kio-extras exposes `tar:/`, `zip:/` (and
more) so every KDE application browses an archive as a directory and extracts by
copying (https://invent.kde.org/network/kio-extras; overviews:
https://www.linux.com/news/master-kio-slaves/,
https://en.wikipedia.org/wiki/KIO). The KHelpCenter "KIOWorkers" section documents the
installed protocols. A `pdf:/` worker exposing pages, images, attachments and text as
virtual files is the same shape as `tar:/` — list, stat, get — with one difference worth
designing for: a PDF's "entries" are *derived* (a rendered page, an extracted image)
rather than stored byte ranges, so the worker needs a renderer behind it, which is
exactly what `viewer-ffi` / `pdf-retrieve` already are.

**KioFuse bridges KIO to a FUSE mount** (https://techbase.kde.org/Projects/KioFuse) —
KDE ships the KIO→FUSE direction as an existing component, so a kio_pdf would get a
POSIX-file view nearly free on KDE systems, and a native FUSE plugin (Rust `fuser`
crate) covers non-KDE systems from the same underlying abstraction. That ordering — one
abstraction, two thin faces — is RFC 0003's design question, not this document's.

**What Okular integrates with KDE that we would be measured against**: an embeddable
KParts component (`okularpart`) that Kile, KDevelop and others host
(https://github.com/KDE/okular); thumbnails in Dolphin/Gwenview via
kdegraphics-thumbnailers (https://apps.kde.org/kdegraphics_thumbnailers/); MIME-type
association and the standard print dialog. For this tree the analogous surface is: a
thumbnailer (we already render miniatures for the thumbnail panel), a file-preview
component, and `pdf:/`. Arch packaging note from `doc/environment.md`: KF6 packages
carry no `kf6-` prefix (`kio`, `kconfig`, `ki18n`).

---

## 6. Where this tree stands today

From `doc/state-of-play.md` and the crate map, not from memory. What exists: three
rasterisers behind one display list; full decode of every §7.6 encryption revision *in
both directions* (saving into encrypted documents works); text selection and clipboard
in logical content order; whole-document search; form fill with undo/redo and save;
annotation *adding* (four markups plus free text) and retyping of free-text annotations;
incremental-update save (§7.5.6) — the one form of writing currently permitted;
signature integrity *and* authenticity verification (all Table 260 algorithm families);
embedded-file extraction; six sidebar panels; three windows plus a confined host; a C
ABI; and `tools/pdf-retrieve`, JSON over outline/text/annotations. What does not exist:
any whole-file writer, any raster/extract CLI, print in any form, OCR, redaction,
compression, page surgery, KIO/FUSE, night mode, tabs.

Two current restrictions matter for grading, and the owner has said not to let them
constrain the estimates:

- **The authoring exclusion** (`CLAUDE.md`): no PDF creation from nothing, and no
  clause whose requirements fall on a generator (linearisation, object-stream packing,
  optimisation). Rationale: a viewer's scope discipline. Already amended once, by
  argument, to admit incremental-update writing.
- **`pdf_syntax::Document` is immutable.** Rationale: `interpret` stays a pure function
  of the bytes, which the oracle's cross-renderer comparison rests on; an edit is a log
  beside the document (the `view.rs` pattern). Note for RFC 0002: this rule constrains
  *mutation in place*, and a transform engine that reads an immutable `Document` and
  emits a *new* file would leave the oracle's premise untouched — full in-place
  mutability is one design option, a build-a-new-document writer is another; the
  estimates below assume whichever the design round prefers, i.e. the rule is treated
  as liftable per the owner's directive, and the purity argument is recorded here as
  the cost to weigh rather than a veto.

---

## 7. The gap matrix

Columns: **A** Acrobat (S=Standard tier, P=Pro tier), **X** PDF-XChange (F=free tier,
L=licensed), **F** Foxit, **N** Nitro, **St** Stirling-PDF, **CLI** the qpdf-class
tools of section 3 (which one, where it matters), **us** today. **Estimate** is
easy/moderate/hard **on this architecture, unconstrained by current rules**, with the
one-sentence why. ✓ has it, — lacks it, ~ partial.

| feature | A | X | F | N | St | CLI | us | estimate — why |
|---|---|---|---|---|---|---|---|---|
| split | S | F | ✓ | ✓ | ✓ | qpdf, pdfseparate, pdftk, mutool | — | **easy** once the writer exists: emit chosen pages plus their transitive objects, both already fully readable |
| merge | S | F | ✓ | ✓ | ✓ | qpdf, pdfunite, pdftk, mutool | — | **moderate**: renumbering is bookkeeping, but merging name trees, outlines and interpage destinations correctly is real clause work |
| page reorder | S | F | ✓ | ✓ | ✓ | qpdf, pdftk | — | **easy-moderate**: page-tree rewrite plus fixing destinations that name moved pages |
| page rotate | S | F | ✓ | ✓ | ✓ | qpdf, pdftk cat suffixes | — | **easy**: a `/Rotate` change is expressible as an incremental update — inside even today's rules |
| page delete/insert | S | F | ✓ | ✓ | ✓ | qpdf | — | **easy-moderate**: delete is split's machinery; insert is merge's |
| n-up / booklet / poster | P | L | ~ | ~ | ✓ | mutool poster, gs | — | **moderate**: pure geometry over XObject-wrapped pages, but it is generator-shaped output |
| extract images | ~ | F | ✓ | ✓ | ✓ | pdfimages, mutool extract | — | **easy**: every decoder already in-tree; walk resources, emit PNG, with pdfimages' as-stored/decoded distinction |
| pdf → image (rasterize) | S | F | ✓ | ✓ | ✓ | pdftoppm, mutool draw | ~ internal | **easy**: three rasterisers exist; this is a CLI wrapper and an output encoder |
| image(s) → pdf | S | F | ✓ | ✓ | ✓ | img2pdf | — | **easy** technically (one image XObject per page) — but it *is* authoring from nothing; scope question, not difficulty |
| office ↔ pdf convert | S | L | ✓ | ✓ | ✓ | libreoffice headless | — | **hard** and out of character: an Office layout engine is a different project; candidate for the exclusion list staying closed here |
| compress / optimize | S | ~ | ✓ | ✓ | ✓ | qpdf, gs -dPDFSETTINGS, ocrmypdf -O | — | **moderate**: stream recompression and object-stream packing are mechanical; the wins (image downsampling, font dedup) need a quality policy — preset-shaped, per gs |
| linearize (write) | ~ | — | — | — | — | qpdf | — | **hard** and lowest demand of the batch: Annex F's layout is exacting and nothing here reads faster for it except first-page-over-HTTP |
| repair / clean | ~ | ~ | ~ | ~ | ~ | qpdf, mutool clean | ~ reads robustly | **moderate**: the parser already recovers malformed files; writing out the recovered form is the writer again |
| print | S | F | ✓ | ✓ | n/a | lp (CUPS takes PDF natively) | — | **easy-moderate**: CUPS spools PDF, so baseline print is handing the file over with page selection; the quality path (our raster at printer resolution) is what the rasterisers do |
| print preview | S | F | ✓ | ✓ | n/a | — | — | **easy**: rendering at a stated geometry is the existing core competence; both Linux viewers demonstrably struggle here, we are unusually well placed |
| text editing (no reflow) | S | ~ | ✓ | ✓ | ✓ | — | ~ free-text annots only | **moderate** for the honest subset (retype a run reusing glyphs the embedded subset has, refuse loudly when it lacks one); **hard** generally (subset fonts, positioning arrays, reflow) |
| image/object edit on page | S | L | ✓ | ✓ | ~ | — | — | **moderate-hard**: content-stream surgery with resource bookkeeping |
| form fill | S | F | ✓ | ✓ | ✓ | pdftk fill_form | ✓ | done today, incl. undo/redo and truncation readback |
| form create | P | L | ✓ | ✓ | — | — | — | **moderate**: field dictionaries + appearance machinery exist for reading; writing them is authoring-shaped |
| form flattening | ~ | L | ✓ | ✓ | ~ | pdftk flatten | — | **moderate**: appearance streams are already constructed/read; stamping them into content and dropping fields is a bounded rewrite |
| OCR | P | L (enhanced) | ✓ | ✓ | ✓ | ocrmypdf | — | **moderate** by integrating an engine (Tesseract) — in the sandboxed worker, per the C-dependency rule, which is security and stays; **hard** if ever in-tree. ocrmypdf's graft-don't-rerender is the model |
| redaction | P | L | ✓ | ✓ | ✓ | — | — | **hard**: *correct* removal spans content streams, partially-covered images, annotations and metadata, and a redactor that leaks is worse than none — the one transform where the bar is absolute |
| sanitize (strip JS/attachments/metadata) | ~ | ~ | ~ | ~ | ✓ | qpdf (partially), mutool clean | — | **easy-moderate**: removal is easier than any edit; Okular bug 452403 shows viewer-side demand |
| watermark / stamp | S | F | ✓ | ✓ | ✓ | pdftk stamp, qpdf overlay | — | **easy**: an incremental update adding content — the writing machinery that already exists |
| page numbers / headers / footers | S | F | ✓ | ✓ | ✓ | — | — | **easy-moderate**: same stamping machinery plus §12.4.2 page-label awareness we already have |
| metadata edit (Info/XMP) | S | F | ✓ | ✓ | ✓ | pdftk update_info, exiftool | ~ reads both | **easy**: incremental update of two objects this tree already parses |
| password add/remove | S | F | ✓ | ✓ | ✓ | qpdf | ~ opens+saves encrypted | **moderate**: §7.6 is implemented in both directions; add/remove is a re-encryption rewrite through the writer |
| permissions change | S | F | ✓ | ✓ | ✓ | qpdf | — | **easy** once re-encryption exists; the reader-side policy shape (four levels) is already `CLAUDE.md` doctrine |
| digital signing (create) | S | L | ✓ | ✓ | ✓ | mutool sign | — | **moderate-hard**: verification's CMS/X.509 machinery exists; signing adds DER *writing*, key stores and the ByteRange placeholder dance |
| signature validation UI | S | F | ✓ | ✓ | ✓ | pdfsig | ✓ integrity+authenticity | surface the existing verdicts in chrome; trust-store question open (`doc/todo/51-signatures-and-public-keys.md`) |
| annotation UI (edit/move/delete existing) | S | F | ✓ | ✓ | ✓ | — | ~ add + retype own | **moderate**: the read and write halves exist; a manipulation UI per host is the work (`doc/todo/33-annotation-editing.md`) |
| compare two documents | P | L | ✓ | — | ✓ | — | ~ internal instruments | **easy-moderate**: raster comparison and text extraction are this project's daily instruments; the work is presentation |
| text extraction | S | F | ✓ | ✓ | ✓ | pdftotext | ✓ + pdf-retrieve | done; logical-order extraction is already oracle-checked |
| attachments add | S | F | ✓ | ✓ | ~ | pdftk attach_files | ~ extract only | **easy**: an incremental update adding a §7.11.4 file spec |
| KIO worker (`pdf:/`) | — | — | — | — | — | — | — | **moderate**: kio_archive is the model, `viewer-ffi`/`pdf-retrieve` the backend; C++ shim against the C ABI |
| FUSE plugin | — | — | — | — | — | — | — | **moderate**: same abstraction over the `fuser` crate; KioFuse covers KDE from the KIO side |
| night mode | ~ | F | ✓ | ✓ | n/a | — | — | **easy**: a recolour at composite time; top-tier tracker demand (Evince 1256, Sumatra 628) |
| tabs | ✓ | F | ✓ | ✓ | n/a | — | — | **moderate**: pure host chrome, times three hosts |
| PDF/A convert | P | — | ✓ | — | ✓ | gs, ocrmypdf | — | **hard**: a conformance rewriter against ISO 19005 — a standard this tree does not yet read |

---

## 8. The exclusions question, framed

`CLAUDE.md` excludes "[a]uthoring a document from nothing" and every clause whose
requirements fall on a generator — amended once already, by argument, to admit what a
*user* does to an open document, written back as §7.5.6 incremental updates. The rule of
that house style: an exclusion is revisited **by argument, never by attrition**. Round
785's RFC 0002 owes the argument; this section frames what it must cover.

The surveyed features sort into three tiers against today's scope:

1. **Inside scope now**: annotation UI, form fill, signature verification surface,
   compare, night mode, tabs, print preview (it renders an open document), attachments
   extraction — and page rotation and watermarking, which fit through the incremental-
   update door that is already open.
2. **Transforms of an existing document** — split, merge, reorder, delete, compress,
   flatten, sanitize, redact, re-encrypt, OCR-layer grafting. These produce a *new
   file*, which today's exclusion catches via "no clause whose requirements fall on a
   generator". The amendment argument round 785 must make: a transform's *input*
   semantics are already wholly our job (we must understand every object to view it);
   its output contains no content the producer did not specify — the program
   re-serializes, it does not compose; and the qpdf precedent shows the
   structural/semantic line can be drawn cleanly. What the argument must also price:
   which generator clauses stop being out of scope (object streams and xref streams on
   the *write* side, at minimum), what the oracle's purity premise needs (a transform
   as `Document` → new bytes keeps `interpret` pure; in-place mutation would not —
   unless the owner lifts that too, which they have explicitly opened), and what "the
   producer's bytes stay in the file" (the incremental-update promise) becomes when the
   operation's whole point is producing different bytes.
3. **Generation from nothing** — images→PDF, office→PDF, form creation, scanner
   ingest. Squarely the old exclusion. The owner's directive says to present what
   lifting it unlocks rather than defend the line: it unlocks Stirling-parity
   (convert-to-PDF is a top web-service draw), the natural KIO write direction
   (copying a PNG *into* `pdf:/doc.pdf/pages/` is generation), and CLI parity with
   img2pdf. Office↔PDF specifically is flagged as staying excluded on effort grounds
   (a layout engine, not a PDF question) — the one candidate for remaining on the
   closed list with its reason intact.

Orthogonal and untouched by any of this: JavaScript stays excluded (its own security
argument), multimedia/3D stays excluded, XFA stays excluded on §K.1's own permission,
and the sandbox rules are not negotiable in any tier.

---

## 9. Headline findings

1. **The demand centre is transforms, and one investment unlocks nearly all of them.**
   Almost every "—" in the us-column that grades easy or moderate is gated on the same
   missing piece: a whole-file **writer** (serializer for a new document built from
   read objects). Split, merge, delete, compress, repair-write, re-encrypt, flatten and
   sanitize all collapse onto it. RFC 0002's core design question is that writer, not
   any individual tool.
2. **Cheapest high-value wins, in rough order**: pdf→image CLI (wrap the existing
   rasterisers), extract-images CLI (decoders exist), print preview (render at printer
   geometry — and the incumbents are demonstrably bad at it), watermark/stamp and
   metadata edit and attachments-add (incremental updates, the door already open),
   page rotate (likewise), night mode (tracker-top demand, composite-time recolour),
   compare (our instruments, resurfaced), sanitize.
3. **Print is demand-backed, not just owner interest**: both major Linux viewers
   rasterize badly or break at preview; CUPS accepts PDF natively so baseline printing
   is cheap, and a *good* preview is this tree's existing core competence.
4. **Viewers grow into editors everywhere** — pdf.js, Sumatra and mupdf-gl all added
   annotation editing under user pressure; Okular's oldest wishes are light text
   editing. The owner's "basic text editing without reflow" has a well-evidenced honest
   subset: retype-in-place reusing the embedded subset's glyphs, refusing loudly
   otherwise.
5. **KIO/FUSE have no tracker demand and real integration value** — they are
   differentiators nobody else ships (nothing in section 1 or 2 has them), with solid
   KDE precedent (kio_archive, KioFuse) and a backend this tree already has in
   `viewer-ffi` and `tools/pdf-retrieve`. They rise or fall on the owner's integration
   vision, and RFC 0003 should argue them on that register honestly.
6. **OCR and redaction are the two features where every product tier agrees on value
   and difficulty** — both Pro-tier everywhere, both present in Stirling, and the two
   hardest rows in the matrix for us (an external engine under the sandbox rule;
   correctness-critical removal).

## Open questions for the owner

- Tier 2 (transforms): amend by class ("content-preserving transforms of an existing
  document are in scope"), or per-operation? RFC 0002 will draft the class wording.
- Tier 3 (generation): is images→PDF wanted enough to open the generation door, or
  should RFC 0002 confine itself to transforms? Office conversion is recommended to
  stay excluded either way.
- `pdf_syntax::Document` immutability: is the preferred unconstrained design in-place
  mutation, or an immutable read side feeding a builder/writer (which keeps the
  oracle's premise for free)? The estimates above work under either.
- CLI shape: one `pdf-utils` binary with subcommands beside `pdf-retrieve`, or fold
  retrieval in as a subcommand too?
- Which of the cheap wins in finding 2 are wanted *before* the writer lands, as
  standalone rounds?

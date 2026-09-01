# RFC 0002 — The transform suite

Status: **accepted** — 2026-09-01, by the owner's word: "Please start the command line features."
Round: 785, commissioned by the owner; implementation from round 867 (ADR 0800), on the long-lived
branch `round-867`. **§13's seven questions were not individually answered**: the recommendations
this document makes stand as defaults, each recorded as a stated assumption in ADR 0800 §6 for the
owner to overrule, and §11's amendment of `CLAUDE.md` is not yet ratified because no writer has
landed.
Companions: RFC 0001 (the survey this argues from), RFCs 0003–0005 (file-system faces,
print, text editing — each consumes the transform layer proposed here). The number 0002
held at merge (round 788); no collision arose.

**Commissioned by the owner**, with an explicit directive quoted here because it changes how this
document must be read:

> the RFCs must NOT be limited by current project rules.

Concretely: the immutable-`pdf_syntax::Document` rule and `CLAUDE.md`'s authoring exclusion ("we
do not create PDFs") do **not** constrain the proposals below. Where a current rule is relevant it
is recorded as *a current restriction with its original rationale*, and then the unconstrained
design is proposed anyway. §11 is the amendment argument the owner can ratify or refuse.

---

## 1. What this RFC proposes, in one paragraph

A **document-transform suite**: split, merge, page assembly (insert / reorder / rotate / delete),
image extraction, page rasterisation to image files, and PDF optimisation — delivered as **one
CLI binary, `pdf-transform`**, over **one library crate, `pdf-transform`, whose public API is
called the *transform seam* throughout this document**. The seam is the deliberate product: round
786 designs the KIO worker, the FUSE filesystem and the UI integration in parallel, and each of
those is one more consumer constructing the same plan types and supplying its own byte sinks. The
CLI is merely the first consumer, exactly as `viewer-ui` was the first consumer of `viewer-core`.
Under the seam, **all byte emission stays in `pdf-syntax`**, which grows a whole-file serializer
beside §7.5.6's incremental updater — the one genuinely new lower-layer capability the suite
needs, and the subject of §10's recommendation.

## 2. Motivation

The owner's list, plus the neighbours that belong beside it:

- People keep a second toolbox next to their viewer — qpdf, pdftk, poppler-utils, or a
  Stirling-PDF container — for operations that are about the *file*, not the page: split this,
  merge those, pull the images out, shrink it, turn page 3 into a PNG. A project that aims to be
  the noticeably fastest PDF viewer available already owns every reader those tools are built on,
  and reads real-world files at a robustness level the corpus gates measure every round. The
  parsing, decryption, filter, font, image and rasterisation layers are the expensive 90% of such
  a toolbox; what is missing is the writing 10% and a front door.
- The retrieval precedent exists and worked: `tools/pdf-retrieve` (ADR 0257) is "a program asks a
  document questions" over the readers this tree already had. The transform suite is the same
  move one step further: "a program asks for a *new file* derived from documents this tree can
  already read."
- Round 786's consumers need a seam, not a binary. A KIO worker that shows a PDF as a folder of
  pages, a FUSE mount that materialises `page-007.png` on `open()`, and a viewer menu item
  "extract this chapter" are all the same operations with different transports. Designing the CLI
  without the seam would leave 786 scraping stdout; designing the seam first makes the CLI thin.

## 3. Current restrictions, each with its original rationale

Recorded before the unconstrained design, per the owner's directive. None of these is treated as
binding below; every one of them exists for a reason worth preserving *in some form*, and §11
says which survive and how.

1. **The authoring exclusion.** `CLAUDE.md`: "Authoring a document from nothing — we do not
   *create* PDFs, and no clause whose requirements fall on a generator is in scope." Rationale:
   scope containment — a generator owes linearisation, object-stream packing, optimisation, and a
   producer's whole obligation set, and a viewer project that drifted into authoring would dilute
   the fidelity goal. Already amended once, by argument: §7.5.6's incremental update for what a
   *user* does to an open document (form fill, annotation) is in scope and shipped (ADR 0121).
2. **Incremental update is today's only write path.** ADRs 0100, 0121:
   `crates/pdf-syntax/src/write.rs` appends new objects, a cross-reference section and a chained
   trailer; the producer's bytes stay in the file byte for byte. Rationale: it is the one form of
   writing the standard itself defines as leaving the original intact, so nothing signed,
   archived or notarised is disturbed, and the writer's correctness burden is minimal.
3. **`pdf_syntax::Document` stays immutable.** Rationale, and it is load-bearing: `interpret` is
   a pure function of the bytes, the view state and the user's input, and the oracle's whole
   cross-renderer comparison rests on the first of those being a function of the bytes alone. An
   editor that mutated the document would cost that silently.
4. **No clock in the core crates.** ADR 0121 made the file identifier a digest so that saving the
   same edit twice produces the same file. Rationale: determinism is testability.
5. **Dependencies are argued in `doc/stack.md`**, and a C dependency touching untrusted bytes is
   confined (principle 3). Rationale: security posture and the teaching goal.
6. **A document's restrictions are the reader's to set** (`CLAUDE.md` principle 3,
   `doc/todo/38-a-documents-restrictions-have-levels.md`): Table 22's `/P` bits are a policy with
   four levels — off, on, ask, warn — asked once, in a place a host can supply.

The one discovery worth stating up front, because it dissolves half the feared conflict:
**transforms do not need a mutable `Document` at all.** A transform reads one or more immutable
documents and *builds a new file* — new object table, objects copied by reference from the
sources, new cross-reference structure — through a builder that owns only the output. Restriction
3 survives the entire suite untouched, and with it the oracle's purity argument. What actually
needs amending is restriction 1 (and 2 as its consequence), and only §11's carefully drawn slice
of it.

## 4. The shape of the suite

### 4.1 One binary, subcommands

```
pdf-transform <subcommand> [arguments] [options]

  split         one input, many PDF outputs
  merge         many inputs, one PDF output
  pages         one input, one PDF output: delete / insert / reorder / rotate
  images        extract the images a document embeds, as files
  render        rasterise pages to image files
  optimize      rewrite one input smaller
  attachments   list or extract §7.11.4's embedded files
```

Prior art splits three ways: poppler ships one binary per transform (`pdfseparate`, `pdfunite`,
`pdfimages`, `pdftoppm`, `pdfdetach`), pdftk one binary with keyword operations and a trailing
`output` keyword, mutool and qpdf one binary with git-style subcommands / long options. The
subcommand form is proposed because it is the modern convention (mutool, qpdf, git, cargo), it
keeps the suite discoverable (`pdf-transform --help` lists the verbs), and it matches this tree's
`pdf-retrieve <verb> <file>` precedent. `pdf-retrieve` stays a separate binary: it answers
questions, this one makes files, and folding read-only verbs under the transform binary is listed
in §13 as an owner question rather than proposed.

### 4.2 The page-range grammar, shared by every subcommand

One grammar, stated once, used by `split --pages`, `merge`'s per-input ranges, `pages`,
`images --pages` and `render --pages`. Prior art is genuinely split — qpdf
(`1-5,r2,x3,:even` positional parity), pdftk (`1-6even`, `r3-r1`, `~5-6` subtraction, rotation
suffixes), cpdf (`all`, `end`, `reverse`, `~1`, parity by page number), mutool (`1,5,10-15,20-N`)
— so imitation of any single tool is impossible and a deliberate choice is owed. Proposed:

| form | meaning |
|---|---|
| `5` | page 5, counted from 1 |
| `3-7` | inclusive range; `7-3` is the same pages reversed |
| `1-end` | `end` is the last page (mutool's `N`, cpdf's `end`; a word, not qpdf's `z`) |
| `r1`, `r3-r1` | counted from the end: `r1` is the last page (qpdf's and pdftk's spelling) |
| `a,b,c` | concatenation, order significant, duplicates allowed |
| `x3-4` | exclusion from the running selection (qpdf's spelling) |
| `3-7:odd`, `:even` | filter the range by **page-number parity** |
| `@iv`, `@{A-3}` | the page whose §12.4.2 **page label** is `iv` / `A-3`; braces when the label contains `,`, `-` or `:`; `@iv-@ix` is a label-addressed range |

Two departures from prior art, both deliberate and to be documented as choices:

- **`:odd`/`:even` filter by page number, not by position in the selection.** qpdf filters by
  position, pdftk and cpdf by page number; the duplex-printing use case people actually have is
  page-number parity, and position parity surprises. The qpdf behaviour is recoverable by
  composing selections; the reverse is not.
- **Label addressing is in the grammar, not a mode switch.** No surveyed tool can address "the
  page labelled iv" at all, and this tree already reads §12.4.2 (`crates/pdf-model/src/page_label.rs`)
  and prints labels in three hosts. It is the cheapest genuinely-better-than-prior-art feature in
  the suite. Resolution is at plan time against the named source document; an unmatched label is
  a refusal naming the label, and a document whose label sequence produces duplicates resolves
  `@x` to the first match, stated in `--help` as a choice.

### 4.3 Inputs, outputs, and secrets

- **Multi-file output names are printf-style patterns**, following pdftk `burst`/`pdfseparate`/
  mutool: `-o 'part-%d.pdf'`. `%d` is the ordinal, zero-padded to the width of the count
  (`%03d` overrides); `%p` is the first source page number of the piece, `%l` its §12.4.2 label
  (sanitised: path separators and control bytes replaced, a choice stated in `--help`), `%t` the
  bookmark title for `split --at-bookmarks` (same sanitisation). A pattern without `%d` when more
  than one file would be written is a usage error, not a silent overwrite — pdfseparate's rule.
- **Single-file output goes to `-o out.pdf`, and `-o -` writes the bytes to stdout** (qpdf's
  convention). Nothing else is ever on stdout when `-o -` is given.
- **Diagnostics go to stderr, always.** stdout carries bytes (under `-o -`) or the report
  (§4.5), never prose.
- **Inputs are positional.** `merge` interleaves per-input ranges with a colon:
  `a.pdf:1-5` (cpdf attaches ranges positionally, qpdf with `--range=`; the colon form is
  compact, and the long form `--input a.pdf --range 1-5` exists for paths containing `:`).
- **Passwords never appear on the command line.** An argv password is visible in `/proc` on the
  machine and in every shell history; pdftk's `PROMPT` keyword is the honest prior art. Proposed:
  `--password-prompt` (interactive, the default when a document refuses and stdin is a tty) and
  `--password-fd <n>` (scripted, one line per encrypted input, in input order). A `--password`
  flag is deliberately absent, and its absence is documented so it reads as a decision rather
  than an oversight.

### 4.4 Exit codes

qpdf is the strongest prior art (0 = clean, 2 = error, 3 = warnings, never 1 because wrappers
use it) and poppler the most uniform (0/1/2/3/99). Proposed, and one number is this project's
own:

| code | meaning |
|---|---|
| 0 | success, no warnings |
| 2 | error — the operation did not produce usable output; the message on stderr says why |
| 3 | success with warnings — output written, recoverable malformations reported on stderr; `--strict` turns this into 2, `--quiet-warnings` into 0 (qpdf's `--warning-exit-0`) |
| 4 | **refused by name** — the operation is well-formed but this program declines it: a policy refusal (§8), or an unsupported construct hit on the path (trap 5: unsupported input stays loud; a transform that silently dropped what it could not carry would be the quiet-corruption failure mode) |

Exit 1 is left to the shell and to argument-parsing failures, following qpdf's argument.
The distinction between 2 and 4 is the point: 2 means the *file* defeated us, 4 means *we*
declined, and a caller scripting the suite can tell them apart without parsing stderr.

### 4.5 The report

`--report=json` writes a machine-readable account of what was done to stdout (mutually exclusive
with `-o -`): per output file — pages taken and from where, objects written, bytes, warnings; for
`optimize` — per-category savings; for `images --list` and `attachments --list` — the inventory
without extraction (pdfimages `-list` / pdfdetach `-list` precedent). Same discipline as
`pdf-retrieve`: a fixed shape this tree writes and never parses, one small hand-rolled module, no
serde dependency for a shape this small unless `doc/stack.md` decides otherwise.

## 5. The transform seam — what round 786 sits on

**The seam is the public API of a new library crate, `pdf-transform`** (under `crates/`, because
its consumers are shipped programs, not instruments). Its design rules, each inherited from a
boundary this tree already proved:

1. **A transform is a pure plan applied to sources through caller-supplied sinks.**
   Schematically (final Rust is implementation-round work, but the shape is normative for 786):

   ```text
   Plan        — pure data: which transform, which selections, which options.
                 Built by the CLI from argv, by KIO/FUSE/UI directly. No paths inside.
   Source      — bytes this tree may read: a whole mapping or a seekable reader + length,
                 plus an optional Secret (the §7.6.4.1 password), reusing viewer-core's type.
   Sinks       — the caller hands out one io::Write per output the plan names, on demand,
                 keyed by the pattern-expanded name. The CLI's sink opens files; FUSE's
                 fills a kernel buffer; the UI's fills memory. The library never opens a path.
   Policy      — the document-restrictions answer (§8), off/on/ask/warn-shaped, host-supplied.
   Budget      — explicit memory and operation ceilings, same family as the interpreter's.
   Report      — typed: what was written, what was warned, what was refused and why.
   apply(plan, sources, sinks, policy, budget) -> Result<Report, Refusal>
   ```

2. **No filesystem, no clock, no environment.** The same rules that made `viewer-core`
   confinable for free (its host list is the proof) make the seam confinable and testable: a
   transform's output is a function of (sources, plan, policy) and nothing else. This is also
   what makes §9's determinism claim checkable rather than asserted.
3. **Streaming is in the types, not an option.** `Source` admits a seekable reader because §12's
   performance stance requires copying stream bytes by range; `Sinks` are writers because a
   FUSE consumer cannot hold a 2 GB output in memory any more than the CLI should.
4. **The seam is transport-free.** Whether `apply` runs in-process, or in a confined worker with
   the plan and report crossing a pipe (§8), is the caller's choice; nothing in the API names a
   process arrangement. This is `viewer-confined`'s lesson: the boundary was designed
   transport-free and the confinement cost one outcome type, not a redesign.

What the seam is **not**: it is not `viewer-core`'s boundary and it must not leak into it.
`viewer-core`'s vocabulary is a person at a window's; a transform is a batch job over documents
that need not be open in any viewer. The UI integration round 786 designs will translate a menu
action into a `Plan` and hand the result back through its own chrome — one more consumer, zero
new `Command`s, unless 786 finds a genuine need and argues it against `doc/ui-boundary.md`'s
test.

## 6. The features

Each with: motivation, prior art, proposed CLI, architecture mapping, difficulty, open questions.
Prior-art flags cited here were verified against the tools' current documentation
(qpdf: <https://qpdf.readthedocs.io/en/stable/cli.html>; mutool:
<https://mupdf.readthedocs.io/en/latest/tools/>; pdftk-java and poppler-utils:
<https://manpages.debian.org/bookworm/>; Ghostscript:
<https://ghostscript.readthedocs.io/en/latest/VectorDevices.html>; Stirling-PDF:
<https://docs.stirlingpdf.com/>; cpdf: <https://www.coherentpdf.com/cpdfmanual/indexse2.html>).

### 6.1 `split`

**Motivation.** The most-asked-for transform in every toolbox: one file into per-page files, into
page-range pieces, or into chapters along the outline.

**Prior art.** `pdftk in.pdf burst output pg_%04d.pdf` (default pattern `pg_%04d.pdf`, plus a
`doc_data.txt` metadata report); `pdfseparate -f n -l m in.pdf out-%d.pdf` (pattern with `%d`
required for multi-page output); `qpdf --split-pages[=n] in.pdf out-%d.pdf` (groups of n,
`%d` becomes a zero-padded page range, and `--remove-unreferenced-resources=auto` prunes shared
resources while splitting); `mutool merge` invoked per piece. Splitting *at bookmarks* is the one
on this list no surveyed CLI does first-class (Stirling-PDF's web UI offers "split by chapters").

**Proposed CLI.**

```
pdf-transform split in.pdf -o 'page-%d.pdf'                    # every page its own file
pdf-transform split in.pdf --every 10 -o 'part-%d.pdf'         # pieces of ten pages
pdf-transform split in.pdf --pages 1-3,7-end -o 'sel-%d.pdf'   # one piece per comma-group
pdf-transform split in.pdf --at-bookmarks=1 -o '%d-%t.pdf'     # pieces at outline level 1
```

`--pages` makes one output per comma-separated group (so `--pages 1-3,7-end` writes two files);
`--every n` is qpdf's `--split-pages=n`; `--at-bookmarks[=depth]` cuts where an outline item at
that depth or shallower lands, resolving each item to its page exactly the way
`pdf_model::retrieval::sections` already does for `pdf-retrieve` (ADR 0257) — that machinery,
including its two documented choices about where a section ends, is reused rather than rebuilt.

**Architecture.** The seam's planner computes, per piece, the page set; `pdf-transform`'s
assembly walks each page's object closure — `/Resources` (with §7.7.3.4's inherited attributes
resolved onto each emitted page, because the ancestors that carried them are not coming along),
annotations, the structure-tree fragments that reach the kept pages — renumbers, and hands the
object list to `pdf-syntax`'s serializer (§10). Document-level carrying: the outline subset whose
destinations survive, §12.4.2 page labels recomputed so every piece starts with the label its
pages had in the source, name-tree entries that are still referenced, `/Info` and XMP per §9's
determinism rules. Destinations pointing out of the piece are dropped with a warning (exit 3),
not silently.

**Difficulty: moderate** — the page-closure walk and attribute flattening are new code with real
edge cases (shared resources, inherited `/MediaBox`/`/Rotate`, structure parents), but every
reader it needs exists.

**Open questions.** Whether a piece keeps the source's whole outline (grayed context) or only its
own subtree — proposed: own subtree, it is what every consumer of a chapter file expects.

### 6.2 `merge` and `pages`

**Motivation.** The other half of split; and the single-document page edits (delete, insert,
reorder, rotate) that Stirling-PDF's page-operations tab shows people expect as one family.

**Prior art.** `pdfunite a.pdf b.pdf out.pdf` (whole files only); `pdftk A=a.pdf B=b.pdf cat
A1-21 Bend-1odd output out.pdf` (handles, ranges with rotation suffixes `north/east/…/left/
right/down`); `qpdf --empty --pages a.pdf 1-5 b.pdf 6-4 -- out.pdf` (`--empty` so a pure merge
inherits no primary file's metadata; `--collate[=n]` interleaves; `--rotate=[+|-]90:range`
separately); `mutool merge -o out.pdf a.pdf 1-5 b.pdf` (positional file/range interleaving);
`mutool poster` (page chopping — out of scope here); pdftk `shuffle` (collation).

**Proposed CLI.**

```
pdf-transform merge a.pdf b.pdf -o out.pdf                     # concatenate whole files
pdf-transform merge a.pdf:1-5 b.pdf:end-1 -o out.pdf           # per-input ranges
pdf-transform merge --collate a.pdf b.pdf -o out.pdf           # interleave (pdftk shuffle)

pdf-transform pages in.pdf --delete 3,7-9 -o out.pdf
pdf-transform pages in.pdf --rotate +90:2-end:even -o out.pdf  # relative; --rotate 180:5 absolute
pdf-transform pages in.pdf --move 5:1 -o out.pdf               # page 5 to position 1
pdf-transform pages in.pdf --insert other.pdf:2-4@7 -o out.pdf # other's 2-4 before position 7
```

Rotation follows qpdf's spelling (`[+|-]angle:range`, sign meaning relative to the page's
existing `/Rotate`, no sign absolute, angle a multiple of 90) rather than pdftk's compass words.
`pages` operations compose left to right over the current page list, stated in `--help`; each
`--flag` may repeat.

**Architecture.** Same assembly machinery as split with two additions: **cross-file renumbering**
(trivial once renumbering exists at all) and **document-level merging**, which is where the real
edges live — concatenating outlines (one top-level item per source, a documented choice),
merging §7.9.6 name trees with collision renaming (reported, exit 3), reconciling `/AcroForm`
field-name collisions the way qpdf does not (an honest warning naming the fields is tranche-one
behaviour; silent coexistence of duplicate fully-qualified names is the thing to refuse),
optional-content order concatenation, and page labels per source. Rotate alone is the easy one:
`/Rotate` is one inheritable integer per page.

**Difficulty: moderate** — the mechanism is split's; the merge-time document-level
reconciliations are a long tail of individually small decisions, each of which must be a
documented choice rather than an accident.

**Open questions.** Collation grouping syntax (`--collate=n` per qpdf?); whether `pages` and
`merge` are truly two verbs or one (`merge` with a single input and edit flags subsumes `pages`;
two verbs are kept in the proposal because the common cases read better).

### 6.3 `images` — extract embedded images

**Motivation.** Pulling the originals out of a scanned or picture-heavy document without
re-encoding; the inventory (`--list`) is also the diagnostic people reach for.

**Prior art.** `pdfimages [-f n -l m] [-j|-jp2|-jbig2|-ccitt|-png|-tiff|-all] [-list] in.pdf
root` — writes `root-NNN.ext`; `-all` keeps native JPEG/JP2/JBIG2/CCITT bytes and converts the
rest to PNG; `-list` prints page, size, colorspace, encoding, dpi. `mutool extract in.pdf`
dumps `img-%d` and `font-%d` files into the working directory.

**Proposed CLI.**

```
pdf-transform images in.pdf -o 'img-%d.png'          # every image decoded to PNG
pdf-transform images in.pdf --native -o 'img-%d'     # pass-through: DCT bytes as .jpg, JPX as
                                                     # .jp2, elsewise decoded PNG (pdfimages -all)
pdf-transform images in.pdf --pages 1-10 --min-pixels 32 -o 'img-%d.png'
pdf-transform images in.pdf --list --report=json
```

Default is decoded PNG because it is the answer that always works; `--native` is the lossless
answer where the embedded format has a standalone file form. An image's soft mask is composited
into PNG alpha by default (`--no-mask` keeps them separate as `img-%d.mask.png`), which is a
choice pdfimages does not offer and users ask for.

**Architecture.** `crates/pdf-model/src/image.rs` already decodes every image an interpreted page
can place, JBIG2/JPX/CCITT through the confined `pdf-sandbox` worker — extraction reuses that
path unchanged, so the codec sandbox is not bypassed (§8). PNG encoding uses the `png` crate
already in the workspace (`render-cpu`, `viewer-confined` and others depend on it today). The
walk enumerates image XObjects per selected page's resources rather than interpreting content
streams (an inline-image pass over the content stream is the one interpreter touch, and the
interpreter already parses them). Naming: `%d` ordinal; `--report=json` carries the
page/object provenance rather than encoding it into filenames.

**Difficulty: easy** — every decoder exists; the new code is enumeration, PNG writing (a
dependency already in-tree) and the mask-compositing choice.

**Open questions.** Whether de-duplication (the same image object placed on 40 pages) extracts
once (proposed) or once per placement; JBIG2/CCITT native forms are not standalone files
(pdfimages invents `.jb2e`+globals and `.params` sidecars) — proposed: decoded PNG only for
those two, with `--native` saying so per image rather than inventing sidecar formats.

### 6.4 `render` — pages to raster images

**Motivation.** Thumbnails, previews, slides, OCR pipelines, and round 786's FUSE view of pages
as image files all want "page N at D dpi as PNG bytes".

**Prior art.** `pdftoppm [-png|-jpeg|-tiff] [-r dpi | -scale-to N] [-f n -l m] [-singlefile]
in.pdf root` → `root-NNN.png` (dpi default 150); `pdftocairo` adds vector outputs (`-pdf -svg
-ps -eps`) and stdout via `-`; `mutool draw -o 'page%03d.png' -r 72 [-w W -h H] in.pdf 1-5,N`
(dpi default 72, `-F` forces format for stdout).

**Proposed CLI.**

```
pdf-transform render in.pdf --dpi 150 -o 'page-%d.png'
pdf-transform render in.pdf --pages 7 --scale-to 1600 -o -    # longest side 1600px, stdout
pdf-transform render in.pdf --pages @iv --dpi 300 --format ppm -o 'p-%l.ppm'
```

`--dpi` (default 150, poppler's, not mutool's 72 — the modern-screen answer), or `--scale-to
[WxH|N]` fitting the page box; `--format png|ppm|pgm` with PNG default; `--page-box
crop|media|…` selecting among §7.7.3's boxes with the viewer's own default (crop). JPEG output
is deliberately absent until the encoder decision (§6.5) lands, and its absence is stated in
`--help`.

**Architecture.** This is packaging, not construction: `interpret` → `render-cpu` at a scale
derived from dpi (72 user-space units per inch, §8.3.2.3's user space — the same mapping every
raster gate already performs), PNG out through the in-tree encoder.
`crates/pdf-model/examples/render_at.rs` is essentially this feature as an example today. The
one honest question is whether `render` may use a GPU backend for throughput; proposed: CPU only
in tranche one — the oracle backend is the correctness reference, a batch tool wants no device
dependency, and rayon across pages is the cheap win (§12).

**Difficulty: easy** — the shortest distance between an existing capability and a shippable verb
in this RFC.

**Open questions.** Anti-aliasing / text-rendering knobs (proposed: none; the viewer's rendering
is the product); whether annotation appearances draw by default (proposed: yes, §6.3.2.2's
obligation, with `--no-annotations` opting out).

### 6.5 `optimize` — smaller files

**Motivation.** "Make this attachment fit under the mail limit" is the single most common reason
a normal person opens a PDF toolbox (Stirling-PDF ships Compress on its front page); archives
want lossless shrinking (object streams, recompression, dead objects); both are one verb with a
lossless default.

**Prior art.** Two schools. **qpdf, structure-preserving**: `--object-streams=generate`,
`--stream-data=compress`, `--recompress-flate`, `--compression-level=9`, `--optimize-images`
(recompress images as JPEG only when smaller, with `--oi-min-width/height/area` floors),
`--linearize`, `--remove-unreferenced-resources`. **Ghostscript, re-distilling**:
`-sDEVICE=pdfwrite -dPDFSETTINGS=/screen|/ebook|/printer|/prepress` bundles downsampling
targets (72/150/300 dpi colour) with font and quality policy — but gs *re-interprets the
document to marks and writes a new one*, preserving appearance rather than structure. `mutool
clean -gggg -z` garbage-collects, deduplicates and deflates.

**Proposed CLI.**

```
pdf-transform optimize in.pdf -o out.pdf
    # lossless default: recompress Flate at max level, deflate uncompressed streams,
    # generate object streams + xref streams, drop unreachable objects,
    # deduplicate identical streams. Never touches image pixels.

pdf-transform optimize in.pdf --images downsample=150,quality=80 -o out.pdf
    # lossy, explicit: downsample raster images above ~1.4x the target to 150 dpi at their
    # placed size, recompress as DCT at quality 80 where smaller. Never silent.

pdf-transform optimize in.pdf --profile email|screen|print -o out.pdf
    # named bundles over the same knobs, each bundle's contents printed by --help
    # (the gs -dPDFSETTINGS convention, with the contents stated rather than folklore)

pdf-transform optimize in.pdf --linearize -o out.pdf           # phase two, see below
```

The **structure-preserving school is the proposal** and re-distilling is rejected outright: gs's
approach discards tagging, form structure, and every object it does not understand — this
project's whole premise is fidelity to what the producer wrote, and a "compressor" that rewrites
the document into its own dialect fails that premise even when the pixels match. The report
(§4.5) prints per-category savings so the user knows what the number came from; a lossy run that
fails to shrink an image keeps the original (qpdf's `--optimize-images` rule).

**Architecture.** Reachability from the trailer is a mark-and-sweep over `pdf-syntax`'s object
graph (the readers exist; the walk is new). Recompression decodes and re-encodes Flate streams
through the same `flate2` this tree ships. Object-stream and xref-stream *generation* are the
serializer's (§10) — §7.5.7 and §7.5.8 on the producer side. Image downsampling decodes through
`pdf-model`'s existing image path (sandboxed codecs included), resamples on the CPU (the
area-averaging machinery in `pdf-render` is prior art in-tree), and re-encodes — which needs a
**DCT encoder, a dependency this tree does not have** (`zune-jpeg` decodes only). That is a
`doc/stack.md` decision the owner must make (§13), and lossy image optimisation is gated on it;
everything else in this verb is not. `--linearize` (Annex F) is explicitly **phase two, and may
be declined permanently**: it is the one sub-feature whose whole value is claimed by byte-range
streaming consumers, its Annex is long, qpdf's implementation history shows it is a defect
farm, and no other part of the suite depends on it.

**Difficulty: hard** — not for any one mechanism, but because "smaller, appearance-identical, and
honest about it" is a three-way obligation, the lossy path adds an encoder dependency and a
quality argument, and §9's oracle must hold over the whole corpus.

**Open questions.** The encoder dependency; whether `--profile` names ship at all or the knobs
stay explicit (proposed: ship them, people demonstrably think in gs's presets); default
compression effort (zlib 9 vs `zopfli`-class — measure first, principle 2's rule).

### 6.6 `attachments` — and the adjacencies already in the tree

**Prior art.** `pdfdetach -list | -save n | -saveall [-o path]`; pdftk
`attach_files`/`unpack_files` (the write direction).

**Proposed CLI.**

```
pdf-transform attachments in.pdf --list [--report=json]
pdf-transform attachments in.pdf --save-all -o dir/
pdf-transform attachments in.pdf --save NAME -o file.bin
pdf-transform attachments in.pdf --attach report.csv --to-page 3 -o out.pdf   # the write direction
```

**Architecture.** The read direction is plumbing over readers that already ship: §7.11.4's
embedded-file streams and both of their homes (the name tree and per-page file-attachment
annotations — the viewer already extracts both on click, and `/Collection` documents already get
their schema read). The write direction (`--attach`, pdftk's `attach_files`) is the smallest
consumer of the writer: one embedded-file stream, one name-tree insertion — and it is the one
verb in the suite §7.5.6's *incremental* writer could serve today, which makes it a natural
first-landing candidate. **Difficulty: easy** (read direction), **easy-moderate** (write).

**Text extraction is deliberately not in this suite.** It exists: `pdf-retrieve` answers a
document's or a section's text as `Interpretation::text` byte for byte, and a test holds it to
that so the tool cannot drift from the measured extraction (ADR 0257). Duplicating the verb here
would create the second copy that drifts. What the transform work should hand `pdf-retrieve` is
one small gap: a `--pages` selection using §4.2's shared grammar, so the two tools' users learn
one range language. Listed as adjacent work, not as a transform.

**`info` is likewise not proposed** (`pdfinfo`'s job): `pdf-retrieve document` already answers
it as JSON.

## 7. Where the write path lives

One rule, extending the one that already holds: **`pdf-syntax` emits every PDF byte this project
writes.** Today `crates/pdf-syntax/src/write.rs` is "the *only* writing in the tree"
(`doc/crate-map.md`); the serializer (§10) lands beside the incremental updater in that same
module family, so clause 7's syntax on the way out continues to have exactly one home, with
`Document::encrypt_for_update`'s machinery giving both writers §7.6 on the way out. The layering:

| layer | gains | does not gain |
|---|---|---|
| `pdf-syntax` | the whole-file serializer: object table → §7.5.7 object streams (optional) → §7.5.4 table or §7.5.8 stream → trailer → §14.4 identifiers; encryption on the way out | any knowledge of pages, images or what a transform means |
| `pdf-model` | nothing structural — its readers, image decode and `retrieval::sections` are consumed as they are; small additions where a transform needs a reader the viewer never did (e.g. enumerating a page's image XObjects without interpreting) | any write path |
| **`pdf-transform` (new)** | the seam (§5): planning, page-closure assembly, renumbering, document-level merge/carry logic, optimisation passes, the raster and extraction verbs' orchestration; the CLI binary in its `src/bin/` | byte-level emission (delegated down), paths/clocks/processes (the caller's) |
| `viewer-core` and hosts | nothing in this RFC; round 786 decides what its UI consumer needs | — |

`fuzz/` gains targets for the new surface: the serializer round-trip (write then re-read with
this tree's own reader) and the planner over hostile range/label inputs.

## 8. Security posture

The suite must not become the unconfined route around the sandbox, and three commitments keep it
from becoming one:

1. **Same parsing path, same budgets.** Transforms read documents through the identical
   `pdf-syntax`/`pdf-model` path the viewer uses — `#![forbid(unsafe_code)]` on every crate that
   touches PDF bytes, the streaming-decompression windows and explicit budgets that already
   bound bombs (`doc/todo/14-stream-the-decompression.md`'s measurements), the same typed
   refusals. There is no "it's just a CLI" second parser and no relaxed mode.
2. **The codec sandbox is not optional here either.** JBIG2, JPX and CCITT decode only in the
   confined `pdf-sandbox` worker, exactly as in the viewer; a transform that needs decoded
   pixels (images, render, lossy optimize) spawns the worker, and a build without the worker
   beside it refuses those images by name rather than falling back in-process. The suite
   inherits this by using `pdf-model`'s image path rather than reaching around it.
3. **The seam is confinable by construction, and confining it is the stated direction.** Because
   `apply` names no filesystem, no clock and no process (§5), the transform core can run as a
   confined child on the `pdf-view-worker` pattern — plan in, report out, sources and sinks as
   descriptors the broker opened — with the CLI as the unconfined broker doing only argv, path
   opening and pattern expansion. Proposed sequencing, honestly costed: **tranche one ships
   in-process** (the parse path is memory-safe, budgeted and fuzzed, which is the same posture
   `pdf-viewer` itself ships with today outside `pdf-viewer-confined`), with the worker split as
   its own follow-up round once the verbs settle — a transport change, not a redesign, and the
   seam's design is what makes that sentence checkable.

One transform-specific hazard is named now so it is designed against rather than discovered:
**pattern-expanded output names are attacker-influenced** when `%t` (bookmark title) or `%l`
(page label) is used — a title of `../../.bashrc` must not escape the output directory.
Sanitisation strips path separators, parent references and control bytes, is stated in
`--help`, and gets its own tests and a fuzz seed. The report names every sanitised name.

## 9. Determinism, and what a transform's oracle looks like

A transform's output is bytes, so its correctness story needs more than "it opened in Acrobat".
Four layers, strongest first:

1. **Byte determinism.** Same sources, same plan, same version ⇒ same bytes, with no flag
   needed. No clock (metadata dates are written only when the caller passes `--date <ISO-8601>`;
   by default `/ModDate` and XMP dates are carried or omitted, a documented choice), no
   randomness, §14.4's second identifier a digest of the output (ADR 0121's precedent; qpdf
   needs `--deterministic-id` to promise this — ours is the default and the flagless behaviour).
   This is what makes every other layer a test instead of a demo.
2. **Self read-back.** Every output re-opens through this tree's own reader — the same one the
   corpus gates trust — and its declared structure is verified: xref complete, closure reachable,
   every copied object byte-identical to its source where the transform promised pass-through.
   ADR 0121 proved this layer catches what diffs cannot.
3. **The raster oracle, and it is the load-bearing one.** A page that a transform carried must
   draw identically: render page k of the output and its source page with the same backend at
   the same scale — `render-cpu`, the correctness oracle — and require **bit-identical** rasters
   for lossless transforms (split, merge, pages without rotate, lossless optimize), a stated
   `raster-compare` tolerance for lossy optimize, and the rotation-transformed comparison for
   rotate. This turns "appearance-preserving" from a claim into a gate, reuses the comparison
   machinery the project lives on, and is derivable from the specification (the same content
   stream, resources and boxes shall mark the same pixels) rather than from any other tool.
4. **Foreign readers as evidence.** Outputs opened by poppler/mutool and checked by
   `qpdf --check`, in exactly principle 5's register: agreement is evidence our reading of the
   writer-side clauses is right, never the definition of right. (Interface conventions are
   imitated from these tools; correctness never is.)

Plus two property gates that fall out for free once determinism holds: `split` then `merge`
reproduces the source's pages under layer 3; `optimize` is idempotent — its own output, optimized
again, is byte-identical. The corpus is the population for all of this: a transform gate that
splits/merges/optimizes the pdf.js corpus and holds layers 2-3 over every document is the
suite's equivalent of the render corpus gate, and it is how the robustness denominator
(`CLAUDE.md`'s second question) gets answered for writing as it is for reading.

## 10. The writer: incremental update versus a real serializer

The central architecture decision, argued as options with a recommendation.

**Option A — incremental update only (the status quo).** §7.5.6 appends; the original bytes
ship inside every output. Merge is impossible (two originals, one file), split ships the whole
2 GB source inside every 10-page piece, and optimize is a contradiction in terms (the output
contains everything it removed). Serves user edits and `--attach`; serves the suite not at all.
**Rejected for transforms, retained unchanged for what it does today.**

**Option B — re-distillation (the Ghostscript shape).** Interpret every page to marks and write
a new document that draws the same. Rejected: it forfeits text, tagging, forms, and the
producer's constructions wholesale — a fidelity project cannot ship a writer whose output is its
own dialect of the input; and it converts every writer bug into silent visual corruption that
only layer-3 comparison can see. Its one legitimate descendant survives as `render` (§6.4),
where rasterisation is the *stated* product.

**Option C — a structure-preserving serializer (the qpdf shape). Recommended.** A new
whole-file writer: collect the output's object set (copied by reference from immutable source
`Document`s, plus objects the transform synthesised — a new page tree, a merged name tree),
renumber, emit each object in clause 7 syntax **with stream bytes copied encoded, by range,
untouched** unless the transform is *about* those bytes, then object streams (optional), xref
(table or stream), trailer, §14.4 identifiers. The producer's content is preserved not as
original file bytes (Option A's guarantee) but as original *object* bytes reachable through a
new skeleton — which is exactly what split, merge and optimize mean.

Sub-decisions inside Option C, each proposed with its reason:

- **Output form follows the inputs by default**: xref stream + object streams when every source
  is 1.5+, classic table otherwise — ADR 0121's "the kind the file already uses" argument,
  promoted from sections to whole files; `--object-streams=generate|disable` overrides
  (qpdf's vocabulary).
- **Renumbering is total and sequential.** No attempt to preserve source object numbers;
  provenance goes in the report, not the file.
- **Linearisation is not part of the serializer** and Annex F stays writer-side-excluded unless
  the owner opts in (§6.5); a serializer designed for later linearisation costs design freedom
  now for a feature that may never be wanted.
- **`pdf_syntax::Document` remains immutable**, and the serializer takes `(&[&Document],
  object-selection, replacements)` — the same "log beside the document" shape ADR 0121's save
  path and `view.rs` already use, scaled up. No transform mutates a parsed document, ever; §3's
  oracle-purity rationale survives verbatim.

## 11. The amendment, ready to ratify

What must change in `CLAUDE.md` for the recommendation to be legal, what a writer then owes, and
what it costs. Presented as the wording change plus its consequences; the owner ratifies,
narrows, or refuses.

### 11.1 The exclusion, redrawn

The current exclusion conflates two things the transform suite forces apart. Proposed
replacement for the "Authoring a document from nothing" entry, in full:

> **Authoring content from nothing** — we do not compose pages: no layout engine, no
> text-setting, no chart drawing, no "HTML to PDF". No clause whose subject is deciding what
> marks a page should contain falls on this project.
>
> **Assembling documents from existing documents is in scope.** Splitting, merging, reordering,
> rotating, extracting and optimising operate on content some producer already specified; every
> content stream in their output is a producer's, carried byte for byte or recompressed without
> reinterpretation. The writer this requires is §10 of RFC 0002's serializer: it emits
> structure (object table, streams containers, cross-reference, trailer, identifiers), never
> content. §7.5.6's incremental update remains the only writing that touches a file a user is
> *editing in place*; the serializer is how a *new* file is derived from old ones.
>
> Generator obligations come into scope only where the serializer actually emits the construct:
> §7.5.4/§7.5.5/§7.5.7/§7.5.8 on the way out, §14.4, §7.6 encryption on the way out. Annex F
> stays excluded until linearisation is separately ratified. The ledger's `writer-side` status
> narrows accordingly (§11.2).

The boundary line that keeps the exclusion enforceable: **does the operation invent marks?**
Rotate does not (it writes an integer the producer's renderer already honours); a watermark
stamp *does* (it composes new content over pages), which is why qpdf's `--overlay`/`--underlay`
and Stirling's watermarking are **deliberately not in this suite** despite being conventional —
they are the first feature on the far side of the redrawn line, and taking them later must be
its own argued amendment, not scope creep. Variable-text field appearances
(`crates/pdf-model/src/variable_text.rs`) already sit on this line today, sanctioned by
§12.7.4.3's own requirement; the line is where it always was, now written down.

### 11.2 What it does to the ledger

The ledger's `writer-side` status is defined in `doc/conformance/ledger.toml`'s header as
"addresses a PDF generator; this program writes only §7.5.6's updates" — a definition this RFC
falsifies the day the serializer lands. The amendment: `writer-side` splits into rows the
serializer now owes (moved to `partial` with the debt named — §7.5.7's and §7.5.8's producer
halves are the certain movers) and rows still addressed to a content generator (definition
rewritten to name the serializer's boundary). `grep -n 'writer-side' doc/conformance/ledger.toml`
is the worklist and prints its own size; the re-derivation is one session's reading, and it is
the same motion the transfer-function entry already demonstrated — a restriction drawn early,
taken off by reading rather than by attrition.

### 11.3 What it costs, honestly

- **A writer is attack surface in the other direction**: this project starts *producing* files
  other parsers read. A malformed output is this project's defect in a way a misrendered page
  never was. §9's layers 2-4 are the mitigation, and they must be gates, not habits.
- **The corpus obligation doubles for the verbs' population**: every transform gate is another
  full-corpus pass in §2's sequence, with wall-clock cost to budget.
- **Producer clauses join the conformance denominator** — bounded by §11.1's "only where the
  serializer emits the construct", but real: those rows must be read, implemented and cited like
  any others.
- **The teaching claim extends**: the serializer must be exemplary, not merely correct, because
  a writer is exactly what students will read first.

## 12. Performance stance

Stated as the design's assumptions, so an implementation that misses them is failing a stated
requirement rather than a hope:

- **Memory is O(working set), never O(file).** A 2 GB split holds: the xref and object metadata
  of the source, the closure computation for the current piece, and one bounded copy window at
  a time — stream bytes cross from source to sink **encoded, by byte range**, never decoded,
  never whole-file-buffered. The streaming machinery argued in
  `doc/todo/14-stream-the-decompression.md` is the read side of this; the serializer's write
  side is designed to the same rule. Decoding happens only where the transform is about decoded
  bytes (images, render, lossy optimize), under the per-object budgets that already exist.
- **This is a throughput tool, and that is a stated contrast.** `CLAUDE.md`'s latency-first rule
  is about the interactive viewer; a batch transform optimises documents-per-second, so rayon
  across pieces/pages/images is the default shape (render is embarrassingly parallel; split's
  pieces are independent). The contrast is written here so nobody imports the viewer's rule into
  the wrong context — or the reverse.
- **Startup discipline is inherited, not re-argued**: the CLI pays for what the verb needs and
  nothing else — no full page-tree walk to split pages 1-3 out of a thousand-page document
  (incremental parsing already guarantees the open; the closure walk is per-piece).
- **Numbers are set by measurement, not here.** The gate's perf floor (e.g. split throughput on
  the corpus's largest documents, optimize wall-clock per 100 MB) is fixed once a baseline
  exists, per the project's standing rule; this RFC commits only the *shape*: transform gates
  carry perf floors from their first landing, because a batch tool's regressions hide even more
  quietly than a viewer's.

## 13. Difficulty ranking, and the questions only the owner can answer

**Ranking** (each one sentence, per the commission):

| feature | estimate | why |
|---|---|---|
| `attachments` (read) | easy | plumbing over shipped readers |
| `render` | easy | `interpret` + `render-cpu` + in-tree PNG encoder, already an example |
| `images` | easy | decoders and sandbox path exist; new code is enumeration and PNG writing |
| `attachments --attach` | easy-moderate | smallest writer consumer; could even land on §7.5.6 alone |
| `split` | moderate | page-closure walk and inheritance flattening are new, on existing readers |
| `merge` / `pages` | moderate | split's machinery plus a long tail of document-level merge choices |
| `optimize` (lossless) | moderate-hard | reachability, dedup, object-stream generation, all under §9's oracle |
| `optimize` (lossy images) | hard | DCT encoder dependency plus a quality argument plus tolerance gates |
| `--linearize` | hard, deferred | Annex F wholesale, value confined to streaming consumers, defect-prone in prior art |

**Questions for the owner**, most consequential first:

1. **Ratify §11.1's redrawn exclusion?** The whole suite hangs on it; the sub-question with
   teeth is whether the "no invented marks" line (no overlay/underlay/watermark/page-numbering)
   is where you want the fence, since it excludes several conventional toolbox features.
2. **The DCT encoder dependency** (lossy optimize, JPEG render output): a reviewed crate per the
   `crypto-bigint` precedent, or in-tree, or decline lossy image optimisation? This gates the
   feature users will ask for first ("make it smaller").
3. **Confinement tranche** (§8): accept in-process tranche one with the worker split as a
   follow-up, or require the brokered worker before the first verb ships?
4. **Restrictions policy for a non-interactive tool** (§3 item 6): Table 22's assembly/extraction
   bits under the four-level shape — a pipe cannot "ask", so propose default `off` (the
   program is the reader's) with `--restrictions=on|warn` for deployments that want them
   honoured; confirm that default, because it is the owner's principle being applied to a new
   surface.
5. **Linearisation**: declined permanently, or kept as phase two? §6.5 recommends deferring
   without designing for it; a later reversal costs serializer rework.
6. **Naming**: `pdf-transform` for crate and binary, with `pdf-retrieve` staying separate? The
   alternative — one umbrella binary with read and write verbs — is more discoverable and more
   churn.
7. **Metadata stance** (§9): confirm the deterministic default — no dates written unless
   `--date` is passed, and no self-naming `/Producer` line in other people's files (writing one
   is the convention among all surveyed tools; declining it is a statement).

## 14. What lands first

If the owner ratifies §11, the natural first implementation round is **`render` + `images` +
`attachments` (read)**: three easy verbs, zero writer dependency, immediate user value, and they
force the seam, the CLI grammar, the range parser and the report format into existence — after
which the serializer round (§10) and `split` land the writing half against gates that already
exist, and 786's consumers meet a seam that has already survived three verbs' worth of contact.

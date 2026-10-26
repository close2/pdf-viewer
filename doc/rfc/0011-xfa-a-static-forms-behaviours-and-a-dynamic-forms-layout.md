# RFC 0011 — XFA in two halves: a static form's behaviours, and a dynamic form's layout

Status: **proposed** — ready for the owner. Nothing in this document changes a ledger status, a
`Cargo.toml`, a line of `CLAUDE.md` or a line of code that ships; the round that wrote it added no
instrument and no dependency.
Round: 1773 — the first of the XFA rounds the owner set in `doc/questions/A633` (2026-10-26): *"I
have downloaded XFA. We should start implementing it one round per batch."*
Builds on: RFC 0009 section 4 (XFA, and the sentence every window now says, ADR 1932), RFC 0008
(scripts: the tiers, the `Scripts` level, `pdf-script` and its confined worker), `CLAUDE.md`'s XFA
exclusion and its *Authoring content from nothing* line as amended (ADRs 1014, 1120, 1123).
Asks: **Q639**, the amendment to the authoring exclusion that the dynamic half needs, by name
(section 7). ADR 2380 records where the halves are cut and what the static half's first round is.

`§N` in this document is a clause of ISO 32000-2 and nothing else; *XFA 3.3* is Adobe's *XML Forms
Architecture (XFA) Specification, version 3.3* of 9 January 2012, the text ISO 32000-2 itself names
in §K.2, cited by its chapter, its section title and its printed page. Quotation marks mean
verbatim from `doc/md/ISO_32000-2_sponsored_EC3.md`, or the owner's words where the owner is named;
XFA 3.3 is held as licensed to one reader (`doc/third-party-data.md`) and is **cited and
paraphrased, never quoted** — an element or attribute name in code type (`occur`, `subformSet`) is
the grammar's name for a thing, not a quotation.

---

## 0. In one paragraph

XFA 3.3 draws the line this RFC needs itself, and draws it at the grammar. A **static** form is one
whose template, merged with data, is laid out exactly as it will be presented; in the variant XFA
calls *XFA Foreground* (XFAF) the page's appearance is PDF and the template carries only the form's
logic, and XFA 3.3 lists by element the grammar an XFAF form may not use for anything but its
defaults — `occur`, `subformSet`, `pageSet`, `pageArea`, `contentArea`, `draw`, a subform's
`layout`, `x`, `y`, `w`, `h` and the rest — with the reason against each: pagination, drawing and
placement are *delegated to the PDF appearance stream*. A **dynamic** form is one whose number of
subforms is decided by its data (an `occur` whose bounds are unequal or unbounded, or a
`subformSet`), and its boilerplate lives in the template because no PDF page can hold a layout that
moves. So the static half is **calculations, validations, formatting and events over the AcroForm
§K.2 already makes the form** — no mark this program invents, and much of it reachable through
what `pdf-model` and `pdf-script` already are — and the dynamic half is **a layout engine run on the
producer's template**, which is the middle of the authoring exclusion and is asked for here by name
before a line of it is built. The static half's first round has a contract, and it is a defect
before it is a feature: **this tree's save of a filled hybrid form leaves the XFA `datasets` packet
saying the old values**, against §K.2's rule for a writer "modifying a PDF file with an XFA
resource".

## 1. The text in hand

`doc/XFA-3_3.pdf` is the owner's copy (7 698 028 bytes, 1 584 pages, untagged, produced by Acrobat
Distiller 10.1.1). Its SHA-256 is the one `doc/third-party-data.md` already records for the copy a
round fetched from the Internet Archive's capture of the PDF Association's normative-references
page, so it is the same file, and its conversion is already under `doc/md/XFA-3_3.md`, made by
`tools/spec-md.py` with this project's own `quorra-retrieve` — the converter the AdobeSupplement and
XFDF conversions came from, whose script has not changed since. **The conversion is not re-made
here**: the source is byte-identical, and what a re-run could change is only what the reader under
the script has learned since, which section 2's one load-bearing table shows it did not need. The PDF carries no structure tree, so the conversion is in the page's content order: one
`<!-- page N (N) -->` marker per page, the printed folio equal to the page index, running heads
left in the text (an untagged page has nothing to mark them as artifacts), and no `##` headings —
a citation is *chapter, section title, page*, and `grep -n '^<!-- page 272 '` finds it. Its tables
were checked where this RFC rests on one: the *Grammar Excluded from XFAF* table on pages 272–273
reads the same under `pdftotext -layout` as in the conversion, row for row.

**`doc/specifications.zip` need not carry it yet.** The archive holds every specification a gate
opens and its conversion (`doc/environment.md`, *The specifications*); no gate, test or example
opens the XFA text today, and the XFDF and AdobeSupplement texts — held the same way — are not in
it either. The first implementation round whose test derives an expected value from XFA 3.3 makes
it a text a fresh clone needs, and that round asks the owner to add it.

## 2. Where XFA 3.3 cuts static from dynamic

### 2.1 Three kinds of form, by XFA 3.3's own names

XFA 3.3 chapter 1, *Static versus Dynamic Forms* (pages 28–30), and chapter 9's section of the same
name (page 333) name three kinds, and the table on page 29 maps them onto Adobe Designer's terms:

| XFA 3.3's name | what the PDF holds | what the template holds | Designer 8.2 calls it |
|---|---|---|---|
| **static XFAF** (XFA Foreground, since XFA 2.5) | every page's appearance, as PDF | the form's logic only: the fields, the subforms that relate them, scripts | a *PDF form* |
| **traditional static** | a minimal wrapper | the whole form, boilerplate included, at fixed occurrences | converted to XFAF on output |
| **dynamic** | a page for processors that decline (a notice) | the whole form, laid out at run time from the data | an *XFA form* |

The criterion is per subform (page 333): a subform is static when it has no `occur`, or when its
minimum, maximum and initial occurrences are equal; it is dynamic when they differ or the maximum
is unbounded; and a form is dynamic if it contains a `subformSet`. Chapter 1's data-binding
overview (page 26) states the consequence for XFAF in one line: a form whose fixed content is PDF
must be static.

Between the two sits what chapter 8 calls **growable** containers (pages 274 onwards): a form
whose data structure is fixed but whose containers stretch to their content. It is not dynamic by
page 333's test, and it is laid out at run time all the same.

### 2.2 The grammar an XFAF form may not use

Chapter 7, *Grammar Excluded from XFAF* (pages 272–273), is the boundary written as a table. An
XFAF form may express these only at their defaults — XFA 3.3 explains that omitting an element and
stating it with default values are the same thing in its grammars:

| element | what is excluded | the reason the table gives, paraphrased |
|---|---|---|
| any element | `use`, `usehref` | no prototype references |
| `proto`, `area`, `subformSet` | all use | no prototypes; only static objects |
| `pageSet`, `pageArea`, `contentArea`, `medium` | all use | pagination and print control are the PDF's |
| `draw` | all use | drawing is the PDF appearance stream's |
| `occur` | all use | single occurrences only |
| `field` | `border`, `colSpan`, `margin`, `keep` | the field's relation to the page is the PDF's |
| `border` | even and left-hand borders | delegated to the appearance stream |
| `subform` | `layout`, `x`, `y`, `w`, `h`, `minH`/`maxH`/`minW`/`maxW`, `occur`, `overflow`, `pageSet`, `break*`, `keep`, `margin`, `border`, `columnWidths`, `para`, `locale`, … | allowed only to express the form's logic, never to change its appearance or its number of instances |
| `script` | the `instanceManager` object | a script may not change the number or order of objects in the form |

Chapter 2, *Connecting the PDF to the XFA Template* (pages 72–74), adds the three things the PDF
side of an XFAF form must be: Table 29's `/NeedsRendering` **must be false**, because an XFAF
template does not hold what regenerating a page would need; the template and its packets are
Table 224's `/XFA`; and every PDF field is a root field whose name is an XFA-SOM expression naming
its template field — `form.` followed by the subform names from the outermost, with a `[n]` index
for a repeated name.

### 2.3 The brief's hypothesis, tested

The brief guessed that the halves separate *at the template's `layout` and `presence`
properties*. **Half of that held.** `layout` is on the far side, but as one row of section 2.2's
table among twenty, and not the row that decides: a subform's `layout` says how its children
flow, and an XFAF form keeps it at its default (`position`) because placement is the PDF's.
`presence` is not on the table at all. Its four values (chapter 17, the `presence` property, page
630) each name the phases an object takes part in — interaction, layout, rendering, and the
automation of calculations, validations and events — and the static half reads every one of them
as automation and display: `inactive` suppresses an object's calculations, validations and events
(chapter 10, *Effect of changing the presence value*, pages 408–409), `hidden` and `invisible` stop
it rendering. **What `presence` decides only in the dynamic half is whether a hidden object still
takes space**, and that sentence has a reader only where something lays the page out. So the cut
is `occur` and `subformSet` (page 333's test) with Table 29's flag (page 72's rule), and section
2.2's table says what an XFAF form may not *do* with the rest — the census in section 4 shows that
it is not a list of names a static template never states. `presence` belongs to both halves, the
automation half of it to the static one.

## 3. What ISO 32000-2 says, and what it now binds

- **The permission** stands, and this RFC does not ask to give it up: §K.1, "a PDF processor may
  choose to not implement this feature".
- **The obligation that becomes live with the static half**: "If an interactive processor supports
  XFA forms, that processor shall clearly indicate to the user when they are interacting with an
  XFA form". Every window already says once per document that it carries an XFA form (ADR 1932);
  the static half changes what the sentence says — *this XFA form's calculations, validations and
  formatting run* — and not whether it is said.
- **The page**: "Whether or not the pages generated by the processing of an XFA schema are
  materialized as PDF pages is implementation dependent". A static half that lays nothing out is
  on the near side of this sentence by construction.
- **The writer's rules**, which bind *this program* the moment it saves: §K.2 opens them with
  "When creating or modifying a PDF file with an XFA resource, a PDF writer shall follow" (the
  conversion drops the sentence's last two words, *these rules*; the PDF has them), and the first
  is "PDF interactive form field objects shall be present for each field specified in the XFA
  resource. The XFA field values shall be consistent with the corresponding V entries of the PDF
  field objects." The third is the reason a hybrid file's AcroForm runs no script today: "No A (see
  "Table 166 -Entries common to all annotation dictionaries") or AA entries (see "Table 190
  -Additional entries specific to a screen annotation" and "Table 191 -Additional entries specific
  to a widget annotation") shall be present in the annotation dictionaries of fields that also have
  actions specified by the XFA resource." The behaviours live in the template, so RFC 0008's sites
  never see them.
- **The resource**: §K.2 names what it holds — the form template, "which describes the
  characteristics of the form, including its fields, calculations, validations, and formatting",
  the data, "which represents the state of the form", and the configuration, "which shall be used to
  properly process the form template and associated data".
- **`/NeedsRendering`**, Table 29: "A flag used to expedite the display of PDF documents containing
  XFA forms. It specifies whether the document shall be regenerated when the document is first
  opened." Read with XFA 3.3 page 72, `true` is the mark of a template that is not XFAF.

### 3.1 The defect the static half begins with

`ViewState::save` (`crates/pdf-model/src/view/writing.rs`) writes §7.5.6's incremental update: one
replacement field object per value typed, carrying `/V`, and its appearance. It does not touch
`/XFA`. **So a person who fills a hybrid form here and saves it writes a file whose `datasets`
packet still holds the values the producer wrote, beside `/V` entries that say what the person
typed** — exactly what §K.2's first writer rule forbids, and what a processor that reads the XFA
data (it is, per §K.2, "the state of the form") will show as the form's values. Nothing under
`crates/` or `raster/` writes or reads the `datasets` packet (`grep -rni datasets crates raster
--include=*.rs` finds nothing). This is a writer-side obligation inside the scope the tree
already claims — filling a field and saving it — and it needs no amendment: replacing a packet
stream is §7.5.6's "objects that have been changed, replaced, or deleted", it is data and not a
mark, and the producer's bytes stay in the file beneath it.

## 4. The census

Every file under `doc/pdf.js` and `doc/corpora` matching `*.pdf`, read with `mutool show <file>
trailer/Root/NeedsRendering` and `mutool show <file> trailer/Root/AcroForm/XFA` under the walk bound,
the packets then decoded and parsed with an XML parser; mutool is an instrument here, never a
reading of the standard. 1 479 files, 1 470 of which mutool opened. **8 state an `/XFA` resource,
all as a packet array, and 4 of those state `/NeedsRendering true`**; no file states the flag
without the resource. The instrument was checked against what is already known: the three pdf.js
documents RFC 0009 section 4.2 named by their bytes are among the four, and the fourth is the one
`doc/corpora` document that section counted.

| file | `/NeedsRendering` | template | scripts (JavaScript, FormCalc) | `calculate`, `validate`, `format` | fields: template, AcroForm | what page one shows |
|---|---|---|---|---|---|---|
| pdf.js `annotation-button-widget.pdf` | absent | 2.8 | 0, 0 | 0, 0, 0 | 9, 9 | the form |
| pdf.js `annotation-choice-widget.pdf` | absent | 2.8 | 0, 0 | 0, 0, 0 | 5, 5 | the form |
| pdf.js `annotation-text-widget.pdf` | absent | 2.8 | 0, 0 | 0, 0, 0 | 7, 7 | the form |
| pdf.js `bug1815476.pdf` | absent | 2.8 | 1, 0 | 0, 0, 0 | 25, 25 | the form |
| pdf.js `issue14130.pdf` | `true` | 3.3 | 0, 0 | 0, 0, 0 | 0, 0 | a notice |
| pdf.js `xfa_issue14315.pdf` | `true` | 3.3 | 0, 0 | 0, 0, 0 | 0, 0 | a notice |
| pdf.js `xfa_filled_imm1344e.pdf` | `true` | 2.8 | 1 432, 5 | 21, 187, 34 | 298, 1 | *Please wait…* |
| corpora `jhove-errors/PDF-HUL-114/MOZILLA-1671648-0.pdf` | `true` | 2.8 | 811, 5 | 19, 176, 37 | 287, 1 | *Please wait…* |

What it says, and it is not what the brief expected:

- **The four hybrids are the static half's reach, and their template holds almost no behaviour.**
  Three are pdf.js's widget tests; the fourth, a Spanish form, carries one script — a button's
  click that calls `xfa.host.print`, which leaves the process and is a Tier 2 call under RFC 0008
  section 4.3. None carries a calculation, a validation or a format. What all four do carry is a
  `datasets` packet, so **the static half's first reach in these corpora is section 3.1's save,
  not its scripts**. Every hybrid's AcroForm has as many terminal fields as its template has
  fields, and no field or widget carries `/AA` — §K.2's third writer rule, kept.
- **The behaviour is in the dynamic files.** The two government forms carry 2 243 JavaScript
  scripts and 10 FormCalc ones between them, 363 validations, flowed subforms (`tb`, `lr-tb`,
  `row`, `table`), page areas, and a single AcroForm field each — the placeholder that page one's
  *Please wait…* belongs to. `issue14130.pdf` has a subform with `occur` minimum 0 and maximum
  unbounded, dynamic by page 333's test; `xfa_issue14315.pdf` has no varying occurrence and is still
  not XFAF by its flag. **So scripts reach the reader of these corpora only through the dynamic
  half**, and section 5's script steps serve forms the corpora do not yet hold — which is why they
  come after the save, and why the census is re-run as the corpora grow.
- **The XFAF table is not a presence test.** All four hybrids state grammar the table lists — the
  root subform's `layout="tb"`, a page subform's `w` and `h`, an empty `proto` — because the table
  admits it *to express the form's logic, not to vary its appearance* (page 273). An XFAF
  recogniser that refused a template for naming `layout` would refuse every hybrid here. What
  separates them is the flag, the occurrences and the subform sets; Adobe's configuration agrees
  without being the test — the four hybrids' `config` packets set Acrobat's `dynamicRender` to
  `forbidden` and the four others to `required` (XFA 3.3 chapter 30, page 1310, an Adobe
  configuration grammar).
- **Every one of the eight has a document-level `/Names /JavaScript` tree**; what it holds was not
  read here, and RFC 0008 section 3.4's census is where such scripts are classified.

## 5. The static half, in the order it would be built

Each step is one round at the owner's pace (one per batch, `A633`), each leaves every file
compiling and the windows driven, and none composes a mark.

1. **The resource, read** — `pdf_model::xfa` grows from *does this document state Annex K* to
   *what does its resource hold*: §K.2's two forms (one stream, or the packet array), each packet
   decoded under the stream budgets every decoder already has, parsed by the workspace's own
   `xmlparser` (the stack XMP and XFDF use, so no dependency is added), under a depth and node
   budget of its own. The template is read into a `Template` of containers (`subform`, `field`,
   `exclGroup`) with their names, their `presence`, and their automation children (`calculate`,
   `validate`, `event`, `format`, `script`); the `datasets` packet into the data it holds; the
   `config` packet for the settings XFA 3.3 chapter 18 lets a template's processing depend on.
   **An XFAF form is recognised by the flag and page 333's test** — `/NeedsRendering` false, no
   `occur` whose bounds differ, no `subformSet` — and not by the mere presence of a name section
   2.2's table lists, which every hybrid in section 4 has; a document outside it is reported as the
   dynamic half's and is not half-run — trap 5's rule that unsupported input stays loud.
2. **The datasets packet kept true on save** — section 3.1's defect closed: `ViewState::save`
   writes a replacement `datasets` stream in which each field a person changed carries the value
   written to its `/V`, found through the SOM name that ties the two (XFA 3.3 pages 72–73), and
   leaves every other byte of the packet as the producer wrote it. The array form replaces one
   packet's stream; the single-stream form replaces the one stream. This round's verdict is
   §K.2's sentence, tested in both directions: a filled, saved hybrid file whose `datasets` agree
   with its `/V`, and a file this tree did not change that is written byte for byte as before.
3. **Formatting, with no engine** — the `format` and `picture` elements, through XFA 3.3's picture
   clauses (chapter 26, pages 1150–1186) and its canonical formats (chapter 24, pages 1045–1048),
   the way RFC 0008's Tier 0 serves the `AF*` library: a value is a value, the oracle can compare
   it, and nothing is evaluated. `doc/todo/56` already notes that the picture clause is the only
   locale machinery any held text specifies.
4. **Validations without a script** — the `validate` element's null test and format test (chapter
   10, *Validations*, pages 380–386), reported to the person the way XFA 3.3 chapter 14's *User
   Experience of Validation* describes and the four windows already report a commit a field's
   `/K` refuses (ADR 1592).
5. **Calculations, validations and events in JavaScript** — through `pdf-script`, behind RFC
   0008's order and under the reader's `Scripts` level, `off` by default: a script element whose
   content type is JavaScript runs in the same realm as the document's other scripts, with an
   `xfa` host object of the XFA-SOM the template needs (chapter 3, *Scripting Object Model*,
   pages 86–121) where `pdf-script`'s surface today lists `xfa` as absent, and the activation order
   chapter 10 states: on one stimulus events first, then calculations, then validations (page 379
   and pages 404–407). A write to a field is an edit in the outcome, which the view state applies
   beside a person's typing — and which step 2 then writes into `datasets` as well as `/V`.
6. **FormCalc** — the language XFA 3.3 makes the one an XFA processor must support, and the
   default when a script states no content type (chapter 11, *Selecting a Script Language*, page
   412); JavaScript is Adobe's addition. **A static half without it has served only the forms
   whose producer chose JavaScript**, so it is owed, and it has no engine anywhere this tree can
   take one from: its grammar (chapter 25, pages 1049–1080) and its built-in functions (pages
   1081–1149) are a language to write — an interpreter in Rust with `#![forbid(unsafe_code)]`, run
   in the confined script worker beside Boa under the same budgets and the same `Scripts` level,
   because what it evaluates is still a document's code.

**What the static half does not need**: a layout engine, a typesetter, a page that was not the
producer's, or a font this program chooses. A field draws through the AcroForm widget's appearance
and §12.7.4.3's variable text, as it does today; XFA 3.3 page 74 says an XFAF field's own `x` and
`y` place it, and §K.2's consistency rule binds the writer to put the widget where the template
does, so the widget's `/Rect` is drawn and a file whose two disagree is the writer's defect, said
by a report rather than repaired by a guess.

## 6. The dynamic half

A dynamic form's pages do not exist until a processor makes them. XFA 3.3 chapter 8 (pages
274–332) gives the layout of growable containers — text placed in a container that grows, flowed
layout (`tb`, `lr-tb`, `rl-tb`, `row`, `table`), content split across a page boundary, pagination
strategies, adhesion, leaders and trailers, tables — and chapter 9 (pages 333–370) the data binding
that decides how many instances of each subform there are and the layout that follows from it. The
boilerplate is the template's `draw` elements: text, images and lines the processor sets at run
time, text **laid out from font metrics**, not from glyphs a producer positioned. And Table 29 puts
the whole of it at the open: `/NeedsRendering` asks for the regeneration "when the document is
first opened".

**Why it is the authoring line's middle and not its edge.** `CLAUDE.md`'s test is *does the
operation invent marks?*, sharpened by ADR 1120 to the marks' provenance. Every mark a dynamic form
shows is named by its producer — a `draw`'s text, a field's value, a border — so the provenance half
of the test is met. What is not met is the second half ADR 1123 states: *a clause fixes the
position*. Here nothing does. XFA 3.3's layout algorithm is the position, and running it is the
thing the exclusion was written to keep out: "no layout engine, no text-setting". A generated page
also has no producer's bytes for `raster_golden` or the oracle to hold it against, so its
correctness becomes what this program drew last time unless the reference is something else — and
the only something else in the world is Adobe's own output, which principle 5 refuses as a target.

**What it would cost**, written down:

- **The launch path.** `/NeedsRendering` puts template parsing, data binding and layout between
  open and page one. *Nothing eager* and *incremental parsing* still bind, so the layout is per
  page and lazy; whether page one of a flowed form can be laid out without laying out what precedes
  it is a property of XFA 3.3's pagination, and an implementation round measures it first.
- **The oracle.** A page with no reference is compared against an earlier output of this program,
  by name, as ADR 1014 holds its appended page; the comparison says *unchanged*, never *correct*.
- **Typesetting.** Text set from metrics, line-breaking, the rich text of chapter 27 that
  `pdf_model::rich_text` already lays out for §12.7.4.3 (ADR 1623), and fonts the template names
  that the PDF may not embed — `CLAUDE.md`'s rule on system fonts applies.
- **The surface.** A layout DOM (chapter 8, *The Layout DOM*), data binding (chapter 4 pages
  171–213 and chapter 9), and the oracle's and the fuzzer's reach extended to both.

## 7. The amendment, asked for by name

**Q639 asks the owner to amend the authoring exclusion with a fifth near-side construction**, worded
in the shape of the four before it:

> **A page laid out by a producer's own XFA template.** Where a document's interactive form
> carries an XFA resource whose template is not XFAF — Table 29's `/NeedsRendering` is `true`, or
> the template states an `occur`, a `subformSet` or other grammar XFA 3.3 excludes from XFAF —
> the pages that template generates, laid out by XFA 3.3's own layout and binding rules from the
> template and the document's own data, are on the near side. Every mark on such a page is one
> the template names; what this program decides is only what XFA 3.3's algorithm decides. Nothing
> may be placed on it that the template and its data do not state, and a page so laid out is said
> to be generated, in the window's title or report, as an appended page is.

The test the construction passes and the one it does not, stated rather than hidden: it passes
ADR 1120's provenance test — every mark is the producer's — and **fails ADR 1123's second part**,
because no clause of ISO 32000-2 fixes the positions; XFA 3.3, a text ISO 32000-2 names as the
format of the resource (§K.2), fixes them instead. That is the sentence the owner is asked to
accept or refuse: *a normative reference's layout algorithm may stand where a clause's position
would*. What stays on the far side whatever the answer: a watermark, a stamp, text this program
composes, HTML to PDF, and any layout whose source is not the document's own template.

**Meanwhile**: nothing of the dynamic half is built; a `/NeedsRendering` document shows the page
its producer wrote for processors that decline, and every window says why (ADR 1932). The static
half needs no amendment and proceeds at one round per batch.

## 8. Prior art, as evidence of demand only

- **pdf.js** implements both halves in `src/core/xfa/` (RFC 0009 section 4.3 counted 16 379
  lines in 27 files): a template parser, a data binder, a layout engine and an HTML emitter, and a
  FormCalc parser. Its existence says forms in the world need both; it is no reading of XFA 3.3.
- **`pdf-xfa`** 1.2.0, the one Rust XFA stack, is AGPL-3.0-only or commercial and fails
  `deny.toml` (review 1768); nothing here would take it.
- **Adobe** writes the forms (Designer, LiveCycle) and reads them (Acrobat, Reader). XFA 3.3's
  chapter 28 lists where Adobe's implementation departs from the specification, which is the
  register in which Adobe's behaviour enters this tree: a recorded departure, never a target.

## 9. Easy and difficult, against this tree

| step | what is already here | what is new | difficulty |
|---|---|---|---|
| resource read | `xfa::stated`, `xmlparser`, the stream budgets | the template and data models | small |
| datasets on save | `ViewState::save`, §7.5.6's update | a packet rewritten by SOM name | small; a defect today |
| picture clauses | Tier 0's `AF*` formats as precedent | chapter 26's grammar | medium |
| validations without script | the commit refusal in four windows | the null and format tests | small |
| JavaScript behaviours | `pdf-script`, Boa, the confined worker, the `Scripts` level | an `xfa` SOM host object, the activation order | medium |
| FormCalc | the worker, the budgets | an interpreter and its built-ins | large |
| dynamic layout | rich-text layout for §12.7.4.3 | layout DOM, binding, pagination, typesetting, the oracle's reference | very large, and amendment-gated |

## 10. The current restrictions, each with its rationale, and the unconstrained design

- **`CLAUDE.md`'s XFA exclusion**, on §K.1's permission. Its rationale was the effort, which the
  owner has now chosen to spend at a pace. The unconstrained design is the whole of XFA 3.3: this
  RFC proposes the static half under the exclusion's own words (it lays nothing out) and the
  dynamic half under Q639's amendment.
- **The authoring exclusion**, which protects the oracle's comparison, the scope of the correctness
  claim and the project's shape (RFC 0009 section 5.3). The dynamic half is a layout engine, so it
  needs section 7's amendment; the static half composes nothing and needs none.
- **The script exclusion**, which admits a script as far as this document and this viewer reach.
  XFA's scripts reach the form's own values and presence, so they sit inside it; the parts of XFA
  that leave the process — chapter 13's submissions, web services and database connections,
  chapter 16's signed submissions — are excluded exactly as RFC 0008's Tier 2 is, each refused by
  name.

## 11. The ledger

Annex K's three rows stay `out-of-scope`; their notes point at this RFC. When the static half's
first steps land the rows do not move to `implemented` — §K.1's permission was taken for the whole
annex — but to `partial` once the owner has ratified this RFC and `CLAUDE.md`'s XFA entry says the
static half is in scope; that is the owner's edit, and the status follows it. §K.2's writer rules
(section 3.1) are owed as soon as this tree writes a hybrid file, which it already does.

## 12. Questions for the owner

1. **Q639: the fifth near-side construction** (section 7) — a page laid out by a producer's own
   XFA template. *Recommendation*: accept it **later and separately**, once the static half is in
   and its rounds have shown what the layout costs at the open; the amendment is asked now so that
   no round assumes it.
2. **The static half under the XFA exclusion's words** — is `CLAUDE.md`'s XFA entry to be
   reworded so that a static form's behaviours are in scope, with the dynamic half still excluded
   until question 1 is answered? *Recommendation*: yes; the wording is the owner's.
3. **FormCalc as a second language in the script worker** — *Recommendation*: yes, in Rust, after
   the JavaScript behaviours, since XFA 3.3 makes it the one required language.

## 13. What is built when

1. **Now**: this RFC, the census in section 4, the Annex K notes pointing here, Q639.
2. **The next XFA round, and it needs no answer first**: section 5's steps 1 and 2 together — the
   resource read into a template and its data, and `ViewState::save` writing the `datasets` packet
   consistent with every `/V` it writes, judged on §K.2's sentence over the census's hybrid files
   filled and saved, and on a file this tree did not change being written byte for byte as before.
   Its record names the step it leaves next.
   **Built** (session 1781, ADR 2396): `pdf_model::xfa::Resource` reads both of §K.2's forms into
   the template's containers and the `datasets` packet, and `ViewState::save` splices the data value
   each changed field binds; the four hybrids of section 4 fill, save and read back consistent
   (`crates/pdf-model/tests/xfa_datasets.rs`). What it names rather than writes — a numeric or date
   field's canonical form, a dynamic form, a binding it does not follow — is `Written::xfa_unkept`,
   and the first of those is step 3's. Step 3 is next.
3. **Then, one round per batch**: steps 3 to 6 in order, each with its ledger note.
4. **On Q639's answer**: the dynamic half, whose first round is a measurement — the cost of laying
   out page one of the census's `/NeedsRendering` documents — before a line of layout ships.

## 14. Sources

- ISO 32000-2:2020, Annex K (K.1, K.2), Table 29, Table 224, §7.5.6, §12.7.4.3 —
  `doc/md/ISO_32000-2_sponsored_EC3.md`, with the PDF for §K.2's sentence the conversion cuts.
- XFA 3.3 (9 January 2012) — `doc/md/XFA-3_3.md`: chapter 1 pages 26 and 28–30; chapter 2 pages
  31–32 and 72–74; chapter 3 pages 86–121; chapter 7 pages 248 and 272–273; chapters 8 and 9 pages
  274–370; chapter 10 pages 378–409; chapter 11 page 412; chapter 17 page 630; chapters 24–26.
- RFC 0008, RFC 0009 section 4, review 1768 (the `pdf-xfa` licence), ADRs 1014, 1120, 1123, 1623,
  1932; `doc/questions/A254`, `Q633`, `A633`.

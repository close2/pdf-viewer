# RFC 0008 — A script is a document acting on its reader: what running one would mean, where the line is, and which engine

Status: **proposed** — ready for the owner. Nothing in this document changes a ledger status, a
`Cargo.toml`, a line of `CLAUDE.md` or a line of code that ships; the round that wrote it added one
read-only instrument (`crates/pdf-model/examples/javascript_census.rs`) and no dependency.
Round: 1296 — commissioned by the owner on 2026-09-30: *"use one round to create an RFC for adding
JavaScript. what would it mean, where should we draw the line, which library..."*
Builds on: `doc/todo/56`, the 2026-08-28 finding that a memory-safe engine exists and the owner's
decision of the same day on the source of the host object model. Where this RFC repeats a number
from that file it re-measured it; where it disagrees, §9 says so.
Companions: RFC 0007 (a policy asked once, in advance — the shape §6.3 reuses), ADR 1155 and ADR
1291 (the four levels over a link and a submission, the two nearest neighbours of a script), ADR
0014 and ADR 0713 (the confined worker a script would live in), ADR 1122 (a value drawn plain is a
departure said out loud — the shape a script's absence already has).

`§N` in this document is a clause of ISO 32000-2 and nothing else. ISO 21757-1 is written "ISO
21757-1 section N"; Adobe's reference is named by its title and the page's heading, never with a
section sign; neither is quoted, for the reasons `doc/todo/56` §3 gives — Adobe's words are a
vendor's under MIT, and a quotation mark in this tree means *verbatim from `doc/md/`*.

---

## 0. What is wrong today, in one paragraph

A form whose total field carries `AFSimple_Calculate("SUM", "Line1, Line2, Line3")` opens in this
viewer, accepts a value in each line, and never changes the total. A currency field stores `1234.5`
and shows `1234.5`, because the `/AA /F` that would make it read `$1,234.50` is refused with the
sentence `JavaScript: excluded by CLAUDE.md principle 5`. A validation script that would refuse a
date in the past refuses nothing. None of this is a defect in the tree — every one of those
refusals is named, reported and deliberate — and all of it is what a person filling the form sees.
`CLAUDE.md`'s exclusion says why: *"a sandboxed script engine is a separate project with its own
security argument"*. That sentence was written when this tree had no sandbox. It now has two
confined workers, a four-level policy shape for everything a document asserts over its reader, and
an edit log beside an immutable document. The security argument is no longer a separate project;
it is a placement question, and this RFC works it out so that the owner can decide it.

## 1. The reframe

**A script is the sharpest case of a document acting on its reader**, and the tree already has a
rule for that case. `CLAUDE.md` principle 3's second heading: *a document's restrictions are the
reader's to set, and they have levels* — `off`, `on`, *ask*, *warn* — asked once, in a place a host
supplies. A `/P` flag says what the reader may not do; a script says what the document *will* do,
on open, on a keystroke, before a print. It is the same direction of assertion with more force, and
it belongs under the same shape. Everything in §6 follows from putting it there.

Three things the reframe settles before any design:

1. **The standard specifies the dispatch and not one API name.** §12.6.4.17 says *when* a script
   runs and §12.6.3 says *from where*; the contents are ISO 21757-1's, which the owner has decided
   not to buy, and the host object model is read from Adobe's reference as a documented choice
   (`doc/todo/56` §3, the owner's words). So the ledger rows this RFC moves are about *dispatch* —
   the `shall`s below — and every API member is a choice in principle 5's own sense.
2. **"No engine" and "an engine" are not the only two positions.** Most real form scripts are a
   single call into a library the standard does not define and Acrobat ships: `AFNumber_Format`,
   `AFDate_FormatEx`, `AFSimple_Calculate`. A native implementation of that library, dispatched on
   the call's text with no ECMAScript at all, serves those documents and puts nothing new in the
   process. That is Tier 0 in §4, and it is why the line is drawn in three places rather than one.
3. **The oracle's comparison is untouched**, because a script's effects are edits in the log
   beside `pdf_syntax::Document` — like a person's — and the oracle runs with scripts `off`.
   `interpret` stays a pure function of the file, the view state, and what the user *or a script*
   did; the document never changes.

## 2. What it would mean — every site the standard names, and what a reader gets from each

The sites, in the standard's own words. Each `shall` is addressed to a PDF processor, and each is
today refused by name in `crates/pdf-model/src/action.rs` (`refused`), in
`crates/pdf-model/src/requirements.rs` (`EnableJavaScripts`) and in
`crates/pdf-model/src/forms_data.rs` (an FDF's `/JavaScript`).

### 2.1 The action, and the document-level library

§12.6.4.17:

> Upon invocation of an ECMAScript action, a PDF processor shall execute a script that is written
> in the ECMAScript programming language.

> Depending on the nature of the script, various interactive form fields in the document may
> update their values or change their visual appearances.

> When the document is opened, all of the actions in this name tree shall be executed, defining
> ECMAScript functions for use by other scripts in the document.

and Table 221's `/JS`: "(Required) A text string or text stream containing the ECMAScript script to
be executed." Table 32's `/JavaScript` entry is that name tree — "A name tree mapping name strings
to document- level ECMAScript actions". **What a reader gets**: the function library every field
script calls (`function total() { … }` defined once, called from six calculate scripts), and the
document that hides or shows fields on open (`this.getField("Section B").display = display.hidden`).
**The cost that comes with it**: the tree is `shall`-executed *on open*, so it is on the launch path
of every scripted document; §6.6 says where it goes so that time-to-first-page does not pay for it.

### 2.2 The document's own events — Table 200

§12.6.3's Table 200, five entries, each "An ECMAScript action that shall be performed" — `/WC`
"before closing a document", `/WS` "before saving a document", `/DS` "after saving a document",
`/WP` "before printing a document", `/DP` "after printing a document". **What a reader gets**: a
print hook that fills a "printed on" field, a save hook that clears a draft watermark field. **What
this tree can honour**: this program writes only §7.5.6's incremental update and prints nothing yet
(RFC 0004 is a draft), so `/WS` and `/DS` have a site and `/WP` and `/DP` do not until printing
exists; `/WC` has one.

### 2.3 A page's events — Table 198

Table 198's `/O`: "(Optional; PDF 1.2) An action that shall be performed when the page is opened
(for example, when the user navigates to it from the next or previous page or by means of a link
annotation or outline item). This action is independent of any that may be defined by the
OpenAction entry in the document catalog dictionary (see 7.7.2, "Document catalog dictionary") and
shall be executed after such an action." and `/C`, "when the page is closed". **What a reader
gets**: a page that resets a quiz field when re-entered. **What the tree already does**: reads both
(`action::for_page`) and performs what they name; a script action among them is refused by name.

### 2.4 An annotation's events — Table 197

Ten entries, `/E` `/X` `/D` `/U` `/Fo` `/Bl` `/PO` `/PC` `/PV` `/PI`, each "An action that shall be
performed" on enter, exit, down, up, focus, blur, page open, page close, page visible and page
invisible — any action type, and in the corpus very often a script. Table 197's own ordering
sentence for `/PO`: the action "shall be executed after the O action in the page's additional -
actions dictionary" and the catalog's `/OpenAction`, "if such actions are present"; `/PC` "shall be
executed before the C action". **What a reader gets**: a rollover that highlights a field on
enter, a button whose mouse-up runs the form's submit logic or resets it. **What the tree already
does**: raises all ten (`action::for_annotation`, `viewer-core`'s pointer and page-turn events —
the ledger's §12.6.3 row) and performs a non-script action at each; the plumbing exists, the
payload does not.

### 2.5 A field's events — Table 199, all four scripts by the table's own text

> An ECMAScript action that shall be performed when the user modifies a character in a text field
> or combo box or modifies the selection in a scrollable list box. This action may check the added
> text for validity and reject or modify it.

> An ECMAScript action that shall be performed before the field is formatted to display its value.
> This action may modify the field's value before formatting.

> An ECMAScript action that shall be performed when the field's value is changed. This action may
> check the new value for validity. (The name V stands for "validate.")

> An ECMAScript action that shall be performed to recalculate the value of this field when that of
> another field changes. (The name C stands for "calculate." ) The order in which the document's
> fields are recalculated shall be defined by the CO entry in the interactive form dictionary (see
> 12.7.3, "Interactive form dictionary").

And §12.6.3's own warning that these are not as narrow as their names, NOTE 2: "The effects of an
action triggered by one of these events are limited only by the action itself and can occur outside
the described scope of the event." — followed by the sentence that describes a form working: "For
example, the user's modifying a field value can trigger a cascade of calculations and further
formatting and validation for other fields in the document." **What a reader gets**: the four
things a filled form does that a drawn form does not — a keystroke mask (`/K`), a formatted display
(`/F`), a refused value with a message (`/V`), and a total that follows its lines (`/C`). §12.7.4.3's
NOTE already names the third as a reason appearances are built at viewing time: "fields containing
current dates or values calculated by an ECMAScript".

### 2.6 The calculation order — Table 224's `/CO`

> (Required if any fields in the document have additional- actions dictionaries containing a C
> entry; PDF 1.3) An array of indirect references to field dictionaries with calculation actions,
> defining the calculation order in which their values will be recalculated when the value of any
> field changes (see 12.6.3, "Trigger events").

**What the tree does today**: `crates/pdf-transform/src/merge.rs` *carries* `/CO` — a merge
concatenates the sources' orders in input order — and nothing computes it; the ledger's §12.7.3 row
says so in as many words ("the first time this tree writes the entry it still does not execute").
Round 1294 noticed the same. A calculation order is the standard making the *document* state the
dependency order rather than leaving the reader to infer one, which is exactly the property that
makes `/C` implementable without a dependency analysis: run the array in order, once, after any
value changes.

### 2.7 Two clauses that bind a processor which *does* run scripts

§12.11.1: "A PDF processor that supports document requirements shall evaluate them before execution
of any ECMAScripts." — an ordering obligation this tree meets today by construction (nothing runs)
and would have to *schedule* once something does; and Table 274's `EnableJavaScripts`: "Requires
support for execution of ECMAScripts appearing in ECMAScript actions and in the ECMAScript name
tree for document-level ECMAScripts." — a requirement this program currently declines with a
sentence and would then meet. §12.11.5's `/RH` handler — "The name of a document-level ECMAScript
action stored in the document name dictionary" that "shall disable execution of the requirement
handler" — becomes an entry to read rather than one the exclusion covers whole.

And the standard's one acknowledgement that a processor may be unable: §12.6.4.14's rendition
action, Table 218: "Either the JS entry or the OP entry shall be present. If both are present, OP is
considered a fallback that shall be executed if the interactive PDF processor is unable to execute
ECMAScripts." A rendition is clause 13's and stays excluded; the sentence is quoted because an
honest argument puts it in front of the owner.

### 2.8 The library the standard does not define, and most forms call

A field's `/AA /F` in a real document is, far more often than not, one call:
`AFNumber_Format(2, 0, 0, 0, "$", true)`, `AFDate_FormatEx("mm/dd/yyyy")`,
`AFPercent_Format(2, 0)`, `AFSpecial_Format(2)`; its `/K` is the matching `_Keystroke`; its `/C` is
`AFSimple_Calculate("SUM", …)`; its `/V` is `AFRange_Validate(true, 0, true, 100)`. **These are not
ECMAScript and not ISO 21757-1's**: Adobe supplies them from a script library shipped with Acrobat
(`AForm.js` in an installation's JavaScripts folder), the *JavaScript for Acrobat API Reference*
does not document them (measured in `doc/todo/56`: no `AF*` heading in either of its two pages), and
`pdf-association/pdf-issues` #100 — a reporter listing some ninety of these names that existing
files call — was closed *wontfix*, with Adobe's position on the thread that they are private methods
of the products and not for standardisation. The only published description is a parameter menu
without an algorithm, in Adobe's *Interapplication Communication API Reference* under the Acrobat
Forms plug-in's `SetJavaScriptAction`.

So **what the tree would write of its own** is every line of that library: the number parser and
its rounding, the separator styles, the negative styles (`0`–`3`), the fourteen date formats and
their month names, `AFSpecial`'s zip, zip+4, phone and SSN masks, `AFRange`'s two bounds,
`AFSimple`'s five functions (`AVG SUM PRD MIN MAX`) and how a field name with a trailing `.` names a
whole subtree, `AFMergeChange`, `AFMakeNumber`, `AFExtractNums`, `AFParseDateEx`. For scale, and as
evidence rather than a target: pdf.js's `src/scripting_api/aform.js` is 627 lines. Every rule in it
is a documented choice under principle 5, and it is the largest block of documented-choice code this
tree would ever take on. §4's Tier 0 is that library, and §10's question 2 is whether it is built
first.

### 2.9 What the tree already does with all of the above

Measured by `grep -rn "JavaScript\|AFNumber\|AFDate" crates/ --include=*.rs` on 2026-09-30, and read:

- `action.rs` refuses `/S /JavaScript` by name (one sentence, `refused`), and `ViewState::perform`
  does nothing with a `Refused` — the standing shape of trap 5: loud, not silent.
- `requirements.rs` refuses `EnableJavaScripts` with "ECMAScript is excluded by this project's
  principle 5", and the penalty arithmetic of §12.11.3 is computed over it.
- `forms_data.rs` refuses an FDF's `/JavaScript` (Table 246) by name on import.
- `pdf-archive` and `pdf-transform` *remove* script actions and the name tree on the way to PDF/A-2
  (ISO 19005-2 section 6.6.1 forbids them) and keep them for PDF/A-4 where its section 6.6.2 permits
  them — the writer side already knows every site §2.1–§2.5 names, because it has to find them to
  strip them.
- `merge.rs` carries `/CO`; `appearance.rs` reads a *line annotation's* `/CO` (a caption offset,
  §12.5.6.7) — the same key, unrelated.
- **ADR 1122 is the precedent for what a script's absence looks like when it is said out loud**: a
  rich-text value drawn plain is reported as a departure on the entry that carries the formatting,
  rather than drawn silently. A field whose `/F` is refused is the same shape — the raw value is
  drawn *correctly per the appearance stream the file carries*, and the refusal names what was not
  run. That is the state this RFC would replace with the thing itself.

The ledger, on the same day: §12.6.4.17 is the tree's **only** `script-behaviour` exclusion row;
§12.6.3, §12.7.3, §7.7.4 and §12.6.4 are `implemented` with the script half named in each note as
excluded rather than owed; §12.11.5 is `out-of-scope` on the argument that a handler is a script and
nothing runs; §12.11.1 and §12.11.2 are `implemented` because nothing runs. §7 says which move.

## 3. The census — what the world's scripts actually do, on this date

`doc/todo/56` measured *how many* documents carry a script (`refused_action_census`,
`witness_census`) and left two things uncounted: **at which site** each hangs, and **what it
calls**. Both decide where the line goes, so this round built the instrument and ran it once, over
every corpus `tools/state.sh` and `doc/oracle-and-corpus.md` name plus the two crawls under
`corpus-cache/`:

```sh
find -L doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf' > scratchpad/paths
find -L doc/corpora corpus-cache/openpreserve corpus-cache/tika-issue-tracker corpus-cache/safedocs \
     -name '*.pdf' >> scratchpad/paths
cargo build --release -p pdf-model --example javascript_census
RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 8 --tree 12 -- \
    target/release/examples/javascript_census @scratchpad/paths
```

The instrument is `crates/pdf-model/examples/javascript_census.rs`. It walks every object the
cross-reference table lists, classifies each `/AA` key by the dictionary that holds it (a catalog's,
a page's, a field's or an annotation's), follows each `/A`, `/OpenAction` and Table 32 name-tree
entry through its `/Next` chain, and runs a **tokeniser** — never an engine — over each distinct
script: an identifier chain followed by `(` is a call, a chain rooted at a host object is a
property use, and a `.name(` on a result is a method. Regular-expression literals are not
recognised, which adds a spurious name now and then and removes none; comments and strings are
skipped. Every figure below is **what this run found on 2026-09-30**, and the command above
reproduces it. `CLAUDE.md`'s rule applies: the number lives in the instrument's output, and this
section is a reading of one run.

### 3.1 The population

90 763 files named; **90 319 opened**, 423 did not, and 21 over 128 MiB (15 in the SafeDocs crawl,
6 in the Tika batches) were counted rather than read, because the first walk died at the data
limit on a six-gigabyte fuzzing artefact and a census that dies on one document measures nothing.
**929 carry a script at a site the census classifies — 1.03 %**; 947 state a `/S /JavaScript`
dictionary somewhere, so 18 hang at no site the standard's tables name (a `/A` on something that is
neither annotation nor outline item, or a `/JS` the census could not read).

| corpus | opened | scripted | share |
|---|---|---|---|
| SafeDocs crawl | 65 705 | 267 | **0.41 %** |
| Tika issue tracker | 22 900 | 594 | 2.59 % |
| `doc/corpora` | 487 | 8 | 1.64 % |
| openpreserve | 264 | 4 | 1.52 % |
| pdf.js test files | 963 | 56 | 5.82 % |

The crawl is the honest denominator for robustness; the two trackers over-sample by construction
(a viewer's regression suite; the attachments of bug reports), and the ratio between them and the
crawl — six to fourteen times — is the measurement of that bias. Both are quoted.

### 3.2 The sites

| documents | distinct scripts | site |
|---|---|---|
| **522** | 1 538 | Table 32's name tree — the document-level library |
| 328 | 928 | field `/AA /K`, keystroke (Table 199) |
| 322 | 757 | field `/AA /F`, format (Table 199) |
| 169 | 1 958 | a widget annotation's own `/A` |
| 96 | 1 102 | field `/AA /C`, calculate (Table 199) |
| 68 | 179 | field `/AA /V`, validate (Table 199) |
| 64 | 448 | annotation `/AA /Bl`, blur (Table 197) |
| 54 | 68 | page `/AA /O`, open (Table 198) |
| 48 | 50 | the catalog's `/OpenAction` |
| 43 | 143 | annotation `/AA /Fo`, focus |
| 37 | 176 | annotation `/AA /D`, down |
| 33 | 1 885 | a link annotation's `/A` |
| 23 / 23 / 16 / 10 / 5 | 23 / 23 / 16 / 10 / 5 | catalog `/AA /WC`, `/WP`, `/DP`, `/WS`, `/DS` (Table 200) |
| 23 / 22 / 17 / 12 / 8 / 8 / 5 | 275 / 80 / 56 / 26 / 20 / 15 / 11 | annotation `/AA /U`, `/E`, `/X`, `/PO`, `/PC`, `/PV`, `/PI` |
| 15 | 2 104 | an outline item's `/A` — fifteen documents, two thousand scripts: one producer's outline |
| 8 | 12 | page `/AA /C`, close |
| 4 | 4 | an `/AA` key no table defines |
| 1 | 18 | a rendition action's `/JS` (Table 218) |

Three readings. **The document-level library is the commonest site**, in 56 % of scripted
documents — which is why §6.6's placement of the open sequence matters more than any field
trigger's. **The field triggers are next** — `/K` and `/F` in a third of scripted documents each,
`/C` in a tenth, `/V` in fewer — and they are the four sites Tier 0 serves. **`doc/todo/56` §4.2's
second bullet is confirmed and sharpened**: most scripts reach a reader through the name tree, a
widget's or link's own `/A`, or `/OpenAction`, not through an `/AA`.

Distinct scripts per document: 214 carry one, 489 two to five, 142 six to twenty, 84 more; 5.65 MB
of distinct script text in all, the largest single script 250 400 bytes (`PDFBOX-4453-0.pdf`), 456
documents storing a script as a stream, none cut at the tokeniser's 1 MiB bound.

### 3.3 The verdicts, over the 929

| documents | share | verdict |
|---|---|---|
| 176 | **18.95 %** | every script is one `AF*` call and nothing else — **Tier 0 alone serves them** |
| 328 | **35.31 %** | every call is `AF*`, Tier 1, an ECMAScript built-in, a function the document defines, or a method on a result — **inside the candidate core** |
| 460 | 49.52 % | call at least one Tier 2 name — see the next paragraph before reading this as half |
| 141 | 15.18 % | call a name in neither list — the tail, §3.6 |
| 14 | 1.51 % | use `eval`, an arrow function, `let`/`const` or `class` |

The Tier 2 half is four calls: `this.getURL` (295 documents), `app.launchURL` (172), `this.print`
(75), `this.submitForm` (23). Two of those become a *request* under an existing policy (§4.4) and
one waits on RFC 0004; only the rest are refused outright, and their rows sum to **at most 110
documents (11.8 %)** — `buttonImportIcon` 20, `app.popUpMenu` 13, `this.closeDoc` 10, `print` 10,
`exportDataObject` 8, `execDialog` 7, `execMenuItem` 7, `getPrintParams` 6, `setAction` 5,
`app.openDoc` 4, and a tail of twos and ones down to one document that calls `Net.HTTP.request`,
`app.beginPriv` and `app.trustedFunction` together. That bound is derived from this run's table
(overlaps between rows are not visible in it); the instrument prints the exact figure as its own
line on the next run, added after this one. **So the honest reading is: a fifth of scripted
documents need no engine, a third stay inside the core, about a tenth call something this RFC
refuses outright, and the remaining share is one script — §3.4.**

The language: `this` in 596 documents, `if` 531, `new` 399, `typeof` 301, `var` 277, `for` 153,
`function` 153, `return` 76, `while` 43, `try`/`catch` 37, `switch` 19, `do` 4, `throw` 4, `with`
1 — and ES2015 or later in 14. **The world's form scripts are ES3-shaped**, which is the finding
§5.2 prices the self-written interpreter against.

### 3.4 The commonest script in the world is Adobe's own viewer check

`app.alert` in 432 documents; `app.viewerVersion` read in 344, `app.viewerType` 294, `app.language`
278, `app.platform` 277, `this.ADBE` 277, `app.findComponent` 269, `app.response` 273, `this.getURL`
295 — one producer's boilerplate, and one witness read to be sure (`0100888.pdf`, via `qpdf --qdf`):
a document-level script named `ADBE` that, on open, compares `app.viewerVersion` with 7.0 and
9.0, and in the failing branches puts an `app.response` or an `app.alert` to the reader and then
`app.launchURL`s Adobe's download page or `app.findComponent`s the XFA plug-in. It is LiveCycle
Designer's stub, and it is in roughly **300 of the 929**.

Two consequences, and both are decisions. **What `app.viewerType` and `app.viewerVersion` answer
decides whether three hundred documents put a dialog to the reader on open.** A truthful answer
(this program's name and version) takes the "old viewer" branch every time; §10's question 11 asks
the owner, and recommends the truthful answer under the dialog cap of §6.8 — a document that tells a
reader it wants Adobe Reader 9 has told them something true about itself, and a program that
answers to another's name has started curve-fitting one clause over. **And `this.getURL` and
`app.launchURL` in these documents are on that stub's *fallback* branch**, which is why §3.3's
Tier 2 half is not half.

### 3.5 The `AF*` family as the world writes it

`AFDate_FormatEx` in 207 documents, `AFDate_KeystrokeEx` 203, `AFNumber_Format` 151,
`AFNumber_Keystroke` 145, `AFSpecial_Format` 74, `AFSpecial_Keystroke` 74, `AFSimple_Calculate`
32 (523 occurrences), `AFMergeChange` 23, `AFRange_Validate` 20, `AFParseDateEx` 18,
`AFSpecial_KeystrokeEx` 15, and `AFMakeNumber` below the cut. That is Tier 0's list, ranked by the
world, and dates lead: the fourteen date formats and their month names are the first documented
choice Tier 0 makes.

### 3.6 Tier 1 as the world uses it, and the tail

`this.getField` in 182 documents and 12 279 occurrences (the most-called host method by far, with
bare `getField` in 56 more), `app.response` 273, `util.printd` 34, `this.getAnnots` 34,
`this.resetForm` 29, `app.beep` 24, `this.getNthFieldName` 19, `console.println` 15,
`this.calculateNow` 14, `util.printf` 12; on a field, `.setFocus` 62, `.isBoxChecked` 14,
`.checkThisBox` 12; the properties `event.value` 92, `display.hidden` 58, `display.visible` 51,
`event.rc` 50, `this.numPages` 43, `this.dirty` 37, `this.pageNum` 34, `event.change` 32,
`event.willCommit` 18, `event.commitKey` 8. Every one of these is in §4.2's table.

The tail (324 distinct names in 141 documents): `app.findComponent` 269 — Adobe's plug-in
installer, which belongs in Tier 2's list and is added to the instrument's; `syncAnnotScan` 34 — a
`Doc` method with no effect here, Tier 1; bare `setFocus`, `split`, `replace`, `toString` — a
chained call the tokeniser lost at a line break, ECMAScript's own; and the per-document functions
(`checkDate`, `calculateBalans`, `f_resetall`, `elfCheck`) that a document defines in its
name-tree library and calls from a field — defined, and counted as unknown only where the
definition is `var f = function () {}` rather than `function f()`. The tail is a tokeniser's
residue and a producer's vocabulary, not a third API.

## 4. Where the line is drawn — three tiers, a reason for each, and how an excluded call fails

The census ranks the demand and Adobe's privileged-context marker ranks the danger; neither is
the line. **The line is the process boundary**: a script may reach what the confined process
already holds — this document's values, this viewer's state — and nothing else, and the four
levels decide whether it runs at all.

### 4.1 Tier 0 — in scope with no engine: the `AF*` library, native, and `/CO` honoured

The library §2.8 describes, re-implemented in Rust as a set of documented choices, dispatched when a
field's `/AA` entry is *textually* one `AF*` call with literal arguments. No ECMAScript is parsed,
no engine is constructed, nothing new enters the process. `/CO` is honoured — the array run in
order once per value change — and `AFSimple_Calculate` is what runs at each entry.

*Reason*: it serves one scripted document in five outright (§3.3's first line) and sits on the
path of every document in the core, at zero security cost; and it is owed under any engine anyway,
because the engine cannot supply a library the standard does not define. It is also the only tier the oracle can compare: a formatted value is a
value, and `raster_golden` sees the field either way.

*What it is not*: a script engine. A `/F` that says `AFNumber_Format(2,0,0,0,"$",true)` runs; a
`/F` that says `if (event.value > 0) AFNumber_Format(…)` does not, and is reported as *a script
this tier does not run* — the same sentence as today, one tier narrower.

### 4.2 Tier 1 — an engine, confined: what reads and writes this document and this viewer

Every site §2.1–§2.6 names, under a host object model limited to what stays inside the process:

| object | admitted | not admitted (Tier 2) |
|---|---|---|
| `event` | every property Adobe's "event properties" lists — `value`, `change`, `changeEx`, `rc`, `willCommit`, `commitKey`, `selStart`, `selEnd`, `target`, `targetName`, `source`, `name`, `type`, `shift`, `modifier`, `keyDown`, `fieldFull`; `richValue`/`richChange` read-only | — |
| `Field` | value, appearance-relevant properties (`display`, `hidden`, `readonly`, `required`, `textColor`, `fillColor`, `strokeColor`, `borderStyle`, `lineWidth`, `textSize`, `textFont`, `alignment`, `charLimit`, `comb`, `multiline`, `password`, `doNotScroll`, `currentValueIndices`, `numItems`, `exportValues`), the choice methods (`getItemAt`, `setItems`, `insertItemAt`, `deleteItemAt`, `clearItems`), the button caption methods, `checkThisBox`, `isBoxChecked`, `setFocus`, `getArray`, `valueAsString`, `name`, `type`, `page`, `rect`, `doc` | `browseForFileToSubmit`, `buttonImportIcon`, `setAction`, `signature*`, `setLock`/`getLock` |
| `Doc` (`this`) | `getField`, `getNthFieldName`, `numFields`, `calculateNow`, `calculate`, `resetForm`, `dirty`, `getAnnot(s)`, `getOCGs`, `pageNum`, `numPages`, `getPageLabel`, `getPageBox`, `getPageRotation`, `gotoNamedDest`, `scroll`, `zoom`, `zoomType`, `layout`, `info` (read), `documentFileName`, `title`, `getPageNumWords`/`getPageNthWord` (this document's own text) | everything in Tier 2's `Doc` list |
| `app` | `alert`, `beep`, `response`, `setTimeOut`/`setInterval`/`clearTimeOut`/`clearInterval` (bounded to this document's lifetime), `viewerType`, `viewerVersion`, `viewerVariation`, `platform`, `language`, `formsVersion`, `activeDocs` (this document only), `goBack`/`goForward` | everything in Tier 2's `app` list |
| `util` | `printf`, `printd`, `printx`, `scand`, `crackURL`, `spansToXML`, `xmlToSpans`, `streamFromString`, `stringFromStream`, `iconStreamFromIcon` | `readFileIntoStream` |
| `color` | all of it | — |
| `global` | in-memory, per document | `setPersistent`, `subscribe` |
| `console` | `println`, `show`, `hide`, `clear` — a log the host may show; pdf-issues #744 says the object is in Adobe's reference and absent from ISO 21757-1, and the corpus uses it | — |

*Reason*: these are the things a person filling the form can already do by hand through the edit
log — type a value, clear a field, turn a page, switch a layer — plus a message. A script doing them
is a script acting *inside* the document on the reader's screen, which the four levels govern and
the process boundary contains.

`app.alert` and `app.response` are the two that need a host, and they get one the way every other
question does: an `Event` across the confined wire that a face with a dialogue answers and a face
without one refuses by name (ADR 0713's confined window; ADR 1155's `unanswerable`). A script that
alerts in a loop meets §6.8's coalescing — at most one dialog per trigger, the rest reported.

### 4.3 Tier 2 — excluded, each with its reason, and how each fails

| call | why it is outside |
|---|---|
| `Net.HTTP.request`, `Net.Discovery.*`, `SOAP.*` | leaves the process. The confined worker has no network (seccomp), and the host performs no transmission a person did not see: ADR 1291 sends a form by `ureq` *from the host, after the level is read*, and a script is not a person |
| `app.launchURL` | leaves the machine. **Not refused outright**: it becomes a §12.6.4.8 URI request across the wire, under `viewer_host::Links` and its level (ADR 1155, `A67`: *ask* by default). A script may raise the same question a link raises, never more |
| `this.submitForm`, `Field.browseForFileToSubmit` | the first becomes a §12.7.6.2 submit request under `Submissions` and ADR 1291's client, with the script's arguments read the way Table 239's are; the second is a file chooser, which is ADR 1240's and is a person's act |
| `this.saveAs`, `exportDataObject`, `importDataObject`, `createDataObject`, `exportAs*`, `importAn*`, `importTextData`, `*XFAData`, `util.readFileIntoStream`, `app.browseForDoc`, `app.getPath` | the filesystem. The worker has none (Landlock), and a document that writes files by itself is the thing principle 3 exists to prevent |
| `app.openDoc`, `app.newDoc`, `app.newFDF`, `app.openFDF`, `this.closeDoc`, `app.activeDocs` beyond this one | other documents in the window. One document's script reaching another's state is the confused-deputy the confinement is built to make impossible; ADR 1275's "a document opened beside" is a person's act |
| `this.print`, `this.getPrintParams`, `/WP`, `/DP` | printing is RFC 0004's, not built; when it is, `print` becomes a request under that RFC's own policy |
| `this.mailDoc`, `this.mailForm`, `app.mailMsg`, `app.mailGetAddrs` | mail is a program on this machine started with an attachment; nothing here starts one |
| `app.execMenuItem`, `addMenuItem`, `addSubMenu`, `hideMenuItem`, `addToolButton`, `removeToolButton`, `hideToolbarButton`, `app.popUpMenu(Ex)`, `app.execDialog` | the application's own chrome and a dialog the document lays out; `doc/ui-boundary.md`'s rules are that a document does not draw the viewer |
| `app.trustedFunction`, `trustPropagatorFunction`, `beginPriv`, `endPriv`, and every method Adobe marks privileged | ISO 21757-1 section 9 makes a document's events non-privileged; there is no privileged context here to enter |
| `global.setPersistent`, `global.subscribe` | persistence across documents and sessions is a channel between files |
| `Collab.*`, `security.*`, `SecurityHandler`, `identity`, `this.encryptUsingPolicy`, `signature*` | collaboration servers, credentials, the reader's identity, and cryptography the document steers |
| `this.addScript`, `removeScript`, `addField`, `removeField`, `addAnnot`, `addLink`, `insertPages`, `deletePages`, `replacePages`, `extractPages`, `flattenPages`, `spawnPageFromTemplate`, `addWatermarkFrom*`, `setPageAction`, `setAction` | mutate the document's *structure* or invent marks. `pdf_syntax::Document` is immutable and the watermark is the standing example of the far side of `CLAUDE.md`'s authoring line |
| the 3D and rich-media objects, `Annot3D`, `AnnotRichMedia`, `app.media`, `Doc.media`, `screen` annotation methods | clause 13, excluded |
| `XFA`, `xfa`, `dynamicXFAForm` | Annex K, excluded on its own permission |

**How an excluded call fails, and this is a rule rather than a detail**: as an exception the script
can see — an `Error` whose `name` is `NotAllowedError` and whose message names the call and the
tier — *never* as `undefined` and never in silence. Principle 1's "no silent error swallowing" reads
the same way in a script as in Rust: a form whose button calls `this.submitForm(…)` after
`app.alert("Thank you")` shows the alert, throws at the submit, and the throw is reported **once per
document** with the call's name — which is exactly how such a form degrades in a Reader whose
administrator has turned the same calls off. A `try { … } catch` in the script catches it, which is
the script author's choice to make.

**What the engine may never reach, whatever the level**: the bytes of the file (the script sees
values, never objects; `this.getDataObjectContents` and friends are Tier 2); the filesystem and the
network (the process has neither); the other documents open in the window (the process holds one);
the clock beyond a monotonic timer with a budget (`Date` answers, and answers the same value twice
inside one trigger, so that a script cannot time the host); the host's environment, locale files or
fonts; and anything at all when the level is `off`.

### 4.4 The one place a Tier 2 call is *not* a refusal

`app.launchURL` and `this.submitForm` are listed above as *becoming a request* rather than throwing.
That is deliberate and it is the difference between this RFC and a blanket block: the URI action
and the submit-form action already exist, already cross the wire, and already have a level with a
default the owner chose (`A67`, `A98`). A script that asks for the same thing gets the same answer
through the same door — and a face that cannot ask (the confined window, a batch) refuses it the
way it refuses the action. What is refused outright is a route *around* those doors:
`Net.HTTP.request` is not a link, and `SOAP` is not a form.

## 5. Which library

Every figure below was **measured on 2026-09-30, on this machine, under a load average of 2.8**
(trap 36: a neighbour was fuzzing; the figures are ratios to read, not ceilings to gate on). The
program is `scratchpad/r1296/js-timing/` — a detached crate with one binary per engine that
constructs the runtime, evaluates `1+1`, evaluates a thousand-iteration loop, and then tries a
runaway loop and a five-million-element array under whatever budget the engine offers. Its listing
is kept beside this RFC as `doc/rfc/0008-a-script-is-a-document-acting-on-its-reader/js-timing.txt`
so that the number can be re-taken.

### 5.1 The table

| engine | crate, version | language | licence, `cargo deny` against `deny.toml` | ECMAScript | construct | `1+1` | 1 000-iteration loop | release binary of the probe | packages new to `Cargo.lock` | `unsafe` occurrences / lines | budgets the engine offers |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **Boa** | `boa_engine` 0.22.0 (2026-08-28), `default-features = false` | Rust | `Unlicense OR MIT` — **passes** (advisories, bans, licences) | ES2023-class; 95.2 % of test262 on 2026-08-28 (`doc/todo/56` §2.1) | 0.42–0.59 ms | 0.10 ms | 1.17 ms | 9.55 MB | **+57** of 128 | `boa_engine` 293 / 148 436, `boa_gc` 180 / 4 305, `boa_string` 96 / 5 036; no C | loop-iteration limit (works: `while(true){}` stopped in 9 ms), recursion limit (works, 0.1 ms), stack size; **no memory ceiling and no wall-clock interrupt** — `new Array(5e6).fill(0)` ran to completion in 680 ms under every limit set |
| **QuickJS** via `rquickjs` | `rquickjs` 0.14.0 (2026-09-18), `bindgen` feature | C (the engine), Rust (the binding) | `MIT` — **passes** | ES2023 (QuickJS-NG); 83.5 % of test262 | 0.30 ms | 0.045 ms | 0.12 ms | 1.89 MB | **+3** of 7 | `rquickjs-core` 670 / 22 274, `rquickjs-sys` 6 130 / 47 229 generated bindings, **108 296 lines of C** | wall-clock interrupt handler (works: stopped at 50.03 ms against a 50 ms deadline), memory limit (works: the 5e6 array refused in 5 ms under 8 MiB), stack limit |
| **Duktape** via `ducc` | `ducc` 0.1.5 (2020-07-10) | C | `MIT` — passes | ES5.1 with partial ES2015; last binding release six years ago | 0.25 ms | 0.027 ms | 0.31 ms | 1.01 MB | +3 of 3 | `ducc` 62 / 2 729, **100 960 lines of C** in `ducc-sys` | timeout callback, no memory ceiling in the binding |
| **V8** via `v8` | `v8` 152.2.0 (2026-08-20) | C++ | `MIT` — **fails advisories** (`paste` unmaintained, RUSTSEC) | current | 0.7 ms platform + 1.2–1.9 ms isolate and context (8.6 ms on the first cold run) | 0.05–0.09 ms | — | **64.7 MB** | +28 of 45 | a JIT in C++; a 36.6 MB crate before the prebuilt static library it downloads | heap constraints, `TerminateExecution` |
| **SpiderMonkey** via `mozjs` | `mozjs` 0.26.6 | C++ | `MPL-2.0` — **fails licences** (not on `deny.toml`'s list, and the file says why MPL is not there) | current | not measured: builds SpiderMonkey from source | — | — | — | — | a JIT in C++ | full |
| **Nova** | `nova_vm` 1.0.0 | Rust | `MPL-2.0` — **fails licences** | 77 % of test262 | not measured | — | — | — | — | 1 155 `unsafe` (`doc/todo/56`) | — |
| **A self-written ES5 interpreter** | — | Rust, `#![forbid(unsafe_code)]` | — | ES5 subset | — | — | — | — | 0 | 0 | whatever it is given |

The V8 probe built and ran (its 64.7 MB binary is the measurement that matters); it is not a
candidate for a process that parses hostile input, because a JIT in C++ that fails `cargo deny` on
an advisory before a line of it runs is out on `deny.toml` and principle 3 rather than on any
timing. Nor is SpiderMonkey, whose licence `deny.toml` declines by design. Neither is written down
here to be beaten; they are the two engines whose containment machinery is best.

### 5.2 The self-written interpreter, priced honestly

§3's keyword table says how much language real scripts use: `var`, `if`, `for`, `function`,
`return`, string and number methods, and almost nothing from 2015 onward. An ES5 interpreter with a
tracing collector in safe Rust — the only engine that would satisfy `#![forbid(unsafe_code)]` as
written — is a parser, a bytecode or tree-walker, a garbage collector, and the built-ins the corpus
calls: `String`, `Number`, `Math`, `Date`, `RegExp`, `Array`, `parseInt`/`parseFloat`. `RegExp` and
`Date` alone are each larger than the rest of the tree's parsers put together, and `Date` is the one
a form script uses most (`AFDate_*` is written in terms of it). The `bigint` precedent (the owner
chose reviewed cryptographic dependencies over in-tree arithmetic, 2026-08-14) points the other way,
and an engine is a larger and less testable thing than a bignum. **Priced: months of a round's time
for an ES5 subset, in exchange for a `forbid` the process boundary already gives the same guarantee
as.** Not recommended, and put in the table so that the owner sees it was priced rather than
dismissed.

### 5.3 Scored against principles 1–4 and the sandbox

| | Boa | QuickJS (`rquickjs`) | Duktape (`ducc`) |
|---|---|---|---|
| 1 quality | a quarter-million lines calling itself experimental, one DoS advisory (RUSTSEC-2024-0444), reachable panics (`doc/todo/56` §5's count) — wants `catch_unwind` or a process boundary | a small, old, well-read C engine with four memory-corruption CVEs in two years reachable from script input (`doc/todo/56` §1) | a binding unreleased since 2020; the C engine is maintained, the Rust side is not |
| 2 fast | 0.4 ms to construct, 0.1 ms to evaluate — invisible beside a page; 9.5 MB of binary | 0.3 ms / 0.045 ms; 1.9 MB | 0.25 ms / 0.03 ms; 1.0 MB |
| 3 secure | Rust, so a specification bug's ceiling is a panic rather than type confusion; **but no memory ceiling and no interrupt**, so the budgets are the process's (`RLIMIT_AS`, the channel's deadline) and nothing finer | C, so the ceiling is memory corruption — *inside a process that holds no file bytes, no descriptors and no network*; **and** a memory limit and a wall-clock interrupt that let one script be stopped without losing the worker | C, same argument, weaker language |
| 4 exemplary | a pure-Rust dependency graph a student can read; 57 new packages is the cost | one C dependency, justified in writing as principle 3 asks — the same shape as the JBIG2 and JPX decoders under ADR 0014 | an unmaintained binding is not a thing to teach from |
| the sandbox | contained by the process only: a runaway script costs the *worker* (§6.2 says which one) | contained in-process *and* by the process: a runaway script costs one evaluation | as QuickJS, without the memory ceiling |
| `deny.toml` | passes | passes | passes |

### 5.4 The recommendation, and why it is not the one `doc/todo/56` gave

**Boa, behind a feature, in a confined process of its own** — with the two costs written down: the
57 packages, and the fact that the only budgets are the process's. `doc/todo/56` §7 reached the same
engine for the same reason and this RFC re-measured it: the construction cost is a third of a
millisecond, the binary cost is nine and a half megabytes *only in the build that enables it*, and
the containment gap is real and is closed one layer up by machinery this tree already runs
(`RLIMIT_AS`, a channel that waits with what is left of the budget, a worker the host knows how to
lose).

**And the alternative is named as a live alternative rather than a straw man**: QuickJS through
`rquickjs` is the engine pdf.js runs (compiled to WebAssembly), it is *better* contained
in-process, it is one twentieth of the dependency growth, and its C is exactly what principle 3
says may live in the confined process "justified in writing". If the owner weighs the 57 packages
and the "experimental" in Boa's README above the four CVEs and the C, QuickJS is a defensible
choice, and §6 is the same design with a different crate behind `pdf-script`'s feature. The RFC
recommends Boa because the language-safety class is the property principle 3 was written for, and
because the containment that QuickJS adds in-process is a *convenience* — stopping one script
rather than one worker — where the process boundary is the *guarantee*.

Whichever is chosen: `Cargo.lock` grows by the count above; cold start grows by **zero** for a
document with no script (`OnceLock`, constructed on the first script the level lets run) and by the
construction cost measured above for one that has; the engine crate is behind a feature so that a
build without it has no engine to audit.

## 6. The architecture

### 6.1 A `pdf-script` crate, and what it depends on

One crate, `crates/pdf-script`, with one stated responsibility: *evaluate a document's scripts
against a host object model, and hand back edits*. It depends on `pdf-model` (for `ViewState`,
`FieldValue`, the field tree, `action::Trigger`) and on the engine behind a feature; nothing depends
on it except the confined worker and the tests. `#![forbid(unsafe_code)]` on the crate; the engine
is a dependency, and the crate root says so in the sentence principle 3 asks for (the same shape as
`viewer-gtk`'s binding note in `Cargo.toml`).

Its public surface is small and is data, for RFC 0007's reason (a closure cannot be recorded or
replayed; a request can):

- `Trigger` — which site fired (§2's tables, one variant per key, plus `Open` for the name tree
  and `OpenAction`) and on which object;
- `Script` — the text and where it came from;
- `Outcome` — a list of `Edit`s (a field value, a field property, an OCG state, a page request),
  a list of `Ask`s (`alert`, `response`, a URI, a submission), a list of `Refusal`s (each Tier 2
  call by name, each budget exceeded, each throw), and the `rc` the site listens to;
- `Budget` — instructions, wall clock, memory, and the alert cap.

### 6.2 Where the engine runs — and why not in `pdf-view-worker`

`doc/todo/56` §6 laid out three placements and this RFC agrees with its reading, with one
correction of emphasis. **Host-side is refused** (principle 3 inverted). **Inside
`pdf-view-worker`** is tempting — it already holds the document, the view state and the
confinement — and wrong for the reason `pdf-sandbox` exists: the budget must bound the hostile
thing without bounding the work the user is waiting for, and a script that spins under `RLIMIT_AS`
shared with the rasteriser takes the page with it. **A third worker, `pdf-script-worker`**, on
`confined-transport`'s wire (which "carries a kind byte and a length and does not know what either
means" — it was built to be shared), with its own seccomp profile tighter than the view worker's
(no `mmap` beyond the heap, no descriptors at all), its own `RLIMIT_AS` in the tens of megabytes,
and a deadline per trigger. What crosses to it: the scripts (once, at open), the field values and
properties the trigger needs, the event's fields. What crosses back: an `Outcome`. It never holds
the file's bytes.

*The correction of emphasis*: a third worker is a launch-path question, and §6.6 answers it — it is
**spawned on the first trigger the level lets run**, never at open, so a document with no script,
or a reader at `off`, never starts it.

### 6.3 The policy hook — one place, four levels, the host supplies it, `off` by default

`viewer_host::policy` already holds `Links` and `Submissions`, each a four-level enum in the
permissive-end-last direction ADR 1155 chose because the subject is *this machine doing something
a document asked*. `Scripts` is a third, in the same file, read once per trigger:

| level | what happens |
|---|---|
| `off` | no engine is constructed, no worker spawned; every site is refused by name exactly as today. **The default.** |
| `ask` | the first trigger in a document puts one question — *this document carries N scripts; run them?* — and the answer holds for the document's lifetime. Never per trigger: a keystroke script fires per key |
| `warn` | scripts run; the report says which ran, what each changed, and what each was refused |
| `on` | scripts run, reported in the panel and nowhere louder |

*Why `off` is the default, and this is a recommendation rather than a certainty*: every other
default in this tree is `ask` (`A67`, `A98`), and the argument for *ask* was that a person is
clicking something they can see. A script runs on *open*, with no click, and a Tier 1 script
reaches nothing outside the process — so the honest candidates are `off` (nothing runs until the
reader says so) and `on` (Tier 0 and 1 reach nothing a person could object to). The RFC recommends
`off` until the `script_corpus` gate of §6.7 exists and has been green for a batch, and then puts
the question of `on` back to the owner with the gate's number in front of it; §10's question 1.
`quorra-confined` is pinned to `off`, as it is pinned to `refuse` for links, because it has no
dialogue to ask with.

Tier 2 is **not** a level. An excluded call throws at every level; the levels decide whether the
engine runs, not what it may reach.

### 6.4 The `Doc`/`Field` bridge over the view state and the edit log

`pdf_syntax::Document` stays immutable, and this is the load-bearing sentence of the design. A
script's `f.value = 12` is an `Edit` in `ViewState::edited` — the same map a typed value lands in,
read by `ViewState::annotation` and drawn by `variable_text` — so a calculated total is regenerated
by §12.7.4.3's machinery exactly as a typed one is, and `edits()` hands it to the host to save by
§7.5.6's incremental update. A `display = hidden` is an override beside the Hidden set §12.6.4.11
already keeps; an OCG change is `set_group`. Nothing the engine does has a path to the document.

`interpret` therefore stays a pure function of *(file, view state, what the user did, what a script
did)* — the fourth being a log entry indistinguishable in kind from the third — and the oracle's
comparison, which rests on the first alone, is untouched because the oracle runs at `off`.

### 6.5 The event model — which viewer event fires which action, in what order

The standard fixes the order between sites it names; Adobe's reference fixes the order within a
field's events (its "Form event processing" page is a diagram and two sentences; the order read off
the diagram is recorded in `doc/todo/56` §3 and is a documented choice). Together:

1. **Open** (once): §12.11.1's requirements are evaluated first — "before execution of any
   ECMAScripts"; then Table 32's name tree, every entry, in tree order ("all of the actions in this
   name tree shall be executed, defining ECMAScript functions for use by other scripts"); then the
   catalog's `/OpenAction`; then page one's `/O` ("shall be executed after such an action"); then
   each of page one's annotations' `/PO` ("after the O action"). The `event` object is
   `Doc`/`Open` with `targetName`.
2. **A page turn**: the leaving page's annotations' `/PC`, then its `/C` ("shall be executed before
   any other page is opened"), then the arriving page's `/O`, then its annotations' `/PO`; `/PV` and
   `/PI` as visibility changes — the events `viewer-core` already raises for a non-script action.
3. **A pointer on a widget**: `/E`, `/D`, `/Fo`, `/U` (or `/A`, which "takes precedence" over `/U`),
   `/Bl`, `/X`, with §12.6.3's constraints (no `/X` without `/E`, no `/U` without `/E` and `/D`) —
   already enforced by `headless.rs`'s tests.
4. **A keystroke in a text or combo field**: `/K` with `event.change`, `changeEx`, `selStart`,
   `selEnd`, `value`, `willCommit = false`, `rc` listened to (false rejects the keystroke); a
   selection change in a list box is a keystroke whose `change` is the selection.
5. **A commit** (Enter, Tab, click away — `commitKey` 0–3): `/K` once more with `willCommit =
   true`; then `/V` with `value`, `rc` listened to (false leaves the field unchanged — Adobe's text
   says both that the event does not listen and that false invalidates; the RFC adopts the second
   sentence, as a documented choice, because it is the one that has an effect); then **every** field
   in `/CO`, in order, gets `/C` with `source` the changed field and `rc` listened to (false leaves
   that field's value); then `/F` on every field whose value changed, with `willCommit` and
   `commitKey`, the resulting `event.value` being the *displayed* string — never the stored value.
6. **Save**: `/WS` before the incremental update is written, `/DS` after. **Close**: `/WC`.
   **Print**: `/WP`, `/DP` — refused until RFC 0004 builds printing.

Re-entrancy is bounded: a calculate that writes a field earlier in `/CO` does not restart the chain
(the chain runs once per commit); a script that sets a value from inside `/F` is applied after `/F`
returns; a trigger raised from inside a trigger is queued, not nested, with a depth bound the same
shape as `MAX_FORM_DEPTH`. Each is a documented choice; Adobe's reference is silent on all three
(`doc/todo/56` §3's items 5 and 7).

### 6.6 The launch path, and the name tree that is `shall`-executed on open

Principle 2 says nothing eager, and the standard says the name tree runs "when the document is
opened". The two are reconciled by ordering rather than by skipping: **page one is presented
first**, from the appearance streams the file carries, and the open sequence of §6.5 step 1 runs
*after the first present* — the worker is spawned then, the engine constructed then. A field whose
`/F` changes its displayed value repaints, exactly as a `NeedAppearances` rewrite does today. That
is a documented choice: §12.6.4.17 says *when opened*, not *before the first frame*, and Adobe's
"Document Event Processing" page puts the `NeedAppearances` formatting *after* the `Doc`/`Open`
scripts, which is the same shape. Cost to time-to-first-page: zero for every document, because
nothing on the critical path waits for the worker; cost to the scripted document: the construction
time measured in §5 plus its own scripts, after the page is up, and reported by the `open_cost`
instrument as its own line.

### 6.7 What the gates do

- **The oracle and `raster_golden`, `corpus.rs`, the accessibility and selection censuses**: at
  `off`. Unchanged, and that is the property to protect.
- **`script_corpus`** — a new tier 2 gate in `crates/pdf-model/tests/` on the documents §3's census
  lists (the instrument prints their names; the gate's population is derived from it rather than
  hand-written, trap 25). For each: run step 1 of §6.5 and, for every field with an `/AA`, a
  synthetic commit of its current value (so `/K`, `/V`, `/C` over `/CO`, and `/F` all fire) under
  the §4.3 budgets; compare every field's displayed value against a golden of this program's own
  output (`raster_golden`'s discipline, held by name); count budget exceedances, throws and Tier 2
  refusals as three columns with a ratchet each. A document that finishes under budget stays under
  budget, and a value that changes is a diff to read.
- **Fuzzing**: `fuzz/` has a `script` target from the first engine commit — the engine's input is
  untrusted, and the harness is `pdf-script`'s entry point with a synthetic document. `fuzz/` is
  owned this batch by round 1293; the target is named here and written when the crate exists.
- **`cold_bring_up` and `launch_path`**: unchanged, because the worker is not on the path; a
  `script_open` line beside `open_cost` measures the scripted document's own cost.

### 6.8 Failure modes, each with its consequence

| a script that | meets | and the reader sees |
|---|---|---|
| loops | the wall-clock budget per trigger (Boa: the process deadline and `RuntimeLimits`' loop-iteration limit; QuickJS: the interrupt) | the field unchanged and one report: *the format script of Total exceeded 50 ms and was stopped* |
| allocates | `RLIMIT_AS` on the worker (Boa), or the engine's memory limit first (QuickJS) | the same shape: *…exceeded its memory budget* |
| throws | `Outcome::Refusal` | reported **once per document** per site — never a dialog, never a storm |
| alerts in a loop | the alert cap (one dialog per trigger; the rest coalesced into the report) | one dialog, and *N further alerts were not shown* |
| calls Tier 2 | `NotAllowedError`, §4.3 | if uncaught, the throw above; `launchURL` and `submitForm`, the question the link or the form would have raised |
| crashes the engine (a panic in Boa reachable from input; a memory-corruption in C) | the worker dies; the host has already lost workers (ADR 0713) | *scripts stopped running for this document* and every field as it was — the document is still open, because the view worker is a different process |
| runs at open forever | the open-sequence budget (one deadline for step 1 as a whole) | page one, already presented, and the report |

## 7. What the ledger does, honestly, in both directions

Re-read on 2026-09-30 against `doc/conformance/ledger.toml` rather than copied from `doc/todo/56`
(whose §4.1 was written when four of these rows were `partial`; they are `implemented` now, with the
script half named as excluded in each note):

| row | today | after Tier 1 |
|---|---|---|
| §12.6.4.17 | `out-of-scope`, the tree's only `script-behaviour` exclusion | `partial` — the action executes; the API is a subset by design |
| §12.6.3 | `implemented`; note: "Tables 199 and 200 are excluded rather than owed" | `implemented` with Table 199's four and Table 200's `/WC` `/WS` `/DS` performed, `/WP` `/DP` owed to RFC 0004 |
| §12.7.3 | `implemented`; `/CO` "the first time this tree writes the entry it still does not execute" | `/CO` executed — the sentence retired, **after Tier 0 alone** |
| §7.7.4 | `implemented`; `/JavaScript` settled to §12.6.4.17's exclusion | the tree read and executed at open |
| §12.6.4 | `implemented`; `/JavaScript` among the six refused by name | five refused by name |
| §12.11.1, §12.11.2 | `implemented` *because nothing runs* | **go backwards**: an ordering obligation to schedule, a requirement to meet |
| §12.11.5 | `out-of-scope` on "a handler is a script and nothing runs" | **comes back into scope**: `/RH` read, `JS` handlers disabled when the requirement is verified |
| §12.7.8.3.1 | `partial`, Table 246's `/JavaScript` in an FDF | unchanged — an import, not a trigger |

One row settled, four amended, three that go backwards. **Anyone arguing this on coverage is
overselling it**; the argument is the corpus's (§3) and the `shall`s' (§2), not the ledger's.

## 8. The costs, each written down

1. **The dependency.** +57 packages for Boa (+3 for QuickJS), behind a feature; 9.5 MB of binary in
   the enabled build. `doc/stack.md` gets a row that says what the engine is in a position to
   break: nothing a document without a script touches, and every field value in one that has.
2. **`#![forbid(unsafe_code)]` becomes a process-enforced rule in one crate's dependency graph.**
   No ECMAScript engine that exists is written without a garbage collector in `unsafe`, and Boa's
   is 180 occurrences in 4 305 lines of `boa_gc`. The compiler-enforced rule stays on every crate
   of this tree; what changes is that one confined process links a dependency that has an
   `unsafe` heap — which is already true of the JBIG2 and JPX decoders under ADR 0014, in the same
   place, for the same reason.
3. **The `AF*` library is documented choice from end to end** — ~600 lines whose every rounding
   rule, separator and month name is this project's, with nothing to derive them from. §2.8.
4. **Three ledger rows go backwards** (§7), and §12.11.5 has to be rewritten from the ground up.
5. **The test burden is a golden of this program's own output**, not a specification: the
   `script_corpus` gate can say *unchanged* and *finished under budget*, and cannot say *correct*
   except where a value is arithmetic.
6. **A third worker is a new part** — ADR 0709's sweep population, `confined-transport`'s second
   consumer becoming its third, a protocol to design and to fuzz.
7. **The oracle cannot see it**, and that is the price of keeping the oracle pure: every effect of a
   script is compared against this program's own earlier output, never against a reference.
8. **A reader at `off` gains nothing and loses nothing**, which is 99 % of the world's documents
   (98.97 % of §3's population; 99.59 % of the crawl): this buys the tail.

## 9. The current restrictions, each with its rationale, and the unconstrained design

`doc/rfc/README.md`: an RFC names a standing rule as a *current restriction with its original
rationale* and proposes the unconstrained design. Five bear on this one.

| restriction | its rationale | what the unconstrained design is | what this RFC proposes |
|---|---|---|---|
| **The exclusion**: *"JavaScript and script-driven form behaviour — a sandboxed script engine is a separate project with its own security argument. Field appearance is not excluded; field behaviour is."* | written before the sandbox existed; the argument was that the security work was not this project's | an engine in the confined process under the four levels | **amend it** — §10's question 8 has the sentence — to *in scope as far as a script reaches this document's values and this viewer's state, in a confined worker under the four levels; every call that leaves the process refused by name* |
| **`pdf_syntax::Document` is immutable** | `interpret` is a pure function of the bytes; the oracle rests on it | a script that mutates the document (`addField`, `insertPages`) | **keep it**; §6.4 puts a script's writes in the edit log and §4.3 excludes the structural methods. This is not a restriction the design strains against; it is the design |
| **No C outside the sandbox; `forbid(unsafe_code)` on every crate that touches PDF bytes** | untrusted input never reaches unsafe code | QuickJS or V8 in the worker, or Boa's `unsafe` collector | **kept as written**: the engine is in a confined process holding no file bytes, and the crate that calls it forbids `unsafe` itself; a C engine is admissible there under ADR 0014's precedent if the owner prefers it (§5.4) |
| **No heavy runtime** | a thread pool is not a reason for an async runtime | `deno_core` (V8 + tokio) | **kept**: neither recommended engine needs one; `deno_core` is out on this ground alone |
| **Nothing eager on the launch path** | time-to-first-page is the number a user judges by | the name tree run before the first present, as a literal reading of "when the document is opened" | **kept**, by §6.6's ordering rather than by skipping the `shall` |

## 10. The questions for the owner, numbered, each with a recommendation

The convention (`doc/questions/README.md`) puts every open question in a `Q` file; this round has
one number, `Q193`, which points here and asks the owner to answer these by number in one `A193`.
Eleven, the last added by the census.

1. **The default level.** `off` (nothing runs until a reader turns it on), or `on` (Tier 0 and 1
   reach nothing outside the process, and every other document a person opens already fills its
   forms). *Recommendation*: **`off` until the `script_corpus` gate has been green for a batch**,
   then the question again with the gate's number; `quorra-confined` pinned to `off` throughout.
   The argument for `ask` fails here — a script fires per keystroke, and *ask* is one question per
   document at most, which is a weaker thing than the word promises.
2. **Is Tier 0 — the native `AF*` library and `/CO` — built before any engine?** *Recommendation*:
   **yes**, and judged on its own: it serves a fifth of scripted documents outright (§3.3), costs no
   dependency, is comparable by the oracle, and is owed under any engine anyway. Its criterion is
   the one `doc/todo/56` §7 found the engine step could not meet — a currency field reads
   `$1,234.50` — and Tier 0 is the tier that can meet it.
3. **The library.** Boa (Rust; +57 packages; budgets are the process's) or QuickJS via `rquickjs`
   (C in the confined process, justified in writing; +3 packages; budgets in-process too).
   *Recommendation*: **Boa**, §5.4, with QuickJS named as the defensible alternative and the same
   design behind either.
4. **Do `app.launchURL` and `this.submitForm` go through the existing link and submission policies
   rather than being refused?** *Recommendation*: **yes** — §4.4; a script may raise the question a
   link raises and never more, and a face without a dialogue refuses both as it refuses the action.
5. **Does the engine live in a third confined process (`pdf-script-worker`) rather than in
   `pdf-view-worker`?** *Recommendation*: **a third process**, spawned on the first trigger the
   level lets run — §6.2 — so that a runaway script costs a script and never a page.
6. **Are the document-level name tree and `/OpenAction` scripts run after the first present rather
   than before it?** *Recommendation*: **after**, §6.6, as a documented choice; the alternative
   puts the driver-independent part of time-to-first-page at the mercy of a document.
7. **Is Adobe's reference cited as the source for every API member, with the citation shapes
   `doc/todo/56` §3 fixed (no section sign, the commit of `adobe/dc-acrobat-sdk-docs` pinned)?**
   *Recommendation*: **yes**, and recorded once in the `pdf-script` crate's root comment rather
   than per member. The reference moved between 2026-08-28 and 2026-09-30 — the URL `doc/todo/56`
   cites answers 404 and the content is now under
   `docs/acrobatsdk/html2015/Acro12_MasterBook/JS_API_AcroJS/` in the repository at commit
   `ab3b42a7` (2026-01-22) — which is the pinning rule proving its worth in a month.
8. **Does this RFC amend `CLAUDE.md`'s exclusion, and with what sentence?** *Recommendation*:
   **yes**, replacing the entry with:

   > **JavaScript and script-driven form behaviour** — in scope as far as a script reaches *this
   > document's* field values and appearance properties and *this viewer's* state, evaluated in a
   > confined worker under a memory and a wall-clock budget, and under *A document's restrictions
   > are the reader's to set*, defaulting to `off`. Acrobat's `AF*` form library is in scope as a
   > documented choice with no engine. **Excluded still**: every call that leaves the process — the
   > network, the filesystem, mail, printing until RFC 0004, other documents, the application's
   > chrome, persistence across documents, privileged functions — each refused by a named exception
   > the script can see; any effect that would mutate `pdf_syntax::Document`; ISO 21757-1's 3D API
   > under clause 13; XFA under Annex K.
9. **Is ECMAScript 2020 (ECMA-262 11th edition) the language target**, per pdf-issues #185's
   ISO-approved erratum replacing ISO 21757-1's undated reference? *Recommendation*: **yes**; it is
   the only dated statement of the version anywhere this project can reach, and both candidate
   engines exceed it.
10. **Is `app.setTimeOut`/`setInterval` admitted (Tier 1, bounded to the document's lifetime and
    the trigger budget) or excluded?** *Recommendation*: **admitted, bounded** — the corpus uses it
    for a delayed `calculateNow` after open, and a timer that dies with the document persists
    nothing.
11. **What do `app.viewerType` and `app.viewerVersion` answer?** §3.4: the commonest script in the
    world branches on them, and a truthful answer puts one dialog to the reader in some three
    hundred documents, telling them the form wants Adobe Reader 9. *Recommendation*: **the truth**
    — this program's name and version — under §6.8's one-dialog cap; the alternative is answering
    to another program's name, which is the one thing principle 5 forbids in every other clause.
    If the owner prefers quiet, the level is the lever (`off`), not the answer.

## 11. What is built when

1. **Now, and whatever the owner decides**: the census is in the tree and its command is in §3;
   `doc/todo/56` §4.1 is corrected to the ledger as it is.
2. **On `A193` accepting question 2 — Tier 0**: `pdf_model::aform` (a name that says what it is),
   dispatched from `ViewState` on a commit for `/K` `/F` `/V` `/C` whose text is one `AF*` call;
   `/CO` walked in order; every rule an ADR names as a choice; `script_corpus` in its Tier 0 form
   (values only). One row moves (§12.7.3's `/CO` sentence), and the ledger's §12.6.4.17 stays
   `out-of-scope` because no ECMAScript ran.
3. **On question 8's amendment — Tier 1, in the order `doc/todo/56` §7 argued and this RFC keeps**:
   (a) `pdf-script` with the engine behind a feature, the `Outcome` type, and `/K` + `/F` on one
   field, judged on *the script runs, the budget holds, the log stays beside the document, every
   refusal is named*; (b) `/V`, `/C` over `/CO`, `event.rc`; (c) the name tree, `/OpenAction`,
   Table 198's and Table 197's sites, after the first present; (d) Table 200's `/WC` `/WS` `/DS`;
   (e) the `Scripts` level in `viewer_host::policy`, the three windows' menus, and `A193`'s default.
4. **With (a)**: the third worker and its profile, the `script` fuzz target, `script_corpus` in its
   full form, the `script_open` cost line, §12.11.1's ordering and §12.11.5's `/RH`.
5. **Never, under this RFC**: Tier 2's list, the 3D API, XFA, `Net`, `SOAP`, `Collab`, a watermark.

## 12. Sources, all read on 2026-09-30

- ISO 32000-2, from `doc/md/ISO_32000-2_sponsored_EC3.md`: §7.7.4 (Table 32), §12.6.3 (Tables
  197–200), §12.6.4.14 (Table 218), §12.6.4.17 (Table 221), §12.7.3 (Table 224), §12.7.4.3,
  §12.11.1, §12.11.2 (Table 274), §12.11.5 (Table 276).
- Adobe, *JavaScript for Acrobat API Reference*, as the repository `adobe/dc-acrobat-sdk-docs` at
  commit `ab3b42a75df65543025736e9699e1e63af1922b0` (2026-01-22), MIT-licensed, under
  `docs/acrobatsdk/html2015/Acro12_MasterBook/JS_API_AcroJS/` — the pages "Event type/name
  combinations", "Form event processing", "Document Event Processing", "event properties", "Field
  methods", "Field properties", "Doc methods", "Doc properties", "app methods", "app properties",
  "util methods", "Net.HTTP methods", "SOAP methods", "Collab methods", "global methods", "Global
  object security policy"; the privileged-context marker read off each method's quick-bar. The
  live site's `library/jsapiref/` URL answers 404 on this date.
- Adobe, *Interapplication Communication API Reference*, the Acrobat Forms plug-in's
  `SetJavaScriptAction` (`IAC_API_FormsIntro/`), for the `AF*` argument tables.
- `pdf-association/pdf-issues` #185, #70, #100, #744, #535, #270, #99 (via `doc/todo/56`).
- crates.io on 2026-09-30: `boa_engine` 0.22.0, `boa_gc` 0.22.0, `rquickjs` 0.14.0, `rquickjs-sys`
  0.14.0, `ducc` 0.1.5, `v8` 152.2.0, `deno_core` 0.412.0, `mozjs` 0.26.6, `nova_vm` 1.0.0,
  `quick-js` 0.4.1; the registry sources of each measured crate for the `unsafe` and C counts.
- `scratchpad/r1296/js-timing/` and its listing beside this RFC, for every timing.
- `doc/pdf.js` `src/scripting_api/` (6 287 lines, `aform.js` 627) — evidence about the size of
  the work, never a target.
- `doc/todo/56`, whose measurements of 2026-08-28 this RFC re-took where it could.

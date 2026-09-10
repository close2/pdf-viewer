# Converting to PDF/A: what a user has to be told before it runs

This file is the honest list. It exists because **"convert to PDF/A" is not one operation**: it
is a clause-by-clause set of requirements, of which most are mechanical, some need a default that
somebody has to choose, and a handful cannot be satisfied at all without changing what the
document says. A converter that quietly did the third kind would be producing a file that
*validates* and no longer *is* the document, which is the opposite of archiving.

Read it beside `doc/rfc/0006-pdf-a-validation-and-conversion.md`, which argues the design. This
file does not argue; it lists, and for each limitation it says which of four things is true:

| class | meaning |
|---|---|
| **Refuse** | no conforming file can be made from this document *for the target asked for* without destroying what it says. The verb stops and names the reason — and §2.0 is the part worth reading first, because a refusal is usually of a **part or a level** rather than of the document. |
| **Ask** | it can be done, and it loses something a user might not accept. The verb stops unless told to proceed, and records what it did. |
| **Default** | something must be added or chosen that the source does not state. The default is stated here, and is overridable. |
| **Mechanical** | it can be done losslessly, silently, every time. |

## Provenance, and why nothing below is in quotation marks

Every requirement below is cited by clause. **The clause numbers are ISO 19005-2:2011 and
ISO 19005-4:2020, read first-hand** — the owner bought both and they are in `doc/pdfa/`, with
`tools/pdfa-text.py` writing the readable Markdown. What is *not* here is their wording: those
two files are licensed to a single reader, so this committed file paraphrases and cites, and a
reader who needs the exact sentence opens `doc/pdfa/ISO_19005-2_2011.md` or
`doc/pdfa/ISO_19005-4_2020.md` at the clause named. `CLAUDE.md` allows paraphrase and forbids
paraphrase wearing quotation marks; the absence of quotation marks below is therefore deliberate
and not an oversight. ISO 32000-2 is quoted normally, because `doc/md/` carries it and
`tools/conformance` can check it.

**Two statements below are second-hand and are marked where they appear.** They are about
ISO 19005-1 and ISO 19005-3, which this project does not own (`doc/questions/A17`: part 1 never;
part 3 not bought).

---

## 1. The first limitation is which target you may ask for

**This converter can offer PDF/A-2 and PDF/A-4. It cannot offer PDF/A-1 or PDF/A-3, and that is
not a schedule — it is a rule about evidence.** Principle 5 forbids implementing a requirement
from somebody else's reading of a text we do not have, so a part whose normative text is not in
`doc/pdfa/` cannot be a target at all. The consequence for a user is concrete:

| target | offered | why |
|---|---|---|
| **PDF/A-2b** | yes | text owned. The recommended default for a document that has to be archived and is not being made accessible. |
| **PDF/A-2u** | yes, conditionally | adds ISO 19005-2 §6.2.11.7's Unicode requirement, which some documents cannot meet — see §4.3. |
| **PDF/A-2a** | yes, from a source that is already tagged | §5.1. What is refused is *inventing* a structure tree, not certifying one the document has. |
| **PDF/A-4** | yes | text owned. Built on PDF 2.0, which is the version this tree reads natively. |
| **PDF/A-4f** | yes | the only target that keeps arbitrary attachments (Annex A.2). |
| **PDF/A-4e** | no | Annex B admits 3D and RichMedia, which is `CLAUDE.md`'s clause-13 exclusion by name. |
| **PDF/A-1a/1b** | no, and not later | `doc/questions/A17`. Superseded twice, and the only part with a transparency cliff. |
| **PDF/A-3** | no | text not owned. *(Second-hand: PDF/A-3 is the part usually named for keeping arbitrary attachments in a 1.7-based file. We cannot implement it, and PDF/A-4f is the owned equivalent.)* |

### 1.1 Choosing the target removes more limitations than any other decision

Several entries further down are not really limitations of conversion — they are limitations of
*the target that was asked for*. The four that most often decide the answer:

| the document has | PDF/A-2 | PDF/A-4 | PDF/A-4f |
|---|---|---|---|
| **attachments of arbitrary type** | forbidden (§6.8) | forbidden (§6.9) | **permitted** (Annex A.2) |
| **JavaScript that matters** | forbidden (§6.5.1) | **permitted**, user-invoked only (§6.6.2) | same as -4 |
| **widget additional actions (`/AA`)** | forbidden (§6.5.2) | **permitted** on widgets (§6.6.3) | same as -4 |
| **a page larger than 14 400 units** | forbidden (§6.1.13) | **no limit stated** (see §2.5) | same as -4 |

So the single most useful thing a converter can say to a user is not "this failed" but **"this
cannot be PDF/A-2 and can be PDF/A-4f, for these three reasons."** That is the report the verb
should produce, and it is the reason the validator is built before the converter.

### 1.2 PDF/A-2's base document, and the day it stopped being a limitation

ISO 19005-2 §5.1 makes a conforming file one that adheres to **all requirements of ISO 32000-1**
as modified by part 2, and §6.7.2.1 pulls in ISO 32000-1:2008 §14.8 wholesale for Level A. This
tree is written against **ISO 32000-2** — that is what the conformance ledger is written against
and what every doc comment in `crates/` cites — so for as long as ISO 32000-1:2008 was not here,
every PDF/A-2 requirement was being read in the wrong edition.

**The owner obtained it on 2026-09-07**, and `tools/spec-md.py` prepared it as
`doc/md/ISO_32000-1_2008.md`. What that changes:

- **The gap that mattered is closed.** Where the two editions differ — PDF 2.0 deprecated
  features, added structure namespaces and new standard structure types, and rewrote the
  encryption clauses — a PDF/A-2 requirement can now be read in the edition part 2 actually
  names, instead of in the nearest available one.
- **It is not committed and cannot be**, on the same footing as the PDF/A parts themselves:
  free to obtain is not free to redistribute, so `/doc/*.pdf` ignores it and `doc/md/` ignores
  the prepared text. Committed source cites ISO 32000-1 clauses and does not quote them; only
  ISO 32000-2 quotations are checkable by `tools/conformance`, because only that edition is in
  the tree.
- **PDF/A-4 is still the better default where a user has a free choice**, but the reason is now
  a smaller one: its base is the edition this project implements, so a part 4 verdict cites the
  clause the code was written against. That is a shorter chain of reasoning, not a difference in
  what can be read.

---

## 2. Refusals — and the three different things that word was hiding

**"Refuse" was too blunt a word, and the question that exposed it is the right one: does this mean
we sometimes cannot archive a document we *can* view?** Read literally the answer is yes, and it
is worth understanding why that is not a defect in the tool before reading the entries below.

### 2.0 Viewable and archivable are different properties

A viewer is obliged to put something on the screen. It is not obliged to be *right*, and where the
file does not determine the answer — a font it does not carry, a code no method can name, a stream
whose bytes are on somebody's server — the viewer guesses, silently, and two viewers guess
differently. That is not a failure of viewers; it is what a renderer is for.

PDF/A is the opposite claim: **the file determines its own appearance**, and at Level U or A its
own text, without reference to anything outside itself. A document that leaves the answer to the
viewer is therefore not a document PDF/A can certify unchanged — it is precisely the document the
format exists to rule out. So the divergence the question names is real, and it is the format's
whole point rather than a limit of this program.

**But the honest response to that divergence is almost never to reject the file.** It is to lower
the claim until the claim is true, and that is what separates the three verdicts this section used
to run together. **Read the third column with §9 beside it**: "target another part" is an answer
for somebody choosing a target, and an archive that mandates one has already chosen. For them each
row below turns into a decision about what to *drop*, which is the more useful thing to know and
is what §9 tabulates.

| verdict | how often | what the user does |
|---|---|---|
| **No file at all** | rare | nothing can be read: an encrypted document with no password (§2.4), or a stream whose bytes live outside the file and the page needs them (§2.3). The viewer cannot show this either — the two properties agree here. |
| **Not this part or level — another one works** | the common case | implementation limits refuse PDF/A-2 and PDF/A-4 accepts (§2.5); unnameable codes refuse -2u and -2a and -2b accepts (§2.1); missing word boundaries refuse Level A and -2b accepts (§5.1); an arbitrary attachment refuses -2 and -4f accepts (§3.1). **A conforming file exists; it is not the one that was asked for.** *And where the target is fixed by a deposit rule, this verdict is not available — see §9, which is the version of this table for a user who cannot move.* |
| **A file, with a smaller claim** | frequent | the conversion succeeds, and the report says what could not be verified — a substituted font (§4.9), a transfer function removed (§4.8), an appearance constructed (§4.4). |

The one thing the converter must never do is the fourth option: produce the file *and* the full
claim, leaving the reader to discover that the claim was not earned. §7 is where that discipline
is written down, and it is the reason the report matters more than the exit code.

### 2.1 A font whose program is absent *and* whose codes nothing can name

**This entry has been wrong twice.** It first said any non-embedded font was a refusal, which §4.9
overturned; it then said this narrower case refused the *file*, and it refuses a **level**.

Substituting a face answers the question *what does this glyph look like*. It cannot answer *which
character is this code*, and for some fonts the file is the only place that answer lived:

- a **symbolic TrueType** font addressed through a `(3,0)` cmap, where the code is an index into
  the absent program and means nothing in any other face;
- a **Type 0 font with `Identity-H`** encoding and no `ToUnicode` CMap and no registered character
  collection, where the code *is* a glyph index in a program that is not here;
- a **name-keyed font whose glyph names are private** (`g43`, `uniF0E7`) and outside the Adobe
  Glyph List.

The test is one this tree already implements: §9.10.2's naming methods, which `pdf_font`'s
`naming_gap()` and `uncovered_character()` report per code and per page.

**What follows differs by target, and only the middle row is a refusal:**

| target | why | verdict |
|---|---|---|
| **PDF/A-2b** | §6.2.11.7.1 says the Unicode requirements apply to Level A and U only, and a writer of a Level B file may ignore them | **convert.** Substitute as §4.9, freeze the appearance, and report that the codes could not be named — so the file's *look* is now determined where it was not before, and its *text* is stated to be unverifiable |
| **PDF/A-2u, PDF/A-2a** | §6.2.11.7.2's `ToUnicode` is a `shall` at these levels, and the mapping is not in the file | **refuse the level**, name the fonts, offer -2b |
| **PDF/A-4** | §6.2.10.7 states the same rule as a `should` | **convert**, with the recommendation recorded as unmet |

So the document is archivable. What it cannot be given is a claim about its text, because the file
never made one — and inventing a `ToUnicode` to reach -2u would be manufacturing exactly the
evidence the level exists to require.

- **The user's own workaround beats all of this**: supply the missing font with
  `--font <name>=<file>` and the codes resolve against the real program, at which point every
  level is reachable.

### 2.2 Content that draws `.notdef`

ISO 19005-2 §6.2.11.8, ISO 19005-4 §6.2.10.9: a conforming file shall not reference the `.notdef`
glyph from any text-showing operator, in any rendering mode.

**Whether this refuses anything depends on where the missing glyph is**, and the split is the same
one §2.1 makes.

- **The font is embedded and its program has no glyph for a code the page shows.** **Refuse**, and
  it is a true one: the mapping is fixed by a program the file carries, and the only ways out are
  editing the content stream to stop showing the code or editing the font program to give it a
  glyph. The first takes a mark off the page and the second invents one; ADR 0816's fence is where
  both stop.
- **The font is *not* embedded, and §4.9 is substituting anyway.** Then the converter is building
  the embedded program and chooses what each code maps to, so it need never reference glyph 0 —
  and the clause is satisfied without anything being edited. **Convert.**
  - **The honest way to do that, and the tension in it.** Where this program's own renderer draws
    nothing for a code today, the substituted program gets a real but *empty* glyph: the page
    looks exactly as it did, and no `.notdef` is referenced. That satisfies the clause's letter.
    Its NOTE says the prohibition exists because `.notdef` carries no semantic value — and an
    empty glyph carries none either, so the spirit survives, where mapping the code to some
    *plausible letter* would give it a false one. That is the line: preserve the absence, never
    fill it in.
  - **`A48` allows it, reported per document and recorded in `xmpMM:History` naming the clause** —
    and `doc/adr/0927` records that this is the *thinnest* of the four permissions the owner
    granted, not a comfortable one. The other three state an interpretation the standard defines;
    a code that reaches `.notdef` has no glyph the standard defines, so this one is the case where
    "preserve the absence, never fill it in" is doing all the work. It is allowed because an empty
    glyph preserves the absence. Anything that filled it would not be.
- A `.notdef` on the page is usually the visible symptom of a missing font, so supplying the font
  (`--font`) fixes both this and §2.1.

### 2.3 Streams whose data is outside the file

ISO 19005-2 §6.1.7.1, ISO 19005-4 §6.1.6.1: a stream dictionary shall not carry `/F`, `/FFilter`
or `/FDecodeParams`. ISO 19005-2 §6.2.9.2 and ISO 19005-4 §6.2.8.2 say the same of reference
XObjects.

- **Class: Refuse** for an external stream the page actually uses. The bytes are on somebody's
  disk or server; fetching them is a network operation this program does not have and (principle
  3) will not acquire, and a file assembled from an unverifiable fetch is not an archival object.
  This is one of the two places where "cannot view it either" and "cannot archive it" agree.
- **An external stream nothing draws is not a refusal.** ISO 19005-2 §6.2.2 exempts a named
  resource that is present in a resources dictionary and never referenced from the content stream
  it belongs to — the standard's own words are that such a resource is not used for rendering and
  is therefore exempt from every requirement of the part. Dropping it is then lossless by the
  clause's own reasoning.
- **Reference XObjects are the one case with a real workaround.** ISO 32000-2 §8.10.4 requires a
  reference XObject's containing form XObject to serve as a *proxy* — what a processor draws
  "when the referenced content is not available", and what a processor that does not implement
  `/Ref` at all draws unconditionally. An archived file is precisely the case where the target
  will not be available, so dropping the `/Ref` entry and keeping the proxy writes down what the
  file was going to show. **Ask**, not Refuse, and the report says which page was affected — a
  reader that *could* have reached the imported content will now see the producer's placeholder
  instead.

### 2.4 An encrypted document whose password you do not have

ISO 19005-2 §6.1.3 and ISO 19005-4 §6.1.3 forbid `/Encrypt` in the trailer outright.

- **Class: Refuse** if the file cannot be decrypted. Nothing can be read, so nothing can be
  converted.
- If it *can* be decrypted, see §3.5 — removing the encryption is a decision, not a mechanism.

### 2.5 Geometry outside PDF/A-2's implementation limits

ISO 19005-2 §6.1.13 sets hard limits: integers within ±2 147 483 647, reals bounded, strings
≤ 32 767 bytes, names ≤ 127 bytes, ≤ 8 388 607 indirect objects, `q`/`Q` nesting ≤ 28, DeviceN
≤ 32 colourants, CID ≤ 65 535, and every page boundary between 3 and 14 400 units in each
direction.

- **Class: Refuse for PDF/A-2** where the source exceeds one of them, because the fixes are all
  edits to content: re-nesting `q`/`Q`, rescaling a page, re-encoding a colour space.
- **The workaround is the target.** ISO 19005-4 states no implementation-limits subclause at all
  — its §6.1 runs 6.1.1 to 6.1.12, and the document's own contents list confirms it. (§6.1.1's
  cross-reference to "6.1.13" has no target in the published text; treated here as an editorial
  slip, and noted rather than relied upon.) **A large-format drawing, a deeply nested content
  stream or a 40-colourant DeviceN space can be PDF/A-4 and cannot be PDF/A-2.**

---

## 3. Data loss the user has to authorise

Each of these can be done. Each throws something away. None of them may happen silently.

### 3.1 Attachments — the case that turns on the target

ISO 19005-2 §6.8 permits `/EF` and `/EmbeddedFiles` **only if every embedded file is itself
conforming to ISO 19005-1 or -2**. ISO 19005-4 §6.9 widens that to ISO 19005-1, -2 or -4 —
note that it does *not* list part 3 — and adds two requirements of its own: every embedded file
specification shall carry `/AFRelationship`, and shall carry `/F` and `/UF`. **ISO 19005-4
Annex A.2 (PDF/A-4f) is the one place where an embedded file may be of any type**, and it
requires the `/EmbeddedFiles` key to be present at all.

The decision tree, in the order a converter should try it:

1. **The attachment is a PDF and converts.** Convert it recursively to the same part and embed
   the result. Fully lossless where it works, and the only outcome that keeps both the file and
   the target. *(Cost: the attachment is now a different byte string, which matters if anything
   signed or hashed it.)*
2. **The attachment is not a PDF, or does not convert.** Then PDF/A-2 and plain PDF/A-4 are both
   impossible with it, and there are exactly two honest answers: **retarget to PDF/A-4f**, or
   **drop the attachment**. There is no third one, and this is the case the user's own example
   named.
3. **Dropping is Ask, always**, listing every file dropped by name and size, and recording the
   removal in `xmpMM:History` — which ISO 19005-2 §6.6.6 explicitly asks a converter to do, its
   own example of a thing to record being objects that were not retained.

**Default: never drop.** Report that the document has *n* attachments, that PDF/A-4f keeps them,
and let the user choose between the target and the attachments.

### 3.2 Multimedia and 3D annotations

ISO 19005-2 §6.3.1 forbids annotation subtypes not defined in ISO 32000-1, plus `3D`, `Sound`,
`Screen` and `Movie`. ISO 19005-4 §6.3.1 forbids `Sound`, `Screen` and `Movie`; permits `3D` and
`RichMedia` only in a PDF/A-4e file; and permits `FileAttachment` only in a PDF/A-4f file.

- **Class: Ask.** Removal is the only option, and for `Screen`/`Movie`/`Sound` the media stream
  goes with it.
- **Default: remove the annotation and report it.** The tempting alternative — keep its
  appearance stream by re-badging it as a `Stamp` — is inventing an annotation the producer did
  not write, and ADR 0816's fence is where that stops. Where the removed annotation had a normal
  appearance the page loses that mark, and the report says which page.
- **`FileAttachment` annotations are §3.1's problem in a different dictionary**: in PDF/A-4 they
  force PDF/A-4f, and the *annotation* is what forces it, not just the name tree.

### 3.3 JavaScript, and the actions that carry behaviour

ISO 19005-2 §6.5.1 forbids `Launch`, `Sound`, `Movie`, `ResetForm`, `ImportData`, `Hide`,
`SetOCGState`, `Rendition`, `Trans`, `GoTo3DView` and `JavaScript` actions, the obsolete
set-state and no-op actions, and every named action but the four page-navigation ones. §6.5.2
forbids `/AA` on widgets, fields, the catalog and pages.

ISO 19005-4 is materially different and this is the second big target-choice: §6.6.1 keeps most
of those prohibitions but **§6.6.2 permits JavaScript actions**, on the condition that a
conforming processor only runs them on explicit user invocation and a non-interactive processor
never runs them. §6.6.3 permits `/AA` on widget annotations and restricts (with *should*, and one
*shall* on the key set) the other places it may appear.

- **Class: Ask** for PDF/A-2, because a form that computed its own fields stops computing them.
- **`/A` on a widget goes in both parts.** ISO 19005-2 §6.4.1 forbids `/A` and `/AA` on a widget
  annotation or field dictionary; ISO 19005-4 §6.4.1 forbids `/A` and permits `/AA`. So a button
  that opened a URL through its `/A` action loses it whichever target is chosen — and in PDF/A-4
  the equivalent written as an `/AA` `/U` entry survives, which is a difference worth knowing
  before deciding it is impossible.
- **PDF/A-4 states a workaround, and it is only available there.** §6.4.1 says that a processor
  which removes JavaScript but still needs to keep an interactive form's values or logic shall
  store it as an embedded XFDF file (ISO 19444-1) whose file specification carries
  `/AFRelationship` `FormData`. The behaviour stops working and is not lost: what the form
  computed is written down in a format that outlives it.
- **But that workaround is itself an embedded file, and §3.1 governs it.** An XFDF file is XML,
  not PDF/A, so ISO 19005-4 §6.9 excludes it and **only PDF/A-4f can hold it**; ISO 19005-2 §6.8
  excludes it outright. So the standard's own archival path for form logic **is not available for
  a PDF/A-2 target at all** — converting a live form to PDF/A-2 loses its behaviour with nowhere
  to put it, and the only mitigation is outside the file: keep the source. A converter has to say
  this at the start rather than discover it at the end.

### 3.4 XFA forms

ISO 19005-2 §6.4.2 and ISO 19005-4 §6.4.2 both forbid `/XFA` in the AcroForm dictionary and
`/NeedsRendering` in the catalog.

- **Class: Ask**, and usually a large loss: a *dynamic* XFA form's pages are generated by the XFA
  processor, and what a PDF viewer draws is a placeholder.
- **What makes it survivable**: ISO 32000-2 Annex K requires a conforming hybrid file's AcroForm
  entries to be consistent with the XFA information, so the AcroForm this tree already reads *is*
  the form for a static XFA document. ISO 19005-2 Annex D describes moving the XFA dataset into
  the file rather than losing it; ISO 19005-4 §6.4.1's XFDF route (§3.3) is the newer equivalent.
- **Default: keep the data, drop the `/XFA` key, and refuse a dynamic form outright** — for a
  dynamic one the AcroForm is not the document and the conversion would produce a placeholder
  page wearing a conformance claim. `CLAUDE.md` excludes XFA rendering, so this project cannot
  flatten one and will not pretend otherwise.

### 3.5 Encryption, and the permissions that went with it

Both parts forbid `/Encrypt` (§6.1.3 in each). Removing it is trivial and is not a technicality:

- the document's confidentiality is gone — an archived copy is readable by anyone who holds it;
- Table 22's `/P` permission flags are gone with it, so "no printing", "no extraction" and the
  rest stop being asserted;
- ISO 19005-2 §6.1.7.2 and ISO 19005-4 §6.1.6.2 additionally forbid the `Crypt` filter unless it
  is `Identity`, so per-stream encryption goes too.

**Class: Ask, always, and never a default.** This project's own position on document restrictions
(`CLAUDE.md` principle 3's second half) is that they are the *reader's* to set — but that is about
a reader choosing to ignore them, not about a converter silently stripping them from a file that
will be handed to somebody else. The converter states plainly that the output is unencrypted and
carries no permissions, and does it only when told.

### 3.6 Digital signatures

Both parts permit signatures (ISO 19005-2 §6.4.3 and Annex B; ISO 19005-4 §6.5). **Conversion
invalidates every one of them, and nothing can prevent that.**

- A signature covers a byte range of a specific file. Conversion is a *rewrite* — RFC 0002 §10's
  serializer emits a new object table, new streams and a new cross-reference — so every byte
  offset moves and every digest fails.
- **An incremental update (§7.5.6) does not rescue it either, and it is worth being exact about
  why.** Appending — which is how this tree writes an annotation or a field value — leaves the
  bytes an existing signature covers untouched, so that signature's own digest still verifies.
  What it cannot leave untouched is the *meaning*: a conversion changes objects that already
  exist (a font dictionary gaining an embedded program, a catalog gaining `/Metadata`, a stream
  changing filter), and appending new versions of those is exactly what a certification
  signature's `/DocMDP` transform detects and reports as an alteration. And removing `/Encrypt`
  cannot be done by appending at all, because encryption is a property of the whole file. So the
  honest summary stands: a converted document does not carry its source's signatures.
- **Class: Ask, loudly.** The report names each signature, its signer and whether it currently
  validates, and says that the output will carry none of that. The sensible default is to keep
  the signature *fields* and their appearances (they are annotations, and §6.3.3 requires
  appearances) while stating that the cryptographic assertion is gone.
- ISO 19005-2 §6.1.12 and ISO 19005-4 §6.1.11 additionally allow only `UR3` and `DocMDP` in a
  permissions dictionary, and ISO 19005-2 §6.1.12 strips three keys from a `/DocMDP` signature
  reference. Both are removals from a structure the conversion has already invalidated.

### 3.7 Hidden annotations cannot stay hidden

ISO 19005-2 §6.3.2 and ISO 19005-4 §6.3.2: every annotation but `Popup` shall carry `/F`, and
where present its `Print` bit shall be 1 and its `Hidden`, `Invisible`, `ToggleNoView` and
`NoView` bits shall be 0.

- **Class: Ask.** An annotation the producer hid has two futures and no third: it becomes
  **visible and printable**, or it is **removed**. Both change the document.
- **Default: remove, and report.** A hidden annotation was hidden on purpose — a review comment,
  a redaction marker's companion, a conditional stamp — and making it visible is the more
  surprising of the two outcomes. The user can ask for the other.

### 3.8 Optional content whose states were automatic

ISO 19005-2 §6.9 forbids `/AS` in any optional content configuration dictionary; ISO 19005-4 §6.10
permits it and requires a conforming processor to ignore it. Both require every configuration to
carry a unique `/Name`, and an `/Order` array (where present) to reference every OCG in the file.

- **Class: Ask** for PDF/A-2: `/AS` is what switches layers by zoom, by print-versus-view or by
  user event, and removing it freezes the document into one state.
- **Default: keep the `/D` default configuration exactly as the producer set it** — which is what
  both parts require a reader to render anyway — and report which automatic behaviours stopped.
- `/Name` and `/Order` are **Default** work: a missing name is synthesised (`Configuration 1`,
  unique within the file) and a partial `/Order` is completed with the OCGs it omits, in the
  file's own order.

---

### 3.9 A metadata property its own schema does not define

**The largest single thing between this converter and the corpus**, measured in session 949: of the
PDF/A-2b documents the converter refuses, 273 are refused on this one requirement, and for 272 of
them it is the only one.

ISO 19005-2 section 6.6.2.3.1 requires every property an XMP packet states to *use* one of the
predefined schemas — the XMP Specification's, ISO 19005-1's, this part's — or an extension schema
complying with section 6.6.2.3.2. A packet can name a predefined schema's namespace and still not
use it: `veraPDF test suite 6-6-2-3-1-t01-fail-a.pdf` states `xmpDM:projectRef` as a simple text
value where the Dynamic Media schema defines it as a structured type. The namespace is right and
the value is not one the schema describes.

**Why this is a loss and not a default.** Three routes exist and two are closed:

- *Correct the value to the type the schema defines* — but the schema says what shape a value has,
  not what value this document meant, so a converter doing this is inventing content. `A48`'s line
  forbids it: state an interpretation the standard defines; never fill in an absence.
- *Declare the property in an extension schema container* — section 6.6.2.3.2's container is for
  **extension** schemas, and `xmpDM` is not one. Describing a predefined schema as an extension
  would misrepresent it in the file itself.
- *Remove the property* — which loses what the producer wrote, and is therefore §3's kind of
  answer rather than §4's.

- **Class: Ask.** The report names each property removed, its namespace and the value that was
  there, so a user can put it back by hand or supply a corrected source. Removing metadata is
  cheap to describe and impossible to undo from the output alone, which is exactly the shape §3
  exists for.
- **The refusal stays the default.** A conversion run without the authorisation refuses and names
  the properties, because a document losing metadata silently is the failure this whole section is
  written against.

---

## 4. Choices the source does not make, where a default is right

This is the class the user's ICC example belongs to. Nothing is lost; something must be *chosen*,
and choosing well is most of what makes a converter usable.

### 4.1 The output intent, and the sRGB default

ISO 19005-2 §6.2.4.1 and ISO 19005-4 §6.2.4.1 require every colour to be device-independent —
either directly, or indirectly through the destination profile of a PDF/A output intent. §6.2.4.3
in each part then says exactly when a device colour space is allowed: `DeviceRGB` needs a
`DefaultRGB` or an RGB output intent, `DeviceCMYK` a `DefaultCMYK` or a CMYK output intent, and
`DeviceGray` a `DefaultGray` or (in -2) any PDF/A output intent at all.

Almost every real document uses `DeviceRGB` or `DeviceGray` somewhere. So almost every conversion
must add an output intent, and that means shipping a profile.

- **Default: the ICC's own v2 sRGB profile**, embedded as `/DestOutputProfile` with `/S`
  `GTS_PDFA1` (both parts require that value: -2 §6.2.3, -4 §6.2.3). v2 satisfies both parts'
  profile-version constraints with one file.
- **That default is a convention, and is documented here as one.** ISO 32000-2 §8.6.4.1 gives the
  device colour spaces no colorimetric meaning at all: their values "map directly (or by simple
  conversions) to the application of device colourants", and "the results might not be consistent
  from one device to another". No clause derives sRGB from that, and a document that says nothing
  about its colour has not said sRGB. What can be said is
  that sRGB is what an unmanaged `DeviceRGB` has been shown as on screen for twenty-five years,
  so writing it down changes what the file *asserts* while leaving what a reader *sees*
  approximately where it was. Principle 5's rule for exactly this case is to make the choice
  deliberately and record it as a choice rather than presenting it as derived.
- **Overridable** with `--output-intent-profile`, for a document that was produced for a press.
- **What it costs, and it is not nothing.** Both parts make the output intent's profile the source
  space a conforming reader uses for device colours (§6.2.4.3) *and* the default blending space
  for transparency (-2 §6.2.10, -4 §6.2.9). Adding one therefore changes what every device colour
  in the document means to a conforming reader, and can change how transparent groups composite.
  For sRGB and screen-produced content the change is nil to imperceptible; for a document whose
  `DeviceCMYK` was meant for a specific press it is real, and that is exactly when the user should
  be supplying the press profile instead.
- **Licensing, and the decision**: the ICC's sRGB profile is redistributable under a permissive
  grant with two conditions (ship unchanged with its copyright tag; do not use ICC's name to
  advertise). **`A18` decides it: ship the standard sRGB profile, with a flag to override it.**
  This entry is no longer blocked.
- **`A18` attaches a condition, and it is the interesting half of the answer.** The report must say
  that adding an output intent *reinterprets the marks already in the file* — the difference has to
  be visible in the report, not only in the bytes. That follows from the bullet above rather than
  contradicting it: the change is nil to imperceptible for screen-produced content and real for a
  `DeviceCMYK` meant for a press, and a converter cannot tell the user which case they are in
  unless it says what it did. `doc/adr/0927` has the argument, which is that this is *stating an
  interpretation the standard defines* rather than filling in an absence — §10.4.2 already decides
  what this renderer shows for a `DeviceRGB` colour, and the output intent records it.
- **PDF/A-4 adds a second option worth knowing**: §6.2.3 allows a *page-level* output intent, so a
  document mixing an RGB body with CMYK inserts can carry the right profile per page instead of
  one compromise for all of them. PDF/A-2 has no such thing.

### 4.2 Metadata: XMP, the identification schema, and `/Info`

- **The XMP packet is required.** Both parts require `/Metadata` on the catalog (-2 §6.6.2.1,
  -4 §6.7.2.1). **Default: synthesise one** where the document has none.
- **The identification schema is required and is the conformance claim itself.** ISO 19005-2 §6.6.4
  requires `pdfaid:part` = 2 and `pdfaid:conformance` = A, B or U; ISO 19005-4 §6.7.3 requires
  `pdfaid:part` = 4 and, newly, `pdfaid:rev` — the four-digit year of the revision — with
  `pdfa:conformance` present only for `E` (PDF/A-4e) or `F` (PDF/A-4f). Note the prefix change
  in part 4's table: the conformance property is `pdfa:conformance`, not `pdfaid:conformance`.
  **Mechanical**, and the one place the converter states a claim about its own output.
- **`/Info` goes in opposite directions in the two parts, and this surprises people.**
  ISO 19005-2 §6.6.3 permits a document information dictionary, requires a conforming reader to
  ignore it, and asks a writer to keep it consistent with XMP through Table 7's crosswalk
  (`Title`→`dc:title`, `Author`→`dc:creator[0]`, `Keywords`→`pdf:Keywords`, and so on).
  **ISO 19005-4 §6.1.3 forbids `/Info` in the trailer** unless the catalog has `/PieceInfo`, and
  even then it may contain only `/ModDate`.
  - Converting to **PDF/A-2**: **Default** — copy `/Info` into XMP through the crosswalk, keeping
    both in step.
  - Converting to **PDF/A-4**: **Ask, once** — the document information dictionary is *deleted*.
    Its values survive in XMP if they were copied there first, and a user who looks at
    File → Properties in another reader may notice fields that used to be filled. Copy first,
    then delete, and say so.
- **Custom XMP properties need a schema, or they go.** ISO 19005-2 §6.6.2.3 requires every property
  to come from a predefined schema or from an *extension schema whose description is embedded in
  the file*, using the container schema of §6.6.2.3.3. A document carrying a producer's private
  XMP property with no description is not conforming.
  - **Default: describe them** — emit an extension schema container naming each unknown property,
    its value type and a description. This is the one place the converter writes metadata *about*
    metadata, and it is authoring in a small way; the alternative is deleting the producer's
    properties, which is worse. **Ask** where a property's value type cannot be determined.
  - ISO 19005-4 §6.7.2.3 replaces this with a *should*: an associated file carrying an ISO 16684-2
    schema description. Softer, and easier to satisfy.
- **`xmpMM:History`.** Both parts (-2 §6.6.6, -4 §6.7.5) ask a converter to record what it did,
  and name exactly the kind of thing this document is a list of — content or functionality
  altered, metadata handled, objects not retained. Every **Ask** above writes one entry.
  **Mechanical**, and it is the audit trail that makes the Asks defensible.
- **The trailer `/ID`.** ISO 19005-2 §6.1.3 requires it; both parts require the changing half to be
  updated when an `xmpMM:History` entry is added (-2 §6.6.5, -4 §6.7.4). **Mechanical.**

### 4.3 Text extraction: `ToUnicode`, and the difference between -2u and -4

ISO 19005-2 §6.2.11.7 requires a `ToUnicode` CMap on every font, with four exemptions (predefined
Mac/WinAnsi encodings; Type 1 and Type 3 fonts whose glyph names are all in the Adobe Glyph List
or the Symbol set; Type 0 fonts on the four named Adobe character collections; non-symbolic
TrueType) — **but only for Level A and Level U**. §6.2.11.7.3 adds, for Level A only, an
`/ActualText` wherever a character maps into the Unicode Private Use Area.

ISO 19005-4 §6.2.10.7 states the same rule with the same four exemptions as a ***should***, and
§6.2.10.8 makes the PUA `/ActualText` a *should* as well — with one *shall*: `/ActualText` must
not itself contain PUA values.

- **PDF/A-2b: Mechanical** (the requirement does not apply).
- **PDF/A-2u: Ask.** The converter can synthesise `ToUnicode` from a font's `cmap` or its glyph
  names for many fonts, and for a symbolic subset font with private-use glyph names it cannot —
  there is no evidence in the file for what those codes *mean*, and manufacturing one would be
  manufacturing the evidence the level exists to require. Where that happens, -2u is unreachable
  and -2b is; the report names the fonts. **§2.1 is the same case seen from the font's side**, and
  it is the clearest example of a document that is archivable while a claim about it is not.
- **PDF/A-4: Mechanical**, because the requirement is a recommendation. A conversion to PDF/A-4
  never fails on text extraction, which is worth knowing when a document's text matters less than
  its survival.

### 4.4 Annotation appearances

ISO 19005-2 §6.3.3 requires every annotation — widgets included — to have an appearance
dictionary, except those with an empty `/Rect` and those whose subtype is `Popup` or `Link`; the
appearance dictionary shall contain only `/N`; and a conforming reader renders that appearance and
ignores `/C`, `/IC`, `/Border`, `/BS`, `/BE`, `/CA`, `/DA`, `/Q`, `/DS`, `/LE`, `/LL`, `/LLE`,
`/Sy`. **ISO 19005-4 §6.3.3 states the requirement differently and it matters**: it gives the
`/N`-only rule and requires appearance content to obey §6.2, but the obligation to *have* an
appearance is not restated there — its NOTE 1 attributes that to ISO 32000-2's own Table 166 and
the paragraph following it. So a converter must not read part 4's silence as permission: what
changes between the parts is where the requirement is written, not whether it applies. Both parts
require `NeedAppearances` to be absent or false (-2 §6.4.1, -4 §6.4.1).

- **Default: construct the missing appearance.** ISO 32000-2 §12.5.5 and §12.7.4.3 make the
  appearance the standard's own construction rather than this program's invention, and
  `pdf-model`'s `appearance.rs` and `variable_text.rs` already build them — which is why this sits
  inside ADR 0816's fence rather than outside it.
- **Report every one written**, because a constructed appearance is *this program's* rendering of
  a field or a markup, and a different reader's would differ in detail. **`A21` confirms both
  halves**: constructed appearances are allowed, and every appearance written is reported — the
  difference between the producer's file and ours has to be visible in the report rather than only
  in the bytes.
- **`NeedAppearances` true is the interesting case**: it means the producer deliberately left
  appearances to the reader. Turning it false without constructing appearances would blank the
  form; constructing them freezes this program's rendering into the archive. **Ask.**

### 4.5 Colour space bookkeeping

- **`Colorants` becomes mandatory.** ISO 19005-2 §6.2.4.4 and ISO 19005-4 §6.2.4.4 require an entry
  in the `/Colorants` dictionary for every spot colour used in a DeviceN or NChannel space, which
  ISO 32000-2 leaves optional. **Default: synthesise it** from the DeviceN's own tint transform —
  mechanical, and no mark changes.
- **Separation arrays with the same name must agree.** Both §6.2.4.4 require every `Separation`
  array in the file sharing a colourant name to have the same `tintTransform` and `alternateSpace`,
  compared as PDF objects rather than by result. A document assembled from several producers
  routinely violates this.
  - **Ask.** Making them agree means choosing one definition and rewriting the others, and the two
    definitions may genuinely render differently. The converter reports the disagreement, shows
    both, and rewrites only when told which one wins.
- **Overprint mode.** ISO 19005-2 §6.2.4.2 forbids `OPM` = 1 with an ICCBased CMYK space when
  overprinting is on. **Ask**: changing `OPM` changes how overlapping CMYK marks composite.

### 4.6 Transparency's blending space

ISO 19005-2 §6.2.10 and ISO 19005-4 §6.2.9: a conforming reader uses the PDF/A output intent as
the default blending space, and where the document has none, every page containing transparency
must carry a `/Group` whose `/CS` supplies one (in -4, or a page-level output intent).

- **Default: the output intent from §4.1 supplies it**, and nothing else is needed. This is the
  second reason the sRGB default earns its place.
- Where a page already states a group colour space, it is kept; §6.2.4's restrictions then apply to
  it, which can turn into §4.5's work.
- **PDF/A-2 and PDF/A-4 both permit transparency.** The flattening problem belongs to PDF/A-1
  alone, and PDF/A-1 is not a target here (§1), so `doc/rfc/0006` §5.2's hardest case **does not
  arise for this converter**. That is worth saying out loud: it was the single most expensive item
  in the original analysis, and `doc/questions/A17`'s "part 1 never" removed the occasion for it
  — which is also what `doc/questions/Q19`, "does the fence move for PDF/A-1", was asking about.

### 4.7 Filters, and other purely mechanical rewrites

**Mechanical unless the row says otherwise**, listed so that a user reading a diff is not
surprised by them:

| what changes | clause | note |
|---|---|---|
| `LZWDecode` → `FlateDecode` | -2 §6.1.7.2, -4 §6.1.6.2 | the *decoded* bytes are identical; the stream is not, and the file is usually smaller |
| `Crypt` filters removed | same | follows from §3.5 |
| file header rewritten | -2 §6.1.2 (`%PDF-1.n`), -4 §6.1.2 (`%PDF-2.n`) | plus the four high bytes both require |
| `/Version` in the catalog | -4 §6.1.12 | exactly three characters, `2.n` |
| hex strings padded to an even count | -2 §6.1.6, -4 §6.1.5 | |
| `endstream`/`endobj` whitespace normalised | -2 §6.1.7.1/§6.1.9, -4 §6.1.6.1/§6.1.8 | the serializer emits this shape anyway |
| `/Length` corrected | both | |
| `Interpolate` forced to false | -2 §6.2.8.1, -4 §6.2.7.1 | **not quite mechanical**: it turns image smoothing off, and a low-resolution image will look blockier. Ask where any image sets it true |
| `/Alternates`, `/OPI` removed from images | same | |
| `/OPI` removed from form XObjects | -2 §6.2.9.1, -4 §6.2.8.1 | -2 also forbids `/Subtype2` with the value `PS`, and `/PS`; -4 names only `/OPI`, because PDF 2.0 defines neither of the others |
| PostScript XObjects removed | -2 §6.2.9.3 | they carry PostScript for a PostScript back end and never contributed to a rendered page. **PDF/A-4 has no such clause because PDF 2.0 has no such object**: `PostScript XObject` and `Subtype2` appear nowhere in ISO 32000-2, so §6.1.1's rule about undescribed data covers them |
| `/Requirements` removed | -2 §6.11, -4 §6.12 | |
| `/AlternatePresentations`, `/PresSteps` removed | -2 §6.10, -4 §6.11 | slide-show behaviour stops |
| linearisation lost | -2 §6.1.11, -4 §6.1.10 | both parts permit a linearised file and neither requires one; the serializer writes an ordinary one, so a source optimised for first-page-over-a-network stops being so. Nothing renders differently |
| names checked as UTF-8 | -2 §6.1.8, -4 §6.1.7 | a font or colourant name that is not valid UTF-8 is a **Refuse**: renaming it would break the references that use it |

### 4.8 Halftones and transfer functions

ISO 19005-2 §6.2.5 and ISO 19005-4 §6.2.5: no `/TR`; `/TR2` only with the value `Default`;
halftones only of type 1 or 5 and without `/HalftoneName`; and no `/HTP` — which part 2 spells
`HTP`, explaining in its NOTE 1 that the key was removed by PDF 1.3, while **part 4 as published
spells it `HTO`**. ISO 32000-2 defines no `HTO`, so this reads as a typographic error for `HTP`
and is treated as one here; it is the kind of thing the published PDF/A-4 errata exist for, and
this file records the reading rather than assuming it.

- **Class: Ask, not Mechanical**, and this is easy to get wrong. `CLAUDE.md` records that this
  project used to call transfer functions inapplicable and was **wrong** — §10.5's transfer
  functions decide what a *screen* shows, and this tree implements them because a corpus document
  drew incorrectly without them. So removing a `/TR` is not tidying up a print-only key: it can
  change the rendered image, sometimes drastically (an inverting transfer function is a
  photographic negative).
- **Default: report the transfer function's effect** — the converter can render the page both ways
  and say whether anything changed — and remove it only with that shown. Where it is the identity,
  removal is silent and safe.

### 4.9 Fonts that were never embedded — substitute, and say so

Both parts require every font used for rendering to be embedded (-2 §6.2.11.4.1,
-4 §6.2.10.4.1), and a document that names Frutiger and embeds nothing does not contain those
outlines. **The default is to substitute a face and embed it.** This section argues that, because
the first version of this file said the opposite and the argument against it is decisive.

#### Why refusing is the worse answer

1. **A substitute is already what the reader sees.** A PDF whose font is not embedded has no
   appearance of its own: every viewer picks a face at display time, and they pick different ones.
   Embedding one *removes* that indeterminacy, which is what an archival format is for.
2. **Refusing preserves nothing.** The document does not stay as it was — it stays unarchived, and
   the set of faces a future system will substitute from is further from the producer's than
   today's is. A refusal declines to act; it does not conserve.
3. **The standard contemplates it.** ISO 19005-2 §6.6.6's NOTE 1 and ISO 19005-4 §6.7.5's NOTE
   give, as their examples of a converter action that changes the document's appearance and should
   therefore be recorded in `xmpMM:History`, downsampling and **font substitution**. A standard
   that names the act and tells you where to write it down has not forbidden it.
4. **§6.2.11.1's intent clause does not forbid it either, on its own terms.** That clause asks
   that future rendering match, glyph by glyph, the static appearance of the file *as originally
   created*. For a file that never embedded the font there is no such appearance — it was a
   function of whatever machine opened it. The intent is unachievable for this file whichever
   choice is made, which is why substitution must be *reported*, not why it must be refused.

#### The engineering constraint that makes it a real feature

Substituting is not "pick a face and embed it", and this is the part a user should know before
believing it is easy.

- **The page does not reflow, and this file previously said it might.** Glyph advances come from
  the font *dictionary* — `/Widths` for a simple font, `/W` and `/DW` for a CIDFont — not from the
  font program. ISO 32000-2 §9.2.4 is explicit about why the width is stored in both places:
  "Storing this information in the font dictionary, although redundant, enables a PDF processor to
  determine glyph positioning without having to look inside the font program." So line breaks,
  word positions and page geometry are fixed by the content stream and survive substitution
  untouched. What changes is the shape of each glyph, drawn at a position the file already
  determined.
- **But PDF/A then requires the two to agree.** ISO 19005-2 §6.2.11.5 and ISO 19005-4 §6.2.10.5
  require the glyph widths in the font dictionary and in the embedded program to be consistent to
  within 1/1000 unit. A substitute whose advances differ from the `/Widths` already in the file is
  therefore **not conforming**, and there are exactly three ways out:
  1. **Use a metric-compatible face.** The substitution is then free. This covers the commonest
     case by far — the standard 14, for which metric-compatible open families exist, and which
     §6.2.11.4.1's NOTE 5 explicitly refuses to exempt from embedding.
  2. **Adjust the embedded program's advances to match `/Widths`.** Outlines untouched, metrics
     rewritten to the numbers the file already states. Mechanical, and it keeps the positions and
     the conformance together. It is font surgery, and it is the reason this default is a feature
     with a cost rather than a flag.
  3. **Rewrite `/Widths` to match the substitute.** **Never**: `/Widths` is what positions the
     glyphs, so this one *does* move the text.
  The converter does 1 where it can and 2 otherwise, and reports which.
- **The substitute must itself be legally embeddable** (§6.2.11.4.1). That rules out most fonts
  installed on the machine: a system Arial's OS/2 `fsType` bits usually permit preview and print
  rather than the unlimited universal embedding the clause requires. So the substitute has to come
  from a family this project may ship or may rely on being licensed for embedding. **`A47`
  answers it, and asks for more than this file had proposed**: ship a licence-clean OFL family
  with wide coverage, *including a Noto-CJK-class family*, under the same discipline as
  Liberation — the licence read off a copy, a row in `doc/third-party-data.md`, `/NOTICE`
  extended. Where no shipped face covers a document's characters, **refuse rather than guess**.
  §8.4 carries the packaging half of that answer, which is a naming requirement rather than a
  build detail.

#### What the converter therefore does

- **Default: substitute, embed, and report** — per font, naming the face requested, the face used
  and which of the two metric routes was taken, with an `xmpMM:History` entry as §6.6.6 asks.
- **`--no-substitute` for the user who wants the other behaviour**, which turns every such font
  back into a refusal. Batch archiving wants the default; a curator checking one document may want
  the flag.
- **It can measure the cost, which is unusual and worth using.** This program is also a renderer,
  so it can rasterise the source and the output through the same path and report the difference
  per page. The number is not decoration: it makes "this file now renders as you saw it" a
  *verified* claim rather than a promise — and where the substitution is poor the difference says
  so before anybody archives it.
- **§2.1 remains the exception**: where a code cannot be named, substitution draws the wrong
  character rather than a different-looking one, and that font refuses.

---

### 4.10 JPEG 2000 images, and where the offending field lives

ISO 19005-2 §6.2.8.3 and ISO 19005-4 §6.2.7.3 place seven restrictions on JPEG 2000 data, and
until this was implemented nobody here could see any of them. The useful thing a user needs
told is not the list — it is that **the restrictions divide by where the field sits**, and that
division decides whether the fix costs anything at all.

A JPEG 2000 image in a PDF is a JP2 wrapper (ISO/IEC 15444-1 Annex I.4's boxes) around a
codestream. `colr`, `ihdr`, `bpcc`, `cdef` and `cmap` are **wrapper**; the `SIZ` marker's
component count and per-component depths are **codestream**. Rewriting a box is byte surgery on
a hundred-odd bytes and touches no sample. Changing anything the codestream states means
decoding and re-encoding.

| restriction | clause | where the field lives | cost of the fix |
|---|---|---|---|
| exactly one colour specification marked best | -2 §6.2.8.3, -4 §6.2.7.3 | wrapper (`colr` `APPROX`) | **Mechanical** — mark one, drop the rest |
| colour specification method is one of the three permitted | same | wrapper (`colr` `METH`) | **Mechanical** where a permitted method describes the same colour; otherwise Ask |
| not the enumerated CIEJab colour space | same | wrapper (`colr` `EnumCS` 19) | **Ask** — the samples mean CIEJab, so replacing the box relabels them and changes the picture |
| 1, 3 or 4 colour channels | same | wrapper (`ihdr` `NC`, `cdef`) or codestream (`SIZ` `Csiz`) | **depends, and this is the interesting one** — see below |
| bit depth 1 to 38, the same on every colour channel | same | codestream (`SIZ` `Ssiz`), mirrored in the wrapper | **Re-encode** |
| the JPX baseline feature set | same | codestream | **not checkable here** — the feature set is defined by ISO/IEC 15444-2, which this project does not hold (`doc/questions/Q51`) |
| device colour spaces obey the device colour rules | same | wrapper | **not checkable here** — neither part says which enumerated colour space is *effectively* a device space |

**The channel-count row is where a converter earns its keep.** A two-channel image — greyscale
plus alpha — has two colour channels only because nothing says otherwise. A `cdef` box states
what each channel *is*, and one that declares the second channel as opacity leaves one colour
channel and a conforming image, without a sample being touched. That is not a trick: the box
records a fact about the data, and it is only available when the fact is true. Where the second
channel is genuinely a second colour, no box can say otherwise and the image must be re-encoded.

- **Class: Mechanical for the two wrapper rows above, Ask for the rest.**
- **The universal fallback is transcoding to `FlateDecode`**, and it means JPEG 2000 is never a
  hard refusal. The decoded samples are what any renderer would show, so re-encoding them
  losslessly loses nothing that was visible — at a large cost in file size, often ten times. It
  is the right default only when the alternative is refusing the document.
- **One honesty note about that fallback**: this project's own JPEG 2000 decoder is
  `hayro-jpeg2000` in the sandboxed worker, and `doc/conformance/ledger.toml`'s §7.4.9 row
  records that thirteen corpus codestreams still decode one level off the reference software.
  Transcoding puts that decoder's output into the archived file permanently, where leaving the
  codestream alone does not. Prefer the box rewrite wherever it is available, and say when it
  was not.

## 5. What this converter will not do, on its own rules rather than the standard's

Two refusals that are ours rather than ISO's — though on the first of them the standard turns out
to advise the same thing. A user should know these are policy, because a different tool will do
them. **Neither refusal is as wide as it first looks**, and §5.1 is the one this file got wrong on
its first attempt: what is refused is a specific act, not a whole conformance level.

### 5.1 It will not invent a structure tree — and that is not the same as refusing PDF/A-2a

**A tagged source can be converted to PDF/A-2a, and this file said otherwise in its first
version.** The correction is worth stating as a correction, because the mistake is the common one:
"we cannot auto-tag" was allowed to become "we cannot produce level A", and those are different
claims about different documents.

#### What Level A actually requires

§6.7 is short, and reading it against the modal verbs is what settles this. **Six requirements are
`shall`:**

| requirement | clause | can a converter supply it? |
|---|---|---|
| meet ISO 32000-1:2008 §14.8's Tagged PDF requirements | §6.7.2.1 | **no** — this is the tree itself |
| `/MarkInfo` with `/Marked true` in the catalog | §6.7.2.2 | **yes**, once the rest is verified — see below |
| a structure hierarchy rooted in `/StructTreeRoot` | §6.7.3.3 | **no** — the tree itself again |
| every non-standard structure type role-mapped, possibly indirectly, to a standard type | §6.7.3.4 | **only with the user's help** — the mapping is a statement about what the producer's own type *meant* |
| word boundaries present as space characters within show strings | §6.7.3.2 | **no** — it is a property of the content streams |
| a `/Lang` value, *where present*, that is a valid language identifier | §6.7.4 | **yes** — repairing or removing a malformed one |

**And everything a reader would call accessibility is `should`:** alternate descriptions for
images and formulae (§6.7.5), replacement text (§6.7.7), expansions of abbreviations (§6.7.8),
`/Contents` on non-textual annotations (§6.7.6), the default `/Lang` on the catalog and the
per-element languages (§6.7.4), and marking pagination furniture as artefacts (§6.7.3.1).

So **PDF/A-2a is a "tagged and Unicode-mapped" level rather than an "accessible" level**, and a
document with a complete structure tree and no `/Alt` on a single figure is Level A conforming.
That is not this project's reading being generous — it is what the modal verbs say, and a
validator that failed such a file would be inventing a requirement. (Whether such a file is any
*use* to a screen-reader user is a different question, and the answer is often no; PDF/UA
(ISO 14289) is the standard that asks for that, `doc/md/` carries both its parts, and a converter
can report the gap without failing the file.)

#### What the converter therefore does

1. **The source satisfies §6.7's `shall`s.** Level A is *declared*: validate, then write
   `pdfaid:conformance` `A`. No content changes. This is the case the first version of this file
   wrongly excluded. **How common it is has two answers and they are not in tension**: across
   documents in general, tagged ones are a small minority — the shape `tools/state.sh` and the
   accessibility census print, and the reason `doc/rfc/0006` §5.7 says a converter offering
   auto-tagging would be inventing the semantics of nearly everything handed to it. Across
   documents whose owner *asks for Level A*, it is the normal case, because the people who ask are
   the people who tagged.
2. **The source is tagged and the gaps are ones the file itself answers.** Fill them and report
   each: `/MarkInfo /Marked true` where the tree is there and the flag is not; a `ToUnicode` CMap
   derived from a font's own `cmap` (§6.2.11.7's `shall`, which applies to U and A alike);
   a malformed `/Lang` repaired to a valid identifier. Each of these writes down something the
   file already demonstrates.
3. **The source is tagged and a `shall` needs a judgement.** An unmapped non-standard structure
   type is the one that occurs in practice, and it is **Ask**: the converter lists each unmapped
   type with the elements that use it and the standard types it could map to, and the user says
   which. It does not guess, because `Chapter` → `Sect` is a guess that happens to be right and
   `Sidebar` → `Note` is a guess that may not be.
4. **The source is untagged, or its show strings carry no word boundaries.** **Refuse Level A**,
   name why, and offer -2u or -2b, which the same document usually reaches without difficulty.

#### The refusal that remains, and the standard agrees with it

Auto-tagging — deciding that this run of glyphs is a heading and that one a table cell, what the
reading order of a two-column page is, and what an image *depicts* — stays out. It is
`CLAUDE.md`'s "authoring content from nothing", and a wrong reading order is worse than none
because it misleads confidently.

**ISO 19005-2 §6.7.1 takes the same position in its own words**: it advises writers not to add
structural or semantic information that is not explicitly or implicitly present in the source
material solely to achieve conformance, and its NOTE 2 calls it inadvisable to generate such
information by automated processes without appropriate verification. So the refusal is not this
project being stricter than the standard — the standard advises against exactly the thing being
refused. What neither the standard nor this project refuses is declaring conformance for a
document that already carries the information.

ISO 19005-4 has no conformance levels and its §6.8 is a single paragraph of encouragement pointing
at PDF/UA, so none of this arises for a PDF/A-4 target.

### 5.2 It will not edit content streams to reach conformance

ADR 0816's fence: an operation that invents marks is out of scope. Several requirements can only
be met by rewriting a content stream — `.notdef` references (§2.2), `q`/`Q` nesting past 28
(§2.5), operators that ISO 32000-2 deprecates in a PDF/A-4 target (§6). The converter refuses
those rather than rewriting the producer's page.

**One case was genuinely arguable and `A50` has decided it**: replacing the deprecated `F`
operator with `f`, which ISO 32000-2's Table 59 documents as equivalent
("Equivalent to `f`; deprecated in PDF 2.0"). It changes a byte in a content stream and cannot
change a mark. The fence is drawn at **marks**, so the substitution is allowed — and the answer
fixes its width in the same sentence, which is what keeps this section true rather than
contradicted:

- a **closed list** of operator spellings the standard itself documents as equivalent, and nothing
  reached by analogy from that list;
- applied **only where a target's deprecation rule requires it**, never as tidying;
- **reported**, like every other thing this converter writes;
- and explicitly **no wider licence to rewrite content streams** — the `.notdef` and `q`/`Q`
  refusals above are untouched by it.

So the heading still holds. The converter does not edit content streams to reach conformance; it
respells one operator, from a list, where a rule names it.

---

## 6. Limitations that come from PDF/A-4 being PDF 2.0

ISO 19005-4 §5.1 contains a sweeping sentence that has no counterpart in part 2: features that
ISO 32000-2 describes as **deprecated shall not be used**. Every deprecation in a 1000-page
standard becomes a conversion requirement, and the common ones bite ordinary files:

| deprecated in ISO 32000-2 | where in ISO 32000-2 | conversion |
|---|---|---|
| the document information dictionary | §14.3.3 | §4.2 — deleted, values moved to XMP |
| `/ProcSet` in a resource dictionary | Table 34 | **Mechanical** — removed. Present in almost every file written before 2008 |
| `/CIDSet` in a CIDFont descriptor | Table 122 | **Mechanical** — removed. Note that PDF/A-2 §6.2.11.4.2 *constrains* it instead |
| `/TR` and `/TR2` transfer functions | Table 57 | §4.8, and -4 §6.2.5 forbids them independently |
| a blend mode written as an **array** | Table 57, §11.3.5 | **Mechanical** — reduced to the name a reader would have chosen |
| `/NeedAppearances` | Table 224 | §4.4 |
| `/XFA`, `/NeedsRendering` | Table 224, Table 29 | §3.4 |
| the `F` fill operator | Table 59 | §5.2 — allowed by `A50`, narrowly and reported |
| Adobe-Korea1 and Adobe-Japan2 character collections | §4.2, established notations | a Type 0 font on one of them loses -4's `ToUnicode` exemption (§4.3), which in -4 is only a *should* |
| RC4, AES-128, security handler revisions 1–5 | §7.6 | moot: encryption is forbidden outright (§3.5) |

**A tension in part 4 worth naming rather than resolving**: §5.1 forbids deprecated features, and
§6.1.11 explicitly permits `UR3` in a permissions dictionary — which ISO 32000-2's Table 263 marks
deprecated in PDF 2.0. The two sentences point opposite ways for the same key. The converter takes
the specific clause over the general one (`UR3` may stay), reports that it did, and this paragraph
is here so the decision is visible rather than buried.

**Converting the other direction has its own version cost.** A PDF 2.0 source targeting PDF/A-2
must be written with a `%PDF-1.n` header (§6.1.2), which means every PDF 2.0-only construct in it
is either translated or is a **Refuse**. That is the mirror of this section and is the reason
PDF/A-4 is the natural target for anything recent.

---

## 7. What the converter promises about its own verdict

The last limitation is about the tool rather than the file, and it is the one an archivist will
ask about.

**A clean report means "every requirement this program checks was met", and the program says which
requirements those are.** `doc/rfc/0006` question 6 settles the discipline: a requirement whose
clause nobody here has read, or which is not yet implemented, is reported **by name as not
checked** — never silently omitted, and never implemented from somebody else's rule set. A
validator that omits a check silently is indistinguishable from a document that passes it.

Three consequences a user should hold on to:

1. **A pass is relative to a stated list.** The list is printed with the verdict and is the
   conformance ledger's discipline applied to a second standard.
2. **Disagreement with veraPDF is a question, not a defect.** Where this program and veraPDF
   differ, the answer is in ISO 19005's clause, and the ledger records which reading won and why.
   Agreement is evidence that we read it right; it is not the definition of right.
3. **The output is a new file.** Not an incremental update — the serializer writes a fresh object
   table and cross-reference. Keep the source: the conversion is not reversible, `xmpMM:History`
   records what was done rather than how to undo it, and §3.6's signatures are the sharpest
   reason.

---

## 8. The questions this file could not answer, and the answers

**Every one of them is answered.** This section listed nine open questions when it was written on
2026-09-07; the owner answered the last of them on 2026-09-09, and what follows is what each
decided rather than what each asked. The `Q` files keep the arguments that raised them, which is
the point of keeping both.

**The decision that governs the rest is `A46`**: PDF/A is on, the hold `A15` and `A17` had placed
on it is over, RFC 0006 is ratified, the validator comes first — and **PDF/A-4 is to be finished
and certified first**. Everything below is downstream of that.

### 8.1 Four permissions, and the condition all four carry

| | decided | where it lands |
|---|---|---|
| **`A18`** | Ship the standard sRGB profile, with a flag to override it — **and report that adding an output intent reinterprets the marks already in the file** | §4.1 |
| **`A21`** | Constructed annotation appearances are allowed, and every appearance written is reported | §4.4 |
| **`A48`** | Both proposed constructions are allowed — the DeviceN `/DefaultCMYK` and the empty glyph in place of `.notdef` — reported per document and recorded in `xmpMM:History` naming the clause | §2.2, §10.1 |
| **`A50`** | The deprecated operator substitution is allowed **narrowly**: a closed list of spellings the standard itself documents as equivalent, only where a target's deprecation rule requires it, and reported. No wider licence to rewrite content streams | §5.2 |

The condition is the same in all four and the owner wrote it four different ways, so it is stated
once here: **what was written is reported** — named in the conversion's report, per document, not
merely inferable from a diff. That is what makes these permissions rather than a licence. A
converter that silently produced a conforming file would be indistinguishable from one that
produced a wrong file, and the difference this project cares about is not whether the output
conforms but whether the reader can see what was done to it.

`A48` states the line all four sit on, and it is sharper than §5's own fence:

> state an interpretation the standard defines; never fill in an absence

Three of the four satisfy it comfortably — a `DeviceRGB` colour already means something §10.4.2
decides, a constructed appearance writes down what §12.5.5 describes, and `F` and `f` are the same
operator differently spelled. **The empty glyph is the thin case**, and `doc/adr/0927` says so
rather than pretending otherwise: a code that reaches `.notdef` has no glyph the standard defines.
It is allowed with reporting attached, which is the right shape for a thin case, not a comfortable
one.

### 8.2 What the validator is held to

**`A20`** confirms the not-checked verdict as built — a requirement this project has not read is
reported not-checked, by name, per requirement, per target — and restates the rule that made it
necessary: **a check is never implemented from a secondary source.** The sharp consequence for
this file and for the code is that an `Unchecked` reason may never be weakened to make a row look
better. The reason *is* the report.

**`A22`** takes the recommended names: `quorra-transform archive` and `quorra-retrieve
archive-check`.

### 8.3 Two questions overtaken rather than decided

**`A49` is void.** It asked whether to buy ISO 32000-1:2008, PDF/A-2's base document, and the owner
obtained Adobe's free copy the day after it was asked. §1.2 above is the rewrite that follows.

**`A51` declines a purchase and closes a row for good.** ISO/IEC 15444-2 will not be bought, so the
JPX baseline-feature restriction in §4.10 stays unchecked with its truthful reason — not as a debt
but as a requirement whose defining text this project has decided not to hold. A
conforming-validator claim with no gaps is a *new* question, asked if it is ever wanted.
`doc/adr/0928` records it, including the finding that the 2016 and 2019 files in `doc/` are
fifteen-page previews and cannot settle anything.

### 8.4 The fonts answer, which is bigger than it looks

**`A47`** confirms substitution as the default rather than a refusal, and then asks for something
this file had not proposed: ship a licence-clean OFL family with wide coverage, **including a
Noto-CJK-class family**, under the same discipline as Liberation — the licence read off a copy, a
row in `doc/third-party-data.md`, `/NOTICE` extended. Where no shipped face covers a document's
characters, refuse rather than guess.

It also decides the packaging, and the instruction is unusually specific about *naming*: **two
downloads**, the default one including the universal font family, and a smaller one without it
**named so that it is unmistakably the incomplete one** — the name should say what is missing and
that the download is incomplete, so that a person who does not know what to download, or does not
care about the size, ends up with the bigger one. That is a defaults-and-naming requirement, not a
build-system detail, and §10 is where its consequences land.

### 8.5 The two decisions this file recorded before there was an answer file

They were taken in conversation on 2026-09-07 — that font substitution is the default rather than a
refusal (§4.9), and that a tagged source may be converted to PDF/A-2a (§5.1). Both overturned a
written position in this file. `A47` has since confirmed the first in writing. The second still
rests on the conversation alone, and remains the one decision here without an `A` file behind it.

---

## 9. When the target is not negotiable

Everything above that says *"and another part accepts it"* assumes the user may choose the target.
**Usually they may not.** A deposit rule, a regulator or a company archive names one — "PDF/A-4,
not 4f, not 4e" is a real example — and for that user "use PDF/A-4f instead" is not an answer, it
is a restatement of the problem.

So this section is the same information from the other side: **for a fixed target, what in a
source document forces a decision, and what the decision is.** Every row's answer is a loss,
because the alternative that was not a loss is the one the target rules out.

### 9.1 Target: PDF/A-4, and neither 4f nor 4e

The strictest of the three PDF/A-4 flavours and the one most likely to be mandated, because it is
the plain profile. What it costs:

| the source has | what §6 of the standard says | what you must do | how bad |
|---|---|---|---|
| **an attachment that is a PDF** | §6.9 permits it if the attachment itself conforms to ISO 19005-1, -2 or -4 | **convert the attachment too**, recursively, and keep it. `/AFRelationship`, `/F` and `/UF` become required on its file specification (§6.9) | **no loss** — this is the case most people expect to lose and do not |
| **an attachment that is not a PDF, or is one that will not convert** | Annex A.2 permits any type, and Annex A.2 is 4f | **drop it**, named and recorded | **loss, and unavoidable at this target** |
| **a `FileAttachment` annotation** | §6.3.1 permits it only in 4f | **remove the annotation** (and the file with it, per the row above) | loss |
| **3D or `RichMedia`** | §6.3.1 permits them only in 4e | **remove them** | loss, and total for a 3D document |
| **`Sound`, `Screen`, `Movie` annotations** | §6.3.1 forbids them at every flavour | remove | loss |
| **JavaScript** | §6.6.2 **permits** it, user-invoked only | **keep it** | **no loss** — and note this is the reverse of PDF/A-2 |
| **`/AA` on widgets** | §6.6.3 permits it | keep | no loss |
| **form logic you would rather archive as data** | §6.4.1's XFDF route needs an embedded file, which needs 4f | not available at this target; the JavaScript itself may stay instead | no loss in practice, because of the row above |
| **`/Info`** | §6.1.3 forbids it unless `/PieceInfo` exists, and then only `/ModDate` | copy to XMP, then delete | metadata moves; a reader looking at File → Properties sees a change |
| **anything ISO 32000-2 deprecates** | §5.1 forbids deprecated features | §6's table — `/ProcSet`, `/CIDSet`, `/TR`, `/NeedAppearances`, array blend modes and the rest | mechanical |
| **a font with no embedded program** | §6.2.10.4.1 requires embedding | §4.9 — substitute and embed | appearance frozen to ours; reported |
| **codes no method can name** | §6.2.10.7's `ToUnicode` is a `should` here | convert anyway, record the recommendation as unmet | **no loss of conformance** — this is where -4 is *easier* than -2u |
| **a page over 14 400 units, deep `q`/`Q` nesting, a 40-ink DeviceN** | part 4 states no implementation limits | keep | **no loss** — -4 is where these documents can go |
| **`DeviceCMYK` content** | §6.2.4.3 needs a CMYK output intent or a device-independent `DefaultCMYK` | supply a CMYK profile, **or** take §10.1's spec-defined fallback | not blocked, but the colour is approximate unless you supply the profile |

**The shape of that table is the finding.** For a PDF/A-4-only archive the losses are narrow and
nearly all of them are *attachments and multimedia*: a document with no attachment, no 3D and no
sound converts with nothing lost but its `/Info` dictionary and its deprecated keys. The two
things that can stop it are a non-PDF attachment and CMYK content without a profile.

### 9.2 Target: PDF/A-2b

| the source has | what you must do | how bad |
|---|---|---|
| **an attachment of any kind that is not PDF/A-1 or -2** | §6.8 — convert it or drop it | loss where it will not convert |
| **JavaScript, `/AA`, `Launch`, `Hide`, `SetOCGState`…** | §6.5.1/§6.5.2 — remove | **loss, and larger than at -4**: a computing form stops computing, with no XFDF route available either (§3.3) |
| **`/AS` on an optional content configuration** | §6.9 — remove | automatic layer switching freezes |
| **a page over 14 400 units, `q`/`Q` past 28, DeviceN over 32** | §6.1.13 — nothing can be done | **refused**, and this is the one class -2 cannot take at all |
| **a PDF 2.0 source** | §6.1.2 needs a `%PDF-1.n` header | every 2.0-only construct is translated or refused (§6) |
| **codes no method can name** | nothing — §6.2.11.7 does not apply at Level B | no loss |

### 9.3 Target: PDF/A-2u or PDF/A-2a

Everything in §9.2, plus: **-2u fails on any font whose codes cannot be mapped to Unicode**
(§6.2.11.7.2, §4.3), and **-2a additionally fails on an untagged document, on show strings with no
word boundaries, and on an unmapped non-standard structure type** (§5.1). Neither gap can be
filled by this converter without manufacturing the evidence the level exists to require.

### 9.4 What a fixed target changes about the report

One consequence for the verb rather than the file: with a free choice, the useful report says
*"this cannot be -2, can be -4f"*. With a fixed target it must instead say **"to reach the target
you named, these five things will be removed"** — and then stop, because that list is a decision
the user makes once for a whole archive rather than per document. A converter for this user is a
policy engine with a dry-run mode, not a question-asker.

---

## 10. The resources this needs, and how hard each is to get

The short answer to "is getting these for our licence difficult?": **mostly no, and less than you
would expect — the fonts for the common case are already in this tree under licences that permit
embedding. sRGB is one small open decision. A CMYK profile is the only thing you may have to buy,
and even there the standard defines a fallback.**

| resource | needed for | state |
|---|---|---|
| **substitutes for the standard 14** | §4.9 — by far the commonest non-embedded case | **already shipped.** `data/standard-fonts/` carries ten Foxit faces (Courier, Times, Symbol, ZapfDingbats) under **BSD-3-Clause** and four Liberation Sans faces (standing in for Helvetica) under the **SIL OFL 1.1**. Both licences permit embedding in a document; `doc/third-party-data.md` records what was read and `/NOTICE` carries the obligations, checked by `viewer-ui/tests/notices.rs` |
| **substitutes for anything else** — a corporate face, CJK, a symbol font | §4.9 for the rest | **not shipped, and not hard**: OFL families exist with wide coverage, and the OFL's whole point is that embedding is permitted. The work is choosing the set, reading its licence and adding a row to `doc/third-party-data.md` — the same three steps Liberation already went through |
| **Adobe predefined `CMap`s** | non-embedded CJK fonts, §6.2.11.3.3 | **already shipped** — all 239, **BSD-3-Clause**, in `data/cmaps/` |
| **the Adobe Glyph List, standard-14 metrics** | naming codes (§2.1), widths (§4.9) | **already in the tree** |
| **an sRGB ICC profile** | §4.1's output intent, which almost every conversion needs | **decided by `A18`: ship it, with a flag to override.** The reading it rests on: `doc/rfc/0006` §5.3 read the ICC's terms as a permissive grant with two conditions — ship it unchanged with its copyright tag, do not use ICC's name to advertise — with no copyleft and no field-of-use clause. A second route exists if that reading does not survive scrutiny: an ICC v2 matrix/TRC profile is a small, fully specified structure, and this tree already *reads* ICC profiles, so generating one from published colorimetry rather than redistributing anybody's file is a bounded piece of work |
| **a CMYK output profile** | any document with `DeviceCMYK` content | **the one you supply** — and §10.1 has a spec-defined fallback for when you cannot, at a cost the standard itself states |

### 10.1 CMYK: the profile you supply, and the fallback if you have none

§6.2.4.3 permits `DeviceCMYK` only where a device-independent `DefaultCMYK` colour space has been
set or the current PDF/A output intent carries a **CMYK** destination profile. An sRGB output
intent does not satisfy it.

**The correct answer is your own profile.** A CMYK output intent is a statement about *which press
the document was made for*, and nobody but the document's owner knows that.

**The licence question was an assumption when this was written and has since been checked.** The
ICC's own registry lists the registered CMYK output profiles — the ECI's `PSOcoated_v3` and
`PSOuncoated_v3_FOGRA52`, Idealliance's CGATS, GRACoL and SWOP set, APTEC's offset and flexo set —
and three things about that listing decide it. The copyright is held by those bodies rather than
by the ICC, so the ICC's own permissive grant does not reach them; what the registry links for
each is *characterization data*, a `.txt` of measurements, rather than a profile; and no
redistribution terms are stated for any of them. So the working assumption was right, and it is
now a finding rather than a caution.

**Where an ICC profile's terms actually live, which is the more useful half.** The ICC's guidance
is that a profile's copyright owner and terms of use are normally identified in the `Creator`
field of its header and in its `cprt` tag. That is a rule a program can apply: a converter handed
`--output-intent-profile` can read the profile's own `cprt` and put it in the report, so a user
who embeds somebody's press profile is told whose it is. `data/icc/sRGB2014.icc`'s reads
"Copyright International Color Consortium, 2015", which is what let its grant be established from
the file rather than from a page about the file. So `--output-intent-profile <file>` is the
interface, and for a print-origin archive it is mandatory rather than optional: the archive
decides its house profile once.

**But "no profile" is not a dead end, and the standard supplies the way out** — and `A48` has
since allowed the construction below, reported per document and recorded in `xmpMM:History`
naming the clause. ISO 19005-2
§6.2.4.3's NOTE 2 says that a **DeviceN-based `DefaultCMYK`** is subject to §6.2.4.4 and is
thereby device independent. So a `/DefaultCMYK` written as a DeviceN over
`[/Cyan /Magenta /Yellow /Black]`, with an ICCBased sRGB alternate space and a tint transform, is
conforming — and the tint transform does not have to be invented, because **ISO 32000-2 §10.4.2.5
states it**:

> 𝑟𝑒𝑑 = 1.0 - 𝑚𝑖𝑛(1.0, 𝑐𝑦𝑎𝑛 + 𝑏𝑙𝑎𝑐𝑘)

and the same for green from magenta and blue from yellow. Three things make this the right
fallback rather than a trick:

1. **It is the standard's own algorithm**, not ours, and §10.4.2.1 says exactly what it is for: a
   less-capable processor that is not following §10.3's ICC route.
2. **It is non-destructive.** The content stream still says `0 0 0 1 k`; a `/DefaultCMYK` only
   tells a reader how to interpret those numbers. The producer's CMYK values survive in the file
   and a later re-conversion with a real profile can use them. Nothing is flattened, and nothing
   lands on ADR 0816's fence.
3. **It changes nothing a reader already saw.** A viewer with no CMYK profile does this, or
   something very like it, today.

**And the cost is stated by the standard in the same breath**, which is why this is a documented
default and not a silent one: §10.4.2.1 says these algorithms "are, however, very simple and as
perceived by a human viewer they produce only crude approximations of the original colours". For a
photograph separated for press, that is a real loss of fidelity — visible, and not recoverable
from the rendered page. For the far commoner case of a black rule or a logo drawn in `k`, it is
imperceptible.

So the converter's behaviour is:

- **Default: use the supplied profile.** If `--output-intent-profile` names a CMYK profile, that is
  the output intent and nothing else is needed.
- **Fallback with no profile: write the DeviceN `/DefaultCMYK`**, report it per document, and
  record it in `xmpMM:History` — naming §10.4.2.5 as the transform, so a later reader knows
  precisely which approximation was applied and can undo the interpretation.
- **Report it before converting, not after**: *this document uses DeviceCMYK on n pages; without a
  CMYK profile its colour will be approximated by ISO 32000-2 §10.4.2.5.* That is a first-page
  answer, and for an archive handling print-origin material it is the sentence that prompts them
  to buy one profile once.

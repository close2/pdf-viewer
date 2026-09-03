# RFC 0006 — PDF/A: validating a document, and converting one

Status: **draft**
Round: 895 — commissioned by the owner on 2026-09-03, "please start a round for an RFC for a
feature to be able to convert to PDF/A (different versions)". Written beside rounds 892–894,
which were building and walking throughout; this round writes this file and one line in
`doc/rfc/README.md`'s index, and nothing else.
Companions: RFC 0001 (the survey, whose gap matrix already ranks this feature and grades it
**hard**), RFC 0002 (the transform suite — accepted 2026-09-01, and the layer any conversion verb
would live in; its §10 serializer and §11's redrawn exclusion are the two pieces this document
leans on hardest).

**The owner's standing framing for this series is in force**: an RFC is not limited by the
project's current rules. Where a rule bears on the proposal it is named as a current restriction
with its rationale (§4), and the unconstrained design is proposed beside it.

**Registers.** What Acrobat, Ghostscript, veraPDF and the rest do with PDF/A is evidence about
*demand and convention*, and — for the requirements themselves — evidence about a *reading*.
`CLAUDE.md` principle 5 governs without exception: ISO 19005 is the only source of truth about
what PDF/A requires, agreement with veraPDF would be evidence that we read it right, and
disagreement would be a question to take back to the standard. §0 says exactly how much of that
standard this document was able to read, because the answer is "less than it needed to" and
saying so is the first requirement principle 5 puts on it.

---

## 0. Provenance — what this RFC read, and what it did not

**ISO 19005 is paywalled, in all four parts, and this round did not have it.** That is not an
incidental limitation; it is the central fact about this proposal, and §10 question 2 is the
consequence.

| source | status here |
|---|---|
| **ISO 32000-2** (with Errata Collection 3) | **read in full**, in `doc/md/`. Every §-numbered clause quoted below is from it, verbatim, and checkable by `tools/conformance`. |
| **ISO 14289-1 (PDF/UA-1)** and **14289-2 (PDF/UA-2)** | **read in full**, in `doc/md/`. ISO 14289-2 states that its §8.4.5 "includes requirements matching those of the corresponding clause in ISO 19005-4 (PDF/A-4)", which makes it the one **normative** text in this tree that states a PDF/A-4 requirement in its own words. §5.1 uses it, and says what the caveat is. |
| **WTPDF 1.0** and the **Tagged PDF Best Practice Guide** | in `doc/md/`; industry documents, not ISO standards. |
| **ISO 19005-1, -2, -3, -4** | **not read.** Not owned, not obtainable without purchase. |
| veraPDF's validation profiles, the PDF Association's technical notes, Ghostscript's and qpdf's documentation, ISO's own abstracts | evidence about the requirements and about how others read them — cited as such below, and never as the requirement. |

**The rule this document holds itself to.** Every statement about what a part of PDF/A requires
is marked with where it came from. A statement carrying a `§` and quotation marks is ISO 32000-2's
or ISO 14289's own words. A statement of a PDF/A requirement with no clause number is
**second-hand**: it is what the secondary sources agree the standard says, and it is written here
so the owner can weigh the proposal, not so that anything can be implemented from it. **Nothing in
§3 or §5 is a specification, and no implementation may cite this document as one.** That is
question 2's whole point.

---

## 1. What this proposes, in one paragraph

Two features, sequenced rather than bundled. **A validator**: a new crate, `pdf-archive`, that
decides whether a document conforms to a stated part and level of ISO 19005 and reports which
requirement it fails and where — a reader, over the parsers, the ICC engine, the font loader and
the structure tree this project already has, with every requirement carrying its clause the way
the conformance ledger's rows do. **A converter**: a verb in RFC 0002's transform suite,
`pdf-transform archive --to <target>`, over the structure-preserving serializer that landed under
ADR 0817, doing the removals and additions that are mechanical and **refusing by name** the ones
that are not. The document argues that the validator is the larger part of the value and nearly
all of the reusable work, that three of the converter's requirements are unsatisfiable or fenced
rather than merely hard, and that the honest sequence is validator, then a measurement, then a
decision about the converter. It also argues that neither can be built to this project's
standards without the normative text of ISO 19005, which nobody here has.

---

## 2. Motivation

**The owner asked for it**, on 2026-09-03: "please start a round for an RFC for a feature to be
able to convert to PDF/A (different versions)." That is the demand this document answers, and the
rest of this section is the evidence around it rather than a case for it.

**RFC 0001 already ranked it, and ranked it last.** Its gap matrix (§7) carries the row

> | PDF/A convert | P | — | ✓ | — | ✓ | gs, ocrmypdf | — | **hard**: a conformance rewriter against ISO 19005 — a standard this tree does not yet read |

— Acrobat *Pro* tier, absent from PDF-XChange and Nitro, present in Foxit and Stirling-PDF, and
supplied on the command line by Ghostscript and ocrmypdf. It is the only row in that matrix whose
"why" names a standard rather than a piece of machinery, and that is the whole shape of the
problem: every other transform is work over clauses this tree reads, and this one is work over
clauses nobody here has read at all.

**The demand is institutional rather than personal.** Nobody converts a document to PDF/A because
they want to read it; they do it because a repository, a court filing system, a records
regulation or a funder's deposit rule requires the file to be PDF/A before it will be accepted.
ISO 32000-2's own introduction states the position in one line, and it is the reason the format
exists at all:

> PDF/A (ISO 19005) is the industry standard for the archiving of digital documents.

The consequence for a design is that the *user* of this feature is usually not the author of the
document. They have a file somebody else produced and a rule that says it must conform. That is
exactly the population a converter serves worst and a **validator** serves best — §6 is about
that split, and it is the recommendation this RFC leads with.

**And there is a second motivation, which is this project's own.** `doc/PLAN.md` §4's corpus plan
already names "veraPDF, Isartor (malformed files)" among the corpora to add, and
`doc/corpora/format-corpus/pdfCabinetOfHorrors/` already carries veraPDF-derived fixtures —
`veraPDFHiResChangedHeight.pdf` is the witness in §7.4.8's ledger row for a defect that cost a
whole photograph. Those corpora exist because somebody wrote a conformance checker and needed
files to break it with. A project with a conformance ledger, a clause-citation discipline and a
generated object-model validator is unusually close to being able to *answer* the question those
corpora ask, and unusually far from being able to answer the producer's question.

## 3. What PDF/A is, part by part

**Everything in this section is second-hand** in the sense §0 defines, except where a clause
number and quotation marks appear. It is written to let the owner weigh the proposal; it is not a
specification and no implementation may be built from it.

### 3.1 The shape common to all four parts

PDF/A is a *subset* standard: it takes a version of PDF and forbids what cannot be relied on to
mean the same thing in fifty years, requires what makes the file self-contained, and requires the
file to say which subset it claims. ISO 32000-2 describes the family's purpose in one sentence —
"PDF/A (ISO 19005) is the industry standard for the archiving of digital documents" — and warns
producers that its own deprecations are read more strictly there: "Implementers are cautioned that
some features that are deprecated in this document could have tighter constraints placed on them,
or even be removed completely, in a later version of ISO 32000, or in subset standards such as
PDF/X (ISO 15930), PDF/A (ISO 19005), PDF/E (ISO 24517), PDF/VT (ISO 16612-2 and ISO 16612-3) and
PDF/UA (ISO 14289)."

The three obligations recur in every part:

- **Self-containment.** Every font program embedded; no reference to external content streams, no
  reference XObjects, no linked-not-embedded anything. The file must draw with nothing but itself.
- **Determinable meaning.** Colour anchored to a profile through §14.11.5's output intent or to a
  device-independent space; no encryption, because a file nobody can decrypt in fifty years is not
  archived; no executable behaviour, because JavaScript's meaning is an engine's rather than a
  file's.
- **Self-description.** An XMP packet stating the part and the conformance level (§5.6), so that a
  reader can tell what the file claims to be without inferring it.

And two structural facts about the family matter for a converter:

- **Each part is defined on a base version of PDF**, and a file cannot conform to a part whose
  base is older than the constructions it uses without those constructions being removed or
  rewritten. That is the whole of §5.2's problem and most of PDF/A-1's difficulty.
- **The levels are cumulative within a part**, and the top level is about *semantics* rather than
  bytes — which is why §5.7 argues it is a validation target and not a conversion target.

### 3.2 The parts

| part | ISO | base PDF | levels | the distinguishing thing |
|---|---|---|---|---|
| PDF/A-1 | 19005-1:2005 | PDF 1.4 | a, b | the strictest: **transparency forbidden**, JPEG 2000 forbidden, layers (optional content) forbidden, embedded files forbidden |
| PDF/A-2 | 19005-2:2011 | PDF 1.7 (ISO 32000-1) | a, b, **u** | transparency and JPEG 2000 permitted; optional content permitted; PDF/A files may be embedded in a PDF/A file; adds level **u**, "all text has a Unicode mapping" |
| PDF/A-3 | 19005-3:2012 | PDF 1.7 | a, b, u | identical to -2 except that **any** file may be embedded, whatever its format — the part that exists for hybrid invoice formats |
| PDF/A-4 | 19005-4:2020 | **PDF 2.0** (ISO 32000-2) | none in the a/b/u sense; **PDF/A-4f** (embedded files) and **PDF/A-4e** (engineering) as variants | drops the a/b/u level scheme; the identification schema gains a revision; the base is the version this tree reads natively |

**Level b** ("basic") is about *visual* reproducibility: the page will draw the same. **Level u**
adds that every character shown can be recovered as Unicode. **Level a** ("accessible") adds
tagged PDF — structure, reading order, language, alternate descriptions — and is where PDF/A meets
PDF/UA.

**The part that matters most for this project is -4**, and the reason is architectural rather than
chronological: it is defined on ISO 32000-2, which is the standard this tree's conformance ledger,
every doc comment and every `§` citation already refer to. A requirement of PDF/A-4 is a
constraint on a construction this tree already has a clause row for. A requirement of PDF/A-1 is a
constraint on PDF 1.4, and every one of them has to be translated across a version boundary before
it can be checked against code written to PDF 2.0's numbering.

### 3.3 The three kinds of requirement, which is the classification a converter lives or dies by

The commission asked for exactly this, and it is the most useful thing in the section:

| kind | what it means | examples |
|---|---|---|
| **mechanical** | a program can satisfy it from the file alone, with no judgement and no new marks | no `/Encrypt`; no JavaScript name tree; no `/Launch` action; no LZW; no external content stream; `/Widths` consistent with the embedded program (or a refusal); XMP present and well-formed; the identification schema written |
| **needs a decision no program can make** | satisfiable, but only by a choice about the *document* that a program has no basis for | which output intent — which rendering condition the document is *for* (§5.3); which of several plausible readings a structure tree should encode (§5.7); what a picture's alternate text says |
| **unsatisfiable from an arbitrary input** | the information required is not in the file and cannot be recovered | a font program that was never embedded (§5.1); a Unicode mapping for a symbolic subset font with no `/ToUnicode` and no usable glyph names; a reading order for a page that was never tagged |

**The distribution across the three is the whole design.** Most *count* of requirements is
mechanical, which is why converters exist and appear to work. But the requirements that decide
whether a real document can be converted at all are concentrated in the second and third columns —
and the third column's first row, a missing font program, is on any reasonable expectation the
single most common reason a real document is not PDF/A. §8's layer 5 is how this project would
replace that expectation with a number.

### 3.4 Prior art, and what it tells us

| tool | what it does | the lesson |
|---|---|---|
| **Ghostscript** `pdfwrite` with `-dPDFA` | re-distils the document and writes a PDF/A file; the user supplies a definition file naming the output intent and an ICC profile | two lessons. **The profile is the user's**, not the tool's (§5.3 question 4). And re-distillation is exactly RFC 0002 §10's rejected option B: RFC 0001 already calls it "the cautionary tale, since re-distilling re-interprets content and loses what it does not understand" |
| **ocrmypdf** | writes PDF/A-2b by default, via Ghostscript, but **grafts** the OCR text layer back into the original pages rather than re-rendering them | RFC 0001 records its design principle as "content preservation as a design principle" — and it is the shape a conversion should have: touch what the requirement names, carry everything else |
| **Acrobat Pro** preflight | validates and converts, with a fixup report | the market segmentation is the evidence: RFC 0001 §7 records PDF/A as a **Pro**-tier feature, alongside OCR, redaction and compare — professional rather than everyday |
| **veraPDF** | the reference open-source validator, whose rule sets encode ISO 19005's clauses machine-readably | the architecture is a validation *model* plus rules as data — the same recognition `crates/pdf-spec` embodies for the Arlington object model. §7.1 says why this project must not *generate from* those rules even so |
| **qpdf** | does not claim PDF/A output at all | worth stating: the most careful structure-preserving writer in the prior art declines this feature, which is evidence about its cost rather than about its value |

## 4. Current restrictions, each with its rationale

Per `doc/rfc/README.md`'s convention, named here as restrictions rather than obeyed, with the
unconstrained design proposed afterwards.

1. **The authoring exclusion, as redrawn on 2026-09-03.** The owner ratified RFC 0002 §11.1 with
   "RFC 002 and 003 are approved", and ADR 0816 wrote the amendment into `CLAUDE.md` on the
   `round-867` branch. Authoring *content* from nothing stays excluded; assembling documents from
   existing documents is in scope; and the entry now carries an enforceable test, added by that
   round rather than by the RFC: **does the operation invent marks?** Rotate does not, a watermark
   does. Everything §5 below argues is measured against that sentence.
2. **`writer-side` in the ledger means something new, and it decays.** ADR 0816 replaced the
   status's definition — "addresses a PDF generator; this program writes only §7.5.6's updates"
   became "addresses a generator; this program's writers emit structure, never content" — so a
   round that adds an emitter owes the vocabulary a re-read. A PDF/A converter is such a round
   several times over.
3. **`pdf_syntax::Document` stays immutable**, and the serializer takes sources plus a
   replacement set rather than mutating one. Rationale, unchanged and load-bearing: `interpret`
   is a pure function of the bytes, which is what the raster oracle's whole comparison rests on.
   Nothing proposed here needs it broken.
4. **No clock in the core crates, and byte determinism with no flag.** ADR 0121 and RFC 0002 §9:
   dates are written only when the caller passes `--date`, and §14.4's second identifier is a
   digest of the output. A PDF/A file must carry a creation date and a modification date in two
   places that agree (§5.6), so this restriction and PDF/A meet head-on: the honest resolution is
   that the *caller* supplies the date, not that the library reads a clock.
5. **Nothing eager at startup**, `CLAUDE.md` principle 2: "No parsed data at startup. The
   Arlington-generated tables are compiled-in `static` data, so the object model costs zero parse
   time at launch. Any future data resource follows the same rule." An ICC profile shipped for
   §5.3's output intent is exactly such a future data resource, and the rule already answers how
   it must be carried: as bytes in read-only data, parsed on first use, never at launch.
6. **A C dependency touching untrusted bytes is confined** (principle 3), and `doc/stack.md`
   argues every dependency. The ICC engine in `crates/pdf-model/src/icc.rs` was written rather
   than taken from `lcms` for exactly that reason (ADR 0009); nothing below proposes reversing it.
7. **A document's restrictions are the reader's to set**, with four levels (principle 3,
   `doc/todo/38`). Already implemented once for the transform suite in
   `pdf_model::restriction` (ADR 0803). A converter inherits it unchanged: §5.4's decryption is
   an extraction-shaped operation and asks the same policy the same way.

## 5. The hard cases, each argued

Seven of them. Each says what the requirement is and where that statement came from, what a
conversion would have to do, what this tree already has, and what the honest verb is.

The classification of §3.3 is the thread running through all seven: the mechanical requirements
are not in this section, because they are not hard. What is here is the second and third columns.

### 5.1 Fonts — the requirement a conversion cannot satisfy, and what the verb should therefore do

**The requirement.** Every part of PDF/A requires that the program for every font used to render
content be embedded in the file. This RFC could not read ISO 19005's own wording of it, but it
can read a normative wording of the *same* requirement: ISO 14289-2 states that its §8.4.5
"includes requirements matching those of the corresponding clause in ISO 19005-4 (PDF/A-4) and
ISO 15930-9 (PDF/X-6)", and ISO 14289-2 §8.4.5.5.1 then says

> The font programs for all fonts used for rendering within a conforming file, as determined by
> whether at least one of its glyphs is referenced from one or more content streams, shall be
> embedded within that file, as defined in ISO 32000-2:2020, 9.9.

with the exemption for text rendering mode 3 stated in its NOTE 2, the licence condition —
"Only font programs that are legally embeddable in a file for unlimited, universal rendering
shall be used" — and the completeness condition, "Embedded fonts shall define all glyphs
referenced for rendering within the conforming file." The neighbouring subclauses add the
constraints a converter would also have to meet, all of them ISO 14289-2's numbering:
§8.4.5.3.2's `CIDToGIDMap` on every embedded Type 2 CIDFont, §8.4.5.4's CMap embedding,
§8.4.5.6's glyph widths agreeing between the font dictionary and the font program to within
a thousandth of a unit of text space, and §8.4.5.7's `cmap` subtable requirement on
non-symbolic TrueType.
(Provenance: this is ISO 14289-2's text, which that standard says matches ISO 19005-4's. It is
a normative text, but it is **not** ISO 19005's text, and the earlier parts' wording may
differ.)

**Why a conversion cannot satisfy it.** A document that references Frutiger and embeds no program
for it does not contain the glyph outlines. There are exactly four things a converter can do, and
none of them is conversion:

1. **Find the font on this machine and embed it.** This tree can do this today — `substitute.rs`
   already locates a face by family and by coverage, and `is_substituted()` already reports per
   loaded font whether a program was missing. But the face on this machine is *not* the face
   the producer named. Embedding it produces a file whose glyphs differ from what the source
   drew, which is inventing marks, and it silently converts a document that honestly says "I
   need Frutiger" into one that dishonestly says "these outlines are what I meant". It is also
   the case ISO 14289-2 §8.4.5.5.1's NOTE 3 exists for: most desktop fonts are not licensed for
   unlimited universal embedding, and this program has no way to know which are.
2. **Embed one of the standard 14.** `data/standard-fonts` carries their metrics and this tree
   substitutes with them. Same objection, one degree worse.
3. **Rasterise the page.** This is the re-distillation answer, rejected for the whole suite in
   RFC 0002 §10 option B, and it forfeits text, tagging, forms and searchability — the very things
   an archive format exists to preserve.
4. **Refuse, by name.** Report *which* fonts have no program and *which pages* use them, and exit
   4 — the suite's "refused by name" code, which exists precisely so that a caller can tell "the
   file defeated us" from "we declined".

**The proposal is 4, and it is not a limitation to apologise for.** A file that cannot be made
conformant should be *said* to be non-conformable, with the reason and the witness pages, because
that is a true and actionable answer and the other three are false ones. It is also the answer
that makes the validator the primary product: the validator's report is exactly what a user needs
in order to go back to the producer and ask for a file with its fonts in it.

**The one exception worth carving out**, and it needs no substitution: a font whose glyphs are
referenced *only* in text rendering mode 3 is exempt by ISO 14289-2 §8.4.5.5.1's NOTE 2, and
this tree's interpreter already knows the rendering mode of every show operation. Detecting
that case is a validator feature that will spare real documents a false failure, and it costs
nothing.

### 5.2 Transparency — the case that meets the redrawn exclusion head-on

**The requirement.** PDF/A-1 is defined on PDF 1.4 but does not admit all of it: transparency is
forbidden. PDF/A-2 onwards, defined on PDF 1.7 and PDF 2.0, permits transparency with conditions
on the blending colour space. (Provenance: second-hand — see §0. The prohibition itself is not in
doubt; its exact extent, and in particular whether a constant alpha below 1 with no group and no
soft mask is caught by it, is a question this RFC cannot answer from a normative text and §10
question 6 leaves open.)

**What a converter would have to do.** Flatten: composite the transparent constructions down to
opaque marks, which in the general case means rasterising the affected regions and replacing
vector content with an image. That operation is not a rewrite of the producer's marks — it is the
computation of *new* marks that approximate them.

**And that is `CLAUDE.md`'s fence, applied to the first case since it was drawn.** ADR 0816 put
the test in the file three weeks' worth of sessions before this document: *does the operation
invent marks?* Rotate does not; a watermark stamp does, "which is why qpdf's
`--overlay`/`--underlay` and Stirling's watermarking are **deliberately not in this suite**". A
flattener is on the same side of that line as the watermark and further from it: a watermark adds
marks beside the producer's, and flattening *replaces* the producer's with a rendering of them.
Every argument the exclusion rests on applies: the result is this program's dialect of the input,
every flattener bug becomes silent visual corruption, and RFC 0002 §9's raster oracle — which
demands bit-identical rasters for a lossless transform — has nothing left to assert.

**Named as a restriction, per the RFC conventions; and here is what the unconstrained design
would be.** If the owner amends the fence, flattening is *buildable* here and the machinery is
unusually close to hand: `render-cpu` is the correctness oracle and already composites every
construction clause 11 defines, so "rasterise this region as the oracle would draw it and emit it
as an image XObject" is a small program over a large existing capability. What it would cost is
stated rather than hidden — text inside a flattened region stops being text (so a level-a or
level-u conversion of that region becomes impossible, and the archive loses the searchability it
was created for), vector art becomes resolution-bound at whatever DPI the flattener chose, and
the choice of that DPI is a quality knob with no specification behind it.

**The recommendation is therefore narrower than a refusal**, and it is the recommendation because
it costs the owner nothing to accept:

- **A converter targeting PDF/A-2, -3 or -4 does not need a flattener at all**, because those
  parts permit transparency. The blending-colour-space conditions they impose are *checks*, not
  transformations, and this tree already reads the blending space of every group
  (`content/transparency.rs`) and already resolves an output intent's profile into one.
- **A converter targeting PDF/A-1 refuses a document that uses transparency, by name and with the
  pages listed** — and tells the caller that PDF/A-2b is the format that accepts their file.
  Given that PDF/A-2 has existed since 2011 and PDF/A-4 since 2020, a user asking for PDF/A-1
  specifically is almost always doing so because a deposit rule names it, and telling them
  exactly which pages block it is more useful than silently rasterising their document.

So the fence does not have to move for this feature to ship. It has to move only for **PDF/A-1
conversion of a transparent document**, which is the narrowest possible statement of the
question, and §10 question 5 puts it to the owner in that form.

### 5.3 Colour, the output intent, and the profile a converter has to get from somewhere

**The requirement.** A PDF/A file that paints in device colour spaces must anchor them: either
the file provides an output intent whose `/DestOutputProfile` is an embedded ICC profile, or
every device space used is given meaning some other way. (Provenance: second-hand — §0 — for the
exact form of the condition per part; the mechanism it names is ISO 32000-2 §14.11.5, which this
RFC read in full.)

**What the tree already has is more than a converter needs and exactly what a validator needs.**
`crates/pdf-model/src/icc.rs` is this project's own ICC engine — written rather
than taken from `lcms`, because `#![forbid(unsafe_code)]` and the C-dependency rule both argue
against a C library parsing bytes off a page (ADR 0009). It implements `A2B0`/`A2B1` in `mft1`,
`mft2` and `mAB `, the matrix/curve form, and since ADR 0796 the `B2A0`/`B2A1` direction as well.
`output_intent_space` in `content/colour.rs` reads the catalog's `/OutputIntents`, takes the
first `/DestOutputProfile` it can parse, and — on `round-867`, ADR 0821 — the page-level array
that §14.11.5 also defines. So *checking* that an output intent exists, that its profile parses,
that its component count matches the device spaces the pages actually use, and that the profile
is one an ICC engine can evaluate, is machinery already in the tree.

**Where does the profile come from?** This is the converter's problem and it has no good answer.
`data/` holds `cmaps` and `standard-fonts` and no ICC profile; nothing in `crates/` embeds one.
Three options:

1. **Ship one.** `CLAUDE.md` principle 2 already states the terms — "Any future data resource
   follows the same rule": compiled-in `static` bytes, parsed on first use, zero startup cost.
   Technically this is trivial and the rule is already written. What it is not is free: it is a
   redistributed third-party data file with a licence to check and to keep checked, in a tree
   whose dependency discipline argues every crate in `doc/stack.md`. It is also a *choice about
   the document* being made by the program: which rendering condition a file should declare
   depends on where the file is going, and Ghostscript's `-dPDFA` interface makes the user supply
   it for exactly that reason.
2. **Require the caller to supply one** (`--output-intent-profile <file>`), with no default. The
   library never opens a path — the seam forbids it — so the profile arrives as bytes through
   `Sources`, like any other input.
3. **Refuse to convert a document that has no output intent and paints in device colour.** Too
   strict to be useful: it would refuse most documents that exist.

**Recommendation: option 2 in tranche one**, with option 1 available later if the owner wants a
default. Question 4 puts it.

**And there is a sharper problem underneath, which this round found by reading the code rather
than the standard.** §14.11.5 says of the output intent dictionary:

> The data in an output intent dictionary shall be for informational purposes only, and PDF
> processors are free to disregard it.

This program does not disregard it. `Interpreter::device_space` resolves `DeviceGray`,
`DeviceRGB` and `DeviceCMYK` by asking §8.6.5.6's `/Default…` spaces first, and then — where
nothing replaced the device space and the profile's component count matches — **returns the
output intent's profile as the meaning of that device space**. That ranking is the standard's
own: §8.6.5.7 NOTE 3 sends a reader to §14.11.5 for the intended meaning of a device space, and
§10.4.2.1 ranks ICC above §10.4.2.5's formulae. The ledger's §14.11.5 row records it as
`implemented` on exactly that reasoning.

The consequence for a converter is direct and unwelcome: **adding an output intent to a document
that had none changes what every device colour on every page means, in this very renderer.** No
mark is invented — nothing new is drawn — but existing marks change colour. Two things follow:

- **RFC 0002 §9 layer 3's bit-identical raster gate cannot hold for this verb**, and that is not
  a tolerance to be tuned but a fact to be stated: the operation's *purpose* is to pin down a
  colour meaning the source left open. The honest gate is a comparison of the output against the
  source **rendered with the same profile forced in**, which this tree can do because
  `output_intent_space` is one function; equality there is a real assertion, and it says the
  converter changed nothing except what it declared it was changing.
- **It is a case the fence's wording does not decide.** ADR 0816's test is "does the operation
  invent marks?", and this operation reinterprets them instead. The answer is probably that it is
  in scope — a declaration about colour is metadata, and §14.11.5 itself calls it informational —
  but the fence was written days ago against a watermark, and this is the first case that is
  neither clearly inside nor clearly outside it. It is folded into question 4.

### 5.4 Encryption — forbidden by every part, and the easiest requirement in this document

**The requirement.** Every part of PDF/A forbids encryption: a conforming file carries no
`/Encrypt` and its strings and streams are not encrypted. (Provenance: second-hand — §0.)

**The validator's side is one predicate**: the trailer has no `/Encrypt`. This tree parses the
trailer before it parses anything else, so the check is free and is the first one to run.

**The converter's side is nearly free too, and it is the one requirement where this tree is
already ahead.** `crates/pdf-syntax/src/crypt.rs` implements the standard security handler "at
revisions 2, 3, 4, 5 and 6 — every revision Table 21 lists — over `/V` 1, 2, 4 and 5, with the
`V2`, `AESV2` and `AESV3` crypt filter methods of Table 25 and the `Identity` filter of Table 26";
and the serializer that landed under ADR 0817 states in its own module doc that **the output is
not encrypted** and emits no `/Encrypt`. So "read an encrypted document, write an unencrypted
derivative" is the existing composition of two existing capabilities, and no new clause work is
owed. Public-key handlers (§7.6.5) are refused by name, which is a correct refusal rather than a
gap: a converter meets one and says so.

**The one thing that is *not* a technical question, and it is `CLAUDE.md` principle 3's.**
Stripping encryption from a document is exactly the operation a document's `/P` bits may assert
against, and the project's doctrine is that a restriction "shall always be possible to turn off"
because "this program is the reader's" — with four levels, off / on / ask / warn, asked once in a
place a host can supply. `pdf_model::restriction` already implements all four for the transform
suite (ADR 0803), with the transform's operations named beside the viewer's. A PDF/A conversion
is an extraction-shaped operation in that vocabulary and asks the same question the same way. The
default RFC 0002 §13 question 4 proposed for a non-interactive tool — `off`, because a pipe
cannot "ask" — carries over unchanged, and nothing new is owed.

### 5.5 JavaScript, actions and embedded content — the removals, and one that is not a removal

**The requirements** (provenance: second-hand — §0): no JavaScript actions and no `/JavaScript`
name tree; no `/Launch`, and restrictions on which action types may appear at all; no embedded
files except where the part permits them (PDF/A-3 permits any, PDF/A-2 and -1 do not, PDF/A-4f is
the variant that does); no external content streams and no reference XObjects; no LZW-compressed
streams; and constraints on optional content and on the annotation types that may appear.

**These are the mechanical half of a conversion, and they are genuinely mechanical.** Removing an
action, a name-tree entry or an embedded file changes no mark on any page. `pdf-transform
attachments --remove` already does exactly one member of this family — it takes the entry out of
the tree and marks the objects free by §7.5.4's second mechanism (ADR 0803) — so the shape is
proven; the rest is the same walk over other name trees. Re-encoding an LZW stream as Flate is a
decode and an encode this tree does on both sides today, and it changes no sample. RFC 0001's
matrix already grades this family as its own feature: "sanitize (strip JS/attachments/metadata)"
at **easy-moderate**, with the note that "removal is easier than any edit". A PDF/A converter's
mechanical half *is* that feature, and it is worth noticing that the sanitiser is independently
wanted (Okular bug 452403 is cited there as viewer-side demand for it).

**Two entries in this family are not removals, and they are where care is owed.**

- **Annotations.** The parts require that every annotation whose flags call for a visible
  appearance actually carry an appearance stream, and constrain which subtypes may appear. This
  tree *constructs* appearances the standard states but a file omits — §12.5.6.4's seven icons,
  §12.5.6.15's four, §12.5.6.16's two, and the free-text layout in `variable_text.rs`. Writing a
  constructed appearance into the output is the closest thing in this document to inventing
  marks that is nonetheless defensible, because §12.7.4.3 and §12.5.5 require the appearance and
  the artwork of the icons is this processor's own by the standard's own silence
  (`CLAUDE.md`'s standing example). It is still a decision, and §10 question 7 asks for it.
- **Optional content.** Restrictions on optional content are not satisfiable by deletion —
  deleting a configuration changes which marks appear. This is a check, and where a document
  fails it the honest verb is a refusal.

### 5.6 XMP, the identification schema, and metadata this program would be authoring

**The requirement.** A PDF/A file identifies itself in its document-level XMP packet through the
PDF/A identification schema: `pdfaid:part` (1, 2, 3 or 4), `pdfaid:conformance` (`A`, `B` or `U`
for parts 1 to 3), and for part 4 a `pdfaid:rev`. The parts also require that where the document
information dictionary and the XMP packet both state a property, they agree, and that any XMP
property outside the predefined schemas be described by an embedded extension schema.
(Provenance: second-hand — §0 — except where ISO 32000-2 states the same thing, below.)

**ISO 32000-2 states the agreement rule itself**, in §14.3.4, and it binds *this program as a
writer* independently of PDF/A:

> When writing the time and date of creation for the first time, typically when a new document is
> created, a PDF processor shall ensure that the data in the document information dictionary and
> the document level metadata stream -if both are written -are fully equivalent.

and, for the modification date, the same `shall`. §14.3.2 fixes the packet's form — Table 347's
`/Type /Metadata` and `/Subtype /XML`, and "[t]he contents of a metadata stream shall be the
metadata represented in Extensible Markup Language (XML) and the grammar of the XML representing
the metadata shall be defined according to the extensible metadata platform specification (ISO
16684-1)". Both ledger rows are `implemented` on the reading side.

**What the tree has and has not.** `crates/pdf-model/src/xmp.rs` is a reader over `xmlparser` that
resolves `xmlns` bindings, and its own doc comment already names this schema —
"`<pdfaid:part>` and `<pdfaId:part>` are the same property". So *reading* a PDF/A claim is done
today. There is **no XML writer anywhere in the tree**, and that is the single concrete missing
component a converter needs most and a validator needs not at all.

**And here is the argument that matters, which is not about the missing writer.** Every other
requirement in §5 is about the document's content. This one is about the document's *claim about
itself*. Writing `pdfaid:part 2` and `pdfaid:conformance B` into a file is this program asserting,
in the file, to every future reader, that the file conforms — and no reader of that assertion can
tell whether the program checked or merely stamped. That is a qualitatively different act from
deleting a JavaScript action, and it is the reason §6's ordering is not a matter of taste: **a
converter that writes the identification schema without a validator to justify it is a program
that lies for a living.** The design consequence is a rule, proposed here and worth writing into
the code the day it exists:

> The identification schema is written by the converter **only** after the output has been
> re-read and passed by this tree's own validator, in the same operation. A conversion whose
> output does not validate emits no `pdfaid` properties and exits 4.

The date is the other half of this subsection and is the collision with restriction 4. §14.3.4's
`shall` is on a *writer*, PDF/A needs the dates present, and `CLAUDE.md` allows no clock in the
core crates because determinism is testability. The resolution is the one the suite already uses
for `attachments --attach`: the caller passes `--date`, and the library writes what it is given
into both places, equivalently, or writes neither. A converter therefore **requires** `--date`
rather than defaulting it, which is a small, honest interface consequence of a rule worth keeping.

### 5.7 Level A — a structure tree, and the difference between checking one and inventing one

**The requirement.** Level a (parts 1, 2 and 3) adds to level b everything level u adds — Unicode
mapping for all text — plus tagged PDF: a logical structure tree, a `/MarkInfo` dictionary with
`/Marked true`, declared natural language, alternate descriptions for non-text content, and a
reading order that reflects the document. It is the level that makes an archived document
*accessible*, and it is why PDF/A level a and PDF/UA (ISO 14289) are usually discussed together —
ISO 14289-1's own introduction says PDF/UA "is intended as a companion standard, to be used in
conjunction with ISO 32000, ISO 19005, ISO 15930 and other standards".

**The validator's side is the best-supplied thing in this whole RFC.**
`crates/pdf-model/src/structure.rs` reads §14.7's tree in both directions,
§14.7.3's role mapping in both of the standard's systems, §14.7.6's attributes, §14.7.4's
namespaces, §14.8.4's whole vocabulary, §14.8.2's artifacts, table and list semantics, and
`Tree::logical_order`, whose reading order is already held against `pdftotext`'s word boxes by a
gate. `MarkInfo::read` answers §14.8.1's `/Marked`. `viewer-accessibility` hands all of it to
AccessKit and so to AT-SPI, and `crates/viewer-core/tests/accessibility_census.rs` *already
reports* two of the exact mismatches a level-a check asks about, by name: a `/StructTreeRoot`
without §14.8.1's `/MarkInfo /Marked`, and `/Marked` without a `/StructTreeRoot`. `doc/md/`
carries ISO 14289-1 and -2, the Tagged PDF Best Practice Guide and WTPDF 1.0 as converted
Markdown, so the *normative* neighbourhood of this requirement is readable here in a way ISO
19005's own text is not.

**The converter's side is the one place where "no program can decide this" is not a figure of
speech.** Auto-tagging an untagged document means deciding that this run of glyphs is a heading
and that one a table cell, that this image is decorative and that one needs an alternate
description, and what the reading order of a two-column page with a pull quote is. Those are
editorial judgements about a document's *meaning*, and a wrong one is worse than no tag: a
screen-reader user given a confidently wrong reading order has been actively misled. Producing
alternate text for an image is not even editorial — it is authoring content that is not in the
file at all, which is the half of `CLAUDE.md`'s exclusion the amendment explicitly *kept*.

**And the corpus says how large this is.** The counts are `tools/state.sh`'s and this document
does not repeat them (ADR 0281); what matters is their shape, which the accessibility gate and
`crates/pdf-model/tests/` both print — the great majority of corpus documents carry no structure
tree at all, and the great majority of their pages are untagged. A converter offering "convert to
PDF/A-2a" would therefore be offering to invent the semantics of very nearly every document it
was handed.

**Proposal.** Levels **b** and **u** are conversion targets. Level **a** is a *validation* target
only, and a converter asked for it says so and names what is missing — which is exactly the
report a person needs in order to tag the document properly in a tool built for it. The one
narrow exception worth taking is the already-tagged document: where the source carries a
structure tree, the converter's job is to **carry it across intact**, which is not invention. It
is also work `pdf-transform` owes for its own reasons — RFC 0002 §6.1 already names "the
structure-tree fragments that reach the kept pages" as part of split's closure walk — so the
level-a-preserving path and the split/merge correctness path are the same work.

## 6. Validation and conversion are two features, and only one of them fits this project

The commission asked for a converter. This section argues that the converter's *prerequisite* is
the more valuable half, that it is the half this project is unusually placed to build, and that
the two should be sequenced rather than bundled.

**They ask opposite questions.** A validator asks *does this file meet ISO 19005 part N level L,
and if not, which requirement does it fail and where?* It is a reader: it opens a document, walks
its objects, and decides. It invents nothing, writes nothing, and every one of its answers is a
citation. A converter asks *produce a file that meets ISO 19005 part N level L and that shows
what this one showed.* It is a producer, and its second clause — showing what the source showed —
is the one no standard states and no gate can fully check.

**The asymmetry in cost is not marginal.** Of the requirement families §3 and §5 enumerate:

| family | validator | converter |
|---|---|---|
| encryption absent | one predicate on the trailer | decrypt and re-emit — already possible |
| no JavaScript, no forbidden actions | a walk over the name tree and every `/A` | delete them — mechanical |
| no LZW, no external streams, no `/Ref` | a walk over every stream dictionary | re-encode with Flate — mechanical |
| every font embedded | `is_substituted()`, per loaded font | **unsatisfiable in general** (§5.1) |
| an output intent with a profile | read `/OutputIntents`, parse the profile | **needs a profile from somewhere** (§5.3) |
| no transparency (part 1) | detect groups, soft masks, non-`Normal` blends, `CA`/`ca` | **flattening invents marks** (§5.2) |
| XMP with the identification schema | read the packet, check two properties | **authoring metadata, and a claim** (§5.6) |
| tagged, level a | walk the structure tree the tree already reads | **auto-tagging is authoring** (§5.7) |

Every row's left column is a walk over structures this tree already parses. The right column's
first three rows are mechanical; its last five are either impossible, or need a decision no
program can take, or sit on the far side of `CLAUDE.md`'s redrawn fence. **That is the whole argument, and it is an argument about
this project rather than about PDF/A**: a reader whose defining discipline is citing the clause
it implements is a validator with the hard 90 % already built, and a producer with almost none of
it.

**And the project has the instrument nobody else's validator has.** veraPDF decides conformance
from an object model. This tree decides conformance from an object model *and can render the
page*, on three independent backends held against each other. A requirement whose purpose is
"the file will still look right in twenty years" — every font embedded, every colour anchored to
a profile, no reliance on anything outside the file — can be checked here not only structurally
but by *drawing the page and seeing what the missing thing cost*. A report that says "this
document's headings use Frutiger, which is not embedded; here is page 3 as it draws on this
machine and as it draws with nothing installed" is a different and better artefact than a rule
identifier. Nothing in this section requires that to ship first; it is the argument for why the
validator is *this project's* feature rather than a duplicate of somebody else's.

**The recommendation, stated plainly: build the validator first, and do not commit to the
converter at all until the validator's corpus run says what conversion would actually have to
do.** §8's layer 5 is that measurement, and it costs one round once the validator exists. It is
also, on the evidence of §5, likely to show that the largest single class of non-conforming real
documents is the one class a converter cannot fix — a missing font program — which would settle
question 1 by measurement rather than by argument.

## 7. Where it lives

### 7.1 The validator: a new crate beside `pdf-spec`, not inside it

`crates/pdf-spec` is already "PDF object model validation, generated from the Arlington PDF
Model" — `build.rs` turns the model's key rows of tab-separated data into `static` Rust tables, so
that, in its own words, "[h]and-writing conformance checks for the whole object model would mean
thousands of conditionals no reviewer could audit against the specification; generated tables
keep conformance reviewable as *data*" (ADR 0003). That is precisely the shape a PDF/A validator
wants, and the resemblance to veraPDF's rule sets is not a coincidence: both are the same
recognition, that a conformance standard is a few hundred predicates over an object model and
that predicates are better as data than as code.

**But it must not be generated from veraPDF's rule sets**, and principle 5 is why. A generated
table is a *claim about a specification*; generating one from another implementation's encoding
of that specification would make this project's conformance verdict a restatement of veraPDF's,
with the citations pointing at clauses nobody here had read. That is curve-fitting with a build
script. The rule sets stay what §0's provenance table says they are — evidence about a reading —
and the tree's own table is written from the normative text, clause by clause, exactly the way
the conformance ledger's rows were.

Proposed: **`crates/pdf-archive`**, one stated responsibility — *deciding whether a document
conforms to a part and level of ISO 19005, and saying which requirement it fails and where.* It
depends on `pdf-syntax`, `pdf-model` and `pdf-spec`, and on nothing that writes. Its output is a
typed report: per requirement, a verdict, the clause, and the object or page that witnesses a
failure. `pdf-retrieve` gains a verb, or the crate gains a thin binary of its own; §10 asks the
owner which.

**The requirement set is data, and the shape of that data is the design decision** — what
veraPDF calls a validation *profile*, a word this document avoids because §5.3 needs it for
ICC. A requirement is (part, level, clause, predicate over the object model, witness
extractor). Some are pure `pdf-spec` questions — a key that shall be absent, a value that shall
be one of a set. Some need `pdf-model` — is every font's program embedded, does every
annotation with a normal appearance have one. A few need the interpreter — does any content
stream use a blend mode other than `Normal`. The tiering matters because it says how much of
the validator is cheap: the first tier is a walk over the object graph this tree already
parses, and it is most of the rows.

### 7.2 The converter: a verb in RFC 0002's suite, on RFC 0002's serializer

If a converter is built, it belongs in `pdf-transform` as a subcommand and not in a crate of its
own:

```
pdf-transform archive in.pdf --to pdf-a-2b -o out.pdf
```

Every piece of machinery it needs is the suite's. It reads through `pdf-syntax`/`pdf-model` with
the same budgets and the same confined codec worker (RFC 0002 §8). It emits through
`crates/pdf-syntax/src/serialize.rs`, the structure-preserving serializer that landed on
`round-867` under ADR 0817 — the one that "emits structure and never content". It plans, refuses
and reports through the seam's `Plan` / `Sinks` / `Policy` / `Refusal` / `Report` types, whose
rule 2 — "**No filesystem, no clock, no environment.** [`apply`]'s output is a function of
(sources, plan, policy, budget) and nothing else" — is what makes a converter's output
reproducible enough to be a gate.

**What the seam does not have and would need**, each small and each real:

| gap | what is needed | where it lands |
|---|---|---|
| an XMP **writer** | there is a reader (`crates/pdf-model/src/xmp.rs`, `xmlparser`) and no XML writer anywhere in the tree | `pdf-model`, beside the reader; RDF/XML is a fixed shape, not a general serialiser |
| a date the caller supplies | `--date` exists for `attachments --attach`; §5.6's two-places-agree rule makes it mandatory rather than optional here | `pdf-transform`'s plan |
| an ICC profile to embed | nothing in `data/` is one | §5.3, and it is question 4 for the owner |
| encryption **off** on the way out | the serializer already emits no `/Encrypt`; what is missing is decrypting the *sources* on the way in, which `pdf-syntax` does | nothing new — §5.4 |
| a font **embedder** | `is_substituted()` says a program is missing; nothing writes one | §5.1, and it is the case with no honest answer |
| transparency **flattening** | nothing, anywhere | §5.2, and it is on the far side of the line |

### 7.3 The order these two want to be built in

The validator needs no writer, no serializer, no XMP emitter, no ICC profile and no policy
decision beyond which part to check against. It can be built today, on `main`, against clauses
read from the normative text. The converter needs all six rows of that table, and — for
PDF/A-1, or for level a — the two amendments §5.2 and §5.7 argue *against* making. **They are
not two halves of one feature; they are a feature and its prerequisite**, because a converter
with no validator cannot state whether it succeeded and would be reduced to claiming
conformance rather than demonstrating it — which is the failure mode of every re-distilling
converter §3.4's prior art describes.

## 8. How it would be gated

RFC 0002 §9's four layers apply unchanged to a converter's output, and the validator adds a fifth
question of its own that none of them asks. Strongest first:

1. **Byte determinism.** Same source, same plan, same version, same bytes — no flag. A converter
   has one new source of non-determinism the other verbs do not: the date it must write (§5.6).
   The seam's answer is already the right one — the caller passes it — and the gate is what makes
   that a rule rather than an intention.
2. **Self read-back**, and here it is stronger than for any other verb: the output is re-opened
   through this tree's reader and **run through this tree's own validator**. A converter whose
   output its own validator rejects is a defect with no interpretation needed. This is the loop
   that makes the pair worth more than either half.
3. **The raster oracle, and it is the load-bearing one and the one PDF/A strains.** Render page
   *k* of the output and of the source with `render-cpu` at the same scale and require
   bit-identical rasters. For a conversion that only *removes* what a page never drew with —
   encryption, JavaScript, an embedded file, a `/Widths` inconsistency — bit-identical is
   achievable and is the right bar. For a conversion that flattens transparency or embeds a
   substituted font it is unachievable **by construction**, because those operations change the
   marks; the tolerance would have to be a stated `raster-compare` number, and choosing that
   number is choosing how much of the producer's page the program is willing to lose. §5.2 and
   §5.1 argue that this is a reason not to do those two at all rather than a reason to pick a
   number.
4. **Foreign checkers as evidence, in principle 5's register and no other.** veraPDF over the
   output, and the veraPDF and Isartor corpora through this tree's validator, are exactly what
   poppler and mupdf are for rendering: **agreement raises confidence that we read ISO 19005
   correctly, disagreement is a question to take back to the standard.** Concretely, the
   disagreement is where the value is — a file veraPDF passes and this tree fails, or the
   reverse, is either a misreading here, a misreading there, or a genuinely unspecified point,
   and those three have different consequences. `doc/PLAN.md` §4 already lists both corpora as
   ones to add; this RFC is the first thing in the tree that would give them a denominator.
5. **The corpus, for the question a standard cannot answer.** `CLAUDE.md`'s two denominators
   again: the ledger and the ISO 19005 requirement list answer *coverage*; running the validator
   over every corpus document and asking what share of real files already conform, and by how
   far they miss, answers nothing about coverage but tells the project — and this RFC's §10
   question 1 — whether a converter would have any work to do that a validator's report could not
   already direct a human to do by hand.

## 9. Difficulty, per part and per level

In the style of RFC 0003 §8: the grade is against *this* tree's architecture, unconstrained by
current rules, with the one-sentence why. **V** is the validator, **C** the converter.

| target | V | C | why |
|---|---|---|---|
| **PDF/A-1b** (19005-1, level B) | **moderate** | **hard** | the validator is a walk plus a transparency detector this tree can already build from `content/transparency.rs`; the converter needs a flattener, which invents marks (§5.2), and a PDF 1.4 downgrade of every 1.5+ construction the source uses |
| **PDF/A-1a** | **moderate** | **declined** | level a's checks are `structure.rs` reading what is there; the converter would have to invent a structure tree for nine documents in ten (§5.7) |
| **PDF/A-2b** (19005-2) | **moderate** | **moderate** | the honest sweet spot: transparency is permitted, so the conversion is removals, an output intent and an XMP packet — every one of which is a walk this tree can do, plus the two components §7.2 lists as missing |
| **PDF/A-2u** | **moderate** | **moderate-hard** | adds "every character maps to Unicode"; the *check* is `pdf-font`'s existing `/ToUnicode` and encoding machinery, and the *fix* — synthesising a `/ToUnicode` for a symbolic subset font — is possible only where the glyph names or the encoding permit it, and must refuse otherwise |
| **PDF/A-2a** | **moderate** | **declined** | as -1a |
| **PDF/A-3b/u** (19005-3) | **easy** given -2 | **easy** given -2 | the part differs from -2 essentially in permitting arbitrary embedded files, which for a validator is a relaxed predicate and for a converter is one fewer removal; `attachments` already reads and writes §7.11.4 |
| **PDF/A-4** (19005-4) | **moderate** | **moderate** | on PDF 2.0, which is the version this tree reads natively and cites throughout — the least translation of any part; the tagging requirement is not a separate level here, which removes the -1a/-2a problem |
| **PDF/A-4f** | **easy** given -4 | **easy** given -4 | embedded files permitted, as -3 to -2 |
| **PDF/A-4e** | **moderate** | **hard/declined** | the engineering variant admits 3D and rich media, which is `CLAUDE.md`'s clause-13 exclusion by name; the validator can check the rest and refuse to judge what it does not read |
| the profile-data layer (§7.1) | **moderate** | — | writing a few hundred requirements as reviewable data from the normative text is the bulk of the work, and it is reading rather than research |
| the XMP writer (§5.6) | — | **easy** | RDF/XML in one fixed shape, beside a reader that already resolves namespaces |
| the shipped ICC profile (§5.3) | — | **easy or blocked** | technically trivial, entirely a licensing and data-dependency question for the owner |
| transparency flattening (§5.2) | — | **hard, and fenced** | `render-cpu` makes it buildable; `CLAUDE.md`'s redrawn exclusion makes it not currently legal |
| auto-tagging (§5.7) | — | **out of scope** | inventing a document's semantics is authoring content from nothing, which the amendment kept excluded |

**The shape of the table is the finding.** Every validator cell is *moderate* or *easy*, and none
of them is blocked on anything. The converter column splits cleanly in two: the parts that permit
transparency are moderate and buildable, and everything else is hard, declined or fenced. Nothing
in the middle.

## 10. Questions for the owner, most consequential first

1. **Is this in scope at all, and which half?** The commission asked for a converter. This RFC
   recommends the **validator first**, on the argument of §6: the validator is a reader whose
   every answer is a citation, which is what this project is, and the converter is a producer
   whose hardest requirements are unsatisfiable, undecidable or fenced. **Recommendation: accept
   the validator as a feature now, and take the converter as a separate decision after §8's
   layer 5 has measured what conversion would actually have to do.** The narrower sub-question
   with teeth: is a program that *tells you your file cannot be made conformant, and why*, an
   acceptable answer to "convert to PDF/A" — because on the evidence of §5.1 it is the answer a
   large share of real documents will get.
2. **Buying ISO 19005.** §0 says exactly what this RFC could and could not read, and the honest
   summary is that every requirement §3 and §5 attribute to PDF/A is second-hand except where
   ISO 32000-2 or ISO 14289 states the same thing in its own words. Principle 5 does not permit
   shipping a conformance verdict derived from veraPDF's rule set, so **a validator cannot be
   built to this project's standards without the normative text.** Four parts, plus amendments.
   **Recommendation: this is the one dependency the feature genuinely has, and it should be
   settled before any code.** The nearest precedent points the other way and is worth stating
   for that reason: on 2026-08-28 the owner declined to buy ISO 21757-1 because Adobe's own
   JavaScript for Acrobat API Reference *is* the working specification of that subject. There
   is no such document here. veraPDF's rule set is not ISO 19005 in the way Adobe's reference
   is the JavaScript API; it is somebody's reading of a text we would not have.
3. **Which parts, in which order?** **Recommendation: PDF/A-4 and PDF/A-2b first** — -4 because
   it is built on PDF 2.0, the version this tree reads natively and cites in every doc comment,
   so it needs the least translation; -2b because it is what people are actually asked for and
   what accepts a transparent document. -1 last, and -3 free once -2 exists.
4. **The ICC profile: ship one, or require the caller to supply one?** §5.3. A converter cannot
   write an output intent without a `/DestOutputProfile`, and this tree ships no profile.
   Ghostscript's answer is to make the user supply one. **Recommendation: require
   `--output-intent-profile <file>` with no default in tranche one** — it keeps a data dependency
   and its licence out of the tree, it is honest about the fact that *which* profile is a
   decision about the document rather than about the program, and it can be softened later by
   shipping a default if the owner wants one. The counter-argument is real: a flag with no
   default makes the common case awkward, and every competing tool has a default.
5. **Does the "no invented marks" fence move for PDF/A-1?** §5.2. **Recommendation: no.** A
   PDF/A-1 conversion of a transparent document refuses by name and names PDF/A-2b as the format
   that accepts it. The fence was drawn in ADR 0816 days before this document and a flattener is
   the second feature to arrive at it; moving it for a part of a standard that has been
   superseded twice would be the worst possible reason to move it.
6. **A validator's verdict when the standard is unreadable to us.** Where this RFC could not
   obtain a normative text (§0), a requirement can still be *implemented from a secondary source*
   or *left unimplemented and reported*. **Recommendation: never the first.** A requirement whose
   clause nobody here has read is reported as *not checked*, by name, in the validator's own
   output — the ledger's `unreviewed` discipline applied to a second standard — so that a clean
   verdict always says what it was clean over. A validator that silently omits a check is the
   corpus-going-quiet failure with a new denominator.
7. **Constructed appearances in a converted file.** §5.5. Where a document omits an appearance
   stream the standard requires, this tree can construct one and PDF/A requires one to be there.
   **Recommendation: write it, and report every one written** — the machinery exists, §12.5.5 and
   §12.7.4.3 make the appearance the standard's requirement rather than our invention, and the
   report is what keeps it a decision. But it is the closest call in the document and the owner
   should make it.
8. **Naming and the front door.** `pdf-transform archive --to pdf-a-2b` for the converter;
   for the validator, a verb on `pdf-retrieve` (it answers questions about a document, which is
   what a validator does) or a binary of its own. **Recommendation: `pdf-retrieve archive-check`,
   because RFC 0002 §4.1 already draws the line at "it answers questions, this one makes files"
   and a conformance verdict is an answer.**

## 11. Recommendation

**Build the validator. Decide the converter afterwards, on a measurement rather than on this
document.**

The order is:

1. **Obtain ISO 19005** (question 2). Nothing else in this list is legal under principle 5
   without it, and everything else in this list is cheap once it is done.
2. **`crates/pdf-archive`, part 4 and part 2 level b**, as reviewable requirement data plus a
   walk, over readers that all exist. Every requirement carries its clause, the way every row of
   the conformance ledger does, and every requirement nobody has read is reported as *not
   checked* rather than passed.
3. **Run it over the corpus, and over the veraPDF and Isartor corpora** `doc/PLAN.md` §4 already
   names — the corpus as this project's robustness denominator for a second standard, the other
   two as principle 5 evidence about our reading. The disagreements are the product of that
   round, not the pass rate.
4. **Then answer question 1 with the numbers in hand.** How many real documents fail only on
   requirements a converter could mechanically fix, and how many fail on a missing font program?
   If the second number dominates, the converter is a small feature attached to a large refusal
   and the validator was the whole product. If the first dominates, `pdf-transform archive --to
   pdf-a-2b` is a moderate verb on a serializer that already exists, and it should be built.

**What this RFC deliberately does not propose**: a flattener, an auto-tagger, a font synthesiser,
a bundled ICC profile, and any generation of conformance rules from another implementation's rule
set. Each is named above with the reason, and each is a separate argued amendment if the owner
wants it — never scope creep.

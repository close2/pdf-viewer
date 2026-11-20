# Scope ratifications

`CLAUDE.md` states the principles and, for each exclusion and each amendment to the authoring line,
the rule and the reason in a sentence. This document holds the rest of what ratified them: the date,
the question and the owner's answer, the ADRs and RFCs that argue and build it, the trigger that
reopens an exclusion, and the construction behind each amendment. It is written as what is. An
entry is a *claim about the specification or about the owner's word*, and it decays the way a
ledger row's does, so a round that touches scope opens the entry before it relies on the sentence
in `CLAUDE.md`.

An entry below is the full ratification of what `CLAUDE.md` states in a sentence (`doc/questions/A680`;
ADR 2492 is the arrangement and its proof). `CLAUDE.md` holds the rule, the reason and the closed
list; **revisiting an exclusion is by argument, never by attrition**, and the argument is written
where the exclusion is ratified, in the question and the ADR the entry names. A new exclusion, a new
amendment or a new reopening trigger is added here in the same commit as the sentence in
`CLAUDE.md` that states its rule.

## The exclusions

### Clause 13, the playing of video and the rendering of 3D artwork

**Clause 13, the playing of video and the rendering of 3D artwork** — a media engine, not a
rendering question. **In scope**: what a still page shows of a media object — a movie's
stream-form poster, every producer-supplied appearance, §13.2.2's viability computed and reported
in the clause's own terms — the media file saved or handed to this machine's own player under *A
document's restrictions are the reader's to set*, `ask` by default, and **audio decoded and
presented by this program**: one object's bytes, already unfiltered, decoded in `pdf-media-worker`
under `pdf_sandbox::lockdown::Profile::Media` (no filesystem, no network, no descriptor, no
executable memory, an address-space ceiling and a processor-time budget per request), started on
first play and never at open, the host given samples and never the document's bytes; built in RFC
0015 section 5's order, §13.3's own sample formats first, and the window's audio sink on the output
side (ADR 3297). **Excluded still**, each with the trigger that reopens it:

- **decoding or presenting video** — reopened when audio has shipped and its clock and sinks are
  measured in use, and the owner then decides to open RFC 0015 section 6's order, on an H.264 decoder
  admissible under `deny.toml` (RFC 0012 audits one that is) or on C in a confined media worker
  accepted in writing under ADR 0014;
- **rendering U3D artwork** — a U3D reader in Rust that reads ECMA-363's **third** edition whole and
  is fuzzed, or the owner's decision to write one in-tree (ADR 2400; ISO 32000-2 cites the third
  edition and says it does not cite the fourth, RFC 0015 section 7);
- **rendering PRC artwork** — the project holds ISO 14739-1, and a PRC reader in Rust reads it whole
  and is fuzzed, or the owner's decision to write one in-tree (RFC 0015 section 8).

Rich media (§13.7) and §12.6.4.16's Go-To-3D-View follow the parts they are made of and carry no
trigger of their own; §12.6.4.18's Rich-Media-Execute and §13.7's run time stay with the script
exclusion's ISO 21757-1 clause; §13.5's alternate presentations are planned by no part, on the
clause's own release of the reader.

Ratified on 2026-10-08 (`doc/questions/Q254`, `A254`: RFC 0009 section 11's question 2 adopted as
written); RFC 0009 section 3 is the argument, ADR 1946 the ledger's reading. Amended on 2026-11-20
(`doc/questions/Q946`, `A946`: RFC 0015 section 12's six recommendations adopted) — audio opened
alone, video kept closed behind audio, U3D's trigger restated to the edition the standard cites,
PRC's trigger written; RFC 0015 is the argument and ADR 3297 the first build.

### JavaScript and script-driven form behaviour beyond this document and this viewer

**JavaScript and script-driven form behaviour beyond this document and this viewer** — a script
runs in scope as far as it reaches *this document's* field values and appearance properties and
*this viewer's* state, evaluated in a confined worker of its own under a memory and a wall-clock
budget, and only at the level the reader sets under *A document's restrictions are the reader's
to set* — `off` by default, and pinned `off` in a window that cannot supply a runner. Acrobat's
`AF*` form library is in scope as a documented choice with no engine. **Excluded still**: every
call that leaves the process — the network, the filesystem, mail, printing, other documents, the
application's chrome, persistence across documents, privileged functions — each refused by a
named exception the script can see; any effect that would mutate `pdf_syntax::Document`; ISO
21757-1's 3D API under clause 13; XFA under Annex K. Ratified on 2026-10-08
(`doc/questions/Q286`, `A286`); ADR 1904 names what is built under each half.

### Authoring content from nothing, and what stays on the near side

**Authoring content from nothing** — we do not compose pages: no layout engine, no
text-setting, no chart drawing, no "HTML to PDF". No clause whose subject is deciding what
marks a page should contain falls on this project.

**Assembling documents from existing documents is in scope** (RFC 0002 §11.1, ratified on
2026-09-03). Splitting, merging, reordering, rotating, extracting and optimising operate on
content some producer already specified; every content stream in their output is a producer's,
carried byte for byte or recompressed without reinterpretation. The writer this requires is
§10 of RFC 0002's serializer: it emits structure (object table, streams containers,
cross-reference, trailer, identifiers), never content. §7.5.6's incremental update remains the
only writing that touches a file a user is *editing in place*; the serializer is how a *new*
file is derived from old ones. Generator obligations come into scope only where the serializer
actually emits the construct:
§7.5.4/§7.5.5/§7.5.7/§7.5.8 on the way out, §14.4, §7.6 encryption on the way out, and Annex
F, which `optimize --linearize` emits (`doc/questions/A03`). The ledger's `writer-side` status
narrows accordingly.

## The four amendments to the authoring line

The boundary line they sit on is in `CLAUDE.md`: does the operation invent marks?

### A page composed solely of content the document already holds

**A page composed solely of content the document already holds.** An archival conversion
may *append a page* to carry what its target will not otherwise admit — an embedded image
the target may not attach, a metadata packet whose properties the target's schemas do not
define, the statement a signature made to its reader (who signed, when, and whether it
verified) — where the alternative is losing that content outright. Nothing may be put on it
whose content did not come from the file, and under a Level A target the structure-tree
entries such a page owes (ISO 19005-2 section 6.7) are inside the same permission, because
refusing to describe a page this program appended would make the output non-conforming
rather than more honest. It and the photographed page below are the content streams this
program writes that no clause specifies. Ratified 2026-09-12 (`doc/questions/Q58`, `A58`:
"Q58 agree with recommendation"); ADR 1014 is the argument and names what an appended page
owes — page labels, the outline, the structure tree, and the report's sentence saying a page
was appended.

**A page may also state what an archival conversion removed**, in facts computed from the file
and a fixed vocabulary naming the requirement and the loss; it carries no other text. Ratified
2026-10-28 (`doc/questions/Q706` item 2, `A706`: "agree on all"); RFC 0014 section 4 is the
design, and ADR 2536's section on the statement page is the construction — the facts each listed
object carries, the vocabulary fixed in `crates/pdf-transform/src/archive/statement.rs`, and the
bounds it is refused by name past. It is reopened if a statement is asked to carry anything but
facts computed from the source and that vocabulary.

### A producer's own marks written onto a page the producer wrote

**A producer's own marks written onto a page the producer wrote** — **clause-driven
relocation of producer-written appearance streams**, on a two-part test: a clause requires
the move, and that clause fixes the position (§12.5.5 computes an appearance's matrix from
the annotation's own `/Rect`, `/BBox` and `/Matrix`), so the operators that place it decide
nothing a reader sees, and an appearance whose annotation a target will not admit goes back
onto its own page under §12.5.5's own matrix. It takes in later sites of the same shape, a
flattened form field among them, and **stops where the marks become this program's
invention**: the watermark
stays on the far side, and so do the redaction's overlay text and its fill — §12.5.6.23's
`/OverlayText` and `/IC` (`doc/questions/A64`). ADR 1123 is the construction — a `q` before
the producer's operators and a closing stream after them, §8.4.2's balance kept across the
`/Contents` array — with ADR 1099's appended page as the fallback for its two refusals, each
stated in a sentence: a producer whose content stream pops further than it pushes, and a
moved appearance a remaining, unhidden annotation would be drawn over. Ratified 2026-09-14
(`doc/questions/Q65`, `A65`: "Rule provenance."); ADR 1120 is the argument.

**A redaction's `/RO` is on the near side; its `/OverlayText` and `/IC` are not.** Table 195's
`/RO` is "a form XObject specifying the overlay appearance for this redaction annotation", and
"\[a]fter this redaction is applied and the affected content has been removed, the overlay
appearance should be drawn such that its origin lines up with the lower-left corner of the
annotation rectangle": the marks are the producer's own stream, and the clause both asks for
the move and fixes the position, which is the two-part test above. `/OverlayText` is a string
this program would set in type under `/DA` and `/Q`, and `/IC` a fill this program would paint
— marks it composes, so "the redaction's overlay" the line excludes is read as those two. The
owner declined drawing anything over content that is not gone (`doc/questions/A834`, item 3:
"If a redaction only places something over someting else, it will lead to sensitive
information exposure"), and agreed to draw `/RO` strictly after the removal
(`doc/questions/Q842`, `A842`, 2026-11-13: "agreed"). So the form is appended after the content
the removal wrote and is never walked by it, and a page whose removal is refused draws no
overlay. ADR 2868 is the construction and ADR 2869 the reading of the line; this is reopened if
a `/RO` is ever asked to be drawn where the removal did not run first.

### A page photographed as this reader draws it, where no reader could read its text

**A page photographed as this reader draws it, where no reader could read its text.** An
archival conversion may replace a page's content with one image of the window's own rendering
of it — blank marks included, its annotations left drawn over it — where an unembedded
`Adobe-Identity` font left text that nothing in the file names (often under an Identity
CMap, which §9.7.5.2 forbids with such a font), and only there: last in the owner's order after
`--font`, the shipped faces and re-keying, and refused by name wherever the page's readback
names any character, the interpreter reports more than the fonts it could not draw, or the
document states a structure tree or optional content. It chooses no font and composes nothing
— every mark is the producer's as this reader draws it — and it is carried as the named loss
`text-as-image`, which the report and the file's own history state. Ratified 2026-10-26
(`doc/questions/Q570`, `A570`: "agreed, please answer"); ADR 2394 is the construction.

### A page laid out by a producer's own XFA template

**A page laid out by a producer's own XFA template.** Where a document's interactive form carries
an XFA resource whose template is not XFAF — Table 29's `/NeedsRendering` is `true`, or the
template states an `occur` whose bounds differ, a `subformSet` or other grammar XFA 3.3 excludes
from XFAF (pages 272–273) — the pages that template generates, laid out by XFA 3.3's own layout
and binding rules from the template and the document's own data, are on the near side. Every mark
on such a page is one the template names; what this program decides is only what XFA 3.3's
algorithm decides. Nothing may be placed on it that the template and its data do not state, and a
page so laid out is said to be generated, in the window's title or report, as an appended page is.

**The argument, and the test it fails.** It passes ADR 1120's provenance test — every mark is the
producer's: a `draw`'s text, a field's value and caption, a border, a barcode the template types
and the data fills — and it fails ADR 1123's second part, because no clause of ISO 32000-2 fixes
the positions: XFA 3.3, the text §K.2 names as the format of the resource, fixes them instead. The
sentence the owner accepted is that **a normative reference's layout algorithm may stand where a
clause's position would**. The case for it is the target: "every PDF that exists renders as its
producer specified", and the producer of a `/NeedsRendering` file specified its pages in the
template, not in the wrapper page. What stays on the far side: a watermark, a stamp, text this
program composes, HTML to PDF, and any layout whose source is not the document's own template.
§K.1's permission ("a PDF processor may choose to not implement this feature") is taken until the
build lands, and said, as it was.

**When, and the first requirement.** It is built after the ISO 32000-2 ledger rows reach their
final states, in the order `doc/todo/78` section 4 lists — the merge and page one's launch cost
first. Its first requirement is the owner's: at the reader's script level `off`, which is the
default, page one is laid out from the template and the data alone, and the window says that the
form's scripts did not run. Until it is built, a `/NeedsRendering` document shows the page its
producer wrote for processors that decline, and every window says it is an XFA form and what is
shown instead (ADR 1932).

**Barcodes.** A template's `<barcode>` is one of the marks the template names. Its symbologies are
built from public sources where that is reasonable, the edge cases a missing ISO text leaves open
written down under principle 5, and an ISO text is bought only where it is not (`doc/questions/A840`
item 3); ADR 2877 places the encoders and says what is built.

Ratified 2026-11-13 (`doc/questions/Q840`, `A840`: "agree on 1 and 2"); RFC 0011 section 7 is the
construction as asked, `doc/todo/78` the price (ADR 2837), ADR 2876 the amendment. It replaces the
exclusion `doc/questions/A639` kept until that price was measured. It is reopened if a page is
asked to carry anything the template and its data do not state, or to be laid out by any algorithm
but XFA 3.3's.

### XFA's static half, which needed no amendment

**XFA's static half is in scope** (RFC 0011, cut where XFA 3.3 cuts static from dynamic by ADR
2380; `doc/questions/A633`, `A639`): a hybrid form's template behaviours — its `datasets` packet
kept consistent on save, formatting, validations, calculations and events — at one round per
batch, every mark still the AcroForm widget's. Annex K is normative and in ISO 32000-2; what is
documented separately is the XFA template architecture. Annex K also says why the static half
needs no layout: in a conforming hybrid file "[t]he other entries in the interactive form
dictionary shall be consistent with the information in the XFA resource", so the AcroForm this
tree reads *is* the form.

## What a user does to a document already open

What a *user* does to a document already open — an annotation added, a field filled — is not
authoring, and it is written back by §7.5.6's incremental update: the new objects and a new
cross-reference section appended, never a rewrite of what was there. The producer's bytes stay
in the file, byte for byte, under whatever the user added. This tree already reads that
construction (ADR 0100), which is why it is the one form of writing it is placed to get right.

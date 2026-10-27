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

### Clause 13, the playing of multimedia and the rendering of 3D artwork

**Clause 13, the playing of multimedia and the rendering of 3D artwork** — a media engine,
not a rendering question. **In scope**: what a still page shows of a media object — a
movie's stream-form poster, every producer-supplied appearance, §13.2.2's viability computed
and reported in the clause's own terms — and the media file saved or handed to this machine's
own player under *A document's restrictions are the reader's to set*, `ask` by default.
**Excluded still**, each with the trigger that reopens it: decoding or presenting audio and
video (the owner's decision to build the player, on an H.264 and an AAC decoder admissible under
`deny.toml` — RFC 0012 audits two that are, and ADR 2428 records the build and the order a
reopening takes — or on C in a confined media worker accepted in writing under ADR 0014), and
rendering U3D or PRC artwork (a U3D reader in Rust that reads ECMA-363's fourth edition whole and
is fuzzed, or the owner's decision to write one in-tree; ADR 2400).
Ratified on 2026-10-08 (`doc/questions/Q254`, `A254`: RFC 0009 section 11's question 2
adopted as written); RFC 0009 section 3 is the argument, ADR 1946 the ledger's reading.

### XFA's dynamic half, and its static half

**XFA's dynamic half** — a form whose template generates its pages: Table 29's
`/NeedsRendering` true, or grammar XFA 3.3 excludes from an XFAF form (an `occur` whose bounds
differ, a `subformSet`). The standard hands this one over itself: "[t]he implementation of such
a schema driven page generation involves considerable effort beyond that for a simple PDF viewer
and therefore **a PDF processor may choose to not implement this feature**" (§K.1). It stays
excluded until a measurement round has priced laying out page one of the corpora's
`/NeedsRendering` documents and the owner has accepted, separately, the near-side
construction RFC 0011 section 7 asks for (`doc/questions/Q639`, `A639`).

**XFA's static half is in scope** (RFC 0011, cut where XFA 3.3 cuts static from dynamic by ADR
2380; `doc/questions/A633`, `A639`): a hybrid form's template behaviours — its `datasets` packet
kept consistent on save, formatting, validations, calculations and events — at one round per
batch, every mark still the AcroForm widget's. Annex K is normative and in ISO 32000-2; what is
documented separately is the XFA template architecture. Annex K also says why the static half
needs no layout: in a conforming hybrid file "[t]he other entries in the interactive form
dictionary shall be consistent with the information in the XFA resource", so the AcroForm this
tree reads *is* the form.

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

## The three amendments to the authoring line

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

### A producer's own marks written onto a page the producer wrote

**A producer's own marks written onto a page the producer wrote** — **clause-driven
relocation of producer-written appearance streams**, on a two-part test: a clause requires
the move, and that clause fixes the position (§12.5.5 computes an appearance's matrix from
the annotation's own `/Rect`, `/BBox` and `/Matrix`), so the operators that place it decide
nothing a reader sees, and an appearance whose annotation a target will not admit goes back
onto its own page under §12.5.5's own matrix. It takes in later sites of the same shape, a
flattened form field among them, and **stops where the marks become this program's
invention**: the watermark
stays on the far side, and so do the redaction's overlay and its fill — §12.5.6.23's `/RO`,
`/OverlayText` and `/IC` (`doc/questions/A64`). ADR 1123 is the construction — a `q` before
the producer's operators and a closing stream after them, §8.4.2's balance kept across the
`/Contents` array — with ADR 1099's appended page as the fallback for its two refusals, each
stated in a sentence: a producer whose content stream pops further than it pushes, and a
moved appearance a remaining, unhidden annotation would be drawn over. Ratified 2026-09-14
(`doc/questions/Q65`, `A65`: "Rule provenance."); ADR 1120 is the argument.

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

## What a user does to a document already open

What a *user* does to a document already open — an annotation added, a field filled — is not
authoring, and it is written back by §7.5.6's incremental update: the new objects and a new
cross-reference section appended, never a rewrite of what was there. The producer's bytes stay
in the file, byte for byte, under whatever the user added. This tree already reads that
construction (ADR 0100), which is why it is the one form of writing it is placed to get right.

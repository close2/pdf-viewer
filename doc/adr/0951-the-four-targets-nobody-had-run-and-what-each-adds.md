# 0951 — The four targets nobody had run, and what each of them adds

Session 951. Status: **accepted**. The fifth slice of `quorra-transform archive`
(`doc/questions/A22`): `crates/pdf-transform/src/archive.rs` split into a directory module, the
corpus sweep extended from two targets to all six, and the three levels and two flavours past
PDF/A-2b given what each of them adds — or a refusal with its own argument where it cannot be
given.

## 0. The split came first because everything else needed it

`archive.rs` was 3 523 lines, and every remaining slice of this converter had to edit it. Five
files now, one responsibility each, on the seams the three stages already were:

| file | one responsibility |
|---|---|
| `decision.rs` | `REMEDIES`, the five answers, and the requirement identifiers that key them |
| `prepare.rs` | what a decision needs built from the document before it can be taken |
| `rewrite.rs` | the rewrites, one object at a time, and the serializer walk |
| `to_unicode.rs` | the `/ToUnicode` CMap a font's own encoding derives, and what it cannot |
| `report.rs` | what was decided and done, for a person and for `--json` |
| `mod.rs` | the three stages in order, the plan, and the output's version |

No behaviour changed and the test counts either side say so: 1 806 before, 1 806 immediately
after, 1 812 once this slice's six new tests were written.

One thing the split produced that a reader should not mistake for tidying. `decide` used to carry
a special case — `Answer::Mechanical(Rewrite::IdentificationSchema)` matched by name, so that a
document whose XMP packet could not be edited was refused with the packet's own reason rather
than with a rewrite that would not happen. That special case is now `Prepared::obstacle`, asked of
every mechanical rewrite: **a rewrite is an answer only where the document can take it**, and the
reason it cannot is the reason the requirement is refused with. Three of this slice's rewrites
needed exactly that gate, and a fourth special case would have been the point at which nobody
could read the function.

## 1. Six targets, and the sweep is what found the work

`tests/archive_corpus.rs` covered `PDF_A-4` and `PDF_A-2b`. It covers all six directories now, and
it prints, per target, the requirements that stopped a conversion in rank order. That listing —
not a reading of the standard, and not a guess — is what decided everything below: at PDF/A-2u
two requirements account for every refusal, at PDF/A-2a five do, at PDF/A-4f two, and at PDF/A-4e
one. Four of the six targets had never been run before this session; none of them panicked, none
returned an error, and every document that conformed before a conversion conformed after it.

The counts themselves are not in this file, for `CLAUDE.md`'s reason. The command is in
`doc/pdf-a-conversion-limits.md` section 9.4.

## 2. Level U: the CMap is derived from the clause's own second method, or not at all

ISO 19005-2 section 6.2.11.7.2 wants a `/ToUnicode` CMap on every font outside its four
exemptions, and usable values in the CMaps a file already has.
`doc/pdf-a-conversion-limits.md` section 4.3 draws the line: what can be *derived* is a CMap
written from a font's own encoding where the glyph names are the standard ones; what cannot is a
guess.

The derivation is ISO 32000-2 §9.10.2's **second method** and nothing else — the glyph name the
encoding selected, looked up in the Adobe Glyph List by that list's own algorithm, with Annex D.6
answering for `ZapfDingbats` names the list does not hold. Three consequences, each of which is
the clause rather than a policy:

- **A composite font derives nothing.** The second method is conditioned on a simple font, because
  only a simple font selects its glyph by name; §9.7.4.2 makes a CID an index into the glyphs of
  the font that defined it, which says nothing about any character.
- **A symbolic TrueType reaching its `cmap` by code derives nothing**, for the same reason: no
  name was used, so there is nothing to look up.
- **A font `pdf-font` had to substitute for derives nothing.** The names would then be the
  substitute's rather than this file's — the same guard `pdf_archive` applies when it rules the
  exemption out, applied here for the same reason.

Two rules about what is written, and the second is the one that keeps this honest:

- **The producer's usable values are kept.** A code the file already maps to a usable character
  carries into the derived CMap unchanged; what is replaced is the codes the clause calls
  placeholders — zero, U+FEFF, U+FFFE — and the codes the table never held.
- **A font with one underivable code produces no CMap at all.** A table written over a prefix of
  the referenced codes would say that the rest mean nothing, which is a worse statement than the
  absent entry the clause is complaining about. The refusal names which of six reasons stopped it.

**The population is the validator's findings rather than a walk of this converter's own**, and
that is the load-bearing choice. The clause has four exemptions; reading them belongs to
`pdf_archive`, and a converter that decided for itself which fonts need a CMap would be
re-deriving a reading the project says has one. A findings list is capped, so a document failing
at more fonts than the cap is one ADR 0947's third stage refuses rather than one this rewrite
half-finishes.

**And the corpus cannot show this working, which is a fact about the corpus and is stated rather
than hidden.** All eight veraPDF `PDF_A-2u` documents that fail the clause fail it in exactly the
underivable way, by construction: the second exemption excuses a Type 1 or Type 3 font whose names
are all listed, so a Type 1 font that *fails* the rule is one whose names are not. The witnesses
are therefore three hand-built fixtures in `tests/archive.rs`, embedding a real font program
because the derivation refuses a substituted font — a placeholder replaced by the character the
encoding names, a usable value kept, and a private glyph name refused.

## 3. Level A: the flag is written down, the tree is never invented

`doc/pdf-a-conversion-limits.md` section 5.1, corrected by the owner in conversation and worth
restating as the correction it was: **the converter will not invent a structure tree, and that is
not the same as refusing PDF/A-2a.**

So ISO 19005-2 section 6.7.2.2's `/MarkInfo` with `/Marked true` is written — but only into a
catalog that already states a `/StructTreeRoot`. ISO 32000-2 §14.7.1's Table 353 says what the
entry is: "[a] flag indicating whether the document conforms to tagged PDF conventions". A file
carrying a tree has demonstrated that much and the flag records it; a file with none would be
having the claim made on its behalf by this program. ISO 19005-2 section 6.7.1 advises writers
against adding structural information not present in the source solely to achieve conformance, so
declining is the standard's counsel rather than this project being strict.

What remains refused at Level A is refused **by argument**, which is the third table this slice
added.

## 4. `REFUSED_BY_NAME`, and why "not yet" was becoming a lie

Every requirement outside `REMEDIES` and `WRITER_EMITS` used to be refused with one sentence:
*this converter does not yet meet this requirement*. That sentence promises a later slice. For a
structure tree, for word boundaries inside show strings, for an `/ActualText` covering a span of a
content stream, for a 3D format Annex B does not name, and for a PDF/A-4f target asked of a
document with nothing embedded, it is false — no slice of this converter will ever do those, and a
user told "not yet" comes back tomorrow for the same answer.

`REFUSED_BY_NAME` is ten rows, each carrying the `Because` its argument warrants:
`TheFence` where the act is on the far side of ADR 0816's fence or is authoring content nobody
produced, `NotThisTarget` where no conforming file can be made *from this document for this
target*, `NotBuiltYet` only where a later slice genuinely owes it. `Because::TheFence`'s doc
comment widened to say so — it read "edit a content stream or invent a mark", and inventing
semantics nobody produced is the same fence one clause over.

Nothing in the table is load-bearing for safety: a requirement in none of the three tables is
still refused, and still writes no file. What it is load-bearing for is the report saying
something true. `archive::refused_by_name` is public for the same reason `answered` is — a typo in
one of the keys does not fail to compile, it quietly demotes an argument back to "not yet" — and
`tests/archive.rs` compares both against the identifiers `pdf_archive` actually states.

## 5. PDF/A-4f and PDF/A-4e: two keys the standard supplies, and one target a document can be
   wrong for

ISO 19005-4 section 6.9 requires `/F`, `/UF` and `/AFRelationship` of every embedded file's
specification, and Annexes A.2 and B.4 keep all three while lifting the requirement that the file
*itself* conform. Both keys are written from what §7.11.3's Table 43 already says:

- `/F` and `/UF` are the same file name in two types, so one is written from the other — **and
  only where its bytes are ASCII**. Outside that range a byte string's character set is a fact the
  file does not state, and writing a text string from it would be asserting an encoding.
- `/AFRelationship` gets `Unspecified`, which is not a choice this converter makes: Table 43's own
  row ends "Default: Unspecified", so a specification without the entry already relates to the
  document in exactly the way the name records. None of the other seven values is ever written,
  because none can be established from a file specification.

**Annex A.2 is the one requirement in either part that a document fails by holding nothing at
all**: a PDF/A-4f file shall contain an `EmbeddedFiles` key. Attaching a file to satisfy it would
be adding content no source states, so this is `NotThisTarget` and the report says which target
the document is for.

**Annex B adds almost nothing a converter can act on**, which is a finding rather than an
omission: it relaxes clause 6 for 3D and `RichMedia` annotations, `SetOCGState` and `GoTo3DView`
actions and embedded files of any type, and the one rule it adds that binds a *file* is B.2.2's
`U3D`-or-`PRC` `Subtype`. Translating 3D artwork between formats is a media engine, and
`CLAUDE.md` excludes ISO 32000-2 clause 13 by name.

## 6. What was deliberately not built, and why it is named rather than stubbed

**The recursive conversion of an embedded PDF.** ISO 19005-2 section 6.8 and ISO 19005-4 section
6.9 require an embedded file to conform to a part of ISO 19005 itself, and the honest way to meet
that is to run this whole verb over the embedded bytes under a budget of their own. It is a slice,
not a corner of one, and the sweep says no corpus document is waiting on it — the requirement
stopped zero conversions across all six targets. So it is refused with a sentence that names the
work and names the two flavours that lift the requirement altogether, rather than half-built.

**The `/Lang` repair.** Section 5.1 offers "repairing or removing a malformed one". Repairing means
deciding what the producer meant by a string that is not a language identifier; removing means
throwing away a claim they made. The second is section 3's kind of act — a loss somebody has to
authorise — and it waits on the four-level policy interface section 9.5 describes rather than on a
slice of code.

**The embedded file's media type.** ISO 19005-4 section 6.9 asks the stream for a `Subtype` that
is a MIME media type, and nothing in a file specification states one: a file name's extension is a
convention rather than a declaration.

## 7. One dependency taken, and the argument for it

`pdf-transform` gained `pdf-font`. The Cargo comment states the case and it is the same one the
crate already makes for `pdf-archive`: what a character code selects — §9.6.5's encoding,
§9.10.2's glyph-name route to a character — is a reading this tree has made once, and a converter
deriving it a second time would be a second reading of the same clauses.

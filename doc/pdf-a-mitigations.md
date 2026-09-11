# Mitigations: what a refusal could do instead, requirement by requirement

Commissioned by the project owner on 2026-09-11:

> Please really take a step back for every refusal and consider what the best mitigation would be
> and if a human could choose this mitigation using a configuration.

This file is the answer, and it is a **catalogue rather than an argument**. `doc/rfc/0007` designs
the mechanism — four remedy kinds, a site-and-target pair, a departure with a predicate — and says
in its own section 4.7.5 that *the catalogue is the bulk of this feature, not the mechanism*. What
follows is that bulk: every requirement this converter refuses today, with four answers beside it.

`doc/pdf-a-conversion-limits.md` is the companion and is not repeated here. Where it already argues
a loss, this file cites its section and adds only what is new.

## 0. How to read an entry

Each entry answers the owner's four questions in a fixed order:

- **Mitigation** — what the best answer is, from `doc/rfc/0007` section 2's closed vocabulary
  (`stop`, `discard`, `preserve`, `derive`), stated concretely. Where the honest answer is *none*,
  the entry says so with the argument, and that is finished work rather than a gap.
- **By target** — whether the six targets differ, and whether they differ **in kind** (a different
  answer) rather than in availability (the same answer, reachable or not).
- **From a configuration** — what an operator would have to know, and what they would type. A
  mitigation an operator cannot express, or cannot understand the cost of, is not one.
- **Departure** — RFC 0007 section 4.7.5's kind **A** (another part of ISO 19005 already relaxes it
  — computed, not guessed), **B** (no part does, and departing still leaves an archive), or **C**
  (departing defeats what the format is for), and whether a predicate can narrow it.

The header line of each entry carries the requirement identifier, the clause in each part that
states it, which targets bind it, and what the converter answers today.

### 0.1 Provenance, and why nothing from ISO 19005 is in quotation marks

The same rule `doc/pdf-a-conversion-limits.md` states: ISO 19005-2:2011 and ISO 19005-4:2020 are
licensed to a single reader, so this file cites their clauses by number and paraphrases. ISO 32000-2
is quoted normally, in quotation marks, and `cargo test -p conformance` checks those quotations
against `doc/md/`. A `§` is a clause of ISO 32000-2 and nothing else; ISO 19005 clauses are written
out in full.

**How a section reference is written here.** Three documents are cited constantly and the rule about
`§` makes the distinction cheap to state:

- **`§`** is a clause of ISO 32000-2 and nothing else.
- **"the limits document's section 3.5"**, or "section 3.5 of the limits document", is
  `doc/pdf-a-conversion-limits.md`.
- **"RFC 0007 section 4.6"** is `doc/rfc/0007`; where the sentence already names the RFC, the
  reference is a bare "section 4.6".
- **A bare "section 13.3"** with no document named is a section of this file.
- ISO 19005's clauses are written out in full — "ISO 19005-2 6.2.4.3" — never with a `§`.

### 0.2 Three vocabulary findings, before the catalogue

Working the list turned up three things the RFC's vocabulary does not hold. They are reported here
because they are findings about the format, which section 14 of this file collects.

**`supply` is a fifth remedy kind, and it is the direct answer to the owner's question.** RFC 0007
section 2's four kinds all describe what happens to *the document's* content. A whole class of
refusals is unblocked instead by **the operator stating a fact the document does not** — a role map
for a producer's own structure types, the archive's language, which of two conflicting `Separation`
definitions wins, the media type of an attachment, which of a CMap's two write modes its producer
meant. This is not `derive`: no tool makes anything, and nothing is inferred from the document. It
is a human putting their own knowledge into the file, recorded as theirs. It is the single most
common way a refusal in this catalogue becomes a conversion, and `A48`'s line is not crossed by it,
because `A48` binds *this program's* guessing and not an operator's statement. It does need `A48`'s
other half: the report and `xmpMM:History` must say the value came from the configuration rather
than the document.

**A refusal that is `NOT_BUILT_YET` because a lossless rewrite is owed must not be configurable at
all.** Around a third of the rows below are of that kind: the right answer loses nothing, nobody has
written it, and offering an operator a `discard` to get past it would trade a permanent loss for a
missing afternoon's work. Those entries are marked **owed, not optional** and their configuration
answer is *none by design*.

**Predicates come in two costs and the format should say which.** RFC 0007 section 4.7.2's
`media-type = [...]` is declarative: the converter reads a key and compares it. *"Depart only where
removing the transfer function changes no pixel"* is a **computed** predicate — it needs the
document rendered twice. Both are useful; only the first is free, and a configuration format that
spells them the same way hides that from the person writing it.

## 1. The list, and what the census computed

The instrument is `crates/pdf-transform/src/archive/census.rs` (ADR 0955):

```sh
cargo run -q -p pdf-transform --example archive_census
```

It counts target-requirement pairs. As **distinct requirements** the refusals are **118**, and every
one has an entry below:

| area | requirements refused |
|---|---|
| graphics (colour, halftones, images, content streams) | 38 |
| fonts | 20 |
| implementation limits | 10 |
| file structure | 10 |
| metadata | 8 |
| annotations | 8 |
| actions | 8 |
| logical structure | 4 |
| forms | 4 |
| embedded files | 4 |
| optional content | 3 |
| signatures | 1 |

### 1.1 Kind A is computed, and the computation over-reports

RFC 0007 section 4.7.5 says the census can decide kind A mechanically, and it can: a requirement
that does not bind all six targets is one some part or flavour of ISO 19005 does not state. **57 of
the 118 are kind A candidates by that test** — every implementation limit, every Level A
requirement, the part-2 JavaScript and `/AS` prohibitions, the part-4 `/Info` rules.

**The computation over-reports, and the over-reporting is itself a finding.** Three shapes make a
candidate spurious:

- **Renumbered siblings.** `annotations/subtype-defined-in-iso-32000-1` binds part 2 only and
  `annotations/subtype-defined-in-iso-32000-2` binds part 4: that is one rule under two spellings,
  not a relaxation. The genuine relaxation inside it is narrower — part 4 admits `3D` and
  `RichMedia` at 4e and `FileAttachment` at 4f.
- **Superseded keys.** `graphics/no-halftone-phase-in-a-graphics-state` (part 2, `/HTP`) and
  `graphics/no-halftone-origin-in-a-graphics-state` (part 4, `/HTO`) forbid the key of their own
  edition. Neither part permits what the other forbids.
- **Requirements that only exist at one target because that target invented the feature.**
  `embedded-files/pdfa-4f-carries-embedded-files` binds 4f alone because only 4f requires an
  attachment at all.

So the census's A is a **candidate list a reader must confirm**, not a verdict. Read one by one,
**43 of the 57 are genuine** and 14 are one of the three shapes above. The remaining sort is a
judgement and the entries carry it: **20 requirements where this catalogue says depart from nothing
(C)**, **8 where the departure splits B/C on a predicate**, and the rest **B**.

One caution about what the sort means, because two entries would otherwise look contradictory. **A
is about evidence, not about advice.** A requirement can be a genuine kind A — some part of ISO
19005 does not state it — and its entry can still say *do not depart here*, because departing at the
target that does bind it defeats what that target is for. `logical-structure/structure-tree-root` is
the standing case: 2b and 2u ask nothing of logical structure, so the evidence is A, and a Level A
file with no structure tree is a claim about the one thing Level A means, so the advice is C. Where
the two diverge, the entry's answer is **retarget** rather than either.

### 1.2 The three general findings that recur

Three answers turn up again and again, and stating them once keeps the entries short.

**The 4f preserve.** ISO 19005-4 Annex A.2 lets a PDF/A-4f file hold an embedded file of *any* type.
So at that one target, **anything this converter would otherwise throw away as a byte string can be
kept as an attachment**: a rejected ICC profile, a media stream from a removed `Screen` annotation,
an XMP packet that would not parse, an XFA packet, a halftone or transfer function sampled to a
file, the source document itself. ISO 19005-4 Annex B admits the same for 4e. This turns a long list
of `discard`s into `preserve`s at two of the six targets, and it is the largest single thing this
catalogue found. The cost is honest and small: a reader has to go looking, and the information is no
longer in the place a PDF reader interprets.

**The appended page.** `doc/rfc/0007` section 4.6.1: all six admit as many pages as a document
likes, so appending is available where attaching is not. What it costs is section 4.6.2's list —
page labels, the outline, and at Level A the structure tree — and `Q58` is open on whether composing
such a page is inside `CLAUDE.md`'s authoring exclusion at all. Every entry that proposes one says
so.

**The operator's own knowledge.** section 0.2's `supply`. Where the refusal is *nothing in the file
says which*, the operator often knows, once, for their whole archive.

---

## 2. File structure and encryption

### `file-structure/no-encryption`
ISO 19005-2 6.1.3, ISO 19005-4 6.1.3 · all six · today `not-built-yet`

- **Mitigation** — `discard`, and **a `preserve` nobody had written down beside it**. Decrypting and
  dropping `/Encrypt` is the limits document's section 3.5 Ask and stays that. What the limits
  document's section 3.5 treats as lost with it is Table 22's `/P` flags — *"no printing", "no
  extraction"* — and those are a **statement**, which survives even though the enforcement cannot:
  write the permission set the producer asserted into the report and into `xmpMM:History`, and under
  4f attach the source file's own permission record. The archive then says what the document
  claimed, which is what an archivist actually wants; what it no longer does is enforce it. This is
  the same move `doc/rfc/0007` section 5 makes for a signature, and it applies here for the same
  reason.
- **By target** — no difference: both parts forbid `/Encrypt` outright.
- **From a configuration** — `remedy = "discard"`, and the operator must know two sentences: the
  output is readable by anyone holding it, and the producer's permission flags stop being asserted.
  What a configuration **cannot** carry is the password: it is per document, so the practical shape
  is `--password` on the command line or a per-document sidecar, with `on-missing = "stop"`. An
  archive whose queue is its own documents usually holds one owner password for the fleet, and that
  is expressible; a queue of documents from the public is not, and the configuration should not
  pretend otherwise.
- **Departure** — **C**. `doc/rfc/0007` section 4.7.5 names encryption as its own example and the
  reasoning holds: an archive nobody can decrypt is not an archive. No predicate rescues it — not
  even *"depart only where the user password is empty"*, because a file that is still encrypted is
  still a file whose readability depends on an algorithm surviving.

### `file-structure/crypt-filter-is-identity`
ISO 19005-2 6.1.7.2, ISO 19005-4 6.1.6.2 · all six · today `not-built-yet`

- **Mitigation** — as above; it is the per-stream half of the same act and has no separate answer.
- **By target** — none.
- **From a configuration** — answered by the encryption site; a separate key for it would let an
  operator authorise half a decryption, which is not a state a file can be in.
- **Departure** — **C**, for the reason above.

### `file-structure/document-information-dictionary-needs-piece-info`
### `file-structure/document-information-dictionary-holds-only-a-modification-date`
ISO 19005-4 6.1.3 · PDF/A-4, 4f, 4e · today `not-built-yet`

- **Mitigation** — `preserve`: move what `/Info` holds into the XMP packet and delete the
  dictionary, which is the limits document's section 4.2 answer and §14.3.3's own direction of
  travel. Nothing is lost for the keys XMP defines a home for. For a **custom** `/Info` key there is
  no defined home, and then the operator chooses between an extension schema container (ISO 19005-2
  6.6.2.3.2's machinery, and `discard` where the value's type cannot be determined) and dropping it.
- **By target** — **kind difference, and it is the whole answer**: part 2 states no such rule, so a
  PDF/A-2 target carries `/Info` through untouched. This is the cleanest *retarget* answer in the
  catalogue: the document is not failing, the target is asking.
- **From a configuration** — `remedy = "preserve"` plus `unmapped = "extension-schema" | "discard" |
  "stop"`. The operator needs one fact: a reader's File → Properties panel changes where it reads
  from. Nothing else is visible.
- **Departure** — **A**, confirmed: part 2 permits the dictionary. Narrowable by key name — *keep
  `/Title` and `/Author`, drop the rest* — which is the shape an archive with a catalogue index
  actually wants.

### `file-structure/stream-filters-are-standard`
ISO 19005-2 6.1.7.2, ISO 19005-4 6.1.6.2 · all six · today `not-built-yet`

- **Mitigation** — **none, for a stream anything draws**, and the argument is short: re-encoding
  needs the decoded bytes and the decoded bytes need a decoder nobody specifies. There is no tool
  the configuration could name, because the filter is not a published format. The one case with an
  answer is a stream **nothing references**, where ISO 19005-2 6.2.2's exemption for an unreferenced
  named resource means the requirement never applied (`doc/todo/62`); that is a validator fix, not a
  remedy.
- **By target** — none. Both parts name the same standard filter set.
- **From a configuration** — nothing to offer. An operator can supply a decoder as a tool only if
  one exists, and if one exists the filter is not really outside the set — so the honest shape is
  `stop` with the filter's name in the report, which lets a human decide whether their world has a
  decoder.
- **Departure** — **C**. A stream whose bytes no conforming reader can turn back into data is the
  external-stream case with the dependency moved from a disk to an algorithm.

### `file-structure/no-external-stream-data`
ISO 19005-2 6.1.7.1, ISO 19005-4 6.1.6.1 · all six · today `not-this-target`

- **Mitigation** — **`preserve`, and this is a mitigation the RFC's own text rules out too early.**
  Today's sentence says fetching the data is *"a network operation this program does not have and
  `CLAUDE.md` principle 3 will not acquire"* — true of the library, and beside the point once RFC
  0007 section 4's external-tool API exists. A declared tool that resolves a file specification and
  returns bytes puts the fetch in the operator's program, under their trust, bounded by `timeout`
  and `output-limit`, and the converter then embeds what comes back. The result **loses nothing**:
  the stream's data ends up where the clause wanted it all along.
- **By target** — none in kind. Every target wants the same bytes in the same place.
- **From a configuration** — `remedy = "preserve"` with `tool = "resolve-external"`, and RFC 0007
  section 4.1's rule does real work here: the `/F` file specification is document-derived, so it
  goes to the tool **on stdin**, never in `args`. The operator must understand that they are handing
  a document's own string to a fetcher — the sharpest instance of RFC 0007 section 4.5's warning,
  and the place the documentation should say *you are choosing to run this program on untrusted
  input*. `on-failure = "stop"` is the only sane default: a half-resolved document is worse than a
  refused one.
- **Departure** — **C**, unchanged. A conforming-shaped file that still points off its own edge is
  exactly what the format exists to prevent. The mitigation is the answer here, not the departure.

### `file-structure/permissions-dictionary-keys`
### `file-structure/document-signature-states-no-digest`
ISO 19005-2 6.1.12 (both), ISO 19005-4 6.1.11 (the first) · all six / PDF/A-2 · `not-built-yet`

- **Mitigation** — `discard` the keys, and `preserve` what they *meant*, which is the limits
  document's section 3.6 and `doc/rfc/0007` section 5's signature row: write the signer, the signing
  time and whether the signature verified **before** the conversion into the report, into
  `xmpMM:History`, and — where an operator asks — onto an appended page, which is the form a human
  reader can still read in twenty years. Under 4f the original signed file can be attached whole,
  which preserves the cryptography itself in the only way an archive can: as a separate object
  somebody can still verify.
- **By target** — the attach-the-original answer is 4f and 4e only (section 1.2's 4f preserve). The
  page and the history entry are available under all six.
- **From a configuration** — `remedy = "preserve"`, `record = ["report", "history", "page"]`,
  `attach-source = true` (4f/4e only; naming it under another target is RFC 0007 section 4.6's error
  naming both). The operator needs one sentence: the output carries no valid signature and cannot,
  whatever is configured.
- **Departure** — **B**, and weak. Keeping a `/DocMDP` digest reference leaves an inert key that
  asserts a certification no longer true, which is worse than removing it. Narrowable by key name if
  anybody wants it; nobody should.

### `file-structure/bound-names-are-valid-utf8`
ISO 19005-2 6.1.8, ISO 19005-4 6.1.7 · all six · today `the-fence`

- **Mitigation** — **none.** A font resource name is referenced from a content stream this verb
  carries byte for byte, so renaming is fenced; a colourant name *is* the ink's identity, so
  renaming relabels what the separation is; a structure type name is what the role map maps. Every
  route either edits the producer's page or restates what something means. section 4.7 of the limits
  document of the limits document already records this as a refusal rather than a rewrite, and
  nothing in the remedy vocabulary changes it.
- **By target** — none.
- **From a configuration** — nothing, and that is the right answer rather than a gap.
- **Departure** — **B**, and a good one. The rule is about names being interpretable, not about the
  page: a file whose one offence is a font resource name that is not valid UTF-8 renders identically
  in twenty years, because the name is resolved by byte equality through the resources dictionary
  (§7.8.3). Narrowable, and the narrowing is natural — `names = ["font"]` departs for resource names
  while leaving colourant and structure-type names enforced, and those two are where a bad name
  actually costs a reader something.

### `file-structure/inline-image-filters`
ISO 19005-2 6.1.10, ISO 19005-4 6.1.9 · all six · today `the-fence`

- **Mitigation** — **none.** The `/F` entry is inside the content stream, and moving the image out
  to an XObject rewrites the producer's page.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **splits by filter name, and that split is the interesting part.** An inline image
  filtered with `LZWDecode` is fully decodable by any conforming reader and will be in decades: the
  rule excludes LZW for reasons that were about licensing rather than legibility, so departing
  leaves an archive — **B**. An inline image naming `Crypt` is the encryption case — **C**. So the
  entry is one requirement with two departure kinds, separated by a *declarative* predicate:
  `filters = ["LZWDecode"]`. It is the cleanest example in the catalogue of `doc/rfc/0007` section
  4.7.2's narrowing doing real work.

---

## 3. Implementation limits — ten requirements with one argument and one exception

ISO 19005-2 6.1.13 states ten hard limits; ISO 19005-4 states none at all, its 6.1 running to
6.1.12. So all ten bind PDF/A-2b, 2u and 2a and none binds a part 4 target, and everything below is
the same four answers with one requirement breaking the pattern.

**The shared mitigation is `stop`, and the reason is uniform**: every way of meeting one of these
limits is an edit to what the document says — re-nesting `q`/`Q`, rescaling a page, re-encoding a
colour space, renaming an object, dropping objects until the count falls.
`doc/pdf-a-conversion-limits.md` sections 2.5 and 9.2 already say this and rank it as the one class
PDF/A-2 cannot take at all.

**The shared target answer is the strongest in the catalogue**: part 4 accepts the document
unchanged. A large-format drawing, a deeply nested content stream and a forty-ink `DeviceN` space
are all PDF/A-4 documents. For an operator who may choose, there is nothing to configure; for one
whose target is fixed, the limits document's section 9 table is the honest reading and the answer is
that the document does not fit the mandate.

**The shared departure is kind A, confirmed and unusually strong.** This is not a sibling clause
renumbered: ISO removed the subclause. The committee has judged that a conforming file need not obey
these, which is exactly the ground RFC 0007 section 4.7.5 describes. One caution belongs in the
report and not in the configuration: the limits existed because readers of the era allocated fixed
buffers, so a departed PDF/A-2 file may fail in an old reader in a way a departed PDF/A-4 file will
not — the departure is safe against the *standard* and not against every program.

A predicate narrows it naturally and declaratively, one key per limit: `limits =
["page-boundary-sizes", "graphics-state-nesting"]`. An operator with one wide-format drawing per
thousand documents can depart for the page size alone and keep the other nine enforced.

All ten are stated at ISO 19005-2 6.1.13. What each adds beyond the shared answer:

- `implementation-limits/integer-values` — nothing. §7.3.3 states no substitution and Annex C is
  advice, so clamping would be this converter inventing a value.
- `implementation-limits/real-values` — nothing, and for the same reason; the small-magnitude case
  has no stated rounding either.
- `implementation-limits/string-lengths` — **one thing**: where the over-long string is an
  annotation's `/Contents` or a metadata value, `discard` it and attach the text as a file. But
  attaching needs 4f, which is a part 4 target where the limit does not exist — so the remedy is
  only ever a way of keeping the text while *also* retargeting.
- `implementation-limits/name-lengths` — nothing. A name is resolved by byte equality (§7.8.3), so
  renaming means editing every reference, and the references are in content streams.
- `implementation-limits/indirect-object-count` — **a real remedy, below**.
- `implementation-limits/devicen-colourants` — nothing. Re-encoding the space changes what every
  tint means.
- `implementation-limits/page-boundary-sizes` — nothing. Rescaling a page moves every mark on it,
  and tiling one into several pages composes pages nobody produced.
- `implementation-limits/character-identifiers` — nothing. A CID is the font's own numbering.
- `implementation-limits/graphics-state-nesting` — nothing. The nesting *is* the content stream.
- `implementation-limits/values-written-in-content-streams` — nothing, by construction: the subject
  is the content stream.

### The exception: `implementation-limits/indirect-object-count`

**Built in session 957**: the row is in `WRITER_EMITS`, because the serializer answers it.

- **Mitigation** — `discard` **of nothing anybody can see**, which makes it mechanical rather than a
  loss. RFC 0002 RFC 0002 section 10's serializer writes the objects the document reaches; an object
  no reference reaches is not carried. A file over 8 388 607 objects **because it accumulated
  orphans across incremental updates** — a twenty-year-old form revised two hundred times is the
  standing shape — therefore converts with nothing lost, and nobody had written that down.
- **By target** — it only matters at PDF/A-2, since part 4 states no limit. But it is worth doing at
  every target anyway, because it is what the serializer does regardless.
- **From a configuration** — nothing to configure, and that is right: this is section 0.2's *owed,
  not optional*. An operator should never be asked to authorise the removal of objects nothing
  reaches.
- **Departure** — kind A as above, and rarely needed once the orphans are gone.

---

## 4. Graphics

### 4.1 Output intents — eight requirements about the file's own colour destination

ISO 19005-2 6.2.3 and ISO 19005-4 6.2.3. The converter's present answer is section 4.1 of the limits
document of the limits document: append a PDF/A output intent to whatever array it found. That is
right where the array was silent and wrong where it was not, and these eight are the *was not*
cases.

#### `graphics/pdfa-output-intent-states-a-destination-profile`
#### `graphics/one-destination-profile-per-output-intents-array`
#### `graphics/page-output-intents-have-the-same-shape`
all six · 4f/4e-and-4 for the third · the second **built in session 966**, the other two
`not-built-yet` · **corrected in session 966**

- **The three were one refusal sentence and they are three different waits.** That is this
  entry's first correction, and it was invisible while the block read as one mitigation: the
  middle row's answer turns on a question the file itself settles — *are the two entries' profiles
  the same bytes?* — and the other two turn on questions only an operator can. So the middle row
  is built and the sentences the other two now carry are their own.
- **Mitigation** — `preserve`, and it is nearly mechanical: point every entry that states a
  destination profile at **one** profile object, drop a PDF/A entry that names none, and treat a
  page's own array by the same rule. Where two entries name **different** profiles, one has to win
  and the other's identity is lost — that is the only `discard` in the group, and it is small: an
  output intent names the destination a document was prepared for, so the losing entry's *statement*
  can be written into `xmpMM:History` and the report while its object goes.
- **By target** — the page-level array is a part 4 construct, so that requirement binds 4, 4f and 4e
  only. Otherwise no difference in kind.
- **From a configuration** — `remedy = "preserve"` with `winner = "document" | "first" |
  "supplied"`, where `supplied` means the profile `--output-intent-profile` names. The operator
  needs one fact: which press or display the archive's colour is referred to, and they already had
  to know it to choose a profile at all.
- **Departure** — **B**, and thin. Several entries naming several profiles leaves a file whose
  colour destination is ambiguous, which is not a rendering failure but is a legibility one.
  Narrowable by `allow-multiple = true`, and it should not be the answer when `preserve` costs so
  little.
- **What session 966 built, and where it stops.** The clause's own note says where several entries
  arise — a file conforming to ISO 19005 and to PDF/X or PDF/E at once — and such a file carries
  the *same* profile twice as often as not. So the converter decodes both, and where every entry's
  profile is the same bytes under the same stream dictionary it points them all at one object:
  every entry still refers its colours to the profile it already referred them to, the object that
  goes was a copy, and nothing was decided. Where the bytes differ the entries name two
  destinations, the clause admits one, and the loser's statement is discarded — that is the
  `discard` this entry always said it was, it needs the `winner` a configuration supplies, and it
  keeps its refusal with a sentence naming the half that is done. An entry whose
  `DestOutputProfile` is not an indirect reference is refused too: §7.3.8.1 makes every stream an
  indirect object — ISO 32000-1:2008, 7.3.8.1 states the same sentence, so it binds a part 2
  target as well — so that value is not the profile stream the clause requires and there is
  nothing for the others to share.

#### `graphics/destination-profile-class-and-colour-space`
#### `graphics/destination-profile-carries-the-tags-its-class-requires`
#### `graphics/destination-profile-states-a-correct-profile-id`
all six · today `not-built-yet`

- **Mitigation** — `discard` **plus** `preserve`, and the pairing is the answer. The profile the
  document carries is not one ISO 19005 admits, and this converter cannot repair an ICC profile
  without becoming its author. So: replace it with a profile the operator supplies or with shipped
  sRGB (`--output-intent-profile`, the limits document's sections 4.1 and 10.1), **and keep the
  rejected profile** — attached as a file under 4f or 4e (section 1.2), named in the report and in
  `xmpMM:History` under all six. The document's colour destination is then still recoverable by a
  human even where it is no longer asserted by the file.
- **By target** — differs **in availability, not in kind**: the replacement is the same everywhere,
  and only 4f and 4e can keep the original inside the archive.
- **From a configuration** — `remedy = "discard"`, `replacement = "supplied" | "srgb"`,
  `keep-rejected = "attach" | "report"`. What an operator must understand is stated in the limits
  document's section 10.1 and is the whole cost: every device colour in the file is now referred to
  a different destination, so a print-origin archive should supply its own profile rather than take
  sRGB.
- **Departure** — **C** for the class and colour-space rule and for the missing-tags rule, because
  an output intent whose profile is a link or an abstract profile, or which lacks the tags its class
  needs, does not tell a reader what the file's colours mean, which is the one thing the output
  intent exists for. **B** for the wrong or missing profile identifier: the digest is a bookkeeping
  field and a file with a stale one still renders exactly as intended. Predicate: `fields =
  ["profile-id"]`, declarative, and worth having — a stale identifier is common, harmless, and today
  refuses documents whose colour is entirely well specified.

#### `graphics/no-destination-profile-reference-in-a-pdfx-output-intent`
#### `graphics/no-destination-profile-reference`
PDF/A-2 / part 4 targets · today `not-built-yet`

- **Mitigation** — two cases, and only one of them is a loss. Where the same intent **also** embeds
  a `DestOutputProfile`, removing the reference is mechanical and loses nothing — *owed, not
  optional*. Where it does not, the profile is on somebody else's disk and this is the external-data
  case: the mitigation is section 2's `preserve` through a declared resolver tool, exactly as for
  `file-structure/no-external-stream-data`, and failing that the operator supplies a profile.
- **By target** — no difference in kind; part 2 states the rule for a PDF/X intent and part 4 for
  every intent, which is a scope difference rather than a different answer.
- **From a configuration** — `remedy = "preserve"`, `tool = "resolve-external"`, `on-failure =
  "stop"`, or `replacement = "supplied"`. The operator must know that a fetched profile is trusted
  on their say-so and gets a digest in the report.
- **Departure** — **C**, with the external-data reasoning: a reference is a dependency on something
  outside the archive.

### 4.2 Colour spaces, colourants and overprint

#### `graphics/icc-profiles-claim-a-permitted-edition` (PDF/A-2)
#### `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires` (PDF/A-2)
#### `graphics/icc-profiles-carry-the-tags-their-version-requires` (part 4)
#### `graphics/icc-profiles-conform-to-the-base-standard` (part 4)
ISO 19005-2 6.2.4.2, ISO 19005-4 6.2.4.2 · today `not-built-yet`

- **Mitigation** — **`preserve`, on a route the limits document treats as a loss and the base
  standard actually states.** An `ICCBased` space carries an `/Alternate`, and Table 65 says of it:
  *an alternate colour space that shall be used in case the one specified in the stream data is not
  supported*, adding that where the entry is omitted the space used *shall be* `DeviceGray`,
  `DeviceRGB` or `DeviceCMYK` according to `N`. A converter that rejects the profile is precisely a
  processor that does not support it, so falling back to the alternate is an interpretation the
  standard defines rather than a choice this program made — `doc/adr/0948`'s `Stated` class. Two
  honest costs: the colour becomes device-dependent where it had been managed, and under a part 4
  target `DeviceCMYK` then needs 6.2.4.3's condition met. Keep the rejected profile by section 1.2's
  4f preserve where the target allows.
- **By target** — differs in kind between the two parts. Part 2 names a closed list of ICC editions,
  so a *newer* profile fails it and the document is a PDF/A-4 document unchanged (part 4 asks only
  that the profile match the edition its own header names). That makes **retargeting the first
  answer** and the fallback the second.
- **From a configuration** — `remedy = "preserve"`, `via = "alternate"`, `keep-rejected = "attach"`.
  The operator needs one sentence and it is checkable: *colours drawn through this space stop being
  managed and become device values*. For anybody archiving photographic or brand colour that is a
  real decision; for a logo in flat black it is nothing.
- **Departure** — **B**. A file carrying an ICC profile of an edition the part does not list still
  renders correctly wherever that edition is understood, and ICC editions are published standards
  rather than private formats. Narrowable declaratively by edition: `icc-versions = ["4.4"]`.

#### `graphics/no-icc-space-duplicating-the-output-intent-profile`
#### `graphics/separation-alternate-space-does-not-duplicate-a-current-profile`
ISO 19005-4 6.2.4.2 and 6.2.4.4 · PDF/A-4, 4f, 4e · today `the-fence` in its siting ·
**corrected in session 962, re-examined and held in session 971**

- **Mitigation** — **none, and this entry was wrong twice** (ADR 0965; §13.3.1). It read: mechanical
  and lossless, name `DeviceCMYK` where the duplicate profile was, which ISO 19005-4 6.2.4.3
  licenses exactly because the identical profile is already the file's. Two things stop it. The rule
  binds a space that is *used*, so the failure is reported where the content stream selected it — a
  page, with no object — and the colour space array sits in a resource dictionary no finding names;
  siting the rewrite means walking the content streams a second time to decide which space was used,
  which is the validator's reading made again in the converter. And even sited it would not be a
  restatement: §8.6.7 applies non-zero overprint mode only where the current space is `DeviceCMYK`
  or is implicitly converted to it, so the substitution can decide a composite §8.6.5.7 leaves
  open — the very ambiguity 6.2.4.2's NOTE 2 gives as the reason for the prohibition.
- **Session 971 asked whether `SpotColorantEntry` had changed the siting half, and it has not**
  (ADR 0982). That rewrite was the first to reach an object that is a colour space array, so the
  question was fair; the answer is that it reaches one **because its own finding names that array's
  object**, and these findings name the content stream that *used* the space. Run against the
  corpus witness the finding reads `page 1, ICCBased: a colour space operator uses an ICCBased CMYK
  colour space whose profile is the profile in the PDF/A output intent then current` — a page, a
  name of `ICCBased`, and no object at all. The array is still in a resource dictionary nothing
  points at.
- **The value half is stronger than it was written, not weaker.** §8.6.5.7 says of the implicit
  conversion outright that *"[t]he conditions under which such implicit conversion is done cannot be
  specified in PDF"* and that it *"is completely hidden by the PDF processor and plays no part in
  the interpretation of PDF colour spaces"*. So whether an `ICCBased` CMYK space behaves as
  `DeviceCMYK` — and therefore whether non-zero overprint mode applies to it — is the processor's
  decision by the base standard's own words. Writing `/DeviceCMYK` into the file takes that decision
  away from the processor and settles it. That is not a restatement under any reading.
- **By target** — part 2 states neither rule, so a PDF/A-2 target never asks (kind A, confirmed).
- **From a configuration** — nothing, by design.
- **Departure** — **A**, and pointless: the remedy loses nothing.

#### `graphics/spot-colourants-appear-in-the-colorants-dictionary`
ISO 19005-2 6.2.4.4, ISO 19005-4 6.2.4.4 · all six · **partly built in session 966**, the rest
`not-built-yet` · **corrected in session 962 and again in session 966**

- **Mitigation** — synthesise each missing entry from the space's own alternate space and tint
  transform, which is the limits document's section 4.5 Default and invents nothing. This entry then
  said the open question — how a single colourant's transform is derived from an *N*-input one — was
  "arithmetic rather than policy", and **it is not** (ADR 0965): §7.10 gives a PDF function no way
  to call another, so the §8.6.6.4 `Separation` this would write needs a one-input function the
  general route can only obtain by *sampling* the producer's, which is an approximation written into
  an archive as though it were their definition. One shape could be exact and is the thing to build
  first: a §7.10.2 sampled transform already states its values on a grid, so the samples along one
  axis are the producer's own numbers.
- **Both readings above looked only at the `DeviceN` space, and 6.2.4.4 has a second sentence**
  (ADR 0973). The subclause requires every `Separation` array in one file naming a given colourant
  — expressly including the arrays written inside a `Colorants` dictionary — to state the same
  alternate space and the same tint transform, compared as PDF objects rather than by what using
  them computes. So where the file already states a `Separation` for the colourant, the entry that
  requirement admits is **that array and no other**: there is no derivation to make, nothing to
  sample, and the producer's own definition of the ink is what goes in. That is what session 966
  built, and it is exact in a way the sampled route never could be — the sampled route stays owed
  and stays the harder half, for a file whose spot ink is defined nowhere but inside the `DeviceN`
  that uses it.
- **The lesson is the catalogue's rather than this row's.** Two sessions read this requirement's
  mitigation off the one sentence the row's *title* is about, and the answer was in the next
  paragraph of the same subclause. A clause is not read until its neighbours are.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **B**, and unnecessary. A `/Colorants` dictionary is what lets a reader render one
  ink alone; a file without it renders the same in composite.

#### `graphics/separations-of-one-name-agree`
ISO 19005-2 6.2.4.4, ISO 19005-4 6.2.4.4 · all six · today `not-built-yet`

- **Mitigation** — `supply` (section 0.2), and this is the model case. Two `Separation` arrays name
  the same ink and define it differently; nothing in the file says which its producer meant, and the
  two may genuinely render differently. The converter cannot choose and an operator often can — a
  house ink book says what `PANTONE 293 C` is, once, for every document in the archive.
- **By target** — none. Both parts state it identically.
- **From a configuration** — `remedy = "supply"` with either `winner = "first" | "most-used"` or an
  explicit table `colourants = { "PANTONE 293 C" = "…" }` naming the definition that wins. The
  operator must understand that marks drawn through the losing definition change colour, and the
  report should show both renderings, which the limits document's section 4.5 already asks for.
- **Departure** — **B**. A file with two definitions of one ink is ambiguous rather than
  unrenderable; a reader takes each space as it finds it. Narrowable by colourant name, which is the
  right shape: depart for `All` and `None`, decide the real inks.

#### `graphics/no-overprint-mode-one-under-icc-cmyk`
ISO 19005-2 6.2.4.2, ISO 19005-4 6.2.4.2 · all six · today `not-built-yet`

- **Mitigation** — `discard`: set overprint mode 0 in the graphics state parameter dictionary, which
  is the limits document's section 4.5 Ask. The key is in an object rather than on a page, so no
  fence stands in the way.
- **By target** — none.
- **From a configuration** — `remedy = "discard"`. The cost is statable in one sentence and an
  operator of print-origin material will understand it immediately: *a zero CMYK component stops
  leaving the backdrop alone and starts painting it*, so overlapping marks composite differently.
  For office documents it changes nothing, and that asymmetry is what makes it a good configuration
  question rather than a per-document one.
- **Departure** — **B**, and a strong one for a print archive. Overprint mode 1 is what the file's
  producer meant; the rule exists because the result depends on the device. An archive of
  press-ready artwork may well prefer the producer's intent to the target's determinism, and that is
  a decision only they can make. Predicate: none natural beyond the site itself.

### 4.3 Transfer functions and halftones — seven requirements, two very different subjects

ISO 19005-2 6.2.5 and ISO 19005-4 6.2.5 forbid them in one subclause, and `CLAUDE.md` records why
they must not be answered together: **§10.5's transfer functions decide what a screen shows** — an
inverting one is a photographic negative — while **§10.6's halftones are inapplicable on the
standard's own condition**, describing how a marking device renders continuous tone. So removing a
transfer function can change the page and removing a halftone cannot.

#### `graphics/no-transfer-function-in-a-graphics-state`
#### `graphics/second-transfer-function-is-default`
#### `graphics/halftone-transfer-function-only-where-required`
all six · today `not-built-yet`

- **Mitigation** — `discard`, with the limits document's section 4.8 evidence attached: render the
  page both ways, show whether anything changed, and remove the function only with that shown. Add a
  `preserve` the limits document does not have: **the function itself is data** — a sampled function
  stream or a set of coefficients — so it can be written into `xmpMM:History` and, at 4f or 4e,
  attached as a file. A later reader can then reconstruct what the producer specified even though
  the archive no longer applies it.
- **By target** — no difference in kind; the keeping-a-copy half is the 4f preserve again.
- **From a configuration** — `remedy = "discard"`, `evidence = "always" | "when-it-changes"`, `keep
  = "history" | "attach"`. The operator needs to know that this is the one entry in the halftone
  subclause that can change what a reader sees.
- **Departure** — **B**, and narrowable in both of section 0.2's senses. Declaratively: *depart only
  for `/TR2` with a named function*. Computationally: *depart only where the function is the
  identity*, which is free of cost and needs the file examined. The second is much the better rule
  and the format should be able to say which kind it is.

#### `graphics/halftone-type-is-one-or-five`
#### `graphics/no-halftone-name`
#### `graphics/no-halftone-phase-in-a-graphics-state` (part 2, `/HTP`)
#### `graphics/no-halftone-origin-in-a-graphics-state` (part 4, `/HTO`)
today `not-built-yet`

- **Mitigation** — `discard`: remove the key or the halftone. Nothing this renderer draws changes,
  which is what makes the loss narrow and nameable — what changes is what a **press** does with the
  file, and an archived print master is exactly the document kept for that. So pair it with the same
  `preserve`: record the halftone dictionary in `xmpMM:History`, attach it at 4f or 4e.
- **By target** — the phase and origin rows are the same rule in two editions' spellings and are
  **not** a relaxation of each other (section 1.1). Otherwise identical across the six.
- **From a configuration** — `remedy = "discard"`. One sentence of cost, and it is unusually clean:
  *nothing on screen changes; a press will screen the file its own way instead of the producer's.*
  An office archive can answer that once and never think about it again; a prepress archive will
  want `stop`.
- **Departure** — **B**. A halftone in a graphics state is inert for a display reader, so a departed
  file renders identically for the case PDF/A cares most about. Narrowable per key, and the useful
  narrowing is `keys = ["HalftoneName"]` — a name is a pointer into a device's own table and is the
  most obviously inert of the four.

### 4.4 Rendering intent — one requirement in two places, with two different answers

§8.6.5.8 settles what an unrecognised name means: "If a PDF processor does not recognise the
specified name, it shall use the RelativeColorimetric intent by default."

#### `graphics/rendering-intent-entries-name-one-of-four`
ISO 19005-2 6.2.6 · PDF/A-2b, 2u, 2a · **built in session 957** (`Stated`)

- **Mitigation** — restate the entry as `RelativeColorimetric`, which writes down the interpretation
  the standard defines rather than a choice this converter made — `doc/adr/0948`'s `Stated` class.
  **Owed, not optional.**
- **By target** — part 4 does not state the rule (kind A candidate, and confirmed: no part 4 clause
  binds `/RI` or `/Intent` to the four names).
- **From a configuration** — nothing, by design. Offering an operator a choice here would be
  offering them a choice about a sentence the standard has already written.
- **Departure** — **A**, and unnecessary.

#### `graphics/rendering-intent-operator-names-one-of-four`
ISO 19005-2 6.2.6 · PDF/A-2b, 2u, 2a · today `the-fence`

- **Mitigation** — **none.** The operand is inside a content stream, and ADR 0816's fence stands.
- **By target** — part 4 does not state it, so the document is a PDF/A-4 document unchanged.
- **From a configuration** — nothing.
- **Departure** — **B, and the strongest in the catalogue.** Because §8.6.5.8 tells every conforming
  processor what to do with the name, a departed file renders **identically** to one this converter
  would have produced if it could edit the stream. The departure costs nothing at all except the
  claim, which is precisely the case `doc/questions/Q59` asks about. Not narrowable: the operand is
  a name in a stream and there is no key to predicate on.

### 4.5 Images

#### `graphics/jpeg2000-colour-specification-method`
#### `graphics/jpeg2000-one-best-colour-space-specification`
ISO 19005-2 6.2.8.3, ISO 19005-4 6.2.7.3 · all six · **built in session 971** (`discard`, an
authorised loss); the shapes below keep their refusal · **corrected in session 962 and again in
session 971**

- **Mitigation** — **`discard`, and this entry was wrong about there being none.** Both earlier
  readings — the original, and session 962's correction of it — asked what value could be
  *written* into a `colr` box, and both were right that every one of them is a choice: a `METH`
  outside the three the part admits describes this image's colour in a way the part does not read,
  so writing one of the three states a colour space the box did not, and marking one specification
  best available where the file marks none ranks two of the producer's statements on evidence the
  file lacks. **What neither asked is whether a box can go.** The sentence immediately after the
  method's, in both parts, is the answer: a conforming processor shall use only the selected colour
  space and shall ignore all the other colour space specifications. So the boxes this removes are
  the ones the target's own subclause directs a processor to ignore, and no value of this
  converter's is written anywhere.
- **Which box is selected is the standards' own, in two sentences.** Where exactly one
  specification carries an `APPROX` of `0x01`, both parts' NOTE 2 makes it the best available and
  §7.4.9 sends a processor to the same box — *"a PDF processor should attempt to use the one with
  the highest precedence and best approximation value"*. Where **every** specification states an
  `APPROX` of zero, ISO/IEC 15444-1:2000 I.5.3.3 reserves that field, requires it to be zero, has a
  reader ignore its value, and says a conforming JP2 reader ignores every colour specification box
  after the first — so the first box is the used one by the core part's own rule, and a file
  written to that edition is exactly the file that fails ISO 19005's *exactly one marked best* for
  no other reason. Any other shape — two marked best, or a mixture — is refused, because that is
  the ranking session 962 refused and it stays refused.
- **What it costs, and why it is an Ask rather than owed.** §7.4.9 makes the removed boxes a
  fallback chain: *"[i]f the colour space is given by an unsupported ICC profile, the next lower
  colour space, in terms of precedence and approximation value, shall be used."* A processor that
  can use the box that stays sees no difference at all — the corpus witness renders byte for byte
  identically — and one that cannot now falls back to a device space rather than to the producer's
  second choice. That is small, real and nameable, so it is a loss an operator authorises.
- **Two shapes keep the refusal and their sentences are now their own**: a file whose *only*
  specification states a forbidden method, where there is nothing to remove and nothing for a
  processor to fall back to; and the second row's second sentence, which requires the *selected*
  specification's ICC profile to conform to the base standard — the profile-replacement case, not
  a box removal.
- **By target** — none.
- **From a configuration** — `remedy = "discard"`, and the operator needs one sentence: a reader
  that cannot use this image's best colour specification no longer has the producer's others to
  try. `--authorise jpeg2000-colour-fallback` is that key today.
- **Departure** — **B**, and pointless for the shapes the rewrite reaches.

#### `graphics/jpeg2000-bit-depth`
#### `graphics/jpeg2000-channel-count`
#### `graphics/jpeg2000-no-ciejab-colour-space`
ISO 19005-2 6.2.8.3, ISO 19005-4 6.2.7.3 · all six · today `not-built-yet`

- **Mitigation** — `preserve` by **transcoding to a permitted codec**, which is `doc/rfc/0007`
  section 5's last table row made concrete: decode the samples and re-encode them with
  `FlateDecode`, losing nothing visible. Two costs that have to be said out loud, and the second is
  the serious one: the file grows, often by a great deal; and **this tree's own JPEG 2000 decoder's
  output becomes the archive permanently**, so a bug in it is baked in rather than re-decodable
  later. One cheaper case exists for the channel count — a `cdef` box declaring the second channel
  as opacity leaves one colour channel without a sample being touched.
- **By target** — none in kind.
- **From a configuration** — `remedy = "preserve"`, `codec = "flate"`, `max-growth = "4x"` so a
  queue does not silently quadruple. The operator needs both costs; the decoder-output one is not
  obvious and belongs in the profile comment rather than only in the report.
- **Departure** — **B**. A codestream with five channels or a 40-bit depth is still ISO 15444-1 and
  still decodable; what the clause protects is the narrower set a PDF reader is obliged to handle.
  Narrowable declaratively per field, `fields = ["bit-depth"]`.

#### `graphics/inline-image-interpolation-is-off`
ISO 19005-2 6.2.8.1, ISO 19005-4 6.2.7.1 · all six · today `the-fence`

- **Mitigation** — **none.** The `/I` entry is inside the content stream.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **B**, and close to free: interpolation is a rendering hint about smoothing a
  low-resolution image, so a departed file differs from a conforming one only in how smoothly one
  image is scaled. Not narrowable.

#### `graphics/no-reference-xobjects`
ISO 19005-2 6.2.9.2, ISO 19005-4 6.2.8.2 · all six · today `not-built-yet`

- **Mitigation** — `preserve`, and the limits document's section 2.3 already found the good one:
  §8.10.4 makes a reference XObject's containing form a **proxy** — what a processor draws when the
  referenced content is not available. An archive is exactly the case where it will not be, so
  dropping the `/Ref` entry keeps what the file was always going to show. It is an Ask rather than
  mechanical because a reader that *could* have reached the imported content now sees the
  placeholder instead.
- **By target** — none in kind. A second, better answer exists where the referenced file is to hand:
  resolve it with a declared tool (as in section 2's external-stream entry) and import the pages,
  which loses nothing — but that is `derive` territory and needs the owner's ruling on composing
  pages.
- **From a configuration** — `remedy = "preserve"`, and one sentence of cost: *the page shows the
  proxy the producer drew, not the document it pointed at*.
- **Departure** — **C**. A reference XObject is a dependency on another file, which is the external
  data case wearing a form XObject's clothes.

### 4.6 Blend modes — one rule, two shapes, two permanent halves

#### `graphics/graphics-state-blend-modes-are-defined`
#### `graphics/annotation-blend-modes-are-defined`
ISO 19005-2 6.2.10, ISO 19005-4 6.2.9 · all six / part 4 · **array half built in session 957**
(`Stated`); the bare-name half stays `not-built-yet`

- **Mitigation** — split by shape, as ADR 0955 found. For an **array** of names, §11.6.3's own entry
  says a reader takes the first mode it recognises or `Normal` if it recognises none, so reducing
  the array to that name is `Stated` and **owed, not optional**. For a **bare name** the standard
  does not define, there is **no mitigation**: writing one in its place chooses how these marks
  composite with what is under them, which `A48` forbids, and removing the entry does not state
  `Normal` either, because a graphics state parameter dictionary sets only what it names.
- **By target** — the annotation half is part 4 only (kind A candidate; confirmed, part 2 states no
  rule about an annotation's `/BM`).
- **From a configuration** — nothing for the array half. For the bare-name half a configuration
  *could* offer `remedy = "supply"` with `blend-mode = "Normal"` — the operator asserting what the
  producer's unknown mode should become. It is honest only if the report and `xmpMM:History` say the
  value came from the configuration, and it is the one place in this catalogue where `supply`
  changes **marks on the page** rather than metadata about them. That makes it the `supply` case
  that most needs the owner's ruling.
- **Departure** — **B** for both halves, and the reasoning differs: for an array, every reader
  already ignores what it does not recognise, so departing changes nothing; for a bare name, a
  reader that does not know the mode falls back on its own behaviour, which is undefined — so the
  file's appearance is not determined, and that is uncomfortably close to **C**. The honest entry is
  *B for the array, C for the bare name*, narrowable declaratively by `shape = ["array"]`.

### 4.7 Content streams and resources

#### `graphics/only-operators-the-base-standard-defines`
ISO 19005-2 6.2.2, ISO 19005-4 6.2.2 · all six · today `the-fence`

- **Mitigation** — **none.** Removing an operator edits the producer's page. `A50`'s one exception —
  a closed list of spellings the standard documents as equivalent — does not reach an operator the
  standard does not define at all.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **B**, and unusually well grounded when the operator sits inside `BX`/`EX`.
  §7.8.2: "They bracket a compatibility section, a portion of a content stream within which
  unrecognised operators shall be ignored without error." A departed file containing only such
  operators is therefore drawn identically by every conforming reader, now and in twenty years.
  Outside the brackets the same departure leaves a file whose rendering is an error case, and that
  is **C**. Predicate: `inside-compatibility-section = true` — computed rather than declarative, but
  cheaply, and it separates a safe departure from an unsafe one along the standard's own line.

#### `graphics/content-streams-have-an-explicit-resources-dictionary`
ISO 19005-2 6.2.2, ISO 19005-4 6.2.2 · all six · **page half built in session 957**
(`Mechanical`); the form `XObject` half is now `the-fence`

- **Mitigation** — two cases. For a **page**, the dictionary is usually inherited through the page
  tree (§7.7.3.4) and copying it down changes nothing a reader resolves: mechanical, **owed, not
  optional**. For a **form XObject or a Type 3 glyph procedure** with none, what it draws with
  depends on the stream that invoked it, and one dictionary cannot answer for every invocation —
  **no mitigation**, unless every invocation happens to resolve the same names to the same objects,
  which is a computed predicate a rewrite could check.
- **By target** — none.
- **From a configuration** — nothing for the page case. For the second case an operator could
  authorise *"synthesise from the union of the invoking streams' resources where they agree"*, which
  is a rewrite with a proof rather than a policy — so it belongs in code with a report line, not in
  a configuration.
- **Departure** — **B** for the inherited-page case, which is a bookkeeping rule; **C** for the form
  XObject case, because a stream whose names nothing resolves does not determine its own appearance.

#### `graphics/named-resources-are-defined`
ISO 19005-2 6.2.2, ISO 19005-4 6.2.2 · all six · today `the-fence`

- **Mitigation** — **none, and this is one of the clearest.** Supplying the resource invents an
  object the producer never wrote; taking the reference off the page edits the content stream.
  §7.8.3 makes the resources dictionary what a name is resolved through, so nothing in the file says
  what the missing one was.
- **By target** — none.
- **From a configuration** — nothing. `supply` does not reach it either: an operator naming a
  replacement font or colour space for a name the producer left dangling is guessing exactly as this
  program would be, with less information.
- **Departure** — **C**. This is the format's central promise — the file determines its own
  appearance — failing in the smallest possible way.

---

## 5. Fonts

**The group's headline is `doc/rfc/0007` section 5's own sentence and it survives the review**: for
a face nothing can substitute, `stop` is the only honest answer, because a wrong glyph is not a
remedy. What the review adds is that the sentence covers **fewer** of these twenty requirements than
it looks, and two of them have mitigations nobody had written down.

#### `fonts/cmap-embedded-or-predefined`
#### `fonts/cmap-uses-only-predefined-cmaps`
ISO 19005-2 6.2.11.3.3, ISO 19005-4 6.2.10.3.3 · all six · today `the-fence`

- **Mitigation** — **`preserve`, and it is new.** The refusal's reasoning is that supplying a CMap
  the file neither embeds nor takes from the predefined set means writing the encoding its producer
  did not. That is true of an *invented* CMap and false of a **published** one: this tree ships all
  239 Adobe predefined CMaps in `data/cmaps/` under BSD-3-Clause (section 10 of the limits document
  of the limits document), so where the file names one of those and the base standard's own list
  does not include it, the CMap can be **embedded from the shipped set**. Nothing is invented — the
  mapping is Adobe's, the same bytes the producer's reader would have used — and the file becomes
  self-contained, which is what the clause is for. The same answer serves a CMap that builds on
  another through `usecmap`, with one check the rewrite owes: what it builds on must itself end at a
  CMap the base standard predefines, or the chain has to be embedded too.
- **By target** — none in kind; both parts state the same rule.
- **From a configuration** — `remedy = "preserve"`, `source = "shipped-cmaps"`, `on-missing =
  "stop"`. The operator needs one fact: a name the shipped set does not hold still refuses, and no
  configuration can conjure the mapping.
- **Departure** — **C**. A composite font whose CMap is neither in the file nor in a published set
  does not determine what its codes select, which is the font half of the format's central promise.

#### `fonts/cid-to-gid-map-present`
ISO 19005-2 6.2.11.3.2, ISO 19005-4 6.2.10.3.2 · all six · **part 2 half built in session 957**
(`Stated`); part 4 stays `not-built-yet`

- **Mitigation** — **differs by target in kind, and the difference is a reading nobody had made.**
  ADR 0955 records that §9.7.4.2's Table 121 makes the entry required and states **no default**, so
  writing `Identity` would assert a mapping ISO 32000-2 does not supply. But ISO 19005-2 5.1 makes a
  PDF/A-2 file one that adheres to **ISO 32000-1**, whose own table gives `Identity` as the entry's
  default — so at 2b, 2u and 2a, writing `Identity` restates the base document that target actually
  names, which is `Stated` rather than invention. At a part 4 target the base document is ISO
  32000-2 and the same write is an invention. One requirement, two answers, decided by which edition
  the target is built on.
- **By target** — as above: `Stated` rewrite at 2b/2u/2a, no mitigation at 4/4f/4e except deriving
  the map from the embedded program, which is a reading of the font this converter does not make.
- **From a configuration** — nothing at the part 2 targets, where it is owed code. At a part 4
  target, `remedy = "supply"` with `cid-to-gid = "identity"` is expressible and is the operator
  asserting that their producer's fonts are identity-ordered — which for a subset-embedded TrueType
  CIDFont is nearly always true and is exactly the kind of fact a fleet operator knows.
- **Departure** — **B** at part 4, narrowed by the same predicate; **A**-shaped at part 2 in the
  sense that the other part's base document answers it, though the better route there is the
  rewrite.

#### `fonts/cid-system-info-agrees-with-the-cmap`
ISO 19005-2 6.2.11.3.1, ISO 19005-4 6.2.10.3.1 · all six · today `the-fence`

- **Mitigation** — **none.** The CIDFont's registry, ordering and supplement say which collection
  its CIDs are numbered in, and the CMap's say the same of the codes it produces; making them agree
  relabels every CID in the font, and nothing in the file says which of the two its producer meant.
- **By target** — none.
- **From a configuration** — `supply` is conceivable — an operator naming the collection they know
  their producer used — but it is the one `supply` that cannot be checked against anything, and a
  wrong answer silently changes every glyph. The catalogue's recommendation is to withhold it.
- **Departure** — **B**, narrowed by a computed predicate that makes it safe: *depart only where the
  CIDFont's program is embedded*. Then the glyphs are in the file and the disagreement is
  bookkeeping; where the font is not embedded, the collection is how a reader finds a substitute,
  and departing is **C**.

#### `fonts/embedded-programs-define-every-glyph-shown`
#### `fonts/no-notdef-glyph-shown`
ISO 19005-2 6.2.11.4.1 and 6.2.11.8, ISO 19005-4 6.2.10.4.1 and 6.2.10.9 · all six · `the-fence`

- **Mitigation** — **none**, and this is the RFC's font sentence in its proper place. The file
  carries the program, so the mapping is the producer's and fixed; the only routes are taking the
  code off the page (a mark removed) or drawing a glyph for it (a mark invented). One escape is not
  a remedy but is worth repeating in the report: where the font is **not** embedded, the limits
  document's section 4.9 substitution applies instead, and `--font` supplying the intended face
  moves the document into that case.
- **By target** — none.
- **From a configuration** — nothing beyond `--font`, which is `supply` in its oldest form.
- **Departure** — **B**, and the reasoning is worth the space because it is easy to get wrong. This
  is *not* the font-embedding requirement RFC 0007 section 4.7.5 calls kind C. The program **is**
  embedded; it defines `.notdef`; a departed file therefore renders deterministically in twenty
  years, showing the blank or the box the producer's own font supplies. What the clause protects is
  the quality of the producer's mapping, not the file's self-sufficiency. Narrowable by a computed
  predicate — *depart where fewer than N codes reach `.notdef`* — which is what separates a stray
  character from a document whose text is mostly missing.

#### `fonts/charset-lists-every-glyph-in-the-program`
#### `fonts/cidset-lists-every-cid-in-the-program`
ISO 19005-2 6.2.11.4.2 · PDF/A-2b, 2u, 2a · **built in session 957** (`Mechanical`, by removal)

- **Mitigation** — two lossless routes and the choice between them is the only open part.
  **Recompute** the set from the embedded program, which `pdf_font` already reads; or **remove** the
  entry, which the base standard makes optional and which the clause therefore stops judging. Both
  are mechanical and neither loses anything, because the set is derivable from the program either
  way. **Owed, not optional** in substance.
- **By target** — part 4 states no such rule and ISO 32000-2's Table 122 deprecates `/CIDSet`
  outright, so a part 4 target wants the entry *gone* rather than corrected. Confirmed **A**.
- **From a configuration** — at most `route = "recompute" | "remove"`, and only because an archive
  may prefer the entry present for its own indexing. Neither costs anything.
- **Departure** — **A**, confirmed, and pointless given two lossless remedies.

#### `fonts/vertical-metrics-agree-with-the-program`
ISO 19005-4 6.2.10.5 · PDF/A-4, 4f, 4e · **built in session 976** (`Mechanical`) · corrected in
sessions 962, 966 and 976

- **Mitigation** — the limits document's section 4.9 restatement in the other writing direction,
  **and this entry had the direction backwards** (ADR 0965). It proposed restating `/DW2` and `/W2`
  from the embedded program, "on the same argument that already justifies the horizontal case" — and
  the horizontal case restates the **program**, for the reason that makes the other direction
  unsafe: §9.2.4 makes the font dictionary's numbers what positions a glyph without looking inside
  the program, and §9.7.4.3 gives `/DW2` and `/W2` that role going down the page, so restating them
  moves every glyph on a vertical line. Restating the program's `vmtx` moves nothing.

  **And §9.9.1 says so outright, which is stronger than the inference above and was not found when
  this entry was corrected:** "The "vhea" and "vmtx" tables that specify vertical metrics shall
  never be used by a PDF processor. The only way to specify vertical metrics in PDF shall be by
  means of the DW2 and W2 entries in a CIDFont dictionary." So the direction is not a judgement
  about which side is authoritative — the standard forbids a processor from reading the program's
  side at all. Rewriting `vmtx` to agree with `/DW2` and `/W2` changes nothing any conforming
  processor does, by the clause's own words, and rewriting the dictionary to agree with `vmtx`
  would move marks on the authority of a table the standard says shall never be used.

  **It also says what this PDF/A requirement is *for*, which the entry never asked.** If no
  processor may read `vmtx`, the requirement cannot be about rendering: it is about the archive
  being internally consistent for a reader that is not this one — the same motive as `/CIDSet` and
  `/CharSet`, and the reason those two are deprecated rather than tightened. Worth carrying,
  because it predicts which way *every* agree-with-the-program row should be rewritten.

  **And the sentence *before* §9.9.1's answers a question this entry never asked either.** The
  paragraph above it lists the TrueType tables that "shall always be present if present in the
  original TrueType font program" — `head`, `hhea`, `loca`, `maxp`, `cvt `, `prep`, `glyf`,
  `hmtx`, `fpgm` — and `vhea` and `vmtx` are **not** among them. So the standard declines to
  require their preservation in the sentence before it forbids their use, which makes **removing
  them as lossless as restating them**: this row has the same two routes `/CIDSet` and `/CharSet`
  have. The route taken is *restate*, and for the opposite of the usual reason — removal is the
  **harder** build, a directory record cannot be dropped without rebuilding the directory, while
  an advance already has a field to be written in.

  **Built in session 976** (ADR 0988), and the entry was wrong a third time on the way: it said
  the row waits on `pdf_font::restate`, "which rewrites an sfnt's `hmtx` and nothing vertical".
  That is true of the module's subject and false of its code — `restate.rs` owns the units and the
  tolerance, and the byte surgery is `sfnt.rs`'s, which the round did not hold. What came of the
  constraint is a *better* rewrite: `with_vertical_advances` overwrites the advance field where it
  already is, changing no offset, no length and no outline, and adjusts the two checksums the edit
  invalidates by an exactly computed delta rather than recomputing them — so a program whose
  producer's checksums were already wrong keeps precisely the error it arrived with.
  **What is refused by name** is a glyph in the `vmtx` tail past `numOfLongVerMetrics`, whose
  advance the table states only by inheritance: giving it one of its own means lengthening the
  table and restating every other glyph in the tail with it, and none of those was asked for.
- **By target** — part 2 states no vertical rule (confirmed **A**), so a PDF/A-2 target converts
  unchanged — and that is enforced rather than assumed: the rewrite is its own `Rewrite`, so
  `wanted_by` can ask whether a *failed* requirement called for it.
- **From a configuration** — nothing.
- **Departure** — **A**, and unnecessary.

#### `fonts/type3-glyph-procedures-state-their-width`
ISO 19005-4 6.2.10.5 · PDF/A-4, 4f, 4e · today `the-fence`

- **Mitigation** — **none.** The `d0`/`d1` operands are inside the glyph procedure, which is a
  content stream; and §9.2.4 makes the font dictionary's widths what positions every glyph on the
  line, so restating those moves marks.
- **By target** — part 2 states no such rule (confirmed **A**).
- **From a configuration** — nothing.
- **Departure** — **B**, and safe: §9.2.4 makes the dictionary's widths authoritative for
  positioning, so a departed file lays out exactly as it does today, disagreement and all. Not
  narrowable in any useful declarative way.

#### `fonts/non-symbolic-truetype-uses-a-standard-encoding`
#### `fonts/symbolic-truetype-states-no-encoding`
ISO 19005-2 6.2.11.6, ISO 19005-4 6.2.10.6 · all six · **built in session 962**

- **Mitigation** — the failing entry is in the font dictionary rather than the program, so no fence
  stands in the way: write `WinAnsiEncoding` or `MacRomanEncoding`, or take the symbolic font's
  `/Encoding` away. What made it unbuilt rather than easy is that §9.6.5.4 makes the encoding
  decide which `cmap` subtable a code is looked up through, so the rewrite owes a **proof, per font
  and per code used**, that the program's own tables already agree. **That proof is now made, by
  loading the font twice** — once as the file states it and once as it would be written, comparing
  the glyph every shown code reaches (ADR 0965). A font whose program the file does not carry, or
  whose shown strings outran the survey's budget, is refused instead. The *per code used* half of
  this entry turned out to be load-bearing rather than a convenience: over all 256 codes,
  `StandardEncoding` and `WinAnsiEncoding` reach different glyphs at 80 of them in a full Latin
  face, so a proof over the whole domain would never once have fired.
- **By target** — none.
- **From a configuration** — nothing, deliberately. An operator authorising *"write WinAnsiEncoding
  anyway"* would be authorising moved glyphs they cannot see.
- **Departure** — **B**. The program plus the dictionary still determine what is drawn; the rule
  exists so that the standard's own lookup procedure reaches the glyph without a reader's heuristic.
  Narrowable by a computed predicate: *depart where every code used resolves to the same glyph under
  the file's encoding as under the standard's procedure* — which is the same proof the rewrite
  needs, used to justify leaving the file alone instead of changing it.

#### `fonts/non-symbolic-truetype-program-maps-every-code`
#### `fonts/non-symbolic-truetype-differences-are-listed-names`
#### `fonts/non-symbolic-truetype-differences-need-the-unicode-cmap`
#### `fonts/symbolic-truetype-program-has-a-usable-cmap`
ISO 19005-2 6.2.11.6, ISO 19005-4 6.2.10.6 · all six · today `the-fence`

- **Mitigation** — **none.** Adding a `cmap` subtable invents outlines' addressing the producer
  never shipped; rewriting the `/Differences` array moves the marks on the page. Where the file
  embeds no program at all, section 4.9 of the limits document builds one instead and `--font`
  supplies the real face — again an escape rather than a remedy.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **B** for the three narrower rules (a glyph is still reached, by another route),
  and the same computed predicate as above makes them safe. The fourth,
  `symbolic-truetype-program-has-a-usable-cmap`, is the one where the lookup may reach nothing at
  all, and that is **C**.

#### `fonts/truetype-codes-reach-glyphs-by-the-standard-route`
ISO 19005-2 6.2.11.6, ISO 19005-4 6.2.10.6 · all six · today `the-fence`

- **Mitigation** — **none**, for the reasons above.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **C**, and it is the requirement that states the promise directly: a file whose
  codes reach glyphs only through a mapping the reader invents is a file whose appearance depends on
  the reader. Departing from it is departing from the format.

#### `fonts/embedded-cmap-states-its-own-write-mode`
ISO 19005-2 6.2.11.3.3, ISO 19005-4 6.2.10.3.3 · all six · today `not-built-yet`

- **Mitigation** — `supply` (section 0.2). The stream's `/WMode` and the CMap program's own write
  mode disagree, and between them they decide whether the text runs down the page or across it. The
  converter cannot choose; a person looking at one rendered page can, in a second.
- **By target** — none.
- **From a configuration** — `remedy = "supply"` with `write-mode = "stream" | "program"`. An
  archive of vertical Japanese text can answer this once for a whole producer's output, which is
  exactly the owner's question having a good answer.
- **Departure** — **B**, and weak: departing leaves both statements in the file and every reader
  choosing for itself, which is the ambiguity the clause removes. Prefer `supply`.

#### `fonts/actual-text-covers-private-use-characters`
ISO 19005-2 6.2.11.7.3 · PDF/A-2a · today `the-fence`

- **Mitigation** — **none.** An `/ActualText` entry covers a span of a content stream, so writing
  one means deciding where inside the producer's page the span begins and ends, and what a Private
  Use character was for. That is the evidence Level A exists to require.
- **By target** — **the answer is a retarget, not a departure**: 2b and 2u do not state this rule,
  and neither does part 4. A document with Private Use characters and no `/ActualText` is a PDF/A-2u
  document today.
- **From a configuration** — `supply` is imaginable — a mapping table from the private code points
  to text — and it is the one case where an operator genuinely may know, because a private-use font
  belongs to somebody. It needs the span located, which is content-stream work, so it is not
  available at this fence and should not be promised.
- **Departure** — **C at Level A**, because the level's whole subject is the text the glyphs stand
  for. Confirmed **A** as a census candidate, and that is the honest route: take 2u.

#### `fonts/actual-text-states-no-private-use`
ISO 19005-4 6.2.10.8 · PDF/A-4, 4f, 4e · today `the-fence`

- **Mitigation** — `discard` exists and is bad: removing the entry takes away the only statement the
  file makes about what those glyphs spell. Changing its value is `A48`'s forbidden half. So the
  honest answer is **none worth taking**, with `discard` available to an operator who values the
  claim over the statement.
- **By target** — part 2 states no such rule (confirmed **A**).
- **From a configuration** — `remedy = "discard"`, with a cost sentence an operator will understand:
  *text extraction of that span stops working*.
- **Departure** — **B**. A Private Use character inside `/ActualText` is a worse answer than a
  proper one and a much better answer than nothing; a departed file extracts to the same text it
  does today.

---

## 6. Annotations

#### `annotations/subtype-defined-in-iso-32000-1` (PDF/A-2)
#### `annotations/subtype-defined-in-iso-32000-2` (part 4)
ISO 19005-2 6.3.1, ISO 19005-4 6.3.1 · today `not-built-yet`

- **Mitigation** — section 3.2 of the limits document says removal is the only option. **It is the
  only option for the *annotation*, and not for what the annotation carried**, and separating those
  two is the finding:
  - `discard` the annotation, which is the limits document's section 3.2 Ask and stays the default;
  - `preserve` its **normal appearance**, which is content the producer wrote, by appending it as a
    page (section 1.2's appended page, `Q58` permitting). Re-badging it as a `Stamp` remains fenced
    — that invents an annotation — but drawing the producer's own appearance stream onto a page
    invents no marks;
  - `preserve` the **media stream** of a `Screen`, `Movie` or `Sound` annotation by attaching it,
    which 4f and 4e allow for any file type. The sound is then in the archive, as a file, instead of
    being deleted;
  - `derive` a poster frame, key frames or a transcript through a declared tool, which is the
    owner's own example and is never a default.
- **By target** — differs in kind three ways. Part 2 forbids `3D`, `Sound`, `Screen` and `Movie`;
  part 4 forbids only the last three; **4e admits `3D` and `RichMedia` and 4f admits
  `FileAttachment`**, so for those subtypes the target is the shorter route and the report should
  say so rather than offering a remedy. The attach-the-media answer exists only at 4f and 4e.
- **From a configuration** — `remedy = "discard"` is the base; `preserve = ["appearance", "media"]`
  and `derive = { tool = "movie-poster", placement = "append" }` are the additions. The operator
  needs two sentences: *the annotation is gone from the page* and, where a page was appended, *the
  document has more pages than the original*.
- **Departure** — **B**. A `Screen` annotation in an archive is inert for a reader that does not
  play it; the file still renders. What it is not is *self-contained in meaning*, which is why the
  standard excludes it — and an operator who knows their archive never plays media can reasonably
  say so. Narrowable declaratively and well: `subtypes = ["Screen"]`.

#### `annotations/three-dimensional-only-in-engineering-files` (PDF/A-4, 4f)
#### `annotations/file-attachment-only-in-embedded-file-files` (PDF/A-4, 4e)
ISO 19005-4 6.3.1 · today `not-built-yet`

- **Mitigation** — `discard` the annotation, **and keep what it pointed at wherever the target
  allows**. The `FileAttachment` case is the one worth stating: at plain PDF/A-4 the *annotation* is
  forbidden but the **embedded file is not**, provided it is itself a conforming PDF/A (ISO 19005-4
  6.9). So dropping the annotation and leaving the file in the name tree keeps the attachment and
  loses only the on-page marker — which the limits document's section 3.2 sentence *"the media
  stream goes too"* does not apply to and nobody had separated out.
- **By target** — both requirements exist only because another flavour admits the subtype, so the
  first answer is always *use that flavour*. Confirmed **A**, and this is the pattern section 1.1
  warns about in reverse: here the relaxation is real and is the point of the requirement.
- **From a configuration** — `remedy = "discard"`, `keep-attachment = true`. One sentence: *the
  paperclip disappears; the file is still in the document*.
- **Departure** — **B** for `FileAttachment` at plain 4 — the annotation is a marker, and a file
  that keeps it is still an archive. **C** for `3D` outside 4e, because the flavour exists precisely
  to say which files may carry 3D, and departing from it makes the flavour meaningless.

#### `annotations/three-dimensional-stream-format`
ISO 19005-4 Annex B.2.2 · PDF/A-4e · today `not-this-target`

- **Mitigation** — three, and the census's `not-this-target` undersells it. `derive`: transcode the
  3D artwork to PRC or U3D with a declared tool, which is a real class of program an engineering
  archive already owns. `preserve`: attach the original 3D stream as an embedded file, which Annex B
  admits for any type — the artwork survives in the archive in its own format even though the
  annotation cannot show it. `discard`: remove the annotation and target plain PDF/A-4.
- **By target** — 4e only, by construction.
- **From a configuration** — `remedy = "derive"`, `tool = "3d-transcode"`, `on-failure =
  "preserve"`, which is the fallback chain `doc/rfc/0007` section 7 question 4 asks about and is a
  good argument for allowing one. The operator must understand that a transcoded model is **not**
  the model that went in — geometry converters approximate — and that `xmpMM:History` will say so.
- **Departure** — **C**. A 3D stream in a format the part does not name is content no conforming
  processor can render, which is the format's promise failing.

#### `annotations/printable-and-visible`
ISO 19005-2 6.3.2, ISO 19005-4 6.3.2 · all six · today `not-built-yet`

- **Mitigation** — the limits document's section 3.7 offers two futures, and both are real remedies
  rather than one being a fallback: `preserve` the annotation by clearing the hidden flags so it
  becomes visible and printable, or `discard` it. Section 3.7 of the limits document makes removal
  the default of the two because a hidden annotation was hidden on purpose. The half already built
  is the smaller one — an annotation stating no `/F` at all, where `--authorise annotation-printing`
  writes bit 3.
- **By target** — none.
- **From a configuration** — this is the profile question `doc/rfc/0007` section 5a.1 is built
  around, and the printing yardstick answers it per class without looking at any document: an
  archive that would have accepted a paper copy has already accepted losing what did not print, so
  `remedy = "discard"`. An archive that wants everything says `remedy = "preserve"` and gets a
  document with review comments visible on the page. Both are one word, and an operator can tell
  which they are.
- **Departure** — **B**, narrowable declaratively by flag: `flags = ["NoView"]` departs for an
  annotation meant for print only, which is a real production shape, while leaving `Hidden`
  enforced.

#### `annotations/appearance-dictionary-holds-only-normal`
ISO 19005-2 6.3.3, ISO 19005-4 6.3.3 · all six · today `not-built-yet`

- **Mitigation** — `discard` the `/R` and `/D` entries. What goes is what §12.5.5 has a reader draw
  while the pointer is over the annotation or the mouse button is down — interaction states, which
  no printed page ever carried and no archive reader will trigger. It is the cheapest authorised
  loss in the catalogue. `preserve` is available at 4f or 4e by attaching the streams, and is almost
  certainly not worth doing.
- **By target** — none.
- **From a configuration** — `remedy = "discard"`. One sentence, and the operator will not hesitate:
  *the document loses its hover and pressed appearances*.
- **Departure** — **B**, and equally cheap in the other direction: a file that keeps `/R` and `/D`
  renders identically and archives fine. This is a good example of a requirement where both the
  remedy and the departure are nearly free, which is itself worth reporting to an operator.

#### `annotations/normal-appearance-shape`
ISO 19005-2 6.3.3, ISO 19005-4 6.3.3 · all six · **`/AS` half built in session 957** (`Stated`);
the half with no `/AS` stays `not-built-yet`

- **Mitigation** — split, as ADR 0955 found. Where the annotation states an `/AS` entry, §12.5.5
  already says which stream a reader draws, so collapsing the subdictionary to that one is `Stated`
  and **owed, not optional**. Where it states none, nothing in the file says which state the
  document is in, and choosing is `A48`'s forbidden half: **no mitigation**.
- **By target** — none.
- **From a configuration** — for the second case, `remedy = "supply"` with `state = "Off"` is
  expressible and is the operator asserting what an unstated checkbox means across their archive. It
  changes what a page shows, so it belongs with the blend-mode `supply` in the small set that needs
  the owner's ruling.
- **Departure** — **B**. A file whose appearance dictionary has the wrong shape is one a reader
  resolves by its own rule; it renders, and it renders the same way in twenty years for any reader
  that follows §12.5.5.

---

## 7. Forms and signatures

#### `forms/no-action-on-widget-or-field`
ISO 19005-2 6.4.1, ISO 19005-4 6.4.1 · all six · today `not-built-yet`

- **Mitigation** — `discard` the `/A` entry, which is the limits document's section 3.3 Ask. Two
  `preserve`s sit beside it and neither is in the limits document:
  - **the standard's own archival route for form logic**: ISO 19005-4 6.4.1 says a processor that
    removes JavaScript but needs to keep an interactive form's values or logic shall store it as an
    embedded XFDF file whose file specification carries `/AFRelationship` `FormData`. That is
    `preserve`, sanctioned by the part itself — and because XFDF is XML rather than PDF/A, **only
    PDF/A-4f can hold it**;
  - **a widget's `/A` that is a URI action** could become a `Link` annotation over the same
    rectangle, which no clause forbids. It keeps the behaviour a reader actually uses. It also
    creates an annotation the producer did not write, so it sits on `Q58`'s line and is recorded
    here as a candidate needing the owner's ruling rather than as an answer.
- **By target** — in kind. At PDF/A-2 the behaviour is lost with nowhere to put it (the limits
  document's section 3.3 finding that the XFDF route is unavailable there); at 4 and 4e the XFDF
  route is named by the standard and still unavailable, because the file cannot be embedded; at
  **4f** it works.
- **From a configuration** — `remedy = "preserve"`, `as = "xfdf"` (4f only, and an error naming both
  elsewhere), else `remedy = "discard"`. The operator needs: *the form stops computing; what it
  computed is written down as data* — or, at the other targets, *the form stops computing*.
- **Departure** — **B**. A file with a `/A` on a widget archives perfectly well and renders
  identically; what it carries is behaviour a conforming reader may run. Narrowable declaratively
  and usefully by action type: `actions = ["URI"]` keeps a link working while leaving every scripted
  action enforced.

#### `forms/need-appearances-absent-or-false`
ISO 19005-2 6.4.1, ISO 19005-4 6.4.1 · all six · today `not-built-yet`

- **Mitigation** — the honest pair is the limits document's section 4.4: construct every field's
  appearance, then write the flag false so the file says what it shows. That is **owed, not
  optional** and is the right answer. A second answer exists and is a legitimate operator choice
  rather than a bug: `discard` the flag **without** constructing, accepting that fields whose
  appearance the producer never wrote show empty. A printed copy of that form would have shown the
  same empty boxes, which is what makes it defensible under the printing yardstick.
- **By target** — none.
- **From a configuration** — `remedy = "discard"`, `construct = true | false`. The cost sentence is
  concrete and checkable: *fields with no appearance stream will be blank in the archive*.
- **Departure** — **B**. `/NeedAppearances` true is a request to a reader, which an archival reader
  may ignore; the file is not thereby unarchivable. Not narrowable.

#### `forms/no-xfa-key`
#### `forms/no-needs-rendering`
ISO 19005-2 6.4.2, ISO 19005-4 6.4.2 · all six · `forms/no-needs-rendering`
**built in session 962**, `forms/no-xfa-key` today `not-built-yet`

- **The two are not one question**, which is what building the second half showed (ADR 0965).
  §7.7.2's Table 29 deprecates `/NeedsRendering` in PDF 2.0, names the XFA form as its subject and
  gives it a default of `false`, so removing it states what an absent entry states — and a document
  still holding an `/XFA` fails the row beside it and is refused, so no file the removal reaches had
  a form for any reader to regenerate. The `/XFA` half keeps its refusal and its entry:
- **Mitigation** — the limits document's section 3.4 default: keep the AcroForm's data and drop the
  `/XFA` key, which for a **static** form loses nothing, because ISO 32000-2 Annex K requires a
  conforming hybrid file's AcroForm entries to be consistent with the XFA information.
  `/NeedsRendering` then goes with it, mechanically. For a **dynamic** form the AcroForm is not the
  document and the answer stays a refusal. The addition this review makes: the XFA packet **is
  XML**, so at 4f it can be attached rather than deleted — the form's definition survives in the
  archive even though nothing will render it. That is the owner's *"and only xml"* shape appearing
  as a remedy rather than a departure.
- **By target** — the attach-the-packet answer is 4f (and 4e) only; the key removal is the same
  everywhere.
- **From a configuration** — `remedy = "discard"`, `keep-xfa = "attach"` where the target allows,
  `dynamic = "stop"`. The operator needs to know which kind of form their producer emits, and that
  is a fact about their pipeline rather than about any one document — exactly what a configuration
  is for.
- **Departure** — **B** for a static form, where the `/XFA` key is redundant with the AcroForm the
  standard requires to agree with it; **C** for a dynamic one, because a file whose pages are
  generated by an engine nobody archived does not render at all in twenty years. Predicate: the
  static/dynamic distinction itself, computed, and it is the same test section 3.4 of the limits
  document already owes.

#### `signatures/signature-widgets-meet-the-annotation-rules`
ISO 19005-2 6.4.3, ISO 19005-4 6.5.1 · all six · **built in session 957** (routed to the three
annotation rows it names)

- **Mitigation** — none of its own: the requirement asks the annotation flag and appearance rules
  again of a signature field's widget, and the converter already answers those where the annotation
  clauses state them. What is missing is routing — the decision taken per underlying rule rather
  than per requirement identifier. **Owed, not optional**, and a document refused here today is one
  the conversion would in fact have fixed.
- **By target** — none.
- **From a configuration** — nothing, and offering something would be worse than nothing: an
  operator would be configuring a loss to work around a routing bug.
- **Departure** — not applicable; the requirement is a compound of others that have their own
  entries.

---

## 8. Actions

Eight requirements, one subject: an action carries behaviour, and the only way to meet the clause is
to remove it (section 3.3 of the limits document). What the review adds is that **the behaviour and
the information it encoded are not the same thing**, and three of the eight can keep the second.

#### `actions/no-launch-multimedia-or-form-actions`
#### `actions/no-deprecated-set-state-or-no-op-actions`
#### `actions/named-action-is-page-navigation`
ISO 19005-2 6.5.1, ISO 19005-4 6.6.1 · all six · today `not-built-yet`

- **Mitigation** — `discard`. For a `Launch` action the thing it named is a path on somebody's disk,
  which an archive cannot keep and should not try to; for `ResetForm`, `ImportData` and `Hide` the
  behaviour has no representation outside a running reader; a deprecated set-state or no-op action
  encodes nothing at all, which makes its removal the cheapest `discard` here. A `Rendition` or
  `Movie` action's *media* is the annotation entry's business (section 6) and is preserved there
  where the target allows.
- **By target** — none in kind for these three.
- **From a configuration** — `remedy = "discard"`, and the printing yardstick answers it in one
  breath: none of this reached paper. The operator's cost sentence is *buttons stop doing things*.
- **Departure** — **B** for the no-op and set-state actions, which do nothing a reader will notice.
  **B** for `Launch` with a caution worth printing: a departed file keeps a reference to an
  executable path, which an archive's own security policy may care about more than ISO does.
  Narrowable declaratively by action type, which is the natural key for this whole group.

#### `actions/no-javascript-action` (PDF/A-2 only)
#### `actions/no-additional-actions-dictionary` (PDF/A-2 only)
ISO 19005-2 6.5.1, 6.5.2 · today `not-built-yet`

- **Mitigation** — `discard`, and **a retarget that is better than any remedy**: ISO 19005-4 6.6.2
  permits a JavaScript action outright, on the condition that a conforming processor runs it only on
  explicit user invocation, and 6.6.3 permits `/AA` on a widget annotation. So a document refused
  here for PDF/A-2 **converts to PDF/A-4 untouched**. Where the target is fixed at part 2, the XFDF
  route of section 7 is the only `preserve` and it needs 4f, so at PDF/A-2 the behaviour is simply
  lost.
- **By target** — confirmed **A**, and this is the census's cleanest true positive: part 4 permits
  what part 2 forbids, with no renumbered sibling anywhere.
- **From a configuration** — `remedy = "discard"`. The operator needs one fact and it is a big one:
  *a form that computed its own fields stops computing them, and at this target there is nowhere to
  put what it computed.*
- **Departure** — **A**, and it is the most defensible departure in the catalogue for an operator
  whose archive mandates PDF/A-2: the committee itself decided four years later that a conforming
  file may carry these. Narrowable declaratively — `triggers = ["U"]` for a widget's user-invoked
  action — and worth narrowing, because a document-open JavaScript action and a button's are very
  different risks.

#### `actions/no-optional-content-or-view-action` (PDF/A-2)
#### `actions/optional-content-or-view-action-only-in-engineering-files` (PDF/A-4, 4f)
ISO 19005-2 6.5.1, ISO 19005-4 6.6.1 · today `not-built-yet`

- **Mitigation** — `discard`. A `SetOCGState` action changes which layers are visible; removing it
  freezes the document in whatever the default configuration says, which is what both parts require
  a reader to render anyway. `GoTo3DView` goes with its annotation.
- **By target** — in kind: **4e permits both**, so an engineering archive keeps them; plain 4 and 4f
  do not; part 2 does not. Confirmed **A**.
- **From a configuration** — `remedy = "discard"` with the cost *layer buttons stop working; the
  default layers are what the archive shows*.
- **Departure** — **A**, narrowable by action type.

#### `actions/additional-actions-outside-widgets-hold-only-annotation-triggers`
ISO 19005-4 6.6.3 · PDF/A-4, 4f, 4e · today `not-built-yet`

- **Mitigation** — `discard` the keys outside the permitted set, which is a smaller act than
  removing the dictionary: what goes is a document-open or page-open trigger, and what stays is the
  annotation triggers the part admits.
- **By target** — part 2 forbids `/AA` outright, so this narrower rule exists only at part 4;
  confirmed **A** in the direction *part 4 relaxes part 2*, and the entry above is where the PDF/A-2
  answer lives.
- **From a configuration** — `remedy = "discard"`, `keys = [...]` if an operator wants finer
  control, though the part has already drawn the line.
- **Departure** — **B**. A document-level `/AA` is behaviour a non-interactive archival reader never
  fires. Narrowable by trigger name, declaratively.

---

## 9. Metadata

#### `metadata/xmp-packets-well-formed`
#### `metadata/xmp-packets-state-one-rdf-element`
#### `metadata/xmp-packets-meet-the-xmp-data-model`
ISO 19005-2 6.6.2.1, ISO 19005-4 6.7.2.1 · all six / part 2 for the third · `not-built-yet`

- **Mitigation** — **`preserve`, and it is the owner's own example arriving exactly where it is most
  needed.** The converter writes into the producer's packet by span, so a packet it cannot parse has
  no spans to write into; replacing it throws away everything the producer recorded, and repairing
  it is `A48`'s forbidden half. The third route nobody had written down: **write a fresh conforming
  packet and keep the original** — attached as a file at 4f or 4e, or rendered as a prefixed or
  appended page at any of the six, which is *"instead of losing metadata it could be appended or
  prefixed as an extra page"* in the owner's words. The producer's metadata then survives the
  conversion in a form a human can read and a machine can re-parse later, which is more than the
  malformed packet offered a validator.
- **By target** — the attach answer is 4f and 4e; the page answer is all six; the fresh packet is
  needed everywhere.
- **From a configuration** — `remedy = "preserve"`, `original = "attach" | "page" | "both"`,
  `fresh-packet = true`. Two facts for the operator: *the archive's metadata is what this converter
  could read, not what the producer wrote*, and *the producer's packet is kept as text rather than
  as metadata*. Both are plain, and an operator running a queue will recognise which they want.
- **Departure** — **B** for the one-`rdf:RDF` and data-model rules, where a tolerant parser still
  reads the packet; **C** for a packet that does not parse at all, because metadata no machine can
  read is the one thing an archival format cannot shrug at. The data-model rule is part 2's only
  (confirmed **A**), which gives an operator a cheaper route than departing.

#### `metadata/xmp-packet-header-attributes`
ISO 19005-2 6.6.2.1, ISO 19005-4 6.7.2.1 · all six · **built in session 962**

- **Mitigation** — remove the deprecated `bytes` or `encoding` attribute from the processing
  instruction: byte surgery of exactly the kind `pdf_model::xmp` already does for a property, losing
  nothing a reader of the packet uses, because both attributes describe the packet's own framing.
  Built in `pdf-transform` rather than in `pdf-model` — the change→gate map makes a `pdf-model` edit
  cost every gate in the tree, and the cut is the converter's business (ADR 0965). Three shapes are
  declined rather than guessed at: no `<?xpacket` in the window the validator searches, a header
  padded with the NUL bytes one of ISO 16684-1's wide encodings writes, and a value with no
  quotation marks to cut to.
- **By target** — none.
- **From a configuration** — nothing.
- **Departure** — **B**, and pointless.

#### `metadata/extension-schemas-embedded`
ISO 19005-2 6.6.2.3.2 · PDF/A-2b, 2u, 2a · today `not-built-yet`

- **Mitigation** — emit an extension schema container describing the schema the packet uses, which
  section 4.2 of the limits document permits and calls authoring in a small way, with an Ask where a
  value's type cannot be determined from what is there. The alternative is `discard` — drop the
  property, which is the limits document's section 3.9 machinery and already built for its own
  requirement.
- **By target** — part 4 states no extension schema requirement (confirmed **A**), which makes *use
  PDF/A-4* the cheapest answer for a document whose only fault is an undescribed schema.
- **From a configuration** — `remedy = "preserve"` (emit the container) with `undeterminable =
  "discard" | "stop"`. The operator needs to know that the container describes *shape*, not meaning,
  and that a property whose type nobody can tell will be dropped or will stop the run.
- **Departure** — **A**, confirmed, and narrowable by namespace — which is the ZUGFeRD shape again:
  *accept our own schema, described or not, and nobody else's*.

#### `metadata/extension-schema-container-fields`
ISO 19005-2 6.6.2.3.3 · PDF/A-2b, 2u, 2a · today `not-built-yet`

- **Mitigation** — **none that keeps the description honest.** What is missing is a name for the
  schema, a description of what a property means, or the category saying whether a value is derived
  from the document or supplied from outside it, and none of those is anywhere in the file.
  Supplying one is writing metadata about metadata that nobody produced. Two real answers remain and
  both are losses: `discard` the incomplete container — and then the properties it described are
  undescribed, so the limits document's section 3.9 removal follows them — or `supply`, where the
  **operator** states the missing field for their own schema. The second is the good one when the
  schema is the archive's own, which it often is: a company that emits `acme:` properties can
  describe them once.
- **By target** — part 4 states no such rule (confirmed **A**).
- **From a configuration** — `remedy = "supply"` with a per-namespace table of the missing fields,
  or `remedy = "discard"`. This is the clearest case in the catalogue where a configuration turns an
  unanswerable per-document question into a one-time statement, because a schema belongs to an
  organisation rather than to a document.
- **Departure** — **B**. A container missing its human-readable description still identifies the
  schema and its property types, which is what a later reader needs. Narrowable declaratively by
  field name: departing for `description` while enforcing `schema` and `valueType` is a defensible
  line and a much smaller hole than dropping the rule.

#### `metadata/identification-amendment-form`
ISO 19005-2 6.6.4 · PDF/A-2b, 2u, 2a · today `not-built-yet`

- **Mitigation** — `discard`: remove the malformed amendment identifier. Neither the number nor the
  year can be recovered from a malformed one, the entry is optional, and what is lost is the
  producer's claim about which amendment the file was made to — which was wrong anyway, since it did
  not have the form the part requires.
- **By target** — part 4's identification schema does not state this (confirmed **A**).
- **From a configuration** — `remedy = "discard"`. The cost sentence is almost nothing, and saying
  so is useful: *the file stops claiming an amendment it identified incorrectly*.
- **Departure** — **B**, and hard to want.

#### `metadata/provenance-recorded-action-fields-four`
ISO 19005-4 6.7.5 · PDF/A-4, 4f, 4e · today `the-fence`

- **Mitigation** — **none that adds anything.** An entry in the producer's history that is missing
  its action or its time records something this converter did not witness, and filling it in is
  writing a provenance nobody has — the opposite of what an audit trail is for. `discard` — remove
  the incomplete entry — is available and is a genuine loss: the archive then has no record of that
  step at all. Between an incomplete record and no record, an archivist will usually prefer the
  incomplete one, which is why this is a case where the departure beats every remedy.
- **By target** — part 2 requires the parameters field part 4 demotes to a recommendation, but
  states no rule this converter refuses on; confirmed **A** in the direction that makes a PDF/A-2
  target accept the document.
- **From a configuration** — `remedy = "discard"` with the cost stated as *one step of the
  document's history is deleted*. Most operators should not take it.
- **Departure** — **B**, and it is this entry's recommended answer. A history entry missing a field
  is still evidence; the file remains an archive in every sense. Narrowable declaratively by field —
  depart for a missing `when`, enforce a missing `action` — and by age, if anybody wants entries
  older than the archive's own policy treated differently.

---

## 10. Logical structure

All four bind PDF/A-2a alone, so the census marks all four kind A, and for three of them the honest
first answer is indeed *take 2u or 2b*. The fourth is the best `supply` case in the catalogue.

#### `logical-structure/structure-tree-root`
ISO 19005-2 6.7.3.3 · PDF/A-2a · today `the-fence`

- **Mitigation** — **none.** Auto-tagging decides that this run of glyphs is a heading and that one
  a table cell, and a wrong reading order misleads confidently. Section 5.1 of the limits document
  is the argument and ISO 19005-2 6.7.1 takes the same position in its own words, advising against
  adding structural information not present in the source solely to achieve conformance.
- **By target** — the answer is the target: 2u and 2b ask nothing of logical structure, and part 4
  has no levels at all. A document without a tree is a PDF/A-2u document.
- **From a configuration** — nothing, and an operator should be told why rather than offered a
  switch. The owner's own framing settles what to do instead: *the logical structure is a nice to
  have, but I would rather lose information I wouldn't have had as paper archive anyway* — choosing
  2b or 2u **is** that choice, made once, in the target rather than in a profile.
- **Departure** — **C**. A Level A claim over a document with no structure tree is a claim about the
  one thing Level A means.

#### `logical-structure/role-map-terminates-at-a-standard-type`
ISO 19005-2 6.7.3.4 · PDF/A-2a · today `not-built-yet`

- **Mitigation** — **`supply`, and this is the entry that best answers the owner's question.** A
  non-standard structure type has to map, directly or through further non-standard types, to a
  standard one. The converter will not guess — `Chapter` → `Sect` is a guess that happens to be
  right and `Sidebar` → `Note` is one that may not be — but a **producer's type vocabulary is finite
  and belongs to whoever runs the pipeline**. Stating the map once in a configuration converts an
  unanswerable per-document question into a policy an archive writes on its first day and never
  revisits. The limits document's section 5.1 step 3 already describes the interaction as Ask,
  listing each unmapped type with the elements that use it; the configuration is that list answered
  in advance, which is the whole of `doc/rfc/0007` in one requirement.
- **By target** — 2a only; every other target ignores the role map.
- **From a configuration** — `remedy = "supply"` with `role-map = { Chapter = "Sect", Sidebar =
  "Aside" }`, plus `unlisted = "stop"` so a type nobody has mapped refuses rather than being
  guessed. What the operator has to know is what their own types mean, which is the one thing they
  certainly do know. The report and `xmpMM:History` must record that the mapping came from the
  configuration.
- **Departure** — **B**, and not needed once `supply` exists. An unmapped type leaves a reader
  unable to interpret those elements, which is a real loss at Level A but not an unrenderable file.

#### `logical-structure/catalog-language-identifier`
#### `logical-structure/element-and-property-list-language-identifiers`
ISO 19005-2 6.7.4 · PDF/A-2a · today `not-built-yet`

- **Mitigation** — `supply` or `discard`. The `/Lang` value is not a valid language identifier;
  correcting it means deciding what language the producer meant (`A48`'s forbidden half) and
  removing it drops the only statement the file makes about the text's language, which §14.9.2 has a
  reader use. An **operator** frequently does know — an archive of German municipal records is in
  German — and a configuration that states the default language is making that knowledge explicit
  rather than guessing. Where nobody knows, `discard` is available and its cost is precise: a screen
  reader falls back on its own default.
- **By target** — 2a only among the six; part 4 and Levels U and B state no language rule.
- **From a configuration** — `remedy = "supply"`, `language = "de-DE"`, or `remedy = "discard"`. A
  refinement worth having and cheap: `only-when = "malformed"` so a valid identifier is never
  overwritten. The operator needs to know that the value will be asserted for the whole document.
- **Departure** — **B**. A malformed `/Lang` is ignorable by a reader; the document still renders
  and still archives. Narrowable declaratively by where the entry sits — depart for a structure
  element's `/Lang`, enforce the catalog's — which matches how much each one costs.

---

## 11. Embedded files

#### `embedded-files/embedded-file-is-itself-pdfa` (PDF/A-2b, 2u, 2a)
#### `embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile` (PDF/A-4)
ISO 19005-2 6.8, ISO 19005-4 6.9 · today `not-built-yet`

- **Mitigation** — this is `doc/rfc/0007` section 4.6's flagship and the catalogue confirms its four
  answers, in order of preference: **attach unchanged** (4f and 4e, nothing lost and nothing
  derived); **convert the attachment to PDF/A and attach the result**, which is running this whole
  verb over the embedded bytes under a budget of their own — lossless where it works, and the
  attachment is then a different byte string, which matters if anything signed or hashed it;
  **derive** a PDF from a non-PDF attachment with a declared tool and attach or append it; **append
  its pages** into the document's body, which every one of the six admits; and `discard` last.
- **By target** — the sharpest in-kind difference in the standard: 4f and 4e take the original file
  as it is; 4 takes a conforming PDF/A; 2b, 2u and 2a take a conforming PDF/A-1 or -2 only. The
  appending route is available under all six, which is RFC 0007 section 4.6.1's correction and the
  reason no target is left with only `discard`.
- **From a configuration** — the RFC's own example, and it is expressible: `remedy = "derive"` with
  `tool = "office-to-pdf"` and `on-failure = "discard"`, plus per-target qualifiers so 4f attaches
  unchanged and 2b appends. What the operator has to understand is which of those four happened to
  each file, and the report has to say it per attachment rather than per document.
- **Departure** — **B**, and it is the owner's own case: *accept xml (and only xml) attachments when
  targeting PDF/A-2*. A file that is PDF/A-2 in every other respect and carries one XML attachment
  is plainly still an archival document, and no part of ISO 19005 expresses that permission — part 3
  and 4f both admit anything. Narrowable exactly as RFC 0007 section 4.7.2 shows, by media type and
  `/AFRelationship`.

#### `embedded-files/associated-file-media-type`
ISO 19005-4 6.9 · PDF/A-4, 4f, 4e · today `not-built-yet`

- **Mitigation** — `supply`. The clause asks an associated file stream for a `/Subtype` that is a
  MIME media type, and nothing in a file specification states one: an extension is a convention
  rather than a declaration, so reading it as one would be this converter asserting what the bytes
  are. An operator whose pipeline produces the attachments **does** know, and can say so by
  extension or by relationship.
- **By target** — part 2 states no associated-file rule at all (confirmed **A**), so the requirement
  does not arise at a PDF/A-2 target.
- **From a configuration** — `remedy = "supply"` with `media-types = { ".xml" = "application/xml",
  ".csv" = "text/csv" }` and `unlisted = "stop"`. This is a good test of the owner's question and it
  passes cleanly: the operator needs to know what their own attachments are, types nothing about any
  individual document, and can read the cost in one line — *the archive asserts these media types on
  our authority*.
- **Departure** — **B**. An attachment without a declared media type is still readable; a reader
  guesses, as readers did for decades. Narrowable by relationship or by extension.

#### `embedded-files/pdfa-4f-carries-embedded-files`
ISO 19005-4 Annex A.2 · PDF/A-4f · today `not-this-target`

- **Mitigation** — **none, and it is the only requirement in either part that a document fails by
  holding nothing.** Attaching a file to satisfy it would be adding content no source states.
- **By target** — the answer is plain PDF/A-4, which is what a document with nothing embedded is
  for.
- **From a configuration** — nothing. A configuration that attached a placeholder to satisfy a
  flavour's entry condition would be manufacturing conformance, which is the failure mode this whole
  design exists to avoid.
- **Departure** — **C**, in an unusual direction: departing means writing a file that claims 4f
  while containing none of what 4f exists to carry. The claim would be the only thing the file had.

---

## 12. Optional content

#### `optional-content/configuration-names`
#### `optional-content/order-lists-every-group`
ISO 19005-2 6.9, ISO 19005-4 6.10 · all six · **the `/Order` row built in session 957**
(`Mechanical`); the name row stays `not-built-yet`

- **Mitigation** — section 3.8 of the limits document calls both Default work and the review splits
  them. Completing an `/Order` array with the groups it omits, in the file's own order, invents
  nothing: the groups are in the file and the order is the file's. That half is **owed, not
  optional**. A **name** is different — §8.11.4.3 makes `/Name` a label for a user interface, so a
  synthesised one is text no producer wrote, which is why ADR 0955 flagged it as meeting `A48`'s
  line. The honest resolution is that synthesising `Configuration 1` is not an interpretation of
  anything: it asserts no meaning, it fills a slot the clause requires to be non-empty and unique,
  and the report says the label is the converter's. That makes it `supply` with the converter as the
  supplier — the one place in this catalogue where that is defensible, and it should say so out loud
  in the file's history.
- **By target** — none.
- **From a configuration** — `name-template = "Configuration {n}"` so an operator can choose their
  own wording, or `remedy = "stop"` for an archive that wants no converter-written text in its files
  at all. Both are one line, and the second is a real position.
- **Departure** — **B** for the name (a nameless configuration is a labelling defect), **B** for the
  order. Neither departure is worth taking when the remedies cost this little.

#### `optional-content/no-automatic-states`
ISO 19005-2 6.9 · PDF/A-2b, 2u, 2a · today `not-built-yet`

- **Mitigation** — `discard` the `/AS` entry, which is the limits document's section 3.8 Ask: `/AS`
  is what switches layers by zoom, by print-versus-view or by user event, so removing it freezes the
  document into whatever the default configuration states — which is what both parts require a
  reader to render anyway.
- **By target** — **the cleanest kind-A difference in the catalogue**: ISO 19005-4 6.10 keeps the
  key and requires a conforming processor to ignore it. Same effect for a reader, no loss in the
  file. A PDF/A-4 target is strictly better here, and the report should say so.
- **From a configuration** — `remedy = "discard"`, with the cost *the archive shows the default
  layer set and nothing switches*.
- **Departure** — **A**, confirmed, and unusually comfortable: part 4's own answer is *keep it and
  ignore it*, so a departed PDF/A-2 file is doing exactly what a conforming PDF/A-4 file does.
  Narrowing is unnecessary — the key is one key.

---

## 13. What the catalogue found

### 13.1 Mitigations nobody in this project had written down

Sixteen, in rough order of how much they unblock. Each is argued at its own entry.

1. **The 4f preserve** (section 1.2). Under PDF/A-4f — and, for its own material, 4e — *anything
   this converter would otherwise delete as a byte string can stay in the file as an attachment*: a
   rejected ICC profile, a media stream, an unparseable XMP packet, an XFA packet, a halftone or a
   transfer function, the source document itself. One clause turns a long list of `discard`s into
   `preserve`s.
2. **CMaps from the shipped set** (section 5). The 239 Adobe predefined CMaps are already in
   `data/cmaps/` under BSD-3-Clause, so a file naming one the base standard does not predefine can
   have it **embedded** rather than refused. The mapping is Adobe's, not ours, so nothing is
   invented.
3. **External data resolved by a declared tool** (section 2). The refusal's reasoning — *this
   program has no network and will not acquire one* — is about the library and stops being an
   argument once `doc/rfc/0007` section 4 exists. The fetch is the operator's program, under their
   trust, and the result loses nothing at all.
4. **The `ICCBased` alternate space** (section 4.2). Table 65 states what a processor that cannot
   use the profile does; a converter rejecting the profile is exactly that processor, so the
   fallback is `Stated` rather than a loss this project invented.
5. **A document's permissions preserved as a statement** (section 2). The enforcement cannot survive
   decryption; the producer's assertion can, in the report, in `xmpMM:History`, and on a page.
6. **Orphaned objects and the object-count limit** (section 3). A file over ISO 19005-2's limit
   because twenty years of incremental updates left unreachable objects behind converts with nothing
   lost, because the serializer writes only what the document reaches.
7. **Dropping a `FileAttachment` annotation without dropping the file** (section 6). At plain
   PDF/A-4 the annotation is forbidden and the embedded file is not; only the on-page marker has to
   go.
8. **A malformed XMP packet kept as content while a fresh one is written** (section 9). The owner's
   *append or prefix the packet as a page* answers the case the limits document had no answer for.
9. **The XFA packet attached rather than deleted** (section 7). It is XML, so 4f can hold it — the
   owner's *"and only xml"* appearing as a remedy rather than as a departure.
10. **The standard's own XFDF route for form logic** (section 7), which ISO 19005-4 6.4.1 names and
    which only 4f can actually hold — a `preserve` sanctioned by the part itself.
11. **`Identity` for a missing `/CIDToGIDMap` at a part 2 target** (section 5), because ISO 19005-2
    5.1 makes ISO 32000-1 the base document and that edition states the default. The same write at a
    part 4 target is an invention. One requirement, two answers, decided by the target's base.
12. **A multimedia annotation's appearance appended as a page and its media attached** (section 6).
    the limits document's section 3.2 *removal is the only option* is true of the annotation and
    false of what it carried.
13. **A role map supplied by the operator** (section 10) — the best answer in the catalogue to the
    owner's actual question, and the model for `supply` generally.
14. **A 3D stream attached at 4e while the annotation goes** (section 6), which turns a
    `not-this-target` into a document that keeps its artwork.
15. **Removing `/CharSet` and `/CIDSet` rather than recomputing them** (section 5): the base
    standard makes both optional, so the lighter of the two lossless routes was available all along.
16. **Removing a JPEG 2000 colour specification rather than writing one** (section 4.5), found in
    the nine-hundred-and-seventy-first session after two rounds had answered *none*. The sentence
    beside the rule says a conforming processor shall use only the selected colour space and shall
    ignore all the others, so the boxes that fail are the ones the part itself directs a processor
    to ignore; what it costs is §7.4.9's fallback chain and nothing on the page.

### 13.2 Where the answer is honestly *none*

Twenty-nine requirements have no mitigation, and each entry carries the argument. They fall into
four shapes, and the shapes are more useful than the list:

- **The mark is in the content stream** (ADR 0816's fence):
  `graphics/only-operators-the-base-standard-defines`, `graphics/inline-image-interpolation-is-off`,
  `file-structure/inline-image-filters`, `fonts/type3-glyph-procedures-state-their-width`,
  `fonts/actual-text-covers-private-use-characters`, and nine of the ten implementation limits.
- **The file states two things and nothing says which the producer meant**:
  `fonts/cid-system-info-agrees-with-the-cmap`, `file-structure/bound-names-are-valid-utf8`,
  `graphics/named-resources-are-defined`,
  `graphics/content-streams-have-an-explicit-resources-dictionary` in its form XObject half,
  `annotations/normal-appearance-shape` where no `/AS` is stated.
- **The remedy would be a wrong glyph, and a wrong glyph is not a remedy**:
  `fonts/embedded-programs-define-every-glyph-shown`, `fonts/no-notdef-glyph-shown`, the four
  TrueType `cmap` and `/Differences` rules,
  `fonts/truetype-codes-reach-glyphs-by-the-standard-route`, and the bare-name half of the two
  blend-mode requirements.
- **The information is not in the document and nobody but its producer has it**:
  `logical-structure/structure-tree-root`, `metadata/extension-schema-container-fields` where the
  schema is not the operator's own, `metadata/provenance-recorded-action-fields-four`,
  `embedded-files/pdfa-4f-carries-embedded-files`, `file-structure/stream-filters-are-standard`.

**None of these is a gap.** ADR 0955's discipline applies: a refusal with an argument is finished
work, and the useful thing a report can add is which of the four shapes it is, because the shapes
have different next steps — the first is *ask the producer to re-export*, the second is *ask the
producer which*, the third is *supply the font*, the fourth is *keep the source*.

### 13.3 Owed, not optional — the refusals no operator should be asked about

Twenty-two requirements were refused although the right answer **loses nothing**. They were waiting
on code, not on a decision, and offering an operator a `discard` to get past one would trade a
permanent loss for a missing afternoon's work.

**Eleven of the twenty-two were built in the nine-hundred-and-fifty-seventh session** (ADR 0957),
and are struck from the list here as they were struck from `decision.rs`'s `REFUSED_BY_NAME`:

- `graphics/rendering-intent-entries-name-one-of-four` — `Stated`, §8.6.5.8's own answer written
  into the entry. An inline image's `/Intent` is refused separately and by the fence.
- `graphics/graphics-state-blend-modes-are-defined` and
  `graphics/annotation-blend-modes-are-defined` — `Stated`, **the array half only**; a bare name
  the standard does not define keeps its refusal, with a sentence that now says which half is
  which.
- `graphics/content-streams-have-an-explicit-resources-dictionary` — `Mechanical`, **the page half
  only**; the form `XObject` half is refused by the fence, because one dictionary cannot answer for
  every invocation.
- `annotations/normal-appearance-shape` — `Stated`, **where the annotation states an `/AS`**.
- `optional-content/order-lists-every-group` — `Mechanical`. Its neighbour
  `optional-content/configuration-names` is still refused, and the two reasons are now separate
  sentences rather than one.
- `fonts/charset-lists-every-glyph-in-the-program` and `fonts/cidset-lists-every-cid-in-the-program`
  — `Mechanical`, by the **remove** route rather than the recompute one. The catalogue offered both
  and the base standard decides — **and session 966 found that the edition named here is the wrong
  one for these two rows**, which bind a part 2 target and nothing else: ISO 32000-2 deprecates both
  keys, ISO 32000-1:2008 does not, and what makes removal lossless at the target these rows actually
  bind is that its Table 122 and Table 124 make both entries optional and give each the same meaning
  when absent — a subset indicated by the subset tag in `/FontName` and by nothing else. The route
  was right; the reason had been written for the other edition.
- `fonts/cid-to-gid-map-present` — `Stated`, **at a part 2 target only**, exactly as the entry
  above reads it.
- `implementation-limits/indirect-object-count` — the **serializer's**, which is what the entry
  said: the writer carries only what the converted document reaches, so a file over the limit
  through accumulated orphans converts with nothing lost. A file genuinely over it is still refused,
  by the output's own verdict.
- `signatures/signature-widgets-meet-the-annotation-rules` — **routing**, as the entry said, and it
  needed a fifth kind of answer in the decision table rather than a rewrite: `Answer::AsUnderlying`
  takes the compound row's decision from the three annotation rows it names.

**Four more were built in the nine-hundred-and-sixty-second session** (ADR 0965), and are struck
here as they were struck from `REFUSED_BY_NAME`:

- `metadata/xmp-packet-header-attributes` — `Mechanical`. Byte surgery on the `<?xpacket … ?>`
  processing instruction, declining three shapes rather than guessing at them: no header in the
  window the validator searches, a header padded with one of ISO 16684-1's wide encodings' NUL
  bytes, and a value with no quotation marks to cut to.
- `forms/no-needs-rendering` — `Mechanical`, and **on the refusal of the requirement beside it**.
  §7.7.2's Table 29 deprecates the key, names XFA as its subject and gives it a default of
  `false`; a document still stating an `/XFA` fails `forms/no-xfa-key` and is refused, so no file
  this rewrite reaches had a form for any reader to regenerate.
- `fonts/symbolic-truetype-states-no-encoding` and
  `fonts/non-symbolic-truetype-uses-a-standard-encoding` — `Mechanical`, **with the proof this
  entry asked for**: the font is loaded as the file states it and again as it would be written,
  and every code the content streams showed has to reach the same glyph both ways. A font whose
  program the file does not carry, or whose shown strings outran the survey's budget, is refused.

**Five turned out not to be losslessly buildable at all.** They keep their refusal and lose their
debt: each now carries an argument for why the right answer is a *choice* rather than an unwritten
afternoon, and §13.3.1 has all five — the two duplicate-profile rows
(`graphics/no-icc-space-duplicating-the-output-intent-profile`,
`graphics/separation-alternate-space-does-not-duplicate-a-current-profile`), the two JPEG 2000 box
rows (`graphics/jpeg2000-colour-specification-method`,
`graphics/jpeg2000-one-best-colour-space-specification`), and
`graphics/spot-colourants-appear-in-the-colorants-dictionary` — **and the last of those five did not
stay refused**, which is the correction below.

**Two more were built in the nine-hundred-and-sixty-sixth session** (ADR 0973), and are struck here
as they were struck from `REFUSED_BY_NAME`:

- `graphics/one-destination-profile-per-output-intents-array` — `Mechanical`, **the same-profile
  half only**, on a proof rather than on a reading: both entries' destination profiles are decoded
  and the objects are shared only where they are the same bytes under the same stream dictionary,
  so the object that goes carried a copy and no entry changes what it refers its colours to. Two
  entries naming genuinely different profiles keep the refusal, because one destination is then
  discarded; §4.1 carries the split.
- `graphics/spot-colourants-appear-in-the-colorants-dictionary` — `Mechanical`, **where the file
  states a `Separation` array for the colourant itself**, which turns out to need no derivation at
  all. This row had been moved into the fence four rounds earlier on the finding that the entry
  could only be *sampled* from the producer's N-input transform, and both that finding and the
  original entry had read one sentence of 6.2.4.4 and not the next: the subclause requires every
  `Separation` array in a file naming one colourant, in a `Colorants` dictionary or anywhere else,
  to state the same alternate space and tint transform, compared as PDF objects. So where the file
  defines the ink, the entry is *determined* rather than chosen. The sampled route stays owed for a
  file that defines the ink nowhere else.

**Two of the five that were moved out came back, and the second is the nine-hundred-and-seventy-first
session's** (ADR 0982). The two JPEG 2000 box rows —
`graphics/jpeg2000-colour-specification-method` and
`graphics/jpeg2000-one-best-colour-space-specification` — are **not** back on this list, because
they were never owed: they are a `discard` an operator authorises, which is exactly what §0.2 says
this list must not contain. What came back is the *catalogue entry*, which had said **none** where
the honest answer was **an authorised loss nobody had offered**. §4.5 carries the reading; the short
form is that both previous readings asked what value could be written into a `colr` box and neither
asked whether a box could go, and the sentence that answers it is the one after the method's in both
parts. So this list is unchanged, and what changed is §4.5's own four answers: two of them said
*none* and *nothing* where the truthful pair is *`discard`* and *authorise it*.

**The last one was built in the nine-hundred-and-seventy-sixth session** (ADR 0988), and is struck
here as it was struck from `REFUSED_BY_NAME`:

- `fonts/vertical-metrics-agree-with-the-program` — `Mechanical`, by overwriting the program's
  `vmtx` advance field where it already sits. **The list is now empty, and the entry was wrong a
  third time on the way out**: it named `pdf_font::restate` as the writer the row waited on, and
  that module owns the units and the tolerance while the byte surgery is `sfnt.rs`'s. §13.3.1 has
  the correction and the reading of §9.9.1's *preceding* sentence, which turns out to make
  *removing* `vhea` and `vmtx` as lossless as restating them.

**This is the catalogue's most actionable output for the converter itself.** Nearly a fifth of the
refusals were lossless rewrites nobody had written, and every one of them converts documents that
had stopped — with the correction four rounds of building it produced: **of the twenty-two,
eighteen were waiting on code and four were waiting on a decision after all.** A claim that a
refusal is only unwritten work is itself a claim, and it decays the way a ledger row's does —
**and so does a claim that it is not**, which is what the spot-colourant row is now the standing
example of: it was moved out of this list by argument and came back into it by a better one.

**Nothing on this list was ranked by the corpus, and the last one could not have been.** The
veraPDF corpus's conversion figures did not move by a single document when the vertical rewrite
landed: no file in it sets a composite font vertically with a `vmtx` that disagrees. A round
waiting for the corpus to ask would still be waiting, which is `CLAUDE.md`'s two denominators
stated as a fact about this list rather than as a principle.

#### 13.3.1 What building them corrected in this catalogue

Two entries above were wrong about the work and one about the standard, and the corrections belong
here rather than in a session note:

- **The site of a failure is the object a dictionary is *written in*, not the dictionary.** Both
  blend-mode entries and the rendering-intent entry read as though the failing entry were on the
  object the validator names. It usually is not: a graphics state written directly inside a page's
  resource dictionary is reported at the page's object number, so a rewrite that edits the named
  dictionary alone fixes nothing at all. Both rewrites descend the object they are given.
- **`/Intent` is two keys with one spelling.** §8.9.5.1's Table 87 gives an image `XObject` a
  rendering intent and §8.11.2.3 gives an optional content group an `/Intent` of `View` or
  `Design`. The entry did not say so, and a rewrite taking the catalogue at its word would have
  turned a layer's intent into `RelativeColorimetric`.
- **`/CharSet` is deprecated too**, not only `/CIDSet`. The entry gave the part 4 preference for
  removal on `/CIDSet`'s deprecation alone; §9.8.1's Table 122 deprecates both in PDF 2.0, which
  makes *remove* the better of the two lossless routes at every target rather than at one.

Four more, from the four built in session 962 and the five refused there (ADR 0965). Two are
about the standard and two about the work, and the first would have written a wrong file:

- **The vertical metrics entry had the writing direction backwards.** It proposed restating
  `/DW2` and `/W2` from the program "on the same argument that already justifies the horizontal
  case" — and the horizontal case restates the **program**, for the reason that makes the other
  direction unsafe: §9.2.4 makes the font dictionary's numbers what positions a glyph without
  looking inside the program, and §9.7.4.3 gives `/DW2` and `/W2` that role going down the page.
  Restating them moves every glyph on a vertical line. The entry was also wrong about what the row
  waits on: `pdf_font::LoadedFont::program_vertical_advance` already states the program's number —
  the validator's own predicate is built on it — and what is missing is the *writer*,
  `pdf_font::restate`, which rewrites an sfnt's `hmtx` and nothing vertical.
- **The two duplicate-profile rows are neither mechanical nor sited.** ISO 19005-4 6.2.4.2's last
  requirement binds a space that is *used*, so the failure is reported where the content stream
  selected it — a page, with no object — and the colour space array sits in a resource dictionary
  no finding names; siting the rewrite means walking the content streams a second time to decide
  which space was used, which is the validator's reading made again in the converter. And even
  sited it would not be a restatement: §8.6.7 applies non-zero overprint mode only where the
  current space is `DeviceCMYK` or is implicitly converted to it, so the substitution can decide a
  composite §8.6.5.7 leaves open — the very ambiguity 6.2.4.2's NOTE 2 gives as the reason for the
  prohibition.
- **The JPEG 2000 entry was right about the cost and silent about the value.** These two fields do
  live in the JP2 wrapper, and the rewrite is a hundred-odd bytes that touches no sample — but
  every value it could write is a choice. A `METH` outside the three the part admits describes the
  colour in a way the part does not read, so writing one of the three states a colour space the
  box did not; and marking exactly one specification best available, where the file marks none,
  ranks two of the producer's own specifications on evidence the file does not carry. The entry
  also missed the row's second sentence, which requires the *selected* specification's ICC profile
  to conform — the profile-replacement case, not byte surgery.
- **A PDF function cannot call another one.** The colourants entry called deriving a single
  colourant's transform from an *N*-input one "arithmetic rather than policy"; §7.10 gives no way
  to compose functions, so the §8.6.6.4 `Separation` this would write needs a one-input function
  that the general route can only obtain by **sampling** the producer's — an approximation written
  into an archive as though it were their definition. One shape could be exact and is the thing to
  build first: a §7.10.2 sampled transform already states its values on a grid, so the samples
  along one axis are the producer's own numbers.

Three more, from the two built in session 966 (ADR 0973). Two are about the standard and one is
about this catalogue's own habits:

- **A requirement's answer can be in the paragraph after the one its title quotes.** The colourant
  entry above was written twice — once as "arithmetic", once as "sampling, and therefore a loss" —
  and both readings stopped at 6.2.4.4's first sentence. Its second sentence makes every
  `Separation` of one name in a file agree, *including the ones in `Colorants` dictionaries*, which
  settles the value outright for any file that defines the ink anywhere. Neither reading was wrong
  about what it read. Both were wrong about having finished reading.
- **Three rows sharing one refusal sentence were three different waits.** The output-intent group's
  sentence said "a PDF/A entry naming no destination profile, or several entries naming different
  profile objects, or a page's own array doing either", and a reader could not tell that the middle
  clause of it turns on a question the file answers — are the profiles the same bytes? — while the
  other two turn on questions only an operator can. One sentence for several rows hides exactly the
  distinction that decides which of them is buildable.
- **A justification can be right at a target whose base standard never states it.** Walking the
  decision table for rows that cite an ISO 32000-2 clause while binding a **part 2** target found
  three, and none of them wrote a wrong file — ISO 32000-1:2008 states the same rule in two of the
  three cases, and in the third the ISO 19005 clause was doing the work all along. What was wrong
  was the reason written down, which is the thing this catalogue exists to keep. ADR 0973 has all
  three and what each now says.

Three more, from the nine-hundred-and-seventy-sixth session (ADR 0988). One is about the standard,
one about this catalogue's own habit of naming files, and one about the code a rewrite lands in:

- **A clause's neighbour can answer a question the entry did not think to ask.** This row was read
  three times for *which side may move* and never for *whether the table has to be there at all*.
  The paragraph immediately before §9.9.1's vertical sentence lists the TrueType tables that shall
  always be present if the original had them, and `vhea` and `vmtx` are not in it — so the standard
  declines to require their preservation one sentence before it forbids their use, and **remove**
  is a second lossless route this row had beside **restate** the whole time. The entry had even
  predicted it, in the words "it predicts which way *every* agree-with-the-program row should be
  rewritten"; what it did not do was look.
- **Naming a module is not naming a writer.** The entry said `pdf_font::restate` "rewrites an
  sfnt's `hmtx`", which is true of the module's subject and false of its code: the splice, the
  directory update and the checksums are `sfnt.rs`'s, and a round given `restate.rs` alone was
  given the policy and not the mechanism. A catalogue entry that names where work belongs is
  making a claim about the tree, and that claim decays exactly the way a reading of the standard
  does.
- **A rewrite that reaches an object another rewrite already replaces is not a competitor.** Both
  metric restatements replace the same font program stream, and a font disagreeing in both
  directions has to be restated twice into one set of bytes. One replacement map per rewrite
  cannot express that, and the loss is silent — the second write simply never happens. What
  expresses it is one replacement per object plus a set per requirement saying which asked.

Three more, from the nine-hundred-and-seventy-first session (ADR 0982). Two are about this
catalogue's habits and one settles a question it had left open:

- **"There is no mitigation" and "there is no *lossless* mitigation" are different claims, and this
  catalogue conflated them twice on one entry.** The JPEG 2000 colour-box rows were argued through
  two rounds entirely inside the question *what value could be written here* — and every answer to
  that question is a choice, which is true and was never the whole question. Removing what the
  clause itself directs a processor to ignore writes no value at all, and RFC 0007 section 2's
  `discard` had been in the vocabulary the whole time. **An entry whose four answers include *From a
  configuration — nothing* should be read again, because that line is where this mistake shows.**
- **The used box is named by two clauses, one of which is not ISO 19005's.** ISO/IEC 15444-1:2000
  I.5.3.3 — reserve `APPROX`, set it to zero, ignore its value, and ignore every colour
  specification box after the first — is what makes a part-1-era JP2 file's *first* box the used
  one. That file is precisely the file that fails ISO 19005's *exactly one marked best*, so the
  clause that makes the row fail and the clause that makes it fixable are in different documents.
  `CLAUDE.md`'s rule about reading a clause's neighbours extends one step further than it says: the
  neighbour can be in the standard the clause delegates to.
- **The duplicate-profile pair was re-examined against machinery that had arrived since, and held**
  — see §4.2. A rewrite reaching an object of a new *shape* does not site a failure reported
  somewhere else; what sites a rewrite is the finding naming the object. And §8.6.5.7 turns out to
  say outright that the implicit conversion "cannot be specified in PDF", which makes writing
  `/DeviceCMYK` a decision taken away from the processor rather than one restated from the file.

---

## 14. What the configuration format cannot express

`doc/rfc/0007` section 3 proposes the format, and writing a hundred and eighteen answers against it
is the first test it has had. Ten findings, in the order they cost the most.

**1. A site is finer than a requirement, and the format assumes they are the same.** At least seven
distinctions split one requirement into **two shapes with different answers**, and an operator who
answers the safe shape must not be taken to have answered the dangerous one: a blend mode written as
an array versus a bare name; an inline image filtered with `LZWDecode` versus `Crypt`; a content
stream with no resources dictionary that is a page versus one that is a form XObject; an appearance
dictionary with an `/AS` entry versus without; a static XFA form versus a dynamic one; a
`/CIDToGIDMap` at a part 2 target versus a part 4 one; an unrecognised operator inside `BX`/`EX`
versus outside. `[site."…"]` keyed by requirement identifier alone cannot say any of that. **The
site key needs a shape qualifier**, in the same way RFC 0007 section 4.6 gives it a target qualifier
— and the shapes have to be enumerated by `--remedy-sites`, or an operator cannot discover that the
distinction exists.

**2. There is a fifth remedy kind and it is the one the owner asked about.** section 0.2's `supply`.
Twelve entries reach a mitigation only through it, and it is neither `discard`, `preserve` nor
`derive`: no information moves and no tool runs; a person states a fact the document does not. It
needs its own word precisely because its honesty condition is different — the report and
`xmpMM:History` must say the value came from the configuration rather than from the file.

**3. Some sites must be marked *not configurable*.** section 13.3's twenty-two are refused only
because a lossless rewrite is unwritten. If they appear in `--remedy-sites` alongside the rest, some
operator will configure a loss to get past a missing afternoon's work, and the file they archive
will be permanently worse than the one the next release would have produced. The listing needs a
standing for them — *owed*, not *stop* — and the format needs to refuse an answer for such a site.

**4. Predicates have two costs and one spelling.** section 0.2 again: `media-type = [...]` is a key
comparison, while *depart only where the transfer function is the identity* or *where every code
resolves to the same glyph* needs the document examined and sometimes rendered twice. Both are
legitimate and they belong to different risk classes; `--remedy-sites` should say which a predicate
is, and a budget should bound the computed ones.

**5. `on-failure` needs to be a chain for at least one site.** section 6's 3D case wants *transcode,
else attach the original, else drop the annotation*, and that is three answers in preference order.
RFC 0007 section 7 question 4 asks whether to allow a list and warns it is harder to reason about;
the catalogue found exactly one site that needs it and several that would use it, which argues for a
**bounded** chain — a list of remedies, no conditionals, evaluated in order.

**6. `keep-everything` needs a mechanism preference, or it needs six blocks per site.** The
profile's sentence is *prefer `preserve` wherever it exists, then `derive`, never `discard`*, and
the mechanism that preserves differs per target: attach unchanged at 4f, attach derived at 4, append
as pages anywhere. Written in RFC 0007 section 3's format that is a target-qualified block per
target per site — hundreds of lines saying one thing. **A profile needs to be able to say `prefer =
["attach", "append"]` once** and let the converter pick the first the target admits. That is not a
new mechanism in the sense RFC 0007 section 5a worried about; it is a defaulting rule, and without
it the shipped profile is unreadable.

**7. A profile file has no header, and it needs one.** RFC 0007 section 3's format is a bare table
of sites. A shipped profile has to carry its own name, a one-sentence description an operator reads
instead of the file, and the version of the catalogue it was written against — otherwise a profile
copied between teams cannot be identified, and one written against an older site list fails
confusingly rather than clearly. **This is not the same as the "coherent with" column the RFC used
to propose**: the owner removed that, correctly, because a profile answers refusals and every answer
leaves the file conforming, so no profile is incoherent with any target.

**8. Some remedies need a per-document input, which a fleet configuration cannot hold.** Decryption
needs the document's password. The format can express *what to do*, not *with which secret*, so the
site needs a documented way to say where the per-document input comes from — a command-line flag, a
sidecar file, an operator-supplied resolver — and `stop` when it is absent.

**9. `refuse-any-loss` is the empty file, and that is a problem.** A profile whose every answer is
`stop` names no site, so it is byte-for-byte indistinguishable from having no configuration at all —
and an operator who has to *show* an auditor what their pipeline was told to do cannot. A `default`
key that may only take `stop` costs nothing and makes the statement expressible;
`doc/profiles/refuse-any-loss.toml` uses one and marks it.

**10. The configuration is per site and the outcome is per object.** One document has five
attachments, and the remedy may succeed for three, derive the fourth and fail on the fifth. The
report has to be per object rather than per site, and the report is where an operator's trust in a
queue actually lives. `doc/rfc/0007` section 4.1 says stderr is captured verbatim; what is missing
is that the **result** is a list, not a verdict.

---

## 15. The four shipped profiles

`doc/profiles/` carries them, as TOML in RFC 0007 section 3's format:

| file | its sentence |
|---|---|
| `refuse-any-loss.toml` | every site `stop`; today's behaviour, stated deliberately |
| `as-if-printed.toml` | discard what printing would not have carried; touch nothing it would |
| `only-metadata-loss.toml` | nothing may be lost but metadata; everything else `stop` |
| `keep-everything.toml` | prefer `preserve`, then `derive`, never `discard`; `stop` before losing |

They cannot be tested — nothing parses them yet — so what they are is the proof that this
catalogue's answers are expressible. **Two of the four are expressible as RFC 0007 section 3
stands**, and the two that are not are the findings:

- `as-if-printed` (35 sites) and `only-metadata-loss` (10 sites) use nothing but RFC 0007 section
  3's own shape plus the site-specific keys RFC 0007 section 3 already says a site documents for
  itself.
- `refuse-any-loss` needs **one key the format does not have**, `default` — section 14's ninth
  finding — because without it the profile is the empty file.
- `keep-everything` (48 sites) needs **four things the format does not have**: `prefer`, so one line
  can say *attach where the target allows, else append*; the `supply` remedy; a bounded `on-failure`
  chain; and per-site `keep` lists. Every one of them is marked `NOT-YET-IN-FORMAT` in the file
  itself, which is a more useful artefact than a correct file nobody could maintain — written in RFC
  0007 section 3's format as it stands, it would be one target-qualified block per target per site
  and several hundred lines of repetition.

**Every profile works with all six targets**, and that is a property rather than a coincidence: a
profile answers refusals, every answer leaves the file conforming to the target asked for, and an
answer a target does not admit is `doc/rfc/0007` section 4.6's error naming both — a fact about that
answer, never about the profile.

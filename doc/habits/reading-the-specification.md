# Habits: reading the specification

Status: **standing** — method, not code.
Read by: a round about to read a clause, record what the standard says, or judge whether a
silence is one. `doc/habits/the-ledger-and-claims-about-this-tree.md` is what it writes the
reading into; `doc/errata-read.md` is what an erratum has moved.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **A clause number is an edition's, and a clause that exists in one edition may not exist in the
  other at all.** Session 1005 wrote "§8.8.2 says a conforming reader shall ignore a PostScript
  XObject" into a test and a question; the conformance gate refused it within the minute. **ISO
  32000-2 has no PostScript XObjects** — `grep -c` returns 0, and 8.8.1 says "[t]here are two types
  of external objects", image and form. They live in ISO 32000-1:2008, 8.8.2, which is the edition
  PDF/A-2 delegates to, and its sentence is stronger than the paraphrase: such fragments "shall
  have no effect either when viewing the document on-screen or when printing it to a non-PostScript
  device". **When a rule feels like common knowledge, that is the moment to check which edition
  states it** — and the `§` convention exists so that the gate can.

- **A clause's neighbour can answer a question the entry never asked.** Reading §9.9.1's *preceding*
  sentence — the TrueType tables that "shall always be present if present in the original TrueType
  font program", which lists `head`, `hhea`, `loca`, `maxp`, `cvt `, `prep`, `glyf`, `hmtx` and
  `fpgm` and does **not** list `vhea` or `vmtx` — showed that the standard declines to require those
  two tables one sentence before it forbids their use, so removing them is as lossless as restating
  them (ADR 0988). Three readings of that row had asked only *which side may be rewritten*. The
  method has now overturned four catalogue entries, and once the deciding neighbour was **in another
  standard entirely**: ISO/IEC 15444-1:2000 I.5.3.3 sets `APPROX` to zero and tells conforming
  readers to ignore it, which is why a JP2 written as that part requires is precisely the file that
  fails ISO 19005 (ADR 0982). **Read the paragraph, then the one before it.**

- **"Our vocabulary cannot express this" is a claim about the vocabulary, and it decays like a
  claim about the specification does.** Three places in this tree said a §12.4.4 transition was
  "an animation between two pages, which a display list cannot express", and it was false when it
  was written: a frame is two images, a clip apiece and an alpha — four commands of a vocabulary
  this tree has had since its first rasteriser. What a display list cannot express is *time*, and
  that is a much smaller claim, answered by handing it a fraction. The false one stood for two
  hundred and forty-three sessions and cost the feature all of them (ADR 0230).
- **A subclause is a checklist; check the code against it, not the code against itself.** §9.6.5.4
  names five routes from a code to a glyph; the code that stood in for it implemented one and a
  half — self-consistent, commented, and right about every document anyone had opened.
- **Read the whole subclause before believing the sentence that answered your question.**
  §12.7.4.3 opens by describing a processor *constructing* an appearance and closes by describing
  it *splicing* one.
- **Reading a silence is not reading the sentence it sits in — check the modal verb.** Table 175
  says a processor "**shall** provide predefined icon appearances"; three neighbouring tables say
  **should**. All four were read as one silence for a hundred and nineteen sessions. **"States no
  artwork" and "requires no artwork" are different claims**, and the question is not "may I fill
  this silence" but "does a sentence around it require me to". ADR 0109.
- **A clause can name more than one population in one sentence, and a summary will name one.**
  §11.7.4.4 governs "the B , B\* , b , and b\* operators … **and** the painting of glyphs with
  text rendering mode 2 or 6". **Where a rule lists the operators it applies to, count them against
  the code.** ADR 0110.
- **A claim that the standard is silent is a claim about the whole standard, and it is checkable.**
  Thirty-two sessions asserted no `DeviceCMYK` → RGB conversion exists; §10.4.2.5 is titled
  "Conversion from DeviceCMYK to DeviceRGB". Twice a recorded silence has been a clause four
  subclauses from one the tree cites constantly. `grep -n '^## '` the titles in `doc/md/` first.
- **"The clause says nothing" and "the clause says the opposite" are different findings, and only
  one is a licence.** Image reduction was recorded as unspecified from §8.9.5.3, which is about
  magnification; §10.7.4 says "there shall not be averaging over the pixel area". Only the second
  produces a *departure*, which must be argued and costed.
- **A departure is only honest once you have looked for the others.** One departure looks like a
  compromise; three in one subclause, all in the same direction, is a reading.
- **Where the standard defines nothing, refusing is a result.** `issue6621.pdf`'s `/Mask` is a
  one-bit greyscale image where Table 87 requires an image mask; both readings damage some file.
- **A clause read and dismissed is worth as much as one implemented**, and costs a minute against
  the 20 to 60 a review costs. **A cheap family review is where the expensive findings are** —
  clause 10 was picked because most of it was expected `inapplicable`; nineteen rows were, one was
  §10.4.2.5.
- **Where the standard defers to another document, the deferral is a citation.** §9.7.5.3 hands a
  `CMap`'s syntax to Adobe Technical Note #5014.
- **A default written in a table is not a suggestion**, and a comment arguing for a nicer one is a
  preference wearing a reason: `/MissingWidth` defaults to 0, and half an em cost `issue7439.pdf`
  six half-ems of invented space in one line.
- **A presence condition is not a restriction on meaning.** Table 115's `/CIDToGIDMap` is
  "Required for …" and then says what it *means*; reading the first as bounding the second drew a
  page as garbage. **Read what an entry's *value* means before branching on whether it is there** —
  the mirror: §12.5.6.7's `/LL` was refused on presence and its one corpus witness states `/LL 0`,
  Table 178's own "no leader lines".
- **A rule about how something is *encoded*, implemented as a rule about its value, is invisible
  forever.** §9.3.3 applies word spacing to "the single-byte character code 32", not to any code
  numerically 32.
- **Where two subclauses each condition a branch on one of two flags, the clause that defines the
  flags breaks the tie.** §9.6.5.4 cannot decide a font setting Symbolic *and* Nonsymbolic; §9.8.2
  calls the pair "a historical accident".
- **One dictionary, two clauses, and only the second says who wins.** §8.9.6 defines an image's
  `/Mask`; that an `/SMask` overrides it is in §11.6.4.3.
- **When two clauses disagree, ask which reading makes a file's own words mean nothing.** §12.5.2
  and §12.5.5 disagree about `/CA` beside an appearance stream; honouring both applies
  `highlight.pdf`'s 0.8 twice.
- **The clause can tell you two readers are one algorithm.** §9.6.2.1's NOTE 1 calls a CFF "an
  alternative, more compact but functionally equivalent representation of a Type 1 font program",
  which has now settled three design questions. **And a clause one analogy away is still the
  clause**: Table 124 forbids a `/FontFile` on a CIDFont, which answers what a *writer* may do.
- **Two callers of one clause can use disjoint halves of it.** §14.6.2 gives a property list two
  forms and §8.11 *cannot* use the inline one, so fifteen sessions proved nothing about it.
- **Ask what a feature looks like when its parameters are not their defaults.** Under `Identity-H`
  with `/CIDToGIDMap /Identity` both of §9.7's mappings collapse to nothing. **A parameter whose
  default is the unimplemented behaviour is a gap on every page in the world** (`Tk`), and **a
  default of `true` on an entry nobody implemented is a gap on every file that uses the feature**
  (Table 217's `/PreserveRB`).
- **A rule whose common case is the identity is a rule nobody tests, and the test written beside it
  will agree with it.** §7.6.4.3.2 step (a): for the *empty* password the wrong reading gives the
  same 32 bytes, so nineteen documents opened and every document with a password was refused.
- **A rule that changes nothing today can become load-bearing tomorrow.** Table 58's rule that one
  `m` overrides the previous changed no pixel until §8.5.3.2 made a single-point subpath a dot —
  then 205 unwanted dots on one page. **And a clause about the whole page can be invisible until
  one construction needs it**: §11.4.7 survived three reviews of clause 11's other families.
- **Ask what the clause requires of *this* device before deciding it is a gap.** Overprinting was
  63 documents and six `silent` rows until Table 146 was read against this device's colourants. **A
  gap sized by a corpus is a hypothesis about a clause.**
- **The standard sometimes states answers rather than rules, and those are the tests to write.**
  §12.4.2 gives no algorithm for Roman numerals — it gives nine labels beside a tree. **And a
  clause that states an algorithm can audit a corpus**: §12.3.3's `/Count` in three steps checked
  the reader against 146 producers at once.
- **§6.3.2.2 ranks what a corpus cannot.** Two gates taking the pdf.js corpus as their universe
  produce a demand curve, which cannot rank a requirement no file exercises.
- **A sentence inside a clause you have implemented can bind only a writer, and it becomes a
  requirement the day the program writes.** §7.6.3.2's "the initialization vector is a 16-byte
  random number" sat in an `implemented` row for a hundred and twenty sessions with no site in
  this tree, because a reader *reads* the vector. **After a session that gives the program a new
  verb** — writing, editing, pointing — **re-read the clauses it already claims, for the half
  addressed to the other side.** ADR 0129, and the same shape as ADR 0122's re-read of the rows
  whose reason began "this program has no".

**And what all of the above is read *through*.** These three were `doc/HANDOVER.md`'s "Things
worth knowing" until the five-hundred-and-ninety-third, and they belong here because every clause
reading in this project goes through them:

- **`doc/md/` is the specification in a form code can read** — markdown conversions of the 14
  PDFs. **This entry said "committed" and had been false since the three-hundred-and-eleventh
  session**: `.gitignore` covers `/doc/md/` and `/doc/*.pdf`, and what is tracked is the encrypted
  `doc/specifications.zip` (ADR 0187). A test may still depend on them without a skip path, but for
  the other reason — a test that cannot find them **fails loudly** rather than skipping, which is
  what the owner decided in that session.
  `ISO_32000-2_sponsored_EC3.md` is 24 MB and its `##` headings give a clause number, title and
  line range apiece, which is the whole basis of the citation checker and the ledger — `grep -c '^## '`
  is the count, and this sentence carried one that was wrong by four hundred. **Three
  caveats, each met the hard way**: it is a *conversion*, so a quotation the checker cannot find
  may be an artefact — check `doc/`'s PDF before editing the comment; one heading number
  (`14.8.4.7.3`) occurs twice; and **the conversion drops content** — Table 164's `/Di` row ends
  "Default value: 0." in the PDF and the markdown has no such line, so a reading taken from
  `doc/md/` alone would record a silence the standard does not have. `pdftotext -layout` over the
  PDF is the check, and a **table** is where to expect it. When a gate accuses the standard of a
  gap, suspect the conversion first. **It also *changes* text rather than only dropping it**, and
  the shapes are known: the hyphen of a word broken across a line disappears (`text-tospeech`,
  `implementationdependent`), and one double quotation mark comes out as `"` on one page and as
  `' … '` with spaces inside it on the next — §14.8.6.1 against §14.8.6.3 is the witness, and a
  blockquote quoting the second would have failed the gate with the standard blamed for it. That is
  why `conformance::quote::normalise` drops every quotation mark and `prose::folded` drops hyphens;
  ADR 0375. Extract spec data from here rather than writing it from
  memory; `grep -v '^!\[Image\]'` first, the files carry base64 images inline.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's note on
  black point compensation by ISO 32000's own co-project-leader, and it settled a design question
  the base specification leaves to ISO 18619 — while sitting unread as the same question was being
  answered by looking at other renderers.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding` must be
  one of three names; it does not say what those encodings contain.
- **A dependency between two owed items is a claim about *which clause governs*, and it decays like
  a ledger row.** `doc/todo/11` item 7 said its last case was blocked on item 5's seam for four
  sessions, and item 5's own text said the seam was "what §11.3.7.3 states". Both sentences named
  the wrong clause: §11.3.7.3 governs two *objects* and the blocked case is one object's subpaths,
  which §11.6.2 governs and answers the opposite way — it *forbids* the construction item 7 was
  weighing against a cost, so there was never a trade. **The tell is a dependency whose reason is a
  clause number**, because a clause number is checkable in a minute and nobody checks it: the item
  reads as settled, and the citation is doing the work of an argument. Read the cited clause's own
  subject line before inheriting a block. ADRs 0582 and 0583.
- **A paraphrase inside quotation marks does not just say something false — it stops being
  checkable, which is why `CLAUDE.md` principle 5's rule is mechanical rather than stylistic.**
  `spec-errata check` finds a quotation of text an erratum struck by *matching* it against the
  struck passage; a quotation that is nearly the clause's words matches nothing and is therefore
  invisible to the one instrument built to catch it. `viewer_core::command` quoted §7.11.4.1 as
  "the tree shall map name strings to file specifications" where the clause opened with *the
  associated name tree*, over a sentence Issue #481 struck out — and the two *accurate* copies of
  the same quotation, in `pdf_model::attachment` and `viewer_host::panel`, were both found by the
  tool within a round of the erratum being read, while this one survived three hundred and twenty
  sessions. ADR 0660. **Two consequences worth carrying**: an inline quotation in prose is checked
  by nothing at all — only rustdoc blockquotes are gated, and `--bin quotations` reads `doc/` and
  `ledger.toml` rather than `crates/` — so a phrase in quotation marks inside a sentence is where
  an invented one hides; and a quotation whose words appear in **no** clause is invisible twice
  over, because there is no struck text to match and no blockquote to verify. `pdf_syntax::tree`
  cited §7.9.6 for "by unsigned character code" for as long as the module existed, and ISO 32000-2
  prints that phrase nowhere. `grep` the phrase in `doc/md/` before putting it in quotation marks;
  it costs a second.

**A clause that hands the processor freedom may still name where the algorithm is written down —
follow the cross-reference before recording a silence.** §8.6.6.5's note said for many sessions that
combining an NChannel space's colourants is "the blending the standard leaves to the processor";
NOTE 3's own last sentence points at clause 10.8, where §10.8.3 states the algorithm in four steps.
The disposition survived (the algorithm's condition is unmet on this device) but the claim "the
standard states none" was wrong, and one cross-reference would have caught it (ADR 1193).

**An exclusion is a claim about the standard and decays like any other.** `/DS` and `/RV` were
recorded in two rows, a todo and two ADRs as "XFA rich text, excluded by principle 5"; §K.1's
permission is about schema-driven page generation, not about an entry whose format the standard
describes by pointing at XFA, and this tree already read the identical construct in Table 172's and
Table 177's `/RC`. Before believing a row that hands something to an exclusion, check what the
excluding clause actually permits, and grep the entry's own words across the tree for the same
construct read elsewhere (ADR 1197).

**A media type, a default value or a fallback a converter is about to choose is grepped for in
`doc/md/` before it is recorded as a choice.** Round 1216 wrote a paragraph arguing
`application/octet-stream` as a documented choice and then found §14.13.2 mandating exactly it,
with the `/Params` `/ModDate` answer in the same sentence (ADR 1270). `CLAUDE.md` states the rule
for recording a silence; a choice is a claim that the standard states nothing, and it decays the
same way — grep for the value, not only the clause number.

## "The standard states nothing here" is checked against the clause's own headings before anything rests on it

FDF was sent as `application/fdf` beside a comment saying the standard names no media type, and
§12.7.8.1's sentence naming `application/vnd.fdf` sits four lines under its heading (ADR 1292).
`CLAUDE.md` says this of a ledger silence; it binds a code comment the same way, and a value about to
reach a real server is where such a comment is finally read.

## A reference implementation's disagreement on packing is confirmed by a scratch build in its convention

qpdf pads every hint-table item to a byte where F.4.1 says the fields run without regard to byte
boundaries; one scratch build with padding cleared its warnings and turned a suspicion into a
measurement before the disagreement was recorded and put to the owner (ADR 1293, `doc/questions/Q131`).
The build is what makes the record a fact rather than a reading — and the owner's search then took the
padding as F.4.1's operative reading (`doc/questions/A131`, ADR 1328): a convention every known reader
agrees on is evidence about which reading of an ambiguous sentence does any work, and "the clause is
not unclear" was itself a claim that decayed.

## A brief's premise is checked in the text before anything is built on it

Batch thirty-eight's briefs got three premises wrong and three rounds caught them: a "verbatim"
quotation that was not the standard's words and an identifier (`ColorantTable`) that named Table 402's
entry rather than Table 70's (ADR 1338); a clause said to name the containing page's group when its
own noun names the imported page's (ADR 1339); a residue named as a shape channel when §11.4.3's
only reader of shape apart from opacity is a knockout group (ADR 1340). Grep the quotation in
`doc/md/`, grep the identifier in the tree, and re-read the clause's own noun before building.

## A clause that seems silent on a case is read for its ordering sentence first

§10.7.4 says nothing about a pixel two portions share, but it fixes the order of steps — all
"insideness" computations have been performed before scan conversion — so the pixel is covered by the
area of the set the fill rule declares inside, and there was nothing else to say (ADR 1341).

## What an entry can NAME is read before recording that an act is withheld or excluded

Table 207's `/F` names "the application that shall be launched or the document that shall be opened",
and the tree already opened documents named by three other `/F` entries, so a launch action naming a
PDF was the remote-documents act under the four levels, not a sandbox refusal (ADRs 1368, 1358). And a
shading type's own geometry sentence is read before its values are called "located in the region":
an axial shading "extends indefinitely perpendicular to that axis", so its sampled function holds
nothing the region alone carries (ADR 1363).

## A held text's edition is read off its own title pages, not off a catalogue

Q169 described T.801 as "Amendment 1 and Corrigenda 1 and 2 integrated"; the file's title pages say
T.801 (08/2002), © ITU 2003, with running heads "ISO/IEC 15444-2:2003 (E)", and nothing about
integration — that sentence is ITU's catalogue page's (ADR 1383). Cite what the held file says of
itself.

Before building the table list a brief gives for a font program, read the clause for the dictionary
the program sits under: §9.9.1 requires a `cmap` under a simple font and forbids one under a
`CIDFont`, and the brief for round 1294 had the `CIDFont` case backwards (ADR 1425). A brief's
table numbers are checked the same way — batch forty-five's named Tables 217 and 195–199 for what
the text prints as 221 and 197–200.

Before asking raster for new scene vocabulary for a construction that ends in a function of the
composited pixel — a group blending in a colour space of its own, a luminosity mask under a
profile's own Y — ask whether the group is isolated: §11.4.5 composites an isolated group onto a
fully transparent backdrop, so a frame of its own holds exactly its pixels, and the shared
resolution keeps both backends on one arithmetic (ADR 1471). Four pages refused "before the scene"
were one construction at two scopes.

A brief's description of a clause is checked against the heading in `doc/md/` before a fixture is
written to it: batch fifty-two's named §14.8.2.5.2 as content order (it is the sequencing of
annotations), §14.8.2.6.2 as `Alt` (it is word breaks), §14.8.6 as attributes (it is namespaces) and
put `/Version`'s precedence in §I.1 (it is §I.2); round 1337 wrote each fixture to the text and
said so.

A parameter list a brief names is checked against the tables the clause prints: a brief listed a
vendor's open-parameter names (`pagemode`, `toolbar`, `navpanes`…) as Annex O's, and Annex O's
own Tables O.3 and O.4 hold eleven (ADR 1523). Seven of the brief's seventeen are not in the
standard, and under principle 5 they were not built.

## A worked example is tested against its printed intermediates, not only its result

Guidance Note 7-2's British National Grid example prints its intermediates η and ξ and a result that
disagrees with them by 6 to 8 mm: the intermediates reproduce to every digit and give E 577274.984 m,
where the page prints 577274.99 (round 1418, ADR 1672). A test against the result alone would have
failed a correct implementation or passed a wrong one inside the tolerance it needed. Where an
example prints both, the test asserts the constants and intermediates to their printed digits, and
the result to the budget the decision records.

## "No held text states this value" is checked against the held standard's own EXAMPLEs first

§7.4.9's note said no held text stated a non-D50 Lab white point; §8.6.5.4's EXAMPLE prints D65 as
`[0.9505 1.00 1.0890]`, and the CIE publishes every T.4-coded illuminant's spectrum free (round
1436). Before a note records that a value is unheld, the standard's EXAMPLEs and NOTEs around the
subject are grepped for it — they are part of the held text, and a silence recorded over one of them
is the decay CLAUDE.md's fifth principle warns about.

## A rule obeyed on one structure and undone by a recovery over the same bytes is implemented in name only

Round 1756 was briefed to obey Table 15's `/Size` sentence by letting the objects it ignores come back
through "the repair this tree already has". The repair, `xref::rebuild`, scans the file for every
`N G obj` header and restores every object the sentence had ignored, so the sentence would have
taken effect only on the one document whose table was otherwise sound — and there it lost page one —
at a whole-file scan on sixty-six documents (ADR 2346 §4). Before a brief's premise that a recovery
will catch what a rule drops is built on, the recovery is read for what it restores: if it restores
exactly what the rule removed, the rule is obeyed in name only and the departure stands.

## A table of what a form "may not use" is read down its note column before it becomes a check

Round 1773 (ADR 2380) read XFA 3.3's table of the grammar a static form may not use (its pages 272–273)
as a list of names whose presence makes a form dynamic, and all four hybrid files in the corpora state
names it lists — `layout="tb"`, a page's `w` and `h` — which the table's notes allow where they only
express the form's logic. A recogniser built on presence alone would have refused every hybrid the
corpora hold; the specification's own cut is `occur` and `subformSet` with `/NeedsRendering` false. Before a
table of prohibitions becomes a test of presence, each row's note is read for the use it still permits.

## A requirement on fonts that are used takes its population from the fonts the content uses

Round 1799 (ADR 2433) gave the validator a narrow row for a Type 0 font's `/DescendantFonts` (§9.7.6.1's
Table 119) and first checked every font dictionary in the file; the census then refused `PDFBOX-5784.pdf`,
whose broken Type 0 font is named only in the form's `/DR` and shown by no content stream, and whose outputs
pass veraPDF at both targets. The archive parts bind fonts that are used, in any rendering mode, so the row
reads `Survey::fonts` rather than `for_each_font`. A new row under a clause whose subject is what is used takes
its population from the use, and is censused before it is called narrow.

## A family's syntaxes are listed by their top-level type before one is built

Round 1863 (ADR 2560) was briefed to enforce "the ASN.1 syntax" of a signature policy. ETSI publishes two:
TR 102 272's (RFC 3125's) and TS 119 172-3's, with different constraints under the same name. Before a
parser for "the X syntax" of a family of standards is built, the family's documents are listed by the
top-level type each defines, and the parser names which it reads and refuses the other by name.

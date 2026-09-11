# Habits: reading the specification

Status: **standing** — method, not code.
Read by: a round about to read a clause, record what the standard says, or judge whether a
silence is one. `doc/habits/the-ledger-and-claims-about-this-tree.md` is what it writes the
reading into; `doc/errata-read.md` is what an erratum has moved.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

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

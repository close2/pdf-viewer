# Habits these sessions earned

Status: **standing** — method, not code. `doc/HANDOVER.md`'s Traps are the code half.
Read by: whoever is about to read a clause, judge against another renderer, write a gate, correct
a ledger row, or take a measurement — which is most rounds, one section at a time.

`doc/HANDOVER.md`'s "Habits these sessions earned" is the pointer to this file, and lists the six
sections below so that a reader knows which one to open.

Each was paid for once. Traps are about code; these are about how to work. Every one keeps the
anchor that makes it checkable.

### Reading the specification


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

### Judging against other implementations

- **Compare the references with each other before opening a page.** Four unexplained contradicted
  pages sorted themselves into one group from a table of pairwise means.
- **A tolerance is a claim about a population, so measure the population.** A fixed bound says "two
  independent implementations of this clause are not further apart than *this* on a page of this
  kind", which is a statement the corpus can check: take every reference pair, and take each
  measure over the pairs the **other** bounds admit — a bound measured over the pairs it already
  admits returns the bound. Run over 9898 pairs it found one of eight sitting below its own
  references' spread, rejecting **29.4%** of them where its three siblings reject 0.0%, 1.2% and
  0.5%, and the sentence that claimed to derive it naming a *different* measure's number. ADR 0243.
- **Then check what the number is used for before moving it.** The same bound decided whether two
  references agree at all, so the derived value took 68 contradicted pages to 309 and emptied 457
  out of `ambiguous`. A derivation says where a number should be; it does not say the number has
  only one job.
- **Rank the suspects by a ratio, not a distance** — our worst measurement over the bound it is
  held to. Five times it has chosen the next item before an artefact was opened.
- **Before believing "one pixel out" is rounding, compare the raster sizes.** One reference put
  type a row above ours from a raster *the same size as ours*, which no disagreement about row
  counts can explain. ADR 0064.
- **`magick identify` every panel before believing any number, and the flags before that.**
  `pdftoppm` renders the **`/MediaBox`** unless told `-cropbox`; this tree, the oracle and
  `mutool draw` render the **`/CropBox`**. On `freeculture.pdf` the areas differ by 1.378, so a
  ladder taken without the flag put `poppler` at 9.10 against our 12.18 and would have
  manufactured a 34% defect on four pages that agree to 0.03 of 255. This is the twin of
  `-alpha off`, which returns exactly half the ink on a panel that carries an alpha channel;
  **both are a wrong measurement that looks like a finding**, and both now sit in
  `doc/todo/00`'s step 6 where a session reaches for the command.
- **Before trusting a clean fuzz run, ask what fraction of it got past the first branch.** The
  sfnt target ran 50 000 unseeded inputs in under a second and tested nothing: random bytes do
  not form a table directory, so every run left on the first `?`. Seeded with sixty real
  `/FontFile2` streams it produced two crashers inside a minute. A format with a magic number, a
  count and a directory needs a corpus; a content stream or a date does not. ADR 0175.
- **A rewrite driven by untrusted structure is a larger surface than a reader over the same
  bytes.** Both glyph-table repairs had been reviewed and never fuzzed, and both wrote at an
  offset a document supplied. ADR 0175.
- **A count of "marks missed" is a count of something else until you look at what they read
  back as.** 50 codes over 9 documents drew nothing and said nothing; 26 of them were one code
  of `pr12564.pdf`, and `pdftotext` reads that page as `1101#Strayer#Drive` — the code is the
  document's *space*, and having no outline is correct. The exemption that catches an ordinary
  space is "reads back as whitespace", which is blind to a font that reads a space back as `#`.
  `PDFVIEWER_TRACE_MISSING_GLYPH=1` is the trace that settled it in one run.
- **A page-level number cannot clear a mechanism of a defect that is five glyphs wide.** ADR
  0170's session A/B'd its `loca` repair against `issue7074_reduced.pdf` — ink 19.576 with the
  repair on and 19.576 with it off — and concluded the repair did not reach the page. The
  measurement was right and the inference was not: the page is three words of bold nine-point
  text and the defect was five narrow bars, under a tenth of a level. **Point the A/B at the
  quantity the hypothesis is about** — here, which glyph the space's code resolves to — which is
  one assertion rather than one render. ADR 0174.
- **A corpus can hold one document under a dozen names, and the bucket's shape lies until you
  check.** 154 of the ambiguous bucket's 678 were `tracemonkey.pdf` and eleven copies of it with
  annotations added — `pdftotext -f 9 -l 9 | md5sum` is identical across them. One measurement
  settled all 154, and the honest number to report is *one finding*.
- **When a metric accuses you, find one that measures the same thing differently.** Eight text
  pages failed on mean absolute difference and passed every other bound; the page's *total ink*
  put us within half a level of both voting references. One number from artefacts already written
  turned eight questions into one population.
- **A page that draws the same glyph twice is an instrument, and it needs no reference at all.**
  `issue7696.pdf` is 200×50 and draws four glyphs twice, 80 pixels apart. `poppler`, `mupdf` and
  `ghostscript` draw the two halves *byte-identically*; ours differ by 2893 and `hayro`'s by 3541.
  That is grid-fitting measured from the inside — the three C renderers share `FreeType` and its
  hinting, the two Rust ones place a glyph where §9.4.4's matrix puts it — and it settles a
  contradicted page without comparing anything to anybody. **Ask what a page repeats.**
- **An inconsistency inside a reference's own output outranks any distance from it.** Two
  renderers spacing one line at two different widths cannot both be reading the document's `/W`.
- **Agreement with one reference is not evidence**, and **"both readers fail the same way" is
  agreement about a symptom** — `poppler` reporting the same broken flate stream was taken as
  proof a file was damaged; both readers were deriving the same wrong key.
- **Two references against two is not a tie and not a vote — it is a question with an answer.**
  `Type3WordSpacing.pdf` splits them over a `d1` glyph's stroke colour and Table 111 settles it.
- **An unimplemented feature has a default, and the default is usually "draw it".** That is a more
  common failure of the oracle's premise than shared code.
- **Point your own instrument at their data**, and **ask the reference the same question you asked
  yourself**.
- **A test corpus has a bibliography, and it is the first step rather than an occasional one.**
  Every pdf.js file is named after the issue that introduced it —
  `issueNNNN…pdf` → `github.com/mozilla/pdf.js/issues/NNNN`, `bugNNNNNNN…pdf` →
  `bugzilla.mozilla.org/show_bug.cgi?id=NNNNNNN` — and the issue says what the file was added to
  prove. It corrected a written conclusion on the first afternoon, and §3a now turns on it.
  **A pair of fixtures with a common stem is an A/B the corpus built for you**: `issue7891_bc0`
  and `issue7891_bc1` differ in `/BC [0 0 0]` against `/BC [1 1 1]` and in nothing else.
  Two cautions. The issue describes **that reader's** defect, which may be one this tree does not
  have — pdf.js's 7891 is *ignoring* `/BC`, which `soft_mask::backdrop` reads and §11.6.5.1's
  outside-the-bounding-box rule is applied for. And an issue is evidence about a *file*, never
  about the clause: principle 5 is not suspended because a bug report is specific.
- **A corpus document can be a conformance test, and then it outranks every renderer**
  (`issue14256.pdf` draws one picture eight ways) — **or check a decoder against itself** (an LZW
  image must decode to exactly `width × height` bytes; 96 documents encode one image ninety-six
  ways). Ask **what does this file already say about itself?**
- **Look at what a corpus file is *for* before filing it under a group.**

### Tests, gates and reports

- **A test asserted through the accessor that normalises the thing being tested is not a test.**
  §7.3.7's null-entry rule was checked through `Document::get_key`, which answers `Null` for an
  absent key. **And the accessor need not be one of ours**: `Object::as_dict` answers for a
  *stream* as well as a dictionary, so "the check box still has a dictionary of states" passed
  after the states had been replaced by a stream. `matches!(x, Object::Dictionary(_))` is the
  assertion; the way it was found is the next line. ADR 0130.
- **A discriminating test has to discriminate; check by breaking the thing.**
- **A suite of shapes is a suite of shapes.** ADR 0138's equality test failed on three of eight
  cross-backend scenes and all three had a *curve* crossing the cut; a suite of rectangles would
  have passed and let the defect reach the oracle four pages later. Trap 12b asked what *size* a
  suite's scenes are and ADR 0046 what *parameter* they leave at its default — this is the same
  question a third time, about their geometry.
- **Count a suite's *cases*, not its tests.** `rasterrocket` has 1330 passing tests over 93 218
  lines and a golden-image harness whose case list is the comment "CASES is empty until fixture
  PDFs are added" — and it draws no path fill at all, silently, on a document `pdftoppm` renders.
  Ask of any suite: which of them renders the artefact the program exists to produce, and compares
  it to something? ADR 0136.
- **A constant that is right for the hand-built fixture is a landmine when a real file arrives.**
  `incremental_update.rs` replaced "object 1, the catalog", true of the file the test builds
  itself; in `bug900822.pdf` object 1 is the *encryption dictionary*, and the update wrote a
  catalog over it and produced a file no reader could open. Trap 12a's rule, one level up: take
  the identifier from the document, not from the fixture that happened to be first.
- **A test that skips silently is worse than no test.** A missing corpus is a skip; a present
  corpus that lacks what the test needs is a **panic**.
- **A gap measured on both sides is a fact; measured on one side it is an accusation.**
- **Agreement can be a shared *substitute*, and only removing the sharing shows it.** Six oracle
  pages became contradicted the session §9.6.2.2's fourteen font programs were compiled in, and
  none is a defect: `poppler`, `mupdf` and `ghostscript` resolve a non-embedded standard-14 font
  through this machine's fontconfig, so part of our agreement with them had been reading the same
  URW faces off the same disk. **Ask what data a reference reads from *this machine* before
  crediting its agreement.** ADR 0133, and it is trap 9's second shape from the inside.
- **A gate cannot ratchet what has no consumer**, and **fixing an instrument can be worth a
  feature** — one line moved 25 pages into the judged set and showed one drawing nothing.
- **A page can leave the contradicted list without a pixel moving** (the tolerance class comes from
  what *we* drew, so anything improving extraction loosens a bound — take the raster's digest
  before writing "fixed") **and can leave with pixels moving and still be wrong** (`issue20232.pdf`
  agreed once the y flip was fixed and still draws `56` where three references draw `⌀56`).
- **A page can be visibly wrong inside a verdict the gate cannot fail on**, and 45% of the judged
  set lived in `ambiguous` where nothing watched until the hundred-and-seventy-sixth session gave
  it a ratchet (§3a). The standing example was `issue7406.pdf`, which
  drew a JPEG cyan-on-black while its verdict stayed `ambiguous` — **and it is right now**,
  checked in the hundred-and-seventy-fifth by opening the artefact: all five renderers draw the
  same logo and the verdict is still `ambiguous` (mean 5.07 against a bound of 5.00). Nothing
  announced the fix, because nothing was watching then either. **A page in this bucket was
  unwatched in both directions**, so an example of it went stale as quietly as the defect did —
  which is the whole argument for the list the hundred-and-seventy-sixth session put under it.
- **A page that draws right can read back wrong, and this project's two text gates are built not
  to see it.** Both strip whitespace from the comparison, deliberately — a content stream records
  positions rather than words — so every question about *word separation* is outside them, and a
  readback nobody prints is a readback nobody checks. `issue4304.pdf` is 895 bytes named
  *Words that should have spaces between them*; its advances were fixed in the
  four-hundred-and-fifth session, the picture has been right since, and it went on reading back
  `Wordsthatshouldhavespacesbetweenthem.` for fifty-nine more. **So a round that fixes what a page
  draws asks what it reads back** — `examples/readback` is one command — because selection, search,
  `pdf-retrieve` and the screen reader take the second and no gate does. ADR 0299.
- **A report has a price, paid in gated pages.** Print what a condition matched before trusting its
  count; **measure the corpus before choosing between reporting a gap and closing it** (every
  `/Decode` array in all 974 documents is Table 88's default or its exact reversal).
- **A "not implemented" count of zero can mean "nothing reports it".** `/FontFile` was recorded at
  zero while 57 documents embedded one and drew a substitute in silence.
- **A report that arrives with a fix is worth reading twice**, and neither is a regression however
  it looks in the count.
- **Build the strong gate, then let its own output tell you it is wrong.** A table-attribution
  checker failed fourteen of twenty-five references and all fourteen were correct writing; what
  shipped asserts the weaker true thing and *prints* every cited table's title.
- **A citation nothing checks is a citation that rots**, and **a gate that reads one file format
  checks one file format** — the ledger is 823 notes about ISO 32000-2 and the citation gate read
  Rust sources, so none of it was checked. **A `§` means one document**: `RFC 3986 §5.2` is right
  about the RFC and ISO 32000-2 has a §5.2 of its own.
- **A bucket that means "we failed" must not also come to mean "you have not told us the
  password".** When a ratchet fires on a change you believe in, ask whether the *category* is wrong
  before the number.
- **A gate's numerator moves when its denominator does, and only one of those is news.**
- **A count taken at one call site is not a count.** "Parsing was never the cost" was written after
  instrumenting the pattern path, which runs once where `sh` runs 3576 times. Instrument the
  *function you are accusing*.
- **A number in this file is a claim, and attributing it is a second claim.** `calloc` was 4.5% of
  a page and this file said it was the group's pixmap; `Pixmap::new` is 0.14%. Ask
  `callgrind_annotate --tree=caller`. ADR 0103.
- **Four plausible optimisations, four counts, four refusals — and counting was cheaper than
  any of them**: 0%, 1.3%, 2.5%, and a `Vec::reserve` per show string that *cost* 0.47%.
- **A profile ages past its conclusion, and the conclusion is what survives being read.** One
  profile was carried nineteen sessions; re-measured, its largest item was *four times* the share
  recorded and the sentence beside it had named the fix correctly the whole time.
- **A ratio has two ends, and this file has quoted the wrong one.** Quote the absolute number you
  control.

### The ledger, and claims about this tree

- **When two clauses describe one mechanism, reviewing one leaves the other lying.** Four instances
  in ten sessions; the check is one `grep` for the *other* clause a family cites.
- **"This crate does not have X" is a claim about the crate, and the crate is greppable.**
  §7.6.4.3.2's row said "this crate holds no Annex D table" for a hundred and twenty-nine
  sessions, and `text_string.rs` had held the whole of Table D.3 since the ninety-second — put
  there for §7.9.2.2, a different clause, in the same crate. **A capability recorded as absent is
  worth one `grep` of your own tree before it is believed**, and the two clauses that wanted it
  had no reason to cite each other.
- **A capability recorded as blocked on a decision outlives the decision.** §9.7.5.2's row said
  vendoring the predefined `CMap`s was "a licensing decision rather than a coding one" for a
  hundred and fifty sessions. The decision was taken in the hundred-and-thirtieth and written
  into this file; the *row* never heard, because nothing fires when a stated blocker expires.
  ADR 0140, and it is ADR 0108's regular expression finding its fourth instance — the first
  where the blocker was this project's own.
- **A test that pins a refusal must be rewritten when the refusal ends, and it will not fail
  helpfully.** `a_predefined_cmap_is_refused_by_name` failed with "a predefined CMap this tree
  has no data for must be refused", which reads like a regression and was a success. Its
  replacement asserts what says the `CMap` was consulted — that a *two-byte* code comes back.
  **The same session left the same shape in a ratchet and it went unread for ten.** Session 156
  lifted six documents to 100% of `pdftotext`'s words and left all six in `TEXT_BELOW_FLOOR`, so
  the text gate has been *red since*, with a message beginning "6 document(s) no longer below the
  floor" — and two sessions of "everything re-verified" recorded the summary line the run also
  prints. **When a ratchet fires, read which direction it fired in before believing the word
  `FAILED`**, and after a session that improves a population, prune the list *in the same
  session*: the handover entry said "six fewer" and the constant did not.
- **A wrong diagnosis is a silence with a sentence in front of it.** Two documents were refused
  for "units per em is zero" for eighty sessions; both embed a `/FontFile2` whose stream is
  *short*, and `metrics()` answers zero when it cannot find `head`. The refusal was right and
  the reason was not, so nobody could act on it. **Ask of any report whether its words name a
  cause or an effect** — and the condition for the new one had to be narrowed four times, each
  time by a document that draws (trap 11 again, on a condition rather than on a count).
- **A dependency's error message can name the fix.** `Invalid sfnt version 0x74746366` sat in
  the corpus output for as long as the gate has existed; those four bytes are `ttcf`, so the
  report was saying "this is a font collection" in hexadecimal. Reading it took ninety lines and
  closed two documents. **Convert the number in a refusal you have stopped reading.** ADR 0141.
- **Run the sweeps over the source, not only over the ledger.** The ledger has a gate and the
source does not, and the two-hundred-and-twenty-first session found four claims in `crates/`
false for between forty and two hundred sessions — `pdf-model`'s own crate documentation ("[t]ext
and images are not yet drawn"), `set_dash`'s ("only the 'solid line' case is honoured for now",
the sentence from before ADR 0018), and three of `requirements::unmet`'s arms whose capability
had arrived. **The last had predicted itself**: "a session that builds a layer panel has to come
back and change `OCInteract`". A warning written where the work is does not fire either.

**A retired claim is a string, and strings are greppable.** When a session disproves a sentence
  this tree repeats, the work is done when the *sentence* is gone. "Vertical writing is refused"
  was true until session 36 and still written in four places in session 122 — a ledger row, a doc
  comment and two paragraphs of this file. ADRs 0101, 0111.
- **A prose claim about the code can be turned into a grep, and twice that has paid.** Session 118
  swept the notes for expired reasons ("while §X does not exist"); session 122 for sentences
  claiming an entry is *unread*. Twenty minutes apiece, three live findings apiece.
- **A comment that names a refusal outlives the refusal.** `appearance.rs`'s header listed
  §12.5.6.10's four text markups among things that "state no mark" for eighty sessions after the
  same file started drawing all four. A header is where a reader learns what a module refuses. ADR
  0105.
- **A stale row can understate as well as overstate, and only the overstatements have a gate.**
  Session 82 met six understating rows in one family. **A `silent` count is a *lower* bound on what
  exists.**
- **A row whose evidence is a file can be `implemented` for something the file never touches.**
  §8.7.4.5.2: fourteen tests in `shadings.rs` and not one a `/ShadingType 1`. That is what
  `FILE_ONLY_EVIDENCE_CEILING` counts.
- **A ledger note is a hypothesis the gates test, not a conclusion they inherit.** Three
  `implemented` rows claimed behaviour the code never had, each written from the clause during a
  review, each costing a visible defect, each found by the oracle.
- **A note that gives a reason gives a trigger, and nothing fires it.** "While §11.4.6 does not
  exist" expired forty-six sessions before anyone noticed. ADR 0107. **A row that names a
  *blocker* rather than a gap is the class no gate can watch** — one regular expression over the
  notes finds them in twenty minutes. ADR 0108. **The same regular expression paid again in the
  hundred-and-fifty-first**: §11.3.7.2 said a group's shape "needs §11.4.6", which the
  seventy-first session built — three sessions after the note was written, unnoticed for eighty.
  What §11.4.6 needed turned out not to be that shape at all.
- **A warning written into a ledger note before the code exists is a warning nobody reads when the
  code arrives.** §7.11.2.1's row named a defect three call sites had for as long as they existed.
  ADR 0104.
- **A feature can make a clause reachable, and nothing announces that.** Table 192's `/H`
  describes what happens when a mouse button is pressed, and until the hundred-and-thirty-second
  session nothing pressed one — implemented one session after it was noticed, ADR 0123. **After a
  session that adds a *capability* rather than a clause, re-read the rows whose notes give a
  reason beginning "this program has no".** ADR 0122.
- **An `inapplicable` row decays exactly as a `silent` one does.** §12.7.4.2's field names were
  `inapplicable` on sound reasoning until §12.6.4.11's hide action made a field name decide
  whether an annotation is drawn.
- **A ledger with a status per subclause can find a missing *component*, not only a missing
  feature.** Four rows in two clauses named one absent data structure — a name or number tree —
  which no clause review would have shown and no corpus document would have asked for.
- **A count taken over what you touched is not a count.** This file said clause 7 had no
  `unreviewed` row for six sessions, because the count was taken over the families a session had
  touched.
- **A ledger row is an entry, and an entry gets measured before it gets believed. Price the work
  before believing a reason not to do it.** `mesh_shading_empty.pdf`'s entry said for fifteen
  sessions that closing it needed a Gouraud rasteriser in both backends — true, and one shared
  raster satisfies that constraint *better*, in less code.
- **Read this project's own lists for the sentences that admit ignorance, not only the counts.**
- **Whatever this file asserts, run it once.** "Clippy clean" was claimed while eleven warnings sat
  in the tree.
- **A premise that reads like a fact does not look like a question.** "JBIG2 and JPEG 2000 have no
  memory-safe implementation" sat in `PLAN.md` as a reason, true when written and false for
  months. **Anything deferred on an external condition should carry the date it was last
  verified.**

**The six shapes a refusal takes when it has outlived its reason.** Moved here from
`doc/HANDOVER.md` in the four-hundred-and-forty-sixth, because they are read before a clause
round rather than by every round. Each is a mistake this project made, and each names the sweep
in `doc/todo/01-ledger-partial-rows.md` that would have caught it.

**A reason that names a vocabulary is the fourth of these shapes**, found in the
two-hundred-and-fifty-seventh: §12.6.3's `/Fo` and `/Bl` were owed "keyboard focus, which
`viewer-core` does not have — there is no focus model in `Command` at all, and adding one is a
vocabulary change rather than a clause". No message was needed. The clause says what happens when
an annotation receives the input focus and nothing about how it comes to, so a press inside a
widget's active area gives it — a choice, and the one every pointing interface makes. All ten of
Table 197's events are raised now. **Ask what the program already receives before adding a way to
receive it.**

**A reason that names an architecture is two reasons wearing one coat**, which the
two-hundred-and-seventeenth session found: §12.5.3's `NoZoom` and `NoRotate` were both refused
because they "make an appearance's placement depend on the view, which a resolution-independent
display list cannot express". `NoRotate` depends on §7.7.3.3's `/Rotate`, which is in the *file* —
it was never a view-dependence at all — and `NoZoom`'s real cost is one flag on the interpretation
and a re-read of 51 documents out of 974 (ADR 0168). **Split a refusal into one claim per entry
before believing it.**

**A capability makes clauses reachable, and nothing announces it.** The ten sessions from the
hundred-and-sixty-sixth closed four clauses without anybody picking them off a list: §12.3.3
because a panel existed to display an outline in, §14.3.3 because a panel existed to display
`/Info` in, §7.7.2's `/PageMode` and §12.6.3's trigger events because a sidebar and a pointer had
arrived. Each of those rows said some version of *this program has no ___*, and each stayed true
for between seven and forty-one sessions after it stopped being true. The three sweeps that catch
it are in `doc/todo/01-ledger-partial-rows.md`, the hundred-and-ninety-first session found a
`shall` that had been binding for fifty-six, the two-hundred-and-first found the longest one
yet — §12.3.2.1's magnification and window position, owed since the **hundred-and-thirty-second**
session put scrolling and zoom in the vocabulary, still explained by "a window with scrolling and
zoom, which this program does not have" sixty-nine sessions later (ADR 0162) — and the
two-hundred-and-fourth found the same row family's other half, §12.6.3's four page-scoped trigger
events blocked on "a page-visibility model a one-page-at-a-time window does not have", which is
what a window that turns pages is (ADR 0164).

**And the two-hundred-and-fifty-third and -fourth found the inverse, which no sweep was asking
for: a capability that reached the crate implementing the clause and never reached the program.**
§12.5.6.19's `/H` was `implemented`, argued in ADR 0123, tested with pixels — and `viewer-core`
took the annotation under the pointer from `link_at`, which returns a `/Subtype /Link` and nothing
else, so no host could press a widget for a hundred and fifteen sessions. **The question the
sweeps do not ask is "the model implements this — who calls it?"** Widening the region then turned
a latent default into a wrong pixel in the same sitting: §12.5.6.19's `/H` defaults to `I`, two
tables define the entry and no others do, and a `Square` had been one caller away from inverting
under the cursor. ADR 0177. **The sweep that asks it is `doc/todo/01`'s fifth** — every `pub fn`
in `pdf-model`, grepped against the two host-side crates — and it found §8.11.4.3's `/ListMode`
on its first run, read into `OptionalContent::list_mode` and asked by nothing with a layer panel
on the screen (ADR 0178).

**And the two-hundred-and-fourteenth found a row that would have survived the capability arriving.**
§14.9.3 said `/TU` "names a field in a user interface this program does not have" — false since
the hundred-and-thirty-second — but the window was never the blocker: `Query::FieldAt` answered
with one string, and §14.9.3's `shall` needs two, because the name that *addresses* a field is not
the name a person is shown. **Ask what the program would have to say to obey the clause, not only
what it would have to have** (ADR 0167).

**And a sixth, from the four-hundred-and-fifty-ninth, which is what the *corrections* to these
rows leave behind.** §12.5.6.4's note retired its own refusal by naming the capability that
arrived — "the popup window `/Open` selects … is drawn since the three-hundred-and-twelfth
session" — and it is true about the window and says nothing about `/Open`, which was read nowhere
in the tree. `crate::appearance`'s comment beside it had the same shape: "`/Open` is not read …
this program draws no popup for any subtype", correct when written and expired by the same session.
A row in this state passes every sweep above, because it names no blocker, no missing vocabulary
and no absent architecture — it names a *capability that exists*. **When a note says a clause was
closed by a capability arriving, grep for the entry rather than for the capability**: the
capability is what somebody built, and the entry is what nobody went back to wire to it. ADR 0294.
**`doc/todo/01`'s fifteenth sweep is the instrument**, built in the four-hundred-and-sixtieth: it
reads no reason at all, takes the entries the *clause's own tables* state, and asks both whether
any source file names each and whether the row's own `code = [...]` files do. The second question
is the one that matters — `/Open` was named in `crates/`, by another table's reader — and its first
run found §12.5.6.15's required `/FS`, disposed of by "not a rendering question" while a document
that attached its file to a page carried a file nothing here could reach. ADR 0295.

### Measuring

- **A negative answer from a tool is a claim about that tool, not about the world.** `which
  cargo-fuzz` reports nothing here because `~/.cargo/bin` is not on `PATH`, and two consecutive
  rounds wrote "`cargo-fuzz` is not installed on this machine" into an ADR and a todo — leaving a
  fuzz target unwritten — while the binary had been on disk since 26 July and `doc/environment.md`
  said how to invoke it. The check that would have cost two more seconds is
  `ls ~/.cargo/bin`. Session 428, ADR 0264.
- **Ask the linker what a binary can execute.** `nm <binary> | grep -c <symbol>` answers "could
  this program ever run that code, under any input" with no fuzzing, no coverage build and no
  argument — it found that eleven of thirteen fuzz targets did not contain the function that
  crashed, which is a stronger statement than any of them could have made about *reaching* it.
- **Wall-clock benchmarks lie under load; count instructions instead.** One change measured as a
  24% regression and an 8.5% improvement twenty minutes apart. **A/B in one sitting**, and measure
  the baseline on this machine rather than trusting a number in this file.
- **Take the *before* half with a patch file, never with `git stash`.** The stash stack belongs to
  the clone and not to the worktree, so every parallel round pushes onto the same one: a neighbour's
  `git stash` landing between a round's own push and pop makes the pop apply *their* diff and lets
  theirs take yours. That happened in the five-hundred-and-twenty-third session and cost a recovery
  out of `git fsck --unreachable`, whose dangling `WIP on <worktree-branch>` commits are what a lost
  round is found in. `git diff > round.patch`, `git checkout HEAD -- <files>`, measure,
  `git apply round.patch` — three commands that touch nothing outside the worktree.
- **Pin the pool before counting a serial change in a program that has one.** Callgrind counts
  every thread, so a work-stealing pool's *spin* is in the total and it is not deterministic.
  `open_one` on two corpus pages read **+0.154%** and **+0.010%** for a change that removes an
  allocation and can cost nothing; the diff was `crossbeam_deque::Stealer::steal` +1.24 M and
  +5.11 M, and with `RAYON_NUM_THREADS=1` both pages read −0.003%. This is the converse of
  `doc/performance.md`'s older rule — *quote the clock for a parallel change and the counter for a
  serial one* — and the converse is the direction that produces a phantom regression rather than a
  phantom win. Session 500, ADR 0335.
- **Attribute a regression by removing the suspect, not by reading the profile.** The profile shows
  the *shape* of the extra work, not its cause; one stubbed field said 96 of 110 M.
- **A gate's own printed timings are only as good as what else is in its process.** The oracle's
  `processor time` and `slowest pages` rows read a factor of two high for thirty-nine rounds,
  because a second test in the same binary was walking the same corpus under `rayon` beside it, and
  the report is built from per-page `Instant` spans. One page read 93.2 s where the same work alone
  is 5.4–6.7 s, and a todo quoted the inflated row as evidence for the regression it was a symptom
  of. Nothing could see it, because a self-timing gate has no second reading to disagree with.
  Session 447, ADR 0282.
- **Check that the cheap probe moves the right way before bisecting on it.** Cheap and *sensitive
  to the thing you are looking for* are different properties. `doc/todo/43` named the corpus gate
  as the probe for a slowdown because it "takes seconds"; it is **faster** at the bad end than at
  the good one, so a round following the instruction would have concluded there was nothing to
  find. One sample at each end of the window, before the first bisect step, costs a minute.
- **When a page's error has a suspiciously round size, do the arithmetic.** Seven pixels of
  gradient where there should be an edge is 1800 ÷ 256.
- **Profile before believing an explanation, even one whose arithmetic matches.** A 48-second page
  was attributed to clip masks with `3576 × 485 kB = 1.7 GB`, exactly the memory held and silent
  about the time: callgrind put the masks under 4% and the gradient at 78.9%.
- **A suspiciously clean measurement is a reason to check the instrument.** Four callgrind numbers
  flat to four significant figures meant the benchmark was panicking and callgrind was faithfully
  counting the panic. **Second instance, session 161**: our ink measured at exactly half the three
  C renderers' on ten pages running, 2.00 to three significant figures, which is not what hinting
  does. Our renders and `hayro`'s carry an alpha channel and the three C ones do not, and
  `magick -colorspace Gray` was averaging alpha in as a fourth channel — so **the tell was that
  the two renderers agreeing with us were the two whose output *format* matched ours**. Ask what
  the agreeing group has in common besides the answer.
- **A lesson recorded where it was learned and not where it is *used* has not been recorded.**
  The paragraph above was written in session 161, in this file and in `CONTRADICTED_GLYPH_EDGES`.
  The recipe in `doc/todo/00-ambiguous-bucket.md` — the file a session opens when it goes hunting
  — still carried the broken command, and sessions 197 and 199 followed it and drew the same
  wrong conclusion twice, with the same 2.00 ratio in front of them. Both ADRs carry the
  correction now and the recipe is fixed. **When a habit lands, ask which document a person will
  be holding when they need it.** ADR 0163.
- **Look at the heatmap's shape before opening anything else.** Twelve of the oracle's fourteen
  unexplained pages were diagnosed in two sessions without a debugger: a heatmap that is the
  whole silhouette says colour, one that is glyph outlines says grid-fitting, and the ink table
  then says which. Both are three minutes per page against a list that had not moved in twenty
  sessions.
- **Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's 85
  seconds as the price of having an oracle; 95% was three programs re-answering a question.
- **Measure before optimising, and delete what does not measure.** A `FontRef` cache changed a dense
  page by less than noise and was removed with the reason recorded; the same session's real win was
  hoisting a string allocation, 1.37 ms → 18 µs.
- **An eager lookup on a cold path is a hot-path cost when the path runs per object.** Reading
  `/AcroForm` per constructed appearance was 2.7× the whole feature's cost.
- **A cost written down beside one call is not a cost anybody adds up.** `Pages::index_of`'s doc
  comment says it is a search that cannot skip a subtree and names the two callers it was written
  for; a third arrived, called it *in a loop over 988 outline items*, and inherited the comment's
  blessing without its argument — 344 ms of every page turn. **Ask of any function documented as
  expensive: who calls it in a loop.** One `grep`, and it found a second (`named_page`). ADR 0124.
- **A failed frame must not be reported as a drawn one.** `viewer-ui` answered
  `Rendered::Presented` when its GPU path refused a page, so the core recorded the page as shown,
  never asked again, and the window kept the *previous* page under a title bar naming the new one
  — a page a person cannot view and no reason given. It now answers `Rendered::Failed`, draws it
  on the CPU backend instead (which is what `CLAUDE.md` keeps that backend for), and says which.
  **And a refusal is recorded as an answer**: the scheduler must not re-ask a question whose
  answer cannot change, or the two spin. ADR 0125.
- **A performance defect on a path no gate walks is found by a person using the program.** The
  corpus interprets page one, the oracle renders pages it is handed by index, and neither turns a
  page. The largest document this project owns — ISO 32000-2, 1023 pages, committed in `doc/` —
  was in no gate at all until session 141 made it two tests.
- **Look at what a safe idiom compiles to in a loop that runs per pixel.** `.round()` on a clamped
  float is a library call — 205 M instructions on one page, 10.7%.
- **The exact fix is often available and is usually better than the approximate one.** A memo keyed
  on the input tuple beat an interpolated lookup grid: 3249 M → 1075 M, and simpler.
- **A change made for correctness that is also an order of magnitude faster means the old code was
  doing work that was worse than useless.** One mesh raster replaced 4096 flat pieces: 35.47 G →
  3.08 G, and closer to the references.
- **When the first design of a fix is the obviously safe one, still measure it.** Refusing to cache
  timeouts is unarguable in principle and left two pages accounting for 46 of 57 seconds.
- **A ratio measured on four pages is a fact about four pages.** ADR 0137 counted 1.01–1.13 strips
  touched per command and concluded duplication was not the problem, which is true of those four;
  `issue12841_reduced.pdf` is *two* commands each covering the page, so sixteen strips replay both
  sixteen times. Computing the same ratio per page is one function and is what made the split safe
  to ship. **Ask of any measured constant whether it is a property of the thing or of the sample.**
  ADR 0139.
- **A function only an example calls is a function nobody has measured.** `command_extents` rebuilt
  every command's clip chain from the leaf: 606 ms on one page, six times its whole rasterisation,
  correct and unnoticed for two sessions. **Before moving code onto a path a person waits on, time
  it there.**
- **A priced item names a loop, and the loop it names may not be the one the file takes.** The
  handover priced "colour-managing an image in parallel" for thirty sessions on
  `issue19971.pdf`'s photograph. `image::unpack` is the per-sample conversion, it is obviously
  the loop, and a JPEG does not enter it: `zune-jpeg` writes components into the raster and
  `convert_channels` converts that in place afterwards. Parallelising the obvious one measured as
  noise. **Before optimising a named function, check on the named file that it runs** — one
  `callgrind_annotate` would have said so before the change rather than after. ADR 0147.
- **Ask what a parallel unit's answer depends on before asking how to divide it.** A colour
  conversion is a function of one pixel's samples, so a band boundary changes which conversions
  are *repeated* and never which answer is given, and the split is byte-exact at any band size.
  A rasterisation is not a function of one row's geometry — a curve clipped by a strip's edge is
  re-parameterised — which is the whole of ADR 0138. The two look like the same problem and are
  not.
- **A serial pass over every pixel is what bounds a parallel render**, and it hides inside a
  function whose cost nobody attributed: `impose_on_medium` was 7.8 ms of a 17 ms page, all of it
  eight integer divisions per *transparent* pixel — and §11.4.7's isolated page group makes most of
  a page transparent. Amdahl's law names where to look after any successful division. ADR 0139.

- **A check deferred for cost belongs wherever the cost is already being paid, and nothing tells
  you when that place appears.** Table 45's `/CheckSum` was read and not verified for eighty-three
  sessions, on a reason that was true — "checking would mean inflating every attachment" — and
  that expired the moment one path decoded one stream. The clause names where it belongs:
  "the checksum of the bytes of the **uncompressed** embedded file". **After a session that
  makes something decoded, decompressed or laid out for the first time, re-read the entries whose
  reason for being unread was that nobody had it yet.** ADR 0145, and it is ADR 0108's regular
  expression looking for a different kind of blocker: not "needs §X" but "would cost too much
  here".

- **The three sweeps found a fourth shape in the hundred-and-ninety-first, and it is the
  strongest one yet: a row whose "this program has no ___" was about a *verb*.** §12.8.6 said
  a usage-rights signature grants "features of a PDF processor that are not available by
  default" and that "this program has no feature behind such a gate"; §12.8.2.3 said the same.
  Both were true when written and both stopped being true in the hundred-and-thirty-fifth and
  -sixth sessions, when this program learned to fill in a field and save the file — which are
  exactly the rights Table 258 grants and exactly the changes Table 257's `/P` restricts. And
  the requirement was not a new one: §12.8.2.2.1 has always carried, in a parenthesis, "(These
  changes to the document shall also be prevented if the signature dictionary is referred from
  the DocMDP entry in the permissions dictionary.)" A `shall`, addressed to a processor that
  modifies, unread for fifty-six sessions after this one became one. `ViewState::set_field` now
  refuses at `/P` 1 and permits at 2 and 3, and §12.8.2.3's `should` — remove a UR signature
  the modification exceeds — is named as owed. **After a session that gives the program a verb,
  the rows to re-read are the ones whose reason is about what the program *is*, not only about
  what a clause needs.**

- **Sweep for the reason's *shape*, not for its clause.** Sessions 118 and 122 grepped the
  ledger's notes for "while §X does not exist" and for entries claimed unread. The
  hundred-and-seventy-fourth grepped for a third shape — "this program has no ___", "no panel",
  "which this is not" — over `partial`, `reported` **and** `inapplicable` rows, and found
  §12.6.3 saying "[n]othing raises an event … this crate has no events", which stopped being
  true in the hundred-and-thirty-second when `Command::Pointer` landed. Forty-one sessions. The
  three sweeps are twenty lines of Python apiece and each has paid on its first run.

- **An `inapplicable` row whose reason is "this program has no ___" is a row waiting for a
  session that gives the program one.** §14.3.3 was `inapplicable` because "a viewer with a
  document-properties panel would read it; this one has no panel", and the panel arrived seven
  sessions before anybody re-read the row. That is the second instance after §12.7.4.2's field
  names, and the trigger is ADR 0122's: **after a session that adds a capability, sweep the rows
  whose reason begins "this program has no"** — `inapplicable` as well as `partial`, which the
  earlier sweeps did not cover.

- **Read the whole sentence a feature is built from, and count what the other half is worth
  before deciding.** §12.3.3 says a click makes a processor "jump to a destination **or trigger
  an action**"; the hundred-and-sixty-sixth session built the jump and shipped a `Command`
  variant shaped exactly like half a sentence. Two sessions later the count — 281 corpus outline
  items with an `/A`, 32 of them not a go-to — said the other half was one refactor away, and the
  variant became a path nobody takes and was removed. **A command shaped like half a clause is a
  command that will be replaced**, and the habit is ADR 0110's one level up: where a rule lists
  what it applies to, count them against the code *before* designing the interface.

- **A gate that cannot see a surface is a gate that cannot see a surface.** The corpus
  interprets page one, the oracle rasterises pages it is handed, the text gate reads words and
  the date gate reads strings — not one of them opens a viewer, so every line of chrome this
  project draws is unwatched by all four. `viewer-ui/tests/panel.rs` answers it the only way that
  discriminates: rasterise the panel's own display list with `render-cpu` and *count ink*, then
  delete the glyph drawing and check the count goes to zero. A test that asserted the display
  list held the right number of commands would have passed with every glyph missing.

**And one measurement that was wrong because of *when* it was taken.** §9.10.2's last-resort
permission applied to simple fonts was measured, found to cost `pr4922.pdf` its whole readback, and
dropped — two rounds before the round that removed the interaction. Re-measured after it, the same
code is free and lifts two documents off the floor. **A measurement is a measurement of the tree as
it stands**, and a rule refused on one round's evidence is worth re-measuring after the round that
changes what it touches.

### Code, bounds and dependencies

- **A gap inside a feature you have implemented does not announce itself.** Every missing
  *subsystem* reports, because whoever decided not to build it wrote the report. **A fast path
  inherits none of the rules of the path it skips.**
- **A "nothing here" is data, and dropping it is not the same as recording it.** §7.5's free
  entries and §7.5.8.3's unknown entry types both say an object number names nothing; both were
  *skipped*, so the question fell through to an older section and the reader resurrected objects
  its own file had deleted. **Ask what a `continue`, a dropped branch or an unmatched arm hands the
  question *to*.** ADR 0100.
- **A refusal is not a repair, and the difference is invisible from inside the function that
  refuses.** `Document::load` would not hand back an object whose header named a different number,
  which is right — returning object 2's bytes under number 3 corrupts the graph silently. What it
  handed the question to was the page-tree walk, which found object 3 was not a `/Type /Page`,
  skipped it, and returned the *next* kid: `issue7229.pdf`'s page one was its page two for the
  project's whole life, with `Pages::len()` answering 2 from `/Count` and `get(1)` answering
  `None`. **Every correct local refusal is a question passed upwards, and the caller may answer it
  by drawing something.** ADR 0148, and it is ADR 0100 one level along.
- **A partial repair can be worse than none.** The first version of that fix recovered *in-use*
  entries one at a time and left the misfiled free entry standing, so the page's image became a
  deletion and the page drew **nothing** where it had drawn the wrong page. A displacement is a
  property of the subsection; repairing half of one is a new file nobody wrote. Ask what class the
  defect belongs to before choosing the granularity of the fix.
- **The archetype is the `d` operator.** Every layer of dashing existed and one line read only the
  *empty* array, so not one dashed line in 974 documents. When a feature looks finished, check the
  operand path from the content stream to the state. **A feature switched off in one place is
  switched off everywhere it is not switched on**, and **a clause whose operators are implemented
  can still be unread** (`J`/`j`/`M` from the first commit; Table 57's `/LC`/`/LJ`/`/ML` for
  twenty-three sessions).
- **A lookup table with a deliberate many-to-one entry has no inverse, and reading one backwards
  fails in the direction nothing checks.** `Pages::indices` answers *object → index* and holds an
  entry for an intermediate `/Pages` node as well as for each page, because a destination may name
  a node — its own doc comment says so. Three call sites wanted *index → object* and got it by
  scanning the map for the first matching value, so on a document whose node has the lower object
  number the answer was a node that is not a page: every Table 355 `/Pg` comparison failed and page
  one of ten tagged documents, ISO 14289-1 among them, told a screen reader the page has no
  structure. **The map answered, the answer was well formed, and it named the wrong kind of
  object** — which is why no test and no report saw it for the whole life of the code. Ask what a
  map's entries *mean* before reading one the other way, and take the identity from whatever states
  it (`Page::id`, here). ADR 0342.
- **A cache that reports a perfect hit rate can still be missing.** `render-cpu`'s mask cache
  answered every one of the 303 lookups page 6 made and built 303 identical page-wide masks,
  because the key was the leaf's `ClipId` — a *name* — and the page states one region. **Instrument
  the count of distinct keys, not the hit rate**: a hit rate is a statement about the lookups you
  made, never about the ones you should have made. ADR 0132, and it is ADR 0115 with the sign
  reversed — that key was too weak, this one too strong, and both ask whether the key is what the
  claim is about.
- **A count of what is *shared* is not a count of what can be *reused*.** 5933 fills of 107
  outlines said a coverage cache would hit 55 times over; the outlines are shared through an
  `Arc` and the coverage is not shared at all, because the sub-pixel phase the count left out is
  what a coverage bitmap depends on. **Ask what the cache's key would have to be before believing
  the count.** ADR 0131.
- **A cache is a claim that two things are the same, and the currency of the claim is the key.**
  The font cache said it in the weakest one available — a resource name, which §7.8.3 scopes to the
  dictionary that defines it — and handed a form `XObject`'s `/F1` the page's glyphs for
  thirty-one sessions. Every other cache keys on object identity. ADR 0115.
- **A display list holding the right commands can still draw nothing, and no report will say so.**
  A type 5 mesh was complete, correct and 180 points from where it belonged. Between "we could not
  build it" and "we drew it" there is a third state only the oracle catches.
- **A representation can forbid a correct answer.** No evenly spaced array of colours can express a
  discontinuity. Ask what a data structure *cannot say*.
- **A file's extension is a claim, and the bytes decide.** PDFium ships the standard 14's Foxit
  faces as `.pfb` and every one of them begins `01 00 04 02`, which is a CFF header and not
  PostScript. Four lines of `xxd` settled what a module comment would have got wrong. ADR 0133.
- **A dependency's refusal can be silent *and* size-dependent.** `tiny-skia` insets the clip by a
  pixel before hairline stroking and returns early when the inset is empty, so a hairline stroke
  into a target under three rows tall draws nothing and reports nothing. Found by a test that had
  passed for a hundred and fifty sessions failing at one of its three scales — trap 12b's question
  ("what *size* is every case in this suite?") arriving from the other direction. ADR 0139.
- **A probe is a suite, and a suite of one shape proves one shape.** ADR 0138 split a page with a
  cubic in it and concluded "a clipped line is the same line"; a quadrilateral took ten minutes and
  said otherwise, which moved the rule and doubled what it permits. ADR 0139.
- **A parser that recognises a delimiter without parsing it will be read as parsing it.**
- **An operator that is matched and ignored may still be a rule.** `BX`/`EX` sat with `MP`/`DP` for
  thirty-one sessions; §7.8.2 makes them the one place an unrecognised operator is not an error.
- **Where a clause states arithmetic exactly, two independent implementations are worth more than
  one shared one** — trap 2 sends a device *decision* to the shared crate; §11.3.5.3's formulas are
  the other kind. **Two rasterisers disagreeing is information; two agreeing is not proof.**
- **An assumption a test cannot exercise is not tested, however many tests run over it.** The GPU
  backend demultiplied Vello's output for fifteen sessions; every scene rendered onto an opaque
  background.
- **Ask which arm of your own enum no test has ever taken.** `Rendered` has a variant for a
  tier-1 host and one for a tier-2 host; twelve tests played tier 1 and the tier-2 path asked
  for the same frame for ever. The variant existed, the doc comment explained it, and nothing had
  ever sent it. ADR 0117.
- **A number computed to fit must be checked against the rounding of whatever consumes it.** A
  page fitted to a window by `viewport / extent` is one pixel too tall about half the time,
  because `TargetSpec::for_page` rounds a raster *up* to contain the page and the nearest `f32`
  to the exact ratio is above it as often as below — a fitted page with a scrollbar. The fix is
  not an epsilon: step to the next representable scale until the consumer's rounding lands. ADR
  0116.
- **Two copies of a constant is one defect waiting.**
- **A constant that is a property of the state must reach every paint, including the ones that
  replace the colour.** A shading replaces the current colour, and the line that returned it
  dropped `ca`.
- **A clamp is a decision.** `width.max(0.0)` reads as hygiene and was this program's whole answer
  to a value §8.4.3.2 forbids. Ask what a `max`, `clamp` or `unwrap_or` *decides*.
- **A fallback that fills the page is worse than one that leaves it blank.** "If nothing else
  matched, the code is the glyph index" drew `v 0' ' W` for `What's an interval?`. **What makes a
  fallback legitimate is where the answer comes from, and it is measurable**: §9.10.2's permission
  is taken by asking the *program* what it drew, and the readback rose 96.5% → 97.8% with **no
  document moving the other way**. A fallback that invents text lowers a score somewhere.
- **An optional entry must not erase what the clause states**, which is now four ADRs: a line
  ending (0106), an `/Encoding` name Table 112 does not permit (0111), a `/DA` font `/DR` lacks
  (0112), a missing `/BBox` or `/Rect` (0113, 0114). **And a stand-in may not fall short** — the
  first version of ADR 0112 drew six dots of Arabic punctuation on an otherwise empty page.
- **A shortcut right on the common case is worse than one wrong on all of them.** The Cal-space
  pass-through was nearly correct for `/Gamma 2.2` and badly wrong otherwise, and nothing
  distinguishes the two at runtime.
- **Silent caps are defects, not safety**, and **a bound written for the pathological case can
  refuse a reasonable one** — the bound belongs on the *growth*.
- **A panic in a dependency is a symptom, not a diagnosis**, especially where its arithmetic is
  modular. **Being right for the wrong reason is worse than being wrong.**
- **A dependency can be doing the thing your architecture forbids.** Trap 6 has said since the
  sixth session that `ColourSpace::to_rgb` is the only place a colour becomes RGB, and
  `zune-jpeg` was converting every four-component codestream to RGB with a formula of its own —
  reachable by any `DeviceCMYK` JPEG, invisible to `colour_paths.rs` because every fixture there
  states its samples as hex rather than as a codestream. **Ask of each dependency which of your
  own invariants it is in a position to break**, and write the fixture in the form the dependency
  actually sees. ADR 0149.
- **A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
  `DCTDecode`, `skrifa` font parsing, `flate2` Flate, `tiny-skia` rasterisation. ADR 0014. **A
  dependency can implement more of a specification than the clause cites** — `read_fonts::ps::agl`
  gives the Adobe Glyph List *and* its specification's algorithm. **Look in `read-fonts` before
  writing font-format code**: an earlier handover specified ~80 lines of CFF charset parsing that
  already existed. ADR 0006.
- **The interesting half of a "viewer feature" is usually a clause.** Of the click that follows a
  link, the mouse is four lines and the rest is Table 176's three conditions, §12.5.2's coordinate
  space and §7.7.3.3's rotation.

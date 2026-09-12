# Habits: tests, gates and reports

Status: **standing** — method, not code.
Read by: a round that writes a test, adds or changes a gate, or adds a report.
`doc/traps/instruments-and-reports.md` is the code half, and `doc/todo/02-every-round.md` §2 is
which gates a change actually needs.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **A non-exhaustive `_ => true` arm in a predicate is a report that fires on every variant added
  after it.** `command_blends` fell to `_ => true` for `Command::Shaped`, so any `/I false` knockout
  group drawn on transparency with a stated element reported a blend nothing carried, since ADR 0234
  (ADR 1009). A predicate over an enum names every variant or it is not a predicate.

- **A test that scans sources for a marker reads its own source.** `editions.rs`'s sweep for `§`
  failed on its own comment's `§14`, then on its own table's `§12.11`, then the conformance gate
  failed on its `'§'` char literals — three self-matches before it measured anything else (ADR
  1010). Spell the marker as an escape in the scanner, and exclude the scanner's own file by name
  and say so.

- **A priced follow-up names a population by the shape of its argument; open the witness's display
  list before taking it.** ADR 1000 priced `issue18032.pdf` as "a form knockout group whose elements
  share an affine mode"; its elements were a nested group under a non-separable mode and a group at
  `ca 0`, and the construction that drew it was a different one (ADR 1009). The pricing was right
  about the cost and wrong about the shape, and only the display list could say so.

- **When a new ranked source of meaning is added, re-run the one-conversion test with that source
  stated.** `colour_paths.rs`'s first test guards "one conversion, every route", under the
  parameters its fixture names. A source added later — a default colour space, an output intent, a
  blending space — is a parameter the fixture does not name, and every route agrees under its
  *absence* exactly as they did before it existed; the test stays green while `k` and `cs … scn`
  give two colours on a page with an output intent (ADR 1001). The question is not "does the test
  still pass" but **"which call sites read the new source"**, which is one grep for the field's name.

- **A build error naming a crate this round may not touch is a neighbour mid-edit, not a defect.**
  Wait and re-run before doing anything else; never "just fix" an unclosed brace or a missing item
  in somebody else's slice, and never conclude from one red run that a gate is broken. Four rounds
  in one batch hit this and all four were right to report rather than repair (ADR 0992). The
  converse is the rule that makes it safe: say so in the report, so the round that merges knows
  which failures were transient and which were not.

- **A `--all` or `--workspace` *writing* command is not safe in a tree with a neighbour in it.**
  `cargo fmt --all` rewrites every file it does not like, across every crate, including the ones a
  parallel round is halfway through editing — so a round that runs it to tidy its own change can
  silently reformat somebody else's unfinished work, and the neighbour then cannot tell its own
  edit from the rewrite. `cargo fmt --all --check` is fine; it writes nothing. The rule is the
  *write*, not the flag: scope a formatting run to the packages the round touched
  (`cargo fmt -p <crate>`), and keep the unscoped form for the `--check` that gates the commit.
  Found by the nine-hundred-and-sixty-fifth session, which verified by mtime that it had rewritten
  none of its neighbours' files — a check that only works *after* the fact, and only because it was
  thought of. This is the writing half of trap 23's reading half.

- **An instrument consulted through a truncation is not consulted**, and the failures are not
  exotic. `tools/state.sh quick` was read by grepping its output for the words "fail" and "error"
  rather than for its **exit status**, and reported clean for two days while it exited 101;
  `cargo fmt --check` was read through `head -5`, with the round's own diffs below the cut and a
  concurrent round's file above them. Both are the same mistake as trap 1 one directory over: the
  instrument that says a change happened is not the change. Read a gate's exit status, and read
  all of its output or none of it.
- **A stale binary in the shared build directory answers for a tree that no longer exists.** Four
  agents in one week lost time to this and none of the four suspected it first, because the
  failures look like findings: `cargo test -p conformance` reported this tree as holding more than
  one corpus, and reported the standard as unreadable, from test binaries compiled in worktrees
  that had since been removed — `CARGO_MANIFEST_DIR` is baked in at compile time and the workspace
  shares one `CARGO_TARGET_DIR`. A gate result that contradicts something you can see with `ls` is
  a stale artefact until proven otherwise; `touch` the source and rebuild before believing it.
  The same shape reaches the `--profile gates` worker, which `doc/todo/02` §5 rebuilds every fifth
  round precisely because it goes quietly out of date.
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

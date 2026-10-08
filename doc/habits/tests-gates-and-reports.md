# Habits: tests, gates and reports

Status: **standing** — method, not code.
Read by: a round that writes a test, adds or changes a gate, or adds a report.
`doc/traps/instruments-and-reports.md` is the code half, and `doc/todo/02-every-round.md` §2 is
which gates a change actually needs.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **Staging a batch from `git status --short` has two traps, and both bite in a worktree.** A
  rename prints as `R  old -> new`, so a pipeline that strips the status column hands `git add` one
  path with an arrow in it and **aborts on the first one** — the commit then captures only what was
  already in the index, silently. And a worktree whose gitignored resources are symlinks into the
  main checkout prints its submodules as `T ` (typechange): staging those would commit a symlink
  over a tracked submodule and take the corpus with it. Use `git add -- .`, then read
  `git status --short | grep -vE '^[AMD] '` before committing and unstage every `T `. Session 1005
  did both in one commit and caught them by reading the staged count against the file count.

- **And `git status` cannot see the submodule half at all, which is how session 1019 committed it
  anyway** — having run exactly the check the bullet above prescribes, and got an empty answer. In a
  worktree whose resources were symlinked in, the *index* already records mode `120000` for those
  paths, so the index and the working tree agree and there is nothing for `git status` to report.
  The typechange sits between the index and the **committed tree**, and it surfaces only when the
  branch is diffed against a commit that still holds the gitlinks — which, for a batch branch, is
  the `git merge --ff-only` into `main`, long after the commit is written. `git status` is the wrong
  instrument; the right one names the population from `.gitmodules` and asks the index what mode it
  holds:

  ```sh
  git config -f .gitmodules --get-regexp path | awk '{print $2}' |
    while read -r p; do [ "$(git ls-files -s -- "$p" | cut -d' ' -f1)" = 160000 ] ||
      echo "NOT A GITLINK: $p"; done
  ```

  Run it before committing in a worktree. The repair, if it is already committed, is
  `git update-index --cacheinfo 160000,<sha>,<path>` for each path with the sha read from the
  parent commit, then `--amend`; the working tree is then restored with `git submodule update
  --init`, which re-clones whatever `.git/modules` no longer holds. Session 1019 lost the working
  copies of seven submodules this way — the symlinks the checkout wrote pointed at their own paths,
  so following one went nowhere — and all seven came back from their remotes.

  The general shape is worth more than the commands: **a check that reads the working tree cannot
  see a claim that lives in the index**, and the two staging traps above are both of that kind.

- **On a shared machine, stop a run by its pid, never by a pattern.** `pkill -f 'cargo …'` written
  to stop one round's own gate sequence matched that round's shell as well, and would have matched
  any sibling's `cargo` had the pattern been a word wider (session 997). A background sequence
  started with `setsid nohup script.sh &` has a pid the shell's `$!` does **not** report — that is
  the wrapper's — so record `pgrep -f "bash .*script.sh"` at launch and kill or watch *that*. The
  merge of rounds 992–997 lost three monitors to the same mistake before it wrote this down (ADR
  1016's round and `doc/history/998`).

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
  that had since been removed — `CARGO_MANIFEST_DIR` is baked in at compile time and the
  worktrees then shared one build directory. A gate result that contradicts something you can see with `ls` is
  a stale artefact until proven otherwise; `touch` the source and rebuild before believing it.
  The same shape reaches the `--profile gates` worker, which `doc/todo/02` section 2's tier-2 block
  builds on its first line and `tools/batch.sh gates` on its `build-sandbox` line, ahead of every
  gate that spawns it, precisely because it goes quietly out of date.
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
- **And what the *machine* lacks is a skip too, said out loud, asked of the code rather than of a
  message.** §9.7.4.2 leaves a composite font with no embedded program reachable only by
  character, so `pdf-font` draws it from an `sfnt` face this machine offers and never from the
  compiled-in fourteen — and a fixture of that shape measures the machine's font collection on
  every assertion it makes. Twelve tests in eight binaries failed for that reason and no other when
  the font directories were replaced by an empty `tmpfs`, and two of them were CI's red pushes. So a
  test that needs a face asks `LoadedFont::machine_offers_a_substitute`, which runs the search the
  load runs, and prints a sentence when the answer is no; a predicate reading the refusal's
  *words* instead would swallow the day the code under test started refusing for a different
  reason. Two of the twelve were not machine dependencies at all but defects the machine had been
  hiding — a corpus population that admitted a substitute as though the producer had embedded it,
  and an inequality stronger than §9.2.4 states. ADR 1154.
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
  Rust sources, so none of it was checked. **A `§` means one document**: a section sign after
  `RFC 3986` is right about the RFC and ISO 32000-2 has a §5.2 of its own.
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

## A hand-built fixture whose stream content is itself a file needs a cross-reference table

`Document::open` recovers a table-less file by scanning for `obj`, so an FDF, PDF or portfolio
embedded inside a fixture's stream is found as the *outer* file's objects and the reader returns a
plausible, wrong document; the test then fails on a count with no hint why. Three embedded-FDF
fixtures failed that way and cost forty minutes (ADR 1185). The moment a fixture carries another
file in a stream, write the xref.

## A fixture that plants a resource the page never draws through tests the exemption, not the rule

ISO 19005-2 section 6.2.2 binds only resources the content stream references, so a `tests/archive.rs`
fixture stating two `Separation` arrays while its content draws through one is a silent pass: the
second is an unreferenced named resource and outside the requirement's population. When a fixture
plants a resource for a rule to bite on, its content stream references every copy it plants — and a
new fixture whose requirement does not fail is the first thing to suspect (ADR 1188).

## A gate takes the whole ladder an ADR measured, and prints the population it holds

`stroke_width.rs` held axis-aligned rules only, at eight times the recorded worst, so ADR 1082's
forty-seven-fold improvement on diagonals went unreported for 108 sessions and ADR 0848's stale
0.1802 stayed in a ledger row as "the worst this device produces". When an ADR measures a ladder,
the gate holds every rung of it, its tolerance is the measurement plus a stated slack, and it prints
the population so a reader can see which half is under it (ADRs 1189, 1201).

## A resource bound the clause forbids is measured against the population it bounds

`MAX_FIELD_ANCESTRY = 32` carried the reason "a depth no legitimate form approaches" and sat exactly
on the deepest producer-written chain on the disk, refusing 25 real widgets at its own value. The
instrument that measures a bound walks past it — the census now goes to 1024 — or it cannot see the
far side (ADR 1198).

## A calibration's plant goes into the function, never into a file of a shared tree

`flags::calibrate` feeds a synthetic unaccepted flag through the same attribution paths the sweep
uses, so the plant runs on every gate run and there is nothing to remember to remove; the flag is
assembled from two pieces so the calibrating crate's own source does not carry the defect its sweep
reports (ADR 1213). On a worktree six rounds share, a plant written into a file is a file somebody
else may be editing — and a whole-file `cp` restore of it is the move the brief forbids.

## An instrument that counts what is owed is calibrated against a site where nothing is owed

`Configuration::unbuilt` counted a `discard` as not carried out unless the loss table named the
site; a site that costs nothing is in no loss table, so four answers were reported owed for two
batches, and the mismatch column already printing the evidence was read as a wrinkle rather than a
defect (ADR 1233). Trap 13 plants a defect when a sweep comes back clean; this is its other half —
plant an absence and confirm the count stays quiet.

## A one-sided timing premise decays when the code under it gets faster

`a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw` waited two seconds against a
draw ADR 0650 measured at 27.6 s; ADR 1082's scan converter and the rectangle fast path brought
that draw to 0.61 s, and the test failed alone and passed only under a neighbour's load for several
batches. A wait that establishes "there was something to interrupt" is a ratchet in disguise: its
constant states the measured figure it is a fraction of, and a round that speeds the path it waits
on re-measures it.

## A generated binary fixture's stated provenance is a claim to check, not a recipe to trust

`JPX_TWELVE_BIT`'s doc comment gave the command that made it and said no test read its value; the
command's endianness was wrong, so the fixture carried saturated white rather than the 3000 it
claimed, unnoticed because nothing read it (ADR 1242). A fixture whose content no assertion names
either gains one or says in its comment what it decodes to, measured.

## A gate whose right-hand side is a shared navigational document goes red on a neighbour's work

The frontier-map gate (ADR 1250) was built in a six-round batch and had to be reconciled four times
against rows other rounds were still moving; the last reconciliation is only as good as the moment it
ran. Land such a gate on a boundary round, or hand the merge the one command that re-derives it —
here `cargo test -p conformance --test conformance the_frontier_map`, run first at the merge.

## A correction's explanation sits beside the quotation it retires

`spec-errata applied` marks a site from a 400-character window either side of the quotation, so
`write.rs`'s `startxref` — whose erratum paragraph was correct and complete — read like a site that
had never heard of Issue #101 because the paragraph sat further down (ADR 1262). Move the paragraph,
never widen the window: the window is also what stops an unrelated "struck" three screens away from
excusing a stale quotation.

## A fixture for a construction carries what the construction uses

The `/NeedAppearances` fixture failed three times for reasons that were not the site under test: the
constructed appearance renders a font, so the file needs it embedded with widths inside ISO 19005's
tolerance; it paints in `DeviceGray`, so the page must already fail the output-intent row or nothing
is prepared for it; and a builder that appends objects while a dictionary names one by number
silently re-points the reference (ADR 1257).

## A named-population gate list is keyed by what does not move

`tests/names.rs`'s `STANDING` was keyed `path:line` and failed within the hour on a sibling's edit
above one of the comments — a false failure whose only cure a later round would find is regenerating
the list, which is how a named population degrades into a bare count (ADR 1273). Key such a list by
the thing the finding is about, the file and the name, and let the run print the position; the
frontier map and the oracle's page lists are keyed by name for the same reason.

## A host feature is driven under Xvfb with more than one document before it is called done

Round 1213's tabs compiled and passed their tests in all three windows and were broken in all three:
a tab behind the front never drew, Ctrl + Tab did nothing in GTK, one switch looped forever
(ADR 1275). No gate drives a window's event handling; driving it is the instrument. And `rustfmt` on
a binary root formats every `#[path]` child module, siblings' files included — format module files
one by one.

## A quorra dialogue under Xvfb takes a key only after `windowfocus --sync`

A bare `xdotool key` did not answer the ask card; `xdotool windowfocus --sync <id>` then
`key --window <id>` did (round 1227). A driven test that "could not press Enter" is an instrument
finding before it is a defect.

## A refusal is proved on a neutered copy before the live command runs

Before `tools/batch.sh close` gained its guard, its refusal was proved against the live worktree with
a copy whose `worktree remove`, `branch -D` and `prune` were replaced by `echo` (round 1238): the
refusal fired, exit 1, and nothing could have been removed had it not. Any command that deletes is
tried that way first.

## When driving tabs, the document that states a page mode is opened second as well as first

Four of round 1233's ten defects hid behind one ordering: a second document's `/PageMode` was obeyed
while it was behind, taking a running presentation's full screen (ADR 1303). Open the document with
`FullScreen` or `UseThumbs` second, and drive Ctrl + Tab both ways.

## A test helper that reads a PDF's tail searches bytes, never text

`support::linearized::startxref` read the file's tail as UTF-8 and a cross-reference stream's binary
rows can end within a few bytes of `startxref`; the fixtures hid it and the corpus found it
(ADR 1309).

## A per-pair sweep listing does not clear the file

`--bin cited` prints one line per (clause, file) pair, so fixing the listed line left four more `§`
literals in the same files (round 1244). Grep the whole file for the same shape before counting it
done.

## A host that reacts inside the toolkit's resize callback asks for a new layout from the idle queue

GTK laid the window out once at launch and not again until the first key, so page one's first
transition was rasterised at the pre-full-screen size (ADR 1316). And the first drive is one document
alone: a second document's arrival forces the layout that hides the defect, which is why ADR 1303
saw the jump and could not trace it.

## Never plant-and-restore with `cp` on a file a sibling is editing

A whole-file copy back over a shared file carries the sibling's hunks only by luck (round 1243 got
lucky once). The safe form is the reverse `sed` of the one line planted.

## A defect found by reading is driven to before it is fixed

The wrong-document password retry round 1239 read in the hosts was real, and no GoToR target could
reach it: the core declined an encrypted remote file before any prompt (ADR 1332). Fixing only the
host would have left the prompt missing entirely. Drive the path to the reading first; the fix is
then to what actually happens.

## A read-list entry is a claim that someone read the note against every decision the sweep prints

`overtaken::READ` names 61 notes as read; each may only be added by reading the note against every
later decision the sweep lists for it, and a note whose correction is a NUMBER goes into a named
re-measure population instead (ADR 1355). Under Xvfb, `windowfocus --sync` the viewer's window before
every key: a click on the tab strip does not give keyboard focus (round 1254).

## A gitignored data directory in the shared worktree is checked for being a symlink before writing

`fuzz/corpus` and `corpus-cache` point into the owner's read-only checkout; a round wrote three seeds
there by mistake and removed them (round 1264). `ls -la` first; seeds go under the round's scratch for
the merge to place. And when a host stops placing a control, check the controls-signature comparison:
a field that is in the signature but never placed rebuilds the controls every frame (ADR 1357).

## A press is driven with `mousedown`, a mid-press screenshot and `mouseup`

Both of round 1266's defects — the down appearance dropped by a `Moved` after the press, a drag at
the press point selecting text so the button's action never ran — lived between press and release,
where a plain click shows nothing (ADR 1370).

## A held gate population is checked for a published answer before it is accepted

Eleven pages were held for a password the corpus itself publishes in pdf.js's manifest; nine now
compare, and the references had to be handed the same password or the page only moves from one held
bucket to another (ADR 1377). Before accepting a population held because a clause asks for a person's
answer, look for the answer where the corpus keeps it.

## A whole class of standing false positive gets a rung that prints its members, never a skip list

The owner's uncommitted answer files and a `scratchpad/r<n>` template stood as four false pointer
findings until each became a rung of its own with a reason (ADR 1379); plant a member that must stay
a finding (`A999` with no `Q`) so the rung cannot swallow a real gap.

## A drive is judged in every window, because hosts name the same gesture differently

GTK reports a held move as `Moved`, winit as `Dragged`; a rule written for one message name passed one
host and failed the other (ADR 1382's round). And never `pkill -f` a pattern that appears in your own
command line — it killed a round's shell once; kill by `pgrep -x <binary>`'s pid.

## When a change alters which documents a gate opens, that gate's walk runs in the same round

The one password table gained a row every private copy had lacked, so `issue21579.pdf` joined every
walk, and `save_round_trip` would have failed at the merge on it had the round not run the walk and
examined the name first (round 1277). Read each name that joins or leaves a named list before editing
the list.

## A rasteriser workaround that reshapes a ramp gets a fixture with a hard stop on a whole device row

A compression of the ramp's stops by 1/2000 was invisible at 1× on a centred axis and three or more
rows off at 8× (ADR 1387); the fixture is the stripe page's shape, expected rows from the file in f64,
at every scale.

## A population a round has just read member by member becomes a named population in that round

The corpus gate's 61 incomplete documents sat behind `MAX_INCOMPLETE = 61`; read one by one, they are
`INCOMPLETE: [&str; 61]` grouped by deciding mechanism with the clause beside each group, and a swap of
one document for another now fails by name (ADR 1401).

## A new reading that changes what an older fixture means changes that fixture's assertion in the same pass

M.9.2.7 read by type turned round 1273's Colour-Group fixture into a finding; the assertion was
changed and the ADR names it, rather than weakening the new check to keep the old test green (ADR 1399).
And a script's rendering is judged in each toolkit separately: GTK drew a CJK label and Qt drew boxes on
the same machine because eleven fonts share the family name "Droid Sans" (round 1278) — `fc-list
:lang=<script>` before blaming the program. A fork's helper script gets a name of its own; a shared one
was overwritten mid-run and silently dropped two forks' batches (round 1283).

**A rank over a population that grows is written as a comparison with a named member, or deferred to
the command that ranks.** "The tightest limit this bucket has measured" was written three times in
`oracle.rs` for three different numbers, and one more page overtook all of them the day it was added;
no row points back at a superlative, so it goes false silently. `tools/superlatives.py` lists them
(ADR 1427). And before a fuzz campaign is spent on a target, read its `INITED` coverage against a few
freshly generated seeds: `forms_data`'s 1 344 seeds held no FDF file and `display_list`'s corpus
predated its wire format, and a seed count could not tell (ADR 1423).

**What a round leaves on the owner's disk is named by a command in `doc/environment.md`'s "After a
merge" section, never only in its record**, and `tools/state.sh main-checkout` prints what that
checkout still owes (ADR 1440). And a crate or binary rename is followed by `cargo test -p
conformance`: the ledger checker now sweeps note prose for program names, and ten rows had carried
`render-quorra` since the rename (round 1301).

**A subsetter is held by its outlines through the tree's own interpreter, then once over every face
the machine has.** Every kept glyph of a CFF subset is drawn through `pdf_font::cff::draw` and
compared byte for byte with the whole face's; then all 77 `CFF ` faces on the machine were subset to
every glyph and to three, which exercised the subroutine-reachability walk against every charstring a
real face holds, as no hand-built fixture could (ADR 1449).

**A test held to a dependency's pending patch asks the dependency directly as well as through the
tree's guard**, so the day the owner bumps the `rev` the test fails with the instruction to delete
the guard (`t88_conformance.rs`, ADR 1459). And a figure about a shared cache starts from its size
against its ceiling before its hit rate is read: sccache sat at its 50 GiB ceiling evicting by age,
which made an all-targets build read 918 misses against 11 hits (ADR 1463).

**A sweep's rule "X is prescribed by document Y" is restricted to the line shapes where Y prescribes
X.** The commands gate's first draft took every `check`/`build` line in `doc/verify.md` as a profile
prescription and flagged three packages that run under `--release` legitimately (ADR 1475).

**A drive verdict needs a positive observable, calibrated against a broken input.** The Arabic
find step judged by the absence of "not in this document", which the window never prints; every
step now needs a found signal, a pixel count, a title, an AT-SPI value or a golden, and each new
threshold was set by breaking the input on purpose (an erased digit gives 78 differing pixels
against a threshold of 40, ADR 1478).

**A record states its gates with a figure**, in a paragraph opening `**Gates.**` — nineteen records
between 1284 and 1326 state none and three say "see the report", which is not in the tree;
`records.rs` holds the rule from 1327 on (ADR 1499). And a script that splices a file at an anchor
checks the anchor appears exactly once in the whole file: a record's own prose mentioning
`**Gates.**` truncated it to nine lines until `wc -l` caught it.

**A ratchet that reads a page list reads it from the one file that owns it.** The oracle counted
836 ambiguous pages and its groups held 810: the 26 were first pages `corpus.rs`'s `INCOMPLETE`
names, and the oracle now reads that list with `include_str!` so the reason a page is short lives
in one place and the two counts agree (ADR 1522). And a ranges-into-the-readback that nobody
checked for years surfaced the day earlier bytes shifted: spans recorded inside an `/ActualText`
replacement had always pointed at stale bytes (ADR 1515 §5) — a range into a buffer is held by a
test that moves the buffer.

**A change under `raster/crates/raster-gpu/src/` runs every example `ci.yml` names with `--check`,
under Xvfb, behind the lock.** Nothing in the merge recipe ran them, so their premises went stale
until the owner's CI failed on two of them (`outline_upload`'s witness, `retained`'s cold-frame
signature; ADR 1563). The merge runs them as `tools/batch.sh gates`' `t2-raster_examples` line, and
the round that touches `raster-gpu` runs the same loop, `tools/batch.sh raster-examples`, behind the
lock and says so in its record (ADR 1575).

**`/dev/stdout` is compared with `test -ef` in the shell itself, never inside `$(…)`**: inside a
command substitution it is the substitution's own pipe, so `gates()`'s check that its summary was not
about to overwrite the log always answered no, and every gates log began with the summary line over
`build-sandbox`'s (ADR 1625 fixed it). A check about where output goes runs where the output goes.

**When a sibling's mid-edit keeps the shared build broken for long, test your own files in a
private worktree** — `git worktree add --detach <dir> HEAD` with its own `CARGO_TARGET_DIR` and the
gitignored data linked in (`doc/arlington-pdf-model`, `doc/md`), then `git worktree remove` it and
delete its build directory before you report (round 1399 lost no time to a sibling's clippy error
this way; the private tree is never a crate under the shared one — trap 114).

## A tab or list-tag layout change is looked at with right-to-left text and a tag that needs two faces

Every Latin fixture of round 1412's list tags passed while a CJK tag's full stop was drawing as a box:
the tag was set by two faces and the second was never looked at. A change to tab stops, leaders or
list tags is rendered and looked at (trap 1) with a right-to-left paragraph and a tag whose glyphs
come from two faces, not with Latin text alone.

## A rich fixture's `/Contents` is the rich string's text exactly as it is read

A drive fixture whose `/RC` held a tab had a `/Contents` that spelled the tab as nothing, so the two
texts differed, `/Contents` won (ADR 1635), the window showed plain text and the step found no colour
at all (round 1415, step 57). A fixture that states both carries in `/Contents` exactly the text the
rich string reads to, character for character, or the window is right to ignore the rich string.

## A sentence that is "said" somewhere is located before it is moved

The popup's tab-leader sentence lived in `pdf-model`, so when the leader began to cross to the
hosts and the sentence left `pdf-model`, the C ABI — which had been handed that sentence from there —
would have stopped saying it without anyone noticing (round 1421, ADR 1679). Before a report moves,
every reader of it is found, and each keeps or replaces the sentence on purpose.

## A todo's Status line is read before an item from it is briefed

Batch sixty-six's clause slot carried "`doc/todo/22`'s edges list, one item, if time remains"; the
file's own Status line says done, nothing owed (round 1424). A brief's optional item costs one line
to check and a slot's attention to carry.

## A sweep's held population is mapped from the record, not from an ADR's summary of it

ADR 1680's map from each round to the ADR it wrote named one ADR for round 911, which wrote two, and
the first rewrite made from the map put the wrong number into `pdf-vfs/tests/confined.rs` (round
1431 caught it). Before a citation is rewritten from a map, the map's entry is checked against the
record itself: `grep -l 'Session <n>' doc/adr/` names every ADR a round wrote.

## A scratch copy of a drive script is regenerated after every fixture edit

Round 1439 iterated the drive from a scratch copy of `tools/drive-windows.sh` and read one
iteration's figures off fixtures the copy had been written before, so the numbers were stale and the
step looked wrong for a reason that did not exist. A copy made to iterate on is remade after each edit
to the fixtures it embeds, or the drive is run from the tree's own script.

## A parallel transform's pieces are taken by the name the report gives them, never by position

`split` writes its pieces in parallel and its sinks keep outputs in the order they were opened, so
`foreign_corpus`'s `first_output` compared chapter two's parent tree with the source's page 1 whenever
the machine was loaded enough to reorder the opens — the gate's "`mutool` short answer under load"
story was never the cause (round 1443, ADR 1718). A test that needs one piece of a multi-output plan
takes it by the name the report gives (`first_written`), and a flake blamed on another program is
read against the test's own indexing first.

## A sweep that reads a command reads it up to its comment, and a planted comment calibrates it

Four sweeps (`sandbox_gates.rs`, `state_sections.rs`, `ratchets.rs`, `bounded.rs`'s `state_walks`)
read `-p` and `--test` off the whole of a gate line, comment included, so a comment that named another
package made a sweep fail by name (round 1448); they now read the command's words up to the first `#`,
as the shell does, each with a test whose planted comment names another package and target (round
1454, ADR 1744). A new reader of a command line in `tools/conformance/tests/` uses `command_words`
and carries the same plant.

## A drive step judged by the saved file is also looked at, zoomed, for the chrome it cannot see

Step 67 passed on the saved text while `quorra` drew no caret at all — the caret overlay lay under
the popup windows — and only the photograph, zoomed to the note, showed it (round 1451, ADR 1739). A
step whose verdict is a file's bytes says nothing about what the window drew; its photograph is
looked at as trap 1 asks, and a step that draws chrome gets a verdict on the pixels as well.

## The whole drive runs after any change to keyboard focus

Steps 68 to 70 passed alone while the whole drive failed GTK's steps 64 and 67: a new rule that took
the keyboard off a field on a page press also took it off the note editor the same press had just
opened (round 1458, ADR 1752). A step run by itself cannot see what its rule does to another step's
keyboard; after a focus rule changes, the drive runs whole before the record is written.

## A measurement patch is applied and reversed as a patch file, never by hand

A census that counts members or instructions is a patch on an example that must leave the tree
as it found it; round 1463 kept its counting patch under `scratchpad/r1463/census-members.patch`,
applied it with `git apply` and reversed it with `git apply -R`, so the reversal was exact and
`git diff` on the example read empty (ADR 1762). An edit made by hand and undone by hand is how a
counting line stays in a tree (trap 127's "before" build is the same shape).

## A test that needs work in progress holds it open at something it owns, never at a timed wait

The interrupt test asserted a ten-thousand-fill draw was still unfinished after a wait that had been
cut from seconds to 200 ms as the machine and the rasteriser got faster; the draw finished in 0.4 s
in release, a 2× margin a faster machine would close (round 1473, ADR 1780). The draw now holds at a
mark the test owns and waits on a gate, so the interrupt has something to interrupt on any machine. A
wait chosen against a measured duration decays with the code under it; a hold the test owns does not.

## A forced step is read against the windows the whole drive runs it in

`tools/drive-windows.sh --step` runs the named steps in every window, including `quorra-confined`,
which draws no popup; the whole drive never offers the note steps there, so a `--step` run of them
reports two `wrong` that the full drive would report as `not offered` (round 1469). Before a forced
step's `wrong` is called a defect, check which windows the whole drive runs that step in.

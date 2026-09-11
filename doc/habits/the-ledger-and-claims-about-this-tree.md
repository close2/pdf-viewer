# Habits: the ledger, and claims about this tree

Status: **standing** — method, not code.
Read by: a round that writes or corrects a ledger row, a doc comment, a todo file or a reason —
anything this tree says about itself. `doc/ledger-and-claims.md` is where a false row hides and
`doc/todo/01-ledger-partial-rows.md` holds the sweeps as commands.

`doc/habits.md` is the index of the six, and it states what a habit is and what each keeps.

- **A reason that says a row is "one predicate away" is a claim about the *clause* as well as about
  the tree, and the clause half is the one nobody re-reads.** The route is greppable and so it gets
  checked; the reading sits in a licensed document and so it does not. Two signature rows were
  recorded as one predicate away by two separate ADRs, and neither was: the annex's "entire file" is
  about the moment of signing, so every signature but the newest in an incrementally updated file
  reads as uncovered — and a predicate written before that sentence was settled would have failed
  conforming documents on *our* reading (ADR 0986). **When a reason prices work, check what it
  prices the work against.**

- **A ledger note that is boilerplate multiplies whatever is wrong with it by the size of the clause
  family.** One paragraph written once became 22 copies of a retired quotation of `CLAUDE.md` in a
  single sitting, and they outlived the sentence they quoted by 92 sessions (ADR 0989). The
  quotation gate could not see them because it reports quotations that match *a specification* and
  then diverge — and a quotation of this project's own documents matches nothing, which is
  indistinguishable from not being a quotation at all.

- **Naming a module is not naming a writer.** A catalogue entry said a rewrite waited on
  `pdf_font::restate`, "which rewrites an sfnt's `hmtx`" — true of the module's subject, false of
  its code: the splice, the directory update and the checksums live in `sfnt.rs`. **Three rounds
  were blocked on the wrong file name** (ADR 0988). An entry naming where work belongs is a claim
  about the tree and decays exactly like a claim about the standard.

- **An instrument's stated limit is a claim, and a claim about what *cannot* be done is the most
  expensive kind to inherit.** A limit written into an instrument is read by every later round as
  settled, because it looks like the work of somebody who tried — so nobody tries again, and the
  claim never meets the evidence that would move it. Two in one session (ADR 0981):
  `crates/pdf-archive/src/coverage.rs` said a sentence-level audit "cannot be committed to this
  tree at all", citing a licence that forbids reproducing the standard's **words** — while the
  audit needs a count, a mapping and this crate's *own paraphrase*, which `Requirement::asks` had
  been all along; and `conformance/no-deprecated-features` said "the route to it is in this tree",
  citing a field whose use needs a resolver that is not. **Read both halves of such a claim — what
  it says, and what it cites for it.** They are routinely about different things, and the gap is
  where the work is.

- **A constraint on what this round *can do* decays exactly like a claim about the standard, and it
  is cheaper to test than to reason about.** The eight-hundred-and-eighty-seventh session wrote "there
  is no network access from this account" into ADR 0820 and settled a step of §7.6.4.3.3 from a reading
  because of it. There is network access, and always was: DNS, TLS and HTTPS all answer from the shell,
  and the five minutes it took to find that out produced the Adobe supplement that defines revision 5,
  Apache PDFBox's handler, and a `/R` 5 document with a **published owner password** — the one thing
  that round recorded as unobtainable. **An environmental limit a round is about to reason from is one
  command away from being checked**, and a round that reasons from it instead spends its whole budget
  inside a constraint that does not exist. ADR 0829.
- **A superlative is a claim with no anchor, and the round that falsifies it is the next round doing
  the same work.** Six bullets of `doc/todo/02` §4 each opened "and it is the newest" — one per
  sweep, each written by the round that built that sweep — and five were false, with `doc/todo/01`
  carrying the same phrase five more times. No instrument could see it: the right-hand side is the
  *neighbouring paragraph*, which is nothing's population. Checking an ordinal ("the twenty-second
  sweep") is one `grep` against the catalogue; checking "the newest" means reading every sibling,
  which is the check nobody runs. **So write the ordinal and the pointer, never *newest*, *latest*
  or *the only one*** — and the same goes for a count of a list written beside the list ("five notes
  bind here", over ten bullets; "the four things a round has got wrong", over five). ADR 0882.
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
- **And a claim a sweep counts is a string too, so a correction has to leave it findable.** The
  other edge of the habit above, and the six-hundred-and-eighty-sixth session paid it: `doc/todo/01`'s
  sixteenth sweep finds a row by the words "no corpus document", and two corrections written in this
  project's house style — deleting the sentence outright in one row, quoting it as `[n]o corpus
  document …` in the other — took both rows out of the *population* instead of moving them across
  it. The count fell and nothing had been found. **Where a sweep's own population is a phrase, a
  correction states the retired claim in words the phrase matches**, which is also the honest shape:
  a row that says what it used to say is a row a reader can check. ADR 0523.
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

**A quotation in a ledger note is checked by a *report*, never by a gate, so a paraphrase wearing
quotation marks survives there exactly as long as nobody reads the report.** `cargo test -p
conformance` verifies the rustdoc blockquotes in `crates/`; `cargo run -p conformance --bin
quotations` is what reads `ledger.toml`'s notes and `doc/`'s prose, and it *prints* rather than
fails. Two anchors, both found by somebody finally reading it: §7.6.4.1's row recorded two hosts
quoting a `should` as a `shall` for two hundred and eighty-five sessions, and twenty-three Annex F
rows justified themselves with a phrase — "conforming file" — that occurs nowhere in ISO 32000-2
and is ISO 32000-1's vocabulary (ADR 0984). **A note that argues from the standard owes the
standard's own sentence**, and the check that it is one is a command a round has to run rather
than a gate that runs itself.

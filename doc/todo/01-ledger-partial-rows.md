# Read the ledger's `partial` rows against the code

Status: **standing task, and how much of it is left is a command rather than a figure here** —
`git blame --line-porcelain doc/conformance/ledger.toml`, ordered by the commit that last wrote each
`note = ` line, against `cargo run -p conformance --bin ledger` for the population. This line
carried "~48 of the 244" for a long run of rounds and the figure could only ever be a round's own
arithmetic; what it stands for is the practice below. Bands went in the three-hundred-and-seventy-fifth,
-eighty-seventh, -ninety-fourth, the four-hundred-and-second, -thirteenth, -twenty-ninth, -thirty-seventh
and **-forty-second**. **The reading list is `git blame` since the four-hundred-and-forty-second**:
order the rows by the commit that last wrote each `note = ` line, and the ones nothing has touched
are the ones nothing has read — 40 of the 248 were older than commit 110 of 590, and **fourteen of
the 32** read off the top of that list were wrong, with a fifteenth found beside them — the
`implemented` neighbour one of the fourteen deferred to. **The six-hundred-and-sixteenth found that
the bands are not a floor**: rows still sat well below the last band's, none with a
read-and-kept sentence, and two of the four it read off the bottom of that list were wrong.
**The six-hundred-and-twentieth read the band it named** — five of eight wrong, two of the five a
*status* rather than a note — and found two things about the instrument itself: a commit index is a
property of the base rather than of the ledger, so bands are quoted as **ranks** now; and the rule
for choosing within a band is to read the row whose stated reason is a claim about this codebase
rather than about the standard (ADR 0455).
**The six-hundred-and-thirty-second found both of its defects in the *settled* half of the
vocabulary** — a `partial` whose own note argues it `implemented`, and an `inapplicable` resting on
an account of the requirement that is not the clause's — which is the half no sweep reads, because
a claim that nothing is owed has no missing thing to grep for (ADR 0465).
**Twenty-two sweeps** — nineteen of them here, one over the corpus (ADR 0405), one
in `tools/spec-errata`, where the errata are (ADR 0426), and **the twenty-second built in the
seven-hundred-and-sixty-ninth, the only one whose right-hand side is the workspace's own
membership** (ADR 0709). **The eighteenth was built in
the six-hundred-and-forty-fifth**, the only one that reads no source at all: a parent row's claim
against its own children's denials, `--bin overstated`, ADR 0475, **whose mirror was measured and
declined in the six-hundred-and-fifty-second** — 14 denied term-mentions over 170 parent rows, 3
contradicted, all three noise, and it would not have printed that round's own §9.8 because a denial
generalises where an assertion enumerates (ADR 0481) — **seventeen of them are committed
programs and run every round**, one was run once and declined
(ADR 0265), a fourteenth built in the four-hundred-and-thirty-seventh, and **the fifteenth built
in the four-hundred-and-sixtieth** — the first that ignores what a row says and asks instead who
reads the entries the clause states (ADR 0295), and **the first of the fifteen to be a committed
program rather than a description**, `cargo run --release -p conformance --bin entries` since the
four-hundred-and-eighty-fourth (ADR 0319), with the second sweep following it as `--bin unread`
in the four-hundred-and-eighty-ninth (ADR 0324) and the first as `--bin blockers` in the
five-hundred-and-first (ADR 0336) — a `partial` row
whose note names nothing owed, which breaks the ledger's own definition of the status. Its first
run printed 16 rows and seven were defects, two of them statuses: §9.3, `partial` on an expired
reason for **365 sessions**, which the four-hundred-and-forty-second beat with §11.6's **424**. **The ninth sweep is the first to check that a citation names the *right* table**,
and its first run corrected nine ledger rows and nine source comments — a whole block of §12.5.6's
annotation tables and a whole block of §14.8.5's attribute tables, every one of them ISO 32000-1's
number for something else. **Its second run, seven rounds later, found two more and one of them was
in the round before's own work; its third, eight rounds after that, found three and all three were
in the *source*, beside ledger rows that had been corrected without them.** **The tenth sweep is new in the four-hundred-and-second** — it compares a parent row's stated
*count* of its children with what the children say — and it paid twice on its first run and again on
its second. **The eleventh is new in the four-hundred-and-thirteenth**: it reads the ledger's
*quotation marks*, which no gate in this project has ever done, and its first run found six
misquotations of the standard (ADR 0249). **It is the ninth to become a program, in the
five-hundred-and-fortieth** — a second population inside `--bin quotations` rather than a binary
of its own, because a ledger note and a Markdown paragraph are the same question asked of
different prose — and the rule it gained on the way is what it had been blind to since it was
written: a `'` … `'` is a quotation too (ADR 0375).
Priority: 01 — the population with no gate, and it has paid on every session that touched it
Code: `doc/conformance/ledger.toml`, checked by `cargo test -p conformance`

**A sweep round commits one prose sweep as a program before running any of them.** Fifteen of the
seventeen are commands (`conformance --bin entries`, `--bin quotations`, `--bin unread` since the
four-hundred-and-eighty-ninth, `--bin blockers` since the five-hundred-and-first, `--bin
capabilities` since the five-hundred-and-tenth, `--bin retired` since the
five-hundred-and-seventeenth — the fourth sweep, which is the only one whose population cannot be
derived and which therefore takes its nouns as arguments, ADR 0352 — `--bin callers` since the
five-hundred-and-twenty-fifth, the fifth sweep, whose *number* is its finding and
whose level was therefore not comparable across runs while every round wrote its own script, ADR
0360 — and, since the five-hundred-and-thirty-seventh, `--bin pointers`, the eighth, which asks
whether the file and the symbol a note names still exist and which resolves a fragment *from where
it is written*, ADR 0372; since the five-hundred-and-fortieth, the eleventh, which is
`--bin quotations`'s second population rather than a binary of its own, ADR 0375; and, since the
five-hundred-and-forty-fifth, `--bin tables`, the ninth, which states in code what makes a key a
*claim* about a table and reads a denial as a claim in the other direction, ADR 0380; and, since
the five-hundred-and-fifty-third, `--bin inapplicable`, the seventh, the only sweep that reads the
status nobody expects to come back to, whose count of naming files replaces the stop-list every
hand-run wrote from memory and whose *cousin* is the pair all five of its defects have been,
ADR 0388; and, since the five-hundred-and-sixty-second, `--bin owed`, the fourteenth — the last of
the four whose *level* moved with the session, which stops guessing which words name a debt and
measures instead whether the tree has the **thing** a note names, which is the seventh sweep's own
discriminator with the sign reversed, ADR 0397; and, since the five-hundred-and-sixty-fifth,
`--bin counts`, the tenth — the last of the ten whose level was session-local, which decides what makes
a *cardinal* a claim about a family by the ninth sweep's attribution rule and answers it with the sixth
sweep's family arithmetic, so that neither half is a vocabulary written that morning, ADR 0400) and two
are
still
descriptions, and a description is what let the fifteenth go unrun for twenty-four rounds and then
be rebuilt from its own paragraph (ADR 0319) — `CLAUDE.md`'s "write down the command, not the
answer" failing in the direction it was written for. The cheapest moment to commit one is the
round that next has to run it, because that round has to reconstruct it anyway; at one per sweep
round the backlog is gone in ten, without ever being a marathon. A sweep that is genuinely
twenty lines of Python belongs under `tools/conformance` with the four that already live there.

## The sweeps as commands — what each one asks, and how to read its output

**This section was `doc/todo/02` §4's, and it is here because a round that is not sweeping does not
need it.** §4 keeps the rule — run them after a round that adds a verb, over `crates/`, `tools/`
and `fuzz/` as well as over `ledger.toml` — and the shape they share; what each one *is* belongs
with the reading, which is this file. Every bullet below is unchanged.

- **One reads the ledger's own quotation marks**, which no gate in this project does: the checker
  verifies every rustdoc blockquote in `crates/` and nothing at all in `ledger.toml`. Report only
  the misses that match the standard for at least five words and then diverge — a claim this
  project invented shares no words with it and a misquotation shares most of them. ADR 0249, and
  it is not a gate for a reason the ADR prices.
- **One asks who reads an *entry the clause states***, and it is the only sweep that reads no reason
  at all: `cargo run --release -p conformance --bin entries`, seconds, over `ledger.toml`, the
  standard's own tables and the source roots. It exists for the refusal shape every other sweep
  passes — a row that retires its refusal by naming a capability that arrived, and then nobody asks
  whether the *entry* that turns the capability on was wired to it. **It was described here and not
  committed for twenty-four rounds**, so the round that wanted it rebuilt it from the description,
  which is `CLAUDE.md`'s own rule failing in the direction it was written for. Three findings so far
  (ADRs 0295, 0315, 0319); read the hits whose entry the row's own **note** does not name first.
- **One asks who *quotes* an entry a note claims is unread**: `cargo run --release -p conformance
  --bin unread`, seconds, over `ledger.toml` and the source roots — the second sweep as a program
  (ADR 0324). A hit is a key some source quotes as a lookup string while a note says nobody reads
  it, sharpest where the quoting file is in the row's own `code = [...]`; the dominant noise is
  one short key in three clauses, so read the witness path it prints before believing a hit.
- **One asks whether a stated *blocker* has expired**: `cargo run --release -p conformance
  --bin blockers`, seconds, over `ledger.toml` and the source roots — the first sweep as a program
  (ADR 0336). A blocker sentence naming a clause is judged against the ledger's own account of
  that clause, and the expired ones print first; the three noise shapes it prints rather than
  filters — a correction quoting the wording it retired, a past tense no grep can see, and a
  clause named as the route to something outside the standard — are in the module doc. Read the
  sentence before believing a hit.
- **One asks whether the tree names a capability a note says is absent**: `cargo run --release
  -p conformance --bin capabilities`, seconds, over `ledger.toml` and the source roots — the
  third sweep as a program (ADR 0345). A hit carries the witness path where any source file
  names the lacking noun, and says whether the claim is about *the program* (the population
  that decays) or about *one crate* (usually a boundary it keeps on purpose); the dominant
  noise is a true boundary statement, which a witness does not disprove. Read the sentence
  before believing a hit.
- **One asks the question from the other end — who *calls* it?**: `cargo run --release -p
  conformance --bin callers`, a fifth of a second, over every `pub fn` in `pdf-model` and every
  crate, tool and fuzz target whose manifest names it — the fifth sweep as a program (ADR 0360).
  **Its output is a delta rather than a level**: the finding has twice been that a whole new host
  program took no name off the bottom rungs. Read the rungs from the bottom, and know the two
  directions it is loose in — a short name shared with another type's method reads as named, and a
  name reached through a wrapper reads as unnamed.
- **One asks where else a claim a round *retired* is still written**: `cargo run --release
  -p conformance --bin retired -- <noun> …`, seconds, over `ledger.toml`, the source roots and
  every Markdown document under `doc/` bar `doc/history/` — the fourth sweep as a program (ADR
  0352). It is the one sweep that cannot derive its own population, because what was retired is
  what the last rounds decided, so the nouns are arguments and the rule is `doc/todo/01`'s: give
  it the *mechanism*, not the sentence. Each mention is printed as a correction or as a standing
  claim, and **a noun carrying both is the shape to read first** — somebody wrote the retirement
  here and not there.
- **One asks whether the file — and the symbol — a note names still exists**: `cargo run --release
  -p conformance --bin pointers`, a third of a second, over `ledger.toml`, the source roots and
  every Markdown document under `doc/` bar `doc/history/` — the eighth sweep as a program (ADR
  0372). **A pointer is resolved from where it is written**, so a `tests/x.rs` in a doc comment
  means its own crate's tests and the same words in a document under `doc/` are *unrooted* rather
  than dead; the other three rungs it prints instead of a finding are a fragment that resolves in
  another crate, a metavariable (`doc/todo/NN`), and a path this tree deliberately does not carry.
  The oldest false positive is a correction quoting the pointer it retired, and it is marked rather
  than dropped. Read the sentence before believing a hit.
- **One asks whether the table a sentence cites states the key it gives it**: `cargo run --release
  -p conformance --bin tables`, seconds, over `ledger.toml`, the source roots and every Markdown
  document under `doc/` bar `doc/history/` — the ninth sweep as a program (ADR 0380). It counts a
  key only where the sentence **attributes** it, prints which table *does* state it, and reads a
  **denial** as a claim in the other direction, so "Table 119 gives a Type 0 dictionary no
  `/FontDescriptor`" is agreement and a denial the table contradicts is a hit. Its findings arrive
  in blocks, and its most durable population is a *document*: a number a round retires in the code
  goes on living in the ADR the code came from. **Two more ways it goes quiet, both found in the
  seven-hundred-and-thirtieth** (ADR 0620): a list of keys whose third item carries a parenthesis
  attributes nothing after that item, because `keys_within` stops at the first word that neither is
  a key nor continues a list; and a citation whose attributed noun is a *value* rather than an entry
  has no key beside it at all, so it lands in the keyless count — 0611's finding reached by a second
  route, and this time with a wrong number underneath it.
- **One reads the status nobody expects to come back to**: `cargo run --release -p conformance
  --bin inapplicable`, a fraction of a second, over `ledger.toml` and the source roots — the
  seventh sweep as a program (ADR 0388). Every other sweep walks the rows that *owe* something; this
  one takes an `inapplicable` row's own title and note apart into `/Key`s and identifiers and asks
  whether the tree names them. **The count of naming files is the discriminator** and it replaces
  the stop-list nine hand-runs each wrote from memory: the standard's shared vocabulary reaches
  dozens of files and sorts last, a rare word sorts first. Read the **cousin** it prints before
  anything else — a row that is not `inapplicable` and says the same word is the seventh failure
  shape, and it is where all five of this sweep's defects have been.
- **One asks whether the family a sentence counts holds that many rows**: `cargo run --release -p
  conformance --bin counts`, seconds, over `ledger.toml`, the source roots and every Markdown document
  under `doc/` bar `doc/history/` — the tenth sweep as a program (ADR 0400), and the last of the ten
  whose *level* moved with the session. A cardinal is a claim about a family only where it governs one
  of the ledger's own words for a row and only inside the sentence's own punctuation, and the answer
  side is the family's own arithmetic rather than a reader's memory of the convention — a count that
  leaves out the `General` row is right, and so is one that counts the clause's own row in. Read the
  **contradictions** first: two numbers for one family in two sentences of one note are wrong whatever
  the ledger holds, which is where both of this sweep's largest findings were.
- **One reads the status whose own definition promises a debt**: `cargo run --release -p
  conformance --bin owed`, seconds, over `ledger.toml` and the source roots — the fourteenth sweep
  as a program (ADR 0397), and the last of the four descriptions whose *level* moved with the
  session. A `partial` row must say which requirements are not executed; a note that names a debt
  names a **thing**, and a thing this tree does not have is a name no source carries. So the
  discriminator is the seventh sweep's with the sign reversed — there a term the tree *names* under
  a row claiming absence, here a term the tree *lacks* under a row claiming a debt — and the
  reading list is every row whose vocabulary the tree names in full, the one naming nothing
  specific first. The noise is a debt named in prose with no identifier in it, printed rather than
  filtered.
- **One reads *these documents'* quotation marks and the ledger's**, on the same discriminator and
  for the same reason: `cargo run --release -p conformance --bin quotations`, seconds, over every
  Markdown file this project wrote under `doc/` **and over `ledger.toml`'s notes**, which is the
  eleventh sweep and had been a hand-written script since the four-hundred-and-thirteenth. Its first
  run over the documents found three sentences quoted as the standard's that ISO 32000-2 does not
  contain, two of which were also standing in `crates/` in prose the gate does not read; its first
  run over the ledger found three more. **Suspect the conversion before the document**: four of its
  suspects were `doc/md/` losing text the PDF has, and the hyphen of a word broken across a line is
  the commonest of them. It reads single-quoted spans as well as double since the
  five-hundred-and-fortieth, and it prints how many so that a clean run says what it was clean over.
  ADRs 0309 and 0375.
- **One asks whether a parent's claim survives its own children**: `cargo run --release -p
  conformance --bin overstated`, a fifth of a second, over `ledger.toml` **and over nothing else**
  — the eighteenth sweep, the thirteenth of them to be a program, and the only one that opens no
  source file (ADR 0475). Every other sweep
  here reads a row against the tree; this one reads a parent's assertion that an entry or a table
  *is read* against a descendant's denial that anybody reads it, so both sides are this project's
  own claims about its own code and a contradiction is a contradiction whatever the standard says.
  It exists for the shape the six-hundred-and-forty-first found by reading and no committed sweep
  could print: **an overstating parent**, which names a thing the tree lacks (the seventh sweep's
  discriminator) under a row claiming the opposite of a debt (the fourteenth's population), so the
  sign is reversed twice over and both walk past it. Three rungs, closest first — the child denies
  the term itself; the child *owns* the term and denies reading, which is the rung §12.11 is on
  and the only one that could hold it, because the parent named Table 276 and the child denied
  `/RH`; and the child's denial names another table or another entry. Read the **unmarked** hits
  first: the dominant noise is a table read in part, marked by attributing each key to the table
  the sentence attaches it to — the ninth sweep's rule, and without it §12.11's own claim demotes
  itself on its neighbour's keys. The noise it leaves is a partitive with no table to divide it
  ("three of the four locations a `/Lang` may occupy"), which is left to the reader on purpose.
  Its first run found two live defects, §9.9.1's and §9.7.6's, both below.
- **One reads no row at all, and it is the newest**: `cargo run --release -p conformance --bin
  overtaken`, a fraction of a second, over the tree's **page-list notes** and `doc/adr/` — the
  nineteenth sweep and the fourteenth to be a program (ADR 0491). A page-list note is the doc
  comment above a `const NAME: [&str; N]` of corpus pages: the oracle's contradicted and ambiguous
  groups, quorra's refusal lists, `text_extraction`'s floors. Those notes carry the diagnosis a
  round reads before deciding whether a page is our defect, and nothing checked them against
  anything. The discriminator uses the one ordering `doc/adr/` already has — **an ADR number is a
  date** — so the sweep compares *the newest ADR a note cites* with *the newest ADR that names one
  of the note's own pages*, and a gap is a decision taken after the note was last revised about a
  page the note explains. Three rungs, closest first, and **all three require a shared page**: the
  later ADR names the list itself; it names a document the note's prose argues; it names only a
  list member the prose never mentions. That last requirement is the sweep's own first finding
  about itself — without it a *census* ADR that prints every list's name put a fifth of the tree's
  notes on rung 1. **Its first run named `CONTRADICTED_ANTIALIASED_EDGES` at the head of rung 2**,
  carrying `colors.pdf`'s pre-ADR-0476 ssim figures nineteen sessions on, in the paragraph
  immediately below the correction that said that paragraph was unaffected; `CONTRADICTED_UNEXPLAINED`
  was second, still asking for a measurement ADR 0489 had made and giving three numbers the gate no
  longer prints. Calibrated per trap 13 against 662's own defect restored to the tree, where it is
  rung 1, rank 1. The noise is the last rung — a 370-page list collects a passing mention for free —
  and a note may deliberately not cite a later ADR about a different property of the same page.
  **The cheapest way to keep off it: cite your own ADR in the note you rewrite.** The 62 notes that
  cite no ADR at all are counted rather than listed; the comparison has no left-hand side.
- **One reads no source at all and no row either, and it is the newest**: `cargo run --release -p
  conformance --bin quoted -- <the oracle's log>`, under a second, over the oracle's page-list
  notes and **the oracle's own printed output** — the twentieth sweep and the fifteenth to be a
  program (ADR 0495). Every other sweep here compares two things this tree wrote *down*; this one
  compares what a note says with what the gate *prints*, which is why it takes an argument. The
  right-hand side costs nothing extra: the oracle already prints all four measures for every page
  it does not call agreement, and a round that touched a note has run it. The discriminator is
  trap 1's by-hand tell mechanised — **a figure quoted in the gate's vocabulary that no page of
  its own note carries** — and the vocabulary is the whole of it, five spellings over four
  measures, because the count that preceded this sweep looked for two tokens and concluded there
  was nothing to anchor. Precision is the discriminator's other half: the gate prints three
  measures to two decimals and the similarity to four, comparison is made at the coarser of the
  two, and a figure written *finer* is another instrument's and drops a rung. Three rungs, closest
  first: contradicted with a confirmed figure beside it on the same line of the note;
  contradicted and written exactly as the gate writes it; contradicted only after rounding.
  **Under every hit it prints what the gate says instead**, nearest value first, because the
  correction comes off the run rather than out of reasoning. Its first run corrected fifteen
  figures across nine notes and found one thing no figure could have said: a note attached to the
  wrong list — forty lines diagnosing a paper under fifteen names sat above a one-page `DeviceN`
  group, and the tell was that group quoting a band none of its one page carries. The noise is a
  note narrating its own correction (the superseded figure is contradicted by construction and the
  prose is right), another instrument's table borrowing the gate's words, and a range read as its
  first endpoint. Calibrated per trap 13 by planting a wrong worst tile in a confirmed sentence:
  named, with the gate's own value offered first, and gone when the plant was restored.
- **One asks the twentieth's mirror question, and it is the newest**: `cargo run --release -p
  conformance --bin unpriced -- <the oracle's log>`, under a second, over the same two sides — the
  twenty-first sweep and the sixteenth to be a program (ADR 0606). `quoted` checks a figure a note
  *quotes*; **its own closing sentence says it cannot ask for one that is missing**, and five
  rounds recorded exactly that debt in the same words — *nothing links a group's note to which
  bound the gate fails its pages on* (sessions 489, 668, 672, 675, 680). This asks. The
  discriminator is **a measure the gate fails one of a note's own pages on, in a verdict of
  `CONTRADICTED`, that the note's prose never names**, and it is ADR 0497's sixth criterion made
  mechanical: a contradicted entry is a standing exemption from a *specific failing bound*, so a
  note pricing its mechanism in ink, cap rows or a perimeter has explained the picture and not the
  verdict. Which bound fails is arithmetic on the gate's own line, `Tolerance::accepts`' three
  ceilings and one floor. **The population is that verdict and no other** — trap 11 — because on
  an `ambiguous` page no two references agreed, so the bound beside them decided nothing. Three
  rungs, closest first: the note names measures and not one of them is a measure its pages fail;
  the note names no measure at all; the note names one failing measure and misses another. Its
  first run named `CONTRADICTED_TIGHT_CONSENSUS`, whose hundred and sixty lines name one measure —
  the worst tile, which is one of its three pages' — while the other two fail on **structural
  similarity alone** under a table of four unlabelled decimals. Calibrated per trap 13 against
  that live defect rather than a plant: rank 1 before the note was written, silent after, and the
  run then reads every failing bound in the pool named by the note that holds its page. **Two
  populations it names rather than counts**: a page whose figure and bound print identically at
  the gate's two decimals, so its own line cannot say what its verdict rests on; and a
  contradicted page sitting in no note at all.
- **One asks a question about *this tree* rather than about the standard, and it is the newest**:
  `cargo run --release -p conformance --bin parts`, a fraction of a second, over `ledger.toml`, the
  source roots and every Markdown document under `doc/` bar `doc/history/` — the twenty-second
  sweep and the seventeenth to be a program (ADR 0709). The tenth sweep reads a cardinal only where
  it governs one of the ledger's own words for a *row*; this reads one governing one of this tree's
  own **parts** — `backend`, `rasteriser`, `crate`, `host`, `worker`, `submodule` — and answers it
  with the workspace's own membership, read off the member directories, each package's `src/bin/`
  and `.gitmodules`. **It is a decay detector rather than a mistake detector**, and that is the
  whole of how to read it: "both backends" was true until the tree grew a third rasteriser, so most
  of the population it walks is correct sentences and what it offers is an ordering.
  **Two rules decide what it reads at all, and each removes a larger population than it keeps.**
  The noun follows the number *immediately*, so "both **native** hosts" — right about two of three
  — is never read; and the form must **presuppose** the size (`both`, `neither`, `either`, or a
  cardinal under a definite article), because "two backends draw the seam" counts two of them and
  claims nothing about how many there are. Reading bare cardinals as well put **293 further
  disagreements** in the first run and the sample was counts of a subset every time.
  Three rungs, closest first: the place is a crate the **whole population depends on**, where no
  pair can be meant — `pdf-render` is that crate for the backends and is where 767's defect was;
  the ledger or an undated document, which speaks about the tree; and a **dated** record
  (`doc/adr/`, whose number is a date, the nineteenth sweep's rule) or a place inside the
  population, which is counted rather than listed because a cross-backend test naming its pair is
  the dominant shape by a factor of six. The noise it leaves: a modifier that *follows* the noun
  ("four submodules under `doc/corpora/`", right about the four there), this project's own
  aphorisms repeated verbatim — trap 2's "a decision either backend can make alone is a decision
  neither has made" arrives as half a dozen hits that are one sentence — and a round's own record
  of running it, which the ninth sweep has too. Calibrated per trap 13 against **767's live
  defect** rather than a plant: `Image::is_smoothed`'s doc comment is rung 1 today, and correcting
  it to name three takes it off the rung and moves the agreeing count by one.
- **One is not run from here at all**: `cargo run --release -p spec-errata -- check doc/*.pdf`,
  seconds, asking the same spans a *different* question — does one of them quote a sentence Errata
  Collection 3 struck out? That needs none of ADR 0249's syntax, because the erratum supplies the
  other side. ADR 0254. **And `check` is only one direction of that question**, which the
  five-hundred-and-fifty-fifth and -sixty-second sessions each paid for: it compares *quotations
  this tree has written*, so an erratum over text nobody has quoted is invisible to it — a clause
  deleted, a subclause renumbered, a table's requirement column raised. The other direction is
  `emit`, read against what the ledger **claims**:

  ```sh
  cargo run --release -p spec-errata -- emit doc/*.pdf
  cargo run --release -p spec-errata -- moved doc/*.pdf
  ```

  **And a third direction is a command since the five-hundred-and-ninety-first** (ADR 0426), for the
  hole `check` and `emit` between them still left — a place that *records* an erratum and then
  quotes the words it removed:

  ```sh
  cargo run --release -p spec-errata -- applied doc/*.pdf
  ```

  Two seconds, over `ledger.toml`, every comment run under `crates/`, `tools/` and `fuzz/`, and
  every Markdown block under `doc/` bar `doc/history/`. **Its discriminator is that the erratum is
  named as data, by the writer, in the place itself**, so nothing is inferred: the `StrikeOut` and
  the `Caret` supply both sides and a hit is a quotation matching what the erratum struck and not
  what it put there. Read the hits that carry **no** mark of a correction first — a correction
  quoting the wording it retired is this family's oldest false positive and is marked rather than
  dropped, `doc/errata-read.md` is that shape from end to end and is counted apart, and a `#NNN`
  this collection does not carry is dropped and counted so that a clean run says what it was clean
  over. Its first run found the §14.8.4.7.2 shape one clause family over, in §9.6.2.2's row, twice.

  **The first of those two filters is a command since the five-hundred-and-sixty-fifth** (ADR 0400):
  `moved` prints every annotation whose instruction uses *move*, *renumber*, *delete* or *insert* **and
  names a clause number**, with what this tree has standing on that number — its ledger rows, its
  citations and its mentions in these documents. Its first run found two errata the hand-filter had
  walked past, one of them because this collection writes an instruction in the past passive as well as
  in the imperative. The other filter is still by eye and still pays: a strikeout whose whole text is a
  requirement word (`Optional`, `Required`, `Deprecated`). `doc/errata-read.md` has what each run found,
  and what this tree does about a number an erratum has moved. A round implementing a clause runs
  `emit` on that document **before** it writes, and not `check` afterwards alone.

## A twenty-third that is not built, and the reason is that its two sides agree

**A parent restating a child's *refusal* and dropping the condition the child stated it under**, and
it is not ADR 0481's mirror: there both rows take opposite stances and the population was fourteen
term-mentions; here both rows **deny**, and the question is whether the parent's denial is the wider
of the two. The seven-hundred-and-sixty-seventh session found one by reading and proposed the sweep;
the seven-hundred-and-sixty-ninth measured it and declined it, and the measurement is one line.

Restore the three §8.9.6 rows to what they said before 767 corrected them and read the operative
words:

- §8.9.6 (parent): "§8.9.6.2 refuses a stencil under a *graphics-state* soft mask, which would be
  two masks on one command"
- §8.9.6.1 (parent): the same clause, **word for word**
- §8.9.6.2 (child): "One case is still refused by name: a stencil under a *graphics-state* soft
  mask, which would be two masks on one command."

**There is no widening to detect.** The three sentences state one claim in identical words, so a
program comparing a parent's denial with its child's would count them as agreeing — which they do.
What bounded the refusal was two paragraphs *earlier in the child's note*, about the pattern
recomposition that needs the mask slot, and the condition was never in the refusal sentence at all.
767 found it by reading `content::image`'s own `if`, and no sweep whose two sides are ledger rows
can reach that: the correct answer was in the **code**, not in another row.

So the shape is real and the instrument is not. **A ledger-internal sweep would not have printed
the finding that motivated it**, which is session 701's clincher arriving for a second sweep, and it
is a stronger reason to decline than a small population is. Revisit this if a note is ever written
that states a refusal's condition *in the refusal sentence*, because then the two sides differ and
there is something to compare.

## Why

All 823 subclauses of the eight technical clauses have been read against this code since the
fifty-sixth session, and so are the 52 of the eight normative annexes since the
three-hundred-and-sixtieth; the statuses are gated: `silent` is **zero** — it was five from the
three-hundred-and-sixtieth to the three-hundred-and-sixty-ninth, when Annex O was built — `REVIEW_OWED` is empty and
fails the build the moment a cited-but-unread clause appears, and `FILE_ONLY_EVIDENCE_CEILING` is
zero and asserted with `==`.

What no gate can watch is a **note that has gone stale**, and the 249 `partial` rows are where
those live. Six failure shapes, in the order they were found:

1. A note that *understates* what the code does (five in session 115).
2. A note whose **reason** has expired — "while §X does not exist", "needs §Y" (117, 118).
3. A note claiming an entry is *unread* where the tree reads it (three in 122, five more in 159).
4. A note whose "what IS done" half is wrong — **the class that resists a grep**, because the
   name being present is what a grep looks for.
5. A note that is *stale about its neighbour*: §7.7.2 listed eighteen catalog entries as unread
   that were read, most of them by the session that built their clause. **A family's parent row
   is not maintained by the sessions that implement its members**, because the clauses do not
   cite each other. Four instances so far (§12.3's parent, §14.8.5.1's, §7.7.2's, and §7.9's in
   the two-hundred-and-seventy-eighth — it called dates, name trees and number trees "features
   this tree does not have yet" while all three of its own child rows read `implemented`, one of
   them with a gate over 1545 corpus date strings).
6. **A note that contradicts itself**, found in the two-hundred-and-seventy-eighth. §12.5.6.5 said
   "`/H`'s highlighting mode is still a response to a mouse this program does not draw" and, four
   sentences later in the same note, that Table 176's `/H` "is honoured since the
   hundred-and-thirty-eighth session … (ADR 0123)". The row was corrected by *appending* the new
   sentence and nobody re-read the paragraph above it. This is the cheapest shape to find and the
   only one whose evidence is entirely inside the row: read a corrected note **whole**, not from
   the correction onwards.

## A sixth sweep, and it is arithmetic rather than a grep: **which parents are behind their children?**

Twenty lines of Python over `ledger.toml` alone, no source and no clause text: build the map of
clause → status, and print every row that is `partial`, `reported` or `unreviewed` while **every
one of its direct children** is `implemented`, `inapplicable`, `out-of-scope` or `writer-side`. A
parent cannot owe less than its children and it can easily owe more, so a hit is not automatically
wrong — but it is always a row nobody has re-read since the last child closed.

**Its first run, in the two-hundred-and-ninety-eighth, produced five and four of them were wrong.**
Three in a shape this file had not named:

- **§7.6.3** was `partial` and its own note opened "[b]oth algorithms are implemented in both
  directions".
- **§9.10** was `partial` and its note said "[a]ll three of §9.10.2's methods are implemented since
  the hundred-and-fifty-sixth session" — a hundred and forty-two sessions earlier.
- **§14.3** was `partial` and its note, corrected four rounds before, said every subclause was
  read.

So the sixth failure shape is **the note was corrected and the status was not**. It is the exact
inverse of shape 1 and it is invisible to every grep in this file, because the note a sweep reads
is the half that is *right*.

The fourth was the ordinary fifth shape with a long fuse: **§9.7.6** said "what is missing is the
predefined `CMap` data §9.7.5.2 owns", which shipped in the hundred-and-fifty-sixth session
(ADR 0140, all 239 of them) — and §9.7.5.2's own row has read `implemented` ever since.

~~The fifth, **§7.9.2**, was read and kept: its `partial` is the object model carrying one string
type where §7.9.2.1 names three, which is a true statement about this tree.~~ **The fifth was
wrong, and this sentence is why it took until the six-hundred-and-twentieth session to say so.**
It is a true statement about this tree and it is not a statement about §7.9.2 — the clause opens
"PDF supports one fundamental string object", and §7.9.2.1's own row, `implemented`, says "[t]his
crate holds exactly that". Every later run of this sweep cited *this line* instead of re-deriving
the hit, so a dismissal written once outlived fifteen runs of the instrument that kept
contradicting it. §7.9.2 is `implemented`; ADR 0455 has the argument and the rule it leaves.

**And the run found one more thing, which is about this file rather than about the ledger.**
§7.9's parent row still said "[d]ates, text streams, name trees and number trees belong to
features this tree does not have yet" — false of three of the four, with a 1545-string gate over
one of them. `doc/todo/01` records that the two-hundred-and-seventy-eighth session's run *found*
exactly this row, and the row was never changed. **A correction recorded in a todo file is not a
correction**, and the only defence is to make the change in the same commit that finds it.

**And §7.9 was wrong a second time, for two hundred sessions, behind its own child.** The note the
two-hundred-and-ninety-eighth session wrote in place of the one above said `partial` was "§7.9.3's
text streams alone, which is `reported`"; §7.9.3 became `implemented` in the
three-hundred-and-eighty-seventh (ADR 0224), and the five-hundred-and-twenty-fifth's re-read
repeated the sentence rather than checking the row it named. The sweep could not see it because
§7.9.2 was also `partial` and stood in front of it — a hit is only printed when *every* child is
complete, so one wrong `partial` hides its parent's. Moving §7.9.2 in the six-hundred-and-twentieth
made §7.9 the sweep's next line, and it was wrong too. **The sweep is a chain rather than a list**,
and a round that clears a hit runs it again in the same session.

## The three sweeps

Twenty lines of Python apiece, each of which paid on its first run. Run all three after any round
that adds a verb.

| sweep | looks for | first catch |
|---|---|---|
| expired blocker | `while §X does not exist`, `needs §Y`, `until §Z` | session 118; found §9.7.5.2's "a licensing decision" 150 sessions after the decision |
| entry claimed unread | every `/Key` in a "Not read:" list, grepped against the tree | six of ten lists had a live entry; §7.7.3.3's had eleven of eighteen |
| capability | `this program has no ___`, `no panel`, `which this is not` | §12.6.3's "this crate has no events", 41 sessions after `Command::Pointer` |
| **retired claim** | the *string* a correction retired, grepped over every other row **and over `doc/adr/`** | §8.9.6.1 still said "reported rather than applied on 28 corpus documents" fourteen sessions after §11.6.4.3 retired that exact sentence |

**`doc/adr/` joined the fourth sweep's targets in the four-hundred-and-twenty-ninth**, and it is the
only change ADR 0265 makes to this file. An ADR is a dated record whose prose nobody maintains, so
the general sweeps over it are 64 hits to 2 defects and are declined — but a claim a *later* round
disproves and leaves standing is a defect wherever it lives, and the round that disproves it amends
the ADR that made it, in the same commit. ADR 0261 said `confined_wire` reaches `pdf_model::interpret`
and that `cargo-fuzz` is not installed here; the four-hundred-and-twenty-eighth disproved both with
`nm` and with `ls ~/.cargo/bin`, and wrote the correction into its own commit message rather than
into the ADR.

**And a fifth: run all four over the *source tree*, not only over the ledger.** The
two-hundred-and-twentieth session found `icon.rs`'s module comment blocked on a flag that had
arrived three sessions earlier, by accident. Run deliberately in the two-hundred-and-twenty-first
it produced four more, all of them false for between forty and two hundred sessions:

- `pdf-model`'s own **crate documentation** opened "[t]ext and images are not yet drawn", true of
  the sixth session; `content.rs`'s module comment said the same. A crate's front door is where a
  reader learns what it does, and it outlives every ledger row that says otherwise.
- `set_dash`'s doc comment said "only the 'solid line' case is honoured for now" — the sentence
  from *before* ADR 0018, on the function the handover calls the archetype of a feature switched
  off in one place.
- `requirements::unmet` named three of §12.11's requirements as unmet whose capability had
  arrived: `OCInteract` (a layer panel, session 167), `AcroFormInteract` (a field a person types
  into, 135) and `Attachment` (`Command::Extract`, 167). **Its own doc comment had predicted it**
  — "a session that builds a layer panel has to come back and change `OCInteract`" — which is the
  strongest form of this failure: a warning written where the work is does not fire either.

**The ledger has a gate and the source does not**, which is why these lasted longer. One `grep`
apiece.

**Run again in the two-hundred-and-sixty-ninth over `crates/`**, after ten rounds that added
panels and verbs: 89 matches, 87 of them true statements about what a *crate* deliberately does
not own — no clock, no filesystem, no toolkit — which is the shape this sweep produces most and
which is worth knowing it produces. The two that were false had both expired in the ten rounds
themselves: `outline.rs` opened "[a]n outline is a *panel* in a viewer that has none", false since
session 166, and `pdf-viewer.rs` said `/PageMode` had "[t]hree of the six it can now obey", false
since the session before. **A comment about a sibling crate's capability decays at that crate's
pace and not at its own.**

## A fifth sweep, from the other direction: **who calls it?**

The three above ask what a *row* claims. The two-hundred-and-fifty-third and -fourth sessions
found two clauses neither of them could see, and both were the same shape from opposite ends: a
capability arrived, and nobody maintains the *callers* of the code it unblocks, because the code
and the caller do not cite each other either.

- **§12.5.6.19's `/H`** was `implemented`, argued in ADR 0123 and tested with pixels — and
  `viewer-core` took the pressed annotation from `link_at`, so no host could press a widget for a
  hundred and fifteen sessions (ADR 0177).
- **§8.11.4.3's `/ListMode`** was read into `OptionalContent::list_mode` and asked by nothing,
  with a layer panel on the screen (ADR 0178).

**Run again in the two-hundred-and-seventy-eighth, and it produced a clause on its second run
too**: 175 functions now, 69 that neither host names, and `Signature::must_cover_whole_file` among
them. Table 255 makes the byte range's coverage a `shall` for two of §12.8.1's sub-filters and a
`should` for the rest; `viewer_core::notes` worded an uncovered tail identically for all of them,
so a file breaking that `shall` read as §12.8.1's NOTE 1 ordinary incremental update. The model
had the distinction, tested; the only host ignored it. **Nothing in the corpus exercises it** —
all six of the 974's signatures are `adbe.pkcs7.*` — which is why it could not have been found
from the demand side at all (trap 8).

The sweep is twenty lines: every `pub fn` in `pdf-model`, grepped against `viewer-core` and
`viewer-ui`. 174 functions, 72 that neither names. Most are internal helpers that happen to be
`pub`; read the ones whose name is a *clause's noun*. What it produced beside `list_mode`, unread
so far: `logical_order` and `logical_text` (§14.8.2.5, which `doc/todo/33` owes), `beads_on_page`
(§12.4.3's articles), `all_folders` and `folder_of` (§7.11.6's collection folders, with an
attachments panel already drawn), `document_language`, `alternate_description` and `actual_text`
(§14.9, waiting on `doc/todo/31`'s host), `print_field` and `user_properties` (§14.8.5's
attributes), `widgets_by_field_name` and `clear_field` (§12.7).

**The fourth sweep paid again in the two-hundred-and-thirty-eighth, on its own subject.** Run
after five rounds that corrected rows, over the *mechanisms* those corrections named rather than
over their exact words, it produced two:

- **§12.5.5** still ended "Not implemented: the NoZoom and NoRotate scaling this clause defers
  to §12.5.3" — twenty sessions after ADR 0168 applied it. The clause that *defers* is where
  the stale sentence lives, which is the shape exactly: correcting §12.5.3 left its neighbour
  lying.
- **§12.5.6.22**'s `/FixedPrint` was still explained by "a resolution-independent display list
  cannot express a size that depends on the view" — the *refusal* ADR 0168 dismantled. The claim
  it supports is still true (`/FixedPrint` waits on printing) and its reason was not, which is a
  fifth way for a row to be wrong: **right conclusion, expired argument.**

So the sweep is worth running over a *mechanism* and not only over a quoted string: grep for
`NoZoom`, `uncoloured`, `ColorTransform` — the noun the correction was about — and read every
row that holds it.

**And it paid a fourth time in the two-hundred-and-ninety-fifth, on the noun the round before had
just given the tree**: `XMP`. Four rows and four source comments still said this program reads
none — §7.7.3.3's page attributes, §8.9.5.1's image dictionary, §14.3's parent, `metadata.rs`'s
own crate-level paragraph, `has_metadata_stream`'s doc comment and `chrome.rs`'s properties panel
— and three of the six had been written *by the round that retired them*, one file away.
**Running this sweep in the same round as the correction is the cheapest it will ever be**, and it
is now what `02-every-round.md` step 4 means by "after a round that adds a verb".

And it produced a fifth row nobody was looking for. **§14.3.4 was `inapplicable`** on the reason
"a question for a program that *writes* or *displays* metadata, and this one does neither" —
false twice, since the hundred-and-thirty-sixth session and the hundred-and-seventy-third
respectively. Reading the clause then found one rule that binds and is met by construction
(§7.5.6's update leaves both metadata sources byte for byte), one excluded by `CLAUDE.md`, one
`may` declined with a reason, and **one `shall` that staying out of the way of is a decision**:
writing a `/ModDate` on save would oblige this program to write `xmp:ModifyDate` too, so the cost
of a date nobody asked for is an XMP *writer*.

**And it paid a third time in the two-hundred-and-ninetieth**, on the nouns four rounds had just
corrected — `notdef`, `Differences`, `SigFlags`. §12.7.4.3 still said "4 now draw their fields and
2 keep the blank because Helvetica has no glyph for their characters", six sessions after §9.6.5.1
gained the `/Differences` that draws one of the two and one session after this project established
that the reason was never a glyph. **Two rows, one mechanism, and the one that was corrected is not
the one that was wrong** — which is this sweep's whole subject, and the third time it has been the
*font* rows.

The same run over `crates/` was clean: 64 capability matches, every one a true statement about a
boundary a crate deliberately keeps — no clock, no filesystem, no toolkit, no window — down from 89
in the two-hundred-and-sixty-ninth because rounds since have retired the false ones. A clean run is
a result: it says the population has not drifted, which is the only way it is watched at all.

**The fourth sweep is new in the two-hundred-and-sixteenth session and it is the cheapest of the
four.** Whenever a row is corrected, the note says so in the row that was corrected — "this
sentence said X" — and X is a string. Grep the *whole* ledger for X's distinctive words: two
clauses describing one mechanism is the commonest shape in this file, and correcting one leaves
the other lying. It found §8.9.6.1 on its first run, and a second understating row beside it:
§8.9.5.1 said `/Mask` was "read only to report it", which stopped being true in the *fourteenth*
session (ADR 0023) and is the **third** entry in that one list to be recorded as unread while the
tree read it — a list that has been wrong three times about itself is a list to check rather than
to read.

Three false-positive shapes on the second, all seen: a note *quoting* its own retired wording
(§9.6), a key named in a sentence about something else (§12.7.5.3), and a key that is a string in
an unrelated list (`/Metadata` in `thumbnail.rs`). Read the hit before believing it.

**A fourth, and it is the most common: one short key, three clauses.** The two-hundred-and-ninth
session's run of the second sweep produced five hits and *all five* were this — §8.4.5's `/BG`
and `/TR` are Table 57's device transfer and black generation, while `appearance.rs`'s `"BG"` is
Table 232's widget background and `soft_mask.rs`'s `"TR"` is Table 145's soft-mask transfer
function. Three clauses, two names, nothing stale. **A clean run of a sweep is a result**: it says
the population it watches has not drifted since the last one, which is the only way that
population is ever watched at all.

## The shape the sweeps found last, and it is a new one: the blocker was the *interface*

The two-hundred-and-fourteenth session ran the capability sweep after a round that added a verb.
**§14.9.3** said Table 226's `/TU` "names a field in a user interface this program does not have"
— the familiar shape, and false since the hundred-and-thirty-second session put a window on this
program. But the window was never what blocked it. The clause is a `shall`:

> An alternative name may be specified for an interactive form field (see 12.7, "Forms") which, if
> present, shall be used in place of the actual field name when an interactive PDF processor
> identifies the field in a user-interface.

and `Query::FieldAt` answered with **one string**, which cannot be both the identity
`Edit::SetField` addresses and the label a person is shown. So the row would have gone on being
true-looking however many windows arrived: what had to change was the *answer's shape*. ADR 0167.

**The lesson for the sweep**: when a row's reason names a capability, ask what the program would
have to *say* to obey the clause, not only what it would have to have. A row can survive the
arrival of the very thing it names.

## The shape the sweeps found before that, and it is the longest-lived

The two-hundred-and-first session ran the capability sweep again. **§12.3.2.1** said a
destination's other two items — "[t]he location of the document window on that page" and "[t]he
magnification (zoom) factor" — are "properties of a window with scrolling and zoom, which this
program does not have". `Command::Zoom` and `Command::Scroll` had been in the vocabulary since
the **hundred-and-thirty-second** session: sixty-nine of them, the longest any of these has run.

The tell is the same every time: the row explains itself by naming something the *program* lacks
rather than something the *standard* leaves open. `viewer_core::Open::apply_view` answers all
eight of Table 149's forms now, and the row is `implemented`. ADR 0162.

## The shape the sweeps found before, and it is the strongest

The hundred-and-ninety-first session ran all three. §12.8.6 said a usage-rights signature grants
"features of a PDF processor that are not available by default" and that **"this program has no
feature behind such a gate"**; §12.8.2.3 said the same. Both were true when written and both
stopped being true in the hundred-and-thirty-fifth and -sixth sessions, when this program learned
to fill in a field and save a file — which are exactly the rights Table 258 grants and exactly
the changes Table 257's `/P` restricts.

And the requirement was not new. §12.8.2.2.1 has always carried, in a parenthesis:

> (These changes to the document shall also be prevented if the signature dictionary is referred
> from the DocMDP entry in the permissions dictionary.)

A `shall`, addressed to a processor that modifies, unread for fifty-six sessions after this one
became one. `ViewState::set_field` obeys it now.

**So: after a round that gives the program a verb, re-read the rows whose reason is about what
the program *is*, not only the ones about what a clause needs.** The same shape as §7.6.3.2's
random initialisation vector, which sat in an `implemented` row for a hundred and twenty sessions
because a reader only ever *reads* one (ADR 0129).

## The seventh run of the fifth shape, and it is the sweep's own subject

**§8.11.2.1 named two Table 96 entries as read by nothing, and the tree read both** — found in the
three-hundred-and-eighteenth session by reading the §8.11 family top to bottom rather than by any
grep, because the row's sentence contains no blocker, no capability and no retired string:

> Two Table 96 entries are read by nothing: /Name, which exists to be shown in a user interface,
> and /Usage, which feeds the automatic state setting of §8.11.4.4.

`/Usage` has been read since the **thirty-fifth** session — §8.11.4.4's usage application
dictionaries fetch it per group and evaluate Table 100's categories against it — and `/Name` since
the **sixty-seventh**, with `viewer_ui::chrome` putting it on a row since the hundred-and-sixty-
seventh. So the entry whose stated purpose is "presentation in an interactive PDF processor's user
interface" was recorded as unread for a hundred and fifty sessions after a panel existed to present
it. §8.11.2.1 is `implemented` and its parent §8.11.2 with it, which is the sixth sweep's shape
arriving one round after the row it depended on was fixed.

**And the same shape one clause along, in the three-hundred-and-nineteenth**: §8.11.3.2 said the
`DP` form was "not implemented", sixty-five sessions after ADR 0178's `groups_referenced_by` covered
it *by construction* — the clause's sentence has one consequence, a reference, and the walk that
answers `/ListMode /VisiblePages` reads the page's `/Properties` rather than interpreting the
stream. Two rows in one family in two rounds, both stale for the same reason: the session that
implements a mechanism does not maintain the rows of the clauses that need it.

**What this adds to the method**: the four greps and the arithmetic all read a row's *reason*, and
this row gave none — it simply listed two keys. The second sweep is the one that should have caught
it (an entry claimed unread) and it did not, because the sentence says "read by nothing" rather
than "Not read:". **Grep the shape, not the wording**: `read by nothing`, `is unread`, `nobody
reads` are the same claim.

## And the fifth shape found in the *code* rather than in a row, in the three-hundred-and-twenty-fourth

`optional_content.rs` explained answering Table 100's `Zoom` category at a magnification of 1.0 by
saying that "a display list has no magnification … the alternative is to thread a scale into
`interpret` and rebuild the display list per zoom, which is a viewer's design question rather than
a clause's". **The tree answered that design question in the two-hundred-and-seventeenth session**:
§12.5.3's `NoZoom` threads exactly such a scale through `ViewState::magnification`, and
`Interpretation::view_dependent` says which pages notice (ADR 0168). So the conclusion was right
and the argument had expired — `doc/todo/01`'s fifth shape, in a doc comment rather than in a
ledger row.

**What replaced it is a measurement**, because the clause has a `shall` behind it (§8.11.4.5:
"[w]henever there is a change to a factor that the usage application dictionaries with event type
View depend on (such as zoom level), the corresponding dictionaries shall be reapplied").
`examples/oc_usage_census` reads every configuration's `/AS` in all 974 documents: 31 state
`/OCProperties`, **six** state a usage application dictionary, and they name `View`, `Print` and
`Export` — **`Zoom`, `User` and `Language` not once**. A path nobody takes is one `CLAUDE.md`
forbids shipping, and now the row says so with a number instead of with an architecture.

## The six sweeps run again in the three-hundred-and-thirty-second, and a clean run is the result

After six rounds that added verbs — §12.5.6.10's markup, `Page`'s identity, §9.10.2's second
method reaching Type 3 fonts — all six were run over `ledger.toml` and over `crates/`:

- **The arithmetic sweep**: one hit, §7.9, which `doc/todo/01` already records as read and kept.
- **Expired blockers**: six hits, every one a row naming a clause it genuinely waits on
  (§11.4.6's knockout groups, §12.10.3's geospatial, §12.6.4.11's hide action).
- **Capability reasons**: 35 hits and every one a true statement about a boundary this tree keeps
  — no clock, no filesystem, no printing path, no comments pane. Two are the *quoted retired
  wording* inside a correction, which is this sweep's oldest false-positive shape.
- **Entries claimed unread**: the same nine §8.9.5.1 and §8.4.5 hits the two-hundred-and-ninth
  session identified as one short key in three clauses.
- **The caller sweep**: 198 `pub fn`s in `pdf-model`, 71 named by neither host. The interesting
  names are all one of three known populations — §14.7/§14.9's structure entries waiting on
  `doc/todo/31`'s host, §7.11.6's collection folders and §12.4.3's beads waiting on
  the panels that now exist (ADRs 0200 and 0202), and functions `pdf-model` calls *itself* (`unresolved_usage` is read by
  `content.rs`, `added_on` by the interpreter), which the sweep cannot see and which are worth
  knowing it cannot.

**A clean run says the population has not drifted**, which is the only way it is watched at all.

## The six run again in the three-hundred-and-forty-second, and the third sweep paid

After three rounds that added verbs — §12.7.5.3's `DoNotScroll`, `LaidOut::overflows`,
`QuorraRasterizer::rasterize_frame` — all six were run over `ledger.toml` and over `crates/`:

- **The arithmetic sweep**: one hit, §7.9, which this file already records as read and kept.
- **Expired blockers**: seven, every one a row naming a clause it genuinely waits on.
- **Entries claimed unread**: fourteen, all of them lists whose entries were checked in the
  two-hundred-and-ninth and three-hundred-and-thirty-second runs, plus §12.7.5.3's own — which the
  round that wrote it had just corrected.
- **Capability reasons**: 24 hits, 23 of them true statements about a boundary this tree keeps.
- **The caller sweep**: 198 `pub fn`s in `pdf-model`, 71 named by neither host — the same three
  known populations.

**The twenty-fourth capability hit was §12.5.6.2 and it had expired thirty sessions earlier.** The
row said `/Subj`, `/RC`, `/IRT`, `/RT` and `/IT` "reach a comments pane this program has no panel
for", and four of the five still do — but Table 172 makes `/RC` "[a] rich text string … that shall
be displayed in the **popup window** when the annotation is opened", and `viewer_ui::chrome` has
drawn that window since the three-hundred-and-twelfth session (ADR 0191). ADR 0199 reads it now.

**What that adds to the method**: a row that lists several entries behind one reason is several
claims, and the sweep reads the reason. §12.5.3's `NoZoom`/`NoRotate` was the same shape in the
two-hundred-and-seventeenth — "**split a refusal into one claim per entry before believing it**" —
and this is that rule applied to a *capability* reason rather than to an architectural one. Five
entries, one sentence, and only one of them named a capability that had arrived.

## All seven run again in the three-hundred-and-seventy-fifth, after five rounds that added verbs

`Query::Caret`, `pdf_model::restriction`, `pdf_render::repeat`, `ImageSource` and
`pdf_model::fragment` had landed in five consecutive rounds and none of them had re-swept, which is
the condition these exist for. Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 which this file already records as read and kept, and
  §O — whose own note already answers it, written by the session that built the annex.
- **Expired blockers**: four, and three are the *quoted retired wording* inside a correction
  (§11.3.7.2, §11.6.4.3, §11.7.4.4 all say "this row used to say"). The live one is §12.10.2's
  wait on §12.10.3, which is real.
- **Capability reasons**: 21 over the ledger, 69 over `crates/`, and every source hit was a true
  statement about a boundary a crate keeps — no clock, no filesystem, no toolkit, no trust store.
- **Entries claimed unread**: 12, eleven of them the known one-short-key-three-clauses population
  (`/Name`, `/Metadata`, `/ID` against `thumbnail.rs`'s key list). **The twelfth was §12.5.2 and
  two of its entries were live**: `/RC`, read since ADR 0199 thirty-three sessions earlier — the
  *same clause* §12.5.6.2's row was corrected for, and this row was never read against it — and
  `/NM`, which `fragment::annotation_named` resolves Annex O's `comment` against since the round
  before last.
- **Caller sweep**: 209 `pub fn`s in `pdf-model`, 75 named by neither host. The new name is
  `restriction::withheld`, and it is the known "functions `pdf-model` calls itself" population —
  `asserted` calls it, and its own doc comment says why it is separate.
- **Retired claim**, run over the nouns rather than the strings, and it paid twice on one phrase.
  **"Marking device"** is what ADR 0204 retired from `CLAUDE.md` in the three-hundred-and-fifty-
  seventh session, and eighteen sessions later it was still in six places: the ledger's own
  *definition of the `inapplicable` status* and the same sentence in `tools/conformance` twice,
  §8.4's parent row, §11.7.5's parent row, §11.7.5.2 (which contradicted itself four sentences
  later), and `requirements::unmet`'s `SeparationSimulation` arm. **A phrase inside a status's
  definition is the worst place for it**: §10.5 spent three hundred and fifty-seven sessions
  `inapplicable` partly because the word the status was explained with named a device the standard
  does not have.
- **The eighth sweep**, below, which is new and which found the `doc/todo/20` this file had been
  carrying as owed.

**And one of the phrase's six places was a defect rather than a comment.** `content.rs` explained
§8.6.8's list by saying `/TR` and `/TR2` "describe a marking device and are read nowhere here",
thirty lines below the `Transfer::read` that has read both since the three-hundred-and-fifty-eighth
— and the `/ExtGState` reader for them was **not** behind the uncoloured-figure flag the rest of
that list is behind, so a transfer function inside an uncoloured tiling pattern or a `d1` glyph
description decided a colour §8.6.8 reserves for whoever uses the figure. Seventeen sessions, and
the stale comment is why nobody looked. `an_uncoloured_cell_that_sets_a_transfer_function_is_ignored`
fails without the guard, painting black where the clause requires the `scn` blue.

**What that adds to the method**: a comment explaining *why* a list is what it is will be read as
the reason not to check the list. The sweeps hunt claims about capabilities; this was a claim about
**which entries a rule covers**, and the code drifted out from under it while the sentence stayed
plausible.

## A seventh sweep, and it reads the rows the sweeps had never looked at: **the `inapplicable` ones**

Every sweep in this file walks `partial`, `reported` and `unreviewed` rows, because those are the
ones that owe something. **`inapplicable` was never swept**, and it is the status a row goes to
when nobody expects to come back — which is exactly the property that lets a wrong reason live
there. The project owner asked for the re-read in the three-hundred-and-fifty-eighth session, after
§10.5's transfer function turned out to be `inapplicable` on a phrase — "marking device" — the
standard does not contain.

The sweep is twenty lines and mechanical: for each `inapplicable` row, take the capitalised
identifiers and `/Key` names out of its own title and note, and grep `crates/*/src` for each. **A
row claiming the tree does not do a thing, whose own vocabulary the source names, is a row to
read.** It hit 49 of 81; most are noise (`DeviceCMYK`, `XObject`, and the sweep's own English —
`Nothing`, `Whether`), and the signal is a *rare* word: `GoToDp` in three files under a `§14.12`
row, `DPart` in four.

**Its first run corrected five rows and amended two more**, and all five were the same shape — a
`§14` row saying a screen does not do this, beside a `§12` row saying the tree draws it:

| row | said | the clause says |
|---|---|---|
| §14.11.3 printer's marks | "outside what this viewer draws … a screen is not a printer" | "[t]he Print and ReadOnly flags … shall be set and **all others clear**" — `NoView` clear |
| §14.11.6.2 trap networks | "drawing it on a screen would paint the artefact-hiding overlaps *as* artefacts" | the same flags sentence, verbatim |
| §14.12.4, §14.12.4.1 document parts | "[n]either is read, and neither reaches a screen" | Table 409's `/Start` is what §12.6.4.5's `GoToDp` shows |
| §14.9.6 pronunciation | `inapplicable`, "the same reading §10.7.2's flatness permission gets" | §10.7.2 is `implemented`, on `CLAUDE.md`'s own rule |

`PrinterMark` and `TrapNet` are both in `annotation.rs`'s `STANDARD_SUBTYPES` and always have
been, and §12.5.6.20 and §12.5.6.21 said so in their own notes. **So the ledger held both answers at
once, in two families, for a hundred sessions.** The sixth sweep cannot see this: it compares a
parent with its children, and these pairs are cousins.

**What that adds to the method, and it is the sweep's own generalisation**: a mechanism gets one row
per clause that mentions it, and the rows are written in different sessions by different reasoning.
Shape 7 is **two rows about one mechanism, disagreeing** — and the tell is that one of them names a
*capability* ("a screen is not a printer") while the other names *code*. When a row's reason is
about what this program is rather than about what the clause says, find the other row.

**Run over the 87 `out-of-scope` rows in the same sitting, it produced no hit.** 26 of them name
something the source names — `RichMedia` in two files, `ECMAScript` in seven, `Rendition` in one —
and every one is a refusal the row already describes: §12.5.6.25 says in its own note that a
`RichMedia` annotation's "appearance streams are drawn where they exist, like any other
annotation's, because nothing in the placement path switches on subtype", which is exactly the
sentence §14.11.3's row was missing. **A clean run on a population is worth recording**, because it
is the only way this file knows a population has been read at all.

**And one distinction the run had to make rather than blur.** §14.11.2.2's guidelines are
`inapplicable` for a different reason than §14.10's web capture: the first is a **permission this
program declines** ("[i]nteractive PDF processors **may** offer the ability to display guidelines"),
the second is a clause about a thing this program is not. `CLAUDE.md` says a permission read is the
stronger answer, and §10.7.2 is `implemented` for exactly that — but it earns the status by naming
code that reads `i` and discards it. §14.11.2.2 has no code to name, so it keeps `inapplicable` with
its reason stated precisely. **The status vocabulary has one word for two situations**, and until
that is worth a status of its own the defence is that every such note says which it means.

## An eighth sweep, and it is the first that checks a note's *citations*: **does the file it names exist?**

The seven above read what a row *claims*. None of them reads what a row *points at*, and a
pointer decays the same way a claim does — faster, because deleting a file is a thing sessions do
on purpose. `tools/conformance` already checks the `code` and `test` arrays; nothing checks the
paths inside a note's prose, or inside a doc comment.

Twenty lines: pull every `doc/todo/NN`, `doc/adr/NNNN`, `crates/….rs` and `examples/…` out of the
ledger's notes and out of every `//` comment in `crates/`, and glob for each one.

**Its first run, in the three-hundred-and-seventy-fifth, produced seven and every one was dead.**

- **§8.9.6.1's `doc/todo/20`**, which this file has carried under "what is still owed" since the
  three-hundred-and-sixtieth session as *a dangling reference whose sentence might still be real*.
  It was not: the sentence said §8.9.6.2 "refuses a stencil painted with a *tiling* pattern", and
  ADR 0169 implemented exactly that in the two-hundred-and-eighteenth session and deleted
  `doc/todo/20` in the same commit — while §8.9.6.2's own row has said so ever since. **The file
  it named was deleted by the session that made the sentence false**, which is what makes this
  sweep cheaper than reading the row: the pointer and the claim died together, and only one of
  them is greppable without knowing anything about the clause.
- **`doc/todo/12`, six times in `crates/`** — `render-quorra/src/lib.rs`, `viewer-ui`'s
  `chrome_ladder` example and its `chrome_over_a_magnified_page` test. The todo was *done* and
  deleted; ADR 0198 is where its argument lives, and the comments now say so.

**What it costs to keep clean, and the one false positive it has.** A corrected note that quotes
its own retired wording reproduces the dead path inside quotation marks, so §8.9.6.1 will hit
every run from now on — the same shape the fourth sweep's oldest false positive has (a note
quoting itself). Read the hit before believing it; one line of context is enough to tell a
citation from a quotation.

**And the shape generalises past files.** A note that cites `crates/foo.rs::some_test` is making
the same kind of claim, and the checker only verifies the ones in the `test` array. A round that
wants a ninth sweep could take the *symbol* halves of every citation in a note's prose.

## A ninth sweep was tried and it is worth knowing it produced noise: **parents *ahead* of their children**

The sixth sweep asks which parents are behind their children. The inverse looks like it should be
stronger — a row saying `implemented` while one of its own direct children still owes something is
claiming every normative requirement in the clause is executed while the ledger itself says one is
not. **It produced 25 hits and none was wrong**, because this ledger's convention is that a parent
row covers the clause's *own* prose and its children own theirs: §7.4's framing is implemented
while five filters are `partial`, and that is a true pair of statements rather than a contradiction.

Run it once to know that; do not run it every round. The one hit worth reading was §O.2, and its
answer was already written into §O's note by the session that built the annex.

## The fifth sweep run again in the three-hundred-and-eighty-sixth, and it paid on the round's own work

**231 `pub fn`s in `pdf-model`, 84 named by no host** — up from 198 and 71, over four host crates now
rather than two, `viewer-confined` having joined `viewer-core`, `viewer-ui` and
`viewer-accessibility`. The populations are the three this file already knows. Two hits are worth
recording and only one of them is closed.

- **`Attachment::checksum_matches`, unreachable from the boundary the same round built.** §7.11.4's
  stream deliberately does not cross with the attachment *list* — a panel drawing five rows would
  otherwise pull five payloads across a pipe — so a confined host holds Table 45's `/CheckSum` in one
  message and the decoded bytes in another, and the clause's rule about the two of them together had
  no way to be asked. `pdf_model::attachment::checksum_matches` is a free function now; the method
  calls it, `viewer_confined::Attachment` calls it, and `tests/confined.rs` asks it end to end.
  **This is the sweep's shape at one round's remove**: nothing was unread when the round began, and
  the round's own transport made it unreachable. A sweep run in the same session as the work is the
  cheapest it will ever be, which this file already says about the fourth.
- **`Collection::initial_document`, and no host can call it at all.** It answers §12.3.5.1's `/D`
  fallbacks — the container, a named embedded file, the first file, or "an empty preview window" —
  and needs the `&Document` that only `viewer-core` holds. Not a confinement gap: `viewer_ui::chrome`
  draws the collection and cannot ask either. Written into §12.3.5.1's ledger row and into
  `doc/todo/34`; closing it is a field on `Answer::Collection` and a consumer for it.

**And one false positive worth naming, because it is new**: `Collection::all_folders` came back as
unnamed and is called by `examples/confined_panels`. The sweep greps `crates/*/src` and an example is
neither — so a function whose only caller is a *demonstration* reads as unread. That is the right
default (an example is not a host) and it is worth knowing the sweep cannot see one.

## All eight run again in the three-hundred-and-eighty-seventh, after eleven rounds with no sweep

The longest gap this file has had. `viewer-accessibility` (376), a signature's DER and CMS (377),
a font-metric band and a selection gate (378), §11.5.3's device branch and its residues (380, 383),
a confined interpreter and rasteriser (381, 386), `--backend` and `--cpu` (384) had all landed
since the last run. Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as
  read and kept. Clean.
- **Expired blockers**: 7 over the ledger and 25 over `crates/`. Three of the ledger's are the
  quoted retired wording inside a correction; §12.10.2's wait on §12.10.3 and §12.5.6.22's on
  printing are real. **Two source hits were live**, and both are below.
- **Entries claimed unread**: 14, twelve of them the known one-short-key-three-clauses population.
  **The thirteenth was §12.5.6.6's `/RC`** and it is the round's implementation.
- **Capability reasons**: 33 over the ledger, 108 over `crates/`, and every source hit was a true
  statement about a boundary a crate keeps — no clock, no filesystem, no toolkit, no trust store,
  no printer. §12.8.2.3's "this program has no feature behind such a gate" reads like the sentence
  §12.8.6's row was corrected for in the hundred-and-ninety-first and is not: it is about the
  *granting* half, and both rows say so in the same words on purpose.
- **Retired claim**, run over the nouns eleven rounds gave the tree — `AccessKit`, `DER`, `CMS`,
  `luminosity`, `backend`, `confined`, `variable text`, `/RC`. Clean but for `/RC`, which paid
  for the third time.
- **Caller sweep**: 231 `pub fn`s in `pdf-model`, 84 named by no host — the same numbers and the
  same three populations as the three-hundred-and-eighty-sixth, `Collection::initial_document`
  included and still open (`doc/todo/34`).
- **`inapplicable` (sweep 7)**: 25 of 83 rows name vocabulary the source names, and none was
  wrong. Annex Q's five are worth recording as the strongest kind of `inapplicable` there is —
  each carries the annex's own NOTE saying "this method is not required by this document".
- **Citations (sweep 8)**: clean. Two hits, both §8.9.6.1 quoting the `doc/todo/20` its own
  correction retired, which is this sweep's known false positive.

**And the first sweep's ledger hit is the longest-lived stale claim this file has recorded: 364
sessions.** §12.5.6.19 said "[w]hat is owed is the value: a text field's /V, a check box's or radio
button's state, and a push-button's /CA caption all need §12.7.4.3's variable text, so a widget
holding one draws its frame and reports the rest". It was written in the **twenty-first** session,
when constructing an appearance arrived, and it was false from the **twenty-third**, two commits
later, when `variable_text::lay_out` did. `appearance::field_text` lays out all three.

Three things make this the sweeps' own shape rather than an accident:

- **The row was corrected four times after the sentence went false** — sessions 105, 132, 138 and
  253 all added to it, by appending. Failure shape 6, and this is its record holder.
- **Its `test` array names a test that reads as confirmation and is not.**
  `a_widget_draws_its_background_and_reports_its_field_value` states no `/DA` anywhere, so the
  report it asserts is `Owed::NoFont` — one of the eight cases that genuinely still report — and
  the test's *own doc comment* had drifted into repeating the row's claim. A row and the evidence
  it cites can go stale together, because the same session writes both.
- **No grep in this file finds it from the ledger side.** "need §12.7.4.3's variable text" is the
  first sweep's shape only because §12.7.4.3 is a clause number; the sentence names no capability,
  no retired string and no unread key.

**Two live source hits from the first sweep**, neither of which any ledger row could have shown:

- **`tools/pdfref/src/main.rs`** opened "[o]ur own renderer needs a parser, which does not exist
  yet, so this cannot compare *us* against anything" — true when the tool was written and false
  from the round that opened a document. The division of labour it describes is still real, so
  what replaced it says *why* the tool compares the references with each other rather than
  claiming it cannot do otherwise.
- **`annotations.rs`'s `a_widget_draws_its_background_and_reports_its_field_value`** said "Table
  192's `/BG` is derivable and §12.7.4.3's variable text is not". The second half is the same
  claim §12.5.6.19's row carried, below, and it lived in the *test the row cites as its evidence*.

## A ninth sweep, and it is the first to check that a citation names the **right** table

The eight above read what a row claims, what it points at, and what its vocabulary implies. None
of them reads a *number*. `tools/conformance` checks that a cited table **exists** and prints its
title — a check the eighty-second session added after finding three ISO 32000-1 numbers in the
ledger — and a number that exists and names the wrong table reads exactly like a right one.

Twenty lines: parse every `Table N -Title` heading out of `doc/md/ISO_32000-2_sponsored_EC3.md`
with its first-column keys, then take every `Table NNN`'s `/Key` citation in `ledger.toml` and in
`crates/` and ask whether that key is one of that table's entries.

**Its first run produced 94 suspects, and eighteen were wrong.** Most of the rest are prose that
names a table and then a key belonging to the dictionary the table describes rather than to the
table itself ("Table 227's `/Ff`", where 227 is the flags inside `/Ff` and 226 is the entry) —
read the hit before believing it, as with every sweep here.

Nine of the eighteen are two **blocks**, which is what makes this sweep different from the others: a wrong
table number does not arrive alone, it arrives as a run of consecutive rows written in one sitting
against the older standard.

| row or file | said | ISO 32000-2 |
|---|---|---|
| §12.5.6.17 movie | Table 188, and a `/Aw` | Table **189**; there is no `/Aw` anywhere in the standard |
| §12.5.6.18 screen | Table 189, and a `/P` | Table **190**, whose five entries do not include `/P` |
| §12.5.6.19 widget | Table 192's `/H` | Table **191**'s; 192 is the `/MK` dictionary the rest of the row is about |
| §12.5.6.20 printer's mark | Table 190's `/MN` | Table **398**'s — which §14.11.3's row already said |
| §12.5.6.22 watermark | Table 191's `/FixedPrint` | Table **193**'s, whose value is Table 194 |
| §14.8.5.5 list | Table 381 | Table **382** |
| §14.8.5.7 table | Table 383 | Table **384** |
| §14.8.5.8 artifact | Table 384, with a `/Subtype` | Table **385**, which has two entries; `/Subtype` is Table 363's |
| §14.11.7 OPI | Table 402's `/OPI` | Tables 87 and 93 state the entry; **405** is its value |
| `pdf-font/src/collection.rs` | Table 127 defines `/FontFile2` | Table **124** |
| `pdf-model/tests/font_collections.rs` | the same sentence | Table **124** |
| `pdf-model/tests/oracle.rs` | Table 111 defines `/Widths` | Table **109** — and the quotation beside it is 109's |
| `pdf-model/tests/oracle.rs` | Table 174's `/Border` | Table **166**'s, beside the `/C` the same sentence puts there |
| `pdf-model/src/view.rs` | Table 179's `/Subtype` | Table **182**'s; 179 is the line ending styles |
| `pdf-model/tests/oracle.rs` | Table 145's `/BC` | Table **142**'s; 145 is the group attributes |
| `viewer-core/src/open.rs` | Table 179's `/QuadPoints` | Table **182**'s — the same pair as `view.rs` |
| `viewer-core/src/query.rs` | Table 98's `/Name` | Table **96**'s, which is the clause its own blockquote cites |
| `pdf-model/tests/actions.rs` | Table 197's `/A` | Table **166**'s; 197 is where the `/AA /U` it beats lives |

**§12.5.6.23's own note is why this sweep should have been run two hundred and eighty sessions
ago.** It says, in the row: "[t]he row previously cited "Table 193", which is the watermark
annotation's table and an ISO 32000-1 number; the redaction table is 195." The hundred-and-fifth
session found *one* of these, named the mechanism exactly, corrected its own row, and swept
nothing. Four of its immediate neighbours were carrying the same error, and one of them —
§12.5.6.22 — is the very watermark row whose table number §12.5.6.23 had been given by mistake.

**What it adds to the method**: a sweep is worth building the moment a correction names a
*mechanism* rather than a sentence, and "an ISO 32000-1 number" is a mechanism. The other place
this rule has already paid is the fourth sweep's "run it over the noun, not the string".

**A gate is not the answer here and that is a decision, not a deferral.** 94 suspects and 18
defects is the wrong ratio for a build failure, and tightening the heuristic enough to gate would
mean deciding which of English's ways of saying "the flags in Table 227's `/Ff`" are legitimate —
a checker that has to be right every time, which is the standard `citation.rs` already sets itself
for `another_document`. It stays a sweep, and it is cheap: one run is under a second.

## The blame list re-offers a row that was read and kept, and that is its one flaw

Found by running it again (ADR 0284). The order works — every defect that round found was at the
top of it — and **seventeen of the rows above the fold are ones the previous run read and kept**,
because keeping a row edits nothing and `git blame` cannot see a reading that changed nothing. So
the list re-offers them for ever and the never-read rows sit underneath.

**The remedy is not a stamp**, which `CLAUDE.md` puts in `doc/history/` anyway. It is that a row
read and kept **records the evidence that kept it** — the grep that was run, the entry that was
checked, the sentence that still binds — which is content rather than bookkeeping and moves the
blame pointer as a by-product. The previous run did exactly that for three of its seventeen (an
image dictionary's `/Intent`, a shading's `/Background` and `/AntiAlias`, a `DeviceN`'s
`/NChannel`) and not for the other fourteen, which is why they are still on top.

## Six rows read in the round that built §12.7.5.5's lock, and one of them was work

Off the same blame list, oldest first, skipping the seventeen above. ADR 0284 has the argument;
the shapes are this file's own.

| row | shape | was | is |
|---|---|---|---|
| **§12.7.5.5** | 4 | "Table 235's `/Lock` and `/SV` are signing behaviour" | `/SV` is; `/Lock` is a `shall` on whoever *changes a value*, which this program does. Read now, as a fourth `Restriction` — **`implemented`** |
| §7.6 | 2 | a revision 4 password outside ASCII needs "Annex D data this crate does not hold" | `pdf_syntax::text_string` has held Table D.3 both ways for hundreds of commits, and `crypt.rs`'s own comment says so |
| §7.7 | 5 | "what it does not read is everything the catalog holds for a *viewer*" | its own child §7.7.2 lists eighteen of twenty-five as read |
| §14.6 | 6 | "[w]hat is *not* read is any tag's meaning" | three sentences after saying optional content rides on `BDC`; four tags are read by name — **and that repair was itself one short**, which the seven-hundred-and-first found: `/Tx` is a fifth and the same note names it (ADR 0560) |
| §14.6.1 | 6 | the same sentence one row down | as above, plus §14.9's four entries and §14.7.5.2's `/MCID` |
| §14.8.2.6.1 | 3 | the `Alt`/`ActualText` exception "is not read" | both are read; every requirement left in the clause addresses a *document* — **`implemented`** |

**The one that turned into work is the one the corpus could never have found**: no document in the
974 states a `/Lock`, so the fixture is the only witness there is. Trap 8, and the reason a
spec-driven track exists at all.

## What is still owed, named

- ~~**§12.8.2.3's `should`**~~ — closed in the hundred-and-ninety-eighth session (ADR 0159).
  Table 258's rights are read, `ViewState::save` rewrites the permissions dictionary without its
  `/UR3` where a save would exceed them, and the condition was *counted* before it was trusted:
  all four corpus documents carrying a `/UR3` grant what this program does, so no file here can
  trip it. What is still owed under §12.8.2.3 is §12.8.2.2.2's comparison of two revisions, which
  needs the digest.
- **~132 `partial` rows** not yet re-read against the code, of 252.
- ~~**§12.5.6.6's `/RC`**~~ — closed in the three-hundred-and-eighty-seventh by the second sweep
  (ADR 0224). It is the **fourth** row to have carried "`/RC` … is XFA rich text, which principle 5
  excludes", after §12.5.6.2's in the three-hundred-and-forty-second and §12.5.2's in the
  three-hundred-and-seventy-fifth — and the first where the sentence hid a *different* `shall`:
  Table 177's `/RC` "shall be used to generate the appearance of the annotation", so a free text
  annotation stating only that entry drew a blank page.
- ~~**§12.3.5.1's `/D` fallback, implemented and reachable from no host**~~ — closed in the
  three-hundred-and-ninety-fourth (ADR 0231), eight rounds after the fifth sweep found it and two
  after this file recorded it as owed. It took what the entry predicted, a field on
  `Answer::Collection` and a consumer, and **a correction recorded in a todo file is still not a
  correction**: it sat named through seven rounds that each had room for it.
- ~~**§12.5.6.19's seven unread Table 192 entries**~~ — **three of the seven closed in the
  four-hundred-and-second** (ADR 0239): `/I`'s form XObject icon, `/IF`'s Table 250 fit whole, and
  three of `/TP`'s seven codes. What is still owed is `/TP`'s codes 2 to 5, which name the side the
  caption goes on and state no proportion for it, and `/RI`, `/IX`, `/RC` and `/AC`, which are
  pointer states a *constructed* appearance has no room for — one stream where §12.5.5 gives a stored
  one three. **The count came first and it decided the shape**: `examples/push_button_census` finds
  42 push-buttons in the corpus, 33 with their own `/AP /N`, so nine can reach the construction at
  all — and the only entry any of the nine states is `/IF`, in a document that states no icon.
- ~~**Annex I.2's version number**~~ — closed in the three-hundred-and-sixty-first session, the
  round after the sweep that found it (ADR 0207). It was worth one line here for exactly one round:
  a `should` nobody had read, two lines from a parser already standing on the number.
- ~~**A dangling `doc/todo/20`**~~ — closed in the three-hundred-and-seventy-fifth by the eighth
  sweep. It was in §8.9.6.**1**'s note rather than §8.9.6.2's, which is part of why nobody found it
  by reading the clause it was about: the refusal had been implemented sixteen sessions before this
  entry was written and a hundred and fifty-seven before it was corrected (ADR 0169), and
  §8.9.6.2's own row had said so all along.
- **§14.11.6.2's one reader-side sentence**, found by the seventh sweep and left unread: if the
  page object's `/LastModified` is more recent than the trap network annotation's, "the page's
  trap networks are invalid and shall be regenerated" — and a reader that cannot regenerate them
  is drawing traps the clause has called invalid. No corpus document states a `/TrapNet`, so this
  is a clause to read rather than a defect to fix, and the round that takes it owes the count
  first, the way `doc/todo/13` did.
- **§7.9.3 closed in the three-hundred-and-forty-sixth**, and it is the first `reported` row to close by a capability this tree gave *itself* one round earlier. The row named its own expiry condition — "this closes the day an entry in scope uses the type" — and ADR 0199's reading of Table 172's `/RC` was that day. Six entries in the whole standard are typed `text string or text stream` and `/RC` is the only one in scope, so implementing the clause was implementing it once. **A row that states its own trigger still has to be re-read by somebody**, and this one waited a round.
- **The 29 `reported` rows are worked out** — all read in the hundred-and-twenty-first and
  -second, and none is of the two known failure classes (a true observation about the wrong half
  of a sentence, ADR 0109; a clause with two populations where the row names one, ADR 0110). 17
  are cryptographic validation needing a trust store, 5 need a second file or a network, 3 are
  icon clauses whose own verb is *should*, and the rest name a device or a user control this
  program does not have.

## A tenth, and it is not a sweep — it is the gate that already runs, with a hole in it

Found in the three-hundred-and-ninety-first session by writing a comment and watching the gate
refuse it *inconsistently*: `QUORRA_FEEDBACK.md section 13` is the spelling this tree uses and the
draft had written `§13` twice, once with a `doc/` in front of it and once without. Only one was
refused.

`tools/conformance`'s `another_document` decides a `§` belongs to some other document when the word
in front of it is an upper-case stem with a `.md` suffix. `doc/` is not upper case. So **a citation
written with a path passed the arm for the whole of its life** — eight in the tree, six of them
naming `QUORRA_FEEDBACK.md`, which is the document the arm's own comment cites as the case it
exists to catch. All eight were being checked against ISO 32000-2's clauses and passing by landing
on one, which is the exact failure its message describes.

One `rsplit('/')` and a test, plus eight rewrites. The citation count went 5095 → 5133, of which
**minus eight** is this correction.

**What it adds to the eight above is a target rather than a technique**: they read the ledger's
prose and the tree's comments, and this one read a *checker*. A predicate about how a string is
spelled is a test of how the author spelled it, and the way to find the next one is to write the
thing the gate is meant to catch and check that it *is* caught — in every spelling a person would
plausibly use.

## All nine run again in the three-hundred-and-ninety-fourth, after seven rounds with no sweep

`Query::Offset` and `Query::FieldSelection` (388), a sub-pixel rule drawn as the pixel line it lies
in (389), `--trace`'s stages and clock (390), `Image::area_averaged` at 7.6× (391), a signature
verified under the signer's key (392) and §12.4.4's transitions drawn (393) had all landed since the
last run. Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as read
  and kept. Clean, for the fourth run running.
- **Expired blockers**: 13 over the ledger and 9 over `crates/`. Four of the ledger's are the quoted
  retired wording inside a correction (§11.3.7.2, §11.6.4.3, §11.7.4.4, §12.5.6.19); §12.10.2's wait
  on §12.10.3 and §12.5.6.22's on printing are real. `pdf-syntax/src/tree.rs`'s "four families …
  were blocked on one small piece of clause 7" reads as a hit and is in the past tense, which is a
  false-positive shape worth naming: **a sweep for a blocker cannot see a tense.**
- **Entries claimed unread**: 24, and every one is the known one-short-key-three-clauses population
  or a list whose entries were checked in the two-hundred-and-ninth, three-hundred-and-thirty-second
  and three-hundred-and-eighty-seventh runs. §12.5.6.19's "[a]ll seven are read by nothing" was
  re-checked against the tree rather than believed — `/I`, `/RI`, `/IX`, `/IF`, `/AC`, `/RC` and
  `/TP` are read nowhere, and the two hits a grep finds for `RI` and `IF` are §8.6.5.8's rendering
  intent and §12.7.8's FDF field. **A true row is a result**; it stays `partial` and stays named.
- **Capability reasons**: 41 over the ledger, 112 over `crates/`, and every source hit was a true
  statement about a boundary a crate keeps. `navigation.rs`'s "this crate has no clock to run one
  with" survives ADR 0230 by construction, because the round that drew a transition put the clock in
  `viewer-core` and said so in the same module comment.
- **Caller sweep**: 242 `pub fn`s in `pdf-model`, 87 named by no host — up from 231 and 84, the
  growth being session 392's DER, CMS and X.509 readers, which are the known "functions `pdf-model`
  calls itself" population. **`Collection::initial_document` is off the list**, which is this
  round's work.
- **`inapplicable` (sweep 7)**: 64 of 83 rows name vocabulary the source names, on a looser
  stop-list than the three-hundred-and-eighty-seventh's 25 of 83. None was wrong.
- **Citations (sweep 8)**: clean. Two hits, both §8.9.6.1 quoting the `doc/todo/20` its own
  correction retired, which is this sweep's known false positive. **The first run of it had a
  parser bug worth recording**: `examples/foo` lives under `crates/<crate>/examples/`, so a glob
  anchored at the repository root reported 32 live paths as dead. An instrument that says a
  citation is broken has to be right about where a file lives.
- **Retired claim**, run over the nouns seven rounds gave the tree — `selection`, `verif`,
  `trust store`, `transition`, `presentation mode`, `no clock`, `sub-pixel`, `trace`. **It paid
  twice, and both were parent rows.**

### The two the fourth sweep found, and they are the fifth failure shape at family scale

**§12.1 is clause 12's own map, and it said "§12.8's signatures read and never verified"** — retired
by ADR 0229 one round earlier, in nine rows of §12.8 that all say the opposite. The map row is
written once and amended by nobody, because the sessions that implement a member do not cite it.

**§12.6.4 said "three are performed and they are the three that change what is displayed"** — and
its own eighteen children say eight. `/GoToE` (§12.6.4.4), `/GoToDp` (§12.6.4.5), `/Thread`
(§12.6.4.7) and `/Named` (§12.6.4.12) had each been implemented by a different session, and
`/Trans` (§12.6.4.15) by the round before this one; §12.6's row repeated the same three one clause
up. **The sixth sweep cannot see this**: it asks whether every child is *settled*, and four of
§12.6.4's are `reported` or `out-of-scope` for good reasons, so the family never qualifies. What
finds it is counting the children — and the last sentence of §12.6.4's note was the fourth sweep's
own subject, a claim ADR 0230 had retired in the row next door: "`/Trans` is the fourth that could
change a mark and does not".

**What that adds to the method**: a parent row that states a *number* about its children is
checkable arithmetic, and no sweep here did that. The sixth sweep compares statuses; this compares
a count in the prose with the rows below it. Worth a tenth sweep the next time a family's parent
says "three of the twenty".

### And the ninth sweep paid on its second run, once on the round before's work

- **`pdf-model/src/navigation.rs`** opened "[`transition`] is Table 164's `/Trans`" — written in the
  three-hundred-and-ninety-third, and `/Trans` is **Table 31**'s, a page object entry. Nine lines
  down the same module comment says "Table 31 lists both as entries of a page object", so the module
  held both answers at once from the day it was written. Table 164 is what the entry's *value* is.
- **§12.6.4.2** cited "Table 206's `/D`" and the go-to action's `/D` is **Table 202**'s; 206 is
  §12.6.4.5's GoToDp dictionary, whose two entries are `/S` and `/Dp`. This one was in the
  three-hundred-and-eighty-seventh's 94 suspects and was not among its eighteen corrections, which
  is what a sweep with a 5:1 noise ratio costs: **the run has to be read to the end.**

80 suspects after both corrections, from 81 before, and the rest are the known prose shape — a
sentence naming a table and then a key belonging to the dictionary that table describes.


## All ten run again in the four-hundred-and-second, and the tenth was built

Eight rounds with no sweep: `doc/HANDOVER.md` restructured into ten files (395), JPEG 2000's
reduced-resolution decode (396), §11.4.6's knockout shape and `/AIS` (397), a check box that could be
checked (398), a shading's clip cropped (399), §11.4.4's non-isolated groups (400), and §12.5.6.6's
free text created and typed into (401). Over `ledger.toml` and over `crates/`:

- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as read
  and kept. Clean, for the fifth run running.
- **Expired blockers**: 9 over the ledger and 13 over `crates/`. Four of the ledger's are the quoted
  retired wording inside a correction (§8.6.8, §11.3.7.2, §11.6.4.3, §11.7.4.4); §12.10.2's wait on
  §12.10.3 and §12.5.6.22's on printing are real; the rest are past tense, which **a sweep for a
  blocker cannot see** and which the three-hundred-and-ninety-fourth already named.
- **Entries claimed unread (sweep 2)**: 31, thirty of them the known one-short-key-three-clauses
  population or lists checked in earlier runs — **and the thirty-first is this round's oldest
  finding.** §12.5.6.19's "[a]ll seven are read by nothing" was re-checked rather than believed and
  was true, which is what the round then implemented.
- **Capability reasons (sweep 3)**: 30 over the ledger and 96 over `crates/`, and every source hit
  was a true statement about a boundary a crate keeps — no clock, no filesystem, no toolkit, no
  trust store, no printer. Two of the ledger's are the quoted retired wording inside a correction.
- **Retired claim (sweep 4)**, run over the nouns eight rounds gave the tree — `knockout`, `/AIS`,
  `check box`, `free text`, `/DR`, `non-isolated`, `reduced resolution`. **It paid twice, both on
  `/AIS`, and the pair is this sweep's own subject**: §11.6.4.3's row was corrected in the
  three-hundred-and-ninety-seventh and the two other rows describing the same mechanism were not.
  §8.4.5 still listed `/AIS` among Table 57's *not read* entries with ADR 0027 as the reason — the
  argument ADR 0234 retired — and §11.5.1 still said it was "immaterial and deliberately not read",
  whose conclusion survives and whose second half does not. **And the same row gave a third**:
  §8.4.5's `/TR`/`/TR2` sentence pointed at "§10.5, which is `silent` now", forty-five rounds after
  ADR 0204 made §10.5 `implemented` — a row saying its neighbour is the ledger's *last* silence when
  the ledger has had none since the three-hundred-and-sixty-ninth.
- **Caller sweep (5)**: 246 `pub fn`s in `pdf-model`, 86 named by no host — up from 242 and 87. The
  new names are `measurement.rs`'s four (§12.10.2's real wait on §12.10.3), `named_page.rs`'s
  `disagreements` and `article.rs`'s `page_array_agrees`, all of them the known "functions
  `pdf-model` calls itself, or that only a test reaches" population. `document_part::first_page`
  reads as unnamed and is reached by every `GoToDp`, through `DocumentPartJump::page_in` — a host
  calling the *wrapper* is a shape this sweep cannot see and is worth knowing it cannot.
- **`inapplicable` (sweep 7)**: 30 of 83 rows name vocabulary the source names, none of them wrong.
- **Citations (sweep 8)**: one hit over files, §8.9.6.1 quoting the `doc/todo/20` its own correction
  retired, which is this sweep's known false positive. **But the shape generalises past files, which
  this file has said since the sweep was built, and running it over *sections* paid**: six comments
  in `crates/` cite "`doc/HANDOVER.md`'s section 0", and the three-hundred-and-ninety-fifth moved
  that section whole into `doc/ui-boundary.md`. The file they name still exists, so the file-level
  sweep sees nothing; what a reader following the pointer finds is one row of a table. **A section is
  a citation and it decays faster than a file, because moving one is a thing a session does on
  purpose.**
- **Table numbers (sweep 9)**: 193 suspects, **three defects and all three in the source**. Both
  are the sweep's own subject — a mechanism gets one row per place that mentions it, and correcting
  one leaves the others lying:
  - `annotation.rs`'s `/H` doc comment said "Table 192 gives `/H` the default `I`". `/H` is **Table
    191**'s, the widget annotation's own entry; 192 is the `/MK` dictionary. This is the *exact*
    correction the three-hundred-and-eighty-seventh made to §12.5.6.19's ledger row, and the source
    comment one directory away was not swept with it.
  - `appearance.rs` twice and `tests/annotations.rs` once said §12.5.6.12's stamp names are "Table
    186's list". Table 186 is the **popup** annotation; the rubber stamp's `/Name` is **Table 184**'s
    — and §12.5.6.12's own ledger row has said 184 all along. **The ledger held the right answer and
    three source comments held the wrong one**, which is shape 7 across the ledger/source line rather
    than between two rows.

## A tenth sweep, built in the four-hundred-and-second: **a parent's stated count against its children**

Invented by the three-hundred-and-ninety-fourth and not built. The sixth sweep asks whether every
child of an owing parent is *settled*, which §12.6.4 never qualifies for because four of its
eighteen children are `reported` or `out-of-scope` for good reasons. What that run wanted instead is
arithmetic on the **prose**: a parent row that says "three of the twenty" is making a checkable claim
about the rows below it.

Thirty lines: for every row with direct children, find a number word or digit in the note followed
within a phrase by a verb of implementation, and print it beside the children's actual statuses.

**Its first run produced 16 hits and two were wrong.** Most of the rest are the shape worth naming
before believing any of them: a count about something that is *not* the children — §9.6's "three of
the clause's properties", §12.7.8's "two entries that would add to a document", §7.4's "[f]our of
them are stream filters". A number in a parent row is usually about the clause and not about the
family, and the sweep cannot tell which without reading it.

The two that were wrong:

- **§14.11 said "[t]wo of its seven subclauses reach a screen … and both are implemented"**, and it
  was wrong on both halves. **Three reach a screen**: §14.11.3's printer's marks left the "for a
  press" list in the three-hundred-and-fifty-ninth, when the seventh sweep read the clause's own
  flags sentence — and this row went on naming them among "[t]he rest [that] are for a press" for
  forty-three rounds, while §14.11.3's own row carried the correction and §12.5.6.20's had said the
  code drew them all along. And **neither of the two it named is `implemented`**: §14.11.2 is
  `partial`. The seventh sweep found the *child*; nothing was watching the parent.
- **§12.3 said §12.3.5's collections and §12.3.6's navigators are "both … read as data with nothing
  presenting them"**, false of the first since the three-hundred-and-fifty-second, where
  `viewer_ui::chrome`'s files tab became the presentation §12.3.5's own `shall` asks for (ADR 0202),
  and further false since the three-hundred-and-ninety-fourth (ADR 0231). §12.3.5's own row says so
  in two sentences.

**Both are the fifth failure shape at family scale**, which is what the three-hundred-and-ninety-fourth
predicted this sweep would find, and both were invisible to the sixth: §14.11's children are not all
settled and §12.3's are not either, so the arithmetic that compares *statuses* never looks at them.
What separates the tenth from the sixth is that it reads the sentence rather than the status column.

**The false-positive ratio is 7:1 and it is not a gate**, for the ninth sweep's reason: tightening it
enough to fail a build would mean deciding which of English's ways of counting are about a family.
One run is under a second.

## The fifth sweep run again in the four-hundred-and-eighth, with a **fifth** host crate

The round built `crates/viewer-gtk`, so the sweep's grep population grew from four host crates to
five. **246 `pub fn`s in `pdf-model`, 85 named by no host — and the GTK host names not one that the
other four do not.**

Two things about that number, and the second is the finding.

- **The delta is what to trust, not the level.** The four-hundred-and-fifth recorded 246 and *86*
  with its own script; this round's script says 246 and 85 over the same four crates at `HEAD`, so
  the two extractions differ by one name and neither is wrong about the population. What is exact is
  the difference a fifth host makes, which was computed by running the sweep both ways in one
  sitting: **zero**.
- **A native host reaches `pdf-model` for *types* and for no function of its own.** `viewer-gtk`
  names `form::Control`, `TextControl`, `ChoiceControl`, `Choice`, `attachment::Attachment`,
  `outline::Outline` and `outline::Item` — the shapes `viewer-core`'s answers carry — and calls
  nothing in that crate the existing hosts do not already call. That is the boundary working as
  designed rather than a null result: the sweep exists because a capability can reach the crate
  implementing a clause and never reach a program, and a whole new program needing no new entry
  point is the strongest available evidence that the entry points are the answers.

## All ten run again in the four-hundred-and-thirteenth, an **eleventh** built, and the fifth sweep run over **eight** host crates

Ten rounds with no full sweep, and the tree grew four host crates in them: `viewer-gtk` (408),
`viewer-host` and `viewer-qt` (410), `viewer-ffi` (411), beside `Command::Delegate` (409),
`RasterFormat` losing `#[non_exhaustive]` and `Answer::Field` gaining a `ShownValue` (411), and
`Edit::SetField` carrying §12.7.5.4's selection as indices (412). Over `ledger.toml` and over
`crates/`:

- **Expired blockers (sweep 1)**: 6 over the ledger and 41 over `crates/`. Four of the ledger's are
  the quoted retired wording inside a correction (§11.3.7.2, §11.6.4.3, §11.7.4.4, §12.5.3);
  §12.10.2's wait on §12.10.3 is real; §7.7.2's is past tense. **One source hit was live and it is
  below**: `viewer-gtk/src/controls.rs` said this host "cannot" ask for a page without its widget
  appearances, three rounds after it could.
- **Entries claimed unread (sweep 2)**: 19, every one the known one-short-key-three-clauses
  population or a list checked in an earlier run. §8.11.4.3's `/Configs` was re-checked rather than
  believed and is true — the only thing in the tree that names it is `examples/oc_usage_census`, and
  the sweep is right that an example is not a reader.
- **Capability reasons (sweep 3)**: 18 over the ledger and 68 over `crates/`, and every source hit
  was a true statement about a boundary a crate keeps. Two of the ledger's are the quoted retired
  wording inside a correction.
- **Retired claim (sweep 4)**, run over the nouns ten rounds gave the tree — `native host`, `GTK`,
  `Qt`, `C ABI`, `Delegate`, `list box`, `RasterFormat`, `ShownValue`, `cancel`, `memfd`,
  `bare CFF`, `consensus`. Clean over the ledger: §12.7.5.4's row was corrected by the round that
  read Table 234, §12.5.5's and §12.7.4.2's name `Command::Delegate` as the thing that arrived, and
  no row still says a list is drawn by nothing.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file already records as read
  and kept. Clean, for the sixth run running.
- **`inapplicable` (sweep 7)**: 72 of 83 rows name vocabulary the source names, on a looser
  stop-list than earlier runs, and none was wrong. §14.8.5.5, §14.8.5.7 and §14.8.5.8 were re-read
  because their rare words (`PrintField`, `Decimal`, `Pagination`) come back every run, and all
  three carry the three-hundred-and-eighty-seventh's table-number corrections intact.
- **Citations (sweep 8)**: **it paid, and on a whole block.** `doc/todo/37` was deleted by the
  four-hundred-and-ninth session — the round that closed its last item — and **seven citations to it
  survived**: §12.7's ledger row and six comments in `viewer-confined`, `viewer-gtk` and
  `viewer-host`. This is the first run of this sweep to find a *live* claim behind a dead pointer
  rather than a dead claim: `controls.rs`'s said the GTK host "cannot" ask for a page drawn without
  its widget appearances, and `Command::Delegate` is what `Host::open` has sent since the round that
  deleted the file. **The pointer and the claim died in the same commit and only the pointer is
  greppable**, which is what this sweep is for. §8.9.6.1's two hits are the known false positive.
  One thing the run is worth recording for: **the sweep's own file globbing has to know where a file
  lives** — an `examples/foo` under `crates/<crate>/examples/` read as dead until the glob was
  fixed, which the three-hundred-and-ninety-fourth already recorded and which cost a second run
  here.
- **Table numbers (sweep 9)**: 73 suspects after the parser was taught that two of the standard's
  446 table headings carry a Markdown `##` — **and five defects, all five in the source, and two of
  them inside a function whose own doc comment had been corrected for the same thing one round
  before.**

### The five wrong numbers, and the first three are one entry

`/H` is **Table 191**'s, on the widget annotation, and **Table 176**'s, on the link. Table 192 is the
`/MK` appearance characteristics dictionary and states no `/H` at all. §12.5.6.19's ledger row was
corrected in the three-hundred-and-eighty-seventh; `annotation.rs`'s `highlight` doc comment in the
four-hundred-and-second; **and three more places in the same file were left**: the `Highlight`
enum's own doc comment above it, and *twice inside `highlight`'s body* — the comment naming the two
tables that define the entry, and the comment on the `_ =>` arm that takes the default. Five places,
one entry, three rounds. **The round that corrects a comment does not read the function under it.**

- `pdf-model/src/view.rs` — `mark_up`'s doc comment put `/QuadPoints` in **Table 179**, the line
  ending styles, and then in **Table 166**, which states it for no annotation. It is **Table 182**'s,
  the text markup annotations'. The three-hundred-and-eighty-seventh corrected this file's *other*
  `/QuadPoints` sentence and `viewer-core/src/open.rs`'s, and left these two.
- `pdf-font/src/lib.rs` — "§9.6.2's Table 109 names `/MissingWidth`". Table 109 is the Type 1 font
  dictionary and has no such entry; `/MissingWidth` is **Table 120**'s, on the font descriptor —
  which is what the three lines of code under the comment read it off.
- `pdf-model/src/requirements.rs` — "Table 43's `/Schema` and each file's Table 44 collection item
  dictionary", **two wrong numbers in one sentence**: `/Schema` is Table **153**'s, on the collection
  dictionary, and the collection item dictionary is Table **46**. 43 is the file specification and 44
  is the additional entries in an embedded file stream.
- `pdf-model/tests/actions.rs` — "Table 166's `/A` beats Table 197's `/AA /U`", **and 166 was itself
  a correction**: the three-hundred-and-eighty-seventh changed it from 197 to 166, and Table 166's
  nineteen entries do not include `/A`. There is no `/A` common to all annotations; the test's own
  annotation is a `/Link`, so it is **Table 176**'s. *A wrong number replaced by another wrong
  number* is what a 5:1 noise ratio costs when a run is read to the end without the table beside it,
  and it is the first time this file has recorded one.

### The tenth sweep paid again, and its finding is the second-longest-lived this file has

**§12.7.6 and §12.7.6.1 both said "the other two are refused by name" — reset performed, submission
and import refused — and the import has been *performed* since the hundred-and-thirty-second
session.** The sentence was written in the ninety-seventh, so it stood for **280 sessions**, behind
only §12.5.6.19's 364.

Three rows held the right answer the whole time and none of them is the two above: §12.7.6.4's own
row opens "[r]ead and performed" and names `Request::Import`, `Event::NeedsFile` and
`ViewState::import`; §12.6's row has counted import-data among the ten of Table 201's twenty types
performed since the three-hundred-and-ninety-fourth; and §12.7.6.4's status is `partial` rather than
the `reported` its parent claims for it. **The sixth sweep cannot see this** — §12.7.6.2 is
`reported` for a good reason, so the family never qualifies — and the fourth cannot either, because
no round ever *retired* the sentence anywhere. What finds it is reading a parent's prose against the
rows below it, which is the tenth sweep's whole subject and its second pair of hits in two runs.

### The fifth sweep over **eight** host crates, and the delta is again zero

`viewer-core`, `viewer-ui`, `viewer-accessibility`, `viewer-confined`, plus `viewer-gtk`,
`viewer-qt`, `viewer-ffi` and `viewer-host`. **327 `pub fn`s in `pdf-model` (249 distinct names), 86
named by none of the original four and 85 named by none of the eight** — so **four whole new host
programs, one of them in C, take exactly one name off the list**: `ViewState::widget_appearances`,
which is session 409's own work. The four-hundred-and-eighth measured the same delta at zero for one
new host; four of them make it one.

**And the sweep was run a second way, over `viewer-core`'s own vocabulary rather than over
`pdf-model`'s functions**, because "who calls it" has a second layer now that a host is not
`viewer-ui`: every variant of `Command`, `Query`, `Answer`, `Event` and `Edit`, against each of the
six crates that speak it. `Event` is unanimous — all fifteen named by all four programs — and the
finding is in `Query`:

- **`Query::Find` and `Query::LogicalSelection` are named by no program at all.** The only things in
  the tree that name either are `viewer-core`'s own headless test and `viewer-confined`'s transport,
  which is a pipe rather than a host. So this viewer has a text search implemented, tested and
  reachable, and **nothing a person can press**; and §14.8.2.5's logical content order, whose ledger
  row reads `implemented` on the strength of the query, had no consumer.
- **The second of the two is closed in this round**: `viewer-ui` copies the page selection on `c`,
  asking `Query::LogicalSelection` first and saying which of the two orders it got.
  `Query::Find` is left open and is named here so that the next round does not have to find it
  again — a find bar is a feature and not a sweep's business, and **a correction recorded in a todo
  file is not a correction**, so it is written down as owed rather than as done.

### An eleventh sweep, and it is the first to read the ledger's *quotation marks*

The four-hundred-and-twelfth found a note quoting Table 227 bit 1 in single quotes with wording the
standard does not use, and observed that `tools/conformance` verifies every rustdoc blockquote in
`crates/` — 567 of them — and nothing whatever in `ledger.toml`. ADR 0249 is the decision and the
numbers; the sweep is thirty lines and it is **the discriminator rather than the match** that makes
it usable:

**977** double-quoted spans of four words or more in the ledger's notes; **560** occur verbatim in
some document under `doc/md/`; **417** occur in none. A gate cannot be built on 417, because almost
all of them quote something that is not the standard and the ledger has no syntax that says so — a
row's own retired wording, `CLAUDE.md`, a report this program prints, another implementation. So the
sweep reports only the misses that **match the standard for at least five words and at least half
the quotation, and then diverge**: 12 of them, and **6 were defects**.

- **"an array of character codes and glyph names"**, in §9.6.5, §9.6.5.1 and §12.7.4.3. Table 112's
  own word is **character** names, and in a font clause the two are not interchangeable.
- **§8.4.4 quoted §10.7.2 as "a PDF processor may ignore this parameter"** — a sentence ISO 32000-2
  does not contain — while §10.7.2's own row carries the real one. Two rows, one permission, and the
  seventh failure shape inside a quotation.
- **§8.3.2.4** dropped "(initial)" out of the middle of a quotation; **§7.9.3** elided a
  cross-reference without an ellipsis.

The other six suspects are all the same false positive and it is about the instrument: the Markdown
conversion of the PDF breaks words across lines — `text-tospeech`, `hierarch y`, `T h`,
`implementationdependent` — so a quotation that is exactly right cannot be found. `quote::normalise`
does not repair those either, which is why two blockquotes written *in this round* failed the gate
until they were shortened.

**A twelfth sweep since the four-hundred-and-eighteenth, and it lives in `tools/spec-errata`
rather than here**: the same 977 spans against *Errata Collection 3* instead of against `doc/md/`.
It needs none of the syntax ADR 0249 priced, because the erratum supplies the other side of the
comparison — a span matching a sentence an erratum struck out is the standard's by construction,
whatever else the ledger quotes. First run: **21 ledger landings, 4 stale quotations in 3 rows**
(§7.6.3, §7.8.3 twice, §14.6), the rest being corrections quoting the wording they retired, which is
this file's own fourth-sweep shape. It also found a **third population of quotation**, which neither
this file nor ADR 0249 had counted: a pair of quotation marks inside ordinary rustdoc *prose*, which
`CLAUDE.md` binds exactly as hard as a blockquote and which the gate's `> ` scanner walks past —
39 landings, **6 stale quotations**, the worst of the three. `spec-errata check` is where
it runs; ADR 0254 is why it is not here and not a gate. **Its one known gap closed in the
five-hundred-and-fortieth**: `quoted_spans` collected double quotes only, because an apostrophe
would make every possessive an opening mark, and §12.7.5.2.2's stale quotation was in single
quotes. The rule that tells the two apart lives in `conformance::quote::quoted_spans` now and all
three populations share it — an opening `'` needs a space or a bracket before it, a closing one
needs a space or ordinary punctuation after it, and a double quotation mark ends the search so
that §9.4.3's operator names cannot swallow the quotations after them. It took the ledger's
landings from 12 to 20 in the struck-out-of-another-clause bucket on the round it landed. ADR
0375.

**The four-hundred-and-nineteenth ran it again and found a fourth and a fifth population**, both by
walking into one of them while reading §7.8.3 for an unrelated clause:

- **A quotation inside an ordinary `//` comment.** The scanner read `///` and `//!` only, and its own
  doc comment gave the reason — "a `\"` in a `//` comment is not making `CLAUDE.md`'s claim" — which
  is a claim about `CLAUDE.md` that `CLAUDE.md` contradicts twice over: it asks for the clause "in
  its doc comment, its module comment, **or the comment above the block**", and binds quotation
  marks rather than doc comments. **13 landings, 2 stale quotations** (§7.8.3's struck fourth bullet
  in `content.rs`, §8.9.7's NOTE 3 in `inline_image.rs`).
- **A quotation with an ellipsis in it.** `overlaps` compared a quotation whole, so a quotation of
  *parts* of one sentence could only match a struck passage shorter than itself — blind to exactly
  the shape `CLAUDE.md`'s own `…` convention produces. **8 landings, 4 stale quotations**, two of
  which are the two files the round before recorded itself as having missed.

**One false positive set the rule**, and it is the same lesson the twelfth sweep learned about
noise: asking whether *any* elided segment matched reported a §11.6.5.2 comment against a sentence
about `/BaseFont` on the four words "the same as the". The test is now one segment quoting the
passage whole **or** every segment inside it. ADR 0255.

**A sixth population is named and not counted**: every quotation of the standard in `doc/*.md`, in
`doc/todo/`, in `doc/HANDOVER.md` and in the 255 ADRs. Nothing reads any of it. The reason to expect
something there is the only reason any of these five was swept — each of the first sweeps found
something.

**And the sweep found a defect in the file rather than in a claim**: 17 rows carried **72
double-escaped quotation marks**, `\\\"` in the TOML, which decodes to a literal backslash before
the quote — so 36 quotations rendered with stray backslashes. Fourteen of the seventeen are the §8.4
family, written in one sitting, which is the ninth sweep's block signature applied to punctuation.
Repaired, and checked by round-tripping the file through `cargo run -p conformance --bin ledger`
rather than by reading it.

**What it adds to the method**: when a sweep's raw output is too noisy to act on, the move is not a
tighter grep but a *measure of how close the miss is*. A claim this project invented shares no words
with the standard; a misquotation shares most of them, and the difference is one binary search.

## What is still owed, named

- ~~**`Query::Find` reaches no program.**~~ **Closed in the four-hundred-and-fourteenth**: three
  hosts ask it — `viewer-ui` draws its own find bar, `viewer-gtk` a `GtkSearchBar` and `viewer-qt` a
  `QToolBar` — and the round that reached for it found the clause waiting behind it. Annex O's
  `search` needed a *document*-wide search that `Query::Find` is not, so `Command::Find`,
  `Event::Searched` and `viewer_core::search` arrived with the bar and the fragment parameter came
  off `Parameter::unhonoured`'s list. ADR 0250. **What the sweep got right is worth keeping**: it
  named the gap without fixing it, and the next round did not have to find it again.
- **~118 `partial` rows** not yet re-read against the code, of 252 — 14 more went in the
  four-hundred-and-thirteenth.

### The ninth sweep, run over the two families the four-hundred-and-fourteenth touched

Annex O's five rows and §14.7's fourteen, every `Table NNN` in them checked against the entries ISO
32000-2 actually puts in that table. **Nothing wrong**, which is the first clean run this sweep has
had and is worth recording as a result rather than as a silence: `Table Annex O.3` is the PDF object
identifiers and `Table Annex O.4` the open parameters, as both rows say; Table 354's ten entries
include `/PronunciationLexicon` and `/AF`, which §14.7.2's "unread" list names and which a 9 000-byte
window on the Markdown does *not* reach — the table is split across two header rows in the
conversion, so a check that read only the first block would have reported two false positives. The
instrument's own reach is part of the sweep, and this is the second time the Markdown's shape has
been the thing to watch after the four-hundred-and-thirteenth's broken words.

## All twelve run again in the four-hundred-and-twenty-ninth, after **fifteen** rounds with no sweep

The longest gap this file has had, and several of the fifteen were large: find bars in three hosts
and Annex O's `search` (414), the painted group's space measured (415), **the errata** — `doc/md/`
presenting struck passages as current text, `tools/spec-errata` built, some thirty stale quotations
fixed, a Type 3 glyph's own resources, and `Do`/`gs`/`scn` reporting a resource a file never defined
(416–419) — the readback cache and §9.4.4's vertical writing mode (420), `pdf-retrieve` and a
quadratic `Tree::walk` (421), three corpora submodules and pdfbox's frozen extraction (422–423),
`Document` made `Sync` (424), 1944 web documents and the first crasher (425), §11.4.7's page group
composited and §11.7.2's conversion into it (426–427), and twelve fuzz binaries found not to contain
the interpreter (428). Over `ledger.toml`, over `crates/`, and — for the first time, because
`SOURCE_ROOTS` reaches them — over `tools/` and `fuzz/`:

- **Expired blockers (sweep 1)**: 4 over the ledger and 18 over the source roots. Three of the
  ledger's are the quoted retired wording inside a correction (§11.3.7.2, §11.6.4.3, §11.7.4.4) and
  §12.10.2's wait on §12.10.3 is real. Every source hit is past tense or true; the one worth
  *checking* rather than reading is `pdf-sandbox/src/decode.rs`'s wait for `close2/hayro`'s
  `feat/reduced-resolution-allocates-less`, and `Cargo.toml` pins `2a1abd14` where that branch's
  commit is `1dc833f7`, so the blocker holds. **A blocker naming a revision is checkable and a
  blocker naming a clause is not** — worth knowing which kind a hit is before reading it.
- **Entries claimed unread (sweep 2)**: 19 over the ledger and 25 over the source, every one the
  known one-short-key-three-clauses population. Two were re-checked rather than believed:
  §12.5.6.7's `/Measure`, where the only reader in the tree is `measurement.rs` on Table 265's
  *viewport*, and §8.11.4.3's `/Configs`, unchanged since the four-hundred-and-thirteenth.
- **Capability reasons (sweep 3)**: 16 over the ledger and 100 over the source, every source hit a
  true statement about a boundary a crate keeps. §12.5.6.2's `/Subj` was re-checked against
  `viewer-confined`'s `subject` field, which is Table 349's *document information* and not Table
  172's — the one-short-key shape wearing a struct field.
- **Retired claim (sweep 4), run over the nouns fifteen rounds gave the tree** — `find bar`,
  `search`, `writing mode`, `errata`, `readback`, `retrieval`, `SafeDocs`, `pdfbox`, `page group`,
  `ink cube`, `fuzz`, `Tree::walk`, `composite font`, `Type 3`. **It paid four times and all four
  are one mechanism**, which is this sweep's own subject at its largest yet. See below.
- **Caller sweep (5)**, over eight host crates **and over `tools/` and `fuzz/`**: 335 `pub fn`s in
  `pdf-model` (257 distinct names), **111 named by none of the eight hosts, 89 named by no host, no
  tool and no fuzz target**. So **22 names come off the list because a tool or a fuzzer reaches
  them** — `logical_text`, which this file listed as unread from the two-hundred-and-fifty-third run
  onward and which `tools/pdf-retrieve` has asked since the four-hundred-and-twenty-first; `verify`,
  `signed_attributes_encoding` and `timestamp_imprint`; `spans_under` and `text_under`; `scan`,
  `section` and `sections`. **A tool is a consumer and this sweep could not see one for 176
  sessions.** The finding is below.
- **Arithmetic (sweep 6)**: three hits — §7.9.2 and §O, which this file records as read and kept,
  **and §9.4, which is new**: `partial` above four `implemented` children. Its reason had been
  retired by its own child nine rounds earlier.
- **`inapplicable` (sweep 7)**: 71 of 83 rows name vocabulary the source names, on the same loose
  stop-list as the four-hundred-and-thirteenth's 72, and none was wrong.
- **Citations (sweep 8)**: 4 hits, **0 defects and a new false-positive shape**. §8.9.6.1's
  `doc/todo/20` and §12.7's and `viewer-gtk/src/controls.rs`'s `doc/todo/37` are the known shape, a
  correction quoting the pointer it retired. The fourth is
  `tools/spec-errata/src/main.rs`'s `doc/errata.md`, and it is **a redirection target rather than a
  citation** — the file is what `emit >` writes and `.gitignore` names it for the same licence
  reason as `doc/md/`. A sweep that reads paths cannot see a `>` in front of one.
- **Table numbers (sweep 9)**: 412 headings parsed, 68 suspects, **3 defects — and two of them are
  in `tools/`, which this sweep had never reached.** Below.
- **Parent's stated count (sweep 10)**: 185 counted claims raw, 16 after the count is compared with
  the family's size and its settled total, **1 defect**: §11.7 states its count of the family twice
  and the two disagree. Below.
- **Ledger quotation marks (sweep 11)**: **1028** double-quoted spans of four words or more, 516
  verbatim in some document under `doc/md/` and 512 in none; 32 that match the standard for five
  words and then diverge, **3 of them defects**. Below. The other 29 are the two known shapes — an
  `…` elision, which `CLAUDE.md`'s own convention produces, and the conversion's broken words
  (`text-tospeech`, `hierarch y`, `T h`, `None ;`).
- **The errata (sweep 12)**, `cargo run --release -p spec-errata -- check doc/*.pdf`: **151 lines
  for 120 distinct struck passages, unchanged since the four-hundred-and-eighteenth finished reading
  them, so nothing remains unread and the number is not moving.** 28 in-clause landings, every one
  annotated in place **except one**, and a second in the "elsewhere" bucket that is the same
  sentence. Below.

### The four the fourth sweep found, and they are one mechanism in four rows

**Writing mode 1 and the predefined `CMap`s.** §9.4.4 was moved to `implemented` in the
four-hundred-and-twentieth after being `partial` for three hundred and eighty-four sessions on a
sentence that was false from the thirty-sixth; §9.7.5.2's data shipped in the hundred-and-fifty-sixth
(ADR 0140). Four rows went on describing a tree that had neither:

- **§9.4** — "§9.4.4 stays partial because the vertical branch of the displacement formula has no
  writing mode 1 to reach it", and the row's own `partial` rested on that clause alone. Now
  `implemented`; the clause states nothing above its four subclauses and all four are settled.
- **§9.7** — *both* halves of its `partial` reason were retired: "Table 116's seventy-odd predefined
  CMaps, refused by name on 15 corpus fonts" and "vertical writing, which is §9.2.4's missing /W2
  metrics rather than this clause's". §9.2.4's own row opens "[b]oth writing modes, from the
  thirty-sixth session", two clauses above.
- **§9.7.5** — "the other predefined names are refused because their data is not in the tree", the
  third row in this family to carry the sentence §9.7.6's row was corrected for in the
  two-hundred-and-ninety-eighth. **No earlier sweep could reach it**: the arithmetic that finds a
  `partial` parent above settled children never looks here, because §9.7.5.4 is `partial` for a
  reason of its own.
- **§9.3.7** — "which is trivially true here because only mode 0 exists". Right conclusion, expired
  argument, in an `implemented` row, which is the population sweeps 1, 2, 3 and 6 all skip.

**And one status the sweeps do not look for at all.** §9.7.5.1 was `partial` above a note that named
nothing owed — a row breaking the ledger's own definition of the status, which says "the note says
which are not". Every requirement §9.7.5.1 states is executed and each was already named in the note.
`implemented`, and the expiry is the same one: the predefined data landing in the hundred-and-fifty-sixth.

**What that adds to the method**: the sixth sweep asks whether a `partial` row's *children* are
settled. Nothing asks whether a `partial` row's *note* names anything owed at all — and that is
arithmetic on one row rather than on a family, cheaper than any sweep here, and would have found
§9.7.5.1 without knowing a thing about fonts. Worth a fourteenth the next time a round has room.

**And the status move cost §9.4 its evidence, which is the gate working.** The row named two whole
*test files*, tolerated on a `partial` row and refused at zero by `FILE_ONLY_EVIDENCE_CEILING` on an
`implemented` one, so `cargo test -p conformance` failed the moment the status changed. One named
test per subclause now. **A ratchet that only bites on a status change is a ratchet that bites when
a claim gets stronger**, which is when evidence matters most.

### The fifth sweep over `tools/` and `fuzz/`, and the one `pub fn` nothing names

`SOURCE_ROOTS` has been `["crates", "tools", "fuzz"]` since the four-hundred-and-twenty-eighth and
this sweep had never used it. Running it three ways in one sitting is what makes the number mean
something: 111 unnamed over the hosts, **89** once the six tools and the fourteen fuzz targets are
added, so a tool or a fuzzer is the only consumer of 22 names.

**`ViewState::clear_field` is named by nothing in the tree** — no host, no tool, no fuzz target, no
example and **no test**. It is the operation an undo needs, and its doc comment states the design
that separates it from `Edit::SetField` with an empty value: "the old value may have been the file's
own, and re-stating that as an edit would make every later save carry a change nobody made". That
distinction is invisible until a document is *written*, which is why the absence of a test is the
part worth acting on rather than the absence of a host.
`saving.rs::forgetting_an_edit_restores_the_documents_own_value_without_logging_one` asserts both
halves and the save after them. A consumer is a feature and is left named rather than built, which
is what this sweep got right about `Query::Find` in the four-hundred-and-thirteenth.

### The three wrong table numbers, and two are in a tool

- **`tools/pdf-retrieve/src/lib.rs`, twice.** `Wanted::subtypes` said an annotation subtype filter
  compares "Table 164's own names" and `Note::subtype` said "Table 164's `/Subtype`". **Table 164 is
  the transition dictionary.** The annotation types are **Table 171**'s and the entry is **Table
  166**'s, "[e]ntries common to all annotation dictionaries" — which the same doc comment cites
  correctly three lines down for `/Contents`, `/Subj` and `/QuadPoints`. Written in one sitting in
  the four-hundred-and-twenty-first, which is this sweep's block signature; the *first* run of it
  over `tools/` found them.
- **`crates/pdf-model/src/signature.rs`** — "Table 259 also records the older `/UR`". Table 259 is
  the FieldMDP transform parameters dictionary; `UR ( Deprecated in PDF 2.0 )` is a value of **Table
  256**'s `/TransformMethod`, and `/UR3` is **Table 263**'s key in the permissions dictionary. The
  comment had the *distinction* right and the table wrong, which is the shape that survives review.

### The tenth sweep's finding: a row that counts its own family twice

**§11.7 said "[t]wo of its five subclauses are satisfied" and, four sentences later, "four of its
five subclauses are satisfied".** The nineteenth session re-decided §11.7.4 and appended the second
count without reading the first — failure shape 6, standing for **410 sessions**, and the longest
this file has recorded after §12.5.6.19's 364 and §12.7.6's 280. Neither number survives contact
with the rows below it, which are one `inapplicable`, one `implemented` and three `partial`, each
naming what it owes.

**And the count was left behind by the two rounds that changed this family most.** The
four-hundred-and-twenty-sixth read §11.7.2 and §11.7.5.3 as what bounds §11.4.7's page group; the
four-hundred-and-twenty-seventh carried out the conversion §11.7.5.3 names and took a web sample's
61 blocked pages to 0. The parent row mentioned neither. **A parent row is not maintained by the
sessions that implement its members** — this file's fifth failure shape, and the tenth sweep is now
the third instrument to have found it at family scale.

### The three misquotations, and one of them is a full stop

- **§7.6.4.1** quoted "There is nothing inherent in PDF encryption that enforces the document
  permissions." The sentence continues "specified in the encryption dictionary", so the quotation
  ends a sentence the standard does not end — an elision *with a full stop inside the quotation
  marks*, which reads as complete where an `…` would have read as cut. Restored whole.
- **§8.4.3.3** quoted "at both ends of open subpaths (and dashes 8.4.3.6)", dropping
  `, "Line dash pattern"` from inside the parenthesis and "when they are stroked" from the end.
- **§12.3.3** quoted Table 152's `/Title` as "the text that shall be displayed on the screen for
  this item" where the standard capitalises. The same row writes `[t]his value`, `[w]hen an item`
  and `[c]licking` four times over, so the convention was there and one quotation missed it.

**And the sweep found a defect in the file rather than in a claim, for the second time**: §7.3.9's
row carried a `\\\"` — a double-escaped quotation mark, which decodes to a literal backslash before
the quote. The four-hundred-and-nineteenth repaired 72 of these in 17 rows and this one survived,
in a row none of the seventeen was next to. Round-tripped through
`cargo run -p conformance --bin ledger` rather than read.

### The errata sweep's one finding, and it is the fourth sweep's shape across the errata line

`check` prints **151 lines for 120 distinct passages**, which is exactly what
`doc/errata-read.md` recorded when the four-hundred-and-eighteenth finished reading them: **nothing
remains unread, and the number has not moved in eleven rounds.** That is the answer to the lead the
four-hundred-and-nineteenth left, and it is worth writing down because a passage count that stops
moving is the only evidence that a population has been read to the end.

The 28 in-clause landings are all annotations sessions 416–419 wrote in place, **except**
`crates/viewer-host/tests/host_mappings.rs:138`, which quoted §7.11.4.1's "shall map name strings to
file specifications" — struck outright by Issue #481 with the two bullets around it. And
`crates/viewer-host/src/panel.rs:144` quotes the *same sentence* and sits in the "elsewhere" bucket
only because it cites §12.3.5 rather than §7.11.4.1, so `Landing::in_clause` files it away from its
own finding. **`pdf_model::attachment` was corrected for this exact sentence in the
four-hundred-and-eighteenth and the two copies of it one crate over were not** — the fourth sweep's
subject, with an erratum in place of a session's correction. What survives §7.11.4.1 is its NOTE
about pre-PDF-1.6 identification, and that is what both comments rest on now.

## All twelve run again in the four-hundred-and-thirty-seventh, and the **fourteenth was built**

Eight rounds with no sweep, several of them large: a three-component APP14 JPEG asked for four
channels and a `/Length 0` stream reported as missing drawing (430), five pages moved between
contradicted groups and `spec-errata`'s comparison found case-sensitive (431), a diagonal sub-pixel
stroke substituted (432), **65 944 web documents surveyed** with a hang and a crasher fixed (433), a
silent population split and 22 435 lost codes taken to 780 (434), `build_soft_mask` converting a
buffer 99.96% transparent and all 84 budget refusals opened (435), and a document's own press
honoured (436). Over `ledger.toml`, `crates/`, `tools/` and `fuzz/`:

- **Expired blockers (sweep 1)**: 21 over the ledger and 71 over the source roots. Three of the
  ledger's are the quoted retired wording inside a correction; §12.10.2's wait on §12.10.3 and
  §12.5.6.22's on printing are real; the rest are past tense, which **a sweep for a blocker cannot
  see**. Every source hit is past tense or a true statement about a dependency.
- **Entries claimed unread (sweep 2)**: 51 over the ledger, every one the known
  one-short-key-three-clauses population. Five were re-read whole rather than believed — §12.4.3's
  `/N` and `/B`, §12.5.6.7's `/L` and `/Rect`, §12.7.3's `/Fields` and `/SigFlags`, §9.6's
  `/Encoding` and §12.5.6.6's `/BS` — and all five are rows that already say what they mean.
- **Capability reasons (sweep 3)**: 48 over the ledger and 124 over the source roots, and every
  source hit was a true statement about a boundary a crate keeps — no clock, no filesystem, no
  toolkit, no trust store, no printer, no network.
- **Retired claim (sweep 4)**, run over the nouns eight rounds gave the tree — `APP14`,
  `three-component`, `/Length 0`, `hairline`, `sub-pixel`, `blank glyph`, `Latin`, `B2A`,
  `demultiplied`, `budget refusal`, `press`, `web sample`. **Clean over both**, which is what a
  sweep says when the rounds that made the corrections made them everywhere.
- **Caller sweep (5)**, over eight host crates and over `tools/` and `fuzz/`: 339 `pub fn`s in
  `pdf-model` (261 distinct names), 93 named by none of the eight hosts, **74 named by no host, no
  tool and no fuzz target** — the three known populations. `ViewState::clear_field` is still on the
  list and this round's answer to it is below.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, both of which this file records as read and
  kept. Clean, for the eighth run running.
- **`inapplicable` (sweep 7)**: 71 of 82 rows name vocabulary the source names, on the same loose
  stop-list as the four-hundred-and-twenty-ninth's, and none was wrong.
- **Citations (sweep 8)**: 4 hits, 0 defects — §8.9.6.1's `doc/todo/20`, §12.7's and
  `viewer-gtk/src/controls.rs`'s `doc/todo/37`, all three corrections quoting the pointer they
  retired, and `spec-errata`'s `doc/errata.md`, which is a redirection target rather than a citation.
- **Table numbers (sweep 9)**: 409 headings parsed, 82 suspects, **one defect**, and it is below.
- **Parent's stated count (sweep 10)**: 124 counted claims raw, 88 after the comparison. **Two
  defects and a convention**, below.
- **Ledger quotation marks (sweep 11)**: **1058** double-quoted spans of four words or more, 570
  verbatim in some document under `doc/md/` and 488 in none; 16 that match the standard for five
  words and then diverge, **0 of them defects** — every one is a known shape (an `…` elision, a
  Markdown `**` or `[Table N]` inside the span, the conversion's broken words, or a correction
  quoting its own retired wording). **And the sweep has a floor this round walked into from the
  other side**; see below.
- **The errata (sweep 12)**, `cargo run --release -p spec-errata -- check doc/*.pdf`: **151 lines
  for 120 distinct struck passages, unchanged for eight rounds**, and 27 in-clause landings, every
  one an annotation written in place. The population is still read to the end.

### The fourteenth sweep, built, and what it printed on its first run

Twenty lines over `ledger.toml` alone: every `partial` row whose note contains none of the words a
note owing something uses. **16 rows, and seven were defects — two of them statuses.**

| row | was | is |
|---|---|---|
| **§9.3** | `partial` since the thirteenth session | `implemented` |
| **§11.3.7.1** | `partial` since the fifteenth | `implemented` |
| §12.8.4.2, §8.11.1, §8.11.4, §8.11.4.1, §10.4.2 | notes naming nothing owed | notes naming what the family owes |

The other nine are the sweep's own noise and it is one shape: a note that names something owed in
words the sweep's vocabulary does not hold — "writer-side", "counted, not parsed", "which needs the
validator", "what is absent is the distinction". Read the hit before believing it, as with every
sweep here.

**§9.3 is the longest-lived stale claim this file has recorded: 365 sessions**, past §12.5.6.19's
364 and §12.7.6's 280. The row said "[e]ight of Table 102's nine text state parameters are
implemented … the ninth, text knockout, is §9.3.8, which was this tree's third `silent` row until
the fourteenth session made it report" — a sentence written in the thirteenth session and false from
the **seventy-second**, when §9.3.8's `/TK` was *drawn* rather than reported, which §9.3.8's own row
has said ever since.

Three things make it the sweeps' own shape rather than an accident, and the third is why a fourteenth
sweep was worth building:

- **The row's reason is a report, and a report that has stopped being one still reads as a reason.**
  Sweeps 1, 3 and 4 all read a row's reason: this one names no blocker, no capability and no retired
  string, only a neighbouring clause and a status that had moved.
- **The sixth sweep's arithmetic never looks here.** §9.3's children are not all settled, because
  §9.3.1 was `partial` too — and §9.3.1 was `partial` for the *same* expired reason, so the family
  disqualified itself with the very error being looked for.
- **§9.3.1 is a defect the fourteenth sweep could not print either**, and it was found by reading
  §9.3's family off the back of the hit. Its note ended "Partial only because of Tk, whose absence is
  §9.3.8" — which contains the word *partial*, so the sweep's own vocabulary counted it as owing
  something. **A row that names its debt and is wrong about it is invisible to an instrument that
  only asks whether a debt is named.** Both are `implemented` now, with §9.3.1's one untested
  sentence asserted rather than derived: a text state operator "may appear outside text objects, and
  the values they set are retained across text objects in a single content stream", and `q`/`Q`
  carries them.

**And the status move cost §9.3 its evidence, which is the gate working** — it named two whole test
*files*, refused at zero by `FILE_ONLY_EVIDENCE_CEILING` on an `implemented` row. The same ratchet
caught §9.4 in the four-hundred-and-twenty-ninth. Four named tests now, one per requirement group.

### The tenth sweep's finding, and the clause number the errata had given away

§14.8.4.7 said "See the three below" above **four** child rows, two of them carrying the *same*
title — `§14.8.4.7.3 Ruby and warichu elements` and `§14.8.4.7.4 Ruby and warichu elements`. The
first row's own note explained the duplication and had it backwards: it said the number "appears
twice in `doc/md/` … the citation checker finds the first, which is this one", reading a
**renumbering** as an artefact of the conversion.

**Errata Collection 3 Issue #133 inserts a new subclause at §14.8.4.7.3 — link elements — and
renumbers the ruby one to §14.8.4.7.4**, saying so in its own editor's note. So the ledger held
§14.8.4.7.3's content under a number that had moved, and the clause the number now names, with two
`shall`s in it, had never been read. **One row of the family already knew**: §14.8.4.7.2's note,
written in the four-hundred-and-eighteenth, cites "§14.8.4.7.3's link element" correctly. Two rows
about one mechanism, disagreeing — the seventh failure shape, one row apart. ADR 0273; `ClauseIndex::title`
takes a number's *last* heading now, which moves exactly one title of the standard's 1017 headings,
counted before the change rather than after.

**The same lag was in the source, in the place the fourth sweep says to look.** `structure.rs`
documented `Annot` and `Form` as "an association between content and …" — Table 368's wording before
Issue #437 replaced it with *encloses* — and §14.8.4.7.2's ledger row was corrected for that exact
word in the four-hundred-and-eighteenth while the two doc comments one directory away were not.

**And one thing the tenth sweep taught about itself.** Seven parent rows state a count of "the N
below" that is neither the number of direct children nor the number of descendants, and **five of
them are a convention rather than a defect**: the count excludes the family's own `General` row
(§14.10.3, §14.10.4, §14.8.4.7, §7.6.4.4, §14.8.2 all read correctly that way). The two that match
nothing were corrected — §7.6.4's "twelve rows below" against twenty, §9.7's "seventeen" against
sixteen — and §7.6.4.4's sentence gave three counts of its own family in one clause of prose, of
which none survived the rows below it.

### The eleventh sweep has a floor, and a quotation of ISO 32000-**1** falls through it

The old §14.8.4.7.3 row quoted `RP` as punctuation "used only when a PDF processor cannot place the
ruby annotation text adjacent to the ruby base text". **ISO 32000-2 does not contain that sentence**;
its Table 369 says "used only when a ruby annotation cannot be properly formatted in a ruby style and
instead is formatted as a normal comment, or when it is formatted as a warichu".

The eleventh sweep did not report it, and the reason is the discriminator ADR 0249 priced: a span is
printed only where it matches the standard for **at least five words and at least half the
quotation** before diverging, which is what separates a misquotation from a claim this project
invented. A quotation of the *older* standard is neither — four words in common and then a different
sentence — so it sits among the 488 spans that occur in no document under `doc/md/` and are never
looked at. **A ledger row written against ISO 32000-1 is a population that sweep cannot see**, and
the ninth — which finds ISO 32000-1's table *numbers* — is the only instrument here that reaches it.

### The ninth sweep's one defect

**§12.5.6.2 said "Table 172's `/CA` is read, as Table 166's entry of the same name"**, and Table 172
states no `/CA` at all: its nine entries are `/T`, `/Popup`, `/RC`, `/CreationDate`, `/IRT`, `/Subj`,
`/RT`, `/IT` and `/ExData`. `/CA` and `/ca` are both Table **166**'s, common to every annotation.
The entry was the markup annotation's in ISO 32000-1 and moved. The row's second half had the right
answer beside the wrong number, which is this sweep's most durable shape.

### The fifth sweep's open item, answered by reading the caller rather than by building one

`ViewState::clear_field` is named by no host, no tool, no fuzz target and no example, and the
four-hundred-and-twenty-ninth left it named on the grounds that "an undo is a host's feature". **The
undo exists and needs something else.** `viewer_core::Open::replay` makes undo a *replay* rather than
an inverse — the log is cleared with `clear_all_fields` and re-applied up to the cursor — so
forgetting one field is what a replay does by not reaching that entry, and a per-field clear is a
second route to a state a host can already reach. The doc comment said "[t]he operation an undo
needs" and now says what it is. **A `pub fn` nothing names is sometimes a function whose job
something else already does**, and that is a question for the caller side rather than a feature to
build.


## Thirty-two rows read in the four-hundred-and-forty-second, and the reading list came from `git blame`

The headline job rather than the sweeps, and the round before it had recorded ~80 of 248 rows never
re-read without being able to say *which*. **`git blame` says which.** Twenty lines over
`git blame --line-porcelain doc/conformance/ledger.toml`: take each row's `note = ` line, take the
commit that last wrote it, and order the `partial` rows by where that commit falls in
`git log --reverse`. A note nobody has touched in three hundred commits is not proof nobody has read
it — but every re-reading session in this file records itself *in the note* ("this row said X until
session N"), so an untouched note is the closest thing to a list of unread rows this project can
produce, and unlike the running estimate it is checkable. Of 590 commits, **40 `partial` rows had
notes last written before commit 110** and this round read 32 of them, oldest first.

**Fourteen of the thirty-two were wrong, a fifteenth row was found beside them, and four moved to
`implemented`.** Every one was opened against the
code and against the clause; the failure shapes are this file's own, and one is new.

| row | shape | was | is |
|---|---|---|---|
| **§11.6** | 2, 5 | "a graphics-state soft mask is reported; transparency groups are the silence" | both were built in the two sessions *after* the note was written — §11.6.6's groups in the seventeenth, `build_soft_mask` in the eighteenth. **424 sessions**, past §9.3's 365 |
| §11.4.1 | 2, 5 | a group's result as a soft mask's source "still reported rather than built" | ADR 0027, eighteenth session |
| §11.3.7 | 1 | "[w]hat is absent is the distinction" | ADR 0234 states a knockout element's shape as a second command; what is absent is a shape *channel* |
| **§8.9.3** | 1, 4 | "only 1- and 8-bit components are unpacked, and 2, 4 and 16 … are refused" | all five, and `tests/bit_depths.rs` opens by saying so — `implemented` |
| §8.9.1 | 5 | the same sentence, deferring to §8.9.3's row | §8.9's parent three lines above already said "all five bit depths" |
| §8.9.6 | 5 | `partial` "for §8.9.6.2's last sentence" | §8.9.6.2's own row had answered it |
| §8.9.6.2 | new | quoted "the effect is to smooth the edges of the image" and called it "a recommendation" | **ISO 32000-2 does not contain that sentence.** It is ISO 32000-1's; this standard's is a `shall` about a different noun — "the effect shall be to smooth the edges of the mask, not to interpolate the painted colour values" |
| §12.5 | 2, 5 | "no annotation responds to input, and §12.6's actions are not read" | §12.6.1 and §12.6.2 are `implemented`; ADR 0177 takes a press to a widget |
| **§9.8.3** | 3, 5 | Table 122's four entries, "none of which is read" | `/Style`'s PANOSE chooses a substitute and `/FD`'s classes are read — by the session that wrote §9.8.3.2's and §9.8.3.3's rows, which did not touch the two rows above them |
| §9.8.3.1 | 3, 5 | the same sentence, one row down | as above |
| §7.6.6 | 2 | `partial` because "the /EFF path itself is unexercised — no embedded file's bytes are ever read" | a host has extracted an attachment's bytes since §7.11.4's panel. **Implemented this round**, below |
| §8.6.6 | 14th sweep's | a note naming nothing owed | §8.6.6.5's `/NChannel` and `/Colorants`, which nothing reads |
| §12.5.6.13 | new | "/Path supersedes it as the table requires" | **Table 185 requires nothing of the kind.** §12.5.6.9's Table 181 does say it of `/Vertices`; Table 185 marks `/InkList` "(Required)" flatly. Drawing the `/Path` alone is this crate's choice — `implemented`, with the test the row said it lacked |
| §14.11.2 | 6 | `partial`, owing §14.11.2.2's guidelines "which are `inapplicable` on their own row" | a child that owes nothing cannot make its parent owe something — `implemented` |
| §14.11.2.1 | 6 | `partial` above a note naming nothing owed | the boxes, their defaults and "its intersection with the media box" are all here; the rest of the clause is a NOTE about presses — `implemented` |

**The seventeen rows read and kept** are worth naming, because a row read and left alone is the only
way this file learns a population has stopped drifting: §7.4.2, §7.4.6, §7.4.7, §7.6.4.4.2, §7.9.2,
§8.6.5.8, §8.6.5.9, §8.6.6.5, §8.7.4.1, §8.7.4.3, §8.9.6.4, §8.10, §8.10.4, §8.10.4.1, §8.10.4.3,
§9.7.5.4, §12.5.6.3 and §12.6.4.8. Three were checked by grepping the whole tree for the entry the
row calls unread — an image dictionary's `/Intent`, a shading's `/Background` and `/AntiAlias`, a
`DeviceN`'s `/NChannel` — which is the second sweep run on one row at a time, and all three held.

### Clause 11's `partial` rows are clean now, and the reason they were not is legible

Twenty-seven of them, and **the blame order says why three were stale and twenty-four were not**:
§11.3.7, §11.6 and §11.4.1 are the only three whose notes were last written before commit 90, and
every other clause 11 row has been rewritten within the last ninety commits by the rounds that built
transparency groups, the knockout shape, the page group and the press. **A family being worked on
is a family whose rows are maintained; the three that fell out are the three nothing since had cause
to open.** All three were the same failure — a note describing what the tree owed two sessions
before the thing arrived.

### The row that turned into work: §7.6.6's `/EFF`

Table 20's `/EFF` names "[t]he name of the crypt filter that shall be used when encrypting embedded
file streams that do not have their own crypt filter specifier", and nothing in this tree read it.
The row's reason for that was "no embedded file's bytes are ever read", which was true when it was
written and false since `Command::Extract` and §7.11.4's attachments panel — so a document writing
`/StmF /Identity` beside a cipher in `/EFF` handed its attachment back as ciphertext, silently, to
the one verb that asks for one. `Document::stream_method` reads it now, after the stream's own
`/Crypt` specifier and before `/StmF`, which is the entry's own order; §7.6.6's rule that "related
files ( RF ) shall use the same crypt filter as the embedded file ( EF )" holds by construction,
both being `/Type /EmbeddedFile` streams.

**The corpus cannot exercise it**, which is why the test is a corpus document with one entry blanked
out. `encrypted-attachment.pdf` and `auth-event-ef-open.pdf` state *both* routes to `StdCF` — the
stream's own `/Crypt` specifier and `/EFF` — so the reader's answer is the same whether or not it
reads the second. Blanking the specifier with spaces leaves every byte offset where the
cross-reference table says it is, so what is opened is a real producer's file minus one entry, and
both halves of Table 20's sentence are asserted: with `/EFF` the stream takes `StdCF` and §7.6.6
refuses it for want of a key, and with `/EFF` blanked as well it falls back to `/StmF` and the bytes
pass through. Trap 8's shape exactly — a `shall` no document on disk can rank.

### And two source claims went with them

The fourth sweep's rule, run on this round's own nouns rather than as a sweep: `appearance.rs` said
in two doc comments that a `/Path` supersedes `/InkList` "which the same tables say shall be ignored
when it is present", which Table 185 does not say — the same sentence as §12.5.6.13's row, one
directory away. And `encryption.rs`'s own test comment said the two corpus documents reach `StdCF`
"only through `/EFF`", which is how the entry came to look exercised: they reach it through the
stream's `/Crypt` specifier, and `/EFF` sits beside it doing nothing.

## The blame list's last three, and the first of them was two departures

Run again in the four-hundred-and-fifty-eighth. Of 607 commits, **20 `partial` rows have notes last
written before commit 110**, and seventeen of them are the ones the four-hundred-and-forty-second
recorded as read and kept — which is this file's own flaw arriving as predicted, because keeping a
row edits nothing. **The three that are genuinely unread are §12.5.4, §12.5.6 and §12.5.6.8**, all
three written in one sitting, all three about annotations.

**§12.5.4 was two silent departures, and neither is a shape the sweeps look for** (ADR 0293):

| sentence | was | is |
|---|---|---|
| §12.5.4's "the border shall be drawn completely inside the annotation rectangle" | applied to the four rectangular styles; Table 168's `U` put its path *on* the bottom edge | the edge raised by half the width, the same arithmetic the other four get |
| Table 166's "[i]f an annotation dictionary includes the BS entry, then the Border entry is ignored" | obeyed for the width, the style and the dash; `/Border`'s corner radii were read whatever `/BS` said | `/Border` is read only where no `/BS` is present |

**The first is worth a shape of its own: a clip absorbed the departure.**
`Constructed::bounded` clips a link's construction to `/Rect`, so the half of the underline that
fell outside was cut off rather than drawn — and what a reader saw was a line half the width the
document asked for. Neither a report, nor a refusal, nor a distance from a reference names that.
The four subtypes that opt *out* of that clip because their geometry is in default user space
(§12.5.6.7's `/L`, §12.5.6.9's `/Vertices`, §12.5.6.10's `/QuadPoints`, §12.5.6.13's `/InkList`)
are exactly the ones where the same mistake would have been visible. **Ask what a bounded
construction's box is absorbing.**

**The second is a precedence rule losing to an argument from completeness.** The corner radii are
the one thing Table 166's array states that Table 168 has no entry for, so reading them beside a
`/BS` looks like taking a value from the only place that supplies it. A precedence rule is not
about which entry is better informed; it is about which entry is read. `crates/pdf-model/examples/border_precedence_census.rs`
counted the population before either fix was believed: of 33 781 annotations stating no `/AP`, one
states a `U` border and none of the six stating both a `/BS` and a non-zero radius is a subtype
whose border this crate constructs — trap 8, and the fixture is a pair differing only in the `/BS`.

## The blame list's last two, and both rows were right

Read in the four-hundred-and-fifty-ninth (ADR 0294). §12.5.6's "Table 171's subtypes are all
recognised" and §12.5.6.8's account of the inscription and of `/RD`'s order both hold against the
code, checked entry by entry. **The block was real anyway and neither departure was the row's
subject**, which is the finding worth keeping: reading a row is not the same as reading the clause
family the row sits in.

| clause | sentence | was | is |
|---|---|---|---|
| §12.5.6.4 | "when open, it shall display a popup window containing the text of the note", and Table 175's `/Open` | read nowhere in the tree — one reader of `"Open"` in `crates/`, and it reads Table 186's entry on the *popup* | `popup::opens_with_the_page`, as a disjunction of the two entries |
| §12.5.6.8 | §12.5.4's "the width and dash pattern for the lines drawn by line, square, circle, and ink annotations" | a `/BS` `/S` of `B` or `I` reported as an appearance this crate could not derive | nothing reported; the style is not an entry this subtype's `/BS` supplies |

**The first is a shape none of the fourteen sweeps has, and it is now `doc/habits.md`'s sixth
refusal shape.** §12.5.6.4's note had *already been corrected once*, in the
three-hundred-and-twelfth session, by retiring its refusal with the words "the popup window /Open
selects … is drawn since" — a sentence that is true about the window and says nothing about the
entry. A row in that state names no blocker, no missing vocabulary and no absent architecture, so
every sweep above passes it. **The grep that finds it takes the entry named in the correction and
asks who reads it**, which for this row was one line:

```sh
grep -rn '"Open"' crates/ | grep -v tests
```

**And the second is trap 11 rather than trap 5**: the report fired where the clause asks for
nothing, which costs a page off the oracle's judged set rather than a mark off the page. The
condition to test is whether the *table* gives the entry a meaning for that subtype, not whether the
entry is present — Table 180 and Table 181 both restrict `/BS` to a width and a dash, and
§12.5.6.9's polygon had it right all along, which is what made the inconsistency legible.

## A fifteenth sweep, and it is the first that asks who reads an *entry the clause states*

Built in the four-hundred-and-sixtieth, for the sixth refusal shape the round before named and
left with no instrument. The fourteen above all read what a row *says* — its blocker, its
capability, its retired string, its citation, its table number, its arithmetic. A row that retires
its refusal by naming a capability that arrived says nothing wrong: it names something that
exists. So this sweep reads no reason at all.

Thirty lines of Python over `ledger.toml`, `doc/md/ISO_32000-2_sponsored_EC3.md` and the source
roots:

1. **The population** is every row whose note explains itself by an arrival — `since the …th
   session`, `since ADR NNNN`, `is drawn since`, `now has`, `the window that arrived`.
2. **The entries** are the ones the clause *itself* states: every `Table N -Title` heading printed
   inside the clause's own span of the standard, whose first column is `Key`, and every key in it.
   Not the entries the note names — the note is what is not to be trusted.
3. **Two questions per entry.** Does any `.rs` file under `crates/`, `tools/` or `fuzz/` contain
   the string at all? And does any file the row itself names in `code = [...]`? **The second is
   the discriminator the four-hundred-and-fifty-ninth needed by hand**: `"Open"` *was* named in
   `crates/`, by the popup reader, under a different table, so question 1 alone passes the very
   row this sweep exists for.

**Its first run: 168 rows in the population, 30 of them stating an entry their own code does not
name — 57 entries, 38 named nowhere at all and 19 named only elsewhere.** Most are refusals the
row already describes and `CLAUDE.md` already closes: §12.6.4.6's `/Win`, §12.6.4.9's four sound
entries and §12.6.4.10's two are excluded actions, Table 200's `/DS` is an ECMAScript action,
Table 177's and Table 228's `/DS` are XFA's default style string, §12.7.5.5's seed values are the
signer's. Read the hit before believing it, as with every sweep here — and note the conversion
trap in passing: Table 200's rows come out of `doc/md/` with the columns shifted, so four of its
five keys read as prose and only one reached the output.

**The one that was work is §12.5.6.15, and it is the sixth shape exactly.** The row is
`implemented`, its note is "**all four are drawn since the two-hundred-and-sixty-sixth session**"
— an arrival — and it disposed of Table 187's **required** `/FS` in eight words: "the embedded
file, which is not a rendering question". True about rendering, and the reason nobody asked the
other question. `/FS` was named in `crates/` and in neither of the row's own two files: the one
reader is §12.6.4.4's embedded go-to, which follows a *target path* into an attached document —
a different clause's use of the same entry, which is `/Open`'s shape one clause along.

So a document that attached its file to a **page** rather than to §7.7.4's name tree carried a
file no part of this program could reach: the icon was drawn and nothing else. That is the
corpus's one file attachment annotation and all six in ISO 32000-2's own PDF. ADR 0295.

**And the sweep found its own refusal's other half, which is worth more than the finding.**
§7.11.4.1's row *named the missing caller in so many words* — "§12.5.6.15's file attachment
annotations being the caller the clause names, and not yet built" — so the ledger held the answer
and the question in two rows of two different families, which is the seventh failure shape
(two rows about one mechanism, disagreeing) with the disagreement being about *this tree* rather
than about the standard. No grep above finds it: the sentence names a clause, not a capability, a
blocker or a retired string.

### Its second run, and the row that hid a `shall` behind a true sentence

Re-run in the four-hundred-and-eightieth session on the tree twenty sessions later: **182 rows in
the population, 24 stating an entry their own code does not name, 43 entries** — 8 named nowhere
and 35 only elsewhere. Read that shape before reading the rows: the *entries* fell and the "named
only elsewhere" share rose, because a row whose `code` array has gone stale beside a crate that
grew a second file reads exactly like a defect and is not one. **The sweep is a reading list and
not a gate** — the fourteenth's own lesson, and this run is the second document of it.

**The hit that was work is §12.5.6.2's `/IRT` and `/RT`**, and the reason it survived fifteen
sweeps is that the row's sentence about them is *true*: they "reach a comments pane rather than a
raster", which is exactly what a relationship and a reply type do. What that sentence stopped
anybody reading is the paragraph four below Table 172, where the same two entries make a **group**
and hand it nine shared entries — "the corresponding entries in the subordinate annotations shall
be ignored. These entries are Contents (or RC and DS ), M , C , T , Popup , CreationDate , Subj ,
and Open ." Two of those are not a pane at all: `/C` is ink and `/Contents` is what §12.5.6.6 draws.

The corpus has **one** `/IRT` in 34 835 annotations, so it could not rank this and never would
have. ISO 32000-2's own PDF has 2074, 322 of them `/RT /Group`, and **213 popup windows hanging
off a subordinate that came up blank** — the erratum's words are in the primary. ADR 0315.

**The lesson for the sweep is about its *hits*, not its population.** Question two prints an entry
the row's own files do not name; what decides whether that is work is whether the row's disposal of
it is a claim about *the entry* or about *the clause*. "Not a rendering question" was the first
kind and was wrong; "reaches a comments pane" was the second kind, was right about the entry, and
was silent about the paragraph the entry points at. **Read what the clause does with an entry, not
only what its table says the entry is.**

### Its third run, and the first one a program did

**The sweep was never committed.** It was described here and rebuilt from the description by the
four-hundred-and-eightieth session before it could be run at all, which is the failure `CLAUDE.md`'s
"what is written down is the command that counts it" exists to prevent — and a reconstruction is not
the same instrument twice, which is why this run's numbers are not the previous two's. It is
`conformance::entries` now, with the invocation `doc/todo/02` §4 states:

```sh
cargo run --release -p conformance --bin entries
```

**And it gained the one filter the second run's lesson asks for.** That run ended by saying a hit is
work only where the row's disposal of the entry is a claim about *the entry* rather than about *the
clause* — a question no program can settle. What a program can settle is the case with no claim at
all: **the row's own note never writes the key**. So each reported entry says whether the note names
it, and the ones it does not are the list to read first.

Its first run as a program: **215 rows in the population, 41 stating an entry their own `code` does
not name, 102 entries — 30 named nowhere in the tree and 72 only elsewhere, of which 43 are not named
by the row's own note either.** The population is larger than the reconstruction's for two reasons
worth knowing before comparing the numbers: the arrival vocabulary is seven phrases rather than five,
and a clause's tables are taken from its **own** text rather than from its span, which excludes a
parent row from owning every subclause's table.

**The hit that was work is §12.6.4.2's `/SD`**, and it is the sixth refusal shape for the third time
in twenty-five rounds. Table 202 gives a go-to action a structure destination beside its page
destination and says which wins — "[i]f present, the structure destination should take precedence
over destination in the D entry" — and `grep '"SD"' crates/` found **nothing**, in a tree whose
§12.3.2.3 row has read `implemented` since the algorithm that resolves a structure element to a page
landed. The capability was there, the entry that turns it on was wired to nothing, and §12.6.4.2's
own note said "`/SD`, the structure destination alternative, resolves through §12.3.2.3 like any
other" — a true sentence about `Destination::read` and a false one about this clause. ADR 0319.

**Two things about the output that are the instrument rather than the tree.** Table 165's `/name` and
Table 200's `/DS`, `/DP` and `/dictionary` are `doc/md/` shifting a table's columns, which the
four-hundred-and-sixtieth and -eighty-first sessions both met; and a row whose `code` array has gone
stale beside a crate that grew a second file prints an entry the right reader does name. Neither is
worth tightening the sweep for — read the hit, as with every sweep here.

## All fifteen run again in the four-hundred-and-eighty-ninth, the second committed as a program, and the instrument caught a split

Five rounds since the last full sweep, three of which were pure motion — `content.rs` into
`content/`, `pdf-font`'s `lib.rs` into modules, `viewer-ui`'s binary into modules — beside
`Command::Present` (ADR 0316) and the entries sweep itself (ADR 0319). Over `ledger.toml`,
`crates/`, `tools/`, `fuzz/` and, for the fourth sweep, `doc/adr/`:

- **The second sweep, as a program** (`cargo run --release -p conformance --bin unread`, ADR
  0324). First run: **62 rows claim an entry unread, 171 keys between them — 55 confirmed
  (quoted by no source), 116 quoted somewhere over 49 rows, 53 by the row's own code.** Most of
  the 116 are the two known shapes at machine scale: a note quoting its own retired wording, and
  one short key in three clauses. **One was a defect**: §7.5.5 said "`/Info` is unread because
  §14.3.3 deprecates it" while `metadata::Information::read` takes the entry off the trailer for
  the properties panel — the deprecated-so-unread disposal, one clause claiming another's reader
  does not exist.
- **Entries (sweep 15)**: 217 rows in the population; **140 entries over 43 rows before the
  instrument correction, 106 over 42 after it** — the splits kept `content.rs` and
  `pdf-viewer.rs` as module roots *so that citations stay valid*, and the sweep read each listed
  path as one file, so 34 entries moved to "named only elsewhere" with nothing in the tree
  changed. `entries::covered_by` now applies Rust's own rule — a module root `foo.rs` owns
  `foo/` — and `unread` shares it. The remaining hits are the known populations; **the one
  worked is §14.13's**, below.
- **Quotations (sweep 11's prose sibling)**: 2885 quotations in 425 documents, 1366 verbatim, 21
  diverging, **0 defects** — every divergence is a correction quoting the wording it retired, or
  `doc/md/` losing a table row the PDF has (Table 29's `/OpenAction` default and Table 176's
  wrapped `OpenArrow` row, both checked with `pdftotext -layout`).
- **Ledger quotation marks (sweep 11)**: 1205 double-quoted spans of four words or more under a
  session-local normaliser (not ADR 0249's, so the level is not comparable across runs), 633
  verbatim in `doc/md/`; 45 matched the standard for five words and diverged, 24 of them without
  an `…` elision, and **0 were defects** — every one read down to the conversion's own spacing,
  an editorial `[bracket]`, or a correction quoting the wording it retired, with the two
  closest-looking (§11.6.6's B(Cb,Cs) sentences) verbatim once the punctuation is normalised.
- **Expired blockers (sweep 1)**: 7 over the ledger, 14 over the source roots — four of the
  ledger's the quoted retired wording inside a correction, §12.10.2's wait on §12.10.3 and
  §12.5.6.22's on printing real, every source hit past tense or true.
- **Capability reasons (sweep 3)**: 32 over the ledger, 78 over the source roots, every source
  hit a true statement about a boundary a crate keeps.
- **Retired claim (sweep 4)**, over the five rounds' nouns and this round's own — `Present`,
  the split paths, `/SD`, and **`silent` as a noun**. It paid three times on the last: §14.12.4.1
  and §14.13.8 each called a neighbour "`silent`" in a ledger that has had no such row since
  Annex O's five were built, and §14.8.6 used the word for a requirement addressed to a
  *document*. Two ADRs also carried numbers this round's ninth-sweep corrections retired (0284's
  Table 237, 0295's Table 172), amended in the same commit per ADR 0265's rule.
- **Caller sweep (5)**: 280 distinct `pub fn` names in `pdf-model`, 92 named by no host, **76 by
  no host, tool or fuzz target** — the same three known populations.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (sweep 7)**: 70 of 80 rows name vocabulary the source names, none wrong.
- **Citations (sweep 8)**: 3 hits, all three the known false positive — a correction quoting the
  `doc/todo/20` or `doc/todo/37` it retired.
- **Table numbers (sweep 9)**: 409 headings parsed, ~1000 citations checked, 105 suspects —
  most of them this run's own parser truncating a table the conversion splits across header
  rows — and **ten defects, a block**: "Table 99's `/Configs`" in §8.11.1 and §8.11.4 (`/Configs`
  is Table **98**'s, the properties dictionary's), "Table 172's `/Contents`" in §12.5.6.15 and
  four source comments (`/Contents` is Table **166**'s), "Table 237's `/SV`" in §12.7.5.5 (Table
  **235**'s, whose *value* is 237), "Table 354's `/StructTreeRoot`" in §14.7.2 and "Table 408's
  `/DPartRoot`" in §14.12.4.1 (both the catalog's Table **29** entries, given to the tables their
  values point at), "Table 168's `/N`" twice in the source (Table **170**'s appearance
  dictionary), "Table 257's `/DocMDP` level" (the level is `/P`) and "Table 11's `/DecodeParms`"
  (Table **5**'s, whose CCITT entries Table 11 defines).
- **Parent counts (sweep 10)**: 10 hits, 0 defects — §11.7's and §14.11's double counts are
  their own corrections quoting the retired numbers.
- **Sweep 14**: 24 hits, every one naming a debt in words outside the sweep's vocabulary.
- **The errata (sweep 12)**: "151 struck passage(s) … that doc/md/ still carries as current
  text", unchanged, so the population is still read to the end; the quoting landings are the
  known in-place annotations, `doc/todo/01`'s own records among them.

### Seventeen rows read off the blame list, from the commit-138-to-165 band

The band above commit 110 is the read-and-kept set; this round read the next seventeen, oldest
first: §14.6.2, §14.9.2, §14.9.2.2, §14.7.4.2, §14.8.6, §14.8.6.2, §14.8.2, §14.8.2.2.1,
§14.8.2.2.2, §14.8.2.3, §12.11.3, §12.11.5, §12.11.6, §7.7.4, §14.13, §14.13.2 and §14.13.8.
**Six were wrong, and one turned into work**:

| row | shape | was | is |
|---|---|---|---|
| §14.6.2 | 3, 5 | property-list contents read for optional content, `/MCID`, §14.9's four "and nothing else" | also §14.8.2.2.2's Table 363 artifact entries and §14.13.5's associated files — a list of what *is* read is maintained by nobody either |
| §14.9.2.2 | 5 | descriptor `/Lang` shares its debt with "§9.8.3's `/Style` and `/FD`" | both have readers (`pdf_font::panose`, `substitute.rs`), which §9.8.3.2's and §9.8.3.3's own rows said all along; `/Lang` is the one left |
| §14.8.2 | 5 | "[w]hat is left is the reading-order half, which needs a consumer" | §14.8.2.5 is `implemented` with consumers; what is left is what the children name |
| §7.7.4 | 3, 5 | "Two are read" of Table 32's ten trees; `/Pages` and `/Templates` owed to §12.7.7 | four are read — `named_page::NamedPages` runs both trees' invariants, and the row went on owing them after it landed |
| §14.8.6 | wording | "what is `silent` is the requirement" | the requirement addresses a *document*; the ledger's status word retired from the prose |
| §14.13.8 | 4 | §14.12 "a `silent` row of its own" | `partial`, with `document_part::first_page` following `/DParts` from a jump's own part |

**The one that turned into work is §14.13.3**, and it is ADR 0295's shape one family over: the
row read `implemented` on `attachment::associated`, and that function had **no caller outside its
own tests** — so a payload a producer associated with the catalog and filed under no name was a
file no panel could list and no host could extract. `attachments` now appends the catalog's
associated files to §7.7.4's tree, deduplicated by embedded stream because PDF/A-3 writers state
one payload both ways, and the fixture pins both the reachable file and the deduplication. The
structure-element and `DPart` `/AF` sites keep their rows' own debts: each needs a walk, not a
list.

## All fifteen run again in the five-hundred-and-first, the first sweep committed as a program, and the sweep taught its own third noise shape

Six rounds since the last full sweep, landed in one wave — JPEG 2000's reduced-resolution decode
(486), RSASSA-PSS verified (487), the selection-gate design (488), the accessibility parent-tree
route (490), the interface font's character route (491), the knockout backdrop and the group's
press (492), two performance rounds (493, 495) and `/CL`'s callout (494). Over `ledger.toml`,
`crates/`, `tools/`, `fuzz/` and, for the fourth sweep, `doc/adr/`:

- **Expired blockers (sweep 1), as a program** (`cargo run --release -p conformance --bin
  blockers`, ADR 0336). First run: **20 blocker sentences over the ledger (6 printed as expired,
  9 holding, 5 naming no clause) and 26 over the source roots (9, 10, 7)** — and 0 defects, every
  printed-expired hit being a correction quoting the wording it retired or a contrastive "while
  §X says". What the program adds over the grep is the judgement a person used to redo each run:
  a blocker naming a clause is checked against that clause's own row, expired-first. **Its first
  run taught it a third noise shape**, now in the module doc beside the other two: §12.10.2's
  "needs §12.10.3's external references" waits on the EPSG registry that §12.10.3 *points at*, so
  a settled row settles nothing about the wait — a clause can be named as the route to something
  outside the standard, and that row now names the registry rather than the clause.
- **Retired claim (sweep 4)**, over the six rounds' nouns — `target_resolution`, `RSASSA-PSS`,
  `/CL`, `character_glyph`, `elements_on_page`, `/StructParents`, `knockout`, `blending space`,
  `Node`, `get_key_of`. **It paid twice.** `examples/free_text_census.rs` still said `/CL` is
  "the callout line `doc/todo/33` holds open", one round after 494 closed exactly that item and
  amended the todo — the census that counts the population was not swept with it. And
  `doc/todo/README.md`'s index line for item 31 still listed "the empty answer any page but the
  first of a large tagged document gets" among what is left, one wave after 490 closed it (ADR
  0325) and amended the file — **an index row decays at its item's pace, not its own**, which is
  the two-hundred-and-sixty-ninth's sibling-crate lesson one directory up.
- **Unread (sweep 2, program)**: 61 rows claim, 167 keys; 52 confirmed, 115 quoted over 49 rows,
  50 by the row's own code — the known one-short-key population, 0 defects.
- **Entries (sweep 15, program)**: 222 rows in the population, 42 stating an entry their own
  `code` does not name, 106 entries — 29 named nowhere, 77 only elsewhere, 36 not named by the
  row's own note. The known populations; nothing worked.
- **Quotations (program)**: 2951 quotations in 446 documents, 1398 verbatim, 22 diverging, **0
  defects** — every divergence a correction quoting retired wording or a quotation of another
  document (an RFC's SHALL) sharing five words with the standard.
- **Capability reasons (sweep 3)**: 43 over the ledger, 148 over the source roots. **One
  defect**: §14.7.2 still said "the data is this crate's and the consumer is not — nothing in
  this program yet hands a structure tree to anybody" four sentences above its own appended
  correction — failure shape 6, corrected by rewriting the paragraph rather than appending again.
- **Caller sweep (5)**: 284 distinct `pub fn` names in `pdf-model`, 92 named by no host, 76 by
  no host, tool or fuzz target — the same three known populations.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (sweep 7)**: 27 of 80 rows name vocabulary the source names, none wrong.
- **Citations (sweep 8)**: 4 hits, all the known correction-quoting-its-pointer shape
  (`doc/todo/20` twice, `doc/todo/37` twice).
- **Table numbers (sweep 9)**: 1159 citations checked, 90 suspects, **0 defects** — the first
  fully clean run this sweep has had over ledger and source together. Every suspect read down to
  a correction quoting the retired number (188/189/190/191/172), prose naming a nearby key, a
  table's *value* named beside its entry, or the parser missing a split-header table's keys
  (Table 122, Table 377), which is the known instrument shape.
- **Parent counts (sweep 10)**: 6 hits, 0 defects — two are corrections quoting their own
  retired double counts.
- **Ledger quotation marks (sweep 11)**: 1224 spans of 4+ words under a session-local
  normaliser, 848 verbatim in `doc/md/`, 40 matching five words and diverging, **0 defects** —
  every one an `…` elision, a correction quoting retired wording, or the conversion's broken
  words ("hierarch y").
- **Sweep 14**: 5 hits, every one naming its debt in words outside the sweep's vocabulary
  ("is not,", "writer-side", "aggregate of the two below").
- **The errata (sweep 12)**: "151 struck passage(s) of 4 words or more that doc/md/ still
  carries as current text" over all fourteen PDFs, unchanged, so the population is still read to
  the end; every quoting landing is a known in-place annotation, `doc/todo/01`'s own records
  among them. **Run over one PDF alone it prints 150**, which is worth knowing before reading a
  moved number as movement: the population is the *annotations'*, so the invocation's document
  list is part of the count.

### Thirteen rows read off the blame list, the next band after commit 165

§12.8.\*'s four rows in the band (§12.8.2.1, §12.8.4.1, §12.8.4.5, §12.8.5.3) were left to the
parallel round that owns the signature family.
Oldest first: §14.8.4.2, §14.8.5.3, §12.10, §12.10.1, §12.10.2, §12.9, §12.9.1, §12.3.5.2,
§12.3.6, §9.8.3.3, §12.7.8.3.2, §12.7.7 and §12.7.8.3.3. **Two were wrong; eleven were read and
kept, each recording the evidence that kept it** — the grep run, the function checked, the
boundary confirmed — which is what moves the blame pointer without a stamp.

| row | shape | was | is |
|---|---|---|---|
| **§12.3.5.2** | 2, 5 | "`partial` for the panel" | the panel arrived in the three-hundred-and-fifty-second (ADR 0202) and draws this clause's folder tree; `partial` now for the entries the walk does not read (a folder's own `/Thumb`, `/CreationDate`, `/ModDate`), with Table 159's `/Free` named rather than owed — its `shall` fires "when a new folder is added", a verb this program does not have |
| **§12.10.2** | blockers' 3rd noise shape, live in a row | "needs §12.10.3's external references" — expired-looking the day §12.10.3's row settled | the wait named against the EPSG registry and ISO 19162 grammar the entries point at, which is what it always was |

**And one precision recorded on a kept row rather than a correction**: §14.8.4.2's
"`standard_role` is the question a consumer asks" is true in two halves across the boundary —
`viewer_core::accessibility` crosses `Tree::role`'s mapped string and `viewer_accessibility::role`
reads it as a standard type — so the fifth sweep listing `standard_role` as unnamed by hosts is
the two-crate split, not an unasked question.

## All fifteen run again in the five-hundred-and-tenth, the third sweep committed as a program, and the generator caught re-stamping a retired sentence

Twelve rounds since the last full sweep — the codespace inversion (502), the reference's two
answers (503), the function grid (504), the refused photograph (505), the borrowed token (506),
the map's two answers (507), the damaged prefix (508) and the rest of the wave. Over
`ledger.toml`, `crates/`, `tools/`, `fuzz/` and, for the fourth sweep, `doc/adr/`:

- **Capability reasons (sweep 3), as a program** (`cargo run --release -p conformance --bin
  capabilities`, ADR 0345). First run: **ledger 47 sentences — 34 witnessed by the tree, 40
  about the program, 7 about one crate; source 142 — 116 witnessed, 78 program, 64 crate.**
  What the program adds over the grep is the two judgements each run redid by hand: the lacking
  noun grepped against the tree with the witness printed, and the program-versus-crate subject
  tell. **One ledger hit was live**: §14.7.6.3's "a validity mechanism for a processor that
  edits, which this is not" — expired since the hundred-and-thirty-fifth session made this an
  editing program, kept as a *choice* on the clause's own words (deprecated with PDF 2.0, and
  the increment is a "may"). The fifth failure shape, in an `implemented` row.
- **Retired claim (sweep 4)**, over the twelve rounds' nouns — `save_round_trip`,
  `crypto-bigint`, `addressable_codes`, `annotation_rectangles`, `Sampled`, `DeferredColours`,
  `FUNCTION_GRID`, `contradicted_frame`, `fixed_format_number`, `elements_on_page`,
  `page_object`, `Corrupt`, `truncated` — clean; the rounds' corrections were made everywhere,
  §7.4.4.1's and §7.4.4.2's rows already carrying the five-hundred-and-eighth's rework. **Run
  over the retired exclusion wordings it paid twice, and the second is a mechanism**:
  `Exclusion::Xfa`'s doc still read "deprecated by ISO 32000-2 itself and specified outside it"
  (amended in `CLAUDE.md` a hundred and fifty sessions ago), and the ledger header's own status
  definition still read "we do not create files" — because **the header is generated and the
  retired sentence lived in the generator**, `bin/ledger.rs`'s `PREAMBLE`, which stamped it back
  over the correction session 137 made in the file. `doc/ledger-and-claims.md` had recorded the
  header as corrected ever since. A correction to generated text is not a correction until it
  reaches the template.
- **Blockers (program)**: ledger 20 sentences — 6 expired, 9 holding, 5 naming no clause;
  source 26 — 9, 10, 7. 0 defects; every expired hit a correction quoting retired wording or a
  contrastive "while §X".
- **Unread (program)**: 63 rows claim, 172 keys; 53 confirmed, 119 quoted over 51 rows, 54 by
  the row's own code — the known one-short-key population, 0 defects.
- **Entries (program)**: 232 rows in the population, 44 stating an entry their own `code` does
  not name, 108 entries — 29 named nowhere, 79 only elsewhere, 37 not named by the row's own
  note. The known populations (excluded actions, pronunciation's declined permission); nothing
  worked.
- **Quotations (program)**: 3049 quotations in 470 documents, 1441 verbatim, 22 diverging, 0
  defects — every divergence a correction quoting retired wording or another document's
  five-word overlap.
- **Caller sweep (5)**: 286 distinct `pub fn` names in `pdf-model`, 92 named by no host, 77 by
  no host, tool or fuzz target — the same three known populations, the wave's new names
  (`contradicted_frame`, `decode_parts`, `field_locks`) all reached by `pdf-model` itself or a
  test.
- **Arithmetic (sweep 6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (sweep 7)**: 66 of 80 rows name source vocabulary on a session-local loose
  stop-list, and **no row in the population changed since the five-hundred-and-first's run** —
  checked with `git diff` over the ledger's status lines, which is the cheap way to carry a
  clean read forward.
- **Citations (sweep 8)**: 3 hits, all the known correction-quoting-its-pointer shape
  (`doc/todo/20`, `doc/todo/37` twice).
- **Table numbers (sweep 9)**: 1024 citations checked, 61 suspects — most of them the
  session-local parser's own stop-list eating `/Name`, `/Type` and `/Style` and the known
  split-header tables — and **one defect**: `spec_annotation_census.rs` said "Table 353's
  `/MarkInfo`", the catalog's Table 29 entry attributed to the table its value points at,
  which is the four-hundred-and-eighty-ninth's ten-defect block shape one more time.
- **Parent counts (sweep 10)**: 4 hits, 0 defects — §11.7's and §11.7.4's corrections quoting
  their retired counts, §12.4's and §14.3's histories accurate against their children.
- **Ledger quotation marks (sweep 11)**: 1245 spans of 4+ words under a session-local
  normaliser, 695 verbatim, 34 matching five words and diverging, **0 defects** — every one an
  `…` elision, the conversion's broken words ("hierarch y", "text-tospeech",
  "implementationdependent"), or §8.4.4's correction quoting its retired wording.
- **Sweep 14**: 15 hits under a session-local debt vocabulary, every one naming its debt in
  words outside that vocabulary ("writer-side", "Counted, not parsed", "which needs the
  validator"). 0 defects.
- **The errata (sweep 12)**: "151 struck passage(s) of 4 words or more that doc/md/ still
  carries as current text" over all fourteen PDFs, unchanged — the population is still read to
  the end.

### Twelve rows read off the blame list: §12.8.\*'s four freed rows, then the band from commit 185

Oldest first: §12.8.2.1, §12.8.4.1, §12.8.4.5, §12.8.5.3 — the four the five-hundred-and-first
left to the parallel signature round, free again now that the family landed — then §12.5.6.23,
§7.10.2, §8.6.4.4, §7.11.3, §12.7.8.3.4, §12.7.8.3.1, §11.7.4.4 and §9.6. **Two were wrong,
reading §9.6 found two more beside it, and three moved to `implemented`**:

| row | shape | was | is |
|---|---|---|---|
| **§9.6** | 5 | `partial` for "the one thing actually left: §9.6.5's MacExpertEncoding, the one name Table 112 permits and Annex D's tables here do not cover" | false since the four-hundred-and-fifty-first transcribed Table D.4 (ADR 0286) and corrected §D and §D.4, leaving the §9.6 family carrying the refusal — **`implemented`** |
| **§9.6.5**, **§9.6.5.1** | 5, 7 | both said "MacExpertEncoding is the one named encoding absent: a font naming it is refused and reported" | three rows, one mechanism, the corrected pair in another family — both **`implemented`**, with `name_keyed.rs`'s permitted-names test as evidence |
| **§12.5.6.23** | 5 | redaction's second phase excluded because "`CLAUDE.md` excludes writing files" | the exclusion's pre-amendment wording; the conclusion stands on the amended exclusion's own terms — §7.5.6's append-only update cannot express a phase whose verb is *destroy* |

The ten kept rows each record the evidence that kept them — §7.10.2's absent `/Order` reader,
§7.11.3's absent `/EP` reader, §12.8.4.1's `security_store` counting while `authenticity` reads
the CMS's own certificates — which is what moves the blame pointer without a stamp.

**And the round's own tooling misfire is worth its sentence**: the script appending the kept
rows' evidence had a doubled backslash in its note-matching regex, so five sentences landed at
the end of the *next* backslash-free note — caught by grepping the file rather than trusting
the exit status, which is `doc/todo/02` §6's rule, and re-landed by a script that asserts each
insertion's own clause block.

## What is still owed, named

- **The `partial` rows not yet re-read against the code**, which stood at ~47 of 244 and which
  the four-hundred-and-eighty-ninth took down by seventeen — the whole commit-138-to-165 band of
  the blame list — the five-hundred-and-first by thirteen more, the band from commit 165
  to 185 minus §12.8.\*'s four, **and the five-hundred-and-tenth by twelve: those four, freed by
  the signature family landing, plus the band from commit 185 to 202** (§12.5.6.23 through
  §9.6). **Nothing below commit 202 is unread now** apart from rows read and kept, whose
  evidence is recorded in their notes; the band from commit 213 (§12.7.6.4) onward holds the
  rest — **and the five-hundred-and-seventeenth read that band to commit 409**, which the section
  below records. What replaces the blame order as a
  way in is `doc/habits.md`'s six refusal shapes, the sixth of which was added by the round that
  emptied it.
- **The fourteenth sweep's own vocabulary.** Nine of its sixteen hits are rows that *do* name a debt
  in words the sweep does not hold, and §9.3.1 shows the inverse: a row naming a debt it is wrong
  about is invisible to it. Widening the vocabulary makes the first number smaller and the second
  problem no better; what the run shows is that the sweep is a *reading list* and not a gate.
- **§14.8.6.3's enclosure requirement** — Errata Collection 3 requires a `math` element under a
  `Formula` structure element and the namespace on every MathML type *and attribute*, and `Tree::role`
  checks neither. `doc/todo/48` carries it and this round walked past it in the same family.

## All fifteen run again in the five-hundred-and-seventeenth, the fourth committed as a program, and both of its defects were in ADRs

Six rounds since the last full sweep — the C caller's vocabulary and the flag obeyed backwards
(511), a quorra release's chord floor and round cap (512), the Arabic witness read out (513), the
door that did not ask the signature (514) and the wave beside them. Over `ledger.toml`, `crates/`,
`tools/`, `fuzz/` and — for the fourth sweep, now wider than `doc/adr/` — every Markdown document
under `doc/` this project wrote bar `doc/history/`:

- **Retired claim (sweep 4), as a program** (`cargo run --release -p conformance --bin retired --
  <noun> …`, ADR 0352). First run over seventeen nouns: **544 mentions, 7 nouns carrying both
  shapes, 2 defects and both in `doc/adr/`.** ADR 0235's consequence still read "`RadiosInUnison`
  crosses and is not obeyed" six rounds after the five-hundred-and-eleventh read that sentence out
  of §12.7.5.2.3 — **and that round's own record names ADR 0235 as the fourth place carrying the
  claim**, having corrected the other three. ADR 0337's "what this does not do" still filed
  `freetext_no_appearance.pdf` under `doc/todo/21`'s per-character fallback, which ADR 0348 read
  out in the five-hundred-and-thirteenth while correcting `doc/todo/21`, `doc/todo/22` and
  §12.7.4.3's row. **What the program adds over the grep is the order**: a mention is a
  *correction* or a *standing claim*, and a noun carrying both is where every finding this sweep
  has ever had lives. **And the run taught the program one rule**: a sentence
  containing Markdown's `~~` is a retirement whatever words are inside it, because this project
  strikes the retired sentence and writes the correction *after* it, in the next sentence. **And
  it taught its own noise shape**: a noun that is also an ordinary
  English word costs the run its signal — `prefix` returned 262 mentions and `joining` 36, nearly
  all of them threads and lists being joined. The nouns that paid were the ones a session invented.
- **Blockers (program)**: ledger 20 sentences — 6 expired, 9 holding, 5 naming no clause; source 26
  — 9, 10, 7. 0 defects; every printed-expired hit a correction quoting the wording it retired or a
  contrastive "while §X says" (§10.7.4's zero-width stroke against §8.4.3.2 is the live one, and it
  is a documented choice rather than a wait).
- **Unread (program)**: 63 rows claim, 172 keys; 53 confirmed, 119 quoted over 51 rows, 54 by the
  row's own code — identical to the five-hundred-and-tenth's five numbers, which is the cheapest
  evidence there is that a population has not drifted. 0 defects.
- **Capabilities (program)**: ledger 47 sentences — 33 witnessed by the tree, 40 about the program,
  7 about one crate; source 141 — 115, 77, 64. 0 defects; every source hit a true statement about a
  boundary a crate keeps.
- **Entries (program)**: 236 rows in the population, 113 entries reported over 45 rows — 32 named
  nowhere, 81 only elsewhere, 37 not named by the row's own note. The known populations (the
  excluded sound and movie actions, §12.11.1's `/RH` which §12.11.5's row disposes of by name), and
  **one worked**: §12.8.2.3's Table 258 `/Msg`, the one entry of the usage-rights table nothing
  quotes, which is the producer's stated *reason* for the rights and belongs beside the sentence
  `notes.rs` already prints. Named on the row rather than built.
- **Quotations (program)**: 3123 quotations in 483 documents, 1462 verbatim, 23 diverging, **0
  defects** — every divergence an `…` elision or the conversion's own shape.
- **Caller sweep (5)**: 286 distinct `pub fn` names in `pdf-model` — the same population as the
  five-hundred-and-tenth — with a session-local extraction whose *level* is not comparable across
  runs (101 unnamed by the eight hosts, 82 by no host, tool or fuzz target). The three known
  populations.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (7)**: 43 of 80 rows name source vocabulary under a session-local stop-list; none
  wrong, and no row in the population changed since the five-hundred-and-tenth's read.
- **Citations (8)**: 5 hits and **1 defect**. §8.9.6.1's `doc/todo/20`, §12.7's and
  `viewer-gtk/src/controls.rs`'s `doc/todo/37` are the known correction-quoting-its-pointer shape;
  **`crates/viewer-ffi/src/form.rs`'s `doc/todo/37` is a plain citation, written in the
  five-hundred-and-eleventh to a file the four-hundred-and-ninth deleted**. The audit it names is
  ADR 0235's, and the comment says so now. Worth recording about the instrument as well: a sweep
  over paths reads a *test fixture's* path as a citation, which is why the three under
  `tools/conformance` are not read.
- **Table numbers (9)**: 1104 citations checked, 80 suspects, **0 defects** — every suspect read
  down to the known prose shape (a table named and then a key of the dictionary it describes, or a
  table's *value* beside its entry), and the parser under-collects a flag table's names, which is
  the known instrument shape.
- **Parent counts (10)**: 17 counted claims, 0 defects — the known convention (a count excluding the
  family's own `General` row) and counts about the *clause* rather than the family.
- **Ledger quotation marks (11)**: 1243 spans of 4+ words under a session-local normaliser, 719
  verbatim in `doc/md/`, 30 matching five words and diverging, **0 defects** — every one an `…`
  elision, §8.4.4's correction quoting its retired wording, or the conversion's broken words
  ("hierarch y", "implementationdependent").
- **Sweep 14**: 19 hits under a session-local debt vocabulary. Eleven are the known shape (a debt
  named in words the vocabulary does not hold); **§12.3.2 was in both this list and the blame band**
  and is corrected below.
- **The errata (12)**: "151 struck passage(s) of 4 words or more that doc/md/ still carries as
  current text" over all fourteen PDFs, unchanged, so the population is still read to the end; every
  landing is a known in-place annotation.

### Twelve rows read off the blame list, the band from commit 213

Oldest first: §12.7.6.4, §14.8.2.1, §7.6.4.3.2, §12.3.2, §8.11, §9.8.2, §12.8.2.3, §12.3.2.2,
§12.3.4, §12.7.3, §12.5.6.5, §12.2. **Two were wrong, one hid a `shall` nothing had read, and one
status moved.**

| row | shape | was | is |
|---|---|---|---|
| **§14.8.2.1** | 5, 7 | "a selection is still taken in content order, and the map between the two offsets is what remains" | `Tree::logical_range` **is** that map, answered by `Query::LogicalSelection` and copied by since the four-hundred-and-thirteenth — and §14.8.2.5's own row has said so in those words ever since. Every rule this clause states is a pointer to a subclause and all six are answered — **`implemented`** |
| **§12.3.2.2** | new, and it is a `shall` | `partial` "for `Target::Number` alone" | the integer form *is* read and `page_index_in_target` resolves it against §12.6.4.4's embedded target; what cannot be resolved is a page in a file this reader does not open, which is §12.6.4.3's refusal. **And the clause bounds the bounding box in the same paragraph** — "[i]f any side of the bounding box lies outside the page's crop box, the corresponding side of the crop box shall be used instead" — which nothing had read: `interpret` puts the displayed box at the *origin* rather than clipping to it, so a `/FitB` magnified to fit ink the viewer never draws. `open::content_box` cuts it |
| §12.3.2 | 14th sweep's | `partial` above a note naming nothing owed | its one unsettled child named, and §12.3.2.2's wait with it |
| §12.7.6.4 | precision | what is owed is "any other data format that it supports" | what is owed is **XFDF**, which the clause names; the open-ended tail is satisfied by supporting what one supports |

**The nine kept rows each record the evidence that kept them** — the grep for `AllCap` and
`SmallCap` (nothing names either, and every `Script` in the tree is part of *PostScript*), the grep
for `"CO"`, `crypt.rs`'s shared conversion and its overlay of the password on the padding string,
§8.11.4.4 still naming what is missing, `Query::Preferences` still answering Table 147 whole —
which is what moves the blame pointer without a stamp.

**§12.5.6.5 is the one that was read *whole* rather than from its correction onwards**, which is
this file's own sixth shape and the row that produced it. Both halves agree now, six rounds after
the two-hundred-and-seventy-eighth found them disagreeing.

## All fifteen run again in the five-hundred-and-twenty-fifth, the fifth committed as a program, and the number it prints was the finding

Eight rounds since the last full sweep — the retained frame quorra asks this tree to adopt, the
hollow font, the door that did not ask the signature, three roads becoming three items, the bound a
`Vec` doubled past, the samples a stream stops short of, the two refusals an annex outlived and the
clip that contains a mark. Over `ledger.toml`, `crates/`, `tools/`, `fuzz/` and — for the fourth
sweep — every Markdown document under `doc/` this project wrote bar `doc/history/`:

- **Caller sweep (5), as a program** (`cargo run --release -p conformance --bin callers`, ADR
  0360). First run: **289 distinct `pub fn` names in `pdf-model`, 15 crates naming it in a
  manifest; 174 named by a dependent crate, 19 by a tool or a fuzz target, 73 only inside
  `pdf-model` itself, 21 only by a test or an example, 2 by nothing at all** — 115 that no crate
  under `crates/` asks. What the program adds over the script is the one thing this sweep needs
  most: **its output is a delta and a delta cannot be read off two instruments**, and the level had
  been session-local on every run (246/85 against the previous round's 246/86; 101/82 against
  92/77 with the population unchanged). Three judgements move into it — a consumer is a crate whose
  *manifest* names `pdf-model`, so a dev-dependency's `src/` cannot call it; a definition and a
  `#[cfg(test)]` item inside `src/` are not callers; the three known populations are rungs. **Two
  defects, and the first is the sweep's own shape at one remove**: `ViewState::additions`'s doc
  comment named "what a host asks to know whether there is anything to save" and no host asked it,
  while `viewer_core::Open::dirty` answered that question from the undo log's *length* — so a
  document saved and left open never took its unsaved mark off. `Open::saved_at` is the distance
  the answer needed. The second is `Namespace::is_standard`, named rather than built: the caller it
  waits for is `doc/todo/48`'s second owed item, and §14.8.6.2's `shall` is addressed to a document.
- **Table numbers (sweep 9)**: 1088 citations checked, 69 suspects, **1 defect** —
  `pdf-model/src/view.rs` wrote "Table 177's `/AP`" and, in the *same sentence*, "§12.5.6.6's Table
  177 makes the file's own `/AP` decisive in its `/DA` row", which is the right attribution: `/AP`
  is **Table 166**'s and Table 177 states it of nothing. The file held both forms at once, which is
  this sweep's most durable shape.
- **Blockers (program)**: ledger 21 sentences — 6 expired, 10 holding, 5 naming no clause; source
  26 — 9, 10, 7. 0 defects; every printed-expired hit a correction quoting the wording it retired,
  a past tense, or §10.7.4's documented choice against §8.4.3.2.
- **Capabilities (program)**: ledger 47 sentences — 33 witnessed by the tree, 40 about the program,
  7 about one crate; source 144 — 118, 78, 66. 0 defects; every source hit a true statement about a
  boundary a crate keeps.
- **Unread (program)**: 63 rows claim, 172 keys; **50 confirmed, 122 quoted over 52 rows, 54 by the
  row's own code** — three keys moved from confirmed to quoted since the five-hundred-and-tenth's
  and -seventeenth's identical five numbers, and the reason is the *ledger* rather than the tree:
  23 notes were rewritten in the eight rounds between. The sharpest-looking hit is `/Interpolate`
  in three rows of the §8.9 family, and all three are corrections narrating the entry as
  *formerly* unread — the known false positive, read down before it was believed.
- **Entries (program)**: 236 rows in the population, 113 entries over 45 rows — 31 named nowhere,
  82 only elsewhere, 36 not named by the row's own note. The known populations (the excluded sound,
  movie and launch actions, the conversion's shifted columns in Tables 165 and 200); nothing worked.
  §12.5.6.19's Table 191 `/AA` is the one hit whose row says nothing about it, and §12.6.3's row
  is where it is read.
- **Quotations (program)**: 3223 quotations in 502 documents, 1495 verbatim, 23 diverging, **0
  defects** — every divergence an `…` elision, an RFC's own wording, or the conversion's shape.
- **Retired claim (sweep 4, program)**, over the eight rounds' nouns — `render_retained`,
  `RetainedScene`, `encode_source`, `Query::Highlight`, `Answer::Highlighted`, `ImportData`,
  `short_of_its_grid`, `intersected`, `Clip::Value`, `stated_width`, `hollow`, `NamingGap`,
  `blockers`, `capabilities`: **164 mentions, 4 nouns carrying both shapes, 0 defects.** Every
  correction is a round's own record of what it retired and every standing mention is a true
  sentence about the mechanism that replaced it — which is what this sweep says when the rounds
  that made the corrections made them everywhere.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (7)**: 61 of 80 rows name source vocabulary under a session-local stop-list;
  none wrong, and **no row in the population changed since the five-hundred-and-seventeenth's
  read** — checked with `git diff` over the ledger's status lines, which is the cheap way to carry
  a clean read forward.
- **Citations (8)**: 6 mentions of 3 dead paths, **0 defects** — `doc/todo/20` twice, `doc/todo/37`
  three times and `spec-errata`'s `doc/errata.md` redirection target, all of them the known
  correction-quoting-its-own-pointer shape, `viewer-ffi/src/form.rs` included since the
  five-hundred-and-seventeenth turned its plain citation into one.
- **Parent counts (10)**: 160 counted claims raw; the 35 that are checkable arithmetic — every
  "aggregate of the N below" against the family under it — are all correct, and the rest are counts
  about the *clause* rather than about the children. **0 defects here, and the one this round found
  was invisible to it**: §12.6.3's count is about §12.6.4's family and not about its own.
- **Ledger quotation marks (11)**: 1248 spans of 4+ words under a session-local normaliser, 866
  verbatim in `doc/md/`, 47 matching five words and diverging, **0 defects** — every one an `…`
  elision, an editorial bracket, §8.4.4's correction quoting its retired wording, or the
  conversion's own escaping.
- **Sweep 14**: 10 hits under a session-local debt vocabulary, every one naming its debt in words
  outside it ("writer-side", "may be ignored", "aggregate of the two below"). **§E was in both this
  list and the blame band** and is corrected below.
- **The errata (12)**: "151 struck passage(s) of 4 words or more that doc/md/ still carries as
  current text" over all fourteen PDFs, unchanged, so the population is still read to the end;
  every landing is a known in-place annotation, `doc/todo/01`'s own records among them.

### Fifteen rows read off the blame list, the band from commit 184 to 500

Oldest first: §12.6.4.4 — the one row the five-hundred-and-seventeenth left under the fold — then
§8.9.5.1, §7.9, §12.6.3, §12.5.6.7, §8.11.4.4, §8.11.4.5, §12.5.6.21, §14.11.6.2, §14.12.4,
§12.4.3, §E, §E.1, §I and §I.2. **Two were wrong, one of them took three rows with it, and four
statuses moved.**

| row | shape | was | is |
|---|---|---|---|
| **§I.2** | 6 | `partial` for a writer-side sentence, above a note saying that sentence "§7.5.6's incremental update meets by leaving the header where it found it" | the note corrected and the status not — the sixth failure shape exactly. The clause's other two writer sentences are a `may` this program declines and a verb it does not have, and its one `shall` on a processor ("shall attempt to read any PDF file, even if the PDF file's version is more recent") is met by construction, because nothing on the open path consults the version — **`implemented`** |
| **§I**, **§E.1**, **§E** | 5 | §I `partial` "for I.2's writer-side sentence"; §E.1 `partial` because its one reader-side sentence "points at Annex I … and is `partial` for the version half"; §E the aggregate of the two below | a row `partial` for a *neighbour's status* falls the moment the neighbour moves, and three did — all three **`implemented`**, in the same commit as the row they waited on |
| **§12.6.3** | 10, across families | "every action a trigger performs is one of the **eleven** §12.6.4 types this program performs" | a count agreeing with neither neighbour: §12.6.4's row says eight of the seventeen it documents, §12.6's ten of Table 201's twenty, and the eleventh was §12.6.4.8's `/URI`, read entire and printed rather than opened |

**The ten kept rows each record the evidence that kept them** — the `Action::Refused` carrying
`GoToE`'s own name, the one reader of `"IT"` being §12.5.6.6's callout intent and of `"Measure"`
§12.10.2's viewport, `Recommendation::Unanswerable` still answering Table 100's `User` and
`Language`, `TrapNet` still in `STANDARD_SUBTYPES` with no corpus document stating one, nothing in
the tree naming `DPartRoot`, and `beads_on_page` still on the new program's rung for a function
only its own crate names — which is what moves the blame pointer without a stamp.

**And the three status moves are one shape worth naming**: §E.1 and §I were `partial` because a row
they pointed at was, and §I was `partial` because *its own child's note* said the debt was paid.
Nothing in this file's fifteen sweeps looks for a row whose reason is a neighbour's **status**; the
sixth's arithmetic compares statuses but only where every child is settled, and §I.2 was the one
that was not. What found it was the blame band.

## All fifteen run again in the five-hundred-and-thirty-seventh, the eighth committed as a program, and the pointer a crate wrote to a test it never had

Four rounds since the last full sweep — the grid a shading is divided across, the window a content
stream is read through, two storage structures and the byte a scan was taking, a design round, the
two operators a deferred clause decides, the operands that stopped allocating — and several of them
corrected ledger notes while one *deleted* a claim about `doc/todo/47`, which is exactly the
condition these exist for. Over `ledger.toml`, `crates/`, `tools/`, `fuzz/` and every Markdown
document under `doc/` this project wrote bar `doc/history/`:

- **Pointers (8), as a program** (`cargo run --release -p conformance --bin pointers`, ADR 0372).
  First run: **4609 path pointers — 2525 live, 104 absent, 14 in another crate, 1616 unrooted, 106
  a form, 244 not carried — and 48 symbol pointers, 9 undefined**, in a third of a second. 84 of
  the 104 absent are in `doc/adr/` and in this file's own records of earlier runs, which is the
  dominant shape over the ADRs and is not a defect: an ADR saying the remaining question is
  "recorded in `doc/todo/47`" is true about the day it was written, and the file being closed since
  does not falsify it. **Three defects.** `crates/viewer-host/src/policy.rs` explained
  `resolve_import`'s purity by "which is what `tests/import_policy.rs` does" and that file has
  never existed in any commit — the policy is tested in `tests/host_mappings.rs`, one file over;
  `crates/viewer-accessibility/tests/tree.rs` sent the bus half of its question to a
  `tests/atspi.rs` that was never written; and `doc/errata-read.md` quoted
  `content.rs::alternate_image` for a function that moved to `content/image.rs` when the module was
  split, which only the *symbol* half could find. **The two path defects were found by the
  resolve-it-from-where-it-is-written rule and by nothing else**, both being fragments naming a
  file of their own crate.
- **Table numbers (sweep 9)**: 1074 *attributed* key citations checked against 408 tables — a key
  is only a claim about a table where the sentence attributes it, and reading every key in every
  sentence that names a table was 545 hits to nothing — **7 defects, six of them in `pdf-model`
  and five in one `enum`'s doc comments**: `/Length1` under Table 126 instead of 125, an ICC
  stream's `/N` under 66 instead of 65, `/ShadingType` under 78 instead of the common 77, a Type 0
  font's `/Encoding` under 122 instead of 119 (all `examples/damaged_stream_census.rs`), a widget's
  normal appearance `/N` under Table 168 instead of 170 (`view.rs`, where the
  five-hundred-and-twenty-fifth's single finding also was), a parent tree's `/Nums` under 354
  instead of 37, and `/Enforce` under 148 instead of 147. **A block written in one sitting is this
  sweep's oldest shape.** The known noise: a *flags* table has bit numbers rather than keys, and
  `doc/md/` renders Table 92's abbreviations in the second column, so its keys read as absent.
- **Parent counts (10)**: 70 checkable count claims against a family, 20 matching neither the
  direct children nor the descendants, and **1 defect** — §14.8.2 said "[o]f the twelve rows below"
  over thirteen, all thirteen present since the ledger was generated, so the count was wrong when
  it was written **by the five-hundred-and-first**, a sweep round.
- **Blockers (program)**: ledger 21 sentences — 6 expired, 10 holding, 5 naming no clause; source
  27 — 10, 10, 7. 0 defects; every printed-expired hit a correction quoting the wording it retired,
  a past tense, a `while §X` that is a contrast rather than a wait, or §10.7.4's documented choice
  against §8.4.3.2.
- **Capabilities (program)**: ledger 49 sentences — 34 witnessed by the tree, 42 about the program,
  7 about one crate; source 145 — 119, 79, 66. 0 defects.
- **Unread (program)**: 63 rows claim, 174 keys; **49 confirmed, 125 quoted over 53 rows, 55 by the
  row's own code**. 0 defects — the two keys that moved since the five-hundred-and-twenty-fifth are
  the ledger moving, not the tree.
- **Entries (program)**: 240 rows explain themselves by an arrival and name code, 1 names none;
  640 table entries stated, 112 reported over 45 rows — 29 named nowhere, 83 only elsewhere, 34 not
  named by the row's own note. The known populations; nothing worked.
- **Quotations (program)**: 3386 quotations in 533 documents, 1586 verbatim, 23 diverging, **0
  defects**.
- **Callers (program)**: **296 distinct `pub fn` names in `pdf-model`** (289 in the
  five-hundred-and-twenty-fifth), 15 crates naming it in a manifest; 176 named by a dependent crate,
  20 by a tool or a fuzz target, 77 only inside `pdf-model`, 21 only by a test or an example, **2 by
  nothing at all** — 120 that no crate under `crates/` asks. **The delta is the finding and it is
  clean**: the two named by nothing are the same two, both disposed of in their own doc comments by
  the round that found them, and the seven new names all have an in-crate caller.
- **Retired claim (4, program)**, over the four rounds' nouns — `StreamSource`, `ObjectsLost`,
  `with_stated_length`, `Window`, `widen`, `TokenTooLong`, `Unbuffered`, `PARALLEL_CELLS`,
  `rows_in_parallel`, `round_to_greater`, `atlas_repacked`, `Agreement::Bounded`, `points_from`,
  `operands_before`: **1512 mentions, 2 nouns carrying both shapes, 0 defects** — and 1462 of the
  1512 are `Window` and `widen`, which is this file's own warning about an ordinary English word
  arriving twice in one wave. `StreamSource` and `Agreement::Bounded` have **no** mentions at all,
  which is what a round that swept its own work looks like.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (7)**: 57 of 80 rows name source vocabulary under a session-local stop-list;
  none wrong, and **exactly one row in the ledger changed status since the
  five-hundred-and-twenty-fifth** — this round's own — checked with `git diff` over the status
  lines, which is the cheap way to carry a clean read forward.
- **Ledger quotation marks (11)**: 1270 spans of 4+ words under a session-local normaliser, 913
  verbatim in a specification, 78 matching five words and diverging of which 22 hold no elision,
  **0 defects** — every one a `…` or `...`, an editorial bracket, a correction quoting its retired
  wording, or the conversion's own shape (`text-tospeech`, a table title expanded, two bullets
  quoted as one sentence).
- **Sweep 14**: 8 hits under a session-local debt vocabulary, every one naming its debt in words
  outside it ("writer-side", "may be ignored", "defers each to a subclause").
- **The errata (12)**: "151 struck passage(s) of 4 words or more that `doc/md/` still carries as
  current text" over all fourteen PDFs, unchanged, and 75 quotations quoting text struck out of the
  clause they cite — every landing a known in-place annotation, this file's own records among them.

### Twelve rows read off the blame list, the band from commit 511

Oldest first: §12.7.4 (511), §9.2.4 (512), §12.7.6.2, §12.8.2 (513), §8.7.3 (514), §11.7.5, §12.11,
§7.7.2 (515), §14.8.4, §14.9 (516), §12.8.2.2, §12.8.2.4 (517). **Two were wrong and they are one
mechanism**, which is this file's commonest shape.

| row | shape | was | is |
|---|---|---|---|
| **§12.8.2.4** | 4 | `partial`, and "`FieldMDP` … is recognised where a `/Reference` states it" | **nothing in this tree names the string.** `has_transform` takes the method name and `DocMDP` is its only caller, so the half of the note saying what *is* done was the wrong half. **`reported`** — none of the clause is executed, and what a person is told is `notes.rs`'s sentence on every signature; the field-freezing the clause exists for arrives through §12.7.5.5's lock, which Table 259 says a writer copies into these parameters |
| **§12.8.2** | 5 | the parent, saying `FieldMDP` and `UR` were both recognised | `UR` is (`usage_rights` matches `/UR` and `/UR3`); the other half was a claim about a sibling nobody had grepped |

**The ten kept rows each record the evidence that kept them** — §12.7.4's two `partial` children
beside its `implemented` §12.7.4.2, `Action::Refused` still carrying `SubmitForm`'s own
sentence, the twenty-five `match` arms §12.11 counts, the four Table 29 entries §7.7.2 calls
genuinely unread and no source names, the forty-one `StandardType` variants §14.8.4 counts, and
§9.2.4's reading of what Tables 110 and 111 do and do not require of a glyph box.

## All fifteen run again in the five-hundred-and-forty-fifth, the ninth committed as a program, and the parser had been reading the standard's longest tables short

Eleven rounds since the last full sweep — typed type-4 values (536), the region census refused
(538), the raster a page decoded thirty-six times (539), the quotation mark a conversion changed
(540), the colour a device computes (541), the geometry phase divided (542), the frame that says it
is stale (543), the trailer eight megabytes from the end (544) — with `doc/todo/46` and `doc/todo/47`
deleted along the way, which is the condition the eighth sweep exists for. Over `ledger.toml`,
`crates/`, `tools/`, `fuzz/` and every Markdown document under `doc/` this project wrote bar
`doc/history/`:

- **Table numbers (9), as a program** (`cargo run --release -p conformance --bin tables`, ADR 0380).
  First run: **409 tables captioned, 305 stating entries; 4680 sentences name a table; 1830
  attributed key citations — 1706 the table agrees with, 75 absent, 3 a denial the table
  contradicts, 46 under a table that states no entries, 0 under no such table.** What the program
  adds over the grep is the two judgements every hand-run redid — *which* table does state the key,
  printed under each suspect, and whether the sentence is a correction or a standing claim — plus
  one the grep could not make at all: **a denial is a claim in the other direction**, so "Table 119
  gives a Type 0 dictionary no `/FontDescriptor`" is agreement rather than a hit, and a denial the
  table *contradicts* is the same defect from the far end. **Eleven defects, two in the source and
  nine in this project's documents**, and the documents are the finding: six of the nine are a
  round's own correction that stopped at the code (537's two numbers left in ADR 0366, 489's `/H`
  left in ADR 0123 twice, 216's `/Ascent` block left in five documents), and **ADR 0245 named ADR
  0244 as carrying "Table 189's `/R`", corrected two of the three places and left the ADR it had
  just named**.
  - **And the instrument had been reading the standard short in two ways**, both fixed in
    `entries::tables_in`, which the fifteenth sweep shares. The conversion breaks a long table into
    a run of pipe tables under one caption and the parser filed the **first block only** — Table 31
    stated six of twenty-eight entries, Table 125 none of its three lengths — which is what the
    earlier hand-runs recorded as "the parser missing a split-header table's keys" and could not
    remove. And `key_of` stripped a trailing `1`, `2` or `3` as a footnote marker, so `/BG2`,
    `/Length1`, `/FontFile2`, `/DW2`, `/UR3` and `/BlackIs1` were keys of nothing; **no table in the
    standard carries a numeric footnote in its first column**, checked over all 305. The first run
    printed 382 suspects and the two fixes took it to 84 before a rule of its own was added.
- **Entries (15, program)**: **756 table entries stated over 244 rows, 174 reported over 46 rows —
  41 named nowhere, 133 only elsewhere, 39 not named by the row's own note.** Up from 640 and 112,
  and **the whole of that delta is the parser fix above**: this sweep had been reading 116 of the
  standard's entries as though they did not exist. The reading list is the known populations — the
  excluded sound, movie and rendition actions, §12.11.1's `/RH`, §14.9.6's declined pronunciation
  permission — and nothing worked.
- **Pointers (8, program)**: 4799 path pointers — 2653 live, 92 absent, 14 in another crate, 1668
  unrooted, 118 a form, 254 not carried; 52 symbol pointers, 12 undefined. **0 defects.** The three
  new undefined symbols are one renamed test
  (`quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over`, replaced by
  `quorra_states_what_it_will_not_stage` in the four-hundred-and-fifty-sixth) and the document that
  names it **records the replacement forty lines further down**, which is this sweep's oldest false
  positive at document scale.
- **Blockers (program)**: ledger 21 sentences — 6 expired, 10 holding, 5 naming no clause; source 27
  — 10, 10, 7. **Identical to the five-hundred-and-thirty-seventh's ten numbers**, which is the
  cheapest evidence there is that a population has not drifted. 0 defects.
- **Capabilities (program)**: ledger 51 sentences — 36 witnessed by the tree, 44 about the program,
  7 about one crate; source 150 — 123, 83, 67. 0 defects; every witness the known loose-noun shape.
- **Unread (program)**: 62 rows claim, 173 keys; 49 confirmed, 124 quoted over 52 rows, 54 by the
  row's own code. 0 defects.
- **Quotations (program)**: 3445 quotations in 553 documents, 1611 verbatim, 24 diverging; 1393 in
  794 ledger notes, 1089 verbatim, **1 diverging**. 0 defects.
- **Callers (program)**: 122 names no crate under `crates/` asks, 177 named by a dependent crate, 80
  only inside `pdf-model`, 21 only by a test or an example. The delta is clean.
- **Retired (4, program)**, over the wave's twelve nouns — `RasterCache`, `RASTER_BUDGET`,
  `device_program`, `ShadingProgram`, `FunctionPaints`, `encode_threads`, `Stale`, `MustFollow`,
  `capture_presented`, `approximated`, `find_startxref`, `quoted_spans`: **299 mentions, 2 carrying
  both shapes, 0 defects** — and 196 of the 299 are `Stale`, which is this file's own warning about
  an ordinary English word arriving as a type name.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean.
- **`inapplicable` (7)**: 47 of 80 rows name source vocabulary under a session-local stop-list; none
  wrong, and **exactly one row in the ledger changed status since the five-hundred-and-thirty-
  seventh** — §8.9.5.4, `partial` to `implemented` — checked with `git diff` over the status lines,
  which is the cheap way to carry a clean read forward.
- **Parent counts (10)**: 25 counted claims against a family, 10 matching neither the direct children
  nor the descendants, **0 defects** — every one a count about the rows *beside* it (§8.11.1's three
  subclauses, §11.7.4.1's four) rather than about its own children, which is this ledger's
  convention and this sweep's dominant shape. §14.12's "two rows below it that are `writer-side`" is
  §14.12.2 and §14.12.3 exactly.
- **Sweep 14**: 9 hits under a session-local debt vocabulary, every one naming its debt in words
  outside it ("Aggregate of", "writer-side", "conditional on importing the target page").
- **The errata (12)**: "151 struck passage(s) of 4 words or more that `doc/md/` still carries as
  current text" over all fourteen PDFs, unchanged, and 71 quotations quoting text struck out of the
  clause they cite — every landing a known in-place annotation.

### Eleven rows read off the blame list, the band from commit 518

Oldest first: §9.9.1 (518), §10.4.2.3 (520), §10.4 (524), §11.3.6 (525), §12.5.6.17, §12.5.6.18,
§12.5.6.20, §12.5.6.22 (528), §7.6.5, §12.8.1, §12.8.2.2.2 (534). **One was wrong, and it is the
fifth failure shape beside a row that had already been corrected for it.**

| row | shape | was | is |
|---|---|---|---|
| **§10.4.2.3** | 5, 7 | `partial` for grey-to-CMYK, "which has no caller: nothing here converts *to* CMYK" | `ColourSpace::to_cmyk` has converted a grey to ink since the four-hundred-and-twenty-sixth, and **§10.4.2.4's own row says so in as many words** — two rows about one mechanism, one corrected and the other left standing. The conclusion holds and the reason had expired: what has no caller is *this clause's* arithmetic, because a grey goes to RGB by §10.4.2.2 and then through ADR 0263's right inverse of the ink cube rather than through black generation and undercolour removal |

**The ten kept rows each record the evidence that kept them** — the three `/Length` entries named in
two comments of `program.rs` and by no reader, `CMYK_CORNERS` still being what a `DeviceCMYK` colour
reaches a pixel through, the three evaluators of §11.3.6 still being three, every reader of `"MK"`
being on a field path, no source naming `/FixedPrint` or `/MN` at all, `EnvelopedData` appearing
nowhere, `revocation_lists` counting `/CRLs` and asking nothing of them, and nothing anywhere
reconstructing a signed revision — which is what moves the blame pointer without a stamp.

## All fifteen run again in the five-hundred-and-fifty-third, the eleventh committed as a program, and both defects were in ADRs a later round had already disproved

Eight rounds since the last full sweep, and **every one of them corrected an earlier ADR in place**
— 0378's rules 4 and 5 (549), 0382 §6 (through 0383), 0384 §6 (through 0385) — which is the
condition this file's fourth sweep exists for and the hazard this round was pointed at. Over
`ledger.toml`, `crates/`, `tools/`, `fuzz/` and every Markdown document under `doc/` this project
wrote bar `doc/history/`:

- **`inapplicable` (7), as a program** (`cargo run --release -p conformance --bin inapplicable`,
  ADR 0388). First run: **80 `inapplicable` rows stating 305 terms — 60 named by no source, 245
  named over 72 rows, 231 of them carrying a cousin row that is not `inapplicable` and says the
  same word.** What the program adds over the grep is the two judgements each hand-run redid: the
  **count of naming files** in place of a stop-list, so the level is a property of the ledger and
  the tree rather than of the session — the nine hand-runs printed 25, 64, 72, 71, 27, 66, 43, 61,
  57 and 47 of about 80 with the population barely moving — and the **cousin**, which is where all
  five of this sweep's defects have been. The extraction takes two derived rules where the
  hand-runs took a word list: an inner capital makes an identifier wherever it stands, and a plain
  `Capitalised` word counts only where it does not open a sentence. That alone took the first run
  from 440 stated terms to 305. **One defect**: §14.5, below. And one by-catch about the file
  rather than about a claim — §Q's note read "answering one question — does this page contain
  transparency — and The annex states", a sentence spliced by an append, which is what put a
  capital `The` in the middle of it and what the extraction rule printed.
- **Retired claim (4, program)**, over the wave's nouns — `Stale`, `Base`, `composed`,
  `Plan::Reproject`, `Cadence`, `approximated`, `device_pixels`, `Path::walked`, `last_phases`,
  `bytes_uploaded`, `readback`, `detach_presenter`, `Presenter`, `RasterCache`: **2036 mentions, 10
  carrying both shapes, 0 defects**, and 1445 of the 2036 are `Base` and `readback`, which is this
  file's own warning about an ordinary English word arriving as a type name for the third wave
  running. **Run again over the nouns of the corrections themselves** — `SHARE`, `ASSUMED`,
  `TooDear`, `present_texture`, `capture_presented`, `atlas_repacked`, `Settled`: 1264 mentions, 5
  carrying both shapes, **2 defects, and both are the shape the round was pointed at**. See below.
- **Table numbers (9, program)**: 409 tables captioned, 305 stating entries; 4698 sentences name a
  table; **1849 attributed key citations — 1710 the table agrees with, 89 absent, 4 a denial the
  table contradicts, 46 under a table that states no entries, 0 under no such table. 0 defects.**
  The absent are up from 75 because the five-hundred-and-forty-fifth wrote its own record of every
  number it retired, which is this sweep's own level moving under the round that corrected it.
- **Pointers (8, program)**: 5013 path pointers — 2791 live, 92→97 absent, 14 in another crate,
  1738 unrooted, 118 a form, 255 not carried; 52 symbol pointers, 12 undefined. **0 defects**;
  every absent path is an ADR naming a `doc/todo/NN` its own round deleted, another project's
  `crates/`, or a correction quoting the pointer it retired.
- **Blockers (program)**: ledger 22 sentences — 6 expired, 10 holding, 6 naming no clause; source
  27 — 10, 10, 7. **0 defects**, and the one sentence more than the five-hundred-and-forty-fifth's
  21 is that round's own "[r]ead and kept" line on §12.5.6.22.
- **Capabilities (program)**: ledger 52 sentences — 37 witnessed by the tree, 45 about the program,
  7 about one crate; source 151 — 124, 84, 67. 0 defects.
- **Unread (program)**: 62 rows claim, 173 keys; 49 confirmed, 124 quoted over 52 rows, 54 by the
  row's own code — **identical to the five-hundred-and-forty-fifth's five numbers**, which is the
  cheapest evidence there is that a population has not drifted. 0 defects.
- **Entries (program)**: 244 rows explain themselves by an arrival and name code, 1 names none; 756
  table entries stated, 174 reported over 46 rows — 41 named nowhere, 133 only elsewhere, 39 not
  named by the row's own note. The known populations; nothing worked.
- **Quotations (program)**: 3574 quotations in 571 documents, 1613 verbatim, 24 diverging; 1394 in
  794 ledger notes, 1089 verbatim, **1 diverging** — §8.4.4's correction quoting the sentence ISO
  32000-2 does not contain, which is the known shape. 0 defects.
- **Callers (program)**: 299 distinct `pub fn` names in `pdf-model` (296 in the
  five-hundred-and-thirty-seventh), 122 that no crate under `crates/` asks, 177 named by a
  dependent crate, 80 only inside `pdf-model`, 21 only by a test or an example, **1 by nothing at
  all** — down from two, and the delta is the finding: `Namespace::is_standard` is off the list.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean, and the only sweep of
  the fifteen that has never printed anything else.
- **Parent counts (10)**: 41 counted claims against a family, 6 matching neither the direct
  children nor the descendants, **0 defects** — every one a count about the *standard* or about the
  rows beside the clause ("thirty-three rows below" caught as "three", "two subclauses of the
  standard disagreeing"), which is this sweep's dominant shape.
- **Sweep 14**: 19 hits under a session-local debt vocabulary — still session-local, which is the
  next candidate for a program — every one naming its debt in words outside it ("is not read",
  "reach nothing further", "applied to nothing", "the projection is not").
- **The errata (12)**: "151 struck passage(s) of 4 words or more that `doc/md/` still carries as
  current text" over all fourteen PDFs, unchanged, and **71 quotations quoting text struck out of
  the clause they cite — the same 71 as the five-hundred-and-forty-fifth**. The population is still
  read to the end.

### The two defects, and both are a correction that stopped at the document it was written in

`doc/adr/` joined the fourth sweep's targets in the four-hundred-and-twenty-ninth for exactly this:
"a claim a *later* round disproves and leaves standing is a defect wherever it lives, and the round
that disproves it amends the ADR that made it, in the same commit". Eight rounds ran; two of them
did not.

| document | said | disproved by | the correction lived only in |
|---|---|---|---|
| **ADR 0382 §6** | "The escape hatch is complete and needs nothing from upstream" | ADR 0383 — presenting a texture needs the surface, `Device` keeps it private, and a host configuring its own needs a format only a `&wgpu::Adapter` gives | ADR 0383, for five rounds |
| **ADR 0384 §6** | "It cannot be worked around from here", of the reprojection an atlas repack costs | ADR 0385 — true about *capturing*, a non-sequitur about *drawing*: the base was thrown away by `Stale::settled`, not by the repack | ADR 0385, for three rounds |

**The first is the sharper**, because the sentence is a decision's own conclusion rather than an
aside: a reader of 0382 who never opens 0383 comes away believing this tree has a working escape
hatch. Both are struck and re-argued in place, naming the ADR that disproved them. ADR 0388.

### The seventh sweep's first run as a program: §14.5, and it is the retired exclusion again

§14.5's page-piece dictionaries were `inapplicable` because "[n]othing here writes a PDF, so there
is no private data to recognise" — `CLAUDE.md`'s pre-amendment wording, false since the
hundred-and-thirty-fifth session and corrected in the ledger's own *generated* header by the
five-hundred-and-tenth (ADR 0345). **The sweep found it from the tree's side**: `tests/saving.rs`
names `/PieceInfo`, under a row saying nothing here writes.

Right conclusion, expired argument, which is this file's fifth shape. The reason that holds is
narrower and does not mention what this program does: a data dictionary holds "private data needed
by the PDF processor" *that produced it*, keyed by that processor's own name, and this one has
produced none. Nothing is owed by the writing this program does do — Table 351's required
`/LastModified` binds a data dictionary a processor writes, and the clause's one reader-side `shall`
("modification dates shall be compared only for equality and not for sequential ordering") binds a
processor comparing them against private data of its own.

### Eleven rows read off the blame list, the band from commit 534 to 536

Oldest first: §12.8.3.4.3, §12.8.4, §12.8.4.4, §12.8.5.2 (534), §12.4, §12.6.4.15 (535), §12.1,
§12.3.5, §12.3.5.1, §12.6, §12.6.4 (536). **Two were wrong, they are one count, and the ADR they
were both written from says both answers.**

| row | shape | was | is |
|---|---|---|---|
| **§12.4** | 4, 7 | "seven of Table 164's twelve transition styles are drawn … with the other five reported by name" | **four** are reported — `Blinds`, `Glitter`, `Dissolve`, `Fly` — and the fifth was `R`, which Table 164 defines as the cut, "[t]he new page simply replaces the old one with no special transition effect". `transition::note`'s own doc comment says "`R` is the one style with nothing to report" |
| **§12.6.4.15** | 4, 7 | "the five of Table 164's twelve styles no frame is shaped for, each reported by name" | the same error in the same words, one clause away |

**And the source of both is ADR 0230, which holds both answers twelve lines apart**: "[t]he other
five are named and **reported by name**", and then, under its own table, "`R` is therefore not
reported and the other four are." The two rows took the first half. That is the sixth failure shape
— a document corrected by appending, read from the correction backwards — with the ledger inheriting
it. The ADR is amended too, per ADR 0265's rule.

**The nine kept rows each record the evidence that kept them** — `pades_departures` checking exactly
the attributes §12.8.3.4.3 names and nothing where the `/SubFilter` is not `ETSI.CAdES.detached`,
`SecurityStore` still counting rather than reading, the VRI count coming off the DSS's own `/VRI`,
`timestamp_imprint` still what the byte range is checked against, §12.1's counts checked against
§12.6's ten and §12.6.4's eight rather than believed, `Collection::initial_document`,
`Sort::ascending` and `FieldKind::is_in_the_item` all three still in `collection.rs`, and each of
§12.6.4's nine refusals still reaching `Action::Refused` under its own name — which is what moves
the blame pointer without a stamp.

## All fifteen run again in the five-hundred-and-sixty-second, the twelfth committed as a program, and the errata a `check` cannot see

Nine rounds since the last full sweep. Over `ledger.toml`, `crates/`, `tools/`, `fuzz/` and every
Markdown document under `doc/` this project wrote bar `doc/history/`:

- **Sweep 14, as a program** (`cargo run --release -p conformance --bin owed`, ADR 0397). The last
  of the four descriptions whose *level* moved with the session — nine hand-runs printed 16, 24, 5,
  15, 19, 10, 8, 9 and 19 hits over rounds that moved almost no rows, each under a debt vocabulary
  written that morning. **The discriminator is the seventh sweep's with the sign reversed**: there a
  term the tree *names* under an `inapplicable` row claiming absence, here a term the tree *lacks*
  under a `partial` row claiming a debt. A note that names a debt names a **thing**, and a thing
  this tree does not have is a name no source carries, which is a count rather than a guess.
  First run: **225 `partial` rows stating 2983 terms — 159 named by no source, over 105 rows,
  leaving 120 rows whose every stated term this tree already names.** The rows are ordered by their
  *rarest* term's reach, commonest first, so a row that names nothing specific heads the list.
  **One defect, and it is at the top of that order**: §12.7.6.1, `partial` above a note naming
  nothing owed, over a clause that is three bullets and no `shall` at all — `implemented`, with the
  three form action types' three answers asserted rather than described.
  - **A learned debt vocabulary was tried first and does not work**, which is worth recording so
    that nobody builds it again. Scoring each word by how much likelier it is in an owing note than
    in a settled one measures **topic** rather than debt: the top terms come out `digest`, `stored`,
    `appearance`, `knockout`, `signature` — the subjects of the clauses that happen to be `partial`
    — and every one of the 228 notes then holds some word from any lexicon wide enough to be useful,
    so the hit count is zero at every threshold. Requiring a debt word to span half the clause
    families removes the topic and leaves function words. The ledger cannot teach a vocabulary
    because status and subject are not independent in it.
- **The errata, from the other end** (`cargo run --release -p spec-errata -- emit doc/*.pdf`), which
  is this round's second hazard and is now a line in `doc/todo/02` §4. `check` compares *quotations
  this tree has written*, so an erratum over text nobody has quoted is invisible to it — which is how
  ISO/TS 32001 §5.1.3's deletion went unseen until the five-hundred-and-fifty-fifth went looking.
  `emit` prints **1097 annotations over the three documents that carry any**; twenty are structural,
  and **two of the three renumbering errata were unrecorded**: Issue #452 moves §14.7.5.1.1 up a
  level and renumbers the rest of §14.7.5, under five ledger rows and some twenty source citations,
  and Issue #196 inserts a new §7.6.5.3 and pushes the existing one down. The third is #133, which
  ADR 0273 read in the four-hundred-and-thirty-seventh — an instrument agreeing with a known finding
  is what says it works. **And one erratum changes a requirement rather than a number**: Issue #22
  raises Table 166's `/AP` to "Required except for conditions listed below (PDF 2.0)", and the 2020
  text already bound a writer in prose, against which `view.rs` said in two places that "an
  annotation with no `/AP` is legal". `doc/errata-read.md` carries the table and the argument.
- **Blockers (program)**: ledger 22 sentences — 6 expired, 10 holding, 6 naming no clause; source 28
  — 10, 10, 8. 0 defects; every printed-expired hit a correction quoting the wording it retired or a
  past tense.
- **Capabilities (program)**: ledger 53 sentences — 38 witnessed by the tree, 46 about the program,
  7 about one crate; source 153 — 126, 87, 66. 0 defects; every source hit a true statement about a
  boundary a crate keeps.
- **Unread (program)**: 64 rows claim, 175 keys; 48 confirmed, 127 quoted over 54 rows, 55 by the
  row's own code. 0 defects — the known one-short-key population.
- **Entries (program)**: 249 rows explain themselves by an arrival and name code, 2 name none; 788
  table entries stated, 193 reported over 47 rows — 56 named nowhere, 137 only elsewhere, 55 not
  named by the row's own note. The known populations; nothing worked.
- **Quotations (program)**: 3755 quotations in 590 documents, 1662 verbatim, 25 diverging; 1428 in
  794 ledger notes, 1112 verbatim, **1 diverging** — §8.4.4's correction quoting the sentence ISO
  32000-2 does not contain, which is the known shape. 0 defects.
- **Callers (program)**: 122 names no crate under `crates/` asks, 177 named by a dependent crate.
  The delta is clean.
- **Pointers (program)**: 5276 path pointers — 2920 live, 100 absent, 14 in another crate, 1845
  unrooted, 123 a form, 274 not carried; 54 symbol pointers, 12 undefined. 0 defects; every absent
  path is an ADR naming a `doc/todo/NN` its own round deleted or a correction quoting the pointer it
  retired.
- **Table numbers (program)**: 409 tables captioned, 305 stating entries; 4778 sentences name a
  table; **1879 attributed key citations — 1735 the table agrees with, 91 absent, 4 a denial the
  table contradicts, 49 under a table that states no entries, 0 under no such table. 0 defects.**
  The absent are up from 89 because this round wrote its own record of what it corrected, which is
  this sweep's own level moving under the round that reads it. One suspect was checked to the end
  rather than assumed: ADR 0301's "Table 377's `/Placement`" is right, and reads as absent only
  because Table 377's first column is a *structure element* rather than a `Key`.
- **`inapplicable` (program)**: 80 rows stating 310 terms — 60 named by no source, 250 named over 72
  rows, 235 carrying a cousin. None wrong.
- **Retired claim (program)**, over the wave's fourteen nouns — `Presenter`, `detach_presenter`,
  `Layer`, `PresentCost`, `QuorraWindowRenderer`, `render_retained`, `Digest::ALL`,
  `TRIED_WHEN_UNSTATED`, `Shake256`, `substituted_media_box`, `ccitt_rows`, `Tree::child`,
  `supports_text_ranges`, `select::continues`: **913 mentions, 8 carrying both shapes, 0 defects**
  — and 768 of the 913 are `Layer` and `Presenter`, which is this file's own warning about an
  ordinary English word arriving as a type name for the fourth wave running.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean, and still the only sweep
  of the fifteen that has never printed anything else.
- **Parent counts (10)**: 4 counted claims against a family under a session-local pattern, 0
  matching neither the children nor the descendants. **This is the last sweep whose level is still
  session-local**, and it is the next one a program should take over.
- **The errata (12)**, `check`: "151 struck passage(s) of 4 words or more that `doc/md/` still
  carries as current text" over all fourteen PDFs, unchanged, and **71 quotations quoting text
  struck out of the clause they cite — the same 71 as the five-hundred-and-forty-fifth and the
  -fifty-third**. The population is still read to the end, which is what makes `emit`'s findings
  above legible as a *different* question rather than as a gap in this one.

### Ten rows read off the blame list, the band from commit 541 to 546

Oldest first: §11.3.7.2, §11.6.4, §11.6.4.1, §11.6.4.4 (541), §12.7.2 (542), §11.4, §11.4.3,
§11.4.8 (544), §12.3, §14.11 (546). **Three were wrong, two of them statuses, and one is the fifth
failure shape found twice on one row.**

| row | shape | was | is |
|---|---|---|---|
| **§11.6.4.1** | 525's | `partial` because "[w]hat this row owes is which of the two quantities the last two are, which is `/AIS` and is §11.6.4.3's" | `/AIS` is Table 57's, stated by §11.6.4.3's and §11.6.4.4's own text and by nothing in this clause. Its own prose states one requirement — the values "shall come from one or more of the following sources" — and all three sources are here. A row `partial` for a **neighbour's** debt, which is §E.1's and §I's shape one family over and which no sweep looks for — **`implemented`** |
| **§12.7.2** | 14th sweep's, found in the band | `partial` above a note naming nothing owed | every other sentence of the clause is definitional; the one `shall` addressed to a processor — an appearance "shall be consistent with the object's current value as a field" — is what `appearance::regenerates` does. **`implemented`**, with the merged single-widget dictionary given evidence rather than an assertion |
| **§14.11** | 5 | "§14.11.2 is `partial` for §14.11.2.2's guidelines", and §14.11.2's page boundaries listed among the `partial` ones | §14.11.2 and §14.11.2.1 moved to `implemented` in the four-hundred-and-forty-second, **a hundred and twenty sessions before**. The row's own last sentence is "[a] parent row is not maintained by the sessions that correct its children" — written by the four-hundred-and-second, about the half of the row that was then wrong, while the half beside it went the same way. §14.11.3 is what keeps it `partial`, alone |

**The seven kept rows each record the evidence that kept them** — §11.3.7.2's six sources against the
clause's own bullets (with one precision: `/AIS` is borrowed from §11.6.4.3 and §11.6.4.4 and kept
because this clause is where the shape/opacity distinction is drawn), §11.6.4's four children,
§11.6.4.4's own last sentence stating `/AIS` outright, §11.4's eight rows still saying what its note
says, §11.4.3's live deferral to §11.4.4, §11.4.8's having no requirement of its own, and §12.3's
five subclauses counted against the rows rather than believed.

**And one by-catch about the file rather than about a claim.** §7.2.3's note read "worth 5." and then,
four sentences later, "`doc/todo/53`.84% of the whole. ADR 0370." — a percentage split in half by an
append, exactly as §Q's sentence was spliced in the five-hundred-and-fifty-third. Repaired.

## All fifteen run again in the five-hundred-and-sixty-fifth, the thirteenth committed as a program, and the errata a hand-filter cannot see either

Three rounds since the last full sweep. Over `ledger.toml`, `crates/`, `tools/`, `fuzz/` and every
Markdown document under `doc/` this project wrote bar `doc/history/`:

- **Sweep 10, as a program** (`cargo run --release -p conformance --bin counts`, ADR 0400) — a parent
  row's stated count against its children, and the last of the ten whose printed *level* moved with the
  session: ten hand-runs gave 16, 185, 124, 10, 160, 17, 70, 25, 41 and 4 counted claims over a ledger
  whose families barely move, because each round wrote the pattern that morning. **The obvious
  discriminator here is a vocabulary of counting**, which is exactly what sweep 14's own lesson forbids,
  so both halves come from sweeps that already exist: **the ninth's attribution** decides what makes a
  cardinal a claim (it must govern one of the ledger's own words for a row — `row`, `subclause`, `child`,
  `below` — within three words and inside the sentence's own punctuation, and the family is the clause
  the sentence *names*, which is how a count of another family's rows becomes checkable at all), and
  **the sixth's arithmetic** answers it (every cardinality the family's own rows can produce, so the two
  conventions this ledger keeps are derived rather than remembered: a count that leaves out the
  `General` row, and a count with the clause's own row *in* the family, which is how this project writes
  "§11.7 — fourteen rows"). First run: **170 clauses have a row below them; 5184 sentences govern one of
  those words; 296 attributed counts — 125 the family agrees with, 45 it can be counted no such way, 126
  attributed to a clause with no rows below it; 3 places count one family twice.** 0 defects in the
  ledger, and the instrument's own evidence is that its four known findings — §11.7's double count,
  §14.8.2's twelve over thirteen, §12.6.4's and §12.7.6's — all come back as `[correction]` hits on the
  numbers their rounds retired. Three rules came off the first run rather than out of a design: `below`
  is an ellipsis and needs the number immediately in front of it ("its own table twelve lines below"
  counts lines), a cardinal's reach stops at a semicolon or a colon, and a number with a full stop or a
  leading zero in it is a clause number or an ADR's name rather than a quantity. **Those figures are
  the run before this round's own corrections**; run again after them it prints 5254 sentences and 304
  attributed counts — 129, 46, 129 and 4 — because the notes written above state counts of their own,
  which is the ninth sweep's and the twelfth's own level moving under the round that reads them, and is
  worth knowing before reading a moved number as movement.
- **The errata, from the other end again** (`cargo run --release -p spec-errata -- moved doc/*.pdf`).
  The five-hundred-and-sixty-second read `emit`'s 1097 annotations by hand and named the three words to
  filter for; **a filter written down is a filter somebody re-invents**, so it is a command now, and its
  first run found **two structural errata the hand-run had walked past**. Issue #477 moves all of
  §12.3.6 down a level — the same shape as #452, missed because this collection writes an instruction in
  the past passive ("was moved and demoted") as well as in the imperative — and Issue #256 says
  §12.6.4.8's `/Base` text "applies to all relative URIs in a PDF document and is not limited to only
  URI actions as is currently implied". 15 of 2865 annotations are structural; the noise is a NOTE
  renumbered rather than a clause. `doc/errata-read.md` carries both, and the standing answer to *what
  does this tree do about a clause number the errata have moved*.
- **Blockers (program)**: ledger 22 sentences — 6 expired, 10 holding, 6 naming no clause; source 28 —
  10, 10, 8. **Identical to the five-hundred-and-sixty-second's ten numbers.** 0 defects; every
  printed-expired hit a correction quoting the wording it retired, a past tense, or a contrastive
  "while §X".
- **Capabilities (program)**: ledger 53 — 38 witnessed by the tree, 46 about the program, 7 about one
  crate; source 156 — 128, 90, 66. 0 defects; the three sentences more than last time are this round's
  own new modules.
- **Unread (program)**: 64 rows claim, 175 keys; 48 confirmed, 127 quoted over 54 rows, 55 by the row's
  own code — **identical to the five-hundred-and-sixty-second's five numbers**. 0 defects.
- **Entries (program)**: 249 rows explain themselves by an arrival and name code, 2 name none; 788
  entries stated, 193 reported over 47 rows — 56 named nowhere, 137 only elsewhere, 55 not named by the
  row's own note. Identical, and the known populations.
- **Quotations (program)**: 3799 quotations in 595 documents, 1679 verbatim, 25 diverging; 1435 in 794
  ledger notes, 1119 verbatim, **1 diverging** — §8.4.4's correction quoting the sentence ISO 32000-2
  does not contain, which is the known shape. 0 defects.
- **Callers (program)**: 122 names no crate under `crates/` asks, 177 named by a dependent crate. The
  delta is clean.
- **Pointers (program)**: 5317 path pointers — 2939 live, 103 absent, 14 in another crate, 1858
  unrooted, 127 a form, 276 not carried; 57 symbol pointers, 12 undefined. 0 defects; every absent path
  is an ADR naming a `doc/todo/NN` its own round deleted or a correction quoting the pointer it retired.
- **Table numbers (program)**: 409 tables captioned, 305 stating entries; 4794 sentences name a table;
  1888 attributed key citations — 1742 the table agrees with, 93 absent, 4 a denial the table
  contradicts, 49 under a table that states no entries, 0 under no such table. **0 defects.**
- **`inapplicable` (program)**: 80 rows stating 310 terms — 60 named by no source, 250 named over 72
  rows, 235 carrying a cousin. None wrong.
- **Owed (program)**: 225 `partial` rows stating 2983 terms — 159 named by no source over 105 rows,
  leaving 120 rows on the reading list. Identical to its first run.
- **Retired (program)**, over the wave's ten nouns — `recover_compressed_objects`,
  `CompressedRecovery`, `object_streams`, `signed_area`, `wound_counter_clockwise`, `Path::reversed`,
  `owed`, `Digest::Shake256`, `Role::Document`, `supports_text_ranges`: **452 mentions, 2 carrying both
  shapes, 0 defects** — and `owed` is this file's own warning about an ordinary English word arriving as
  a type name for the fifth wave running, since a sweep named after it cannot be told from the debt it
  measures.
- **Arithmetic (6)**: two hits, §7.9.2 and §O, read and kept before. Clean, and still the only sweep of
  the fifteen that has never printed anything else.
- **The errata (12)**, `check`: "151 struck passage(s) of 4 words or more that `doc/md/` still carries
  as current text" over all fourteen PDFs, unchanged, and **71 quotations quoting text struck out of the
  clause they cite — the same 71 as the five-hundred-and-forty-fifth, -fifty-third and -sixty-second**.

### Fifteen rows read off the blame list, the band from commit 553 to 564

Oldest first: §12.5.1, §12.5.6.19 (553), §12.7.5.3 (555), §12.7.4.1 (556), §12.7, §12.7.6, §14.11.3,
§8.5.3.3.1 (557), §11.7.4 (559), §12.5.6.9, §12.7.5.4, §8.11.4.3 (562), §7.8.3, §8.10.2, §8.4.5 (564).
**Two were wrong, one of them a status, and both are the fifth failure shape with the right answer one
clause away.**

| row | shape | was | is |
|---|---|---|---|
| **§7.8.3** | 5, 7 | `partial` because `/Properties` "is read only for `/OC` (§8.11.3.3) and not for §14.6.2's marked-content property lists in general — which is clause 14's gap rather than this one's" | `content/marked.rs`'s `property_list` resolves a `BDC` operand's **name** through the `/Properties` subdictionary for §14.9's four entries, §14.7.5.2's `/MCID` and §14.13.5's associated files, which is this clause's own rule about a list holding an indirect reference — and §14.6.2's own row has listed what a property list is read for all along. `/ProcSet` is the only entry nothing looks at, and §14.2 is `inapplicable` because the array "shall be used only when the content stream is printed to a PostScript language compatible output device". **`implemented`**, with one named test per requirement group |
| **§8.4.5** | 1, 5 | five of Table 57's entries on the not-read list: `/SM` "the silence recorded under §10.7.3", and `/BG`, `/BG2`, `/UCR`, `/UCR2` because they "describe a marking device" | `content/ext_gstate.rs` reads all five. `/SM` is the smoothness tolerance `pdf_render::shading::Ramp::resolution_for` samples a shading by, which **§10.7.3's own `implemented` row has said since the seventy-fourth session**; the four black-generation entries set `black_generation_stated`, which is what makes §11.7.2's `DeviceCMYK` group space a *reported* departure rather than a silent one. `partial` now for **`/FL` alone**, which is the one of Table 57's routes that reaches nothing while `i` is read and discarded — trap 5's shape at a cost of nothing |

**The two rows are one lesson twice**: a not-read list is a list of claims, and the row that gets
corrected when a mechanism arrives is the mechanism's own row rather than the list that mentions it.
§10.7.3 and §14.6.2 both hold the right answer, in the same words, in a row nobody reading §8.4.5 or
§7.8.3 would open. **And neither is visible to any of the fifteen sweeps**: `/SM` and `/Properties` are
*keys the row says are unread*, which is the second sweep's own subject — and the second sweep asks
whether a **source quotes the key**, so a key the tree reads under its own row's `code` array reads as
noise in the one-short-key population it prints every run.

**The thirteen rows read and kept each record the evidence that kept them** — the `Refusal::NotDerivable`
answers for §12.5.6.19's four pointer states, the absence of any `"CO"` in the tree under §12.7's
calculation order, `MAX_FIELD_ANCESTRY = 32` reported rather than silent under §12.7.4.1, `TextControl`
carrying the three flags §12.7.5.3 says are carried rather than applied, the one reader of `"IT"` and of
`"Measure"` under §12.5.6.9, `examples/oc_usage_census` still being the only thing that names `/Configs`
under §8.11.4.3, and — under §8.10.2 — `Xmp::read` and `attachment::associated` both being written for
any dictionary while their one caller each passes the catalog, which is the fifth sweep's shape rather
than that row's.

## A sixteenth sweep, and it reads a claim no other one looks at: **"no corpus document does X"**

Built in the five-hundred-and-seventieth session (ADR 0405). The fifteen above all read a row's
*reason* — a blocker, a vocabulary, an architecture, a capability, a citation, a count of children.
This one reads a row's **evidence**, and specifically the negative kind: a sentence asserting that
nothing in the corpus exercises the clause. That sentence is what parks a requirement, and nothing
in the tree had ever re-run one.

The population is one grep and it is large — around 165 live sentences of the shape "no corpus
document …", "nothing in the corpus …", "no witness", across `doc/conformance/ledger.toml`,
`doc/todo/`, the standing `doc/*.md` and `crates/`. What makes the sweep tractable is that most of
them name a **PDF name**, and a name is countable:

```sh
cargo run --release -p pdf-model --example witness_census -- --pdfjs Collection Threads IDTree
cargo run --release -p pdf-model --example absence_audit          # the structural half
cargo run --release -p pdf-model --example operator_shape_census  # the third half: an *order*
```

`witness_census` asks each term three ways of the same document — the file's raw bytes (what a
grep sees), every object the cross-reference table names *including the ones inside object
streams*, and every stream's decoded data — and prints the three counts side by side.
`absence_audit` re-asks the same claims through the readers that would act on them, because a name
being present is not the structure being present. **Run both**: that is ADR 0403's rule, and here
it is the object walk that finds what the byte search cannot rather than the other way round.

**And a third instrument answers the claims neither can, which is a sentence about an *order* of
operators** (ADR 0548). `operator_shape_census` lexes a page's `/Contents` and every form `XObject`
its resources reach, so a witness that is a sequence — a segment with no move before it, a `q` and a
`Q` with a `Tm` between them — is countable the way a name and a token already were. It skips inline
image data through `pdf_model::inline_image::scan`, because bytes lex into keywords and an image
whose samples spell `l` would be a witness the census invented; and it prints how many content
streams it does **not** reach, which for the crawl's first pages is 37 685 pattern cells, Type 3
procedures and appearance streams.

**What it counts is the clause's shape, which is not the same population as the program's
behaviour** (ADR 0563, trap 13's second shape). A lexer sees a keyword; the interpreter also
requires the operator's *operands* to parse as numbers, and on §8.5.2.1's one curated witness that
second condition removes every hit. So a row is settled by this census when the question is
*whether the standard's shape occurs*, and by an instrument that interprets when the question is
*what it costs the page* — `cargo run --release -p pdf-model --example refused_segment_census` is
that one for §8.5.2.1, counting the report the interpreter already raises.

### The three ways such a claim goes wrong, and the first run found all three

- **A stale count.** §14.7.2's `/IDTree` was "no corpus document has one at all — the 89 tagged
  ones state none" in three places while `Tree::element_by_id`'s own note, corrected by an earlier
  round, said 12 of those 89. A correction that stops at the document it was written in is
  `doc/habits.md`'s "a retired claim is a string, and strings are greppable", and this is its
  largest instance: one crate contradicting itself in two files.
- **A count contradicted inside one file.** §14.10.2's row said "[n]o corpus document writes a
  `/SpiderInfo`" while §7.7.2's row, in the same `ledger.toml`, listed "`/SpiderInfo` (§14.10's web
  capture, **5 documents**)". Both were written from a measurement; only one of the measurements
  was taken.
- **A population the sentence does not name.** §12.4.3's articles and §12.3.5's collections were
  each "no corpus document" and each true *of pdf.js*. The four `doc/corpora/` submodules hold four
  documents with 115 beads between them and one portable collection with a folder tree — and every
  other absence claim in this tree is stated over those submodules too. **"The corpus" is two
  populations, and a claim is only true against the one it was measured over.**

### The rule that comes out of it, and it is what a round should apply

**Say which corpus.** A sentence that says "no corpus document" without naming a population cannot
be checked without re-deriving what the writer meant, and two of the five falsifications above were
nothing but that ambiguity. `--pdfjs` exists on both examples so that the narrower answer is one
flag rather than a different run.

**And a negative claim about a population needs a re-run, not a re-read.** The other fifteen sweeps
find a reason that expired; this one finds a *number* that was wrong when it was written, and no
amount of reading the sentence beside the code reveals that. It is the only sweep here whose
instrument is a program over the corpus rather than a pattern over the tree.

## A seventeenth sweep, and it reads a claim the twelfth cannot: **an erratum this tree *records***

Built in the five-hundred-and-ninety-first session (ADR 0426), out of the five-hundred-and-ninetieth's
finding: the §14.8.4.7.2 row had **recorded** Errata Collection 3's Issue #437 since the
four-hundred-and-eighteenth and then **quoted the sentence that erratum struck out**, two sentences
later, while four places in `crates/` quoted it as current text. Its lesson is the sweep's whole
subject — **a row that records an erratum is not a row that has applied it** — and nothing looked
for it. The twelfth sweep (`spec-errata check`) asks whether a quotation lands on struck text and
knows nothing about whether the writer had read the erratum, so a place that names it and quotes
the old words reads exactly like a place that never heard of it. **A row that names the erratum
looks maximally diligent, which is precisely why nobody re-reads it.**

```sh
cargo run --release -p spec-errata -- applied doc/*.pdf
```

Two seconds, over `ledger.toml`, every comment run under `crates/`, `tools/` and `fuzz/`, and every
Markdown block under `doc/` bar `doc/history/`. It is in the sidecar rather than under `conformance`
for ADR 0252's reason, which is a rule and not a convenience: the errata are read out of fourteen
PDFs, and nothing this project generates may become what the gate checks the standard against.

**The discriminator is that nothing is inferred.** Every other sweep over quotations has to guess an
attribution — `check` takes the clause from the nearest citation above the span and calls its own
buckets a sort order rather than a verdict. Here the erratum is named as *data*, by the writer,
inside the place itself, and the erratum supplies **both** sides of the comparison: the `StrikeOut`'s
covered text and the `Caret`'s `/Contents`, joined by Table 172's `/IRT`.

**The noise, printed rather than filtered.** A correction quoting the wording it retired is this
family's oldest false positive and here the commonest hit by construction, since the honest way to
record an erratum is to say what the sentence used to be; it is marked `[history]` from a window
either side of the quotation, because this project writes a correction in both orders and a
backwards-only window marks half of them. `doc/errata-read.md` is that shape from end to end and is
counted apart. A `#NNN` this collection does not carry is dropped and counted, so a clean run says
what it was clean over.

**Its first run** compared 1068 quotation-against-erratum pairs over 43 976 places, 372 of which name
an erratum, against 771 changes carrying 363 issue numbers, with 175 `#NNN` tokens naming none — and
put **26** hits on the read-first list. Every one was read. **Two were the §14.8.4.7.2 shape one clause family over, both in §9.6.2.2's
row**: the note opens by quoting the sentence Issue #47 and #48 strike outright, three thousand
characters above its own record that they do, and the five-hundred-and-twenty-third session added a
second quotation of it four sentences from that record while writing ADR 0358. The rest are the
annotations sessions 417 to 419 wrote in place and four dated ADR records.

**And the marker was half the erratum, which the six-hundred-and-fifth found by reading the list
again** (ADR 0440). Every hit still on the read-first list that lived under `crates/` was *correct
writing* — `structure.rs` saying an erratum "replaced it", `type3.rs` saying one "replaces" a
sentence, `write.rs` saying one "has edited that sentence", three places round §12.5.2 saying Issue
#287 "sharpens" the `/BS` precedence. `HISTORY` carried the verbs for what an erratum **removes** and
none for what it **puts there**, and both retire the quoted words equally. `replace` and `sharpen`
join it; **`makes it` does not**, because the note this sweep was built for opens with it — that is
the same line ADR 0426 drew against borrowing `blockers::HISTORY`'s `said`, and it is a test now
rather than a sentence. The read-first list falls 22 → 10, which is where a round wanting to read one
starts.

**And the round that built it repaired the twelfth sweep's comparison**, which had been blind to two
spellings this project's own rules produce: `CLAUDE.md`'s `"[e]ncloses"` for an altered first letter,
and the em dash `doc/md/` writes as a hyphen in a table caption. `squeezed` is
`conformance::prose::folded` now — one comparison in the crate rather than two — and the thirteen
landings that became visible held three more defects, none of which `applied` could see because none
of the three named the erratum: §9.6.2.1's row quoting both of Issue #47 and #48's struck sentences,
and **two rustdoc blockquotes** quoting the sentence Issue #462 strikes out of §9.10.3, in the one
population this project gates. §9.10.3's own row had recorded that erratum four rounds earlier and
closed by warning that "a later round quoting it as current would be quoting text the collection has
removed" — the quotations were already there when it was written.

## Twelve sweeps run in the six-hundred-and-sixteenth, and the blame list is **not** exhausted below commit 534

The twelve committed programs plus `spec-errata`'s `emit`, `check` and `applied`. What each
printed: `blockers` 28 blocker sentences, 10 expired by the ledger's own account and 9 of those
carrying `[history]`; `capabilities` 165 sentences, 136 witnessed; `owed` 225 `partial` rows stating
3171 terms, 168 named by no source, 115 rows whose every term the tree names; `unread` 66 rows and
180 keys, 135 quoted somewhere, 61 of them by the row's own code; `entries` 792 table entries over
265 rows, 186 reported, 47 of them not named by the row's own note; `callers` 133 names no crate
asks; `pointers` 6323 path pointers, 120 absent, 13 undefined symbols; `tables` 2020 attributed key
citations, 95 absent and 6 a denial the table contradicts; `inapplicable` 79 rows stating 309 terms,
57 named by no source; `counts` 313 attributed counts, 4 places counting one family twice;
`quotations` 4514 document quotations with 31 diverging and 1572 ledger ones with 1 diverging;
`applied` 1240 comparisons with 10 on the read-first list.

**The reading list came from the blame ordering rather than from any of those**, and the finding is
what the ordering says about this file. The bullet below asserted that nothing under commit 534 was
unread. Ordering every `partial` and `reported` row by the commit that last wrote its own `note = `
line puts **38 of them below 534, and 18 below commit 200** — §8.9.6.4 at commit 87, §8.10 and its
three children at 89, §7.6.4.4.2, §7.6.5.1 and §7.6.5.3 at 94, §7.9.2 at 95, §8.7.4.1 and §8.7.4.3
at 100, §14.8.2.2.1 and §14.8.2.2.2 at 161, §12.11.3 and §12.11.6 at 162, §14.13 and §14.13.2 at
165, §12.6.4.6, §12.6.4.9 and §12.6.4.10 at 176, §12.5.6.12 at 199. **None of them carries a
read-and-kept sentence**, which is the mark this practice leaves and the reason the blame moves when
a row is read; §7.9.2's and §12.6.4.6's notes are the two checked by hand.

So the bands were bands rather than a floor: each round took the *top* of a list and the rows under
it stayed where they were, and a sentence written about the band was read afterwards as a sentence
about the file. **The four this round read came off exactly that population** — §8.7.4.3, §8.7.4.1,
§8.9.6.4 and §8.6.6.5, commits 100, 100, 87 and 91 — and two of the four were wrong: §8.7.4.3's
`/Background` was a `shall` nothing painted and nothing reported (ADR 0452; painted since ADR 0529), and
§8.9.6.4's "both corpus instances" was three, over a population it never named. **That is one defect
per two rows on a list this file had declared empty**, which is the rate the four-hundred-and-forty-second
found at the top of it.

The other two were confirmations with the evidence written into the note, which is what a re-read
owes when it finds nothing: §8.7.4.1's "no corpus document writes an `/ExtGState` on a Type 2
pattern" was re-*derived* rather than re-read — 38 of the 974 hold such a pattern, none states one,
and the four `doc/corpora/` submodules hold no Type 2 pattern at all — and §8.6.6.5 gained Errata
Collection 3's Issue #309, which moves the `None`-in-`NChannel` restriction off the sentence that
gives `None` its meaning and states it separately as a `shall not` on the file, ratifying a reader
that has never read `/Subtype`.

## The bottom of the list read in the six-hundred-and-twentieth, and a better rule for choosing than either instrument

**The band the six-hundred-and-sixteenth named, read.** The order was re-derived rather than
taken — `git blame --line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported`
row's own `note = ` line, ordered by where its commit falls in `git log --reverse` — and the
re-derivation found a defect in the *list* before it found one in a row.

**The blame ordering agreed with 616 on seven of its eight rows and disagreed on the eighth, which
was the one 616 recommended reading first.** §12.5.6.12 was read in the six-hundred-and-fourteenth
session (ADR 0450's spec-driven half) and its note rewritten, and the two rounds ran in parallel,
so 616's base did not contain 614's commit and 616's list was accurate about a tree that no longer
existed by the time anybody could act on it. The band actually at the bottom is therefore **eight
rows and not nine**: §8.10 and its three form-XObject children, §7.6.4.4.2, §7.6.5.1, §7.6.5.3, and
§7.9.2.

**And the *threshold* is not a number that survives a merge.** 616 wrote "38 rows below commit 534
and 18 below commit 200"; the same query on this base puts 29 below 534 and 17 below 200, and
almost none of that movement is rows being read. `git log --reverse` linearises merges, so a
parallel round landing shifts every index above it — the absolute commit number is a property of
the base rather than of the ledger. **Order by rank, quote a row's position as its rank, and never
carry a commit index between rounds.**

### What each of the eight turned out to be

Five defects in eight rows, and two of the five were status rather than prose:

| row | shape | was | is |
|---|---|---|---|
| **§7.9.2** | 6 | `partial` for "the object model carr[ying] one string type where §7.9.2.1 names three" | the clause opens "PDF supports one fundamental string object" and §7.9.2.1's own `implemented` row says "[t]his crate holds exactly that" — **`implemented`**, ADR 0455 |
| **§7.9** | 4 | `partial` was "§7.9.3's text streams alone, which is `reported`" | §7.9.3 has been `implemented` since the three-hundred-and-eighty-seventh — **`implemented`**, and found only because §7.9.2 moved out of the way |
| **§8.10.4.3** | 9th sweep's | "Three considerations" | **two**; the third is §8.10.4.1's closing sentence and Table 93's `/Group` cell, attributed to the wrong clause |
| **§7.6.5.3** | 1 | "[t]he SHA-1 and AES seed algorithms" | the clause names a *digest* (SHA-1 at 128 bits, **SHA-256** at 256) over seed + `/Recipients` + 0xFF, and separately seven CMS content ciphers; the row had run two mechanisms together, and never recorded that Issue #196 renumbers it to §7.6.5.4 |
| **§7.6.4.4.2** | new | "[s]teps (a) to (d) are implemented", citing a corpus test | the steps are right, and **the cited test does not reach them**: the corpus's eight password-protected documents are three at revision 3 or 4 that all open on their *user* password and five at revision 6, where Algorithm 12 replaces Algorithm 7 |

The other three were confirmations with the evidence written into the note: §8.10's aggregate owes
only §8.10.2's unread entries and §8.10.4's unbuilt import; §8.10.4 and §8.10.4.1 gained the
population they had never stated — `witness_census` over all 1251 PDFs on this disk finds two
documents stating `/Ref` as a name and both are §14.7.2's structure-element `/Ref`, so **no
document states a reference XObject at all**; and §7.6.5.1 gained the one `shall` the clause puts
on a reader, quoted, in place of "As §7.6.5."

**The fifth row's shape is new and worth a name: the row is right, its evidence is not.** Every
other shape in this file is a note that says something false. This one says something true and
points at a test that cannot show it — which is worse, because a reader who checks the citation
gets a green test. The check it needs is the eighth sweep's with the question changed: not *does
the test exist* (`pointers` already asks that) but *does it execute the requirement the row claims*.
Nothing in this project asks that, and no program can — it is read by opening the test. What the
round did instead is what a round can do: write the test. `crypt.rs` runs Algorithm 3's steps (e)
to (h) forward and asserts this reader's steps (a) to (d) unwrap the result, at revisions 2, 3 and
4, and it is the only thing in the tree that reaches `unwrap_owner_entry`.

### The rule for choosing, which is what neither instrument gives

The blame ordering ranks by *age*; the twelve sweeps rank by *the quality of a stated reason*, which
616 showed is a proxy for recency. Neither ranks by how likely a row is to be wrong. The band
suggests one that does, because all five defects share it:

> **Rank by blame, then read the row whose stated reason is a claim about this codebase rather than
> a claim about the standard.**

§7.9.2's reason was an architecture preference; §7.9's was another row's status; §8.10.4.3's was a
count of a list; §7.6.5.3's was two algorithm names; §7.6.4.4.2's was a test path. Not one quoted
the clause. A note that quotes the standard was checked against the standard when it was written
and the standard has not moved; a note that describes the tree was true when it was written and has
been ageing since. **`quotations` already measures exactly this** — it prints, per note, how much of
it is verbatim specification — so the discriminator is available as a number to whichever round
wants to make this a thirteenth program rather than a sentence.

## The band under rank 10 read in the six-hundred-and-twenty-sixth, and the rule held

**Nine rows, not eight**, and the ordering was re-derived rather than taken: `git blame
--line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = `
line, ranked by where its commit falls in `git log --reverse`. **The blame ordering agreed with
620 on all nine** — three parallel rounds were running beside this one and none of them had taken a
row off it — and the jump after rank 9 is large: ranks 1 to 9 sit at four commits, and rank 10 is
five hundred commits later. The nine are §14.8.2.2.1 and §14.8.2.2.2 (rank 1–2), §12.11.3 and
§12.11.6 (3–4), §14.13 and §14.13.2 (5–6), and §12.6.4.6, §12.6.4.9 and §12.6.4.10 (7–9), which is
620's list with the three action rows counted as three.

**620's rule picked the defect on the first pass.** Ranked by whether the stated reason is a claim
about this codebase rather than about the standard, §12.11.3's and §12.11.6's went to the top —
"the clause states no threshold", which reads like a claim about the standard and is a claim about
what somebody looked for — and that pair was the round's work.

### The threshold three documents denied

**§12.11.3 states a threshold and this tree recorded three times that it does not**: the two ledger
rows and `requirements.rs`'s module header. The sentence is the clause's fourth paragraph, and the
computation §12.11.6 names is a **sum over the unmet requirements** compared against 100 — both
halves stated rather than inferred, from Table 273's bound on one entry and §12.11.3's own "total
penalty points". `requirements::penalty_total` now performs it and `viewer_core::notes` says it;
the `should` is still declined, and the declining now rests on the word `should` rather than on a
silence. ADR 0460.

**The shape is `CLAUDE.md`'s own, one level down.** Principle 5 warns that "the specification
defines nothing here" is a claim about the specification and decays. This is its narrower cousin —
*the specification defines no number here* — and it decays the same way, for the same reason: it is
a claim about a search somebody did once. **A row saying the standard states no threshold, no
default, no order, no limit is a row to re-read, and the check is cheap**: read the clause's last
paragraph. All three of this round's denials were of a sentence in the final paragraph of the
clause, which is where a standard puts the consequence after it has finished defining the terms.

### The other six

| row | shape | was | is |
|---|---|---|---|
| **§12.6.4.9** | 1 | `Sound` is "[a]djacent to clause 13's exclusion and not covered by it, because §12.6.4.9 is in clause 12" | the clause's own first sentence hands it to §13.2, **word for word the sentence §12.6.4.10 opens with** — two neighbouring rows, one identical sentence, opposite readings |
| **§12.6.4.6, .9, .10** | 620's new | "`viewer-ui` prints it when a click reaches one", citing `a_name_the_table_does_not_hold_is_not_an_action` | the cited test asserts a name outside Table 201 yields **no** action, the one path that never calls `action::refused`; `Sound` and `Movie` were reached by nothing at all. A click test now reaches all three, and the `code` array named `pdf-viewer.rs`, which holds none of it — the printing is `dispatch.rs` |
| **§14.13** | 9th sweep's | "it lists seven objects that may carry one and says the same sentence about every one" | §14.13.1 lists **eight** — the eighth is a metadata stream, about which §14.3.2 says nothing — and the sentence is *not* the same for the third, whose key is `/MCAF` in a property list. A reader who believed the row would state seven sites and find six |
| **§14.8.2.2.1** | 9th sweep's | "the clause's other test — '[a]ny content that is not included in the structure tree is an artifact…'" | that sentence is **§14.8.2.2.2's**, and §14.8.2.2.1's own `shall` — artifacts in the structure tree go through the `Artifact` element type — had never been quoted by either row |
| **§14.13.2** | 17th sweep's | no erratum recorded | Issue #568 states the two `/AF` forms as a `shall` each for the first time, and Issue #86 puts a UTF-8 `shall` on every name key on the same page |
| **§14.8.2.2.2** | 17th sweep's | leans on a `shall` about EXAMPLE 1 and EXAMPLE 2 | Issue #484 turns that paragraph into NOTE 2 — reading both forms is now a reading of a note. Nothing moves; a reader that accepts both accepts either text |

**§14.13 also stopped restating its children's numbers**, which is sweep 10's shape from the other
direction: it said "6 on catalogs, 30 on structure elements" where §14.13.6's row says 37 arrays
in total, so the parent's breakdown and the children's did not add up and neither could be checked
against the other. The parent now carries the one number that is its own — how many documents state
an `/AF` at all, with the command that counts it — and each site's share stays in that site's row.

### A sharpening of 620's rule, from the row it did not find first

620's rule ranks by *what kind of claim* the reason makes. This round adds *where in the clause the
answer would be*, because it is what made the §12.11 pair cheap and the others slow:

> **A note that says the standard states nothing has to name where it looked.** "The clause states
> no threshold" is checkable in a minute against the clause's last paragraph; "the clause states no
> threshold, and its closing paragraph is about X" is checkable against nothing and would have been
> written by somebody who read it.

That is not a new sweep. It is a rule for *writing* a row, and it is the only kind of rule that
makes the next re-read cheaper rather than the current one.

## The band 626 named read in the six-hundred-and-thirty-second, and both defects were a *status*

**Sixteen rows, and the boundary moved exactly as 626 warned it would.** Re-derived the same way —
`git blame --line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own
`note = ` line, ranked by where its commit falls in `git log --reverse`. **The blame ordering
agreed with 626 on the seven rows it named** and then went on for nine more: 626's rank 10 is this
base's rank 1, and what 626 saw as a gap of five hundred commits is now a band of sixteen at seven
commits, with the next row forty-two commits above it. The sixteen are §12.7.4 (rank 1), §12.7.6.2
(2), §8.7.3 (3), §11.7.5 and §12.11 (4–5), §14.8.4 and §14.9 (6–7), §12.8.2.2, §12.8.3.3.2,
§12.8.3.4.1, §12.8.3.4.2 and §12.8.5 (8–12), and §12.8.3.4.4, .6, .7 and .8 (13–16) — nine of the
sixteen written by two signature rounds, which is why the band is suddenly long. **Ranks, not
indices**: this base has 816 commits and 626's numbers do not name the same rows in it.

**620's rule chose the work for the third time.** Of the sixteen, four stated a reason that is a
claim about this codebase rather than about the standard, and all four paid:

| row | shape | was | is |
|---|---|---|---|
| **§14.8.4** and **§14.8.4.2** | 620's second — the *status* rather than the note | `partial`, because "Annex L's nesting rules are not checked, and are a validator's job rather than a reader's" | that is an argument for owing nothing, and the two rows it names as precedent — §7.11.2.1 and §7.12.4 — are `implemented` for it, as Annex L's own row is `writer-side`. §14.8.4.2 states one `shall` and `standard_role` executes it; §14.8.4 states no prose at all. Both `implemented`. ADR 0465 |
| **§11.7.5.2** (reached from §11.7.5, rank 4) | 620's second again, in the other direction | `inapplicable`, because per-region tracking "needs a second transfer function competing with a first inside a transparency group" | the clause needs no second function: where the topmost object is not fully opaque, "the default halftone and transfer function for the page shall be used", and this tree applies the object's. `silent`, with what is drawn wrong, what would report it, and a population measured at **zero**. ADR 0465 |
| **§14.9.4** (reached from §14.9, rank 7) | 620's newest — the row is right and its evidence is not | `implemented`, "[b]oth of the clause's locations are read" | Errata Collection 3 Issue #483 makes it three, adding an `Artifact` tag's property list; the interpreter meets it by construction and **nothing in the tree ran it**. A test does now, mutation-checked |
| **§8.7.3** | a confirmation, re-derived rather than repeated | "that outline is the backends' to compute … so there is no path here to tile" | still true, and now with the evidence in the row: no crate that builds a display list depends on `kurbo` or a rasteriser, and all three backends expand a stroke themselves. Two errata on §8.7.3.1 recorded, neither moving a requirement |

### The sharpening this round adds: **a settled status is an argument, and the argument names rows**

620's rule ranks by what kind of claim a reason makes; 626's adds where the answer would be. Both
of this round's defects were in the *settled* half of the vocabulary — `implemented`,
`inapplicable`, `writer-side`, `out-of-scope` — and that half is where no sweep looks, because a
claim that nothing is owed has no missing thing to grep for.

> **Where a row's note cites another clause as precedent, the precedent has a status.** Either the
> two agree or one of them is wrong, and which is which is a minute's reading.

§14.8.4.2 cited §7.11.2.1 and §7.12.4 by number, in its own note, and held a different status from
both for two hundred and fifty sessions. **That is a sweep somebody could write and it is not
written**: the population is every row whose note names a clause that has a row, and the hit is a
disagreement between the citing row's status and the cited row's. It is not the eighth sweep, which
asks whether a *file* a note names exists; it is the same question one level up, about a claim
rather than a pointer. Whichever round wants a thirteenth program has it named here.

**And an `inapplicable` row's reason decays exactly as `CLAUDE.md` says a scope line does.** The
seventh sweep reads `inapplicable` rows already, but it asks whether the *tree names the row's
vocabulary* — a discriminator that cannot see a row whose vocabulary is right and whose account of
the requirement is wrong. §11.7.5.2 names `/TR`, `/HT`, `/BG` and `/UCR` and the tree names all
four; the sweep had nothing to say. Only reading the clause found it.

## The band read again in the six-hundred-and-thirty-seventh, and the rule paid a fourth time

**Twelve rows at five commits now, where 632 found sixteen at seven.** Re-derived the same way and
on this base, which has 824 commits: the four rows 632 read have left the band, and what is left is
§12.7.4 (rank 1), §12.7.6.2 (2), §12.11 (3), §12.8.2.2, §12.8.3.3.2, §12.8.3.4.1, §12.8.3.4.2 and
§12.8.5 (4–8, one commit), and §12.8.3.4.4, .6, .7 and .8 (9–12, one commit). The next row above is
**forty-two commits away** — §10.4.2.4 and §10.4.2.5 — which is the same gap 632 measured, because
nothing between has moved. The band is now entirely §12.7 and §12.8, which is what happens when a
band is worked from the bottom: what survives is what two signature rounds wrote in two sittings.

**620's rule chose the work for the fourth time, and 620's *newest* shape is what paid.** Of the
twelve, four state a reason that is a claim about this codebase — §12.7.4, §12.11, §12.8.2.2 and
§12.8.3.4.2 — and the other eight rest on a capability (a network, a trust store) that
`CLAUDE.md` excludes or the sandbox forbids. Rank 1 was read.

| row | shape | was | is |
|---|---|---|---|
| **§12.7.4** and **§12.7.4.1** | 620's newest — the row is right and its evidence is not | both cited `variable_text.rs::quadding_moves_the_line_within_its_box` as their only test, and §12.7.4.1's opening claim is "Table 226's inheritance is implemented" | that fixture is a **single merged widget**, so the `/Parent` chain it walks has no links and §12.7.4.1's inheritance was asserted by nothing in the tree. `form.rs::a_fields_type_flags_and_value_come_from_the_ancestor_that_states_them` states `/FT`, `/Ff` and `/V` two links up and reads all three back; cutting `MAX_FIELD_ANCESTRY` to 1 fails it. §12.7.4's `code` array named only `appearance.rs` while every sentence in its note is about `view.rs` and `form.rs`. ADR 0469 |

### What this round adds: **a family head's citations are the least maintained thing in the ledger**

632 found the *status* of a family head stale because "a family's parent row is not maintained by
the sessions that implement its members" — §12.7.4's own note says exactly that sentence about
itself, and was corrected in the three-hundred-and-seventy-first for it. What nobody corrected was
the arrays the corrected note rests on. So:

> **When a note is corrected, the `code` and `test` arrays are corrected in the same edit or they
> are not corrected at all.** A note argues about three children and cites one child's test; the
> prose reads as maintained and the evidence is whatever the row was first written with.

That is greppable and it is a **nineteenth** sweep somebody could write — it was written here as
"a thirteenth", which was an ordinal two other things already had, and the six-hundred-and-forty-fifth
separated the count of sweeps from the count of committed programs (ADR 0475 §1): for each
row, does its `note` name a source file that its own `code` array omits? §12.7.4 named `view.rs`
and `form.rs` in prose and neither in its array, which is the hit. The eighth sweep already checks
that a path a note names *exists*; this asks whether the row's own arrays agree with its own
sentences.

### What the six-hundred-and-forty-first adds: **a counted claim in a note owes a command**

`CLAUDE.md`'s rule — "a fact that can be counted is not written down; what is written down is the
command that counts it" — binds the instruction files, and the ledger has always been exempt
because a row's job is to record a claim. That exemption is where two of this round's findings
were. §12.8.3.3.2 said "`issue17069.pdf` is the corpus's one witness" and there are three; §12.8.5
said "no corpus document carries a document timestamp" and that one holds. Neither number had
anything behind it, and the difference between them was invisible until both were re-derived. So:

> **A note stating a count over the corpus names the command that produces it, or the round that
> writes the count adds one.** Two counters in an existing census is usually the whole cost —
> `signature_algorithm_census` grew both of these in about twenty lines, because the walk that
> answers the row's real question already had the data in its hand.

It is the ledger's half of what `tools/state.sh` is for the instruction files, and it is cheaper
than the alternative the tenth sweep already measures: a cardinal that outlives its measurement
gets *quoted*, and then a later round has two documents to correct instead of one.

### What the six-hundred-and-forty-eighth adds: **the count and the walk that produced it**

The rule above paid four times in one round — §12.8.2.2's "the corpus's one certification
signature", §12.8.3.4.2's "four corpus documents", §7.6.4.1's "eight corpus documents" and a
comment's "exactly one states a `/S /Launch` action" — and three of the four were **right**. The
fourth was wrong, and the way it was wrong is the part worth keeping:

> **A count over the corpus is a claim about a walk as much as about the world.** "Exactly one
> document states a `/S /Launch` action" was produced by something that visited only the objects
> the cross-reference table lists, and an action dictionary written *directly* inside its
> annotation or its outline item has no object number. The right answer is two. The same bound
> reported zero `/S /GoToR` and zero `/S /SubmitForm` over a corpus that states one of each — which
> would have made two of this round's rows look unrankable.

So when a note's count is re-derived, re-derive the *population* with it: ask what the walk cannot
see before believing a zero, and prefer a zero that has been probed to one that has been computed.
The cheapest probe is the crudest one — `grep -l` for the literal name over the corpus's bytes,
which is what caught this: two files held `/GoToR` and the census claimed none did.

Three of this round's four counts also needed no new instrument. §7.6.4.1's was already produced by
a gate — `cargo test -p pdf-model --test corpus -- --ignored` prints every locked file and
`MAX_LOCKED` ratchets the total — and the row simply never said so. **A note whose count already
has a command owes the command's name, not a new census.** Look for the gate before writing one.

## An eighteenth sweep, built in the six-hundred-and-forty-fifth: **a parent's claim against its own children**

The six-hundred-and-forty-first found §12.11 by reading and said outright that no sweep could have
printed it. This is that sweep — `cargo run --release -p conformance --bin overstated`, ADR 0475 —
and it is the **first one that opens no source file at all**: a parent row saying an entry or a
table *is read* makes a claim its descendants are the detail of, so both sides are sentences in
`ledger.toml` written by this project about its own code. A contradiction is then a contradiction
whatever the standard says, which is the ambiguity every tree-facing sweep has to read past.

**Why the twelve committed programs are blind, stated once.** The seventh reads a term the tree *names* under a row
claiming nothing is owed; the fourteenth reads a term the tree *lacks* under a row claiming a debt.
An overstating parent names a thing the tree lacks under a row claiming the **opposite** of a debt.
The sign is reversed twice, so neither can be widened to reach it without turning into the other.

Three judgements went into it and each is worth keeping:

- **The denial vocabulary is `unread::CLAIMS` unchanged**, so this sweep and the second cannot come
  to disagree about what a denial is.
- **The assertion vocabulary is five words on a word boundary** — "read" as a whole word is this
  ledger's verb, and the boundary is what keeps "unread", "reader", "reading" and "already" out. One
  idiom had to be excluded by name: "**Read and kept** in the five-hundred-and-sixty-fifth" says a
  *round read the row*, not that the tree reads the entry, and it was two of the first run's hits.
- **Stance is a property of a clause rather than of a sentence**, so `unread::sentences` could not be
  reused. §14.12.4's row is the witness — "Table 409's `/Start` and `/DParts` are read; Table 408 is
  not" holds both stances inside one full stop, and read whole it asserts the opposite of what it
  says.

**Read the unmarked hits first.** The dominant noise is *a table read in part*: the parent names the
entries it reads and the child the entries nobody reads, both citing the same table. Marking it needs
the **ninth sweep's attribution rule** rather than a plain key comparison, and §12.11 is why — its own
row enumerates "Table 273's `/S`, `/V` and `/Penalty`, Table 275's twenty-five types, Table 276's
handlers", so a mark counting every key in the part as the asserted table's would have demoted the
Table 276 claim on Table 273's keys, and the one defect the sweep was built for would have printed as
noise. The noise it leaves on purpose is a partitive with no table to divide it — §14.9.2's "[t]hree of
the four locations a `/Lang` may occupy are read" against §14.9.2.2's fourth read by nothing, both true.

### Its first run: nine contradictions, two defects, and the planted one it names

170 rows have descendants and assert 118 terms between them; 49 of those a child asserts too. Nine
are contradicted: four carry a mark that demotes them, two more sit on the third rung, one is
§14.9.2's partitive, and **two were defects**.

**§9.9.1 said Table 125's three lengths were "read by nobody", and §9.9's own row had contradicted it
for twenty sessions.** `pdf_font::program::stated_extent` reads all three since ADR 0459 — `/Length1`
alone for a `/FontFile2`, since Table 125 makes it "the entire TrueType font program", and the sum of
the three for a `/FontFile` — and the claim is checkable at all because each is stated in bytes "after
it has been decoded using the filters specified by the stream's Filter entry". What the lengths are
*not* used for is what the sentence was written about: `read-fonts` finds the eexec boundary in the
bytes rather than at `/Length1`, so no outline depends on them. The row even carried "**Read and kept
in the five-hundred-and-forty-fifth session**", which was true when written; the six-hundred-and-twenty-fifth
made it false and did not come back. **The fifth failure shape with the sign reversed, inside one
family: a parent that had outgrown its child.** The `partial` is unchanged and is the `/Length3`-of-zero
requirement, still not executed.

**§9.7.6 said "Table 119's entries are read" and its own child says one of the six is not.**
`/BaseFont` is deliberately unread for a Type 0 font on the clause's own NOTE — "an arbitrary name,
since there is no font program associated directly with a Type 0 font dictionary" — which §9.7.6.1's
row has said all along. Five of six, and the parent claimed the table.

**And the instance it was built from was planted back**, which is trap 13 and is the whole reason to
run that check: with §12.11's pre-six-hundred-and-forty-first note restored the sweep names it on
rung 2, unmarked, quoting both sides; with the corrected note it names the correction instead, marked
as a row quoting the wording it retired. A sweep written over the wrong side of a defect reports a
clean tree.

**The discriminator not taken is named in ADR 0475 §5** — a row against the *tree*, `--bin capabilities`
with the sign flipped — with what it would cost: its answer side is already `--bin owed`'s
measurement, its noise is the half a program cannot settle (a row describing what a *clause* requires
rather than what this tree does), and **it would not have found §12.11**, because source comments cite
table numbers freely and a tree-facing matcher would have reported the claim corroborated.

## The mirror of the eighteenth sweep, measured in the six-hundred-and-fifty-second and **not** built

The obvious next instrument after ADR 0475 is the same comparison with the sign flipped: a parent
row **denying** that anything reads a term its own descendant asserts. It costs almost nothing —
`overstated`'s extraction, its `parts` splitter and both vocabularies already exist, and the change
is which side each is applied to. It was measured before being built, which is trap 11, and the
measurement says not to build it:

> **170 parent rows with descendants; 14 denied term-mentions between them; 3 contradicted by a
> descendant, and all three are noise on reading.** §8.11.4's `/Name` against §8.11.4.3's is the
> one-short-key shape (a *configuration's* `/Name` and a *group's*), and §9.8.3's Table 122 and
> §9.8's Table 120 are both a table denied in part beside a table read in part.

Fourteen mentions is not a population, and the reason is structural rather than a small sample: a
row asserting a capability **enumerates** — that is what makes it a summary of its children — while
a row denying one **generalises**, and a generalisation names no term for a program to match.

**The proof is this round's own defect.** §9.8 said "[t]he dimensional metrics are read by nobody"
while §9.8.1 has said `/Ascent` and `/Descent` are read since the three-hundred-and-seventy-eighth
session. That is exactly the mirror shape, in the family the mirror sweep would have been aimed at
— and the mirror sweep **would not have printed it**, because "the dimensional metrics" is neither
a `/Key` nor a `Table NNN`. Mapping a category noun onto a table's entries is a judgement about
what a sentence means, which is the one thing every sweep in this file refuses to do.

So the eighteenth sweep is directional on purpose, and the entry that would have said *build the
mirror* says instead: **an understating parent is found by reading, and the reading list is the
blame ordering.** ADR 0481.

## Rows read in the six-hundred-and-fifty-second, the band at the top of the blame list

The ordering was re-derived rather than taken (616's rule): `git blame --line-porcelain
doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = ` line, ranked by where
its commit falls in `git log --reverse`. This base has **851 commits** and **240** `partial`-or-
`reported` rows with a blamed note. 648 left the list beginning at §10.4.2.4 and §10.4.2.5, and both
were still rank 1 and 2; §9.7.5 and §11.7 are 3 and 4, §9.8 is 5.

Five read — ranks 1, 2, 3, 5 and 13 — and **three were defects, all of them the fifth failure shape
inside one family**:

| row | shape | was | is |
|---|---|---|---|
| **§10.4.2.4** | 5 | `partial` because "Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` are read by nobody" | all four have been read since the four-hundred-and-twenty-sixth session — `content/ext_gstate.rs` sets `black_generation_stated` — and what is owed is that they are not **evaluated**. §8.4.5's row has said they are read since the five-hundred-and-sixty-fifth and §11.7.5.3's since the four-hundred-and-twenty-seventh, so the answer stood in two other rows while this one denied it through two rewrites of its own tail |
| **§9.8** | 5 | "[t]he dimensional metrics are read by nobody" | `/Ascent` and `/Descent` are read by `pdf_font::vertical_extent` and `variable_text::Metrics::read`, which §9.8.1's row has said since the three-hundred-and-seventy-eighth. The parent *denying* what its child asserts — the shape the section above measures and declines to build a sweep for |
| **§10.4.2** | 6, in prose | "`partial` for what **two** of the four conversions below owe" | **three**: §10.4.2.4 is `partial` too, and its debt is of a different kind — not a direction nobody takes but a pair of functions a file may state. `--bin counts` cannot see it, because the cardinal governs *conversions* rather than one of the ledger's own words for a row |

**The two kept rows each gained the evidence that kept them, and both were 620's third shape** —
the row is right and its evidence is not, for the eighth round running:

- **§10.4.2.5** cited `colour_paths.rs::a_cmyk_colour_is_the_same_however_it_is_drawn`, which asserts
  that `k`, `scn` and an image's samples agree with **one another**. That is trap 6's one-conversion
  rule and it is true whichever conversion is used, so it cannot see this row's subject. The test
  that can was already in the tree and cited by nothing:
  `colour.rs::the_conversion_into_ink_round_trips_through_the_classic_formula_and_not_the_cube`
  writes the clause's own formula out, pins §10.4.2.4-then-§10.4.2.5 exact on pure red and on
  §10.4.2.3's grey, and pins the ink cube as a different answer.
- **§9.7.5** cited one embedded-CMap test for a sentence whose other two thirds are the Identity
  pair and the 239 predefined files. §9.7.5.2's two are cited beside it now, and the **239 has a
  command**: it is `pdf_font::predefined::PREDEFINED`'s length, which `build.rs` writes from a
  directory walk, so `ls data/cmaps | wc -l` gives 242 less the three things beside the data that
  are not data (`LICENSE_ADOBE`, `SHA256SUMS`, `PROVENANCE.md`).

**Enumeration was run third and its yield was a bounded negative.** Walking `ext_gstate.rs`'s Table
57 arms against every ledger sentence denying one of that table's entries finds §10.4.2.4 and
nothing else live: §10.6.5's `/HT` and `/HTO` and §8.6.7's `/OP`, `/op` and `/OPM` are true denials,
and §11.5.1's `/AIS` and §9.6.4's `/FontBBox` are corrections quoting what they retired. Walking
`ColourSpace::to_cmyk`'s arms the same way corroborates §10.4.2.3 — a `Gray` space falls to the
`rgb_to_ink` arm, so that clause's own grey-to-CMYK arithmetic still has no caller. Neither found a
row the other two instruments could not; what enumeration bought was the *bound*, which is what
turns "and nothing else" from an impression into a statement.

**And `emit` filed an erratum two clauses from where it belongs, for the second round running.**
Issue #640 prints under `## 10.4.2.4` and strikes the *grey* of §10.4.2.2's `red = grey` formula and
of §10.4.2.3's `black = 1.0 − grey`, writing *gray* — the spelling both clauses already use
everywhere else. The annotations sit on the page whose last line is §10.4.2.4's heading, and `emit`
files by the clause the outline puts a page in. Recorded in §10.4.2.2's row; it changes nothing.

## Choosing the family by measurement rather than by eye, in the seven-hundred-and-fifth

The three rounds from the six-hundred-and-ninety-first each chose a *family* rather than a rank, and
each sharpened the criterion: take the family whose rows quote each other's figures (ADR 0551), and
then the one where **a claim is held in duplicate**, because a claim held in duplicate has somewhere
to disagree with itself (ADR 0560). That last criterion is a search rather than a judgement, and
running it as one costs a minute:

> For every parent whose subtree holds two or more `partial` rows, take each pair of those rows and
> count the five-word sequences their notes share, keeping only sequences that at most four rows in
> the whole ledger carry. Rank the families by the total.

The rare-sequence filter is the whole of it: without it every pair scores on the ledger's shared
vocabulary, and with it what surfaces is a *list* or a *figure* one round wrote into two rows. Its
first run put §12.8 at the head, with §12.8.3's subtree the reason — five rows all `partial`, sharing
a digest list and a sentence about Table 260's families — ahead of §12.5.6, §8.11.4, §12.5, §12.4.4
and §10.7. §11.3.7, §12.3.5 and §14.8.2.2 also score, and each is a family in which every row is
`partial`.

**What the ranking cannot show is how far the duplication goes**, which is the reason to read the
family rather than the score: §12.8.3's actual finding was a paragraph of 92 words standing
byte-for-byte identical in **fifteen** rows, `partial` and `reported` alike, ending in two counts
that had been stale for four rounds. The pairwise comparison saw a fraction of it, because the rows
it compares are only the `partial` ones.

**Not built as a sweep, deliberately** (ADR 0567 §7): its output is a ranking rather than a hit list,
no line of it is a defect, and it settles nothing a person does not then have to read. What it
replaces is choosing by eye, which is what the three rounds before it did.

## The ranking run a second time, in the seven-hundred-and-tenth, and two rules for reading it

The search above was run on a base one round later and gave a different head, which is what a
ranking is for. Two things about its **output** cost that round time and belong here rather than in
its ADR:

- **Strip the clause-level parents.** §12, §11, §8, §14, §7 and §10 sort above every real family
  and none of them is one: a subtree with ninety-six `partial` rows has 4560 pairs and scores on
  the tail of them. The first run did not say so only because §12.8 happened to beat them. A parent
  worth reading has a dot in its number.
- **The total ranks the family; the *pairs* choose the reading.** The head was §12.5, whose
  subtree carries more `partial` rows than a round can read properly, and what it was actually
  opened on is its three strongest pairs: §12.5.4 ~ §12.5.6.8, §12.5.2 ~ §12.5.5, and
  §12.5.3 ~ §12.5.6.4. Each is a *quotation* one round wrote into two rows, which is the shape the
  rare-sequence filter exists to surface.

**The pair to take is the one where the two rows do not merely quote the same sentence but disagree
about what it leaves standing.** §12.5.2 and §12.5.5 both quote Table 166 on `/CA` and `/ca` beside
a stored appearance; only one of them had noticed that Errata Collection 3 took `/BM` out of the
list they were both reasoning from. ADR 0579 has that reading and the two further defects it led to
— a paraphrase citing the wrong NOTE, and a `shall` in the same paragraph that no code read and no
report named.

## The ranking run a third time, in the seven-hundred-and-sixteenth, and a rule the first two could not see

The search was run again on a base one block later, and its *order* did not move: §12.5 heads it,
§12.8 is second, §12.7 third. What moved is the head family's score, and the reason is a property of
the instrument rather than of the ledger.

**A family the last round read scores higher for having been read.** Run over the ledger as it stood
before the seven-hundred-and-tenth session's commit and over the ledger now, with one instrument on
both sides, §12.5.2 ~ §12.5.5 goes from 17 shared rare sequences to **21** and §12.5's total from
221 to **225**. That pair is precisely what 710 opened the family on: it read the two rows against
each other and rewrote both, in one round's voice, with one round's vocabulary — which is what
reading a family *is*, and it leaves behind more shared rare sequences than it found. Every other
family's total is unchanged.

So the ranking is **self-reinforcing over one round**, and the third rule for reading it is:

> **Take the strongest pair the previous round named and did not read**, rather than the top of the
> family's list. A pair that has just been corrected is the one pair in the family whose two rows
> are known to agree.

In the seven-hundred-and-sixteenth that was §12.5.4 ~ §12.5.6.8, at 24 — the strongest pair below
any clause-level parent in the whole ledger bar §12.4.4 ~ §12.4.4.1 and §10.7.4 ~ §10.7.5 — which
ADR 0579 §1 named and left.

**And the pair was clean, which is not a wasted reading.** Both rows quote §12.5.4's sentence about
the four subtypes whose `/BS` supplies width and dash alone, and enumerating the `/BS`-bearing tables
confirms the division: Tables 176, 177 and 191 say "the annotation's border" and Tables 178, 180,
181 and 185 say the line, the rectangle or ellipse, the line again and the paths — so `Border::simulated`
is asked by exactly the three subtypes whose entry is a border, and the fifth table the sentence does
not name is Table 181's, excluded correctly for a reason neither row states. **What the pair bought
was the pages**: it put the round on Tables 179 and 180, and `spec-errata emit` over those pages is
where all three of its findings are (ADR 0593). A pair that survives its reading has still chosen
where to look.

### What that reading found, and it is the first blindness rather than the third

- **Issue #515 is a `Caret` with no `StrikeOut`** — the first of the three ways `check` cannot see an
  erratum, and the first met since the seven-hundred-and-tenth named the third. It adds "filled with
  the annotation's interior colour, if any" to Table 179's `RClosedArrow` row, which `Ending::filled`
  had already derived from "in the reverse direction from" `ClosedArrow`. **The reading was right and
  the arithmetic around it was wrong in four places at once**: the function's doc comment, the test's
  doc comment, the test's *name*, and §12.5.6.6's ledger row all said *four* over five arms. No pixel
  moves.
- **The fill was decided twice, and trap 13 is what found it.** Calibrating the renamed test by taking
  `RClosedArrow` out of `filled` — the test passed. `draw_ending` asks `filled` for three of the five
  shapes it names and the arrowhead arm asked its own `closed && interior != Colour::None` three
  matches below. The two expressions agree and always have; what the duplication cost is the reach of
  a correction, and the two shapes outside `filled`'s reach are the two the erratum is about. **701's
  shape inside one function**, and the place the duplicate had not yet disagreed was the place a
  correction would have had to land.
- **Issue #513 explains a sweep hit that had been standing at the head of `--bin quotations`' output.**
  It is an EDITOR NOTE saying the ISO PDF's own row height obscures the end of `OpenArrow`'s sentence;
  `doc/md/` ends that cell at "an open" and begins the next with the word that finishes it, and
  ADR 0192 quotes the whole sentence correctly. **A hit a sweep prints every round is not a hit nobody
  has explained** — the sweep's own instruction is *suspect the conversion*, and here the specification
  says so itself. The document now carries the answer beside the quotation.


## The third rule's first use, in the seven-hundred-and-twentieth, and it leaves the family

The search was run again on a base one round later. **Its family order did not move** — §12.5, then
§12.8, then §12.7 — and the third rule sent the round out of all three, which is what that rule is
for.

ADR 0593 §1 named two pairs stronger than the one it read and left both: **§12.4.4 ~ §12.4.4.1**
and §10.7.4 ~ §10.7.5. The first is the strongest pair below any clause-level parent in the whole
ledger, and it is the one this round took. **A rule that only reordered the pairs inside the head
family could never escape it**, which is the difference between the third rule and a tie-break:
self-reinforcement is a property of the family, so the cure has to be allowed to leave.

**And the pair was not clean.** Both rows carry the same two sentences about Table 164's twelve
transition styles — one saying seven are shaped and **four** of the other five reported, `R` being
the cut the table defines, and one, three lines later, saying the clause is "`partial` for the five
styles". The two contradict each other, in both rows, and the history is the shape 710 named:
§12.4's parent row was corrected to four in the three-hundred-and-eighty-eighth session and the
*middle* sentence of each of the pair in the six-hundred-and-sixty-third, and both times the
closing tally below was left standing. **A correction that reaches the sentence stating a mechanism and not the
sentence counting it is one failure; the same string surviving a round whose whole subject was that
string is another.**

What the pair then bought was the code. `viewer_core::transition::note` asked a `&Style` while
`frame` asked a `&Transition` — style **and** `/Di` — so a `Wipe`, `Cover`, `Uncover` or `Push` in a
direction outside the four quarter turns shaped no frame and said nothing: a cut in silence, which
is the one outcome the function exists to prevent. Two expressions answering one question, which is
701's shape and 716's, for the third round running; and a *test's* own doc comment had asserted the
report existed since the three-hundred-and-ninety-third. ADR 0600, with the property test that holds
`note` and `frame` against each other over every style crossed with every direction.

**`emit` found nothing against the clause and a contradiction two pages above it.** Against §12.4.4
there is no erratum this tree had not recorded, which is the answer the round wanted. Against Table
161, two pages earlier, there are two — Issue #432 rewriting *AA to ZZ* as *AA to AZ*, and Issue
#593 inserting *AAA to ZZZ for the next 26* after it — **both `Review`/`Accepted`, and mutually
exclusive.** ADR 0601 has the geometry that places them and the rule that comes out of it: **an
erratum is evidence about the standard, in the way another renderer is evidence about our reading**,
and where two disagree the published clause and its own arithmetic decide.

## The third rule's second use, in the seven-hundred-and-twenty-fifth, and a `shall` that was paid

The search was run again on a base one round later. **Its family order did not move** — §12.5, then
§12.8, then §12.7 — and stripping the clause-level parents leaves three pairs above every other in
the ledger: §12.4.4 ~ §12.4.4.1, §12.8 ~ §12.8.3 and **§10.7.4 ~ §10.7.5**. ADR 0600 §1 named two
and read one, so the third rule gives the pair it left, and the family it lands in is one no round
of this method had opened.

**The pair scores on each row quoting the *other's* clause**, which is the strongest form of what the
rare-sequence filter looks for: §10.7.4's "[z]ero-width strokes may be done in an
implementation-defined manner that may include fewer pixels than the rule implies" stands in both
rows, and §10.7.5's NOTE — "[t]his is the thinnest line that can be rendered at device resolution" —
stands in both. And they disagree about what those sentences leave standing, which is 0579's rule
for choosing among such pairs.

**§10.7.5's row said a `shall` was unpaid and it had been paid.** The row narrates the
four-hundred-and-thirty-second session's measurement of `tiny-skia`'s hairline — taken for every
width up to *and including* one device pixel, laying one pixel down per step along the line's longer
device axis, so a 45° rule one device pixel wide carried 141.42 of its own 200 against the fill of
the same outline's 177.44 — and ended it "Not paid … `doc/todo/11`". The four-hundred-and-fifty-fifth
paid it: `render_cpu::at_or_under_the_quantum` is `<=`, `sub_pixel_coverage.rs`'s turned ladder
carries the `1.0` rung and says it "is the rung that used to fail", §10.7.4's own row records ADR
0285 in full, and **`doc/todo/11` — the pointer that sentence ends with — heads that item "closed
(ADR 0285)"**. Worse than stale: **the two reasons the row gave for not paying are the two ADR 0285
decided the other way**, so the row argued from correct facts to the opposite conclusion. ADR 0610.

**No sweep here could have printed it, and that is worth knowing rather than fixing.** `--bin
blockers` judges a blocker that names a *clause*; this one named a conclusion. `--bin owed` looks for
a debt naming a thing the tree lacks; the thing this debt named — the hairline — is a thing the tree
has, in a dependency. The defect is a conclusion, and nothing in this tree ranks conclusions. The
ranking does not rank them either; what it does is put the two rows side by side.

### The ninth sweep's keyless count is a hiding place as well as a noise filter

Reading the same family's errata turned up the *same* wrong table number in two more places, and
neither is printable by the sweep built for it. `doc/errata-read.md` says "Table 58's `/FL`" where
the graphics state parameter dictionary is Table 57 — the confusion §10.7.5's own row records having
carried until the three-hundred-and-eighty-ninth — and **Table 58 is the path construction
operators, which state no entries**, so `--bin tables` counts the citation among its keyless ones
instead of among its absences. Three rows down the same document writes "moved here from Table 58"
with no key beside the number, which is not an attribution at all and is below the sweep's
discriminator by design.

Calibrated per trap 13, one instrument over three states of the cell: with `Table 58` the sweep
prints nothing; with `Table 166` — a table that does state entries — it prints the citation **and
names Table 57 as the table stating the key**; with `Table 57` it agrees. So the blindness is
exactly one class wide, and the rule that comes out of it is about writing rather than about
building: **write a claim in the form its sweep reads** — a table number with its key beside it, a
pointer as a path, a debt with its identifier. `oracle.rs` had the third form of the same failure in
the same family, a pointer written as the prose "the handover's list of departures" for a list the
handover has not held since it became an index. ADR 0611, which also says why widening `--bin
tables` to print keyless attributions is declined.

## The pairwise ranking runs out, and its successor is measured — the seven-hundred-and-thirty-fourth

The seven-hundred-and-thirtieth reports that ADR 0593's third rule had nothing left above rank 4
and fell through to a tie broken by an older rule. **A method that has exhausted its head will
start re-reading old ground**, so this section is the successor and the measurement behind it.
ADRs 0627 and 0628.

### What was measured first, and it says not to build another ranking over notes

Nine rounds of findings are recorded, from the six-hundred-and-ninety-first to the
seven-hundred-and-thirtieth. Each names the rows it found a defect in, so there is a labelled set:
**21 defect rows** that were `partial` or `reported` at their own round's base. Eleven candidate
signals were scored over every live row at each of the nine bases — each round's ledger taken at
the commit *before* its own — and each defect row's position in the ranking recorded as a
percentile, where 50 is what choosing at random gives:

| signal | mean percentile |
|---|---|
| distinct §-references in the note | 38.6 |
| `Table NNN` citations | 40.0 |
| cardinals that quantify | 40.8 |
| age of the note's last rewrite (the blame ordering) | 41.7 |
| note length in words | 42.3 |
| cardinals × revisions | 42.4 |
| **revisions — how many commits have rewritten the note** | 46.0 |
| revisions per commit since the row was born | 46.2 |
| cardinals per hundred words | 46.8 |
| `/Key` tokens | 48.1 |
| **the pairwise rare-sequence score, the incumbent** | 48.3 |

**Nothing here is worth a rule.** The best of the eleven moves the needle by a ninth on 21 points,
and the *incumbent* scores 48.3 — at row level the pairwise ranking is indistinguishable from
choosing at random, which is not a criticism of it: it ranks **families**, and it says so. What the
table settles is that no cheap property of a note ranks the rows inside one. **Revisions were the
hypothesis worth having and they lost**: the recurring shape is a correction that reached one
sentence and not another — 730's list audited three times, 720's tally left standing twice — so a
count of rewrites looked like it should predict, and it predicts less than the note's length does.
The defects are *conclusions*, and 725 already recorded that nothing in this tree ranks conclusions.

So the successor is not a ranking over notes.

### The rule

> **Rank each live ledger row by the errata annotations that fall on it whose issue number this
> tree names nowhere. Reassemble the issue from every clause `emit` files it under, and read the
> issue whole.**

Eight of the nine rounds found something through `spec-errata emit`, and the seven-hundred-and-sixteenth
put the reason plainly: a pair that survives its reading has still chosen where to look. This rule
chooses the same thing directly, and it has three properties the pairwise ranking does not:

- **Its population is finite and known.** The collection's issue numbers are a closed set, so
  *how much is left* is a question with an answer instead of a rank.
- **It decays on use.** Reading an erratum records its number, which takes it out of the
  population. The pairwise ranking does the opposite: the seven-hundred-and-sixteenth measured a
  family scoring **higher** for having been read, because two rows rewritten in one round's voice
  share more rare vocabulary than they did before.
- **Its hits are defect-shaped.** An erratum this tree names nowhere is a specific sentence of the
  standard nobody here has read, not a suspicion about a note.

**Two limits, stated rather than discovered later.** *Named* is not *read properly*: the
seven-hundred-and-thirtieth's finding was about Issue #619, which this tree names and whose four
carets it had counted as two — so the population is a lower bound on the debt and the ranking is
blind to a misread erratum. And it is an errata ranking rather than a defect ranking: three of the
nine rounds' headline findings had no erratum in them at all.

### The measurement that says it has a head

Reconstructed at each of the nine rounds' own base commits — `git grep` for the issue numbers the
tree named then, against the same `emit` output, since the PDF does not change:

- The unread population falls **monotonically**: 103, 100, 97, 94, 91, 90, 89, 86, 86 issues
  landing on a live row. About two a round, which is what nine rounds of reading errata *is*.
- Of the eleven errata those rounds recorded, **eight were in the population** at their round's
  base. The other three were errata the tree already named and had misread, which is the limit
  above showing itself.
- **The head did not move and nobody was on it.** §12.8.1, §12.5.2, §12.7.5.5 and §9.8.1 are in the
  top six at every one of the nine bases; the rows the nine rounds actually landed on ranked 1, 4,
  8, 17, 17, 22, 32, 39 and 50. §7.6.6 is the one row that left the head, after the
  six-hundred-and-ninety-first read two of its issues — the decay working.

At this base the whole population is **241 issue numbers named nowhere** of the **356** carrying an
annotation, and **63** of those change the text and land on a `partial`, `reported`, `silent` or
`unreviewed` row, over **41** rows. **Both figures are over-estimates and the step-2 note below says
by how much**: they were taken with the prefixed grep alone, which cannot see the bare numbers
`doc/errata-read.md`'s own tables record a verdict under.

### The recipe, as commands

```sh
# 1. the annotations, keyed to page and clause. Not committable: ADR 0187.
cargo run --release -p spec-errata -- emit doc/ISO_32000-2_sponsored_EC3.pdf > /tmp/emit.md
# 2. every issue number this tree names. Two greps, because neither is right alone — see below.
grep -rhoIE 'Issues? #[0-9]+(,? (and )?#[0-9]+)*' crates doc tools fuzz \
  --exclude-dir=md --exclude-dir=pdf.js --exclude-dir=corpora \
  --exclude-dir=arlington-pdf-model --exclude-dir=safedocs | grep -oE '#[0-9]+' | sort -u
sed 's/&#[0-9]*;//g' doc/errata-read.md | grep -oE '#[0-9]+' | sort -u   # the record's own column
# 3. the ranking: for each `## <clause>` heading in the emit, attribute its annotations to the
#    nearest ledger row at or above that clause number, keep the rows whose status is live, drop
#    the issues step 2 found and the ones carrying neither a StrikeOut nor a Caret, and rank by
#    annotations. Then read one issue *whole*, across every heading it appears under.
# 4. rank a second time over EVERY row, whatever its status, and take the head of the two —
#    preferring the settled row where they tie. Say which ranking the row came from. See below.
```

**Step 4's ordering is an argument rather than a convention, and it is the one the fifth use ran.**
The two rankings answer different questions and the second is the sharper of the two:

- **A live row's count ranks a debt the ledger already declares.** `partial`, `reported` and
  `silent` all say something is owed; an erratum landing there refines a known gap, and the row
  will be re-read anyway by whichever round pays it off.
- **A settled row's count ranks a *claim*.** `implemented` says every normative requirement in the
  clause is executed, and `inapplicable`, `out-of-scope` and `writer-side` say no requirement
  reaches this program at all. `CLAUDE.md` says both kinds of claim decay — its own §10.5 entry is
  the standing example of an *inapplicable* that was wrong — and an unread erratum on such a row is
  the only signal this project has that one has. The falsest row the ledger can hold is an
  `implemented` over a requirement nobody read.
- **So one ranking over every row, and the settled row wins a tie.** Not two lists read in order:
  ranking the settled ones *above* the live ones outright would ignore the count, and the count is
  the whole instrument. What the tie-break encodes is that at equal evidence the row asserting more
  is worth more.
- **The count means less on a settled row than on a live one, and that is a reason to read the head
  rather than to reweight it.** #307 was one caret per clause on two `implemented` rows and the
  fourth use met it by walking into it. A weighting that tried to price this would be a second
  ranking over notes, which the measurement above says not to build.

**And the record is what keeps step 2 honest**, which is the fourth use's other finding: an erratum
read to a verdict is written into `doc/errata-read.md`, where step 2's second grep reads it. **An
issue number written anywhere else must carry the `Issue #` prefix** — the form `**#214**` is
invisible to *both* greps, and a bare-number search over the tree is not the repair, because
`doc/HAYRO_ISSUES.md` lists another project's issues under the same numbers. An erratum read only
far enough to break a tie is left in the population on purpose; saying so is cheaper than a grep
that cannot tell the two apart.

**Step 2 is two greps and neither is right alone, which the second use found by running it.** The
prefixed search is not decoration and it is not sufficient:

- **Why the prefix.** A numeric character reference is a `#` and digits: `&#124;` is how a Markdown
  table cell escapes a pipe, and there are a handful under `crates/`, `doc/` and `tools/` — ADR 0484
  first, and now the documents describing this trap. A search for the bare number `124` finds them
  and answers *recorded*. **Issue #124 is one of the two this rule's first use found unread**, on a
  page the seven-hundred-and-tenth had opened.
- **Why one grep is not enough.** `doc/errata-read.md` — the tree's own record of every erratum it
  has read — writes the issue number **bare, in a table column**, so the prefixed grep answers
  *nowhere* for issues that carry a verdict there. The shortfall is large: at the seven-hundred-and-
  thirty-ninth's base the prefixed grep found 113 of the 351 issues carrying an annotation while that
  one file records 159. So the population 734 measured is an over-estimate, and the head it named
  moves under the repair — §12.7.5.5 loses two of its four and §12.8.1 becomes the head.
- **Why a bare-number grep over the tree is not the repair either.** `doc/HAYRO_ISSUES.md` and
  `doc/HAYRO_ISSUES_FOR_QUORRA.md` list *another project's* GitHub issues and name `#54`, `#55`,
  `#680` and `#681` among others, all of which are live errata numbers. A number-only search answers
  "recorded" from a document about a different tracker. Two greps, unioned, and the numeric character
  references removed from the second.

**Not built as a sweep, and the reason is the tool's rather than the rule's**: `emit`'s output is
derived from documents this project may not redistribute, so a program that consumed it would take
the same argument `--bin quoted` and `--bin unpriced` take. That is the shape it would have if a
round decides it is worth building; what it would buy over the recipe is the attribution in step 3,
which is twenty lines of arithmetic and the only part a person can get wrong.

## The rule's second use, in the seven-hundred-and-thirty-ninth, and it corrected its own step 2

Run as written, the recipe put §12.7.5.5 at the head with §12.8.1 one behind it. Run with step 2
repaired — `doc/errata-read.md`'s own bare column unioned in — **§12.8.1 is the head and had never
moved off it**, which is what 734's reconstruction across nine bases predicted. Both of §12.7.5.5's
top two issues turned out to be recorded already, in that file, in a table row.

**The head paid.** §12.8.1 carries nine annotations under five issue numbers, and reading them
together took the round to Table 255 — where the row's own claim, *Table 255 entire*, was standing
over five entries with no reader. Four of the five say nothing to a reader and are declined in the
row with the entries' own words; the fifth, `/V`, states the one thing in that table addressed to
whoever validates: "[t]he value is 1 if the Reference dictionary shall be considered critical to the
validation of the signature". A file writing it is naming the part of its own validation that this
program does not perform, and nothing here was reading the entry. ADR 0637; `doc/errata-read.md` has
the five errata with their rectangles.

**Two things about the rule itself, both from running it:**

- **The step-2 shortfall above is the finding to carry forward**, because it is not a detail of one
  round: an issue *recorded* is recorded in `doc/errata-read.md`, and that is exactly the file the
  prefixed grep is blind to. A rule whose population is measured by a search that cannot see its own
  record will keep re-offering read ground, which is the failure it was built to replace.
- **The two blindnesses this round met are the ones already on the list**, and meeting them is
  evidence they are the right list: #117's strike is one word, under `check`'s four-word floor, on a
  quotation in `ledger.toml` — the population that has a gate; and #219 had to be reassembled across
  two clause headings, seven annotations on two tables in two clauses.

## The rule's third use, in the seven-hundred-and-forty-sixth, and it needed a tie-break

**The head moved for the first time, and it moved because the two rounds before it were read.**
§12.8.1 and §12.5.2 are gone from the ranking — every issue on them now carries a verdict in
`doc/errata-read.md` — which is the decay the rule was chosen for, working. §12.7.5.5 and §9.8.1,
the other two rows 734's reconstruction found in the top six at all nine bases, are at ranks 8 and 7.

**Three rows tie at the head with seven annotations apiece**, which the rule as written does not
settle. §7.7.4's seven are #214, the global *name string* → *string* rename, five times over on
Table 31's rows, and #672, which marks `/IDS` and `/URLS` *deprecated in PDF 2.0*; §14.8.5.3's are
#357, *version* → *level* on three of Table 384's CSS owners, and #224, which inserts *structure*
into a NOTE; §12.10.2's change a requirement level and an entry's meaning. **So the tie-break is
what the annotations do rather than how many there are**, and it is worth stating because a count of
carets weighs a five-times-repeated editorial substitution as five: read the row whose errata strike
a *cell* — a requirement level, a type, a description — ahead of the row whose errata substitute a
word in prose. Both losing rows were read far enough to say that, which is the cheap part; #672's
two deprecations are worth a later round's attention on their own, since `/IDS` and `/URLS` are Web
Capture's and this tree reads neither.

**The head paid, and it paid by making an implementation possible rather than by naming a debt.**
§12.10.2's row claimed *Table 269 entire* and §12.10's blocker said turning a page coordinate into a
latitude needs the EPSG registry. Issue #534 strikes three words of `/PCSM`'s description and writes
its whole shape in their place — *a 4x4 affine transformation matrix in row order*, applied to the
position as *[ x y z 1 ]* — and twelve numbers for a 4×4 matrix is §8.3.4's own elision one dimension
up, so the leg from the object's coordinates to the projected system is a matrix multiplication with
nothing outside the standard in it. The registry owns the *second* leg. ADR 0653;
`doc/errata-read.md` has the four errata with their rectangles.

**Three things about the rule itself, from running it:**

- **The instrument was sound this time**, which is worth recording because the two uses before it
  each found something wrong with the instrument. Step 2 as 739 repaired it needs no further
  correction: 133 of the 302 issue numbers carrying a strike or a caret are named nowhere, and
  spot-checking the head's four against both greps and `doc/HAYRO_ISSUES.md` turned up no collision.
- **A tie is the normal case at the head and the rule should have expected one.** The ranking's
  units are annotations, and an issue lands between one and seven of them on a row, so the top of
  the list is a plateau rather than a peak. The tie-break above is the rule's, now.
- **The blindness this round met is the second on the list, again**: #534's strike is
  `projected coordinate system.` — three words, under `check`'s four-word floor — and the doc
  comment on `Geospatial::projected_matrix` quoted the sentence it sits in. Two consecutive uses of
  this rule have found a quotation resting on struck text that `check` cannot see, which says the
  floor is where the errata debt now is.

## The rule's fourth use, in the seven-hundred-and-fiftieth, and two more things about the instrument

**The head is the plateau the third use left standing.** §12.10.2 is off the ranking — 746 read it —
and §7.7.4 and §14.8.5.3 are back at the top with seven annotations apiece, which is 746's tie-break
run a second time between the two rows it had already ranked below the winner. §7.7.4 takes it on the
same rule: Issue #672 appends *; deprecated in PDF 2.0* to two of Table 32's cells, and §14.8.5.3's
four carets swap *version* for *level* in the name of a referenced CSS specification. ADR 0660;
`doc/errata-read.md` has the five errata with their rectangles.

**The head paid twice, and neither hit was on the head's own row.** #672 turns two of §7.7.4's six
unread name trees from *owed to Web Capture* into *deprecated in PDF 2.0*, and its third caret does
the same to §7.7.2's `/SpiderInfo`, so one erratum deprecates the whole of the catalogue's Web
Capture surface. #214 — the global *name string* → *string* rename — took the round to §7.9.6, where
it strikes *lexically* and *in lexical order*, and where `pdf_syntax::tree` was citing that clause
for the phrase "by unsigned character code", which ISO 32000-2 prints nowhere at all.

**Two things about the rule itself, and both are about a record rather than about a ranking:**

- **A round that reads an issue without recording it in `doc/errata-read.md` leaves it at the head.**
  746 read #214 and #672 far enough to break its tie and wrote both into its ADR and into this file
  as bolded bare numbers — `**#214**` — which is a form *neither* of step 2's greps can see: the
  first wants the `Issue #` prefix and the second reads `doc/errata-read.md` alone. This is 739's
  finding in a new costume, and the repair is a rule about writing rather than a third grep, because
  the collision families that make a bare-number search useless are still there. **An erratum read to
  a verdict is recorded in `doc/errata-read.md`; an erratum read only far enough to rank it is left
  in the population on purpose, and saying so is cheaper than a grep that cannot tell the two apart.**
- **The ranking drops `implemented` rows, and the true head is inside them.** Step 3 keeps the rows
  whose status is live, which is what makes this a ranking of what is *owed* — but an erratum's whole
  point is that it can add a requirement to a clause this tree calls complete, and a row claiming
  `implemented` over an unread requirement is the falsest row the ledger can hold. Measured at this
  base, with `implemented` admitted: §9.6.4 carries **11** unread annotations under four issues and
  §7.4.1 carries **8**, both above the live head's seven, and §7.9.2.4, §7.5.4, §7.6.4.4.3, §7.10.5.3,
  §12.5.6.1 and §12.10.4 all appear in the top twenty. This round met the shape rather than measuring
  it from outside: #307 adds *Keys shall not be the null object.* to §7.9.6's Table 36 and §7.9.7's
  Table 37, both `implemented`, and the ranking could not see either. **A round running this rule
  reads the ranking twice — live rows, then all rows — and says which one it took its row from.**

## The rule's fifth use, in the seven-hundred-and-fifty-fifth, and the fourth step's first run

**The fourth step was written by the round that could not run it, and running it moved the head.**
Over live rows the head is §14.8.5.3 with seven annotations — the plateau the fourth use left
standing, §7.7.4 having gone off the ranking because 750 read it, which is the decay working for the
third use running. Over **every** row, §D.3 carries **fifteen** under three issues, more than twice
that, and is `implemented`. §9.6.4 and §7.4.1, the two rows 750 measured from outside at 11 and 8,
are second and third and are still unread. ADR 0671; `doc/errata-read.md` has the three errata with
their rectangles.

**The head paid, and what it paid with was not a requirement.** All three errata correct Table D.3's
own *presentation* — a glyph printed in the `Character` column, the alias of a code the annex marks
undefined, one `Unicode` cell of another such code — and not one of them moves a byte this program
decodes. What reading them was worth is what they made the round look at: §D.3 was `implemented` on
a **round trip**, which searches the same array it indexes and therefore cannot see a wrong
transcription at all, and Issue #461 names the exact mistake that gate would have missed. It strikes
the `Š` the standard printed in code 0x8a's `Character` column, where the `Unicode` column says
U+2212 and `Š` is 0x97's — so a transcription taken from the column the erratum corrects would decode
a minus sign as a capital S with caron, and every one of the module's ten tests stays green when that
is planted. All 256 rows are compared against `doc/md/` now, in both directions.

**Three things about the rule itself, from running it:**

- **The second ranking is where the rule now earns its keep, and one number says why.** Of the 126
  issue numbers this tree names nowhere, the live ranking's whole head is seven annotations; the
  head of the full ranking is fifteen. Four uses of this rule read live rows because the recipe said
  to, and the largest single accumulation of unread errata in the collection was sitting on a row
  that claims to owe nothing.
- **A settled row's defect is a *gate* rather than a behaviour, and that is the shape to expect.** A
  live row's erratum tends to name a requirement nobody implemented. A settled row's erratum tends
  to find the requirement implemented and the *evidence* for it weaker than the row says — here, a
  round trip standing in for a transcription check, and a note citing Table D.2 for a table that is
  D.3. Neither is something a corpus can see.
- **Nothing had to be reassembled this time, and that is not the rule relaxing.** All fifteen
  annotations fall under one `emit` heading because Annex D.3 is one table on six pages. The four
  uses before this each had an issue split across clause headings, and the check costs one grep.

## The rule's sixth use, in the seven-hundred-and-sixtieth, and the live head has now stood for four

**The fourth step out-ranked the live list for the second time running, and by more.** Over live rows
the head is §14.8.5.3 with seven annotations — the same plateau the third, fourth and fifth uses each
left standing, because not one of the three took its row from that list. Over **every** row §9.6.4
carries **eleven** under four issues and is `implemented`; §7.4.1 with eight is second. Both figures
reproduce what 750 measured from outside before the step existed, which is the calibration that says
the arithmetic is right before it is trusted. ADR 0681; `doc/errata-read.md` has the four errata with
their rectangles.

**The head paid with a *denial* rather than with a gate.** Issue #111 inserts a NOTE saying a Type 3
glyph description "can use any PDF operator from any operator category … subject to additional
restrictions described in this clause" — and §9.6.4's row had denied exactly one of those categories
since the tenth session, saying an inline image in a glyph description "draws nothing yet and
reports, which is §8.9.7's gap rather than this one's". §8.9.7 has been `implemented` since the
**eleventh** (ADR 0019). Measured, it draws, it is placed by the description's own matrix, and
nothing is reported; all three claims were false for seven hundred and fifty rounds.

**Three things about the rule itself, from running it:**

- **The live ranking's head has not moved in four uses, and that is the fourth step working rather
  than the decay failing.** §14.8.5.3 has been the live head since 746 because 750 took §7.7.4, 755
  took §D.3 and this round took §9.6.4 — the full ranking out-ranks the live one every time it is
  run. A round that wants the live head takes it deliberately; the rule will not hand it over.
- **The shape 755 predicted held for a second row, with a different mechanism.** There a settled
  row's evidence was a round trip that could not fail. Here it was a *sentence* — a row asserting a
  gap in another clause family, which no sweep in this project is placed to print: `--bin overstated`
  reads the inverse relation, `--bin blockers` reads a stated blocker, and `spec-errata check` is
  blind to the erratum that leads to the clause because the strike is one word.
- **A row's denial of a capability decays exactly as its claim does, and nothing measures it.** The
  ledger's sweeps compare a row against the *code*; this sentence was wrong about a **sibling row's
  status**, and the only reason it was ever read is that an erratum put a round on the clause. Worth
  a sweep if a second instance turns up: every `partial`-shaped sentence inside a settled row's note
  that names another clause as the reason.

**And an erratum's *added* text cannot be a rustdoc blockquote.** `cargo test -p conformance`'s
`every_quotation_is_the_standards_own_words` asks `doc/md/` for every blockquote under `crates/`, and
an inserted sentence is in no clause of that conversion, so it fails with *§9.6.4 does not contain …
as written* — correctly, since the alternative is a gate that cannot tell a paraphrase from an
amendment. `measurement.rs`'s convention for Issue #534 is the one to follow: an erratum's
replacement goes in *italics*, naming the issue, never between `> `. This file and
`doc/errata-read.md` had the rule written down for *struck* text only.

## The rule's seventh use, in the seven-hundred-and-sixty-fifth, and the head a *mention* took off the list

**The full ranking out-ranked the live one for the third time running.** Over live rows the head is
§7.6.4.1 and §7.6.6 with six annotations apiece; over **every** row it is §7.4.1 with eight under two
issues, `implemented` — the figure 750 measured from outside and 760 named as second, reproduced
before it was trusted, which is 755's calibration practice for the third round in a row. Both issues
fall under one `emit` heading. ADR 0691; `doc/errata-read.md` has both with their rectangles.

**The head paid with a sentence that changes who a clause is addressed to.** Issue #216 strikes
*files* from §7.4.1's "PDF files support a standard set of filters that fall into two main
categories" and writes *processors shall* in its place, so Table 6 stops describing what documents
contain and becomes a closed set this program owes. It is met — five byte-to-byte filters, `Crypt` a
pass-through, four image codecs handed to the image pipeline — and nothing asserted it: every filter
had a test of its *output* and none asked whether the table was covered, so a name dropped from
`decode_reported` or from `is_image_codec` became `Unsupported`, which is what a name from no table
gets, with the rest of the crate green. Issue #527 corrects the clause's EXAMPLE 3 — a base-85 stream
printed without its `~>` marker, and `/Length 447` becoming 449, which is exactly those two bytes.

**Three things about the rule itself, and the first is the biggest thing this rule has learned about
its own record.**

- **A mention is not a use, and step 2 cannot tell them apart.** 760 recorded that an early draft had
  written two issue numbers in full and taken §14.8.5.3 off the ranking without a verdict — and the
  sentence recording that fix writes both numbers with the `Issue #` prefix, inside backticks, in
  order to say they should not be written that way. So both left the population anyway, and the live
  head this rule had carried for four consecutive uses vanished from the live ranking. Measured:
  restoring the two gives back 760's own figures exactly, 120 named nowhere and §14.8.5.3 at the live
  head with seven; without them the live head is six. **The repair is a rule about writing**, because
  a bare-number search is already ruled out by `doc/HAYRO_ISSUES.md` and excluding `doc/history/`
  would silence true records: *a sentence about the form of an issue number must not contain one* —
  write "with the `Issue #` prefix" and say how many, never which. **This is the eighth blindness on
  the list and the second that is the instrument's rather than an erratum's.**
- **A settled row's erratum found the evidence weaker than the row for a third time, by a third
  mechanism.** 755 found a round trip that could not fail; 760 found a sentence about a sibling row;
  this found a *set* with no closure check. The three have nothing in common except the status, which
  is the fourth step's whole argument working.
- **The reading found a wrong number no sweep can print, in the parent row.** §7.4's note called Table
  6's ten "[f]our … stream filters implemented here, one … a pass-through … and four … image codecs",
  which is nine, while the same note has said since ADR 0587 that all *five* of Table 6's byte-to-byte
  filters can be windowed. `--bin counts` is right not to see it: a cardinal is a claim about a family
  there only where it governs one of the ledger's own words for a row.


## The rule's eighth use, in the seven-hundred-and-seventy-first, and the head was a tie the tie-break settled

**The two rankings tied at the head for the first time, and step 4's last clause is what decided
it.** Over live rows the head is §7.6.4.1 and §7.6.6 with six annotations apiece — unmoved from the
seventh use, because no round has taken either. Over **every** row the head is those two and
§12.6.4.17 at six, `out-of-scope`, and "preferring the settled row where they tie" is the whole of
why a round read a clause-13 row. §7.4.1 has left both rankings, the seventh use having read it.
ADR 0708; `doc/errata-read.md` has the five errata with their rectangles.

**The settled head paid nothing and the row below it paid.** §12.6.4.16's two issues — a Table 220
row naming the wrong action type for the type it defines, and the action widened to a RichMedia
annotation — are inside `CLAUDE.md`'s multimedia exclusion from end to end, and `action.rs` refuses
the keyword whatever the target is. That is a legitimate outcome for this rule and worth saying
plainly: the population decays by two, the row's claim is confirmed rather than moved, and the round
carries on down the ranking. The next settled row, §7.9.2.4 at five and `implemented`, is where the
work was — the erratum there makes a *literal* string one of the two forms every byte string may be
written in, and §7.3.4.2's end-of-line rule turned out to be unimplemented under an `implemented`
row whose note enumerated everything else the reader took.

**Three things about the rule itself, from running it:**

- **A tie at the head is not a coin toss, and the argument for the tie-break survives the round that
  cashed it.** The settled row won and paid nothing; two live rows would each have paid something,
  because a `partial` row's erratum names a debt by construction. What the tie-break buys is not a
  better expected value on one round — it is that the *only* signal this project has for a decayed
  settled claim gets read at all, and a rule that broke ties the other way would never read one. So
  the round did both: the head to a verdict, then the ranking downward until a row paid. **That is
  the practice to keep**, and it is cheaper than it sounds: reading a settled head to a verdict is
  minutes when the answer is an exclusion the standard's own words fall inside.
- **A settled row's evidence was weaker than its claim for a fourth time, by a fourth mechanism.**
  755 found a round trip that could not fail, 760 a sentence about a sibling row's status, 765 a set
  with no closure check; this found a row claiming **two written forms** with a test of one. §7.9.2.4
  is `implemented` and its whole test list was a hexadecimal-string test, and the erratum is precisely
  the sentence that makes the literal form the other half of the claim. The four share nothing but the
  status.
- **The mis-filing the fourth step inherits is the outline's, and it decided this round's head.**
  `emit` attributes an annotation by the outline section for its *page*, so a page holding two
  subclause openings files everything under the later one — all six of §12.6.4.16's annotations print
  under §12.6.4.17. It cost nothing here, both rows being settled under one exclusion, and it is the
  four-hundred-and-twenty-ninth's finding reached from the ranking's side. **A round whose head is a
  row it did not expect reads the annotation text before the heading**: the strike says which sentence
  it is over, and the heading only says which page it was on.

## The rule's ninth use, in the seven-hundred-and-seventy-fourth, and the mis-filing reached a recorded verdict

**The two rankings agreed at the head for the first time, on two live rows, and the third use's
tie-break decided between them.** Over live rows the head is §7.6.4.1 and §7.6.6 with six
annotations apiece — unmoved since the seventh use — and over **every** row it is the same two:
§12.6.4.17 left when the eighth use read it, and no settled row reaches six, which is four
consecutive uses of the fourth step finishing their work rather than a new regime. With no settled
row to prefer, the cell-over-prose tie-break leads with §7.6.6, whose issue rewrites Table 27's
`/Recipients` type cell to *byte string or array*; §7.6.4.1's substitutes *a crypt filter* for a
plural in prose three times. Both confirm their rows — the first binds an entry nobody here reads,
behind §7.6.5's named refusal, and the second leaves a reader's tolerance now stated in the row
instead of implicit. ADR 0712; `doc/errata-read.md` has all seven errata with their rectangles.

**The walk downward paid twice, and the first payment was a strike the outline had filed one
clause late.** Page 734 opens §14.4 and reaches §14.5, so `emit` prints §14.4's annotations under
§14.5's heading — and one of §14.5's two unnamed issues turned out to be §14.4's: two strikes
deleting *contents of the* and *'s contents* from the file identifier sentences, so both
identifiers become ones based on the PDF file at the time rather than on its contents. A gated
rustdoc blockquote in `write.rs::identify`, its prose, and §14.4's ledger note all stood on the
struck words — three words and two, under `check`'s four-word floor, the third of this rule's uses
to find quoted text on a strike below it. And the same coarseness had already reached a *verdict*: the record's own
row for the issue struck beside it judged §14.5's row — "names no digest" — one clause away from
the writer that names MD5. The eighth use's rule gains its sharper half: **a verdict written under
a heading is a claim about a page, not about a clause, until the rectangle has been placed.**

**The second payment is the settled-row mechanism by a fifth shape.** §14.7.6.2, `implemented`,
carries the erratum that inserts the class-route precedence rule the published clause never had —
attribute objects attached through `/C` may repeat `/O`, and the later in array order takes
precedence. `Tree::attributes` satisfies it by construction and always has; the row's one test
attached a single class object, **which no ordering of the class route can fail**. 755 found a
round trip that could not fail, 760 a sentence about a sibling row, 765 a set with no closure
check, 771 a claim of two written forms with a test of one — and now a rule satisfied by
construction whose only fixture was too small to exercise it. The new test is calibrated per trap
13 against a plant that walks the classes in reverse: it passes the single-class fixture and fails
the new one.

**One thing about the record, measured**: the eighth use's history file closed at "110" of a
population of 115 after five verdicts, and one of its five had carried a verdict in
`doc/errata-read.md`'s tables since the four-hundred-and-eighteenth session — so four newly left,
and this round's base count under the recipe's own parse is 111. A closing figure is a derivation,
and the greps are the instrument.

## What is still owed, named

- **An artifact census, which four numbers in this tree need and no command produces.**
  §14.8.2.2.2's row carried "30 of the 953 corpus first pages mark at least one artifact" from a
  one-off run against a corpus that is now 974, and nothing re-derives it: `witness_census` counts
  *names*, and artifact-hood is a `BDC` tag read by the interpreter. What it needs is one
  `pdf-model` example in `file_attachment_census`'s shape — open each corpus document, interpret
  page one, count the documents with a non-empty `Interpretation::artifacts`, and break the count
  down by Table 363's `/Type`. Perhaps sixty lines and one corpus pass; the same pass would settle
  the per-site `/AF` counts that §14.13, §14.13.3 and §14.13.6 each state and that do not add up
  against one another.
- **The `partial` rows not yet re-read against the code.** **The bands are not a floor**, and the
  section above has the measurement: 38 rows sit below commit 534 and 18 below commit 200, none of
  them carrying a read-and-kept sentence. The bands taken so far are the five-hundred-and-twenty-fifth's
  from 184 (§12.6.4.4, the row left under the fold) to 500 (§I, §I.2), fifteen rows, the
  five-hundred-and-thirty-seventh's from 511 (§12.7.4) to 517 (§12.8.2.4), thirteen rows, the
  five-hundred-and-forty-fifth's from 518 (§9.9.1) to 534 (§12.8.2.2.2), eleven rows, the
  five-hundred-and-fifty-third's from 534 (§12.8.3.4.3) to 536 (§12.6.4), eleven more, the
  five-hundred-and-sixty-second's from 541 (§11.3.7.2) to 546 (§14.11), ten more, and the
  five-hundred-and-sixty-fifth's from 553 (§12.5.1) to 564 (§8.4.5), fifteen more, plus the
  read-and-kept sets whose evidence is in their notes. ~~**The next band is the bottom of the list
  rather than the top**: §8.10 and its three form-XObject children at commit 89, §7.6.4.4.2 and
  §7.6.5's two `reported` rows at 94, §7.9.2 at 95, and §12.5.6.12's rubber stamp at 199, which is
  the one of those that can change a pixel.~~ **Taken in the six-hundred-and-twentieth**, minus
  §12.5.6.12, which the six-hundred-and-fourteenth had read in parallel; five of the eight were
  wrong and the section above has each. ~~**The next band is the eight rows the ordering now puts
  under rank 17** — §14.8.2.2.1 and §14.8.2.2.2, §12.11.3 and §12.11.6, §14.13 and §14.13.2, and
  §12.6.4.6, §12.6.4.9 and §12.6.4.10.~~ **Taken in the six-hundred-and-twenty-sixth**, and it was
  nine rows rather than eight; six of the nine were wrong and one of the six was work. ~~**The next
  band starts at what is now rank 10**, five hundred commits above the nine just read — §12.7.4,
  §12.7.6.2, §8.7.3, §11.7.5, §12.11, §14.8.4, §14.9, then §12.8.2.2 and its four neighbours.~~
  **Taken in the six-hundred-and-thirty-second**, and it was sixteen rows rather than twelve — the
  band runs on to §12.8.3.4.8 and the next row after it is forty-two commits above. Four of the
  sixteen were read and all four paid. ~~**The twelve left in that band are the next one**~~ —
  **rank 1 of the twelve, §12.7.4 with §12.7.4.1, was taken in the six-hundred-and-thirty-seventh
  and paid.** ~~**The eleven left are the next band**: §12.7.6.2, §12.11, and the nine signature rows
  from §12.8.2.2 to §12.8.3.4.8, which share two commits and, between them, one paragraph of
  boilerplate repeated five times — five `reported` rows citing three `pdf-model` tests for a
  report that `viewer_core::notes` makes, which is 620's newest shape waiting to be checked, and
  which is the shape that has now paid on four consecutive rounds.~~ **Taken in the
  six-hundred-and-forty-first, and the prediction in that sentence was right**: the five `reported`
  rows cited three `pdf-model` tests apiece for a sentence `viewer-core` writes, so nothing in the
  tree had ever asserted the report those rows *are*. §12.8.3.4.1 turned out to be a sixth of the
  same shape — its note says "which is what a test asserts" and its array named no PAdES test at
  all. Two counted claims went with them: §12.8.3.3.2's "the corpus's one witness" is three
  documents, and §12.8.5's "no corpus document carries a document timestamp" holds. ~~**The band's
  remaining rows are §12.7.6.2 and the four §12.8.3.4.x that keep their statuses**; the next band
  begins forty-two commits above at §10.4.2.4 and §10.4.2.5.~~ **Taken in the
  six-hundred-and-forty-eighth**, over a base of 843 commits where the band had shrunk to three —
  §12.7.6.2 at rank 513 and §12.8.2.2 with §12.8.3.4.2 at 517, with the gap above now fifty-nine
  commits to §10.4.2.4 rather than forty-two. All three were read and a fourth taken from below
  the gap, §7.6.4.1 at rank 578; every one of the four stated a reason that is a claim about this
  tree rather than about the standard, which is 620's rule choosing the work for the sixth time.
  **620's third shape paid for the seventh round running, and this time the defect was one an
  earlier round had already fixed for its siblings.** §12.7.6.2 is `reported` — a status whose
  whole content is "a person is told" — and cited
  `action.rs::a_name_the_table_does_not_hold_is_not_an_action`, which asserts that `/Teleport`
  yields *no* action and therefore never calls `action::refused` at all. That is the exact citation
  the six-hundred-and-twenty-sixth session found false for §12.6.4.6, §12.6.4.9 and §12.6.4.10 and
  replaced with an end-to-end click test. **Enumerating `refused`'s ten arms against the ledger
  found the second survivor**: §12.6.4.3 carried the same dead citation, twenty-two rounds after
  the shape was named and closed for three of its five siblings. So the enumeration technique found
  the row the band's ordering could not — §12.6.4.3's note had been rewritten since and ranks
  nowhere near the top — and the lesson is narrower than "enumerate call sites": **when a round
  fixes a defect that a family shares, the population it fixed is the arms of the function, not the
  rows it happened to be reading.**
  **And the shape 616 warned about has a second direction now**: a *parent* row overstating what
  its children do. §12.11 listed "Table 276's handlers" among what it reads while both of its
  children say `/RH` is read by nobody — every previous instance of the fifth failure shape had the
  parent understating, so a sweep looking for a missing thing would never have printed it.
  **The next band was taken in the six-hundred-and-fifty-second**, over a base of 851 commits:
  §10.4.2.4 and §10.4.2.5 at ranks 1 and 2, §9.7.5 at 3, §9.8 at 5 and §10.4.2 at 13, five rows and
  three defects, each one the fifth failure shape inside its own family. ~~**The rows left at the top
  are §11.7 at rank 4, §10.7.5 at 6, §10.7 at 7 and §8.6.5.7 at 8**, then the cluster of six at
  rank 9, of which five are left once §10.4.2 comes out — §7.6.4, §7.6.4.4, §8.11.4.1, §9.7 and
  §12.8.4.2, all sharing one commit.~~ **Taken in the six-hundred-and-fifty-seventh**, over a base
  of 859 commits where that prediction came out exactly — §11.7 at rank 1, §10.7 at 3, §8.11.4.1 at
  8, §9.7 and §12.8.4.2 in the cluster at 5–9 — **five rows read, four of them defects, and every
  one of the four is a parent summarising a family it had stopped reading**. §10.7 counted §10.7.3
  among "two parameters the clause lets a processor ignore and which are ignored" while Table 57's
  `/SM` has moved a shading's sampling since the seventy-fourth session; §11.7 attributed the whole
  of §11.7.5's debt to §11.7.5.3's black generation while §11.7.5.2 has been `reported` since the
  six-hundred-and-thirty-seventh; §9.7 named "§9.7.5.1's remainder" in the same commit that moved
  §9.7.5.1 to `implemented`; §8.11.4.1 named two of its three `partial` children while its own
  parent §8.11.4 has named all three since the same session. §12.8.4.2 was kept, with what its one
  cited test actually asserts written in. **§10.7's is the sharpest, because the identical claim was
  retired one row over and did not travel**: §8.4.5 carried `/SM` on its not-read list as "the
  silence recorded under §10.7.3" and the five-hundred-and-sixty-fifth corrected it there — a
  retired claim is a string, and the round that retires one owes a grep of the *tree* rather than of
  the family it is reading (ADR 0101). ~~**The rows left at the top are §10.7.5 at rank 2 and
  §8.6.5.7 at 4**, then §7.6.4 and §7.6.4.4 out of the cluster, then §11.5.3 at 10 and §11.3.4
  at 11.~~ **Re-derived in the six-hundred-and-sixty-third over a base of 869 commits, where that
  prediction also came out exactly** — §10.7.5 at 1, §8.6.5.7 at 2, §7.6.4 and §7.6.4.4 at 3–4,
  §11.5.3 at 5, §11.3.4 at 6, then nine sharing 7–15 on one commit. **That round took §8.6.5.7 by
  620's rule and then went somewhere else**, because step 7 below is a bigger population than the
  band is: §8.6.5.7's *first sentence* said "no place the shortcut would apply" while the same note
  three sentences down had said since the four-hundred-and-thirty-sixth that the conversion **is**
  performed on a page compositing in a press — the sixth failure shape, 227 sessions old, in the row
  the ordering puts second. ~~**The rows left at the top are §10.7.5 at rank 1, §7.6.4 and §7.6.4.4 at
  2–3, §11.5.3 at 4 and §11.3.4 at 5**, then the cluster of nine.~~ **Re-derived in the
  six-hundred-and-sixty-seventh over a base of 879 commits, where that prediction came out exactly
  again** — 242 `partial`-or-`reported` rows with a blamed note, §10.7.5 at 1, §7.6.4 and §7.6.4.4 at
  2–3, §11.5.3 at 4, §11.3.4 at 5, then the cluster of nine at 6–14. **That round read ranks 1 and 4
  and then went where step 7 pointed**, as 663 did: §11.5.3's `partial` rests on two residues and the
  one it writes out is asked for by no document in either population, while the one it is silent
  about is the crawl's majority case. ~~**The rows left at the top are §7.6.4 and §7.6.4.4 at ranks 1–2
  and §11.3.4 at 3**, then the cluster of nine — and the two aggregates are sweep 10's shape rather
  than 620's, so a round taking them is checking arithmetic over a family and not a claim about the
  tree.~~ **Re-derived in the six-hundred-and-seventy-first over a base of 887 commits, where that
  prediction came out exactly for the fourth band running** — still 242 `partial`-or-`reported` rows
  with a blamed note, §7.6.4 and §7.6.4.4 at 1–2, §11.3.4 at 3, then the cluster of nine at 4–12
  (§11.3.7, §11.4.1, §12.5, §7.6.6, §8.6.6, §8.9.6, §8.9.6.2, §9.8.3, §9.8.3.1, all one commit), then
  §14.6, §14.6.1, §7.6 and §7.7 at 13–16. **That round read none of them and went where step 7
  pointed**, as 663 and 667 did — the third round running to make that choice, which is itself a
  statement about the two instruments: the blame list ranks a row by when it was last *written*, and
  step 7 ranks a claim by whether the world moved under it, and only the second of those has a
  population that grew by fifty-three times. The band is where it was.
  **The six-hundred-and-ninety-first took it, over a base of 916 commits where the prediction came
  out exactly for the fifth band running** — 242 `partial`-or-`reported` rows with a blamed note,
  §7.6.4 and §7.6.4.4 at ranks 1–2, §11.3.4 at 3, the cluster of nine at 4–12 and §14.6, §14.6.1,
  §7.6 and §7.7 at 13–16 — **and read it by *family* rather than by rank**, which is what the band's
  own shape was asking for and what none of the eight bands before it had done: four of the top
  sixteen are §7.6's, and three of the round's four findings are a disagreement between two rows of
  that one family, which a band taking one row from each of sixteen families cannot see by
  construction. §7.6's corpus arithmetic accounted for every encrypted document as opening while
  §7.6.4.2's own row and `corpus.rs`'s `MAX_UNREADABLE_ENCRYPTION` both record two refused;
  §7.6.4.2's own four figures were 26/19/4/6 against 25/23/3/6; §7.6.4 was `partial` on a reason
  §7.6.4.2 records as *not* a debt; and §7.6.4.4.2 said "the one whose known password is the
  owner's" where three of the eight are, one of them asserted by a test in the tree. Two errata went
  with them, both bare `Caret`s under §7.6.6 — Issue #74's "or 5", which licenses reading a crypt
  filter at `/V` 5 and therefore every AES-256 file in the corpus, and Issue #184, which retires the
  `/Length` ambiguity a `crypt.rs` comment still asserted. **No status moved and no code was wrong**,
  which is worth recording rather than passing over: eight bands running had moved something, and a
  round whose findings are all prose has still made the family's next reading cheaper. ADR 0538.
  **The rows left at the top are §11.3.4 at rank 1**, then the cluster of eight the round's four
  leave behind — §11.3.7, §11.4.1, §12.5, §8.6.6, §8.9.6, §8.9.6.2, §9.8.3, §9.8.3.1 — then §14.6,
  §14.6.1 and §7.7; and **§9.8.3 with §9.8.3.1, and §14.6 with §14.6.1, are the same family shape
  one size down**, two rows apiece whose notes already argue with each other about Table 122 and
  about which tags are read. Re-derive the order before believing this sentence, for the reason two
  paragraphs up.
  **And the rule the round adds is about *choosing within* a band rather than about the band**:
  where the top of the blame list holds several rows of one clause family, read the family. ADR
  0455 ranks by the kind of claim a reason makes and ADR 0460 by where in the clause the answer
  would be; both rank a row against the standard or against the tree. This one ranks a row against
  **its siblings**, which is the comparison the fifth failure shape is defined by and which no
  ordering by age can produce.
  **The six-hundred-and-ninety-seventh took §11.4 by that rule and the seven-hundred-and-first took
  §14.6**, re-deriving the ordering each time rather than reading it here. On the base the second of
  those measured, §7.6.4.4 is rank 1 — ADR 0538's family, which the round that read it did not
  finish — §11.3.4 is 2, and §11.3.7, §12.5, §8.6.6, §8.9.6, §8.9.6.2, §9.8.3 and §9.8.3.1 share
  3–9, with §14.6, §14.6.1 and §7.7 at 10–12. **§14.6 was taken because all three of its rows are
  `partial` and two of them state the same list**, which tags this tree acts on by name: a claim
  held in duplicate is a claim with somewhere to disagree with itself, and it did. The count said
  four while the note's own previous sentence named the fifth — §12.7.4.3's `/Tx`, whose `shall`
  makes `appearance::spliced` cut where the tag says. §14.6 also wrote §8.11.3.3 twice for the
  clause §14.6.1 has had right all along, and gave as its reason for `partial` that §14.7's and
  §14.8's semantics are unimplemented, which the ledger's own rows under both deny. §14.6.2 said twice
  inside one sentence that an undefined `/Properties` name is reported, and no such report has ever
  existed — correctly, because §8.11.3.2 makes such a section ordinary content. **§14.6.1 moved to
  `implemented`**: its `partial` rested on a tag that is a structure type going unread, which
  §14.7.5.2 makes a `should` on the producer while saying the tag "is not directly related to the
  document's logical structure". ADR 0560. **The rows left at the top are §7.6.4.4 and §11.3.4**,
  then the cluster of seven; §9.8.3 with §9.8.3.1 is the family shape one size down, two rows whose
  notes argue with each other about Table 122.
- **The self-contradicting note has no instrument, and the seven-hundred-and-first measured the
  obvious one and declined it.** ADR 0551 closed on a shape all eighteen sweeps are blind to — two
  paragraphs of one note contradicting each other — and the construction that would see part of it
  is the eighteenth sweep with both sides inside one row: `overstated::parts`, `terms_in` and
  `is_an_assertion` against `unread::is_a_claim`, every piece already public. Measured before being
  built, which is ADR 0481's method: **794 rows with a note, 259 asserting a term, 930 assertions,
  46 contradicted inside one note, 24 of them marked as a correction quoting its retired wording —
  and every one of the 22 unmarked is noise.** Two of the three noise shapes are worse here than
  across rows: a part naming two terms with one stance each pairs them for free inside one note,
  where across rows the other row still has to deny the same term. **The third is the reason, and it
  is structural**: ADR 0523 made it this project's rule that a correction states the retired claim in
  words the sweep matching it can still find, so a note repaired for a self-contradiction *contains*
  the contradiction on purpose — the population of an intra-row sweep is defined to be dominated by
  the notes somebody already fixed. And it would not have printed either of that round's own two:
  a cardinal against an enumeration two sentences away, and a `partial` reason against a clause's
  modal verb, are neither of them an assertion and a denial over a `/Key` or a `Table NNN`. **An
  intra-row contradiction is found by reading, and the reading list is the family.** ADR 0560 §5.
- **A negative measured before the crawl is a negative nobody has measured, and the crawl is on this
  disk.** `doc/habits.md` says a negative claim decays when the population grows; the
  six-hundred-and-sixty-third was the first round to act on it in the ledger and the first row it
  tried paid four ways. §12.4.4.1 read "over the *page tree* of all 964 openable corpus documents and
  the 14 in `doc/` … **not one states a `/Trans`, a `/Dur` or a `/PresSteps`**". Re-derived with the
  same instrument over both populations: the curated corpora are **still 0 of 1133**, and
  `CC-MAIN-2021-31` is **276, 86 and 1 of the 65 703 that open** — so §12.4.4.2, §12.6.4.13 and
  §12.6.4.15 got their first witness that is not a fixture, all three in one crawled slide deck, and
  §12.4.4's four undrawn styles got a ranking (`Dissolve` 221 pages, `Blinds` 16, `Glitter` and `Fly`
  **none**). The cost is minutes: `presentation_census` chunked through `xargs -P 8` is under a
  minute over all 65 944 and `refused_action_census` reads every object of all of them in ninety
  seconds. About sixty sentences of this form are left in `ledger.toml`; **read them with the
  control run as well as the crawl run**, because the old sentence is usually right about its own
  population, which is exactly why nothing in the tree can see it. ADR 0490.
  **Six more were re-derived in the six-hundred-and-sixty-seventh, outside 663's two families, and
  two of them were false** (ADR 0493). "About sixty" was an impression and the population is a
  command:

  ```sh
  grep -o -E '[^.]*(no corpus document|nothing in the corpus|no corpus |no witness|no document (in|states|carries))[^.]*\.' doc/conformance/ledger.toml | wc -l
  ```

  Three things that round adds, and the first is the one that had blocked the others:
  **an instrument has a population too.** `witness_census`, the census the five-hundred-and-seventieth
  built *for* absence claims, had `doc/pdf.js`, `doc/corpora` and this project's fixtures hard-coded,
  so §12.5.6.7's "no document in **any population this project measures**" was a true sentence about
  the census reading as one about the world — `--crawl` is a scope on it now. **A name census is not
  a structural one**: `witness_census --crawl CL` says 81 documents and `free_text_census` says 33
  annotations, because three of four spot-checked `/CL`s are resource keys (ADR 0403's own warning,
  paid again). And **a zero owes a control** — `luminosity_mask_census` prints the blends it finds in
  any space beside the ones it finds in the space the row is about, and a planted fixture was run
  through both the census and `interpret` before the zero was written down.

  **Ten more were re-derived in the six-hundred-and-seventy-first and five were false** (ADR 0496),
  and what that round added is the *other* instrument's population: `absence_audit`, the structural
  half of this sweep, had the same three roots hard-coded that `witness_census` did, so ADR 0493's
  finding was half a repair. It has `--crawl` now and seven new blocks — §12.2's four Table 147
  boundary entries, §12.11.1's `/Requirements`, §12.5.6.21 and §14.11.6.2's `/TrapNet`, §10.7.2's
  `/FL`, §7.11.4.2's `/RF`, §12.6.3's `/PV` and `/PI`, §12.6.4.7's thread action — and its own
  `/VP` block now names each viewport's `/Measure` subtype, which is what turned §12.9.2's negative
  over. **Five false**: §12.2 (0 curated, 96 crawled), §10.7.2 (0, 88), §12.6.3 (0, 5), §12.7.5.5's
  `/Lock` (0, 90) and §12.9.2's rectilinear measure (0, 127). **Five held**: §7.11.4.2, §12.11.1,
  §12.5.6.21, §14.11.6.2 and §12.6.4.7, each now on a population fifty-three times the size.

  Three things that round adds:

  - **Plant a witness against a census you are about to believe, and plant it against the census
    rather than against the reader.** A hand-built file stating all seven constructs, dropped into
    `doc/corpora-own` for one run and deleted, scored **zero for the thread action** — because the
    first draft asked only the top-level objects and the file wrote its action inline inside the
    annotation's `/AA`, which is the six-hundred-and-forty-eighth session's finding exactly, in code
    written by a round that had just read it. A resource dictionary's `/ExtGState` was invisible for
    the same reason. `visit` recurses now. Two of the seven blocks would have reported a false zero.
  - **A negative can be false and its *sharper* half survive, and that is a different row from
    either.** §12.2's claim was two sentences in one — "none states any of the four boundary
    entries", which is false, and "the half of the clause that can change a pixel has no corpus
    witness", which is true, because all 96 witnesses state `/ViewArea` and `/ViewClip` as
    Table 147's own `/CropBox` default and the one document naming another box names it on the
    *print* pair. Writing only "false" would have thrown away the true half.
  - **The two instruments disagree where the claim is about a structure, and the direction says
    which is right.** §7.11.4.2's `/RF`: 55 710 crawled documents' raw bytes contain it, 32 192
    documents' decoded streams do, **one** states it as a name, and **none** carries it on a file
    specification. §12.11.1's `/Requirements`: 411 documents' streams, not one catalog. A byte
    search would have called both clauses well witnessed.

  Quoted as ranks rather than as commit numbers, for the reason
  the section gives. Re-derive the order before believing this sentence: a parallel round merging
  ahead of yours can take a row off it, which is what happened three bands ago.

  **Where the sweep stands, as a command and then as a reading.** The command above counts the
  sentences; this one splits them into the rows that have been re-derived and the rows that have
  not, on the only evidence a program has — whether the row names the crawl:

  ```sh
  python3 - <<'EOF'
  import re
  txt = open('doc/conformance/ledger.toml').read()
  crawl = re.compile(r'CC-MAIN|crawl', re.I)
  neg = re.compile(r"[^.]*(?:no corpus document|nothing in the corpus|no corpus |no witness|no document (?:in|states|carries))[^.]*\.")
  for block in txt.split('[[clause]]'):
      m = re.search(r'clause = "([^"]+)"', block)
      if m and neg.search(block):
          print(('done' if crawl.search(block) else 'OWED'), '§' + m.group(1))
  EOF
  ```

  ~~Of the 45 rows carrying such a sentence, 10 named the crawl before this round and 11 more do
  now, leaving **24**.~~ **Those three numbers are wrong and the script above is what says so**: run
  against the six-hundred-and-seventy-first session's own commit it prints **7 done and 38 owed**
  before that round and **17 and 28** after it, so the round that wrote the sentence moved 10 rows
  rather than 11 and left **28** rather than 24. The population, 45, is right. The lesson is the one
  this whole section is about, one turn further in: **a round that states a count beside the command
  that produces it has to run the command**, and the four groups below are what a reading produces
  when the count it starts from was carried over rather than measured — they named 24 rows where the
  instrument named 28 (ADR 0502). What is left is still **not** that many more runs of
  `witness_census`, which is the point of writing this down: the rows that remain are mostly the ones
  a *name* census cannot settle, and each needs an instrument of its own. **Run the script rather
  than reading a number off this bullet** — the six-hundred-and-seventy-sixth session retired
  §12.8.2.2.1 and the four groups below are its list, not a level. They are the four rows the earlier
  reading missed, folded in:

  - ~~**Five need a content-stream census, which nothing in this tree has.**~~ **Four do, and the
    fifth is a token rather than a shape** — corrected in the six-hundred-and-eighty-sixth (ADR
    0523), which found the content-stream census already built: `witness_census`'s third column
    searches every stream's *decoded* data, so anything whose witness is a **token** is in reach
    without an interpreter. ~~§9.7.5.4 (`beginrearrangedfont` and `beginusematrix` in a CMap) is one
    of those, and its control run is taken … leaving the crawl run owed, which is one invocation.~~
    **Done in the six-hundred-and-ninety-sixth, and it holds** (ADR 0548): 0 of the 65 703 crawled
    documents that open write either operator in any decoded stream, against 0 of 1239 curated, with
    **`endcmap` in 46 028 of them** as the control that the search reaches a CMap operator at all —
    a stronger control than the single `usecmap` the curated run had. Twenty-three minutes.
    ~~The four that are genuinely shapes: §8.5.2.1 …, §9.4.2 …, §9.7.6.2 …, §11.6.7 ….~~ **Two of
    the four are done and the instrument they shared is built**: `examples/operator_shape_census`
    lexes a page's `/Contents` and every form `XObject` its resources reach, so an *order* of
    operators is countable now as a *token* already was. **§8.5.2.1 is false** — a segment operator
    with no current point occurs on one curated first page and five crawled ones, and the row went
    from `implemented` to `partial` because of it, and back to `implemented` in the
    seven-hundred-and-second when the refusal and its report landed (ADR 0563) — **whose census
    figures are keyword counts and an upper bound**: the interpreter also requires an operator's
    operands to be numbers, and `examples/refused_segment_census` asks it directly and finds **0
    pages** over 1230 curated first pages — and **§9.4.2 is false in the half it states and
    true in the half it means**, which is the rule below. What is left of this group is
    **§9.7.6.2** (a codespace range a byte-by-byte match reads differently from a numeric one) and
    **§11.6.7** (a tiling pattern's paint); neither is an operator shape, so neither is this
    program's, and both need a reader's own answer rather than a lexer's. The artifact census
    already owed below is the operator census's shape and could be a fifth question in it.
  - ~~**Nine need a structural block in `absence_audit`, which the six-hundred-and-seventy-first round
    added eight of and the six-hundred-and-seventy-sixth the ninth.**~~ **Eight of the nine were
    taken in the six-hundred-and-eighty-second** (ADR 0516) — §7.6.5, §7.9.2.2.2, §8.9.5.2, §8.10.3,
    §11.6.5.2, §12.3.2.2, §12.4.2 and §12.5.1 — and **seven of the eight negatives were false**.
    ~~§12.8.2.2.1 (a `/DocMDP` whose `/P` is not 2 — `witness_census --crawl` already says **144**
    crawled documents name one against the corpus's one, so this is a false negative waiting for its
    `/P` values)~~ — **done, and the prediction was right**: 143 of the 65 944, of which 122 state
    `/P` 1 (ADR 0502). **A round adding one adds a block, not a heuristic**, and the blocks are
    cheap: the six-hundred-and-seventy-first's eight cost about a hundred and thirty lines and four
    minutes over the whole crawl, the ninth about forty and nothing extra, and this round's eight
    about three hundred and fifty lines and ninety seconds on top of a five-and-a-half-minute pass.

    ~~**§14.8.2.5.3 is the one left, and it was in the wrong group.** `/ReversedChars` is a
    marked-content tag inside a content stream, so it belongs with the five above rather than here:
    no dictionary anywhere states it, and a structural block over the object graph would report a
    false zero for exactly the reason this file keeps writing down.~~ **The group was right and the
    instrument was already here** — taken in the six-hundred-and-eighty-sixth (ADR 0523). A
    structural block would indeed have reported a false zero; what nobody had noticed is that
    `witness_census`'s **third** column is a substring search of every stream's *decoded* data,
    which is a content-stream census for anything whose witness is a **token**. It found the tag in
    one curated document and three crawled ones, and printed its own discriminator on the fourth:
    the one hit scored *as a name* rather than only in a stream is `/S /ReversedChars` in a
    structure tree with a `/RoleMap` to `/Span`, which is not this clause at all. **And the sentence
    had decayed twice** — it was measured over *first pages*, and the curated witness writes its tag
    on page 6, so the population and the instrument's reach were both narrower than the words.

    **A sixth rule, from the six-hundred-and-ninety-sixth** (ADR 0548): **a negative about a
    *structure* and a negative about its *consequence* are different sentences, and the instrument
    has to print both.** §9.4.2's row said no corpus document moves `Tm` inside a `q` … `Q` pair in
    a text object; four such pairs sit on `NegativeFontSize.pdf`'s first page, so the outer claim is
    false — and not one *well-formed* page draws differently, because Table 106 makes the next `Tm`
    replace what the `Q` restored, so the restore reaches a mark only in a damaged stream. One extra
    column in the same state machine separates the two. The other direction cost more: §8.5.2.1's
    row called a reachable requirement `implemented` on the strength of the shape having no witness,
    which is the same conflation upside down.

    **Three things the six-hundred-and-eighty-sixth adds to the recipe.** **A negative can be false
    and the code still owe nothing** — §7.6.5's one witness is a file declined by name, which is
    trap 5 working. **A
    negative can be false and the *residue* it justifies survive one condition narrower**: §11.6.5.2
    is false by 2882 crawled documents while the refusal it names is reached by six of them, because
    `soft_mask_entry` asks about the codec only where `worth_combining` has already refused the finer
    grid — count the population the sentence is about, not the population that shares its noun.
    **And probe a positive as well as a zero**: the first draft of the `/Decode` block scored
    `issue10339_reduced.pdf`'s `/Decode [255.0 0.0]` a departure, because on an eight-bit `Indexed`
    image Table 88's default is `[0 255]` and its "exact reversal" is that and not `[1 0]` — a
    census that compares against one family's default retires a claim that holds.
  - **Seven are not a claim about a corpus at all**, and a round should stop rather than measure:
    §7.5.6 (a multiply-updated file that lowered its own version), §8.9.3 and §11.6.2 (a construction
    with no file behind it, argued from the clause), §10.7.4 (a ladder run with no document in the
    way), and the rows §8.4.3.5, §9.7.4.2 and §12.5.4 already answer with `long_mitre_census`,
    `hollow_glyph_census` and `border_precedence_census` — ~~those three owe a `--crawl` argument
    rather than a re-reading~~ **and all three have one since the six-hundred-and-eighty-sixth**
    (ADR 0523), which is the same `Scope` selector ADR 0493 gave `witness_census` and ADR 0496
    `absence_audit`, plus `rayon`, plus the explicit file list two of them already took. **All
    three negatives were false**, and two of them left a *sharper* claim standing that the wider
    count would have thrown away — the rule ADR 0516 found at §11.6.5.2, met twice more. §9.7.4.2
    is the one the four-group reading missed. ~~**§12.7.5.4 is what is left of this group**, and its
    instrument exists too: `variable_text_census` … owes the same scope selector.~~ **Done in the
    six-hundred-and-ninety-sixth** (ADR 0548): the census has the selector, its control run
    reproduces the row's figures to the digit, and **the negative holds** — the crawl states two
    list-box widgets over two documents, both with an `/AP` `/N` stream and neither in a
    `/NeedAppearances` document. **And the sentence the script actually matches in that row is a
    correction quoting a retired negative**, so §12.7.5.4 was a member of the noise group below all
    along; what was owed was the *count* beside it rather than the sentence, which is a distinction
    worth carrying: the regex defines the queue's population, and a row can be noise for the regex
    while its measured figures are stale. Run the script rather than adding any of this up, for the reason the bullet above
    gives.
  - **Six are this population's own noise shape and nothing is owed.** Three are a *correction*
    quoting the negative it retired, which is the same false positive the twelfth and seventeenth
    sweeps print and is why the grep at the top of this bullet is a reading list rather than a
    verdict: §9.6 ("[t]his row said `partial` … and both halves stopped being true"), §9.8.1 ("[t]he
    claim … is now false in both halves") and §12.8.2.4 ("[t]his row read `reported` on the claim …
    and that claim was false") each *are* the repair, quoted. **Three more are the grep's own
    sentence boundary rather than the ledger's**, and they are the rows the four-group reading left
    out: the regex ends a sentence at any full stop, so a clause number or a file name inside one
    splits it — §8.11.4 matches "no corpus document" out of a fragment beginning at "§8.11.**4**'s
    usage application dictionaries", §9.10.2 out of one beginning at "§9.10.**3**'s row", and
    §8.6.5.6 out of a correction cut short at `bug886717.**pdf**`. Read the hit and it is finished
    with; a round meeting any of these six has done its work by reading it.
- **Run the sweeps before your own edit as well as after it, and account for every number that
  moves.** The twelve already run on any round the ledger moves; what the six-hundred-and-fifty-seventh
  added is that the run *before* is not optional and the deltas are stated rather than reported.
  A level is one integer, produced by a program that does not know what this round is trying to say,
  and it reviews the sentence a round is about to add for free. It has now caught two: the
  six-hundred-and-fifty-second's draft wrote a denial in words `--bin overstated` scores as an
  assertion (8 contradictions became 12), and this round's first correction to §8.11.4.1 repeated
  three entry names §8.11.4.3's row already carries, doubling `--bin unread`'s standing noise on all
  three (69 rows / 182 keys became 70 / 185). Both sentences were *true*; neither was worth writing.
  The rewrite points at the neighbouring row instead of repeating its list, which is the same
  conclusion the six-hundred-and-fifty-second reached about §9.8.1's — **an entry list lives in one
  row**. ADR 0485, and it is not a gate: these levels move for good reasons every round and pinning
  them would make twelve reading lists into twelve ratchets.
  **And the six-hundred-and-sixty-seventh added a third catch and a noise shape with it.** `--bin
  owed` went 181 unnamed terms over 114 rows to 182 over 115 and the row that left the reading list
  was §11.5.3's, on the term **`luminosity`** — which is not a debt but the leading segment of the
  citation `examples/luminosity_mask_census`, read as a `/Key` because a solidus followed by letters
  is what that sweep's key extractor looks for. **It is a standing shape rather than a new one**:
  `examples/border_precedence_census` yields `border`, which no source names either, so *every* row
  that obeys `CLAUDE.md`'s "write down the command" rule inside a `partial` note moves this level by
  one. Neither repair is right — dropping the citation is the instrument choosing what the ledger may
  say (ADR 0490 §6), and teaching the extractor about paths is a guess, because a `/Key` and a path
  segment are the same characters. Know the shape and account for the one. ADR 0493.
  **And the six-hundred-and-seventy-first found the same shape one sweep over, in `--bin unread`.**
  Its confirmed count fell 46 → 44 and its quoted count rose 136 → 138, both on the single key
  **`/FL`**, under §8.4.5's row and §10.7.2's — because the census this round wrote to *measure* how
  many documents state `/FL` names the string `"FL"`, and that sweep asks whether any source quotes a
  key the row calls unread. **So a round that measures an unread entry makes its own row look wrong**,
  and the repair is neither to drop the census nor to teach the sweep about examples: the sweep's own
  discriminator already handles it, because its read-first list is the keys named by *the row's own
  `code` array*, and that number did not move (68 both times). Read the witness path, which is what
  the sweep's closing sentence already says. **And `--bin tables`' absent list moved 99 → 100, on this
  round's own prose**, which it noticed only because it ran the sweeps a third time on the *committed*
  tree rather than on the ledger alone — an ADR, a history file and this file are `SOURCE_ROOTS` too.
  §12.2's finding turns on a default, so the sentence carrying it names a page-boundary value beside
  the number of the table whose entry takes it, and the sweep reads the pair as a key citation; it
  prints the right answer itself (`stated by: Table 31, Table 396`) and marks the hit `[correction]`,
  which demotes it. **That is the sweep's own second documented noise shape** ("a table's *value* is
  named beside its entry"), charged to any round that writes down what a default resolves to. **And
  it briefly read 102, because the two places describing the hit repeated the pairing that caused
  it** — documenting this shape instantiates it, once per place, which is what a sweep over adjacent
  words does. The finding's sentence is not rewritten to dodge it (ADR 0490 §6); its *description*
  gives the example once, which is the ordinary reason to give an example once, and the level settles
  at 100. Levels, on the committed tree,
  after → the value each had before the round: `counts` 6877 ← 6835, `quotations` 1772 ← 1769 in the
  ledger with all three new ones verbatim and 5335 ← 5316 over the documents, `tables` 5861 ← 5836
  sentences and 2217 ← 2213 key citations, `pointers` 7153 ← 7115, `owed` 3553 ← 3530 terms with 181
  over 114 rows unchanged, `entries` 285 ← 283 rows, `overtaken` 494 ← 493 decision records, and
  `--bin overstated`'s corroborations 56 ← 55 with its 8 contradictions and 7 marks standing. **`owed` gained no phantom this round and the reason is worth knowing**: the citations
  added are `examples/absence_audit` and `examples/witness_census`, whose leading segments are
  `absence` and `witness` — ordinary English words that appear in those very files, so the extractor's
  phantom key is *named by a source* and never reaches the unnamed list. ADR 0493's shape costs a
  round one phantom only when the invented noun is invented.
- **§12.8.2.4's transform, named rather than built.** `has_transform(document, dict, b"FieldMDP")`
  plus Table 259's `/Action` and `/Fields` beside `UsageRights` is the whole of the recognition
  half; the validation half is §12.8.2.2.2's and needs the signed revision reconstructed.
- **A noun that is an ordinary English word is a noun the fourth sweep cannot rank.** `prefix` and
  `joining` between them produced 298 of the first run's 544 mentions and not one finding, and
  `Window` and `widen` produced 1462 of the five-hundred-and-thirty-seventh's 1512. Choosing
  the invented nouns first is free; widening the program's matching would only make it worse.
- ~~**The fourteenth sweep's vocabulary is the next one a program should take over.**~~ **Taken over
  in the five-hundred-and-sixty-second** (ADR 0397), and the answer was not the one this entry
  predicted. It said the seventh sweep's discriminator was "available to it: a discriminator taken
  from the ledger instead of from memory" — and a vocabulary *learned* from the ledger measures
  topic rather than debt, because status and subject are not independent in it. What the seventh
  sweep actually supplies is its **measurement**, taken from the tree: a note that names a debt names
  a thing, and a thing this tree does not have is a name no source carries. **§9.3.1's inverse is
  still no better** — a row that names a debt and is wrong about it remains invisible — and that is
  a property of the question rather than of the instrument.
- ~~**Sweep 10's level is the last one that is still session-local.**~~ **Taken over in the
  five-hundred-and-sixty-fifth** (ADR 0400). This entry said it "needs no discriminator at all — the
  count is in the sentence and the family is in the file", and **the first half was wrong in the way
  sweep 14's prediction was**: the family is indeed in the file, and *which numbers are about it* is the
  whole problem — ten hand-runs printed ten incomparable levels because each wrote a pattern for that.
  What settled it was again another sweep's measurement rather than a new heuristic: the ninth's
  attribution rule for the population, the sixth's family arithmetic for the answer. **Only sweep 6
  itself is left as a description**, and it is two hits long and has never printed anything else.
- **The sixteenth sweep's structural half names one claim per block and there are always more.**
  `absence_audit` hard-codes one block per claim, which is what makes each honest — the reader that
  would act on the entry is the one asked — and what makes it incomplete. The claims a name census
  cannot settle are the ones left: a flag's bit, a value's range, a group's colour space, a
  producer's arithmetic. A round adding one adds a block, not a heuristic. **It was seven blocks and
  three hard-coded roots until the six-hundred-and-seventy-first**, which gave it `--crawl` and eight
  more, and the six-hundred-and-eighty-second added eight claims in nine blocks. **The ninth block is
  the narrower half of §11.6.5.2's claim, and that is the rule the round adds**: a claim whose
  falsification is a count and whose residue is a *condition* wants two blocks rather than one,
  because the two populations differed by three orders of magnitude and one report line cannot say
  both. **The seven-hundred-and-twenty-fifth adds a block for a claim that was not a negative at
  all**, which is the third kind this example can settle: §10.7.5's `/SA` was counted in two written
  sentences that disagreed — 49 in a ledger row, 30 in `oracle.rs` — neither naming a population and
  neither naming a command, and neither a name census could arbitrate because the clause fires on the
  *value* and a `/SA false` states the entry too. A block asking `Object::Boolean(true)` by the same
  two routes §10.7.2's `/FL` block already uses settles it, and the same run reprints the `/FL`
  figure exactly, which is the backwards planted-witness control the bullet below asks for. ADR 0610.
  Two rules the blocks
  themselves teach: **walk into each object's nested structure**, because a producer may write an
  action, an annotation or a resource dictionary inline and a top-level-only walk reports a false
  zero (session 648, paid again in this example's own first draft); and **plant a witness stating
  every construct before believing any zero**, which is what caught that.
- **`absence_audit` and `witness_census` are not the only two instruments with a population, and
  the six-hundred-and-eighty-sixth gave three more the same selector** (ADR 0523): `long_mitre_census`,
  `hollow_glyph_census` and `border_precedence_census` take `--pdfjs`, the curated corpora or
  `--crawl`, and the first two gained `rayon` with them. **What that round adds to the recipe is a
  control nobody had been running**: before believing what a census says about the crawl, run it
  against the population the *old sentence* was measured over and check that it reprints the old
  numbers. Two of the three did, to the digit — 33 781 constructed borders with one `U` and one `D`;
  42 `/CIDToGIDMap` streams in 30 documents with 214 of 221 programs partly hollow. A census that had
  drifted under its row would have printed something else, and there would have been nothing to say
  about the crawl until that was explained. It is the planted-witness rule pointed backwards: a
  *positive* the instrument is already claimed to produce.
- **The fifth sweep's second population is still a by-hand run.** `--bin callers` reads functions;
  the four-hundred-and-thirteenth ran the sweep a second way over `viewer-core`'s own vocabulary —
  every `Command`, `Query`, `Answer`, `Event` and `Edit` variant against the crates that speak it —
  and that is where `Query::Find` and `Query::LogicalSelection` turned out to reach no program. A
  round that wants a sixteenth has it named here.

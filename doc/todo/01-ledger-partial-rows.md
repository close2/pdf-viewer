# Read the ledger's `partial` rows against the code

Status: **standing task, and how much of it is left is a command rather than a figure here** —
`git blame --line-porcelain doc/conformance/ledger.toml`, ordered by the commit that last wrote each
`note = ` line, against `cargo run -p conformance --bin ledger` for the population. The rows nothing
has rewritten for longest are the ones nothing has read, and the reading list is that order, quoted
by **rank** rather than by commit index (ADR 0455).
Priority: 01 — the population with no gate, and it has paid on every round that touched it
Code: `doc/conformance/ledger.toml`, checked by `cargo test -p conformance`

**A sweep round commits one prose sweep as a program before running any of them.** This is the
binding rule the sweep programs under `tools/conformance` each cite. A sweep that lives only as a
description is rebuilt from its own paragraph by whichever round next needs it, and its level is
not comparable across runs because every round writes its own pattern (ADR 0319, ADR 0360) —
`CLAUDE.md`'s rule that what is written down is the command that counts a fact, failing in the
direction it was written for. The cheapest moment to commit one is the round that next has to run
it, because that round has to reconstruct it anyway. What is still a description is listed in
*What is still owed* below.

## The partial and reported rows, and what would move each

Each row's note states one current reason and what would move it (ADR 1574); this list is that
reading in one line per row, and the note is where it is argued. The membership is
`grep -B3 'status = "partial"\|status = "reported"' doc/conformance/ledger.toml`, never this list.

- **§7.4.7** — the pinned `hayro-jbig2` ignores EXTTEMPLATE, so such a region is refused by segment;
  moves when the owner's fork takes `doc/patches/hayro-jbig2-extended-template.patch` (ADR 1459).
- **§7.4.9** — e-sRGB, e-sYCC, CIE Jab and non-D50 CIE Lab take the device fallback because their
  defining texts are not held; moves on a text the owner buys, or a document `jpx_colour_census`
  finds stating one where it decides. The irreversible path's one-level differences are not debt.
- **§7.6** — aggregate: the public-key handler; moves with §7.6.5's family.
- **§7.6.5, §7.6.5.1, §7.6.5.2, §7.6.5.3** (`reported`) — the handler is refused by name; the build
  waits on `doc/questions/A66`'s trigger, which `doc/questions/A168` reads as unlit.
- **§7.6.6** — Table 27, which is the handler's; moves on the same trigger.
- **§12.8, §12.8.3, §12.8.3.4** — aggregates of §12.8.3.4.4 (ADR 1035).
- **§12.8.3.4.4** — no signature in reach states a policy whose constraints could be enforced;
  moves when one does and its named specification is held. The network is not what is missing
  (ADR 1291).
- **§12.10, §12.10.2** — the projection is not built; moves on the build `doc/questions/A171` owes,
  census first.

§12.1 states no requirement and is `implemented` (ADR 1573).

## Why this file exists

Every subclause of the technical clauses and of the normative annexes has been read against this
code, and the statuses are gated: `silent` is zero, `REVIEW_OWED` fails the build the moment a
cited-but-unread clause appears, and the file-only evidence ceilings are asserted with `==` (see
*The gate's own ceilings* below). What no gate can watch is a **note that has gone stale**, and the
`partial` rows are where those live — with the settled rows close behind them, because a claim that
nothing is owed has no missing thing to grep for (ADR 0465).

## The failure shapes

Each sweep below exists for one of these, and the shapes are what the sweeps' own comments cite.

1. **A note that understates what the code does.**
2. **A note whose reason has expired** — *while §X does not exist*, *needs §Y*. The first sweep.
3. **A note claiming an entry is unread where the tree reads it.** The second sweep. It is the
   cheapest shape to check and the one nothing fires on. **Grep the shape, not the wording**:
   *read by nothing*, *is unread*, *nobody reads* and *Not read:* are the same claim.
4. **A note whose *what is done* half is wrong** — the class that resists a grep, because the name
   being present is what a grep looks for.
5. **A note that is stale about its neighbour: a family head gone stale.** A family's parent row is
   not maintained by the rounds that implement its members, because the clauses do not cite each
   other. Its commoner direction is a parent understating its children; its other direction is a
   parent *overstating* them (the eighteenth sweep, ADR 0475). At family scale it is a parent's
   stated count that its children contradict (the tenth sweep, ADR 0400).
6. **A note that contradicts itself.** A row corrected by *appending* a sentence, with nobody
   re-reading the paragraph above it; and its sharpest form, **the note was corrected and the status
   was not** — invisible to every grep, because the note a sweep reads is the half that is right.
   Read a corrected note **whole**, not from the correction onwards. No instrument sees the
   intra-row form (see *Measured and declined*); it is found by reading, and the reading list is
   the family (ADR 0560 section 5).
7. **Two rows about one mechanism, disagreeing.** A mechanism gets one row per clause that mentions
   it, written by different reasoning; the tell is that one row names a *capability* (*a screen is
   not a printer*) while the other names *code*. The sixth sweep cannot see this — the pair are
   cousins, not parent and child. When a row's reason is about what this program is rather than
   about what the clause says, find the other row. The seventh sweep prints the cousin (ADR 0388).

Three further lessons the sweeps found, each a variant worth naming:

- **Right conclusion, expired argument.** A claim still true whose stated reason is not — the
  fourth sweep's finding over §12.5.5 and §12.5.6.22 after ADR 0168, and the same shape in a doc
  comment as well as in a row.
- **The blocker was the interface.** When a row's reason names a capability, ask what the program
  would have to *say* to obey the clause, not only what it would have to have — a row can survive
  the arrival of the very thing it names (ADR 0167, §14.9.3's `/TU`):

  > An alternative name may be specified for an interactive form field (see 12.7, "Forms") which, if
  > present, shall be used in place of the actual field name when an interactive PDF processor
  > identifies the field in a user-interface.

- **The reason names something the program lacks rather than something the standard leaves open**
  (ADR 0162, §12.3.2.1). After a round that gives the program a verb, re-read the rows whose reason
  is about what the program *is*, not only the ones about what a clause needs — §12.8.2.2.1's
  parenthesis is the standing example of a `shall` that waited on a verb:

  > (These changes to the document shall also be prevented if the signature dictionary is referred
  > from the DocMDP entry in the permissions dictionary.)

## The sweeps as commands — what each one asks, and how to read its output

`doc/todo/02` section 4 keeps the rule — run them after a round that adds a verb, over `crates/`,
`tools/` and `fuzz/` as well as over `ledger.toml` — and the shape they share; what each one *is*
belongs with the reading, which is this file. **Every sweep's hits are read before they are
believed**: a sweep ranks, and the noise shapes each prints are named in its module doc and below.
**A clean run is a result** — it says the population has not drifted, which is the only way that
population is ever watched at all.

**The oldest false positive, common to every sweep that reads prose**: a correction quoting the
wording it retired. It is marked rather than dropped wherever a sweep can recognise it.

1. **Expired blockers** — `cargo run -p conformance --bin blockers`, seconds, over `ledger.toml` and
   the source roots (ADR 0336). A blocker sentence naming a clause (*while §X does not exist*,
   *needs §Y*, *until §Z*) is judged against the ledger's own account of that clause, and the expired
   ones print first. Three noise shapes are printed rather than filtered: a correction quoting the
   wording it retired, a past tense no grep can see, and a clause named as the route to something
   outside the standard.
2. **An entry claimed unread that a source quotes** — `cargo run -p conformance --bin unread`,
   seconds (ADR 0324). A hit is a key some source quotes as a lookup string while a note says nobody
   reads it, sharpest where the quoting file is in the row's own `code` array. The dominant noise is
   **one short key, three clauses** — §8.4.5's `/BG` and `/TR` are Table 57's while `"BG"` in
   `appearance.rs` is Table 232's — so read the witness path before believing a hit. A claim no
   source quotes prints as `confirmed:`, because only the clause can settle whether a reader owes
   the entry at all. The by-own-code count does not reach zero by the ledger's own habits: what is
   left is a sentence retiring the claim it names, a calibration naming a planted defect, a
   neighbouring key, or a true claim whose own code quotes the key for another purpose. A round
   that *measures* an unread entry makes its own row look wrong (the census names the key); the
   read-first list, keyed to the row's own `code`, is what does not move.
3. **A capability a note says is absent that the tree names** — `cargo run -p conformance --bin
   capabilities`, seconds (ADR 0345). A hit carries the witness path and says whether the claim is
   about *the program* (the population that decays) or about *one crate* (usually a boundary it
   keeps on purpose — no clock, no filesystem, no toolkit). The dominant noise is a true boundary
   statement. **A comment about a sibling crate's capability decays at that crate's pace and not at
   its own**, which is why the sweep runs over the source tree as well as the ledger: a crate's
   front-door documentation outlives every ledger row that says otherwise, and a warning written
   where the work is does not fire either.
4. **Where else a retired claim is still written** — `cargo run -p conformance --bin retired --
   <noun> …`, seconds, over `ledger.toml`, the source roots and every Markdown document under `doc/`
   bar `doc/history/` (ADR 0352). The one sweep that cannot derive its own population, because what
   was retired is what the last rounds decided: give it the **mechanism**, not the sentence — the
   noun the correction was about. Each mention prints as a correction or as a standing claim, and a
   noun carrying both is the shape to read first. Two clauses describing one mechanism is the
   commonest shape in this file, and correcting one leaves the other lying. `doc/adr/` is among its
   targets (ADR 0265): an ADR is a record and is never edited, but a claim a later round disproves
   is answered by that round's own ADR naming it. A noun that is an ordinary English word ranks
   nothing; choose the invented nouns first. Running it in the same round as the correction is the
   cheapest it will ever be.
5. **Who calls it?** — `cargo run -p conformance --bin callers`, a fifth of a second, over every
   `pub fn` in `pdf-model` against every crate, tool and fuzz target whose manifest names it (ADR
   0360). A capability arrives and nobody maintains the *callers* of the code it unblocks — the model
   has the distinction, tested, and no host asks it (§12.5.6.19's `/H`, ADR 0177; §8.11.4.3's
   `/ListMode`, ADR 0178). **Its output is a delta rather than a level**: read the rungs from the
   bottom, and know the two directions it is loose in — a short name shared with another type's
   method reads as named, and a name reached through a wrapper reads as unnamed. Its second
   population is still by hand (*What is still owed*).
6. **Which parents are behind their children?** — still a description, and four lines of Python over
   `ledger.toml` alone: print every row that is `partial`, `reported` or `unreviewed` while **every
   one of its direct children** is `implemented`, `inapplicable`, `out-of-scope` or `writer-side`. A
   parent can owe more than its children, so a hit is not automatically wrong, but it is always a
   row nobody has re-read since the last child closed. **Run it at the head of any round that reads
   rows**: a parent behind its children is the one defect that needs no clause read to find. **The
   sweep is a chain rather than a list** — one wrong `partial` child hides its parent, so a round
   that clears a hit runs it again in the same round. A dismissal of a hit written once and cited by
   later runs instead of re-derived is how §7.9.2 outlived the instrument that kept contradicting it
   (ADR 0455).
7. **The status nobody expects to come back to** — `cargo run -p conformance --bin inapplicable`, a
   fraction of a second (ADR 0388). It takes an `inapplicable` row's own title and note apart into
   `/Key`s and identifiers and asks whether the tree names them. **The count of naming files is the
   discriminator**: the standard's shared vocabulary reaches dozens of files and sorts last, a rare
   word sorts first. Read the **cousin** it prints before anything else — a row that is not
   `inapplicable` and says the same word is the seventh failure shape. What it cannot see is a row
   whose vocabulary is right and whose account of the requirement is wrong; only reading the clause
   finds that. The status has one word for a permission this program declines and for a clause
   about a thing this program is not; every such note says which it means.
8. **Does the file — and the symbol — a note names still exist?** — `cargo run -p conformance --bin
   pointers`, a third of a second, over `ledger.toml`, the source roots and every Markdown document
   under `doc/` bar `doc/history/` (ADR 0372). **A pointer is resolved from where it is written**: a
   `tests/x.rs` in a doc comment means its own crate's tests, the same words under `doc/` are
   *unrooted* rather than dead, `raster/` has a `CLAUDE.md` of its own so its `crates/…` is
   `raster/crates/…`, and a module path is read from beside its file. The other rungs it prints
   instead of a finding: a fragment that resolves in another crate; a form (`doc/todo/NN`, a glob, a
   template such as `scratchpad/r<round>/`); a path this tree deliberately does not carry, split into
   what a `.gitignore` pattern ignores (ADR 1391) and a short hand list; an owner's `A` file whose `Q`
   is here (ADR 1379); and a dead pointer inside a **record** — `doc/adr/`, `doc/reviews/`, the dated
   correspondence — which is *historical*, its date rather than a defect (ADR 1403). What is absent
   outside a record is real breakage and is fixed where it is written. A pointer and the claim it
   supports often die together, and only the pointer is greppable without knowing the clause.
9. **Does the table a sentence cites state the key it gives it?** — `cargo run -p conformance --bin
   tables`, seconds (ADR 0380). It counts a key only where the sentence **attributes** it, prints
   which table *does* state it, and reads a **denial** as a claim in the other direction. Its findings
   arrive in **blocks** — a run of consecutive rows written in one sitting against ISO 32000-1's
   numbering — and its most durable population is a document: a number retired in the code goes on
   living in the ADR the code came from. Two ways it goes quiet (ADR 0620): a list of keys whose
   third item carries a parenthesis attributes nothing after it, and a citation whose attributed noun
   is a *value* lands in the keyless count — which is a hiding place as well as a noise filter. Its
   documented noise: a table's value named beside its entry, and a dictionary's key cited under the
   table of flags inside it.
10. **Does the family a sentence counts hold that many rows?** — `cargo run -p conformance --bin
    counts`, seconds (ADR 0400). A cardinal is a claim about a family only where it governs one of
    the ledger's own words for a row and only inside the sentence's own punctuation, and the answer
    is the family's own arithmetic — a count that leaves out the `General` row is right, and so is
    one that counts the clause's own row in. Read the **contradictions** first: two numbers for one
    family in two sentences of one note are wrong whatever the ledger holds.
11. **The ledger's and these documents' quotation marks** — `cargo run -p conformance --bin
    quotations`, seconds, over every Markdown file this project wrote under `doc/` and over
    `ledger.toml`'s notes (ADRs 0249, 0309, 0375). A miss is reported only where it matches the
    standard for at least five words and then diverges: an invented claim shares no words with the
    standard and a misquotation shares most of them. It reads single-quoted spans as well as double,
    and prints how many so that a clean run says what it was clean over. **Suspect the conversion
    before the document** — `doc/md/` losing text the PDF has, and a hyphen of a word broken across a
    line, are its commonest suspects. It has a floor: a quotation of ISO 32000-1 under five words of
    agreement falls through it.
12. **Does a quotation land on text Errata Collection 3 struck?** — `cargo run --release -p
    spec-errata -- check doc/*.pdf`, seconds (ADR 0254), in the sidecar for ADR 0252's reason: the
    errata are read out of PDFs, and nothing this project generates may become what the gate checks
    the standard against. `check` compares only *quotations this tree has written*, so an erratum over
    text nobody quoted is invisible to it. The other directions are commands too:
    `spec-errata -- emit doc/*.pdf`, read against what the ledger **claims** (a round implementing a
    clause runs `emit` on that document **before** it writes); `spec-errata -- moved doc/*.pdf` (ADR
    0400), every annotation whose instruction moves, renumbers, deletes or inserts and names a clause
    number, with what this tree has standing on that number; and `spec-errata -- renumbered
    doc/*.pdf` (ADR 0750), for a **table**'s caption renumbered by a bare strike and caret, grounded
    on a designation the conversion captions and ranked by whether the filing clause captions that
    table. A strikeout whose whole text is a requirement word (`Optional`, `Required`, `Deprecated`)
    is still read by eye. A table designation's correctness is gated (ADR 0760):
    `every_table_designation_names_a_table_the_standard_captions` reads
    `conformance::citation::Scan::designations`, and a foreign standard's table names its document.
    `doc/errata-read.md` records what each erratum read came to.
13. *No sweep holds this number.* It was given to two proposals in turn, both now in *What is still
    owed*, and the count of sweeps is kept apart from the count of committed programs (ADR 0475).
14. **The status whose own definition promises a debt** — `cargo run -p conformance --bin owed`,
    seconds (ADR 0397). A `partial` row must say which requirements are not executed; a debt names a
    **thing**, and a thing this tree lacks is a name no source carries — the seventh sweep's
    discriminator with the sign reversed. The reading list is every row whose vocabulary the tree
    names in full, the one naming nothing specific first. Noise: a debt named in prose with no
    identifier, and a phantom key from a citation such as `examples/border_precedence_census`, whose
    leading segment is read as a `/Key` — know the shape and account for the one (ADR 0493). A row
    that names a debt and is wrong about it remains invisible; that is the question's property, not
    the instrument's.
15. **Who reads an entry the clause states?** — `cargo run -p conformance --bin entries`, seconds,
    over `ledger.toml`, the standard's own tables and the source roots (ADRs 0295, 0315, 0319). The
    only sweep that reads no reason at all. It exists for the refusal shape every other sweep passes:
    a row retires its refusal by naming a capability that arrived, and nobody asks whether the
    *entry* that turns the capability on was wired to it. Read the hits whose entry the row's own
    note does not name first.
16. **No corpus document does X** — a row's negative *evidence*, re-run rather than re-read (ADRs
    0403, 0405, 0548, 0563):

    ```sh
    cargo run --release -p pdf-model --example witness_census -- --pdfjs Collection Threads IDTree
    cargo run --release -p pdf-model --example absence_audit          # the structural half
    cargo run --release -p pdf-model --example operator_shape_census  # an *order* of operators
    ```

    `witness_census` asks each term of the file's raw bytes, of every object the cross-reference
    names including those inside object streams, and of every stream's decoded data; `absence_audit`
    re-asks the claim through the reader that would act on it, because a name being present is not
    the structure being present — **run both** (ADR 0403). `operator_shape_census` lexes content for
    a witness that is a sequence, skips inline image data, and prints how many content streams it
    does **not** reach. What a lexer counts is the clause's shape, not the program's behaviour: a
    question about what a refusal costs the page wants an instrument that interprets
    (`refused_segment_census` for §8.5.2.1). Three ways such a claim goes wrong: a stale count, a
    count contradicted inside one file, and **a population the sentence does not name**. The
    instruments take `--pdfjs`, the curated corpora or `--crawl` (ADR 0523). `absence_audit` holds
    one block per claim, which keeps each honest — the reader that would act on the entry is the one
    asked — and a claim whose falsification is a count and whose residue is a condition wants two
    blocks (ADR 0610).
17. **An erratum this tree *records* and then quotes the words it removed** — `cargo run --release -p
    spec-errata -- applied doc/*.pdf`, two seconds, over `ledger.toml`, every comment under
    `crates/`, `tools/` and `fuzz/`, and every Markdown block under `doc/` bar `doc/history/` (ADR
    0426). A row that records an erratum is not a row that has applied it, and a row that names the
    erratum looks maximally diligent, which is precisely why nobody re-reads it. Nothing is inferred:
    the writer names the erratum, and the `StrikeOut` and the `Caret` (joined by Table 172's `/IRT`)
    supply both sides. A correction quoting the retired wording is marked `[history]`;
    `doc/errata-read.md` is that shape end to end and is counted apart; a `#NNN` the collection does
    not carry is dropped and counted. Its vocabulary for a correction covers what an erratum puts
    there as well as what it removes (ADR 0440).
18. **A parent's claim against its own children** — `cargo run -p conformance --bin overstated`, a
    fifth of a second, over `ledger.toml` and nothing else (ADR 0475). A parent's assertion that an
    entry or a table *is read* against a descendant's denial; both sides are this project's own
    claims, so a contradiction is one whatever the standard says. It exists for the **overstating
    parent**, which names a thing the tree lacks under a row claiming the opposite of a debt, so both
    the seventh and fourteenth sweeps walk past it. Three rungs, closest first: the child denies the
    term itself; the child owns the term and denies reading it; the child's denial names another
    table or entry. Read the **unmarked** hits first; the noise it leaves is a partitive with no table
    to divide it.
19. **A page-list note overtaken by a later decision** — `cargo run -p conformance --bin overtaken`, a
    fraction of a second, over the tree's page-list notes and `doc/adr/` (ADR 0491). A page-list note
    is the doc comment above a `const NAME: [&str; N]` of corpus pages. **An ADR number is a date**:
    a gap between the newest ADR a note cites and the newest ADR naming one of the note's own pages
    is a decision taken after the note was last revised. All three rungs require a shared page. The
    cheapest way to stay off it is to cite your own ADR in the note you rewrite. A note read against
    every decision it prints goes into `overtaken::READ` with the newest decision the reading
    reached (ADR 1355); a note whose figures a later decision moved leaves by re-taking them with
    the instrument the note names, not by an entry.
20. **A figure a note quotes that the gate does not print** — `cargo run -p conformance --bin quoted
    -- <the oracle's log>`, under a second (ADR 0495). Trap 1's by-hand tell mechanised: a figure in
    the gate's vocabulary that no page of its own note carries, compared at the coarser of the two
    precisions. Under every hit it prints what the gate says instead. Noise: a note narrating its
    own correction, another instrument's table borrowing the gate's words, a range read as its first
    endpoint.
21. **The measure a note never names that its page fails on** — `cargo run -p conformance --bin
    unpriced -- <the oracle's log>`, under a second (ADR 0606), ADR 0497's sixth criterion made
    mechanical: a contradicted entry is an exemption from a *specific failing bound*. The population
    is the `CONTRADICTED` verdict and no other (trap 11). Two populations it names rather than
    counts: a page whose figure and bound print identically at two decimals, and a contradicted page
    in no note at all.
22. **A cardinal counting this tree's parts** — `cargo run -p conformance --bin parts`, a fraction of
    a second (ADR 0709), answered with the workspace's own membership. **A decay detector rather
    than a mistake detector**: *both backends* was true until a third rasteriser arrived. The noun
    follows the number immediately, and the form must presuppose the size (`both`, `neither`,
    `either`, or a cardinal under a definite article). Rung 1 is a crate the whole population
    depends on, where no pair can be meant. **A cardinal counting this tree's parts under a
    *departure* is usually a claim in the wrong place**: the fix is the sentence naming where the
    decision is made — `pdf-render`, by trap 2's rule — not a larger number.
23. **Which corpus a sentence counted over** — `cargo run -p conformance --bin undenominated`, ten
    seconds, because its right-hand side is the disk (ADRs 0751, 0753, 0754, 0758). The predicate: a
    sentence quantifies over a corpus and does not say which. Five rungs, closest first: a fenced
    invocation walking some of the corpora and not the rest; an absence or uniqueness naming no
    population in the ledger, then beside the code, then in a document; a count naming none; a
    numeral denominator no population on disk has. A claim that names a population is counted with a
    tally of which name answered it. What it cannot see — whether a claim is *true*, a denominator in
    the previous sentence, a claim counted in pages or glyphs, a recipe outside a Markdown fence — is
    stated in the module doc.
24. **What verb governs the sentence a row's debt rests on** — `cargo run -p conformance --bin
    permitted`, seconds, over `ledger.toml`, `doc/md/` and the standard's tables (ADR 0900). It reads
    a row that owes nothing and says so. Two halves: the note's quotations located in `doc/md/` with
    the verb read off the standard's own sentence, word by word; and every attributed `(table, key)`
    looked up, where a row all of whose named entries the standard states as optional is ADR 0896's
    shape. Beside each hit it prints ADR 0897's column, the clause's `shall` sentences outside its
    tables and NOTEs — a hit over a clause stating none is a status with nothing under it. Its *no
    modal verb* bucket holds three different things the flag cannot separate (a framing subclause,
    an argument the row wins beside a debt in unquotable prose, and a requirement written in the
    indicative), so it is read by hand and nothing moves on the flag (ADRs 0918, 0919). The noise is
    a note quoting the half of the clause it executes. **A `partial` row whose debt is a named
    constant can be measured in an afternoon**, and measuring it is what found ADR 0961's silent
    mesh bound.
25. **Whether a settled row's argument is still a document** — `cargo run -q -p conformance --bin
    departures`, a fraction of a second; `tools/state.sh departures` keeps the per-row lines (ADR
    1166). A `departed` row owes nothing, so every sweep walking the debt walks past it. The gate
    half is in `cargo test -p conformance`: a `departed` row naming an ADR with no file is
    `Problem::ArgumentMissing` (ADR 1119). The print is, per row, the deciding ADR the note's first
    sentence names and every later ADR citing it or the clause. **Nothing is scored**: the four
    shapes of expiry — a capability since built, a property of the output device, an exclusion since
    amended, an inconsistency on the argument's own terms — are looked for by whoever runs it (ADR
    1165).
26. **Whether the tree declares the item a doc comment names** — `cargo run -p conformance --bin
    names`, a second; `tools/state.sh names`; gated by `cargo test -p conformance --test names` at
    zero (ADR 1273). Resolution is the last two segments, looked for in the comment's own crate and
    the workspace crates it depends on. A relative or generic prefix, a dependency's path, and a
    prefix declared nowhere reachable are counted rather than listed.
27. **Whether a file that cites a clause is a file that clause's row names** — `cargo run -p
    conformance --bin cited`, seconds; `tools/state.sh cited` (ADR 1274). A row's `code` array decays
    in one direction: a round adding a reader cites the clause beside the code and forgets the row.
    Only `implemented`, `partial` and `departed` rows are read, and only files citing a clause three
    or more times. It ranks rather than fails, so that no round learns to drop a citation. A citation
    of a heading the ledger rows nothing under by design is counted per class (`cited::Unrowed`, ADR
    1355); every other unrowed citation is listed, and that count is zero on a clean tree.

Three more, unnumbered because they were built as instruments of their own:

- **A command-line flag a message names that no program accepts** — `cargo run -p conformance --bin
  flags`, `tools/state.sh flags`, gated by `cargo test -p conformance --test flags` (ADR 1213).
  Attribution outside a command span is by crate; its calibration test plants a flag no program
  accepts and is not optional (trap 13).
- **A note's own sentence about the row against the row's `status`** — `cargo run -p conformance
  --bin last_sentences`, `tools/state.sh last-sentences` (ADR 1249). A status word in backticks that
  is not the word in the field beside it; the top rung's noise is a sentence naming the status a
  pending question would move the row to.
- **`doc/todo/65`'s membership against the ledger's open rows** — a gate rather than a sweep, because
  both sides are lists: `cargo test -p conformance
  the_frontier_map_places_every_open_row_once_and_nothing_else`, `tools/state.sh frontier` (ADRs
  1237, 1250). A row that closes owes the map a deletion and a row that opens owes it a bucket.

### The gate's own ceilings

`FILE_ONLY_EVIDENCE_CEILING` (`implemented` rows) and `PARTIAL_FILE_ONLY_EVIDENCE_CEILING` (`partial`
rows) count rows whose `test` names a file rather than a test, and both are zero and asserted with
`==`: a row naming `file.rs` names something that passes whatever it contains. **A named test is a
reading, not a rename**: it is calibrated by planting the defect it guards and watching it fail.
What the plants teach:

- A defect can be refused at two independent points, and then one plant fails nothing.
- The plant has to be at the site the row's sentence names.
- A named test can be riding another entry's plant, and it can pass for the wrong reason (trap 27).
- Where the population is empty, the evidence is a fixture or it is nothing (trap 4's converse).
- An aggregate's evidence is one named test per child that states requirements, so an aggregate is
  only as good as somebody having asked what each child owes.

## Measured and declined

- **Parents *ahead* of their children** (the sixth sweep's inverse). This ledger's convention is that
  a parent row covers the clause's own prose and its children own theirs, so a parent `implemented`
  over a `partial` child is a true pair; its population was all noise. Run once to know that.
- **The eighteenth sweep's mirror** — a parent's denial against a child's assertion: a denial
  generalises where an assertion enumerates, and the population was noise (ADR 0481).
- **A parent restating a child's refusal and dropping its condition.** Both rows state one claim in
  identical words, so there is nothing to compare; the condition lived in the child's **code**.
  Revisit if a note ever states a refusal's condition in the refusal sentence itself.
- **An intra-row self-contradiction sweep.** The pieces are public (`overstated::parts`, `terms_in`,
  `is_an_assertion` against `unread::is_a_claim`), and the population is dominated by notes somebody
  already fixed, because ADR 0523 made a correction state the retired claim in words a sweep can
  find (ADR 0551, ADR 0560 section 5).
- **The general sweeps over `doc/adr/`** — mostly true statements about dated records, so only the
  fourth sweep reads it (ADR 0265).
- **Ranking rows by a cheap property of the note.** Eleven signals were scored against a labelled
  set of defect rows; none beat random by more than a ninth, the pairwise rare-sequence ranking
  among them (ADRs 0627, 0628). The defects are conclusions, and nothing ranks conclusions. The
  pairwise ranking still ranks *families* (ADRs 0551, 0560, 0593).

## The rules of practice

**Choosing what to read.**

- **The reading list is `git blame` order, quoted by rank.** A commit index is a property of the
  base, not of the ledger — `git log --reverse` linearises merges — so never carry one between
  rounds (ADR 0455).
- **Rank by blame, then read the row whose stated reason is a claim about this codebase rather than
  a claim about the standard.** A note quoting the standard was checked against it when written and
  the standard has not moved; a note describing the tree has been ageing since (ADR 0455).
- **A note that says the standard states nothing names where it looked.** That is a rule for
  *writing* a row, and the only kind that makes the next re-read cheaper.
- **A settled status is an argument, and the argument names rows.** Where a note cites another
  clause as precedent, the two statuses agree or one is wrong (ADR 0465).
- **The errata ranking**, which is the successor to ranking notes (ADRs 0627, 0628): rank each row by
  the errata annotations that fall on it whose issue number this tree names nowhere, reassemble the
  issue from every clause `emit` files it under, and read the issue whole. Its population is finite,
  it decays on use, and its hits are defect-shaped. Two limits: *named* is not *read properly*, and
  it is an errata ranking rather than a defect ranking. The recipe:

  ```sh
  # 1. the annotations, keyed to page and clause. Not committable: ADR 0187.
  cargo run --release -p spec-errata -- emit doc/ISO_32000-2_sponsored_EC3.pdf > /tmp/emit.md
  # 2. every issue number this tree names. Two greps, because neither is right alone — see below.
  grep -rhoIE 'Issues? #[0-9]+(,? (and )?#[0-9]+)*' crates doc tools fuzz \
    --exclude-dir=md --exclude-dir=pdf.js --exclude-dir=corpora \
    --exclude-dir=arlington-pdf-model --exclude-dir=safedocs | grep -oE '#[0-9]+' | sort -u
  sed 's/&#[0-9]*;//g' doc/errata-read.md | grep -oE '#[0-9]+' | sort -u   # the record's own column
  # 3. for each `## <clause>` heading in the emit, attribute its annotations to the nearest ledger
  #    row at or above that clause number WITHIN THE HEADING'S OWN FAMILY (same top-level clause or
  #    annex letter); keep live rows, drop the issues step 2 found and those carrying neither a
  #    StrikeOut nor a Caret, and rank by annotations. Count an informative annex's apart.
  # 4. rank a second time over EVERY row, whatever its status, and take the head of the two,
  #    preferring the settled row where they tie. Say which ranking the row came from.
  # 5. where the row counts have gone flat, rank the same annotations by ISSUE and take that head.
  #    Say which unit the head came from.
  # 6. count the issues whose every landing is on a row CLAUDE.md's exclusion list covers APART,
  #    and rank the rest. Check the head's table or figure against the clause that captions it in
  #    doc/md/ first: ADR 0712's placement rule files an annotation by its page's outline section.
  ```

  Step 2 is two greps because the prefixed search misses the bare numbers `doc/errata-read.md`
  records in a table column, and a bare-number search over the tree answers *recorded* from
  `doc/HAYRO_ISSUES.md`, which lists another project's issues, and from numeric character
  references such as `&#124;`. **An issue number written anywhere but `doc/errata-read.md` carries
  the `Issue #` prefix.** Step 4's ordering is an argument: a live row's count refines a debt the
  ledger already declares, while a settled row's count ranks a *claim*, and the falsest row the
  ledger can hold is an `implemented` over a requirement nobody read. An erratum read only far
  enough to break a tie stays in the population on purpose. The single-issue and multi-issue line
  parses of `/Subj` both reproduce on a working run. Not built as a sweep: `emit`'s output derives
  from documents this project may not redistribute.

**Counting.**

- **A note stating a count over the corpus names the command that produces it, or the round that
  writes the count adds one** — the counted-claim rule, the ledger's half of what `tools/state.sh` is
  for the instruction files. A note whose count already has a command (a gate's ratchet, a census)
  owes the command's name, not a new census: look for the gate before writing one.
- **Say which corpus.** *The corpus* is several populations, and a claim is only true against the
  one it was measured over (the sixteenth and twenty-third sweeps).
- **A count over the corpus is a claim about a walk as much as about the world.** Re-derive the
  population with the count, walk into each object's nested structure (an action, an annotation or a
  resource dictionary may be written inline), ask what the walk cannot see before believing a zero,
  and probe a zero with the crudest instrument there is — `grep -l` for the literal name over the
  bytes.
- **Plant a witness stating every construct before believing any zero**, and point the same rule
  backwards: before believing what a census says about a wider population, run it over the
  population the old sentence was measured over and check that it reprints the old numbers (ADR
  0523).
- **A negative measured before the crawl is a negative nobody has measured**: read such sentences
  with the control run as well as the crawl run, because the old sentence is usually right about its
  own population (ADRs 0490, 0493).
- **An instrument says what it cannot see.** A population an instrument cannot reach is printed
  under every run — the streams a lexer did not enter, the documents a walk did not open — because
  an unseen population reads exactly like an absent one.
- **A count comes before the work**: the round that takes a clause owes the count first. An entry no
  file states is a clause with no witness, and one with a witness is work with a page behind it.

**Correcting.**

- **A correction recorded in a todo file is not a correction**: make the change in the same commit
  that finds it.
- **When a note is corrected, its `code` and `test` arrays are corrected in the same edit or they are
  not corrected at all.** A note argues about three children and cites one child's test; the prose
  reads as maintained and the evidence is whatever the row was first written with.
- **When a round fixes a defect a family shares, the population it fixed is the arms of the
  function, not the rows it happened to be reading** — enumerate the call sites.
- **An entry list lives in one row**: point at the neighbouring row instead of repeating its list,
  which doubles every sweep's standing noise on it.
- **Run the sweeps before your own edit as well as after it, and account for every number that
  moves.** A level is one integer from a program that does not know what the round is trying to say,
  and it reviews the sentence the round is about to add for free. Levels are not gates: they move for
  good reasons every round (ADR 0485). The finding's sentence is never rewritten to dodge a sweep —
  that would be the instrument choosing what the ledger may say (ADR 0490 section 6).
- **Taking the *before* half of a run.** Every sweep binary resolves the tree from
  `CARGO_MANIFEST_DIR`, not from the working directory, so a binary run inside a copy of the base
  measures the worktree. Give the base commit a checkout of its own and build the sweep binaries
  inside it (`git worktree add --detach <dir> <base>`, then `cargo build -p conformance --bins
  --target-dir <its own>`), with `doc/md/`, `doc/*.pdf` and the two submodules symlinked in from the
  main tree; remove the checkout *and* its build directory afterwards. Checking files out at the base
  in place restores the last **commit** and throws away every edit since the round's own checkpoint.

## What is still owed

- **The fifth sweep's second population**, still a by-hand run: every `viewer-core` `Command`,
  `Query`, `Answer`, `Event` and `Edit` variant against the crates that speak it — the run that found
  `Query::Find` and `Query::LogicalSelection` reaching no program.
- **The sixth sweep as a program.** It is four lines of Python and has never printed noise, which is
  why it is the one still a description.
- **A cited precedent's status against the citing row's** — the population is every row whose note
  names a clause that has a row, and the hit is a disagreement between the two statuses. The eighth
  sweep's question one level up, about a claim rather than a pointer.
- **A note's prose against its own `code` array** — for each row, does its note name a source file
  its `code` array omits? The twenty-seventh sweep reads the other direction (a citing file the row
  omits); this one reads the row's own sentences.
- **An artifact census.** No command counts the documents whose page one marks an artifact (a `BDC`
  tag read by the interpreter, broken down by Table 363's `/Type`); `witness_census` counts names.
  The same pass would settle the per-site `/AF` counts §14.13, §14.13.3 and §14.13.6 each state.
- **§12.8.2.4's transform, named rather than built**: the recognition half is the transform method
  `FieldMDP` with Table 259's `/Action` and `/Fields` beside `UsageRights`; the validation half is
  §12.8.2.2.2's and needs the signed revision reconstructed.
- **`absence_audit`'s blocks**: one block per claim, and there are always more — a flag's bit, a
  value's range, a group's colour space, a producer's arithmetic. A round adding one adds a block,
  not a heuristic.
- **`--bin permitted`'s largest bucket**, the rows quoting nothing the conversion holds: a hit there
  is a misquotation, a quotation of a document other than ISO 32000-2, or a conversion artefact, and
  those want different answers. `--bin quotations` already separates the first from the third, and
  nothing has crossed the two lists.
- **Whether `--bin tables` wants ADR 0760's designation rule**, which is open.

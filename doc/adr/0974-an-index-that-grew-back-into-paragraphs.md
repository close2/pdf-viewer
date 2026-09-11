# ADR 0974 — An index that grew back into paragraphs, and one rule written down six times

Date: 2026-09-11 (session 967)
Status: accepted
Extends ADR 0232 (the handover split by *topic*), ADR 0281 (the counts taken out and the split by
*reader*) and ADR 0428 (the split by *what the round is doing*). This one is the fourth compaction
of the same material, and it is the first that had to argue mostly about **what not to touch**.

## Context

The project owner asked for a pass over "our documentation and instructions which every agent has
to read", on the expectation that cruft had accumulated. The corpus is the twelve files
`tools/round.sh` prints and `doc/HANDOVER.md` indexes, plus `round.sh` itself: **6317 lines and
87 806 words that a round reads before it does anything.**

The three previous compactions each ended with a sentence about the risk they were taking, and each
was right. ADR 0232's was "a round that needed something and could not find it"; ADR 0428's was "a
round that needed a trap and did not open the group it was in". Neither has been recorded as
happening. What *has* happened, four times now, is the other thing: **a file that was reduced to an
index grows its paragraphs back**, and a claim nobody reads is a claim nobody checks.

So the round's first job was to decide what may legitimately come out, ahead of looking at any
particular paragraph, and the second was to apply exactly that and nothing else.

## Decision

**Four headings, and every removal is classified under one of them in the record. Anything that
fits none of the four stays exactly as it was.** The headings are not new — each is a rule this
project already states — and naming them ahead of the diff is what stops a compaction becoming an
edit for taste.

1. **Duplication.** `doc/HANDOVER.md`: *"A lesson lives here exactly once: in a trap if it changes
   how you write code, in `doc/habits.md` if it changes how you work."* Two documents stating one
   thing is how they drift, and ADR 0232 §4 has the archetype — a second copy of the gate sequence
   that said 1369 tests where the gate printed 1371.
2. **A counted fact.** `CLAUDE.md`: *"A fact that can be counted is not written down. What is
   written down is the command that counts it."* Binds counts, rates, gate results, "N of M",
   session numbers and dates. It does **not** bind the standard's own numbers, nor a figure that is
   the *evidence for* a trap.
3. **Session narrative.** A round's story belongs in `doc/adr/` and `doc/history/` — and only where
   the ADR actually contains the reasoning, which this round verified by opening them.
4. **A claim that has decayed.** `CLAUDE.md` says a claim about the specification decays; so does
   one about a vocabulary, an architecture, a capability or *this tree's own parts*. A stale claim
   is **corrected with a sentence saying what replaced it**, never silently dropped.

### 1. The todo index had grown back into paragraphs, and its own preamble says so

ADR 0281 §4 made `doc/todo/README.md`'s index **one line per item**, on the argument that each
file's own header block carries its status, witnesses, clauses and code, and *that* is the
authority — "a summary here that restates them is a second copy to keep in sync, which is how this
index came to say a clause was ignored eighty-seven rounds after it was implemented". That sentence
is still in the file's preamble. Forty rounds later the cells were up to **1663 characters apiece**
and carried ADR lists, session numbers, percentages, page counts and per-round chronicles.

It is the cleanest case in the corpus because the file **argues against its own body**. Trimmed
back: **5650 → 3123 words**, at 123 lines either way, because the growth was inside single long
table cells and no line count could ever have shown it.

**What made it safe rather than a judgement call**: every ADR cited in a cell was checked against
the item file it summarises. Fifty-three rows, and the citation was present in the file in
fifty-one; the two exceptions (ADR 0882 in `01`, ADR 0316 in `32`) were read and are pointers to an
argument the item file makes in its own words, not arguments the cell held alone. **A cell keeps
what a reader needs to *choose* an item** — what it is, whether it is open, blocked or effectively
done, and any pointer to a sibling item — and gives up the account of how it got there.

### 2. The change → gate map had become the test suites' documentation

`doc/todo/02` §2's map exists to say: a change in X is under Y, so run Z. Two of its cells had
become 991 and 407 words — **12.7% of a file every round reads**, for two crates — restating what
each corpus walk asserts, clause by clause.

Every one of those walks opens with a `//!` header that says the same thing at greater length and
*next to the code*: `read_corpus.rs`, `write_corpus.rs`, `foreign_corpus.rs`, `optimize_corpus.rs`
and the rest each state the layers of RFC 0002 §9 or RFC 0003 §4 they are, the clauses they hold
their output to, and what they do when the programs they need are not installed. That is
duplication of the archetypal kind, and the map now names the lines and points at the headers:
**1563 → 539 words** across the three cells, with the trap-10 distinction about `--bins`, the
cost-floor-is-a-count rule and the `tools/bounded.sh` obligation all kept, because those are about
*running the gate* rather than about what it checks.

The same §2 lost the second copy of trap 23. The fuzz block restated the workspace mechanism, the
thirty-three clippy findings and the two rustfmt diffs that the trap owns; what stays is the
operative rule, the `workspaces.rs` derivation (which is about *this section's own lines*), and the
fourteen rounds in which the targets did not compile — which is §2's own incident and is why the
line is in the every-round sequence at all. Its opening sentence had been orphaned by an earlier
edit and read "**They did not**, for fourteen rounds" four paragraphs after its antecedent.

### 3. Session bookkeeping came out of the capability list

`doc/state-of-play.md` says of itself that "every sentence below is something the program does
today". Seventeen of its sentences dated a capability to the round that built it — "since the
seven-hundred-and-fourth session all three windows have the same six panels" — which is ADR 0281
§2's bookkeeping exactly: *an instruction file citing a session number to **date** a claim moves to
`doc/history/`; one citing an ADR for an **argument** is a pointer and stays.* Every one of the
seventeen had its ADR beside it, and every ADR stayed.

### 4. Four decayed claims, corrected rather than dropped

- **`doc/environment.md` named `tools/conformance`'s build script.** That crate has never had a
  build script in any commit of this repository — which is trap 25's own finding, recorded in
  `tools/round.sh` and never carried back to the document that taught the hazard. The bullet now
  says which script actually bakes the path (`crates/pdf-sandbox/build.rs`), says what it used to
  say, and points at the derived check.
- **`tools/round.sh` listed each trap group's numbers**, and all five lists were short: `pixels`
  said "1, 2, 6, 12b" where the group holds 12c and 14 too, `oracle` missed 26, `parsers` missed
  38, `loop` missed five of its six, `instruments` missed nineteen. The script names the group's
  *subject* now and never its numbers, with a comment saying why — `doc/HANDOVER.md`'s table is
  where a group's numbers live, and a second copy of them was a copy nothing checked.
- **`doc/HANDOVER.md`'s trap index was missing trap 39.** `doc/todo/02` §6 warns about exactly this
  — a new trap owes *two* entries, the index row and the group table's number — and records traps
  14, 30 and 31 reaching the index and not the table. This is the same defect in its mirror
  direction, found by cross-checking the two tables and the five files mechanically.
- **`doc/todo/02` §5 still called a binary `pdf-retrieve`.** The paragraph immediately above it is
  about three programs renamed to `quorra-*` and an `install` loop that went on copying the old
  names; the prose beside it had the same defect and nothing noticed. Corrected, with the
  observation that it is that paragraph's own subject happening to prose instead of to a shell
  loop.

### 5. One rule written down six times

The trap-numbering rule — *every trap keeps its number, because `crates/`, `tools/`, `ledger.toml`
and dozens of ADRs cite them and an ADR is not edited to follow a file that moved underneath it* —
appeared in full in `doc/HANDOVER.md` and in all five `doc/traps/` headers. `HANDOVER.md` is the
index and owns it; the five headers now say that in one line. Same for `doc/HANDOVER.md`'s copy of
`doc/habits.md`'s "each was paid for once" paragraph, and for `doc/habits.md`'s second copy of the
`git stash` incident — which had already drifted, dating the same event to the
five-hundred-and-twenty-third session in one file and the five-hundred-and-twenty-fourth in the
other. Neither copy can now say which is right, so the duplicate lost its date and
`doc/environment.md` keeps the incident.

### 6. Two figures, and one heading in the wrong place

`doc/traps/instruments-and-reports.md` said "the corpus gate is 2 s in release". ADR 0428 §6 took
that exact figure out of `doc/todo/02` as one that had drifted, and replaced it with "seconds in
release"; the trap file's copy was never found. It says "seconds" now and names the command.
`doc/environment.md` carried a build-directory sweep paragraph — inside a bullet about commit
messages — whose every claim is `doc/todo/02` §5a's and whose figures (131 GB of 158, 9.4 and 4.7)
are `tools/state.sh disk`'s.

And that trap file's "Things worth knowing" was a `##` heading sitting between trap 39 and trap 34,
so four traps were nested under a section that is not about them. It is at the end now.

## What was deliberately kept, and why

This is the more important half of the record, because the next compaction will meet the same
paragraphs and should not have to re-decide them.

- **`doc/habits.md` in almost its entirety** — 1119 lines, the largest file in the corpus. Every
  entry is a lesson with the anchor that makes it checkable, which is the file's own stated
  contract, and not one of them is derivable by a command. Two duplications came out and nothing
  else.
- **Trap 9's nine mechanisms in `doc/traps/oracle-and-references.md`**, and the follow-on paragraph
  under several of them that says what a later measurement does and does not license. It reads like
  accretion and it is not: each says *this bullet is not a rule you may apply*, which is the part a
  round would otherwise get wrong.
- **The four false-failure witnesses under `doc/todo/02` §2's quiet-machine rule.** Four incidents
  for one rule looks like three too many; they are four different shapes — an inflated clock, a
  contended reference giving a *wrong answer* rather than a late one, a corpus walk's own wall
  clock, and a wait predicate that matches itself — and dropping any of them drops a shape.
- **Every duration in §2 and §5a** ("twenty seconds", "half a minute", "about three minutes"). They
  look like gate figures and they are budgeting hints; they are already in the deliberately-vague
  form ADR 0428 §6 chose, and a round cannot quote one instead of running a gate.
- **`doc/state-of-play.md`'s structure.** It is a capability list written as narrative, and turning
  it into a plain list is a *rewrite* rather than a compaction. The session-dating came out; the
  prose did not.
- **`doc/HANDOVER.md`'s trap index, and the trap titles repeated in it.** A mechanical duplicate
  scan flags 131 shared ten-grams between the handover and one trap file; they are the index rows,
  and the index is what resolves the several dozen citations of a trap by number in one hop
  (ADR 0232 §2). An index that does not repeat is not an index.
- **`doc/environment.md`'s working agreements**, every one of which is an incident with a rule
  attached and several of which record a round harming a *sibling*.

## Consequences

**6317 → 6286 lines, 87 806 → 83 940 words.** The line count is nearly the whole point: 2527 of the
3866 words removed were inside four table cells, and **no line count, and therefore no diff
statistic anybody reads, could have shown that an index had doubled.** A future compaction should
measure this corpus in words.

**The instruments say nothing got worse.** `--bin pointers`: 185 absent before and 185 after, 14
undefined symbols before and after (live fell 7624 → 7619, which is five live pointers removed with
the prose that carried them). `--bin quotations`: 3708 verbatim and 49 diverging before and after in
documents, 1867 and 5 in ledger notes; what moved is two phrases the sweep counted as *unrelated*,
which is its name for a quoted fragment that is not a quotation of the standard.

**The structural question the owner's brief asked — is the reading order right?** Two proposals,
neither taken this round because both are moves rather than removals and both want the owner's or a
later round's word:

- **`doc/state-of-play.md` is not in the every-round set and `doc/todo/README.md` is**, which is
  right. But `doc/todo/02` §2's gate sequence and its map are read in full by every round, and the
  map's *first row* answers most rounds: "these seven crates are under everything, so run
  everything". The rest of the map is a lookup. It would read better as the rule, then the table
  under a heading a round can skip.
- **`doc/habits.md` is 1119 lines behind one index entry**, and its six sections are opened one at a
  time — but `tools/round.sh` names the file, not the section, for three of its ten kinds. Splitting
  it into six files the way `doc/traps/` was split by ADR 0428 is the same argument one file over.
  It is not done here because `doc/habits.md` is cited by name from ADRs and from `CLAUDE.md`, and a
  split owes the same heading-preservation work ADR 0232 §2 did.

**The risk this round takes on** is the one its own method creates: a classification is a judgement,
and four headings make it *look* mechanical. The mitigation is that every removal is named in the
report under its heading, and that the four cases where something looked removable and was kept are
written above rather than left as absence.

## A postscript from the round that merged this

The report of this session named **two** live duplicate todo numbers, `36` and `47`. There are
**three** — `46` is both `46-a-wheel-tick-that-interprets.md` and `46-the-kernel-floor.md`. The
miss is instructive rather than embarrassing: the two that were found were found by reading, and
the third by `ls doc/todo/ | grep -oE '^[0-9]+' | sort | uniq -d`, which is one command and cannot
miss any. It is the same lesson this ADR draws about the five short trap lists in `tools/round.sh`
— **a list a person maintains by reading drifts from the thing it lists**, and the fix is to make
the command the authority.

**And the finding was in the report and in no file.** A round's report is read once, by the session
that merged it; a finding that lives only there is lost the moment that conversation ends. This one
is now in `doc/todo/README.md`, beside the rule it violates, which is where a round looking for a
free number will meet it.

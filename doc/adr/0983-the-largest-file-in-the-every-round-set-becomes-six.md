# ADR 0983 — The largest file in the reading set becomes six, and a map states its rule first

Date: 2026-09-11 (session 972)
Status: accepted
Extends ADR 0232 (the handover split by *topic*), ADR 0281 (the counts taken out, and the split by
*reader*), ADR 0428 (the split by *what the round is doing*) and ADR 0974 (the fourth compaction,
which classified every removal under one of four headings). This one takes the two structural
proposals ADR 0974 wrote down and did not act on.

## Context

ADR 0974's consequences end with two moves it would not make, because both wanted a later round's
word rather than a compaction's licence:

> `doc/todo/02` §2's gate sequence and its map are read in full by every round, and the map's
> *first row* answers most rounds: "these seven crates are under everything, so run everything".
> The rest of the map is a lookup. It would read better as the rule, then the table under a heading
> a round can skip.

and

> `doc/habits.md` is 1119 lines behind one index entry, and its six sections are opened one at a
> time — but `tools/round.sh` names the file, not the section, for three of its ten kinds.
> Splitting it into six files the way `doc/traps/` was split by ADR 0428 is the same argument one
> file over. It is not done here because `doc/habits.md` is cited by name from ADRs and from
> `CLAUDE.md`, and a split owes the same heading-preservation work ADR 0232 §2 did.

Both are right, and the second names its own price. `doc/habits.md` is cited **235 times** across
`doc/`, `crates/`, `tools/` and `CLAUDE.md`, from 162 files — and not one of those citations carries
an anchor. They name the file, and several name a *section by its title*: `doc/habits.md`
*Measuring*, its *ledger section*, `"Tests, gates and reports"` in `tools/round.sh` itself.

## Decision

### 1. `doc/habits.md` becomes an index over six files, and every heading stays a heading

`doc/habits/` holds one file per section, named by subject the way `doc/traps/` is:
`reading-the-specification.md`, `judging-against-other-implementations.md`,
`tests-gates-and-reports.md`, `the-ledger-and-claims-about-this-tree.md`, `measuring.md`,
`code-bounds-and-dependencies.md`. Each takes **all** of what its section held, verbatim, with a
header block in `doc/traps/`'s style — a status line, a `Read by:` line, and the pointer back.

**Not one habit was removed, reworded or reordered.** The check was mechanical rather than
asserted: every non-blank line of the old file from its first section heading to its last line,
sorted, against the six new files' non-blank lines, sorted. The only differences are the six
`### ` headings, which became the files' titles, and the six new header blocks.

**`doc/habits.md` keeps the six headings**, each followed by the path of the file that holds it.
That is ADR 0232 §2's rule applied unchanged: a citation naming a section by its title still
resolves, in one hop, without an ADR being edited to follow a file that moved underneath it.

**What was re-pointed, and what deliberately was not.** `tools/round.sh` names the section file
directly for its `oracle`, `instruments`, `clause` and `measure` kinds — that is the whole point of
the split, and `clause` gains a line because it was already opening two sections.
`doc/HANDOVER.md`'s habits table and its three routing rows link the files. **Everything else
still resolves through the index and was left alone**: 235 citations is not a re-point, it is a
rewrite of the tree's prose, and the index exists precisely so it is not needed.

### 2. `doc/todo/02` §2's map is a rule, then a lookup

The map is now six numbered rules and a table under a `#### The lookup` heading, and the first line
tells a round to read the rules, apply them, and open the table only if none of them answered.

The six are not new. Five were already in the section, in three places: the core-lines paragraph,
the three bullets headed *Three things the map does not license*, and two of the table's own rows —
the seven crates that are under everything, which is the row most rounds need, and the
documents-only row. **Those two rows are gone from the table and are rules 2 and 6**, with every
clause of their text kept; the lookup says so, so that it does not read as a table with holes in it.

**Rule 2 keeps the words "first row" in it.** About fifteen files under `doc/history/` record a
round's gate choice as *"`pdf-model` is the change→gate map's first row, so the whole sequence
ran"*, and that is the idiom this project actually uses. A restructuring that made the idiom
unfindable would have cost more than the ordering bought.

The §2 opening paragraph stopped restating what those rules say and points at them instead, which
is the one duplication this half removed.

### 3. What this cost, honestly

**The corpus got longer, not shorter, and that was the brief.** 6307 → 6438 lines and 84 210 →
85 395 words, over the thirteen files `tools/round.sh` prints and `doc/HANDOVER.md` indexes plus
the six new ones. `doc/todo/02` gained about a hundred words turning two table cells into prose;
`doc/todo/README.md` gained about three hundred writing down a finding (ADR 0993); the six habit
files gained a header block apiece. Two duplications came out under ADR 0974's heading 1 and are
named below. A reordering is not a cut, and a round that reported one as the other would be
measuring the wrong thing.

What *did* fall is the thing the split is for: a round that opens a habit file reads between 122
and 398 lines where it used to read 1130.

### 4. The two removals, each under ADR 0974's headings

- **Duplication.** `doc/running-the-viewer.md` carried the session-142 incident — a round reporting
  "still lags" against a binary three hours and six commits old, one of which was the 40×
  page-turn fix — which `doc/todo/02` §5 owns and argues its whole cadence from (ADR 0428 §4). The
  sentence that is running-the-viewer's own, *`cargo test` only ever builds the debug binaries*,
  stays; the incident is now a pointer. Two documents stating one thing is how they drift.
- **Session narrative.** `doc/todo/02`'s core-lines paragraph dated one of its two cost floors to
  the session that added it. ADR 0281 §2: a session number that *dates* a claim is bookkeeping.
  The ADR beside it stayed, because that is a pointer to an argument.

## What was deliberately kept, and why

- **Every habit.** All six sections moved whole. The temptation a split creates is to tidy while
  carrying, and the mechanical line-for-line check above exists so that a reader of the diff can
  see that nothing was.
- **`doc/habits.md`'s six headings**, even though the file is now forty-five lines and each
  heading is followed by a single line. They are load-bearing: they are what 235 citations resolve
  through, and an index whose entries are not the names people cite is not an index.
- **The whole of the §2 lookup's remaining seven rows.** They read like accretion — the
  `viewer-*` row is one paragraph inside one table cell — and they are not: each names a gate a
  change can reach and a trap that reaching it springs. The ordering change was asked for; a cut
  was not.
- **`doc/running-the-viewer.md`'s `p`, `t` and panel paragraphs**, which restate what
  `doc/state-of-play.md` says the program does. They are not a second copy of a *claim*: one says
  what exists, the other says which key produces it, and a person at the keyboard needs the second.
- **The four ledger habits sitting at the end of *Measuring*.** The `/CheckSum` entry deferred for
  cost, the fourth sweep shape, sweeping for the reason's shape, and an `inapplicable` row waiting
  for a capability are all about a ledger row's reason going stale, and they are in the measuring
  file because that is where the accretion put them. **The split is what made it visible** — a
  section 392 lines long has no visible end, and a file does. Moving them is a move and loses
  nothing, but it is a re-classification rather than a compaction, so it is written down in
  `doc/habits.md` for whoever takes it rather than done here on this round's judgement.

### 5. One habit was handed over and placed

ADR 0984 §5 ends by flagging a lesson for this session, "which owns that file this round": a
quotation in a ledger note is checked by a *report* rather than by a gate, so a paraphrase wearing
quotation marks lives there until somebody reads the report. It is in
`doc/habits/the-ledger-and-claims-about-this-tree.md` with both of its anchors — §7.6.4.1's two
hosts quoting a `should` as a `shall`, and twenty-three Annex F rows arguing from a phrase that is
ISO 32000-1's vocabulary. **That hand-over is the arrangement working**: a sibling round finds a
lesson, writes it in its own ADR, and names the round that owns the file it belongs in, rather than
editing a file two rounds are in.

## Consequences

`doc/habits.md` 1130 → 45 lines, 16 149 → 294 words; the six files hold 1163 lines and 16 555 words
between them, header blocks included. `doc/todo/02-every-round.md` 688 → 710 lines.
`tools/round.sh` 334 → 338. `doc/HANDOVER.md` unchanged in words and two lines longer.

**The instruments say nothing got worse.** `--bin pointers`: 185 absent before and 185 after, 14
undefined symbols before and after. `--bin quotations`: 3709 verbatim and 49 diverging before and
after, in six more documents; one *unrelated* quotation went with the sentence that quoted
`doc/HANDOVER.md`'s own section title. The pointer sweep walks `doc/` recursively and the quotation
sweep reads every Markdown under it, so the six new files are in both populations rather than
outside them — which was checked in `prose::documents` before the directory was made, because a
split that hid a thousand lines from the sweeps would have been the worst possible outcome.

**The risk this round takes on** is the one ADR 0428 named for the traps and nobody has recorded
happening: a round that needed a habit and did not open the file it was in. The mitigation is that
`tools/round.sh` now names the file for four of its ten kinds rather than naming a file with six
sections in it, and that `doc/HANDOVER.md`'s table says what each holds.

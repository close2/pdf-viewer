# 593 — A round reads what it is about

The project owner measured a round and the number that decides everything else is that
`doc/todo/02` §2 is **about eight per cent of it**. The gates were the thing four sessions had
optimised; what nobody had measured is the other ninety-two per cent, which is what a round
*reads* before it can start and which instruments it *chooses*. So this round split the two
documents every round reads whole, grouped the traps by **what a round is doing** rather than by
the order in which they were discovered, wrote §2's change→gate map, changed §5's cadence, and
built `tools/round.sh`.

Date: 2026-08-18.
ADR: [0428](../adr/0428-a-round-reads-what-it-is-about.md).
Todo: [43](../todo/43-the-projects-own-turnaround.md), which gains the denominator it was missing.

## What a round now reads

| | before | after |
|---|---|---|
| `doc/HANDOVER.md` | 882 | 125 |
| `doc/todo/02-every-round.md` | 454 | 404 |
| **the every-round pair** | **1336** | **529** |

The rest is one hop away and reachable by what the round is about: `doc/state-of-play.md` took
"Where we are" verbatim (209 lines), `doc/traps/`'s five files took the traps and the standing
facts about the machinery each is about (542 lines between them), and `doc/todo/01` took §4's
sweep catalogue, which §4's own first sentence already called "the reading".

## The proof that nothing was deleted, because that is the one unrecoverable mistake

Asserting it would have been worthless, so it was checked. Every non-blank line of the old
handover's three large sections was searched for **verbatim** across every Markdown this project
maintains:

| section | lines | not found verbatim |
|---|---|---|
| Traps | 405 | 1 — the section heading, re-titled |
| Things worth knowing | 82 | 0 |
| Where we are | 195 | 1 — the section heading, re-titled |

**Fifteen traps before, fifteen after, and the same fifteen numbers** — 1, 2, 3, 4, 5, 6, 7, 8, 9,
10, 10a, 11, 12, 12a, 12b. At paragraph scale, 84 of the old file's 114 blocks are verbatim
somewhere in the new set; the other 30 are two re-titled headings, two tables that were extended
rather than moved, four merges, and pointers to a file that states the same thing.

**The four merges, each a sentence that was already somewhere else word for word**: the todo bands
(`doc/todo/README.md`'s sorting table), the two tracks (`doc/todo/02` §1, whose version also
carries the band numbers), the closed-by-decision list (`doc/todo/README.md`'s, which gained the
two items the handover had and it did not), and the block-summary rule (`doc/history.md`'s own
preamble, which had stated it all along).

## What the map says, and what it may not do

Derived from the crate graph and from what each gate reads. `pdf-render`, `pdf-syntax`, `pdf-font`,
`pdf-model`, `pdf-spec`, `pdf-sandbox` and `render-cpu` are under **everything**, because every
corpus-scale gate rasterises with `render-cpu` through `pdf-model`. `render-quorra` is under its
own gate; **`render-gpu` is under no gate at all**, which the map says out loud because a round
touching it should know that the corpus did not cover it. The host crates are under no corpus gate,
`viewer-core` is under the two censuses, and a documents-only change runs the core plus the
conformance gate — which is what this round ran.

The two rules that make it safe are the ones that keep it from decaying: **the full sequence every
fifth round, and on any round that can change a pixel**, and the merge paragraph untouched.

## `tools/round.sh`, and the four checks it found true

It says the session, the reading list for the kind of round, which gates that kind needs, and
whether the fifth-round obligations are owed. Then it asks the four questions a round has actually
got wrong here. On this round's first run it found two of them red: `target/`'s binaries older than
`HEAD` — which §5 then fixed — and eight compiled build scripts naming worktrees that no longer
exist, of which **none is the newest for its crate**, so the check reports them as other rounds'
rather than as this one's failure. That distinction is the difference between a check somebody
reads and a check somebody learns to ignore.

## The corpus `cmin` §2 had asked for and no round had spent

`doc/todo/02` §2 has said for many rounds that one `cargo fuzz cmin page` would take
`fuzz/corpus/page` down to the distinct-coverage set, and that "it costs one more merge to do and
no round has spent it". Session 592 spent fifty minutes inside that merge and got nothing back.
This round started it first and let it run beside the documents. It took three quarters of an hour
and it worked: **40 089 files and 4.1 GB became 9 169 files and 653 MB**, and the reduced corpus
was then run — 9 033 seeds, `cov: 37 932`, `ft: 206 585`, no crash, timeout or OOM — against the
`cmin`'s own `37 927 new coverage edges; 206 645 new features`. Coverage kept, three quarters of
the files gone.

**And it bought less than §2 predicted, which is the finding rather than the number.** That
sentence expected "an hour's job rather than an afternoon's". A fork-mode start over the reduced
corpus is about a *third* of the same pass over the whole one, where the file count fell to a
*quarter* — because what `cmin` keeps is the seeds with distinct coverage, and those are the large
slow documents. Its own execution rate says it: 256 a second at the start of the merge, 14 at the
end. §2 carries the correction.

## One edit that looks like a rule broken and is not

`doc/history.md` is in this round's diff, and `doc/todo/02` §6 says a round does not touch it. What
changed is nine words of its *preamble*, which pointed at `doc/HANDOVER.md`'s Traps for "every
durable lesson" and now points at `doc/traps/`. No row was added, no row was edited, and the table
is still closed at 445. The reference check is part of the work rather than a courtesy — ADR 0232
established that and ADR 0281 repeated it — and a pointer left dangling by a move is the thing that
rule exists to prevent. Nine documents were repointed the same way.

## Found and not taken: a second file still carrying a gate's numbers

`doc/ledger-and-claims.md` holds a status table with a `rows` column, and it disagrees with what
`cargo test -p conformance` printed this round in five of its seven cells — one of them the row
whose whole point is that it is zero. It is the same shape ADR 0281 took out of the handover, one
file over, and the fix is to drop the column and keep every row's *description*, which is where the
argument lives. Not taken here because this round's brief named two files and the table's prose
carries numbers of its own that want reading rather than deleting.

# 2110 — The reading set is compacted, and each claim in it checked against the tree

Session 1637, batch ninety-nine's slot 7, a docs round. Status: **accepted** and **applied**.
Asked by the project owner on 2026-10-20: "go through the md files, which are read by every
subagent. Can we compact them? Is all information still valid?" Context: ADRs 0281 (a counted fact
is a command), 1023 (a correction is applied, not appended), 1036 and 1639 (the trap groups and
the every-round lists), 0983 (the habits split), 1005 §2; `doc/reviews/984` Finding 2; ADR 2111
(principle 2 adapted as A25 answered, found by this round).

## 1. What a round reads, measured

`tools/round.sh` decides it: `CLAUDE.md` (which is also in every agent's system prompt), `doc/todo/02`
section 0, the rule block opening `doc/environment.md`, `doc/traps/every-round.md`,
`doc/habits/every-round.md`, then two or three files of its kind. **The brief's premise that a
round reads the trap README and the five group files at launch did not hold**: those are opened
by kind or by a citation (ADR 1639), so they are the largest files of the set and not the largest
part of the launch.

Tokens were measured by printing each version whole in this round's own session and reading the
growth of the next request's input from the transcript (`usage`: input, cache read and cache
creation summed), about 80 tokens of each figure being the call itself:

| what | before | after |
|---|---|---|
| `CLAUDE.md` | 12 582 tokens | 10 454 tokens (−17 %) |
| section 0, the rule block, the two every-round files | 6 371 tokens | 5 832 tokens (−8 %) |
| the every-round set together | 18 953 tokens | 16 286 tokens (−14 %) |
| first request of batch ninety-nine's rounds 1631, 1632, 1633 | 40 963, 41 060, 41 087 tokens | not measurable until a round launches from the merged `CLAUDE.md` |

Words, `wc -w`, before → after: `CLAUDE.md` 5 385 → 4 688; section 0 682 → 569; the rule block
751 → 751 (it gained the wait rule section 0 repeated, and section 0 now points at it);
`doc/traps/every-round.md` 884 → 841; `doc/habits/every-round.md` 510 → 522; `doc/HANDOVER.md`
1 901 → 1 671; `doc/environment.md` 8 941 → 6 449; `doc/traps/README.md` 16 986 → 13 168; the
five group files 54 505 → 54 191; `doc/habits/measuring.md` 18 410 → 18 010; `doc/habits.md`
331 → 239; `doc/habits/the-ledger-and-claims-about-this-tree.md` 6 443 → 7 064 (four habits
moved in); `doc/todo/_brief-template.md` 891 → 891. The set, 115 869 → 108 303 words (−6.5 %). The
brief's hypothesis — a third of the words restate another file or a printed fact — did not hold:
the group files and the habits are incidents, which the rule keeps whole, and they are most of
the words; the README's rule cells were the largest saving, as hypothesised (−22 % on the rows).

## 2. Claims found false, each corrected in place

`CLAUDE.md`: the six principle-2 sentences A25 ruled on (ADR 2111); §8.6.4.4 quoted as
"concentrations of process colourants" where the standard has "the concentrations of these process
colourants"; §10.7.3 quoted with a lower-case "each". `doc/traps/every-round.md`: its rows "the full
index's rows, verbatim" — row 25 was not; "`cargo test -p conformance --test traps` keeps that file
whole" — the gate reads the README alone; "about four-fifths" of citations — the ten carry 77 % and
twelve carry 80 % (`tools/state.sh traps`). `doc/habits/every-round.md`: the sweep habit's lead was a
paraphrase; the count command missed the numbered `## <n>.` habits. The rule block: "Python runs
with `PYTHONDONTWRITEBYTECODE=1`" — `-I` ignores it (trap 195); "trap 50" — merged into 129.
`doc/HANDOVER.md`: "trap 50". `tools/round.sh`: "four-fifths". `doc/todo/_brief-template.md`: the
wait rule's home. Further claims the four parallel readers found false, each corrected in its file:
environment.md 17 (the `render-lib` checkout, the worktree path, `32 GiB / N`, three self-confined
programs where there are four, `cargo fuzz` "no such subcommand", six submodules where there are
seven, fourteen specifications where there are fifteen, the password file's mode, a missing Xvfb
rule, among them); the trap files 16 (traps 7, 9, 10, 12a, 12b, 12c, 15, 20, 22, 27, 34, 39, 107,
111, 131, 156, and the pixels file's standing facts); the habit files 7 (the worktree links, the
ISO PDF "committed", the clock rule inverted, `examples/witness_census`, `set_field`'s refusal,
§12.8.2.3 "owed", the three sweeps as Python).

## 3. Folds and moves (every incident kept, every number still resolves)

Traps 50 → 129, 69 → 195, 145 → 169, 150 → 162, 182 → 1, 110 → 101, 62 → 160, 57 → 86: each
incident under its survivor as a `####` heading, each row keeping its number and naming where it
lives. Habit 53 into habit 62. Four habits about a ledger reason going stale moved from
`measuring.md` to the ledger habits file, as ADR 0983 left owed. `instruments-and-reports.md`'s
"Things worth knowing" moved above the trap list so the file ends with traps, append-able.

## 4. Deletions, by file and first words

**`CLAUDE.md`**: "Read `doc/HANDOVER.md` after this file. It is an index: which file …" (round.sh
prints the reading); "(vello/wgpu)", "Perf gates run in CI", "No system font enumeration, no full
page-tree walk", "A 500-page document must open", "Targets are set once Spike A" (ADR 2111); "What
follows from that choice is an obligation rather than a licence"; "Three things follow, and they are
requirements"; "It is still … it is still"; "The standing example used to be `DeviceCMYK` …" and
"that claim survived thirty-two sessions in this file" (the incident is trap 6's); "the exclusion
it amends is below"; "This entry used to say "[h]alftones and transfer functions …" and "on
evidence the standard states three times over" (applied, ADR 1023); "and the difference is worth
the words"; "The lesson is more general than the clause"; "which this list did not mention for a
long time …"; "Annex O is the reason this entry is not a formality … not one reported"; "— "For
everything else I agree with your suggestions" — and so adopted … `A254` keeps the owner's words"
(in A254); "This entry used to read "deprecated by ISO 32000-2 itself …"", "and the exclusion is
stronger for resting on a permission …", "which is worth knowing rather than assuming"; "Ratified
on 2026-10-08, when the owner answered" (twice, now a citation); "Annex F is in scope since the
owner ratified linearisation on 2026-09-22"; "they are the first feature on the far side of the
redrawn line"; "already … today"; "the line is where it always was, now written down"; "What such
a page holds is the document's own; what this program composes …"; "The ruling is one, not two";
"— the appearances on the line above each have one — and it is an exception …"; "the
recommendation being "[t]reat it as needing the amendment …" … `A58` keeps the owner's four words"
(in A58); "The sentence that draws the line — *it composes new content over pages* —" (moved to the
boundary paragraph); "belongs onto its own page at coordinates the standard states and the producer
chose"; "The class this opens is named and bounded"; "which `doc/questions/A64` puts there for this
fence's own reason"; "This amendment is the scope decision; the on-page construction is ADR 1123,
built."; "A forbidden annotation's marks now go back onto the producer's own page"; "rather than a
difference on the page"; "The construction and those refusals are ADR 1123, against the reading";
"`A65` keeps the owner's two words and the round's reading visibly apart, as `A58` does"; "This
exclusion read "we do not create files", then … amended four times" (each ratification is at its
paragraph); "are not demoted by any of the above"; "the retirement is `doc/adr/`'s and
`doc/history/`'s, and both are still there"; "The reason is principle 4's, and it is the one
measurement …"; "A student should learn from `run_reader` …"; "into history, names and legitimate
uses"; "the grep alone is a reading list". Every principle, every owner's quotation, every rule and
every ratification's Q/A pair and ADR stay.

**`doc/todo/02` section 0**: "each run declaring its kind: `--tree 6` …" and "**A command that can
run past 240 s is waited for …**" (kept once, in the rule block, which item 3 now names).

**The rule block**: nothing; its first line was split into four, by concern.

**`doc/HANDOVER.md`**: "Read `/CLAUDE.md` first — … **Principle 5 is the one that changes how you
work** …" (CLAUDE.md is read first anyway); "the every-round reading is the five short things";
"every sentence here carries the current reason …" (CLAUDE.md's); "**The round's own record is one
*new file*** …" (section 0 item 4); "## Every round" and its paragraph (round.sh and section 0 item
1); "one row per trap, the position that springs it and the rule", "and every entry to its row",
"A round reads [`every-round.md`] up front …", "`tools/state.sh traps` counts what the rounds cite",
"[`doc/habits/every-round.md`] is the ten every round reads … Three habits bind every round …"
(each the README's, round.sh's or the habits file's own sentence).

**`doc/traps/every-round.md`**: "the rest share the last fifth, most of them cited a few times or
never"; "The shared-machine rules — one heavy walk at a time, `ulimit -u 8192` …" (now one line
pointing at the rule block).

**`doc/habits/every-round.md`**: "and the heading — and the habit itself … is under that heading".

**`doc/environment.md`** below the rule block — "Every worktree
lives under"; the pkill section's "`pkill -x <exact-name>` is not the safe form …", "So: `pkill -f`
against a path is the loud failure …", "It is the same shape as the stash …", "A round knows the pid
of everything it launched"; "Name a scratch file after the round (`gates-656.log`)"; "So a walk is
put under a bound … 32 GiB / N … 16 kept for the owner's desktop"; "That paragraph's 32 GiB took the
machine down … four rules replace it"; "hold under 1 000 at work", "about 3 400 tasks a minute" and
the script list (now `tools/bounded.sh --task-budget`); "Both trees were wrong … So do not `git
stash` here"; "341 lines", "all 109 of its substantive added lines", "the six-hundred-and-forties",
"It does not apply to `main` any more"; "A parallel round is given its own `target-dir`", "19-29 GB
apiece", "the number being repeated in reports was a stale one", "It listed the directories *it*
names, `pdfv-rNNN`" and the hand recipe (`tools/worktree.sh close`); "Nothing was damaged", "It costs
one line"; "Nothing was lost: the amend had nothing staged", "But it could as easily", "`pwd` before
believing"; "and this paragraph named only `-A` for sixty-nine rounds", "In isolation the flag holds
against every shape", "The honest state of it", "A checkout of nine files by name"; "it was read for
many rounds as being about the adapter", "having spent the first hour of the round"; "The viewer can
still be run, and this file said otherwise for dozens of sessions", "This is the only way to exercise
the loop"; "Two corrections from the two-hundred-and-thirteenth session's run", "the sidebar's
scrolling has doubled", "the one interface … that shows a character count", "a photograph of the
past"; "The reason is worth understanding rather than obeying blindly"; "rather than duplicating
work … six rounds'", "the first sibling's edit to a low crate invalidates it again"; "and `cargo
clean -p <crate>` takes only"; "Two rounds of the four-hundred-and-fifties lost time to it", "this
bullet named … until session 967"; "`/home/cl/projects/render-lib` inherits the same wrapper", "It
is not the *path* that does this", "This costs nothing and needs no agreement", "Two older cautions
still hold", "A third, written down because it is what this wrapper is *not*"; "fails with 'no such
subcommand'", "it has been there since 26 July"; "pinned at `ba7d4d61`", "73 MB and 12 MB"; "The
fourteen", "In the three-hundred-and-eleventh session … all 436 commits", "thirty files since",
"mode 600", "four tests and eleven measurement examples", "decided by the owner in that session".
`doc/traps/README.md` — the preamble's "4 → 8, 29 → 13, 36 → habits" list (now a grep), "**Which
traps the rounds actually cite is counted, not written here**", "**Two are not optional for the
round they are about.**" (every-round.md's), and the incident detail in 118 rule cells (rows 10, 25,
49, 51, 55, 58, 61, 63, 65, 66, 68, 70, 71, 73–103, 105–109, 111–122, 124–144, 146–149, 151–171,
173–180, 186, 190 and 207): figures, round numbers and ADR citations their group entry
holds. instruments-and-reports.md — "**This section sat between trap 39 and trap 34 until session
967**", "(This line said *2 s* for a long run of rounds", "a table of gate figures is exactly what
lets a round write "unchanged"" (CLAUDE.md's), "**One word of that paragraph is superseded by the
trap below**" (applied), "merged here in the one-thousand-and-twenty-fifth session". The four other
group files — "[`README.md`](README.md) is the index: one line per trap saying the position …"
(the README's own preamble), trap 8's "merged here in the one-thousand-and-twenty-fifth session …
Both incidents are kept …", trap 170's "And after a session change a window started through the
owner's loop may draw and be found by nothing" (trap 174's incident). measuring.md — "and the two
used to carry the incident twice, dated to two different sessions"; habit 104's "A command that can
run past 240 s is started with `tools/wait.sh --start`" (the rule block's). `doc/habits.md` — "It
was the largest file in this corpus sitting behind one entry", "**One thing the split made visible
and did not fix**" (done).

## 5. What stays as it was

`doc/adr/`, `doc/history/`, `doc/reviews/` and the owner's `A`/`Q` files. Every trap and habit
number. The README's last rows and each group file's tail, append-able. `tools/round.sh`'s lists
(one description corrected). No principle's wording beyond A25's ruling; Q532 was not needed.

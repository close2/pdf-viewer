# The ten habits every round reads

Status: **standing** — the every-round half of the habits (ADR 1639).

**This is the short list.** The six habit files hold the rest, each a bold-led item or a numbered
`## <n>.` heading (`grep -cE '^- \*\*|^## [0-9]+\.' doc/habits/*.md` counts them), and a round
opens the one its kind of work is about (`tools/round.sh <kind>` names it); the ten below are the
ones the records of the last batches show rounds paying for most often, whatever the round was
about. Each line is a pointer — the file and the bold lead — and the habit itself, with the
incident that earned it, is there. Nothing is decided from a line here alone.

| read when | the habit | where |
|---|---|---|
| a brief hands you a premise — a clause number, a table, "what is left", a figure | **A brief's premise is checked in the text before anything is built on it** — in `doc/md/`, in the code (`grep` the refusal's call sites) and in the data; a figure quoted from a record carries its unit | [`reading-the-specification.md`](reading-the-specification.md) |
| you are about to write that the standard defines nothing | **"The standard states nothing here" is checked against the clause's own headings before anything rests on it** | [`reading-the-specification.md`](reading-the-specification.md) |
| you implement a clause that ships an EXAMPLE | **A clause that ships an EXAMPLE is tested by running the example through the code** (habit 39) | [`measuring.md`](measuring.md) |
| a brief names a lever, a cache or a cost to cut | **The page is profiled before the brief's lever is built** (habit 59) — the stage that holds the time is found first | [`measuring.md`](measuring.md) |
| you call a host feature done | **A host feature is driven under Xvfb with more than one document before it is called done** | [`tests-gates-and-reports.md`](tests-gates-and-reports.md) |
| you need a baseline from a file a sibling may be editing | **Never plant-and-restore with `cp` on a file a sibling is editing** — a patch of your own, applied and reversed | [`tests-gates-and-reports.md`](tests-gates-and-reports.md) |
| you add a document to a gate's held list | **A held gate population is checked for a published answer before it is accepted** | [`tests-gates-and-reports.md`](tests-gates-and-reports.md) |
| you are sent to act on a sweep's count | **A number from an instrument that has never fired is not a finding** — ten of its hits are read against the standard first | [`measuring.md`](measuring.md), `doc/todo/02` section 8 |
| you want a figure about a dependency | **A dependency's `unsafe` is counted with grep, and a derived status is never hand-written** | [`code-bounds-and-dependencies.md`](code-bounds-and-dependencies.md) |
| two backends disagree on a pixel | **A cross-backend difference is settled against the clause's own set, drawn in both directions** | [`judging-against-other-implementations.md`](judging-against-other-implementations.md) |

**Three more bind every round and live beside the round they bind**: `doc/todo/02` section 7's
"a closed form taken from one renderer is not a limit", "a count that improves is not a picture",
and "a round that changes what gets drawn re-runs `doc/todo/00`'s step 7".

## Context is measured after the reading list, and a slot spanning five crates is two slots

Round 1906 (ADRs 2646, 2647) wired `$event` through `pdf-model`, `pdf-script`, `pdf-formcalc`,
`pdf-script-worker` and `viewer-core` and reached the handover point before its tests were written. A slot's
reading list alone costs about 100k of context; a round reads its own context as soon as the list is read
and sizes what remains, and a brief that names five crates in one slot has written two slots.

## A converter path over three hundred lines is read by a Sonnet subagent before any code is written

Round 1901 (ADR 2636) read `embed_pass` and its routes in `archive/fonts.rs` itself — about three hundred
lines — and that reading cost some 40k of context before the first edit. A path that long is summarised by a
Sonnet subagent into the entry points, the routes and the line numbers the slot needs; the round then
reads only the ranges it will edit.

## A test in a shared worktree never imports a helper from a sibling's untracked file

Round 1903 found round 1902's `tests/archive/as_drawn.rs` not compiling in the worktree (a private
`HELVETICA`, a `Target::Four(None)`), and an export of HEAD plus one round's own hunks will not compile when
a test leans on a sibling's untracked file. A test under `tests/archive/` imports only from the tracked
tree and its own module; a helper two rounds need is tracked by the first and read by the second after it
lands.

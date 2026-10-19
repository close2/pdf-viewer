# 1983 — A brief names its sessions, and the remedies' census is one command

Session 1573, slot 3 of batch ninety-one. Status: **accepted, built**. Context: round 1570 was a
seventh slot of a batch whose branch, `batch-1564-1569`, names six, and `ROUND_SESSION=1570
tools/round.sh` refused it; and an archive slot's premise was two commands, `tools/state.sh
remedies` and `examples/archive_unbuilt`, quoted from a record. Code: `tools/round.sh`,
`tools/state.sh` (`section_remedies`, `quick`). It cites no clause and moves no row.

## 1. The brief is the authority on its slots

A branch is named before its brief is finished, so its range is the first six slots and not the
batch. `ROUND_SESSION` past the range is accepted where the brief, `/home/AI/batch-<first>-brief.md`
(`ROUND_BRIEF` names another), has a slot heading ending `— session <n>` — the shape every brief
from `doc/todo/_brief-template.md` writes — and `tools/round.sh` prints the slot and the brief's path
under "the round" for any session the brief names, in range or not. A session neither names is still
refused, and the refusal now says which brief it read, or that it could not read one. Driven by hand:
1573 prints "Slot 3 of the brief /home/AI/batch-1571-brief.md"; 1577 and 1570 are refused against it;
1570 against batch ninety's brief is its Slot 7; an unreadable brief is said.

## 2. The census is the remedies section's second half

`tools/state.sh remedies` printed `--remedy-sites`'s two totals, which need no document. Its third
heading now runs the census over the corpora `doc/todo/72` names — `cargo run --release -p
pdf-transform --example archive_unbuilt -- doc/pdf.js/test/pdfs doc/corpora` as a small walk — and
keeps the opened count, each target's heading and every site at least one document fails, ranked, with
its kind. A site no document reaches is still owed and the first half counts it. The walk took
201 s and 0.87 GiB, inside a small walk's 600 s and 6 GiB, so **`remedies` leaves `quick`**: a
section that walks a corpus is not quick, and `archive`, which calls it, prints the whole item still.
The veraPDF corpus stays a separate run, for `doc/todo/72`'s reason: its files are built to fail one
rule each.

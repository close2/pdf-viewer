# 0979 — A capability floor that ran on one machine, and 97% of it came from files git does not have

Status: accepted. Session 968.
Context: `crates/viewer-core/tests/accessibility_census.rs`, `doc/todo/31-accessibility-host.md`,
ADRs 0421, 0962, 0970, `doc/traps/instruments-and-reports.md` trap 39.
It **completes ADR 0970**, which fixed one ceiling in this file and named the other half as left
undone.

## What was there

`ratchet` holds fourteen capability floors — elements reached, cells given headers, places a caret
can stand — each a sum over the population, each allowed only to rise. In front of them stood a
guard, and its argument was good:

> Every floor here is a count over 988 documents […] Comparing a smaller population against these
> would fail for the one reason that is not a regression, so the floors are skipped and the skip
> says so — which is the same guard ADR 0421 put under the selection verdict's judged set.

`population()` walks two directories: `doc/pdf.js/test/pdfs`, a submodule pinned by commit, and
`doc/` itself, which holds the specifications this project has bought. **`.gitignore` excludes the
second.**

## Two facts, and between them the defect

**The submodule holds 974 documents. The threshold was 988.** So the threshold was the submodule
*plus fourteen bought files that happened to be on this disk the day somebody measured*. On any
clone without them — which is every other clone, and CI — `files < 988` is true, the guard fires,
the gate prints one line and exits 0.

**These floors have never run anywhere but this machine.** Not once, on any other checkout, since
the guard was written.

And the second fact is worse than the first. Folding the census over the tracked documents alone:

| | tracked (`doc/pdf.js`) | whole population |
|---|---|---|
| elements reached | 4,060 | 216,289 |
| characters a caret reaches | 31,433 | 5,196,091 |
| cells with headers | 58 | 23,032 |

**About 97% of every floor comes from the twenty-five gitignored specifications.** That is not a
flaw in the corpus and it is not surprising once said out loud: pdf.js's suite collects *rendering*
bugs, and tagged structure at this scale lives in documents somebody published for accessibility —
which, here, means the standards themselves. But it means the instrument that guards this project's
accessibility work was, in substance, a ratchet over fourteen bought PDFs, guarded by a count that
silently disabled it everywhere they were absent.

## Calibrated, per trap 13, against the defect itself

Not reasoned about — run. `population()`'s second directory was pointed at a name no clone has, so
the walk sees exactly what a fresh checkout sees, and the **committed** version of the file was run
against it:

```
not ratcheted: 974 documents against the 988 the floors were taken over —
`git submodule update --init doc/pdf.js`, and `doc/environment.md`'s one unzip
test result: ok. 1 passed
```

Fourteen floors skipped, exit 0, and **the remedy it printed could never have worked**: the
submodule was fully checked out — that is where the 974 came from. What was missing was files
`.gitignore` excludes and no command can fetch. The gate told a machine to run something irrelevant
and then passed.

The same simulated clone under this ADR's version walks its 974 documents, runs tier one, passes,
and names all twenty-five absent specifications in the skip. With every specification present it
exits 0 with both tiers; with one moved aside it skips tier two naming that file and still runs
tier one.

## Why the obvious fix is wrong

Restrict the floors to the tracked corpus and the gate becomes portable, honest, and **97% weaker**
— it would no longer notice a regression that cost two hundred thousand elements, as long as four
thousand survived. Trading a gate that runs nowhere for a gate that catches nothing is not a fix.

## The decision: two tiers, and the second is gated by name

**Tier one, always:** the fourteen floors re-measured over the tracked documents alone, guarded by a
threshold counting only tracked documents. Weak, but real, and real *everywhere* — this is the part
CI has never had.

**Tier two, when the evidence is present:** the same fourteen floors over the whole population, run
only if every specification in `RATCHETED_SPECIFICATIONS` is on the disk — **checked by name, not by
count.** Absent ones skip the tier and are named in the skip.

By name is the whole of the correction, and it is ADR 0970's rule arriving from the other side. A
count cannot tell *these fourteen documents* from *any fourteen documents*, so the old guard could
be satisfied by a coincidence of arithmetic — and in fact was, every day, by twenty-five unrelated
files being present. A name list cannot be.

The asymmetry falls out correctly and is worth stating because it is why this is cheap to live with:
**a floor is a lower bound, so an extra specification can only raise it.** Downloading a new
specification needs no change here. Removing one skips the tier and says which — and that is the
event the old guard was reaching for and failed to name.

## What this does not fix, and is not pretending to

The second tier still cannot run on a machine that has not bought these documents, and no amount of
instrument design changes that — the evidence is licensed to a single reader. What changed is that
such a machine now **knows** it is running the weak tier and is told which files would strengthen
it, instead of being handed a green gate that checked nothing.

## A defect this round introduced and then found

ADR 0970's session inserted `NO_PARENT_KEY_SILENT` immediately above `fn ratchet`, which put the
constant **inside `ratchet`'s doc comment** — four paragraphs arguing about the ratchet became the
documentation of a name list, and `ratchet` was left with none. `clippy::pedantic` cannot see it:
both items are documented, and the words are in the wrong place rather than missing. It is the
cheapest possible reminder that `#![warn(missing_docs)]` checks for the presence of a doc comment
and nothing about what it is attached to. Corrected here.

# 968 — A capability floor that ran on one machine, and 97% of it came from files git does not have

Date: 2026-09-11. ADR: 0979.
Files: `crates/viewer-core/tests/accessibility_census.rs`.

ADR 0970 fixed one of this file's ceilings last round and named the other half as left undone. The
other half was worse.

`ratchet`'s fourteen capability floors stood behind a guard that skipped them when the population
was smaller than the 988 documents they were measured over. **`doc/pdf.js` holds 974.** The other
fourteen were bought specifications sitting directly in `doc/`, which `.gitignore` excludes — so on
every clone but this one the guard fired, the floors were skipped, and the gate exited 0. Run
against a simulated fresh checkout, the committed version prints `not ratcheted: 974 documents
against the 988` and checks nothing; **the remedy it prints —
`git submodule update --init doc/pdf.js` — could never have worked**, because the submodule was
exactly what the 974 came from.

Then the fold over tracked documents alone gave the number that decided the design: **4,060
elements against 216,289, and 31,433 characters against 5,196,091.** About 97% of every floor comes
from the twenty-five gitignored specifications. Which is not a flaw in the corpus — pdf.js's suite
collects *rendering* bugs, and tagged structure at this scale lives in documents somebody published
for accessibility, which here means the standards themselves.

That makes the obvious fix wrong: restricting the floors to what every clone has would trade a gate
that runs nowhere for a gate that catches nothing. So two tiers. Tier one is the fourteen floors
re-measured over the tracked corpus, always run — the part CI has never had. Tier two is the same
fourteen over the whole population, run only when every specification is present, **checked by
name**.

By name is the whole correction, and it is ADR 0970's rule arriving from the other side: a count
cannot tell *these fourteen documents* from *any fourteen*, so the old guard was satisfied every day
by twenty-five unrelated files being present. The asymmetry then falls out right — a floor is a
lower bound, so an extra specification can only raise it and needs no change here; a missing one
skips the tier and says which.

Also corrected: last round's insertion put `NO_PARENT_KEY_SILENT` **inside `ratchet`'s doc
comment**, so four paragraphs arguing about the ratchet documented a name list and `ratchet` had
none. `clippy::pedantic` cannot see that — both items are documented, and the words are merely in
the wrong place.

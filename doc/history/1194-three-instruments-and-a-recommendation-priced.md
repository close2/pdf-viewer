# 1194 — Three instruments, and Annex F priced instead of postponed

Instruments round of batch 1189–1194. ADRs 1225 and 1226.

## The launch-path gate says which of its figures the machine was fit to judge

`launch_path.rs` was being **declined rather than disbelieved**: three rounds in one week did not
take its clock figures at all, each on a private judgement about the load average.
`the_load_ceiling` now counts the machine's physical cores from `sysfs`'s topology — the point
above which a freshly woken child cannot be given a core to itself, which is the contention ADR
0916 measured at 43% with the kernel's wait counter reading zero. Each figure is bracketed by a
load reading; above the ceiling the attempt is re-taken up to `LOAD_ATTEMPTS` times, and the loop
**stops at the first attempt taken under it** so a judged figure stays the minimum of exactly
`SAMPLES` children — pooling would have biased the minimum downward and failed a figure out of the
*bottom* of a band. Otherwise the smallest is printed, not judged, with the load named, and the
load is asked first among the declines. ADR 1226.

## §6.3.2.1's declined `should` is `departed`, on a measurement

`crates/pdf-syntax/examples/linearised_census.rs` is new: it counts how many documents state
`/Linearized` inside §F.3.3's 1024-byte window, how many of those state an `L` that is not the
file's length and are therefore what Table F.1 calls not linearised at all, and what an open reads
before page one against Table F.1's `E`, warm or cold. It found the recommendation genuinely declined rather than reached another way — most linearised
files here are opened by reading past their own first-page region and by resolving the page tree
§F.3.10 puts at the far end — and it found the price to be the one §F.1 states, "although not as
efficiently" rather than less correctly: a fraction of a millisecond inside a time to first page
whose largest term is the graphics driver. §F.2's assumed bottleneck is a transport this program
does not use, and §F.4's hint tables name byte ranges for a request it never issues. `doc/todo/42`
item 7 keeps the two parts that *would* pay if the figure moved. ADR 1225.

## `tools/batch.sh check` gained two lines, both calibrated

A record numbered behind one already committed, and an escaped section sign in a doc comment —
`citation.rs` finds a clause by looking for `§` as a character. Both named their plant and a
correctly-numbered record was the control. Reading all fifteen `\u{...}` escapes in `crates/`'s doc comments showed five are section signs
naming real clauses and the other ten name a character invisible on the page, so the rule is the
section sign alone, scoped to the lines a batch **adds**: scoped by file it reported one of the
five against the round editing that file for something else. Those five hide real citations from
every gate that reads them, in `pdf-archive` and `pdf-transform`; they are not this round's files
and want a round that owns them.
